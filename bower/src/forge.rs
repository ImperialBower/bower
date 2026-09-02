//! The network, behind one door.
//!
//! Everything `bower push` does that reaches outside this machine goes through
//! [`Forge`]. The decisions — the safety gate, the preconditions, what gets
//! pushed — are made against the trait and tested against a fake, so only the
//! last inch is untested.

use std::fmt;
use std::path::Path;

/// What is at a remote before we touch it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemoteState {
    /// No such repository. Nothing to destroy.
    Absent,
    /// The repository exists but holds nothing. Nothing to destroy.
    Empty,
    /// The repository holds commits. Whether they are ours is the whole
    /// question `push`'s gate answers.
    HasContent,
}

/// What a push moved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PushOutcome {
    pub commits: usize,
    pub tags: usize,
}

#[derive(Debug)]
pub enum ForgeError {
    /// A required tool is not installed.
    MissingTool { tool: String, install: String },
    /// The remote could not be reached, or refused us.
    Unreachable { repo: String, reason: String },
    /// The command ran and failed.
    Failed { what: String, stderr: String },
}

impl fmt::Display for ForgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTool { tool, install } => {
                write!(f, "`{tool}` is not installed; try: {install}")
            }
            Self::Unreachable { repo, reason } => {
                write!(f, "cannot reach {repo}: {reason}")
            }
            Self::Failed { what, stderr } => write!(f, "{what} failed: {stderr}"),
        }
    }
}

impl std::error::Error for ForgeError {}

/// A place a generated repository can be published to.
pub trait Forge {
    /// Whether `owner/name` exists, and whether it holds anything.
    ///
    /// # Errors
    ///
    /// If the remote cannot be reached. An unknown remote is never assumed
    /// absent: that assumption is how a guard becomes a hazard.
    fn probe(&self, repo: &str) -> Result<RemoteState, ForgeError>;

    /// The remote's `STEPS.md` at its default branch, if it has one.
    ///
    /// # Errors
    ///
    /// If the remote cannot be read. `Ok(None)` means the file is genuinely
    /// absent — a different thing entirely, and the gate treats it so.
    fn read_steps_md(&self, repo: &str) -> Result<Option<String>, ForgeError>;

    /// Create `owner/name`. Only ever called for a remote that does not exist.
    ///
    /// # Errors
    ///
    /// If creation fails or is not permitted.
    fn create(&self, repo: &str, description: &str) -> Result<(), ForgeError>;

    /// Force-push-with-lease `dir`'s `branch` and every tag to `owner/name`.
    ///
    /// # Errors
    ///
    /// If the push is rejected or the tooling fails.
    fn push(&self, dir: &Path, repo: &str, branch: &str) -> Result<PushOutcome, ForgeError>;
}

/// A [`Forge`] that talks to nothing, and remembers everything it was asked.
///
/// Public, not `#[cfg(test)]`: it is the controllability half of this boundary,
/// the same way `bower-testkit` is for the kernel. Integration tests need it as
/// much as unit tests do, and the assertion that matters most in this crate —
/// that `push` was never *called* — can only be made by something that counts
/// calls.
pub struct FakeForge {
    pub state: RemoteState,
    pub steps_md: Option<String>,
    /// When set, `probe` and `read_steps_md` fail with this reason. An
    /// unreachable remote must never be mistaken for an empty one.
    pub unreachable: Option<String>,
    calls: std::cell::RefCell<Vec<String>>,
}

impl FakeForge {
    #[must_use]
    pub fn new(state: RemoteState, steps_md: Option<String>) -> Self {
        Self {
            state,
            steps_md,
            unreachable: None,
            calls: std::cell::RefCell::new(Vec::new()),
        }
    }

    #[must_use]
    pub fn unreachable(reason: &str) -> Self {
        Self {
            state: RemoteState::HasContent,
            steps_md: None,
            unreachable: Some(reason.to_string()),
            calls: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// Every method called on this forge, in order.
    #[must_use]
    pub fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }

    /// Whether anything that changes the remote was called.
    #[must_use]
    pub fn mutated(&self) -> bool {
        self.calls()
            .iter()
            .any(|c| c.starts_with("push") || c.starts_with("create"))
    }

    fn record(&self, what: &str) {
        self.calls.borrow_mut().push(what.to_string());
    }

    fn reachable(&self, repo: &str) -> Result<(), ForgeError> {
        match &self.unreachable {
            Some(reason) => Err(ForgeError::Unreachable {
                repo: repo.to_string(),
                reason: reason.clone(),
            }),
            None => Ok(()),
        }
    }
}

impl Forge for FakeForge {
    fn probe(&self, repo: &str) -> Result<RemoteState, ForgeError> {
        self.record("probe");
        self.reachable(repo)?;
        Ok(self.state)
    }

