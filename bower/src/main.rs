//! The `bower` CLI — the replay layer's front door.
//!
//! Everything the kernel refuses to do lives here: reading a book off disk,
//! parsing `bower.toml`, and writing git objects. `bower-core` stays pure, and
//! CI asserts that with `cargo tree -p bower-core -e normal`.
//!
//! `plan` resolves the book and writes `bower.lock`; `build` replays the plan
//! into a git repository, one commit per step, from an empty tree every time;
//! `verify` runs a compiler against every step's tree and checks that what the
//! book declared is what actually happens.

#![warn(clippy::pedantic, clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use bower_core::prelude::{lock_text, plan, BookPlan, PlannedStep};
use clap::{Parser, Subcommand};

use bower::config::BookConfig;
use bower::forge::{Forge, GitHubForge};
use bower::loader::BookLoader;
use bower::publish::{
    render_plan, BookMeta, MdBookRenderer, PandocRenderer, RenderPlan, Renderer, Target,
    TypstRenderer,
};
use bower::push::{plan_push, PushPlan};
use bower::replay::{book_name, final_blobs, scaffolding, Replayer};
use bower::status::{lock_drift, repo_drift, site_drift, StatusReport};
use bower::verify::{Verdict, Verifier};

#[derive(Debug, Parser)]
#[command(
    name = "bower",
    version,
    about = "Replay a book into deterministic git repositories."
)]
struct Cli {
    /// The book's root directory — the one holding `bower.toml`.
    #[arg(long, default_value = ".", global = true)]
    book: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Resolve the book into a plan, print it, and write `bower.lock`.
    Plan {
        /// Limit the run to one target repository.
        #[arg(long)]
        repo: Option<String>,
    },
    /// Render the book: `--target html` or `--target epub`.
    Publish {
        /// Which artifact to produce. Required: silently producing the wrong
        /// one is worse than asking.
        #[arg(long)]
        target: Target,

        /// Where to write it.
        #[arg(short, long, default_value = "published")]
        out: PathBuf,
    },
    /// Publish a built repository to its configured remote.
    ///
    /// Reports what it would do and changes nothing unless `--execute` is
    /// given. Refuses to force-push over any repository Bower did not generate,
    /// and there is no way to override that.
    Push {
        /// Limit the run to one target repository.
        #[arg(long)]
        repo: Option<String>,

        /// The built repository to publish.
        #[arg(short, long, default_value = "out")]
        out: PathBuf,

        /// The rendered book to publish to the site branch. Defaults to
        /// mdBook's own output directory inside the book.
        #[arg(long)]
        site: Option<PathBuf>,

        /// Actually push. Without this the command only reports.
        #[arg(long)]
        execute: bool,
    },
    /// Report what is out of date: the lock, a built repo, or neither.
    Status {
        /// Limit the run to one target repository.
        #[arg(long)]
        repo: Option<String>,

        /// A previously built repository to check. Absent means "not built",
        /// which is reported rather than treated as an error.
        #[arg(short, long, default_value = "out")]
        out: PathBuf,

        /// The rendered book to check. Defaults to mdBook's own output
        /// directory inside the book.
        #[arg(long)]
        site: Option<PathBuf>,
    },
    /// Check every step's declared `expect` against a real compiler.
    Verify {
        /// Limit the run to one target repository.
        #[arg(long)]
        repo: Option<String>,

        /// Verify only this step.
        #[arg(long, conflicts_with = "from")]
        step: Option<String>,

        /// Verify from this step onward.
        #[arg(long)]
        from: Option<String>,

        /// Scratch directory for the trees under test and their shared cargo
        /// target directory.
        #[arg(long, default_value = "target/bower-verify")]
        work: PathBuf,
    },
    /// Replay the plan into a local git repository, one commit per step.
    Build {
        /// Limit the run to one target repository.
        #[arg(long)]
        repo: Option<String>,

        /// Where to write. One repo uses this directory as-is; several get a
        /// subdirectory each, because two repos cannot share a working tree.
        #[arg(short, long, default_value = "out")]
        out: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let cfg = match BookConfig::load(&cli.book) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("bower: {e}");
            return ExitCode::FAILURE;
        }
    };

    match cli.command {
        Command::Plan { repo } => run_plan(&cli.book, &cfg, repo.as_deref()),
        Command::Build { repo, out } => run_build(&cli.book, &cfg, repo.as_deref(), &out),
        Command::Status { repo, out, site } => {
            let site = site.unwrap_or_else(|| cli.book.join("book"));
            run_status(&cli.book, &cfg, repo.as_deref(), &out, &site)
        }
        Command::Publish { target, out } => run_publish(&cli.book, &cfg, target, &out),
        Command::Push {
            repo,
            out,
            site,
            execute,
        } => {
            // mdBook's own output directory, so `make book` and `bower push`
            // agree about where the rendered book lives without being told.
            let site = site.unwrap_or_else(|| cli.book.join("book"));
            run_push(&cli.book, &cfg, repo.as_deref(), &out, &site, execute)
        }
        Command::Verify {
            repo,
            step,
            from,
            work,
        } => run_verify(
            &cli.book,
            &cfg,
            repo.as_deref(),
            step.as_deref(),
            from.as_deref(),
            &work,
        ),
    }
}

