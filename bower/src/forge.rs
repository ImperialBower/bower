//! The network, behind one door.
//!
//! Everything `bower push` does that reaches outside this machine goes through
//! [`Forge`]. The decisions — the safety gate, the preconditions, what gets
//! pushed — are made against the trait and tested against a fake, so only the
//! last inch is untested.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use crate::schedule::{ForgePr, ForgePrState, MAIN};

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

/// A release to create or refresh, with the files to hang off it.
///
/// A struct rather than five arguments because it crosses the [`Forge`]
/// boundary, and a boundary with five positional strings is one a caller gets
/// wrong exactly once, silently.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Release {
    /// The git tag the release hangs on — `v0.1.0`.
    pub tag: String,
    pub title: String,
    pub notes: String,
    /// Files to attach. Existing files of the same name are replaced.
    pub assets: Vec<std::path::PathBuf>,
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

    /// Every branch head on `owner/name`, as full ref → SHA. Empty for an
    /// empty repository. Read-only; `plan_push` calls it after the gate.
    ///
    /// # Errors
    ///
    /// If the remote cannot be read. An unread remote is never "no heads".
    fn remote_heads(&self, repo: &str) -> Result<BTreeMap<String, String>, ForgeError>;

    /// Every pull request against main, in any state (EPIC-09 Decision 22).
    /// Read-only; `plan_push` calls it after the gate.
    ///
    /// # Errors
    ///
    /// If the forge cannot be read.
    fn pull_requests(&self, repo: &str) -> Result<Vec<ForgePr>, ForgeError>;

    /// Push the commit `src` names in `dir` to the remote ref `dst`, leased
    /// against `lease`, or a plain push when `None`. Returns the SHA pushed,
    /// which the next push of the same ref leases against.
    ///
    /// # Errors
    ///
    /// If `src` does not resolve, or the push is refused — a lease trip
    /// included.
    fn push_ref(
        &self,
        dir: &Path,
        repo: &str,
        src: &str,
        dst: &str,
        lease: Option<&str>,
    ) -> Result<String, ForgeError>;

    /// Push every tag, forced. Returns how many tags `dir` holds.
    ///
    /// # Errors
    ///
    /// If the push is refused.
    fn push_tags(&self, dir: &Path, repo: &str) -> Result<usize, ForgeError>;

    /// Make `branch` the repository's default (EPIC-09 Decision 26). Only
    /// ever scheduled for a remote that had no main.
    ///
    /// # Errors
    ///
    /// If the forge refuses.
    fn set_default_branch(&self, repo: &str, branch: &str) -> Result<(), ForgeError>;

    /// Open a PR from `branch` into main. Returns its number.
    ///
    /// # Errors
    ///
    /// If the forge refuses — for instance, a branch with no commits ahead.
    fn open_pull_request(
        &self,
        repo: &str,
        branch: &str,
        title: &str,
        body: &str,
    ) -> Result<u64, ForgeError>;

    /// Set an open PR's title and body.
    ///
    /// # Errors
    ///
    /// If the forge refuses.
    fn edit_pull_request(
        &self,
        repo: &str,
        number: u64,
        title: &str,
        body: &str,
    ) -> Result<(), ForgeError>;

    /// Create the release at `release.tag`, or refresh the one already there.
    ///
    /// Additive, unlike everything else on this trait: it hangs files off a tag
    /// and destroys no history. Refreshing replaces same-named assets, which is
    /// safe precisely because this project's artifacts are byte-reproducible —
    /// re-shipping an unchanged book uploads the same bytes.
    ///
    /// # Errors
    ///
    /// If the remote cannot be reached, or the upload is refused.
    fn publish_release(&self, repo: &str, release: &Release) -> Result<(), ForgeError>;

    /// Make `branch` the site GitHub actually serves, and ask for one build.
    ///
    /// Pushing a site branch is not publishing a site. Pages has to be told
    /// which branch to read, and it builds on a *push* to that branch — which
    /// has already happened by the time it is listening. Reports what it did so
    /// the caller can print it; a repository already serving something is left
    /// alone, which is a result and not a failure.
    ///
    /// # Errors
    ///
    /// If the remote cannot be reached, or the change is refused.
    fn enable_pages(&self, repo: &str, branch: &str) -> Result<PagesAction, ForgeError>;
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
    /// What GitHub would say about this repository's Pages. `None` is a
    /// repository with no Pages at all.
    pub pages: Option<PagesState>,
    /// The remote's branch heads, as `remote_heads` reports them.
    pub heads: BTreeMap<String, String>,
    /// The forge's PRs, as `pull_requests` reports them.
    pub prs: Vec<ForgePr>,
    /// When set, any call whose record starts with it fails — how a test
    /// makes one move of a schedule fail.
    pub fail_on: Option<String>,
    next_pr: std::cell::Cell<u64>,
    calls: std::cell::RefCell<Vec<String>>,
    releases: std::cell::RefCell<Vec<Release>>,
}

