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