/// Load the book and resolve it through the kernel, reporting either failure in
/// the same shape.
fn resolve(book_root: &Path, cfg: &BookConfig) -> Option<BookPlan> {
    let book = match BookLoader::new(book_root).load() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("bower: {e}");
            return None;
        }
    };
    match plan(&book, &cfg.catalog()) {
        Ok(p) => Some(p),
        Err(errs) => {
            eprintln!("bower: the book does not resolve:\n{errs}");
            None
        }
    }
}

fn run_plan(book_root: &Path, cfg: &BookConfig, only: Option<&str>) -> ExitCode {
    let Some(resolved) = resolve(book_root, cfg) else {
        return ExitCode::FAILURE;
    };

    let selected: Vec<_> = resolved
        .repos
        .iter()
        .filter(|r| only.is_none_or(|want| want == r.repo.0))
        .collect();

    // `build`, `status`, and `push` all refuse an unmatched `--repo`. Printing
    // nothing and exiting 0 reads as success, which is worse than an error.
    if selected.is_empty() {
        eprintln!("bower: no repo matched");
        return ExitCode::FAILURE;
    }

    for repo in selected {
        println!("{} — {} steps", repo.repo, repo.steps.len());
        for step in &repo.steps {
            println!(
                "  {:<28} {:<12} {}",
                PlannedStep::tag(step),
                step.expect,
                step.msg
            );
        }
    }

    let lock_path = book_root.join("bower.lock");
    if let Err(e) = std::fs::write(&lock_path, lock_text(&resolved)) {
        eprintln!("bower: cannot write {}: {e}", lock_path.display());
        return ExitCode::FAILURE;
    }
    println!("\nwrote {}", lock_path.display());
    ExitCode::SUCCESS
}