impl FakeForge {
    #[must_use]
    pub fn new(state: RemoteState, steps_md: Option<String>) -> Self {
        let heads = if state == RemoteState::HasContent {
            BTreeMap::from([(crate::replay::BRANCH.to_string(), "0".repeat(40))])
        } else {
            BTreeMap::new()
        };
        Self {
            state,
            steps_md,
            site_state: RemoteState::Absent,
            site_marker: None,
            unreachable: None,
            pages: None,
            heads,
            prs: Vec::new(),
            fail_on: None,
            next_pr: std::cell::Cell::new(101),
            calls: std::cell::RefCell::new(Vec::new()),
            releases: std::cell::RefCell::new(Vec::new()),
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
        let mut forge = Self::new(RemoteState::HasContent, None);
        forge.site_state = RemoteState::HasContent;
        forge.unreachable = Some(reason.to_string());
        forge
    }

    /// Every method called on this forge, in order.
    #[must_use]
    pub fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }

    /// Every release this forge was asked to publish, in order.
    #[must_use]
    pub fn releases(&self) -> Vec<Release> {
        self.releases.borrow().clone()
    }

    /// Whether anything that changes the remote was called.
    ///
    /// A release counts. It creates nothing destructive, but a dry run that
    /// uploaded a PDF would still have reached out — and "nothing was sent" has
    /// to mean nothing.
    #[must_use]
    pub fn mutated(&self) -> bool {
        self.calls().iter().any(|c| {
            c.starts_with("push")
                || c.starts_with("create")
                || c.starts_with("release")
                || c.starts_with("set_default_branch")
                || c.starts_with("open_pr")
                || c.starts_with("edit_pr")
        })
    }

    fn record(&self, what: &str) {
        self.calls.borrow_mut().push(what.to_string());
    }