    fn read_steps_md(&self, repo: &str) -> Result<Option<String>, ForgeError> {
        self.record("read_steps_md");
        self.reachable(repo)?;
        Ok(self.steps_md.clone())
    }

    fn create(&self, _repo: &str, _description: &str) -> Result<(), ForgeError> {
        self.record("create");
        Ok(())
    }

    fn push(&self, _dir: &Path, _repo: &str, _branch: &str) -> Result<PushOutcome, ForgeError> {
        self.record("push");
        Ok(PushOutcome {
            commits: 0,
            tags: 0,
        })
    }
}

/// GitHub, reached through `git` and `gh`.
///
/// Shelling out rather than linking a network stack is deliberate. `git`'s
/// credential handling is battle-tested and already configured on every machine
/// that has a book to publish, and `gh` already holds a token. Reimplementing
/// either would mean *owning* authentication, and owning authentication badly
/// is how this command's risk gets worse rather than better.
///
/// The cost is two runtime dependencies on binaries rather than crates, checked
/// for before anything else runs. [`Forge`] is the seam that makes a native
/// implementation a later swap rather than a rewrite.
pub struct GitHubForge;

impl GitHubForge {
    /// Confirm `git` and `gh` are installed.
    ///
    /// # Errors
    ///
    /// [`ForgeError::MissingTool`] naming the install command.
    pub fn preflight() -> Result<Self, ForgeError> {
        for (tool, install) in [
            ("git", "xcode-select --install, or your package manager"),
            ("gh", "brew install gh — then `gh auth login`"),
        ] {
            let found = std::process::Command::new(tool)
                .arg("--version")
                .output()
                .is_ok_and(|o| o.status.success());
            if !found {
                return Err(ForgeError::MissingTool {
                    tool: tool.to_string(),
                    install: install.to_string(),
                });
            }
        }
        Ok(Self)
    }
}

/// The HTTPS remote for `owner/name`.
///
/// Pure so it can be tested; the network cannot.
#[must_use]
pub fn remote_url(repo: &str) -> String {
    format!("https://github.com/{repo}.git")
}

/// Turn two API answers into a state.
///
/// `repo_found` is whether `repos/{repo}` returned successfully; `has_commits`
/// whether `repos/{repo}/commits` did. GitHub answers `409 Git Repository is
/// empty` for the second on a repository with no commits, which is a different
/// thing from a repository that is not there.
#[must_use]
pub fn state_from(repo_found: bool, has_commits: bool) -> RemoteState {
    match (repo_found, has_commits) {
        (false, _) => RemoteState::Absent,
        (true, false) => RemoteState::Empty,
        (true, true) => RemoteState::HasContent,
    }
}

fn gh(args: &[&str]) -> Result<(bool, String, String), ForgeError> {
    let out = std::process::Command::new("gh")
        .args(args)
        .output()
        .map_err(|e| ForgeError::Failed {
            what: format!("gh {}", args.join(" ")),
            stderr: e.to_string(),
        })?;
    Ok((
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    ))
}

/// Distinguish "this is not there" from "we could not look".
///
/// A `404` is an answer. Anything else — a `401`, a `403`, a network failure —
/// is a refusal to answer, and must never be reported as absence.
fn is_not_found(stderr: &str) -> bool {
    stderr.contains("404") || stderr.contains("Not Found")
}

impl Forge for GitHubForge {
    fn probe(&self, repo: &str) -> Result<RemoteState, ForgeError> {
        let (found, _, stderr) = gh(&["api", &format!("repos/{repo}"), "--silent"])?;
        if !found && !is_not_found(&stderr) {
            return Err(ForgeError::Unreachable {
                repo: repo.to_string(),
                reason: stderr.trim().to_string(),
            });
        }
        if !found {
            return Ok(RemoteState::Absent);
        }

        let (ok, _, stderr) = gh(&[
            "api",
            &format!("repos/{repo}/commits?per_page=1"),
            "--silent",
        ])?;
        // 409 is GitHub's "the repository is empty" — an answer, not a failure.
        if !ok && !stderr.contains("409") && !is_not_found(&stderr) {
            return Err(ForgeError::Unreachable {
                repo: repo.to_string(),
                reason: stderr.trim().to_string(),
            });
        }
        Ok(state_from(true, ok))
    }

