//! `bower verify` — checking that the book's declared failures actually happen.
//!
//! Every step carries an `Expect` (`bower-core/src/directive.rs:92`). Until now
//! that was recorded and believed. This module runs a compiler against each
//! step's tree and compares what happened to what the book claimed.
//!
//! Verification reads the **plan**, not a git repository: the kernel already
//! hands over a complete `TreeState` per step, so `verify` works before `build`
//! has ever run and never touches `gix`.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

use bower_core::prelude::{Expect, Location, RepoPlan, StepId};

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
    /// Kept for the failure report. Never printed when a step is upheld.
    pub stderr: String,
}

/// A step's claim, measured against what actually happened.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Verdict {
    /// The step behaved as declared.
    Upheld,
    /// It did not. Carries enough to go and fix the book: what happened, the
    /// command that showed it, and that command's stderr.
    Broken {
        happened: String,
        command: String,
        stderr: String,
    },
    /// `expect="none"`.
    Skipped,
}

#[derive(Clone, Debug)]
pub struct StepVerdict {
    pub seq: usize,
    pub id: StepId,
    pub anchor: Location,
    pub expect: Expect,
    pub verdict: Verdict,
}

#[derive(Debug)]
pub struct VerifyReport {
    pub repo: String,
    pub verdicts: Vec<StepVerdict>,
}

impl VerifyReport {
    /// Every step whose claim did not hold.
    pub fn broken(&self) -> impl Iterator<Item = &StepVerdict> {
        self.verdicts
            .iter()
            .filter(|v| matches!(v.verdict, Verdict::Broken { .. }))
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
        }
    }
}

impl std::error::Error for VerifyError {}

pub struct Verifier<'a> {
    pub config: &'a BookConfig,
    pub book_root: &'a Path,
    /// Scratch directory. Rewritten for every step.
    pub work_dir: &'a Path,
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

        // One target directory across every step, so twenty steps are not
        // twenty cold builds.
        let target_dir = self.work_dir.join("target");
        let tree_dir = self.work_dir.join("tree");

        let mut verdicts = Vec::new();
        for step in plan.steps.iter().skip(start) {
            if only.is_some_and(|want| want != step.id.0) {
                continue;
            }

            let verdict = if step.expect == Expect::Skip {
                Verdict::Skipped
            } else {
                let mut blobs = scaffolding.clone();
                blobs.extend(blobs_of(&step.tree));
                write_tree_to_disk(&tree_dir, &blobs).map_err(|source| VerifyError::Io {
                    path: tree_dir.clone(),
                    source,
                })?;

                let check = run(check_cmd, &tree_dir, &target_dir)?;
                let verify = if check.success && step.expect != Expect::CompileFail {
                    Some(run(verify_cmd, &tree_dir, &target_dir)?)
                } else {
                    None
                };
                verdict_for(step.expect, &check, verify.as_ref())
            };

            verdicts.push(StepVerdict {
                seq: step.seq,
                id: step.id.clone(),
                anchor: step.anchor.clone(),
                expect: step.expect,
                verdict,
            });
        }

        Ok(VerifyReport {
            repo: repo_name,
            verdicts,
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
        stderr: o.stderr.clone(),
    };

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

/// Run one command in `dir`, with a shared cargo target directory.
///
/// The command is split on whitespace, not handed to a shell. A book needing a
/// pipeline should put it in a script, the way `bin/security-scan` already does.
fn run(command: &str, dir: &Path, target_dir: &Path) -> Result<Outcome, VerifyError> {
    let mut parts = command.split_whitespace();
    let Some(program) = parts.next() else {
        return Err(VerifyError::EmptyCommand);
    };

    let output = Command::new(program)
        .args(parts)
        .current_dir(dir)
        .env("CARGO_TARGET_DIR", target_dir)
        .output()
        .map_err(|source| VerifyError::Io {
            path: PathBuf::from(program),
            source,
        })?;

    if !output.status.success() && nested_workspace(&String::from_utf8_lossy(&output.stderr)) {
        return Err(VerifyError::NestedWorkspace {
            tree: dir.to_path_buf(),
        });
    }

    Ok(Outcome {
        command: command.to_string(),
        success: output.status.success(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod verify_tests {
    use super::*;

    fn ok(cmd: &str) -> Outcome {
        Outcome {
            command: cmd.to_string(),
            success: true,
            stderr: String::new(),
        }
    }

    fn bad(cmd: &str) -> Outcome {
        Outcome {
            command: cmd.to_string(),
            success: false,
            stderr: "error[E0308]: mismatched types\n".to_string(),
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
    fn matrix__broken_carries_the_stderr() {
        match verdict_for(Expect::Pass, &bad("check"), None) {
            Verdict::Broken {
                stderr, command, ..
            } => {
                assert!(stderr.contains("E0308"));
                assert_eq!(command, "check", "the report must name the command");
            }
            other => panic!("expected Broken, got {other:?}"),
        }
    }

    #[test]
    fn run__reports_success_and_failure() {
        let dir = std::env::temp_dir();
        let target = dir.join("bower-verify-target");
        assert!(run("true", &dir, &target).unwrap().success);
        assert!(!run("false", &dir, &target).unwrap().success);
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
            run("   ", &dir, &dir),
            Err(VerifyError::EmptyCommand)
        ));
    }
}