    /// Record `what`, then fail if `fail_on` names it.
    fn call(&self, what: &str) -> Result<(), ForgeError> {
        self.record(what);
        match &self.fail_on {
            Some(prefix) if what.starts_with(prefix.as_str()) => Err(ForgeError::Failed {
                what: what.to_string(),
                stderr: "refused by FakeForge".to_string(),
            }),
            _ => Ok(()),
        }
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

    fn enable_pages(&self, repo: &str, branch: &str) -> Result<PagesAction, ForgeError> {
        self.record("enable_pages");
        self.reachable(repo)?;
        let action = pages_action(self.pages.as_ref(), branch);
        if !matches!(action, PagesAction::LeaveAlone { .. }) {
            self.record("pages_mutated");
        }
        Ok(action)
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

    fn publish_release(&self, repo: &str, release: &Release) -> Result<(), ForgeError> {
        self.record("release");
        self.reachable(repo)?;
        self.releases.borrow_mut().push(release.clone());
        Ok(())
    }

    fn remote_heads(&self, repo: &str) -> Result<BTreeMap<String, String>, ForgeError> {
        self.call("remote_heads")?;
        self.reachable(repo)?;
        Ok(self.heads.clone())
    }

    fn pull_requests(&self, repo: &str) -> Result<Vec<ForgePr>, ForgeError> {
        self.call("pull_requests")?;
        self.reachable(repo)?;
        Ok(self.prs.clone())
    }

    fn push_ref(
        &self,
        _dir: &Path,
        _repo: &str,
        src: &str,
        dst: &str,
        lease: Option<&str>,
    ) -> Result<String, ForgeError> {
        self.call(&format!(
            "push_ref {src} -> {dst} lease={}",
            lease.unwrap_or("-")
        ))?;
        Ok(format!("sha:{src}"))
    }

    fn push_tags(&self, _dir: &Path, _repo: &str) -> Result<usize, ForgeError> {
        self.call("push_tags")?;
        Ok(0)
    }

    fn set_default_branch(&self, _repo: &str, branch: &str) -> Result<(), ForgeError> {
        self.call(&format!("set_default_branch {branch}"))
    }

    fn open_pull_request(
        &self,
        _repo: &str,
        branch: &str,
        _title: &str,
        _body: &str,
    ) -> Result<u64, ForgeError> {
        self.call(&format!("open_pr {branch}"))?;
        let n = self.next_pr.get();
        self.next_pr.set(n + 1);
        Ok(n)
    }

    fn edit_pull_request(
        &self,
        _repo: &str,
        number: u64,
        _title: &str,
        _body: &str,
    ) -> Result<(), ForgeError> {
        self.call(&format!("edit_pr #{number}"))
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
    /// What GitHub currently believes about `repo`'s Pages, or `None` when
    /// there are none.
    ///
    /// A `404` is the answer "no Pages"; anything else is a refusal to answer
    /// and must not be read as absence — the same rule the repository probe
    /// follows, and for the same reason.
    fn pages_state(repo: &str) -> Result<Option<PagesState>, ForgeError> {
        let (ok, body, stderr) = gh(&["api", &format!("repos/{repo}/pages")])?;
        if !ok {
            if is_not_found(&stderr) {
                return Ok(None);
            }
            return Err(ForgeError::Failed {
                what: format!("gh api repos/{repo}/pages"),
                stderr: stderr.trim().to_string(),
            });
        }
        Ok(Some(parse_pages_state(&body)))
    }

    /// Ask for one Pages build. See [`pages_build_args`] for why it is needed.
    fn request_pages_build(repo: &str) -> Result<(), ForgeError> {
        let args = pages_build_args(repo);
        let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
        let (ok, _, stderr) = gh(&borrowed)?;
        if ok {
            return Ok(());
        }
        Err(ForgeError::Failed {
            what: format!("gh api POST repos/{repo}/pages/builds"),
            stderr: stderr.trim().to_string(),
        })
    }

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

    fn enable_pages(&self, repo: &str, branch: &str) -> Result<PagesAction, ForgeError> {
        let action = pages_action(Self::pages_state(repo)?.as_ref(), branch);
        let create = match &action {
            // Somebody's live site. Read-only from here.
            PagesAction::LeaveAlone { .. } => return Ok(action),
            PagesAction::BuildOnly => {
                Self::request_pages_build(repo)?;
                return Ok(action);
            }
            PagesAction::Create => true,
            PagesAction::Repoint => false,
        };

        let args = pages_source_args(repo, branch, create);
        let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
        let (ok, _, stderr) = gh(&borrowed)?;
        if !ok {
            return Err(ForgeError::Failed {
                what: format!(
                    "gh api {} repos/{repo}/pages",
                    if create { "POST" } else { "PUT" }
                ),
                stderr: stderr.trim().to_string(),
            });
        }
        Self::request_pages_build(repo)?;
        Ok(action)
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

    fn publish_release(&self, repo: &str, release: &Release) -> Result<(), ForgeError> {
        let paths: Vec<String> = release
            .assets
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();

        // `gh release create` fails outright on an existing tag, so which
        // command to run is a question that has to be asked first.
        let (found, _, stderr) = gh(&["release", "view", &release.tag, "--repo", repo])?;
        if !found && !release_absent(&stderr) {
            return Err(ForgeError::Unreachable {
                repo: repo.to_string(),
                reason: stderr.trim().to_string(),
            });
        }

        let mut args: Vec<&str> = if found {
            // `--clobber` is what makes a re-ship idempotent rather than a
            // duplicate-filename error.
            vec![
                "release",
                "upload",
                &release.tag,
                "--repo",
                repo,
                "--clobber",
            ]
        } else {
            vec![
                "release",
                "create",
                &release.tag,
                "--repo",
                repo,
                "--title",
                &release.title,
                "--notes",
                &release.notes,
            ]
        };
        args.extend(paths.iter().map(String::as_str));

        let (ok, _, stderr) = gh(&args)?;
        if !ok {
            return Err(ForgeError::Failed {
                what: format!("gh release {}", if found { "upload" } else { "create" }),
                stderr: stderr.trim().to_string(),
            });
        }
        Ok(())
    }

    fn remote_heads(&self, repo: &str) -> Result<BTreeMap<String, String>, ForgeError> {
        let out = std::process::Command::new("git")
            .args(["ls-remote", "--heads", &remote_url(repo)])
            .output()
            .map_err(|e| ForgeError::Failed {
                what: "git ls-remote --heads".to_string(),
                stderr: e.to_string(),
            })?;
        if !out.status.success() {
            return Err(ForgeError::Unreachable {
                repo: repo.to_string(),
                reason: String::from_utf8_lossy(&out.stderr).trim().to_string(),
            });
        }
        Ok(parse_ls_remote(&String::from_utf8_lossy(&out.stdout)))
    }

    fn pull_requests(&self, repo: &str) -> Result<Vec<ForgePr>, ForgeError> {
        let args = pr_list_args(repo);
        let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
        let (ok, body, stderr) = gh(&borrowed)?;
        if !ok {
            return Err(ForgeError::Unreachable {
                repo: repo.to_string(),
                reason: stderr.trim().to_string(),
            });
        }
        Ok(parse_pr_list(&body))
    }

    fn push_ref(
        &self,
        dir: &Path,
        repo: &str,
        src: &str,
        dst: &str,
        lease: Option<&str>,
    ) -> Result<String, ForgeError> {
        // Resolve first: the refspec carries a SHA, and the SHA is what the
        // next push of `dst` leases against.
        let (ok, stdout, stderr) = git_in(dir, &["rev-parse".into(), format!("{src}^{{commit}}")])?;
        if !ok {
            return Err(ForgeError::Failed {
                what: format!("git rev-parse {src}"),
                stderr: stderr.trim().to_string(),
            });
        }
        let sha = stdout.trim().to_string();
        let args = push_ref_args(&remote_url(repo), &sha, dst, lease);
        let (ok, _, stderr) = git_in(dir, &args)?;
        if !ok {
            return Err(ForgeError::Failed {
                what: format!("git push {src} → {dst}"),
                stderr: stderr.trim().to_string(),
            });
        }
        Ok(sha)
    }

    fn push_tags(&self, dir: &Path, repo: &str) -> Result<usize, ForgeError> {
        // Plainly forced: a replay recreates every tag with the same name and
        // a new SHA, and the gate has already said this remote is ours.
        let args: Vec<String> = vec![
            "push".into(),
            "--force".into(),
            remote_url(repo),
            "--tags".into(),
        ];
        let (ok, _, stderr) = git_in(dir, &args)?;
        if !ok {
            return Err(ForgeError::Failed {
                what: "git push --force --tags".to_string(),
                stderr: stderr.trim().to_string(),
            });
        }
        Ok(count(dir, &["tag", "--list"]))
    }

    fn set_default_branch(&self, repo: &str, branch: &str) -> Result<(), ForgeError> {
        let args = default_branch_args(repo, branch);
        let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
        let (ok, _, stderr) = gh(&borrowed)?;
        if ok {
            Ok(())
        } else {
            Err(ForgeError::Failed {
                what: format!("gh api -X PATCH repos/{repo} default_branch={branch}"),
                stderr: stderr.trim().to_string(),
            })
        }
    }

    fn open_pull_request(
        &self,
        repo: &str,
        branch: &str,
        title: &str,
        body: &str,
    ) -> Result<u64, ForgeError> {
        // `--flag=value`, so a body that starts with `---` (Decision 23's
        // footer, when the book gives no description) is never read as a flag.
        let (title, body) = (format!("--title={title}"), format!("--body={body}"));
        let (ok, stdout, stderr) = gh(&[
            "pr", "create", "-R", repo, "--base", MAIN, "--head", branch, &title, &body,
        ])?;
        if !ok {
            return Err(ForgeError::Failed {
                what: format!("gh pr create --head {branch}"),
                stderr: stderr.trim().to_string(),
            });
        }
        pr_number_from_url(&stdout).ok_or_else(|| ForgeError::Failed {
            what: format!("gh pr create --head {branch}"),
            stderr: format!("no PR number in `{}`", stdout.trim()),
        })
    }

    fn edit_pull_request(
        &self,
        repo: &str,
        number: u64,
        title: &str,
        body: &str,
    ) -> Result<(), ForgeError> {
        let n = number.to_string();
        let (title, body) = (format!("--title={title}"), format!("--body={body}"));
        let (ok, _, stderr) = gh(&["pr", "edit", &n, "-R", repo, &title, &body])?;
        if ok {
            Ok(())
        } else {
            Err(ForgeError::Failed {
                what: format!("gh pr edit #{number}"),
                stderr: stderr.trim().to_string(),
            })
        }
    }
}

/// Whether `gh release view` failed because there is no such release.
///
/// The same distinction this module draws for repositories, and drawn for
/// the same reason: "no release yet" means create one, while "we could not
/// look" must never be mistaken for it and quietly create a second.
#[must_use]
pub fn release_absent(stderr: &str) -> bool {
    let lower = stderr.to_ascii_lowercase();
    lower.contains("release not found") || lower.contains("no release found")
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

/// What GitHub currently believes about a repository's Pages.
///
/// Only the three fields a decision turns on. `ever_built` is the important
/// one: it separates "configured" from "serving", and a repository can sit in
/// the first state forever without anybody noticing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PagesState {
    /// `build_type`: `legacy` deploys from a branch, `workflow` waits for an
    /// Actions workflow.
    pub build_type: String,
    /// `source.branch`, when there is one.
    pub branch: Option<String>,
    /// Whether Pages has ever produced a build. `status: null` and an empty
    /// build list both mean no.
    pub ever_built: bool,
}

/// What to do about a repository's Pages before calling a site published.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PagesAction {
    /// Pages is not set up at all. Create it, pointing at the site branch.
    Create,
    /// Pages exists but has never served a page, so nothing can break by
    /// repointing it at the branch Bower just pushed.
    Repoint,
    /// Already pointed here. It only needs the build that the push could not
    /// trigger.
    BuildOnly,
    /// Somebody is serving something else from this repository. Change
    /// nothing, and say so.
    LeaveAlone { serving: String },
}

/// Decide what a site push should do about Pages.
///
/// The rule is "never take a served site away from its owner". A repository
/// with no Pages, or with Pages that has never built, is nobody's yet —
/// GitHub hands out a `build_type: "workflow"` default that waits on an
/// Actions workflow this project never writes, so a book pushed to a fresh
/// repository would 404 forever unless something repoints it. A repository
/// that is genuinely serving pages is left alone, whatever it is serving.
#[must_use]
pub fn pages_action(current: Option<&PagesState>, want_branch: &str) -> PagesAction {
    match current {
        None => PagesAction::Create,
        Some(s) if !s.ever_built => PagesAction::Repoint,
        Some(s) if s.build_type == "legacy" && s.branch.as_deref() == Some(want_branch) => {
            PagesAction::BuildOnly
        }
        Some(s) => PagesAction::LeaveAlone {
            serving: match (&s.branch, s.build_type.as_str()) {
                (_, "workflow") => "a GitHub Actions workflow".to_string(),
                (Some(b), _) => format!("branch `{b}`"),
                (None, t) => format!("`{t}`"),
            },
        },
    }
}

/// The value of a top-level JSON string field, or `None` for `null`/absent.
///
/// Three fields out of a twelve-field object do not justify pulling
/// `serde_json` into the base binary, where it is an optional dependency of
/// the preprocessor and nothing else.
fn json_str<'a>(body: &'a str, key: &str) -> Option<&'a str> {
    let after = body.split(&format!("\"{key}\":")).nth(1)?.trim_start();
    let rest = after.strip_prefix('"')?;
    rest.split('"').next()
}

/// Read a `GET /repos/{repo}/pages` body into the three fields a decision needs.
///
/// `source.branch` is read after the `"source"` key so a `branch` field
/// somewhere else in the object could never be mistaken for it.
#[must_use]
pub fn parse_pages_state(body: &str) -> PagesState {
    let source = body.split("\"source\":").nth(1).unwrap_or("");
    PagesState {
        build_type: json_str(body, "build_type").unwrap_or_default().to_string(),
        branch: json_str(source, "branch").map(str::to_string),
        // `"status": null` is the state that matters: configured, never served.
        ever_built: json_str(body, "status").is_some(),
    }
}

/// The `gh` arguments that point `repo`'s Pages at `branch`.
///
/// `build_type=legacy` is "deploy from a branch". The default on a repository
/// nobody has configured is `workflow`, which waits for an Actions workflow
/// this project never writes and reads the wrong branch regardless.
///
/// `create` chooses the verb: Pages that does not exist is `POST`ed into being,
/// Pages that does is `PUT` over. The payload is identical either way.
#[must_use]
pub fn pages_source_args(repo: &str, branch: &str, create: bool) -> Vec<String> {
    vec![
        "api".into(),
        "-X".into(),
        if create { "POST".into() } else { "PUT".into() },
        format!("repos/{repo}/pages"),
        "-f".into(),
        "build_type=legacy".into(),
        "-f".into(),
        format!("source[branch]={branch}"),
        "-f".into(),
        "source[path]=/".into(),
    ]
}

/// The `gh` arguments that ask for one Pages build.
///
/// Pages builds on a *push* to its source branch, and Bower pushes the branch
/// before it can configure Pages — so the push that would have triggered the
/// first build has already happened by the time Pages is listening. Without
/// this the settings read correctly, the build list is empty, and the URL 404s
/// indefinitely. Later pushes need no such help.
#[must_use]
pub fn pages_build_args(repo: &str) -> Vec<String> {
    vec![
        "api".into(),
        "-X".into(),
        "POST".into(),
        format!("repos/{repo}/pages/builds"),
    ]
}

/// The arguments that push commit `src` to the remote ref `dst`, leased
/// against `lease` — what the remote held when Bower last looked, or the
/// commit Bower itself pushed there a moment ago (EPIC-09 Decision 21).
///
/// `src` is a SHA, resolved in the built repository before the push, so a
/// tag name can never be misread as a branch. A ref that is not there yet
/// needs no force and has nothing to protect.
#[must_use]
pub fn push_ref_args(url: &str, src: &str, dst: &str, lease: Option<&str>) -> Vec<String> {
    let refspec = format!("{src}:{dst}");
    match lease {
        None => vec!["push".into(), url.to_string(), refspec],
        Some(sha) => vec![
            "push".into(),
            format!("--force-with-lease={dst}:{sha}"),
            url.to_string(),
            refspec,
        ],
    }
}

/// `git ls-remote --heads` output, as full ref → SHA. An empty repository
/// prints nothing, which is an empty map.
#[must_use]
pub fn parse_ls_remote(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter_map(|l| {
            let (sha, name) = l.split_once('\t')?;
            Some((name.trim().to_string(), sha.trim().to_string()))
        })
        .collect()
}

/// The `jq` program `gh pr list` runs: one tab-separated line per PR —
/// number, state, head branch, head SHA, and the `bower-pr:` digest or `-`.
/// A filter rather than JSON, because the base binary parses no JSON.
pub const PR_LIST_JQ: &str = r#".[] | [(.number|tostring), .state, .headRefName, .headRefOid, ((.body // "") | (capture("bower-pr: (?<d>[0-9a-f]+)").d // "-"))] | join("\t")"#;

/// The `gh` arguments that list every PR against main, in any state.
#[must_use]
pub fn pr_list_args(repo: &str) -> Vec<String> {
    [
        "pr",
        "list",
        "-R",
        repo,
        "--base",
        MAIN,
        "--state",
        "all",
        "--limit",
        "1000",
        "--json",
        "number,state,headRefName,headRefOid,body",
        "--jq",
        PR_LIST_JQ,
    ]
    .iter()
    .map(ToString::to_string)
    .collect()
}

/// Read [`PR_LIST_JQ`]'s lines. A line that does not have its shape is
/// skipped rather than guessed at.
#[must_use]
pub fn parse_pr_list(text: &str) -> Vec<ForgePr> {
    text.lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            let [number, state, branch, head, digest] = f.as_slice() else {
                return None;
            };
            let state = match *state {
                "OPEN" => ForgePrState::Open,
                "MERGED" => ForgePrState::Merged,
                "CLOSED" => ForgePrState::Closed,
                _ => return None,
            };
            Some(ForgePr {
                number: number.parse().ok()?,
                branch: (*branch).to_string(),
                state,
                head: (*head).to_string(),
                digest: (*digest != "-").then(|| (*digest).to_string()),
            })
        })
        .collect()
}

