//! `bower verify` — checking that the book's declared failures actually happen.
//!
//! Every step carries an `Expect` (`bower-core/src/directive.rs:92`). Until now
//! that was recorded and believed. This module runs a compiler against each
//! step's tree and compares what happened to what the book claimed.
//!
//! Verification reads the **plan**, not a git repository: the kernel already
//! hands over a complete `TreeState` per step, so `verify` works before `build`
//! has ever run and never touches `gix`.

use std::fmt::{self, Write as _};
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, mpsc};
use std::time::Duration;

use bower_core::prelude::{
    Capture, CapturedOutput, Drift, Expect, Location, PlannedStep, RepoPlan, Scrub, StepId, drift,
    normalize,
};

use crate::config::{BookConfig, DEFAULT_CHECK, DEFAULT_VERIFY};
use crate::materialize::{Blobs, blobs_of, read_dir_recursive, write_tree_to_disk};

/// Where `bower verify` writes its scratch trees when `--work` is not given.
///
/// Under the system temp directory rather than the book's `target/`, because
/// most books that teach Rust *are* cargo workspaces, and a scratch package
/// written inside one is a package cargo refuses to build. The old default,
/// `target/bower-verify`, reported all twenty of the sample book's true claims
/// as false for exactly that reason — and no test caught it, because every test
/// passes `--work` explicitly.
///
/// Keyed by the book's directory name so two books do not share, and thrash,
/// one cargo cache.
#[must_use]
pub fn default_work_dir(book_root: &Path) -> PathBuf {
    let name = book_root
        .canonicalize()
        .ok()
        .as_deref()
        .and_then(|p| p.file_name().map(std::ffi::OsStr::to_os_string))
        .unwrap_or_else(|| "book".into());
    std::env::temp_dir().join("bower-verify").join(name)
}

/// The result of running one command in one tree.
#[derive(Clone, Debug)]
pub struct Outcome {
    pub command: String,
    pub success: bool,
    /// The command ran past its deadline and was killed. `success` is then
    /// false, and no claim can be upheld by it.
    pub timed_out: bool,
    /// Both streams, interleaved in the order they were written: what a
    /// reader's terminal shows. The failure report prints its tail, and a
    /// recorded output is judged against it (EPIC-11 Decision 2).
    pub output: String,
}

/// A step's claim, measured against what actually happened.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Verdict {
    /// The step behaved as declared.
    Upheld,
    /// It did not. Carries enough to go and fix the book: what happened, the
    /// command that showed it, and what that command printed.
    Broken {
        happened: String,
        command: String,
        output: String,
    },
    /// `expect="none"`.
    Skipped,
}

/// What became of one recorded output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutputResult {
    /// The fence holds what the command printed.
    Matches,
    /// The fence is empty: the directive is written and `--record` has not
    /// run yet. Fails `verify` (EPIC-11 Decision 7).
    NotRecorded,
    /// The fence and the live output disagree, first at this line.
    Drifted(Drift),
    /// The step's claim did not hold, or the command never ran, so the output
    /// was not compared (Decision 6).
    Unjudged,
}

/// One output block, judged.
#[derive(Clone, Debug)]
pub struct OutputVerdict {
    /// The block's directive: where the report points and `--record` writes.
    pub loc: Location,
    pub capture: Capture,
    /// What the command printed, normalized. Empty when unjudged.
    pub live: Vec<String>,
    pub result: OutputResult,
}

/// Judge one recorded output against what its command printed. Pure, so the
/// whole table is tested without a compiler.
#[must_use]
pub fn judge_output(
    recorded: &CapturedOutput,
    verdict: &Verdict,
    ran: Option<&Outcome>,
    scrub: &Scrub,
) -> OutputVerdict {
    let (Verdict::Upheld, Some(outcome)) = (verdict, ran) else {
        return OutputVerdict {
            loc: recorded.loc.clone(),
            capture: recorded.capture,
            live: Vec::new(),
            result: OutputResult::Unjudged,
        };
    };
    let live = normalize(&outcome.output, scrub);
    let result = if recorded.lines.is_empty() {
        OutputResult::NotRecorded
    } else {
        drift(&recorded.lines, &live).map_or(OutputResult::Matches, OutputResult::Drifted)
    };
    OutputVerdict {
        loc: recorded.loc.clone(),
        capture: recorded.capture,
        live,
        result,
    }
}

/// The machine's paths a capture must not keep: the scratch tree, the target
/// directory, and cargo's home — each as given and as canonicalized, since
/// cargo prints the canonical spelling (EPIC-11 Decision 4).
#[must_use]
pub fn scrub_for(tree_dir: &Path, target_dir: &Path, cargo_home: Option<&Path>) -> Scrub {
    fn spellings(dir: &Path) -> Vec<String> {
        let mut out = vec![dir.display().to_string()];
        if let Ok(c) = dir.canonicalize() {
            let c = c.display().to_string();
            if !out.contains(&c) {
                out.push(c);
            }
        }
        out
    }
    let mut pairs = Vec::new();
    for tree in spellings(tree_dir) {
        pairs.push((format!("{tree}/"), String::new()));
        pairs.push((tree, ".".to_string()));
    }
    for target in spellings(target_dir) {
        pairs.push((target, "target".to_string()));
    }
    if let Some(home) = cargo_home {
        for h in spellings(home) {
            pairs.push((h, "$CARGO_HOME".to_string()));
        }
    }
    Scrub(pairs)
}

/// Where cargo keeps registry sources: `$CARGO_HOME`, else `~/.cargo`. A
/// diagnostic inside a dependency names a path under it.
fn cargo_home() -> Option<PathBuf> {
    std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cargo")))
}

#[derive(Clone, Debug)]
pub struct StepVerdict {
    pub seq: usize,
    pub id: StepId,
    pub anchor: Location,
    pub expect: Expect,
    pub verdict: Verdict,
    /// This step's output blocks, judged, in document order.
    pub outputs: Vec<OutputVerdict>,
}