    fn read_steps_md(&self, repo: &str) -> Result<Option<String>, ForgeError> {
        let (ok, body, stderr) = gh(&[
            "api",
            "-H",
            "Accept: application/vnd.github.raw",
            &format!("repos/{repo}/contents/STEPS.md"),
        ])?;
        if ok {
            return Ok(Some(body));
        }
        if is_not_found(&stderr) {
            return Ok(None);
        }
        Err(ForgeError::Unreachable {
            repo: repo.to_string(),
            reason: stderr.trim().to_string(),
        })
    }

    fn create(&self, repo: &str, description: &str) -> Result<(), ForgeError> {
        let (ok, _, stderr) = gh(&["repo", "create", repo, "--public", "-d", description])?;
        if ok {
            Ok(())
        } else {
            Err(ForgeError::Failed {
                what: format!("gh repo create {repo}"),
                stderr: stderr.trim().to_string(),
            })
        }
    }

    fn push(&self, dir: &Path, repo: &str, branch: &str) -> Result<PushOutcome, ForgeError> {
        let url = remote_url(repo);
        let run = |args: Vec<String>| -> Result<(bool, String), ForgeError> {
            let out = std::process::Command::new("git")
                .arg("-C")
                .arg(dir)
                .args(&args)
                .output()
                .map_err(|e| ForgeError::Failed {
                    what: format!("git {}", args.join(" ")),
                    stderr: e.to_string(),
                })?;
            Ok((
                out.status.success(),
                String::from_utf8_lossy(&out.stderr).into_owned(),
            ))
        };

        // The branch, with a lease: refuse if the remote moved under us.
        let refspec = format!("{branch}:{branch}");
        let (ok, stderr) = run(vec![
            "push".into(),
            "--force-with-lease".into(),
            url.clone(),
            refspec,
        ])?;
        if !ok {
            return Err(ForgeError::Failed {
                what: "git push --force-with-lease".into(),
                stderr: stderr.trim().to_string(),
            });
        }

        // Tags, plainly forced. A replay recreates every tag with the same name
        // and a new SHA, and the gate has already established that this remote
        // is ours — a lease on a tag we always rewrite would only ever say no.
        let (ok, stderr) = run(vec!["push".into(), "--force".into(), url, "--tags".into()])?;
        if !ok {
            return Err(ForgeError::Failed {
                what: "git push --force --tags".into(),
                stderr: stderr.trim().to_string(),
            });
        }

        Ok(PushOutcome {
            commits: count(dir, &["rev-list", "--count", "HEAD"]),
            tags: count(dir, &["tag", "--list"]),
        })
    }
}

/// Best-effort count for the report. A wrong count is cosmetic; refusing to
/// report a successful push because counting failed would not be.
fn count(dir: &Path, args: &[&str]) -> usize {
    std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map_or(0, |o| {
            let text = String::from_utf8_lossy(&o.stdout);
            let trimmed = text.trim();
            trimmed
                .parse::<usize>()
                .unwrap_or_else(|_| trimmed.lines().count())
        })
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod forge_tests {
    use super::*;

    #[test]
    fn remote_url__is_https() {
        assert_eq!(
            remote_url("ImperialBower/hello-playbook"),
            "https://github.com/ImperialBower/hello-playbook.git"
        );
    }

    #[test]
    fn state_from__separates_absent_from_empty() {
        // GitHub answers 409 for an empty repository's commits, which is not
        // the same as the repository not being there.
        assert_eq!(state_from(false, false), RemoteState::Absent);
        assert_eq!(state_from(true, false), RemoteState::Empty);
        assert_eq!(state_from(true, true), RemoteState::HasContent);
    }

    #[test]
    fn is_not_found__only_for_404() {
        assert!(is_not_found("gh: Not Found (HTTP 404)"));
        // A 401 means we could not look. Reporting that as absence would let
        // an expired token look like an empty remote, and an empty remote is
        // safe to overwrite.
        assert!(!is_not_found("gh: HTTP 401: Bad credentials"));
        assert!(!is_not_found("dial tcp: lookup github.com: no such host"));
    }
}