/// The PR number at the end of the URL `gh pr create` prints.
#[must_use]
pub fn pr_number_from_url(url: &str) -> Option<u64> {
    let url = url.trim();
    let (rest, number) = url.rsplit_once('/')?;
    rest.ends_with("/pull").then(|| number.parse().ok())?
}

/// The `gh` arguments that make `branch` the repository's default
/// (EPIC-09 Decision 26).
#[must_use]
pub fn default_branch_args(repo: &str, branch: &str) -> Vec<String> {
    vec![
        "api".into(),
        "-X".into(),
        "PATCH".into(),
        format!("repos/{repo}"),
        "-f".into(),
        format!("default_branch={branch}"),
    ]
}

/// Run `git -C dir args…`: success, stdout, stderr.
fn git_in(dir: &Path, args: &[String]) -> Result<(bool, String, String), ForgeError> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|e| ForgeError::Failed {
            what: format!("git {}", args.join(" ")),
            stderr: e.to_string(),
        })?;
    Ok((
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    ))
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

    const URL: &str = "https://github.com/folkengine/rust4failures.git";

    fn pages(build_type: &str, branch: Option<&str>, ever_built: bool) -> PagesState {
        PagesState {
            build_type: build_type.to_string(),
            branch: branch.map(str::to_string),
            ever_built,
        }
    }

    // The two responses `folkengine/rust4failures` actually returned on
    // 5 September 2026, before and after its Pages source was corrected.
    const PAGES_BROKEN: &str = r#"{"url":"https://api.github.com/repos/folkengine/rust4failures/pages","status":null,"cname":null,"custom_404":false,"html_url":"https://folkengine.github.io/rust4failures/","build_type":"workflow","source":{"branch":"main","path":"/"},"public":true,"https_enforced":true}"#;
    const PAGES_FIXED: &str = r#"{"url":"https://api.github.com/repos/folkengine/rust4failures/pages","status":"built","cname":null,"custom_404":true,"html_url":"https://folkengine.github.io/rust4failures/","build_type":"legacy","source":{"branch":"gh-pages","path":"/"},"public":true,"https_enforced":true}"#;

    #[test]
    fn enable_pages__never_touches_a_repository_that_is_serving() {
        // The one rule that could cost somebody something. A repository with a
        // live site keeps it, and `enable_pages` must not have written.
        let mut f = FakeForge::new(RemoteState::HasContent, None);
        f.pages = Some(parse_pages_state(PAGES_FIXED));
        // Serving `gh-pages` already; we want `docs`. Someone else's site.
        let got = f.enable_pages("o/r", "docs").unwrap();
        assert!(matches!(got, PagesAction::LeaveAlone { .. }), "{got:?}");
        assert!(
            !f.calls().contains(&"pages_mutated".to_string()),
            "a served site was written to: {:?}",
            f.calls()
        );
    }

    #[test]
    fn enable_pages__repoints_a_repository_that_never_served() {
        let mut f = FakeForge::new(RemoteState::HasContent, None);
        f.pages = Some(parse_pages_state(PAGES_BROKEN));
        assert_eq!(
            f.enable_pages("o/r", "gh-pages").unwrap(),
            PagesAction::Repoint
        );
        assert!(f.calls().contains(&"pages_mutated".to_string()));
    }

    #[test]
    fn parse_pages_state__reads_the_response_that_was_404ing() {
        let got = parse_pages_state(PAGES_BROKEN);
        assert_eq!(got.build_type, "workflow");
        assert_eq!(got.branch.as_deref(), Some("main"));
        assert!(!got.ever_built, "`status: null` is not a build");
    }

    #[test]
    fn parse_pages_state__reads_the_response_that_was_serving() {
        let got = parse_pages_state(PAGES_FIXED);
        assert_eq!(got.build_type, "legacy");
        assert_eq!(got.branch.as_deref(), Some("gh-pages"));
        assert!(got.ever_built);
    }

    #[test]
    fn parse_pages_state__end_to_end_over_the_real_bodies() {
        // The whole point, stated once: the broken repository gets repointed,
        // the working one is only asked to build.
        assert_eq!(
            pages_action(Some(&parse_pages_state(PAGES_BROKEN)), "gh-pages"),
            PagesAction::Repoint
        );
        assert_eq!(
            pages_action(Some(&parse_pages_state(PAGES_FIXED)), "gh-pages"),
            PagesAction::BuildOnly
        );
    }

    #[test]
    fn pages_action__no_pages_at_all_is_created() {
        assert_eq!(pages_action(None, "gh-pages"), PagesAction::Create);
    }

    #[test]
    fn pages_action__configured_but_never_built_is_repointed() {
        // The state a real repository was actually found in: GitHub had handed
        // out `build_type: "workflow"` pointing at `main`, `status` was null,
        // and the build list was empty. Nothing was being served, so nothing
        // could be taken away — and leaving it alone would 404 forever.
        let found = pages("workflow", Some("main"), false);
        assert_eq!(pages_action(Some(&found), "gh-pages"), PagesAction::Repoint);
    }

    #[test]
    fn pages_action__already_pointed_here_only_needs_a_build() {
        let found = pages("legacy", Some("gh-pages"), true);
        assert_eq!(
            pages_action(Some(&found), "gh-pages"),
            PagesAction::BuildOnly
        );
    }

    #[test]
    fn pages_action__a_site_someone_is_serving_is_never_touched() {
        // The rule that matters. A repository publishing something real keeps
        // publishing it, whatever Bower would have preferred.
        let workflow = pages("workflow", Some("main"), true);
        let PagesAction::LeaveAlone { serving } = pages_action(Some(&workflow), "gh-pages") else {
            panic!("a served site must be left alone");
        };
        assert!(serving.contains("workflow"), "{serving}");

        let other_branch = pages("legacy", Some("docs"), true);
        let PagesAction::LeaveAlone { serving } = pages_action(Some(&other_branch), "gh-pages")
        else {
            panic!("a served site must be left alone");
        };
        assert!(serving.contains("docs"), "{serving}");
    }

    #[test]
    fn pages_source_args__match_the_call_that_actually_worked() {
        // Verified by hand against a real repository before this was written.
        assert_eq!(
            pages_source_args("folkengine/rust4failures", "gh-pages", false),
            vec![
                "api",
                "-X",
                "PUT",
                "repos/folkengine/rust4failures/pages",
                "-f",
                "build_type=legacy",
                "-f",
                "source[branch]=gh-pages",
                "-f",
                "source[path]=/",
            ]
        );
    }

    #[test]
    fn pages_source_args__creating_posts_instead_of_putting() {
        let create = pages_source_args("o/r", "gh-pages", true);
        assert!(create.contains(&"POST".to_string()), "{create:?}");
        assert!(!create.contains(&"PUT".to_string()), "{create:?}");
        // The payload does not change with the verb.
        assert_eq!(
            &create[3..],
            &pages_source_args("o/r", "gh-pages", false)[3..]
        );
    }

    #[test]
    fn pages_build_args__ask_for_one_build() {
        assert_eq!(
            pages_build_args("folkengine/rust4failures"),
            vec![
                "api",
                "-X",
                "POST",
                "repos/folkengine/rust4failures/pages/builds"
            ]
        );
    }

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

    #[test]
    fn push_ref_args__lease_against_the_value_read() {
        assert_eq!(
            push_ref_args(URL, "abc123", "refs/heads/main", Some("def456")),
            [
                "push",
                "--force-with-lease=refs/heads/main:def456",
                URL,
                "abc123:refs/heads/main",
            ]
        );
    }

    #[test]
    fn push_ref_args__a_ref_that_is_not_there_forces_nothing() {
        let args = push_ref_args(URL, "abc123", "refs/heads/try/x", None);
        assert_eq!(args, ["push", URL, "abc123:refs/heads/try/x"]);
        assert!(!args.iter().any(|a| a.contains("force")), "{args:?}");
    }

    #[test]
    fn parse_ls_remote__reads_every_head() {
        let text = "1111111111111111111111111111111111111111\trefs/heads/main\n2222222222222222222222222222222222222222\trefs/heads/try/shout\n";
        let heads = parse_ls_remote(text);
        assert_eq!(heads.len(), 2);
        assert_eq!(heads["refs/heads/main"], "1".repeat(40));
        assert_eq!(heads["refs/heads/try/shout"], "2".repeat(40));
        assert!(
            parse_ls_remote("").is_empty(),
            "an empty repository has no heads"
        );
    }

    #[test]
    fn pr_list_args__ask_for_every_state_against_main() {
        let args = pr_list_args("o/n");
        assert_eq!(&args[..5], ["pr", "list", "-R", "o/n", "--base"]);
        assert!(args.contains(&"main".to_string()), "{args:?}");
        assert!(args.windows(2).any(|w| w == ["--state", "all"]), "{args:?}");
        assert!(
            args.windows(2)
                .any(|w| w[0] == "--jq" && w[1] == PR_LIST_JQ),
            "{args:?}"
        );
    }

    #[test]
    fn parse_pr_list__reads_the_jq_lines() {
        // The exact shape `gh pr list --jq PR_LIST_JQ` printed against
        // abstecker/bower-sandbox on 14 September 2026.
        let text = concat!(
            "4\tMERGED\tfeat/merge-me\ta53df691b8873e11ee76dcb9a6a30ded39fcd8cc\t-\n",
            "3\tOPEN\ttry/abandon\t2c754e9e638914b95dadaf478990a9c084b19d6f\t0123456789abcdef\n",
            "1\tCLOSED\ttry/old\tb907fbfa529ed2c9dc1133728fb94ca7f9b04b3f\t-\n",
        );
        let prs = parse_pr_list(text);
        assert_eq!(prs.len(), 3);
        assert_eq!(prs[0].number, 4);
        assert_eq!(prs[0].state, ForgePrState::Merged);
        assert_eq!(prs[0].branch, "feat/merge-me");
        assert_eq!(prs[0].digest, None);
        assert_eq!(prs[1].state, ForgePrState::Open);
        assert_eq!(prs[1].digest.as_deref(), Some("0123456789abcdef"));
        assert_eq!(prs[2].state, ForgePrState::Closed);
        // A line that is not ours is skipped, never guessed at.
        assert!(parse_pr_list("garbage\n").is_empty());
    }

    #[test]
    fn pr_number_from_url__reads_the_last_segment() {
        assert_eq!(
            pr_number_from_url("https://github.com/o/n/pull/42\n"),
            Some(42)
        );
        assert_eq!(pr_number_from_url("https://github.com/o/n/issues"), None);
    }

    #[test]
    fn default_branch_args__patch_the_repository() {
        assert_eq!(
            default_branch_args("o/n", "main"),
            [
                "api",
                "-X",
                "PATCH",
                "repos/o/n",
                "-f",
                "default_branch=main"
            ]
        );
    }

    #[test]
    fn fake_forge__records_the_new_calls_and_can_fail_on_one() {
        let mut forge = FakeForge::new(RemoteState::HasContent, None);
        assert!(forge.heads.contains_key("refs/heads/main"));
        let dir = Path::new("/nowhere");
        assert_eq!(
            forge
                .push_ref(dir, "o/n", "step-001-a", "refs/heads/main", Some("x"))
                .unwrap(),
            "sha:step-001-a"
        );
        assert_eq!(forge.open_pull_request("o/n", "b", "T", "B").unwrap(), 101);
        assert_eq!(forge.open_pull_request("o/n", "c", "T", "B").unwrap(), 102);
        assert_eq!(
            forge.calls(),
            [
                "push_ref step-001-a -> refs/heads/main lease=x",
                "open_pr b",
                "open_pr c"
            ]
        );
        assert!(forge.mutated());

        forge.fail_on = Some("open_pr".to_string());
        assert!(forge.open_pull_request("o/n", "d", "T", "B").is_err());
    }
}