#[derive(Debug)]
pub struct VerifyReport {
    pub repo: String,
    pub verdicts: Vec<StepVerdict>,
    /// The first step that records output while nothing pins its toolchain:
    /// its tree has no `rust-toolchain.toml` and the repo sets no `toolchain`.
    pub unpinned_step: Option<StepId>,
}

impl VerifyReport {
    /// Every step whose claim did not hold.
    pub fn broken(&self) -> impl Iterator<Item = &StepVerdict> {
        self.verdicts
            .iter()
            .filter(|v| matches!(v.verdict, Verdict::Broken { .. }))
    }

    /// Every judged output, with the step it belongs to.
    pub fn outputs(&self) -> impl Iterator<Item = (&StepVerdict, &OutputVerdict)> {
        self.verdicts
            .iter()
            .flat_map(|v| v.outputs.iter().map(move |o| (v, o)))
    }
}

#[derive(Debug)]
pub enum VerifyError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    EmptyCommand,
    UnknownStep(String),
    /// The scratch tree landed inside another cargo workspace, so cargo refused
    /// to build it before compiling a line.
    ///
    /// Its own variant because the alternative is worse than an error: cargo
    /// fails, every step's check fails with it, and the report says twenty true
    /// claims are false. A verifier that cannot run must say so, not blame the
    /// book.
    NestedWorkspace {
        tree: PathBuf,
    },
    /// The toolchain a step's tree pins could not be made ready: rustup failed
    /// to install it. Its own variant for the same reason as
    /// `NestedWorkspace` — a failed download is not a compile error in the
    /// book.
    Toolchain {
        tree: PathBuf,
        output: String,
    },
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::EmptyCommand => write!(f, "a check or verify command is empty"),
            Self::UnknownStep(id) => write!(f, "no step named `{id}`"),
            Self::NestedWorkspace { tree } => write!(
                f,
                "the scratch tree at {} is inside another cargo workspace, so \
                 cargo refuses to build it; pass `--work` a directory outside \
                 every Cargo.toml above it",
                tree.display()
            ),
            Self::Toolchain { tree, output } => {
                write!(
                    f,
                    "the Rust toolchain the tree at {} pins could not be made ready \
                     (`{TOOLCHAIN_PROBE}` failed); rustup said:",
                    tree.display()
                )?;
                let tail: Vec<&str> = output.lines().rev().take(8).collect();
                for line in tail.iter().rev() {
                    write!(f, "\n    {line}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for VerifyError {}

/// The command that asks rustup for a step's toolchain before anything whose
/// output is kept. Any proxied command would do; this one prints one line and
/// compiles nothing.
const TOOLCHAIN_PROBE: &str = "rustc --version";

/// How long one command may run when the repo sets no `timeout`. Generous: a
/// recorded step builds with one job, and a cold build of a crate with
/// dependencies takes minutes. A hung test is caught, just not quickly.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(600);

/// What stays the same from one step to the next.
struct Bench<'a> {
    check_cmd: &'a str,
    verify_cmd: &'a str,
    toolchain: Option<&'a str>,
    /// Run before a pinned step's commands: [`TOOLCHAIN_PROBE`], except in
    /// tests.
    probe: &'a str,
    scaffolding: &'a Blobs,
    cargo_home: Option<PathBuf>,
    /// Held across the toolchain probe. Two workers asking rustup for the same
    /// missing toolchain at once would both start installing it into one
    /// `RUSTUP_HOME`, and the loser's rename fails: a verifier fault reported
    /// as `VerifyError::Toolchain`. One probe at a time; the first installs,
    /// the rest find it ready.
    probe_lock: Mutex<()>,
    /// How long one command may run before it is killed.
    timeout: Duration,
}

/// One step, verified: its claim, its outputs, and whether it records output
/// with nothing pinning its toolchain.
struct StepRun {
    verdict: Verdict,
    outputs: Vec<OutputVerdict>,
    unpinned: bool,
}

/// Write one step's tree, run its commands, and judge its claim and its
/// outputs. A step that records output runs with the serial trio (Decision 12).
///
/// `target_dir` belongs to the calling worker alone: every step builds the
/// same package name, so two steps sharing one target directory write the
/// same binary, and one can run the other's.
fn run_step(
    bench: &Bench,
    step: &PlannedStep,
    tree_dir: &Path,
    target_dir: &Path,
) -> Result<StepRun, VerifyError> {
    if step.expect == Expect::Skip {
        return Ok(StepRun {
            verdict: Verdict::Skipped,
            outputs: Vec::new(),
            unpinned: false,
        });
    }
    let mut blobs = bench.scaffolding.clone();
    blobs.extend(blobs_of(&step.tree));
    write_tree_to_disk(tree_dir, &blobs).map_err(|source| VerifyError::Io {
        path: tree_dir.to_path_buf(),
        source,
    })?;

    let records = !step.outputs.is_empty();
    let env = step_env(&blobs, bench.toolchain, records);
    // rustup installs a pinned toolchain the first time anything asks for it,
    // and says so on the asking command's stderr. Ask first, with a command
    // whose output nobody keeps: the install notes never reach a recording,
    // and a failed install is the verifier's problem, not the book's.
    if bench.toolchain.is_some() || pins_toolchain(&blobs) {
        let probe = {
            let _one_at_a_time = bench
                .probe_lock
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            run(bench.probe, tree_dir, target_dir, &env, bench.timeout)?
        };
        if !probe.success {
            return Err(VerifyError::Toolchain {
                tree: tree_dir.to_path_buf(),
                output: probe.output,
            });
        }
    }
    let check = run(bench.check_cmd, tree_dir, target_dir, &env, bench.timeout)?;
    let verify = if check.success && step.expect != Expect::CompileFail {
        Some(run(
            bench.verify_cmd,
            tree_dir,
            target_dir,
            &env,
            bench.timeout,
        )?)
    } else {
        None
    };
    let verdict = verdict_for(step.expect, &check, verify.as_ref());

    let scrub = scrub_for(tree_dir, target_dir, bench.cargo_home.as_deref());
    let outputs = step
        .outputs
        .iter()
        .map(|o| {
            let ran = match o.capture {
                Capture::Check => Some(&check),
                Capture::Verify => verify.as_ref(),
            };
            judge_output(o, &verdict, ran, &scrub)
        })
        .collect();

    Ok(StepRun {
        verdict,
        outputs,
        unpinned: records && bench.toolchain.is_none() && !pins_toolchain(&blobs),
    })
}

/// The most steps verified at once by default. Each worker has its own target
/// directory, so this is also how many cold builds a run pays for.
const MAX_DEFAULT_JOBS: usize = 4;

/// How many steps to verify at once when `--jobs` is not given.
#[must_use]
pub fn default_jobs() -> usize {
    std::thread::available_parallelism()
        .map_or(1, std::num::NonZero::get)
        .min(MAX_DEFAULT_JOBS)
}

/// Run every step on a small pool of workers and hand the results back in the
/// order of `steps`, whatever order they finished in.
///
/// One target directory per worker, not per step, so twenty steps are four
/// cold builds rather than twenty; and not one for all, because every step
/// builds the same package name and a step could run the binary another step
/// had just written.
///
/// # Errors
///
/// The error of the earliest step, in document order, that failed to run.
/// No worker takes a new step once one has failed.
fn run_steps(
    bench: &Bench,
    steps: &[&PlannedStep],
    work_dir: &Path,
    jobs: usize,
) -> Result<Vec<StepRun>, VerifyError> {
    let work = Mutex::new(steps.iter().enumerate());
    // Set by the first step that errors. A worker takes no new step after it:
    // the serial loop stopped at the first error, and a missing toolchain
    // should not cost a whole run before it is named.
    let failed = AtomicBool::new(false);
    let (tx, rx) = mpsc::channel();
    std::thread::scope(|scope| {
        for worker in 0..jobs.min(steps.len()) {
            let tx = tx.clone();
            let work = &work;
            let failed = &failed;
            let target_dir = work_dir.join(format!("target-{worker}"));
            scope.spawn(move || {
                while !failed.load(Ordering::Relaxed) {
                    // Its own statement, so the guard drops here: in a
                    // `while let` condition it would live through the body,
                    // and the workers would take turns.
                    let next = work
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .next();
                    let Some((pos, step)) = next else { break };
                    let tree_dir = work_dir.join(format!("tree-{:03}", step.seq));
                    let run = run_step(bench, step, &tree_dir, &target_dir);
                    if run.is_err() {
                        failed.store(true, Ordering::Relaxed);
                    }
                    if tx.send((pos, run)).is_err() {
                        break;
                    }
                }
            });
        }
    });
    drop(tx);
    let mut done: Vec<_> = rx.into_iter().collect();
    done.sort_by_key(|(pos, _)| *pos);
    done.into_iter().map(|(_, run)| run).collect()
}

pub struct Verifier<'a> {
    pub config: &'a BookConfig,
    pub book_root: &'a Path,
    /// Scratch directory: one `tree-NNN` per step and one `target-N` per worker.
    pub work_dir: &'a Path,
    /// How many steps run at once; [`default_jobs`] unless `--jobs` says.
    pub jobs: usize,
}

impl Verifier<'_> {
    /// Verify every step of `plan`, or the slice `only`/`from` selects.
    ///
    /// # Errors
    ///
    /// Returns [`VerifyError`] if the scratch directory cannot be written, a
    /// configured command is empty, or `--step`/`--from` names a step the plan
    /// does not contain.
    pub fn run(
        &self,
        plan: &RepoPlan,
        only: Option<&str>,
        from: Option<&str>,
    ) -> Result<VerifyReport, VerifyError> {
        let repo_name = plan.repo.0.clone();
        let known = |id: &str| plan.steps.iter().any(|s| s.id.0 == id);
        if let Some(id) = only.or(from)
            && !known(id)
        {
            return Err(VerifyError::UnknownStep(id.to_string()));
        }
        let start = from
            .and_then(|id| plan.steps.iter().position(|s| s.id.0 == id))
            .unwrap_or(0);

        let cfg = self.config.repos.get(&repo_name);
        let check_cmd = cfg
            .and_then(|c| c.check.as_deref())
            .unwrap_or(DEFAULT_CHECK);
        let verify_cmd = cfg
            .and_then(|c| c.verify.as_deref())
            .unwrap_or(DEFAULT_VERIFY);

        // The scaffolding a step's tree is incomplete without. A book whose
        // template is broken should fail verification, not be excused from it.
        let scaffolding = match cfg.and_then(|c| c.template.as_ref()) {
            Some(dir) => {
                let path = self.book_root.join(dir);
                read_dir_recursive(&path).map_err(|source| VerifyError::Io { path, source })?
            }
            None => Blobs::new(),
        };

        let bench = Bench {
            check_cmd,
            verify_cmd,
            toolchain: cfg.and_then(|c| c.toolchain.as_deref()),
            probe: TOOLCHAIN_PROBE,
            scaffolding: &scaffolding,
            cargo_home: cargo_home(),
            probe_lock: Mutex::new(()),
            timeout: cfg.and_then(|c| c.timeout).unwrap_or(DEFAULT_TIMEOUT),
        };

        let steps: Vec<&PlannedStep> = plan
            .steps
            .iter()
            .skip(start)
            .filter(|step| only.is_none_or(|want| want == step.id.0))
            .collect();

        let mut verdicts = Vec::with_capacity(steps.len());
        let mut unpinned_step = None;
        // In document order, so the first error reported is the first step's.
        for (step, run) in steps
            .iter()
            .zip(run_steps(&bench, &steps, self.work_dir, self.jobs)?)
        {
            let StepRun {
                verdict,
                outputs,
                unpinned,
            } = run;
            if unpinned && unpinned_step.is_none() {
                unpinned_step = Some(step.id.clone());
            }
            verdicts.push(StepVerdict {
                seq: step.seq,
                id: step.id.clone(),
                anchor: step.anchor.clone(),
                expect: step.expect,
                verdict,
                outputs,
            });
        }

        Ok(VerifyReport {
            repo: repo_name,
            verdicts,
            unpinned_step,
        })
    }
}

