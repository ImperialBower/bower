//! `bower status` — what here is out of date?
//!
//! Three artifacts can disagree: the book on disk, the `bower.lock` written by
//! the last `bower plan`, and a repository written by the last `bower build`.
//! This module compares them and says which is stale, naming the steps, tags,
//! and files involved.
//!
//! Nothing here talks to a network, and nothing repairs anything. Reporting is
//! the whole job; the fix is always to re-run `plan` or `build`.

use std::fmt;
use std::path::{Path, PathBuf};

use bower_core::prelude::{lock_text, BookPlan};

#[derive(Debug)]
pub enum StatusError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl fmt::Display for StatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
        }
    }
}

impl std::error::Error for StatusError {}

/// The `bower.lock` on disk, against the one this book would produce now.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LockDrift {
    /// No `bower.lock`. Normal for a book nobody has planned yet — an honest
    /// state, not a failure.
    NeverPlanned,
    InSync,
    Stale {
        /// Lines the file has that a fresh plan would not write.
        only_in_lock: Vec<String>,
        /// Lines a fresh plan would write that the file lacks.
        only_in_book: Vec<String>,
    },
}

/// Compare `<book_root>/bower.lock` against what `plan` would write now.
///
/// The comparison is over rendered text rather than a parsed structure on
/// purpose: [`lock_text`] is the only definition of that format, and a parser
/// would be a second one, free to disagree with it. The cost is that changing
/// `lock_text`'s formatting reads as drift in every book until each is
/// re-planned — which is correct, because the file on disk really is not what
/// the tool would write now.
///
/// # Errors
///
/// Returns [`StatusError`] if the lock exists but cannot be read.
pub fn lock_drift(book_root: &Path, plan: &BookPlan) -> Result<LockDrift, StatusError> {
    let path = book_root.join("bower.lock");
    let found = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(LockDrift::NeverPlanned),
        Err(source) => return Err(StatusError::Io { path, source }),
    };

    let expected = lock_text(plan);
    if found == expected {
        return Ok(LockDrift::InSync);
    }

    Ok(LockDrift::Stale {
        only_in_lock: lines_missing_from(&found, &expected),
        only_in_book: lines_missing_from(&expected, &found),
    })
}

/// Non-blank lines of `a` that do not appear anywhere in `b`.
///
/// A line-set difference rather than a positional diff: the lock is one line
/// per step, so "this step's line changed" is the question, and a step moving
/// position is already visible in its own `NNN` prefix.
fn lines_missing_from(a: &str, b: &str) -> Vec<String> {
    let theirs: Vec<&str> = b.lines().collect();
    a.lines()
        .filter(|l| !l.trim().is_empty())
        .filter(|l| !theirs.contains(l))
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used, clippy::expect_used)]
mod status_tests {
    use super::*;
    use bower_core::prelude::{plan, BookSource, Chapter, RepoCatalog};

    fn scratch(case: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bower-status-{case}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn book_plan(expect: &str) -> BookPlan {
        let text = format!(
            concat!(
                "# One\n\n",
                "<!-- bower repo=\"r\" step=\"first\" file=\"src/lib.rs\" expect=\"{}\" -->\n",
                "```rust\npub fn f() {{}}\n```\n",
            ),
            expect
        );
        let book = BookSource::from_chapters(vec![Chapter::new("src/ch01.md", &text)]);
        plan(&book, &RepoCatalog::from_names(&["r"])).unwrap()
    }

    #[test]
    fn lock__absent_is_never_planned() {
        let dir = scratch("absent");
        assert_eq!(
            lock_drift(&dir, &book_plan("pass")).unwrap(),
            LockDrift::NeverPlanned
        );
    }

    #[test]
    fn lock__identical_text_is_in_sync() {
        let dir = scratch("insync");
        let p = book_plan("pass");
        std::fs::write(dir.join("bower.lock"), lock_text(&p)).unwrap();
        assert_eq!(lock_drift(&dir, &p).unwrap(), LockDrift::InSync);
    }

    #[test]
    fn lock__a_changed_step_shows_both_lines() {
        // A report that says "something changed" without saying what is not a
        // report. Both the stale line and the fresh one must appear.
        let dir = scratch("stale");
        std::fs::write(dir.join("bower.lock"), lock_text(&book_plan("pass"))).unwrap();

        let drift = lock_drift(&dir, &book_plan("test_fail")).unwrap();
        let LockDrift::Stale {
            only_in_lock,
            only_in_book,
        } = drift
        else {
            panic!("expected Stale, got {drift:?}");
        };

        assert!(
            only_in_lock.iter().any(|l| l.contains("expect=pass")),
            "{only_in_lock:?}"
        );
        assert!(
            only_in_book.iter().any(|l| l.contains("expect=test_fail")),
            "{only_in_book:?}"
        );
    }

    #[test]
    fn lock__an_unreadable_lock_is_an_error_not_a_verdict() {
        // A directory where a file should be: reporting "in sync" here would
        // be a lie, and reporting "never planned" would hide a real problem.
        let dir = scratch("unreadable");
        std::fs::create_dir_all(dir.join("bower.lock")).unwrap();
        assert!(lock_drift(&dir, &book_plan("pass")).is_err());
    }
}
