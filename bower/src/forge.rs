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

    /// Whether `branch` exists on `owner/name`, and whether it holds anything.
    ///
    /// Distinct from [`Forge::probe`], which answers about the repository. A
    /// site branch can be absent on a repository full of code, and *absent* is
    /// safe to create while *present without a marker* must be refused —
    /// collapsing the two is how a guard stops guarding.
    ///
    /// # Errors
    ///
    /// If the remote cannot be reached. Never assume absence from silence.
    fn probe_branch(&self, repo: &str, branch: &str) -> Result<RemoteState, ForgeError>;

    /// The `.bower-site` marker on `branch`, if it has one.
    ///
    /// # Errors
    ///
    /// If the remote cannot be read. `Ok(None)` means the file is genuinely
    /// absent, which the gate treats very differently from a failure to look.
    fn read_site_marker(&self, repo: &str, branch: &str) -> Result<Option<String>, ForgeError>;

    /// Publish `dir`'s files as the entire content of `branch`.
    ///
    /// The rendered book is a plain folder, not a git repository, and has no
    /// history worth keeping — each publish replaces it wholesale, exactly as a
    /// replayed repo replaces its own. Implemented as a single orphan commit,
    /// so the site branch never accumulates a thousand commits of regenerated
    /// HTML that cost clone time and tell nobody anything.
    ///
    /// # Errors
    ///
    /// If the directory cannot be staged or the push is rejected.
    fn push_tree(&self, dir: &Path, repo: &str, branch: &str) -> Result<PushOutcome, ForgeError>;

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
    /// The site branch's state, independent of the repository's.
    pub site_state: RemoteState,
    /// The site branch's `.bower-site`, if it has one.
    pub site_marker: Option<String>,
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
            site_state: RemoteState::Absent,
            site_marker: None,
            unreachable: None,
            calls: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// The same fake, with a site branch in the given state.
    #[must_use]
    pub fn with_site(mut self, site_state: RemoteState, site_marker: Option<String>) -> Self {
        self.site_state = site_state;
        self.site_marker = site_marker;
        self
    }

    #[must_use]
    pub fn unreachable(reason: &str) -> Self {
        Self {
            state: RemoteState::HasContent,
            steps_md: None,
            site_state: RemoteState::HasContent,
            site_marker: None,
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

    fn probe_branch(&self, repo: &str, _branch: &str) -> Result<RemoteState, ForgeError> {
        self.record("probe_branch");
        self.reachable(repo)?;
        Ok(self.site_state)
    }

    fn read_site_marker(&self, repo: &str, _branch: &str) -> Result<Option<String>, ForgeError> {
        self.record("read_site_marker");
        self.reachable(repo)?;
        Ok(self.site_marker.clone())
    }

    fn create(&self, _repo: &str, _description: &str) -> Result<(), ForgeError> {
        self.record("create");
        Ok(())
    }

    fn push_tree(
        &self,
        _dir: &Path,
        _repo: &str,
        _branch: &str,
    ) -> Result<PushOutcome, ForgeError> {
        self.record("push_tree");
        Ok(PushOutcome {
            commits: 1,
            tags: 0,
        })
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

    fn probe_branch(&self, repo: &str, branch: &str) -> Result<RemoteState, ForgeError> {
        let (found, _, stderr) = gh(&[
            "api",
            &format!("repos/{repo}/branches/{branch}"),
            "--silent",
        ])?;
        if !found && !is_not_found(&stderr) {
            return Err(ForgeError::Unreachable {
                repo: format!("{repo}#{branch}"),
                reason: stderr.trim().to_string(),
            });
        }
        // A branch that exists always points at a commit, so there is no
        // "empty branch" to distinguish here.
        Ok(if found {
            RemoteState::HasContent
        } else {
            RemoteState::Absent
        })
    }

    fn read_site_marker(&self, repo: &str, branch: &str) -> Result<Option<String>, ForgeError> {
        let (ok, body, stderr) = gh(&[
            "api",
            "-H",
            "Accept: application/vnd.github.raw",
            &format!("repos/{repo}/contents/.bower-site?ref={branch}"),
        ])?;
        if ok {
            return Ok(Some(body));
        }
        if is_not_found(&stderr) {
            return Ok(None);
        }
        Err(ForgeError::Unreachable {
            repo: format!("{repo}#{branch}"),
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

    fn push_tree(&self, dir: &Path, repo: &str, branch: &str) -> Result<PushOutcome, ForgeError> {
        let fail = |what: &str, stderr: String| ForgeError::Failed {
            what: what.to_string(),
            stderr,
        };

        // Staged in a scratch copy so no `.git` is ever left inside the book's
        // own rendered output.
        let staging = std::env::temp_dir().join(format!("bower-site-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&staging);
        copy_tree(dir, &staging).map_err(|e| fail("staging the site", e.to_string()))?;

        let git = |args: &[&str]| -> Result<(bool, String), ForgeError> {
            let out = std::process::Command::new("git")
                .arg("-C")
                .arg(&staging)
                .args(args)
                .output()
                .map_err(|e| fail(&format!("git {}", args.join(" ")), e.to_string()))?;
            Ok((
                out.status.success(),
                String::from_utf8_lossy(&out.stderr).into_owned(),
            ))
        };

        for args in [vec!["init", "-q", "-b", branch], vec!["add", "-A"]] {
            let (ok, stderr) = git(&args)?;
            if !ok {
                return Err(fail(&format!("git {}", args.join(" ")), stderr));
            }
        }

        // The same fixed identity every generated commit uses, so a site
        // commit is not stamped with whoever happened to run the command.
        let (ok, stderr) = git(&[
            "-c",
            "user.name=bower",
            "-c",
            "user.email=bower@invalid",
            "commit",
            "-q",
            "-m",
            "docs: rendered book",
        ])?;
        if !ok {
            return Err(fail("git commit", stderr));
        }

        // Force, without a lease: an orphan commit shares no history with what
        // is there, so a lease could only ever say no. The gate has already
        // established that this branch is ours.
        let (ok, stderr) = git(&["push", "--force", &remote_url(repo), branch])?;
        if !ok {
            return Err(fail("git push --force (site)", stderr));
        }

        let files = count_files(&staging);
        let _ = std::fs::remove_dir_all(&staging);
        Ok(PushOutcome {
            commits: 1,
            tags: files,
        })
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

/// Recursively copy `from` into `to`, skipping any `.git` already present.
fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let path = entry?.path();
        let Some(name) = path.file_name() else {
            continue;
        };
        if name == ".git" {
            continue;
        }
        let target = to.join(name);
        if path.is_dir() {
            copy_tree(&path, &target)?;
        } else {
            std::fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

fn count_files(dir: &Path) -> usize {
    let mut n = 0;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.file_name().is_some_and(|f| f == ".git") {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
            } else {
                n += 1;
            }
        }
    }
    n
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