/// The matrix from spec § 6, as one pure function.
///
/// `verify` is `None` whenever the verify command was not run — either the
/// check failed, or the step declared `compile_fail` and the check is the whole
/// claim.
#[must_use]
pub fn verdict_for(expect: Expect, check: &Outcome, verify: Option<&Outcome>) -> Verdict {
    let broken = |happened: &str, o: &Outcome| Verdict::Broken {
        happened: happened.to_string(),
        command: o.command.clone(),
        output: o.output.clone(),
    };

    if expect == Expect::Skip {
        return Verdict::Skipped;
    }
    // A killed command exits non-zero, which `compile_fail` and `test_fail`
    // would otherwise read as the failure they promised.
    if let Some(late) = std::iter::once(check).chain(verify).find(|o| o.timed_out) {
        return broken("it did not finish in time", late);
    }

    match expect {
        Expect::Skip => Verdict::Skipped,

        Expect::CompileFail => {
            if check.success {
                broken("it compiled", check)
            } else {
                Verdict::Upheld
            }
        }

        Expect::Pass => {
            if !check.success {
                return broken("it did not compile", check);
            }
            match verify {
                Some(v) if v.success => Verdict::Upheld,
                Some(v) => broken("its tests failed", v),
                None => broken("the verify command did not run", check),
            }
        }

        Expect::TestFail => {
            if !check.success {
                return broken("it did not compile, so its tests never ran", check);
            }
            match verify {
                Some(v) if v.success => broken("its tests passed", v),
                Some(_) => Verdict::Upheld,
                None => broken("the verify command did not run", check),
            }
        }
    }
}