fn run_build(book_root: &Path, cfg: &BookConfig, only: Option<&str>, out: &Path) -> ExitCode {
    let Some(resolved) = resolve(book_root, cfg) else {
        return ExitCode::FAILURE;
    };

    let selected: Vec<_> = resolved
        .repos
        .iter()
        .filter(|r| only.is_none_or(|want| want == r.repo.0))
        .collect();

    if selected.is_empty() {
        eprintln!("bower: no repo matched");
        return ExitCode::FAILURE;
    }
    let single = selected.len() == 1;

    for repo in selected {
        let dir = if single {
            out.to_path_buf()
        } else {
            out.join(&repo.repo.0)
        };
        let replayer = Replayer {
            config: cfg,
            book_root,
            out_dir: &dir,
        };
        match replayer.run(repo) {
            Ok(report) => println!(
                "{}: {} commits, {} tags, HEAD {} → {}",
                report.repo,
                report.commits,
                report.tags.len(),
                report.head,
                dir.display()
            ),
            Err(e) => {
                eprintln!("bower: replaying {} failed: {e}", repo.repo);
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}

#[allow(clippy::too_many_lines)]
fn run_verify(
    book_root: &Path,
    cfg: &BookConfig,
    only_repo: Option<&str>,
    step: Option<&str>,
    from: Option<&str>,
    work: &Path,
) -> ExitCode {
    let Some(resolved) = resolve(book_root, cfg) else {
        return ExitCode::FAILURE;
    };

    let selected: Vec<_> = resolved
        .repos
        .iter()
        .filter(|r| only_repo.is_none_or(|want| want == r.repo.0))
        .collect();

    if selected.is_empty() {
        eprintln!("bower: no repo matched");
        return ExitCode::FAILURE;
    }

    let named_step = step.or(from);
    let mut broken_total = 0_usize;
    let mut ran_any = false;

    for repo in selected {
        // A step id belongs to one repo. Judging `--step` against every repo's
        // plan makes a valid request fail on whichever repo happens to sort
        // first — so skip the repos that do not hold it, and let the check
        // below catch a name that no repo holds at all.
        if let Some(id) = named_step {
            if !repo.steps.iter().any(|s| s.id.0 == id) {
                continue;
            }
        }
        ran_any = true;
        let verifier = Verifier {
            config: cfg,
            book_root,
            work_dir: work,
        };
        let report = match verifier.run(repo, step, from) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("bower: {e}");
                return ExitCode::FAILURE;
            }
        };

        println!("{} — {} step(s)", report.repo, report.verdicts.len());
        for v in &report.verdicts {
            let mark = match &v.verdict {
                Verdict::Upheld => "ok  ",
                Verdict::Skipped => "skip",
                Verdict::Broken { .. } => "FAIL",
            };
            println!("  {mark} {:03} {:<28} expect={}", v.seq, v.id.0, v.expect);
        }

        for v in report.broken() {
            broken_total += 1;
            let Verdict::Broken {
                happened,
                command,
                stderr,
            } = &v.verdict
            else {
                continue;
            };
            eprintln!(
                "\n{}: step `{}` claims `{}`, but {happened}.",
                v.anchor, v.id.0, v.expect
            );
            eprintln!("    (`{command}` in the step's tree)");
            let tail: Vec<&str> = stderr.lines().rev().take(8).collect();
            for line in tail.iter().rev() {
                eprintln!("    {line}");
            }
        }
    }

    // Skipping repos that lack the step must not turn a typo into a silent
    // success.
    if let (false, Some(id)) = (ran_any, named_step) {
        eprintln!("bower: no step named `{id}` in any repo");
        return ExitCode::FAILURE;
    }

    if broken_total == 0 {
        println!("\nevery claim holds");
        ExitCode::SUCCESS
    } else {
        eprintln!("\n{broken_total} claim(s) in the book are not true");
        ExitCode::FAILURE
    }
}

fn run_status(
    book_root: &Path,
    cfg: &BookConfig,
    only: Option<&str>,
    out: &Path,
    site_dir: &Path,
) -> ExitCode {
    let Some(resolved) = resolve(book_root, cfg) else {
        return ExitCode::FAILURE;
    };
    // The site's fingerprint covers the chapters' prose, not only the plan.
    let Ok(book_for_site) = BookLoader::new(book_root).load() else {
        eprintln!("bower: cannot read the book");
        return ExitCode::FAILURE;
    };

    let lock = match lock_drift(book_root, &resolved) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("bower: {e}");
            return ExitCode::FAILURE;
        }
    };

    let selected: Vec<_> = resolved
        .repos
        .iter()
        .filter(|r| only.is_none_or(|want| want == r.repo.0))
        .collect();

    if selected.is_empty() {
        eprintln!("bower: no repo matched");
        return ExitCode::FAILURE;
    }
    let single = selected.len() == 1;
    let mut drifted = false;

    for repo in selected {
        let dir = if single {
            out.to_path_buf()
        } else {
            out.join(&repo.repo.0)
        };
        let name = book_name(book_root, &repo.repo.0);
        let scaffold = match scaffolding(cfg, book_root, &repo.repo.0) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("bower: cannot read the template: {e}");
                return ExitCode::FAILURE;
            }
        };
        let expected = final_blobs(repo, &name, cfg.site.as_deref(), &scaffold);

        let repo_state = match repo_drift(&dir, repo, &expected) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("bower: {e}");
                return ExitCode::FAILURE;
            }
        };

        let report = StatusReport {
            repo: repo.repo.0.clone(),
            steps: repo.steps.len(),
            site: site_drift(
                site_dir,
                &name,
                &bower::publish::site_fingerprint(&book_for_site, &lock_text(&resolved)),
                cfg.repos
                    .get(&repo.repo.0)
                    .is_some_and(|r| r.site_branch.is_some()),
            ),
            // The lock covers the whole book, so every repo reports the same
            // verdict for it. Repeating it beats hiding it above the repo it
            // applies to.
            lock: lock.clone(),
            repo_state,
        };
        print!("{report}");
        drifted |= report.has_drift();
    }

    if drifted {
        eprintln!("\nsomething is out of date");
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn run_push(
    book_root: &Path,
    cfg: &BookConfig,
    only: Option<&str>,
    out: &Path,
    site_dir: &Path,
    execute: bool,
) -> ExitCode {
    let Some(resolved) = resolve(book_root, cfg) else {
        return ExitCode::FAILURE;
    };

    let forge = match GitHubForge::preflight() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("bower: {e}");
            return ExitCode::FAILURE;
        }
    };

    // The site's fingerprint covers the chapters' prose, not only the plan.
    let Ok(book) = BookLoader::new(book_root).load() else {
        eprintln!("bower: cannot read the book");
        return ExitCode::FAILURE;
    };
    let fingerprint = bower::publish::site_fingerprint(&book, &lock_text(&resolved));

    let selected: Vec<_> = resolved
        .repos
        .iter()
        .filter(|r| only.is_none_or(|want| want == r.repo.0))
        .collect();

    if selected.is_empty() {
        eprintln!("bower: no repo matched");
        return ExitCode::FAILURE;
    }
    let single = selected.len() == 1;
    let mut blocked = false;

    for repo in selected {
        let dir = if single {
            out.to_path_buf()
        } else {
            out.join(&repo.repo.0)
        };

        let plan = match plan_push(&forge, cfg, &fingerprint, repo, &dir, site_dir, book_root) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("bower: {e}");
                return ExitCode::FAILURE;
            }
        };

        if !report_push(&forge, plan, &dir, execute, &mut blocked) {
            return ExitCode::FAILURE;
        }
    }

    if blocked {
        eprintln!("\nnothing was published");
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn run_publish(book_root: &Path, cfg: &BookConfig, target: Target, out: &Path) -> ExitCode {
    let book = match BookLoader::new(book_root).load() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("bower: {e}");
            return ExitCode::FAILURE;
        }
    };
    let Some(resolved) = resolve(book_root, cfg) else {
        return ExitCode::FAILURE;
    };
    let meta = match BookMeta::load(book_root) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("bower: {e}");
            return ExitCode::FAILURE;
        }
    };

    let links: std::collections::BTreeMap<_, _> = cfg
        .repos
        .iter()
        .map(|(name, r)| (name.clone(), r.links.clone()))
        .collect();

    let plan: RenderPlan = render_plan(&book, &resolved, meta, target, &links);

    // Every renderer is checked before anything is written, so a missing
    // binary costs nothing and says how to fix itself.
    let renderer: Box<dyn Renderer> = match target {
        Target::Epub => Box::new(PandocRenderer),
        Target::Html => Box::new(MdBookRenderer {
            book_root: book_root.to_path_buf(),
            // Only a book that ships a site needs the marker beside its HTML.
            site_marker: cfg
                .repos
                .values()
                .any(|r| r.site_branch.is_some())
                .then(|| {
                    let name = book_name(book_root, "book");
                    // The fingerprint, not the lock: prose changes the render
                    // without changing the plan.
                    let fp = bower::publish::site_fingerprint(&book, &lock_text(&resolved));
                    bower::publish::site_marker(&name, &fp)
                }),
        }),
        Target::Pdf => Box::new(TypstRenderer {
            // One epoch, every artifact: the value that already pins commit
            // times pins the PDF's too.
            epoch: cfg.epoch.unix_timestamp(),
            template: {
                let t = book_root.join("template.typ");
                t.exists().then_some(t)
            },
        }),
    };
    if let Err(e) = renderer.preflight() {
        eprintln!("bower: {e}");
        return ExitCode::FAILURE;
    }

    println!(
        "{} — {} chapters → {target}",
        plan.meta.title,
        plan.chapters.len()
    );
    match renderer.render(&plan, out) {
        Ok(a) => {
            println!("wrote {} ({} bytes)", a.path.display(), a.bytes);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("bower: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Print one repo's push plan and, with `--execute`, carry it out.
///
/// Split out of `run_push` because the two halves — deciding what to say, and
/// deciding whether to act — read better apart, and together they ran past the
/// line limit.
fn report_push(
    forge: &dyn Forge,
    plan: PushPlan,
    dir: &Path,
    execute: bool,
    blocked: &mut bool,
) -> bool {
    match plan {
        PushPlan::NotConfigured { repo } => {
            println!("{repo}: no `github` key — this book does not publish it");
        }
        PushPlan::Blocked { repo, reason } => {
            *blocked = true;
            println!("{repo}: REFUSED");
            println!("  {reason}");
        }
        PushPlan::Ready {
            repo,
            remote,
            branch,
            tags,
            create,
            site,
        } => {
            println!("{repo} → {remote}");
            println!("  branch    {branch}");
            println!("  tags      {tags}");
            if create {
                println!("  create    the remote does not exist yet");
            }
            match &site {
                Some(s) => println!(
                    "  site      {} — {} files{}",
                    s.branch,
                    s.files,
                    if s.create { " (branch is new)" } else { "" }
                ),
                None => println!("  site      no `site_branch` — skipped"),
            }
            if !execute {
                println!("  dry run   nothing was sent; pass --execute to publish");
                return true;
            }
            if create {
                let desc = format!("Generated from the book `{repo}`. Do not open pull requests.");
                if let Err(e) = forge.create(&remote, &desc) {
                    eprintln!("bower: {e}");
                    return false;
                }
            }
            match forge.push(dir, &remote, &branch) {
                Ok(o) => println!("  pushed    {} commits, {} tags", o.commits, o.tags),
                Err(e) => {
                    eprintln!("bower: {e}");
                    return false;
                }
            }
            if let Some(s) = site {
                match forge.push_tree(&s.dir, &remote, &s.branch) {
                    Ok(o) => println!("  site      pushed {} files to {}", o.tags, s.branch),
                    Err(e) => {
                        eprintln!("bower: {e}");
                        return false;
                    }
                }
            }
        }
    }
    true
}
