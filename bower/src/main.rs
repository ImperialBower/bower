//! The `bower` CLI — the replay layer's front door.
//!
//! Everything the kernel refuses to do lives here: reading a book off disk,
//! parsing `bower.toml`, and writing git objects. `bower-core` stays pure, and
//! CI asserts that with `cargo tree -p bower-core -e normal`.
//!
//! As of EPIC-01 Phase 2, `bower plan` is real: it loads the book, resolves it
//! through the kernel, prints the step table, and writes `bower.lock`.
//! `bower build` still reports its configuration and exits non-zero, so no
//! script can mistake progress for completion.

#![warn(clippy::pedantic, clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

mod config;
mod loader;

use bower_core::prelude::{plan, PlannedStep};
use config::BookConfig;
use loader::BookLoader;

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
    /// Replay the plan into a local git repository, one commit per step.
    Build {
        /// Limit the run to one target repository.
        #[arg(long)]
        repo: Option<String>,

        /// Directory to write the generated repository into.
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
        Command::Build { repo, out } => {
            report(&cfg, repo.as_deref());
            eprintln!("bower build: not implemented yet — see EPIC-01 Phase 3.");
            eprintln!("             (would have written to {})", out.display());
            ExitCode::FAILURE
        }
    }
}

/// Resolve the book into a plan, print the step table, and write `bower.lock`.
fn run_plan(book_root: &Path, cfg: &BookConfig, only: Option<&str>) -> ExitCode {
    let book = match BookLoader::new(book_root).load() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("bower: {e}");
            return ExitCode::FAILURE;
        }
    };

    let resolved = match plan(&book, &cfg.catalog()) {
        Ok(p) => p,
        Err(errs) => {
            eprintln!("bower: the book does not resolve:\n{errs}");
            return ExitCode::FAILURE;
        }
    };

    for repo in &resolved.repos {
        if only.is_some_and(|want| want != repo.repo.0) {
            continue;
        }
        println!("{} — {} steps", repo.repo, repo.steps.len());
        for step in &repo.steps {
            println!(
                "  {:<28} {:<12} {}",
                PlannedStep::tag(step),
                step.expect.to_string(),
                step.msg
            );
        }
    }

    let lock_path = book_root.join("bower.lock");
    if let Err(e) = std::fs::write(&lock_path, bower_core::prelude::lock_text(&resolved)) {
        eprintln!("bower: cannot write {}: {e}", lock_path.display());
        return ExitCode::FAILURE;
    }
    println!("\nwrote {}", lock_path.display());
    ExitCode::SUCCESS
}

/// Print what the configuration says, so the parser is observable before the
/// phases that consume it exist.
fn report(cfg: &BookConfig, only: Option<&str>) {
    println!("epoch:    {}", cfg.epoch);
    println!("identity: {} <{}>", cfg.identity.name, cfg.identity.email);
    println!("site:     {}", cfg.site.as_deref().unwrap_or("(none)"));
    println!("catalog:  {} repo(s) declared to the kernel", cfg.catalog().0.len());

    for (name, repo) in &cfg.repos {
        if only.is_some_and(|want| want != name) {
            continue;
        }
        println!("\n[{name}]");
        println!("  github:              {}", opt(repo.github.as_deref()));
        println!(
            "  template:            {}",
            repo.template
                .as_deref()
                .map_or_else(|| "(none)".to_string(), |p| p.display().to_string())
        );
        println!("  verify:              {}", opt(repo.verify.as_deref()));
        println!("  keep_region_markers: {}", repo.keep_region_markers);
        println!("  links.blob:          {}", opt(repo.links.blob.as_deref()));
    }
}

fn opt(v: Option<&str>) -> &str {
    v.unwrap_or("(none)")
}