/// Whether a failed command failed because cargo found a workspace above it.
///
/// A substring match on cargo's own wording, kept as its own function so the
/// signature is pinned by a test rather than buried in an `if`. Matching the
/// short middle of the sentence, not the whole line, because the words around
/// it name paths that differ on every machine.
#[must_use]
pub fn nested_workspace(stderr: &str) -> bool {
    stderr.contains("believes it's in a workspace")
}

/// Does this tree carry its own rustup pin at the root?
#[must_use]
pub fn pins_toolchain(blobs: &Blobs) -> bool {
    blobs.contains_key("rust-toolchain.toml") || blobs.contains_key("rust-toolchain")
}

/// The environment a step's commands run with, beyond `CARGO_TARGET_DIR`.
///
/// The toolchain first (EPIC-11 Decision 10): a tree that pins one keeps its
/// pin, and only a tree that pins nothing gets the repo's `toolchain` key.
/// `run` removes the inherited `RUSTUP_TOOLCHAIN` either way, so whatever
/// launched `bower` never decides.
///
/// Then, for a step whose output the book records, the serial trio
/// (Decision 12): one build job and one test thread, so the order of what is
/// printed does not depend on scheduling, and no backtrace, so an author's
/// `RUST_BACKTRACE=1` does not end up in the book.
#[must_use]
pub fn step_env(
    blobs: &Blobs,
    toolchain: Option<&str>,
    records_output: bool,
) -> Vec<(&'static str, String)> {
    let mut env = Vec::new();
    if let Some(t) = toolchain
        && !pins_toolchain(blobs)
    {
        env.push(("RUSTUP_TOOLCHAIN", t.to_string()));
    }
    if records_output {
        env.push(("CARGO_BUILD_JOBS", "1".to_string()));
        env.push(("RUST_TEST_THREADS", "1".to_string()));
        env.push(("RUST_BACKTRACE", "0".to_string()));
    }
    env
}

/// Run one command in `dir`, with a shared cargo target directory.
///
/// The command is split on whitespace, not handed to a shell. A book needing a
/// pipeline should put it in a script, the way `bin/security-scan` already does.
fn run(
    command: &str,
    dir: &Path,
    target_dir: &Path,
    env: &[(&str, String)],
    timeout: Duration,
) -> Result<Outcome, VerifyError> {
    let mut parts = command.split_whitespace();
    let Some(program) = parts.next() else {
        return Err(VerifyError::EmptyCommand);
    };
    let io = |source| VerifyError::Io {
        path: PathBuf::from(program),
        source,
    };

    // One pipe for both streams, so the text keeps the order it was written
    // in: the order a reader's terminal shows (EPIC-11 Decision 2).
    let (mut reader, writer) = std::io::pipe().map_err(io)?;
    let mut cmd = Command::new(program);
    cmd.args(parts)
        .current_dir(dir)
        .env("CARGO_TARGET_DIR", target_dir)
        // rustup's cargo proxy exports its own pin to every child, so a
        // `bower` started by `cargo run` would hand the workspace's toolchain
        // to the tree and override the tree's `rust-toolchain.toml`.
        .env_remove("RUSTUP_TOOLCHAIN")
        .envs(env.iter().map(|(k, v)| (*k, v.as_str())))
        .stdin(Stdio::null())
        .stdout(writer.try_clone().map_err(io)?)
        .stderr(writer);
    // Its own process group, so a timeout can kill what the command started
    // too: cargo's test binary, a script's `sleep`. Killing cargo alone would
    // leave its child holding the pipe open, and the read below waiting.
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut cmd, 0);
    let mut child = cmd.spawn().map_err(io)?;
    // `cmd` still holds both write ends. Until it is gone the read below never
    // sees end-of-file, and waits for ever.
    drop(cmd);
    let drain = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).map(|_| bytes)
    });

    let deadline = std::time::Instant::now() + timeout;
    let (status, timed_out) = loop {
        if let Some(status) = child.try_wait().map_err(io)? {
            break (status, false);
        }
        if std::time::Instant::now() >= deadline {
            kill_group(&mut child);
            break (child.wait().map_err(io)?, true);
        }
        std::thread::sleep(POLL);
    };
    let bytes = drain
        .join()
        .map_err(|_| io(std::io::Error::other("the output reader panicked")))?
        .map_err(io)?;
    let mut output = String::from_utf8_lossy(&bytes).into_owned();
    if timed_out {
        if !output.is_empty() && !output.ends_with('\n') {
            output.push('\n');
        }
        let _ = writeln!(
            output,
            "bower: `{command}` did not finish within {timeout:?} and was killed"
        );
    }

    if !status.success() && nested_workspace(&output) {
        return Err(VerifyError::NestedWorkspace {
            tree: dir.to_path_buf(),
        });
    }

    Ok(Outcome {
        command: command.to_string(),
        success: status.success() && !timed_out,
        timed_out,
        output,
    })
}

/// How often `run` checks whether its command has finished.
const POLL: Duration = Duration::from_millis(20);

/// Kill a timed-out command and everything it started. `run` made it a
/// process-group leader, so its id is also its group's. Shells out to
/// `kill`, not to `libc`, for one call on a path only a hung step reaches.
fn kill_group(child: &mut std::process::Child) {
    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .args(["-KILL", &format!("-{}", child.id())])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    // Whatever the group kill did, the direct child must go, or `wait` hangs.
    let _ = child.kill();
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod verify_tests {
    use super::*;

    /// A deadline no test command comes near.
    const LONG: Duration = Duration::from_secs(60);

    fn ok(cmd: &str) -> Outcome {
        Outcome {
            command: cmd.to_string(),
            success: true,
            timed_out: false,
            output: String::new(),
        }
    }

    fn bad(cmd: &str) -> Outcome {
        Outcome {
            command: cmd.to_string(),
            success: false,
            timed_out: false,
            output: "error[E0308]: mismatched types\n".to_string(),
        }
    }

    #[test]
    fn matrix__pass_requires_both_to_succeed() {
        assert_eq!(
            verdict_for(Expect::Pass, &ok("check"), Some(&ok("test"))),
            Verdict::Upheld
        );
        assert!(matches!(
            verdict_for(Expect::Pass, &bad("check"), None),
            Verdict::Broken { .. }
        ));
        assert!(matches!(
            verdict_for(Expect::Pass, &ok("check"), Some(&bad("test"))),
            Verdict::Broken { .. }
        ));
    }

    #[test]
    fn matrix__compile_fail_requires_check_to_fail() {
        assert_eq!(
            verdict_for(Expect::CompileFail, &bad("check"), None),
            Verdict::Upheld
        );
        let broke = verdict_for(Expect::CompileFail, &ok("check"), None);
        match broke {
            Verdict::Broken { happened, .. } => assert_eq!(happened, "it compiled"),
            other => panic!("expected Broken, got {other:?}"),
        }
    }

    #[test]
    fn matrix__test_fail_requires_check_to_pass_and_verify_to_fail() {
        assert_eq!(
            verdict_for(Expect::TestFail, &ok("check"), Some(&bad("test"))),
            Verdict::Upheld
        );
        // Passing tests are the interesting failure: the book claimed a bug
        // that is no longer there.
        match verdict_for(Expect::TestFail, &ok("check"), Some(&ok("test"))) {
            Verdict::Broken { happened, .. } => assert_eq!(happened, "its tests passed"),
            other => panic!("expected Broken, got {other:?}"),
        }
        // A compile failure is not a test failure, and must not be accepted as
        // one — that would let a typo masquerade as a teaching point.
        assert!(matches!(
            verdict_for(Expect::TestFail, &bad("check"), None),
            Verdict::Broken { .. }
        ));
    }

    #[test]
    fn matrix__none_is_skipped() {
        assert_eq!(
            verdict_for(Expect::Skip, &bad("check"), None),
            Verdict::Skipped
        );
    }

    #[test]
    fn matrix__broken_carries_the_output() {
        match verdict_for(Expect::Pass, &bad("check"), None) {
            Verdict::Broken {
                output, command, ..
            } => {
                assert!(output.contains("E0308"));
                assert_eq!(command, "check", "the report must name the command");
            }
            other => panic!("expected Broken, got {other:?}"),
        }
    }

    #[test]
    fn run__reports_success_and_failure() {
        let dir = std::env::temp_dir();
        let target = dir.join("bower-verify-target");
        assert!(run("true", &dir, &target, &[], LONG).unwrap().success);
        assert!(!run("false", &dir, &target, &[], LONG).unwrap().success);
    }

    #[test]
    fn nested_workspace__recognises_cargos_own_wording() {
        // The exact sentence cargo prints, kept verbatim. If cargo rewords it,
        // this test fails and someone updates the match — which is better than
        // the tool silently going back to blaming the book.
        let stderr = concat!(
            "error: current package believes it's in a workspace when it's not:\n",
            "current:   /tmp/bower-verify/tree/Cargo.toml\n",
            "workspace: /home/me/project/Cargo.toml\n",
        );
        assert!(nested_workspace(stderr));
    }

    #[test]
    fn nested_workspace__does_not_fire_on_an_ordinary_failure() {
        // A real compile error must stay a real compile error: this predicate
        // is what decides whether a step is judged at all.
        assert!(!nested_workspace("error[E0308]: mismatched types"));
        assert!(!nested_workspace(""));
    }

    #[test]
    fn default_work_dir__is_not_inside_the_book() {
        // The regression, stated directly. The old default was
        // `target/bower-verify`, relative to the current directory — which put
        // the scratch package inside whatever workspace the book lives in.
        let book = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let work = default_work_dir(&book);
        let root = book.canonicalize().unwrap();
        assert!(
            !work.starts_with(&root),
            "{} is inside {}",
            work.display(),
            root.display()
        );
    }

    #[test]
    fn default_work_dir__gives_two_books_two_directories() {
        // One shared cache across books would thrash: every switch would be a
        // cold build.
        let a = default_work_dir(Path::new(env!("CARGO_MANIFEST_DIR")));
        let b = default_work_dir(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("bower-core"),
        );
        assert_ne!(a, b);
    }

    #[test]
    fn run__empty_command_is_an_error() {
        let dir = std::env::temp_dir();
        assert!(matches!(
            run("   ", &dir, &dir, &[], LONG),
            Err(VerifyError::EmptyCommand)
        ));
    }

    /// A shell script in its own directory, run the way `run` runs any
    /// command: split on whitespace, no shell of ours in between.
    fn script(case: &str, body: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bower-verify-run-{case}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("s.sh"), body).unwrap();
        dir
    }

    #[test]
    fn run__captures_both_streams_in_order() {
        // A reader's terminal interleaves the two streams in the order they
        // were written. Stderr-then-stdout would print cargo's closing
        // `error: test failed` above libtest's `running 1 test`.
        let dir = script("order", "echo one\necho two >&2\necho three\n");
        let out = run("sh s.sh", &dir, &dir.join("target"), &[], LONG).unwrap();
        assert_eq!(out.output, "one\ntwo\nthree\n");
        assert!(out.success);
    }

    fn blobs(paths: &[&str]) -> Blobs {
        paths
            .iter()
            .map(|p| ((*p).to_string(), (Vec::new(), false)))
            .collect()
    }

    #[test]
    fn toolchain__a_tree_pin_wins() {
        // hello-playbook teaches the pin. Overriding it would verify a
        // different book than the one a reader checks out.
        for pin in ["rust-toolchain.toml", "rust-toolchain"] {
            let env = step_env(&blobs(&["Cargo.toml", pin]), Some("1.98.1"), false);
            assert!(
                !env.iter().any(|(k, _)| *k == "RUSTUP_TOOLCHAIN"),
                "{pin}: {env:?}"
            );
        }
    }

    #[test]
    fn toolchain__the_key_fills_a_tree_without_one() {
        let env = step_env(&blobs(&["Cargo.toml"]), Some("1.98.1"), false);
        assert!(
            env.contains(&("RUSTUP_TOOLCHAIN", "1.98.1".to_string())),
            "{env:?}"
        );
        assert!(step_env(&blobs(&["Cargo.toml"]), None, false).is_empty());
    }

    #[test]
    fn step_env__a_recorded_output_runs_serially() {
        let env = step_env(&blobs(&["Cargo.toml"]), None, true);
        for (key, value) in [
            ("CARGO_BUILD_JOBS", "1"),
            ("RUST_TEST_THREADS", "1"),
            ("RUST_BACKTRACE", "0"),
        ] {
            assert!(env.contains(&(key, value.to_string())), "{env:?}");
        }
    }

    #[test]
    fn run__never_passes_the_inherited_toolchain_on() {
        // Under `cargo test`, rustup's proxy has already exported
        // RUSTUP_TOOLCHAIN into this process. The child must not see it: that
        // value is how `bower` was started, not what the book pins.
        let dir = script("toolchain", "echo \"${RUSTUP_TOOLCHAIN:-unset}\"\n");
        let out = run("sh s.sh", &dir, &dir.join("target"), &[], LONG).unwrap();
        assert_eq!(out.output, "unset\n");

        let set = [("RUSTUP_TOOLCHAIN", "1.98.1".to_string())];
        let out = run("sh s.sh", &dir, &dir.join("target"), &set, LONG).unwrap();
        assert_eq!(out.output, "1.98.1\n");
    }

    fn recorded(lines: &[&str]) -> CapturedOutput {
        CapturedOutput {
            loc: Location::new("src/ch04.md", 12),
            capture: Capture::Check,
            info: "text".to_string(),
            lines: lines.iter().map(ToString::to_string).collect(),
        }
    }

    fn printed(text: &str) -> Outcome {
        Outcome {
            command: "cargo check".to_string(),
            success: false,
            timed_out: false,
            output: text.to_string(),
        }
    }

    const E0308: &str = "    Checking hp v0.1.0 (/scratch/tree)\nerror[E0308]: mismatched types\n --> src/scratch.rs:4:18\n";

    #[test]
    fn output__a_match_upholds_the_step() {
        let v = judge_output(
            &recorded(&["error[E0308]: mismatched types", " --> src/scratch.rs:4:18"]),
            &Verdict::Upheld,
            Some(&printed(E0308)),
            &Scrub::default(),
        );
        assert_eq!(v.result, OutputResult::Matches);
    }

    #[test]
    fn output__drift_fails_verify_and_names_the_line() {
        let v = judge_output(
            &recorded(&["error[E0308]: mismatched types", " --> src/scratch.rs:5:18"]),
            &Verdict::Upheld,
            Some(&printed(E0308)),
            &Scrub::default(),
        );
        assert_eq!(
            v.result,
            OutputResult::Drifted(Drift {
                line: 2,
                recorded: Some(" --> src/scratch.rs:5:18".to_string()),
                live: Some(" --> src/scratch.rs:4:18".to_string()),
            })
        );
    }

    #[test]
    fn output__an_empty_fence_is_not_recorded_and_fails() {
        let v = judge_output(
            &recorded(&[]),
            &Verdict::Upheld,
            Some(&printed(E0308)),
            &Scrub::default(),
        );
        assert_eq!(v.result, OutputResult::NotRecorded);
        assert_eq!(
            v.live.len(),
            2,
            "the live text is what --record would write"
        );
    }

    #[test]
    fn output__a_broken_step_is_unjudged_and_never_recorded() {
        // The broken claim is the headline. A snapshot of the wrong failure
        // is worse than none (EPIC-11 Decision 6).
        let broken = Verdict::Broken {
            happened: "it compiled".to_string(),
            command: "cargo check".to_string(),
            output: String::new(),
        };
        let v = judge_output(
            &recorded(&[]),
            &broken,
            Some(&printed(E0308)),
            &Scrub::default(),
        );
        assert_eq!(v.result, OutputResult::Unjudged);
        assert!(v.live.is_empty());

        // A command that never ran is unjudged too.
        let v = judge_output(&recorded(&[]), &Verdict::Upheld, None, &Scrub::default());
        assert_eq!(v.result, OutputResult::Unjudged);
    }

    #[test]
    fn scrub_for__names_the_tree_the_target_and_cargo_home() {
        let s = scrub_for(
            Path::new("/w/tree"),
            Path::new("/w/target"),
            Some(Path::new("/home/me/.cargo")),
        );
        assert_eq!(
            normalize(
                "error: failed to parse manifest at `/w/tree/Cargo.toml`\nin /w/tree, /w/target/debug, /home/me/.cargo/registry\n",
                &s
            ),
            vec![
                "error: failed to parse manifest at `Cargo.toml`".to_string(),
                "in ., target/debug, $CARGO_HOME/registry".to_string(),
            ]
        );
    }

    #[test]
    fn scrub_for__covers_the_canonical_spelling_too() {
        // macOS: the temp directory is `/var/folders/…`, and cargo prints
        // `/private/var/folders/…`. Both must go.
        let dir = std::env::temp_dir().join("bower-verify-scrub");
        std::fs::create_dir_all(&dir).unwrap();
        let canonical = dir.canonicalize().unwrap().display().to_string();
        let s = scrub_for(&dir, &dir.join("target"), None);
        assert!(
            s.0.iter().any(|(from, to)| from == &canonical && to == "."),
            "{s:?}"
        );
    }

    /// Stands in for rustup's proxy: the first command to run in a fresh
    /// target directory "installs the toolchain" and says so on stderr, the
    /// way rustup does on a machine that lacks the pinned channel. `probe`
    /// succeeds; anything else prints `error: <name>` and fails.
    const FAKE_RUSTUP: &str = concat!(
        "m=\"$CARGO_TARGET_DIR/installed\"\n",
        "if [ ! -e \"$m\" ]; then\n",
        "  mkdir -p \"$CARGO_TARGET_DIR\"\n",
        "  echo 'info: syncing channel updates for 9.9.9' >&2\n",
        "  touch \"$m\"\n",
        "fi\n",
        "echo \"error: $1\"\n",
        "[ \"$1\" = probe ] && exit 0\n",
        "exit 1\n",
    );

    /// One pinned `compile_fail` step with a `check` output, planned for real.
    fn pinned_step() -> PlannedStep {
        use bower_core::prelude::{BookSource, Chapter, RepoCatalog, plan};
        let text = concat!(
            "<!-- bower repo=\"r\" file=\"rust-toolchain.toml\" expect=\"compile_fail\" -->\n",
            "```toml\n[toolchain]\nchannel = \"9.9.9\"\n```\n",
            "<!-- bower repo=\"r\" output=\"check\" -->\n",
            "```text\nerror: check\n```\n",
        );
        let book = BookSource::from_chapters(vec![Chapter::new("ch.md", text)]);
        let p = plan(&book, &RepoCatalog::from_names(&["r"])).unwrap();
        p.repos[0].steps[0].clone()
    }

    fn fake_bench<'a>(
        case: &str,
        probe: &'a str,
        scaffolding: &'a Blobs,
    ) -> (Bench<'a>, PathBuf, PathBuf) {
        let work = std::env::temp_dir().join(format!("bower-verify-probe-{case}"));
        let _ = std::fs::remove_dir_all(&work);
        (
            Bench {
                check_cmd: "sh rustup.sh check",
                verify_cmd: "sh rustup.sh verify",
                toolchain: None,
                probe,
                scaffolding,
                cargo_home: None,
                probe_lock: Mutex::new(()),
                timeout: LONG,
            },
            work.join("tree"),
            work.join("target"),
        )
    }

    fn fake_rustup() -> Blobs {
        Blobs::from([(
            "rustup.sh".to_string(),
            (FAKE_RUSTUP.as_bytes().to_vec(), false),
        )])
    }

    #[test]
    fn run_step__a_toolchain_install_never_reaches_a_recording() {
        // CI, 12 September 2026: the first `cargo check` on a runner without
        // the book's pinned 1.95.0 printed rustup's install notes, and
        // `--record` wrote them into the chapter.
        let scaffolding = fake_rustup();
        let (bench, tree_dir, target_dir) =
            fake_bench("install", "sh rustup.sh probe", &scaffolding);
        let run = run_step(&bench, &pinned_step(), &tree_dir, &target_dir).unwrap();
        assert_eq!(run.verdict, Verdict::Upheld);
        assert_eq!(run.outputs[0].live, vec!["error: check".to_string()]);
        assert_eq!(run.outputs[0].result, OutputResult::Matches);
    }

    #[test]
    fn run_step__a_toolchain_that_will_not_install_is_the_verifiers_fault() {
        // A failed download is not a compile error in the book.
        let scaffolding = fake_rustup();
        let (bench, tree_dir, target_dir) = fake_bench("broken", "sh rustup.sh nope", &scaffolding);
        match run_step(&bench, &pinned_step(), &tree_dir, &target_dir) {
            Err(VerifyError::Toolchain { output, .. }) => {
                assert!(output.contains("info: syncing"), "{output}");
            }
            Err(other) => panic!("expected Toolchain, got {other}"),
            Ok(_) => panic!("expected Toolchain, got a verdict"),
        }
    }

    fn late(cmd: &str) -> Outcome {
        Outcome {
            command: cmd.to_string(),
            success: false,
            timed_out: true,
            output: String::new(),
        }
    }

    #[test]
    fn matrix__a_timeout_upholds_nothing() {
        // A killed command exits non-zero, which is exactly what
        // `compile_fail` and `test_fail` ask for. A step that hangs has not
        // failed the way the book says it does.
        let cases = [
            (Expect::CompileFail, late("check"), None),
            (Expect::TestFail, ok("check"), Some(late("test"))),
            (Expect::Pass, ok("check"), Some(late("test"))),
            (Expect::Pass, late("check"), None),
        ];
        for (expect, check, verify) in cases {
            match verdict_for(expect, &check, verify.as_ref()) {
                Verdict::Broken { happened, .. } => {
                    assert!(
                        happened.contains("did not finish"),
                        "{expect:?}: {happened}"
                    );
                }
                other => panic!("{expect:?}: expected Broken, got {other:?}"),
            }
        }
    }

    #[test]
    fn run__a_command_past_its_deadline_is_killed_and_says_so() {
        // `sleep` is the shell's child, as a test binary is cargo's. Killing
        // only the direct child would leave it holding the pipe open, and the
        // read would wait for it.
        let dir = script("timeout", "echo started\nsleep 30\necho never\n");
        let began = std::time::Instant::now();
        let out = run(
            "sh s.sh",
            &dir,
            &dir.join("target"),
            &[],
            Duration::from_millis(300),
        )
        .unwrap();
        assert!(
            began.elapsed() < Duration::from_secs(10),
            "{:?}",
            began.elapsed()
        );
        assert!(out.timed_out);
        assert!(!out.success);
        assert!(out.output.starts_with("started\n"), "{}", out.output);
        assert!(!out.output.contains("never"), "{}", out.output);
        assert!(
            out.output.contains("did not finish within 300ms"),
            "{}",
            out.output
        );
    }

    #[test]
    fn run__a_command_inside_its_deadline_is_not_timed_out() {
        let dir = script("in-time", "echo done\n");
        let out = run("sh s.sh", &dir, &dir.join("target"), &[], LONG).unwrap();
        assert!(!out.timed_out);
        assert!(out.success);
        assert_eq!(out.output, "done\n");
    }

    #[test]
    fn default_jobs__is_between_one_and_four() {
        assert!((1..=4).contains(&default_jobs()), "{}", default_jobs());
    }

    /// `n` passing steps, one file each, planned for real.
    fn steps(n: usize) -> Vec<PlannedStep> {
        use bower_core::prelude::{BookSource, Chapter, RepoCatalog, plan};
        let mut text = String::new();
        for i in 0..n {
            let _ = write!(
                text,
                "<!-- bower repo=\"r\" file=\"f{i}.txt\" -->\n```text\n{i}\n```\n"
            );
        }
        let book = BookSource::from_chapters(vec![Chapter::new("ch.md", &text)]);
        plan(&book, &RepoCatalog::from_names(&["r"])).unwrap().repos[0]
            .steps
            .clone()
    }

    fn pool_bench<'a>(check_cmd: &'a str, scaffolding: &'a Blobs) -> Bench<'a> {
        Bench {
            check_cmd,
            verify_cmd: "true",
            toolchain: None,
            probe: "true",
            scaffolding,
            cargo_home: None,
            probe_lock: Mutex::new(()),
            timeout: LONG,
        }
    }

    fn fresh(case: &str) -> PathBuf {
        let work = std::env::temp_dir().join(format!("bower-verify-pool-{case}"));
        let _ = std::fs::remove_dir_all(&work);
        std::fs::create_dir_all(&work).unwrap();
        work
    }

    #[test]
    fn run_steps__two_workers_run_two_steps_at_once() {
        // The first cut held the queue's lock for a whole step, and passed
        // every test, because every result is still right when the workers
        // take turns. Only the order of starts and ends shows it.
        let work = fresh("overlap");
        let log = work.join("log");
        let body = format!(
            "echo start >> {log}\nsleep 0.5\necho end >> {log}\n",
            log = log.display()
        );
        let scaffolding = Blobs::from([("s.sh".to_string(), (body.into_bytes(), false))]);
        let bench = pool_bench("sh s.sh", &scaffolding);
        let planned = steps(2);
        let refs: Vec<&PlannedStep> = planned.iter().collect();

        let runs = run_steps(&bench, &refs, &work, 2).unwrap();

        assert_eq!(runs.len(), 2);
        let order = std::fs::read_to_string(&log).unwrap();
        assert_eq!(order, "start\nstart\nend\nend\n");
    }

    #[test]
    fn run_steps__one_job_runs_one_step_at_a_time() {
        let work = fresh("serial");
        let log = work.join("log");
        let body = format!(
            "echo start >> {log}\nsleep 0.1\necho end >> {log}\n",
            log = log.display()
        );
        let scaffolding = Blobs::from([("s.sh".to_string(), (body.into_bytes(), false))]);
        let bench = pool_bench("sh s.sh", &scaffolding);
        let planned = steps(2);
        let refs: Vec<&PlannedStep> = planned.iter().collect();

        run_steps(&bench, &refs, &work, 1).unwrap();

        let order = std::fs::read_to_string(&log).unwrap();
        assert_eq!(order, "start\nend\nstart\nend\n");
    }

    #[test]
    fn run_steps__an_error_stops_the_steps_not_yet_taken() {
        // An empty check command fails every step the same way. The serial
        // loop stopped at the first; so must the pool, or a missing
        // toolchain costs a whole run before it is named.
        let work = fresh("stop");
        let scaffolding = Blobs::new();
        let bench = pool_bench("", &scaffolding);
        let planned = steps(3);
        let refs: Vec<&PlannedStep> = planned.iter().collect();

        let result = run_steps(&bench, &refs, &work, 1);

        assert!(matches!(result, Err(VerifyError::EmptyCommand)));
        let first = work.join(format!("tree-{:03}", planned[0].seq));
        let second = work.join(format!("tree-{:03}", planned[1].seq));
        assert!(first.exists(), "the first step ran");
        assert!(!second.exists(), "the second step must not have been taken");
    }
}
