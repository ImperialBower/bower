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

use bower_core::prelude::{lock_text, BookPlan, RepoPlan};

use crate::materialize::{read_dir_recursive, Blobs};
use crate::replay::expected_tags;

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

/// A built repository, against the plan that should have produced it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepoDrift {
    /// No directory, or a directory with no `.git`. An honest state.
    NeverBuilt,
    /// The repository keeps its tags in `.git/packed-refs`, which this reader
    /// does not parse. Admitted rather than guessed: reporting "no tags" would
    /// be a confident wrong answer. A freshly replayed repository has loose
    /// refs, so this is only reached after `git gc` or a clone.
    TagsPacked,
    InSync,
    Stale {
        missing_tags: Vec<String>,
        unexpected_tags: Vec<String>,
        differing_files: Vec<String>,
        missing_files: Vec<String>,
        unexpected_files: Vec<String>,
    },
}

/// Compare a built repository at `dir` against `plan` and the files its final
/// step should leave behind.
///
/// `expected` is passed in rather than computed here: the caller already builds
/// the final blobs in order to account for `STEPS.md`, and doing it twice would
/// be two chances to do it differently.
///
/// Tags are read as loose refs from `.git/refs/tags` rather than through `gix`,
/// the same choice `bower/tests/determinism.rs` makes and for the same reason —
/// a drift report that shares a library with the thing it inspects can share a
/// bug with it.
///
/// # Errors
///
/// Returns [`StatusError`] if the directory exists but cannot be walked.
pub fn repo_drift(dir: &Path, plan: &RepoPlan, expected: &Blobs) -> Result<RepoDrift, StatusError> {
    let git = dir.join(".git");
    if !dir.is_dir() || !git.is_dir() {
        return Ok(RepoDrift::NeverBuilt);
    }

    let tags_dir = git.join("refs").join("tags");
    let found_tags = match loose_tags(&tags_dir) {
        Ok(t) => t,
        Err(source) => {
            return Err(StatusError::Io {
                path: tags_dir,
                source,
            })
        }
    };
    if found_tags.is_empty() && git.join("packed-refs").exists() {
        return Ok(RepoDrift::TagsPacked);
    }

    let want_tags = expected_tags(plan);
    let missing_tags: Vec<String> = want_tags
        .iter()
        .filter(|t| !found_tags.contains(t))
        .cloned()
        .collect();
    let unexpected_tags: Vec<String> = found_tags
        .iter()
        .filter(|t| !want_tags.contains(t))
        .cloned()
        .collect();

    let found_files = read_dir_recursive(dir).map_err(|source| StatusError::Io {
        path: dir.to_path_buf(),
        source,
    })?;

    let mut missing_files = Vec::new();
    let mut differing_files = Vec::new();
    for (path, (bytes, _)) in expected {
        match found_files.get(path) {
            None => missing_files.push(path.clone()),
            Some((found, _)) if found != bytes => differing_files.push(path.clone()),
            Some(_) => {}
        }
    }
    let unexpected_files: Vec<String> = found_files
        .keys()
        // `.git` is the repository, not its content.
        .filter(|p| !p.starts_with(".git/"))
        .filter(|p| !expected.contains_key(*p))
        .cloned()
        .collect();

    if missing_tags.is_empty()
        && unexpected_tags.is_empty()
        && differing_files.is_empty()
        && missing_files.is_empty()
        && unexpected_files.is_empty()
    {
        return Ok(RepoDrift::InSync);
    }

    Ok(RepoDrift::Stale {
        missing_tags,
        unexpected_tags,
        differing_files,
        missing_files,
        unexpected_files,
    })
}

/// Loose tag names under `.git/refs/tags`, recursively — a tag may be
/// `release/1.0`, which git stores as a nested path.
fn loose_tags(dir: &Path) -> std::io::Result<Vec<String>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let mut stack = vec![(dir.to_path_buf(), String::new())];
    while let Some((path, prefix)) = stack.pop() {
        for entry in std::fs::read_dir(&path)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let full = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{prefix}/{name}")
            };
            if entry.path().is_dir() {
                stack.push((entry.path(), full));
            } else {
                out.push(full);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// One repository's verdict.
#[derive(Clone, Debug)]
pub struct StatusReport {
    pub repo: String,
    pub steps: usize,
    pub lock: LockDrift,
    pub repo_state: RepoDrift,
}

impl StatusReport {
    /// True when something compared is out of date.
    ///
    /// Absent artifacts are **not** drift. A book nobody has planned and a
    /// repository nobody has built are honest states, not failures — without
    /// that distinction every fresh checkout is a red build, and a red build
    /// nobody believes is one they mute. `TagsPacked` is likewise an admission
    /// that a check could not run, not a claim that it failed.
    #[must_use]
    pub fn has_drift(&self) -> bool {
        matches!(self.lock, LockDrift::Stale { .. })
            || matches!(self.repo_state, RepoDrift::Stale { .. })
    }
}

/// At most this many items of any one drift kind are printed; the rest are
/// summarized. A report longer than a screen is a report nobody reads.
const SHOWN: usize = 5;

impl fmt::Display for StatusReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} — {} steps", self.repo, self.steps)?;

        match &self.lock {
            LockDrift::NeverPlanned => {
                writeln!(f, "  lock      never planned — run `bower plan`")?;
            }
            LockDrift::InSync => writeln!(f, "  lock      in sync")?,
            LockDrift::Stale {
                only_in_lock,
                only_in_book,
            } => {
                let n = only_in_lock.len() + only_in_book.len();
                writeln!(
                    f,
                    "  lock      STALE — {n} line(s) differ; run `bower plan`"
                )?;
                diff(f, '-', only_in_lock)?;
                diff(f, '+', only_in_book)?;
            }
        }

        match &self.repo_state {
            RepoDrift::NeverBuilt => {
                writeln!(f, "  repo      never built — run `bower build`")?;
            }
            RepoDrift::TagsPacked => {
                writeln!(f, "  repo      tags are packed; cannot check them here")?;
            }
            RepoDrift::InSync => writeln!(f, "  repo      in sync")?,
            RepoDrift::Stale {
                missing_tags,
                unexpected_tags,
                differing_files,
                missing_files,
                unexpected_files,
            } => {
                writeln!(f, "  repo      STALE — run `bower build`")?;
                list(f, "missing tag", missing_tags)?;
                list(f, "extra tag", unexpected_tags)?;
                list(f, "differs", differing_files)?;
                list(f, "missing", missing_files)?;
                list(f, "extra", unexpected_files)?;
            }
        }
        Ok(())
    }
}

/// Diff-shaped lines, where the marker is one character and the content is the
/// point. Padding it into a column would push every line off the screen.
fn diff(f: &mut fmt::Formatter<'_>, mark: char, items: &[String]) -> fmt::Result {
    for item in items.iter().take(SHOWN) {
        writeln!(f, "              {mark} {item}")?;
    }
    if items.len() > SHOWN {
        writeln!(f, "              {mark} … and {} more", items.len() - SHOWN)?;
    }
    Ok(())
}

/// Labelled lines, where the label names the kind of drift.
fn list(f: &mut fmt::Formatter<'_>, label: &str, items: &[String]) -> fmt::Result {
    for item in items.iter().take(SHOWN) {
        writeln!(f, "              {label:<12} {item}")?;
    }
    if items.len() > SHOWN {
        writeln!(
            f,
            "              {:<12} … and {} more",
            "",
            items.len() - SHOWN
        )?;
    }
    Ok(())
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

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used, clippy::expect_used)]
mod repo_tests {
    use super::*;
    use crate::materialize::blobs_of;
    use bower_core::prelude::{plan, BookSource, Chapter, RepoCatalog};

    fn scratch(case: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bower-repodrift-{case}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn one_step_plan() -> RepoPlan {
        let text = concat!(
            "# One\n\n",
            "<!-- bower repo=\"r\" step=\"first\" file=\"src/lib.rs\" -->\n",
            "```rust\npub fn f() {}\n```\n",
        );
        let book = BookSource::from_chapters(vec![Chapter::new("src/ch01.md", text)]);
        plan(&book, &RepoCatalog::from_names(&["r"]))
            .unwrap()
            .repos
            .remove(0)
    }

    fn expected_blobs(plan: &RepoPlan) -> Blobs {
        blobs_of(&plan.steps.last().unwrap().tree)
    }

    /// A repository shaped like one `bower build` would leave, without running
    /// git: loose tag files and a working tree.
    fn fake_repo(case: &str, plan: &RepoPlan, blobs: &Blobs) -> PathBuf {
        let dir = scratch(case);
        let tags = dir.join(".git").join("refs").join("tags");
        std::fs::create_dir_all(&tags).unwrap();
        for tag in expected_tags(plan) {
            std::fs::write(tags.join(tag), "0000000\n").unwrap();
        }
        crate::materialize::write_files(&dir, blobs).unwrap();
        dir
    }

    #[test]
    fn repo__missing_directory_is_never_built() {
        let p = one_step_plan();
        let drift = repo_drift(Path::new("/nonexistent-repo"), &p, &expected_blobs(&p)).unwrap();
        assert_eq!(drift, RepoDrift::NeverBuilt);
    }

    #[test]
    fn repo__directory_without_git_is_never_built() {
        let dir = scratch("nogit");
        let p = one_step_plan();
        assert_eq!(
            repo_drift(&dir, &p, &expected_blobs(&p)).unwrap(),
            RepoDrift::NeverBuilt
        );
    }

    #[test]
    fn repo__a_matching_repo_is_in_sync() {
        let p = one_step_plan();
        let blobs = expected_blobs(&p);
        let dir = fake_repo("insync", &p, &blobs);
        assert_eq!(repo_drift(&dir, &p, &blobs).unwrap(), RepoDrift::InSync);
    }

    #[test]
    fn repo__missing_tag_is_named() {
        let p = one_step_plan();
        let blobs = expected_blobs(&p);
        let dir = fake_repo("misstag", &p, &blobs);
        std::fs::remove_file(dir.join(".git/refs/tags/step-001-first")).unwrap();

        let RepoDrift::Stale { missing_tags, .. } = repo_drift(&dir, &p, &blobs).unwrap() else {
            panic!("expected Stale");
        };
        assert_eq!(missing_tags, vec!["step-001-first"]);
    }

    #[test]
    fn repo__changed_file_is_named() {
        let p = one_step_plan();
        let blobs = expected_blobs(&p);
        let dir = fake_repo("changed", &p, &blobs);
        std::fs::write(dir.join("src/lib.rs"), "pub fn tampered() {}\n").unwrap();

        let RepoDrift::Stale {
            differing_files, ..
        } = repo_drift(&dir, &p, &blobs).unwrap()
        else {
            panic!("expected Stale");
        };
        assert_eq!(differing_files, vec!["src/lib.rs"]);
    }

    #[test]
    fn repo__unexpected_file_is_named() {
        let p = one_step_plan();
        let blobs = expected_blobs(&p);
        let dir = fake_repo("extra", &p, &blobs);
        std::fs::write(dir.join("STOWAWAY.md"), "not from the book\n").unwrap();

        let RepoDrift::Stale {
            unexpected_files, ..
        } = repo_drift(&dir, &p, &blobs).unwrap()
        else {
            panic!("expected Stale");
        };
        assert_eq!(unexpected_files, vec!["STOWAWAY.md"]);
    }

    #[test]
    fn repo__missing_file_is_named() {
        let p = one_step_plan();
        let blobs = expected_blobs(&p);
        let dir = fake_repo("deleted", &p, &blobs);
        std::fs::remove_file(dir.join("src/lib.rs")).unwrap();

        let RepoDrift::Stale { missing_files, .. } = repo_drift(&dir, &p, &blobs).unwrap() else {
            panic!("expected Stale");
        };
        assert_eq!(missing_files, vec!["src/lib.rs"]);
    }

    #[test]
    fn repo__packed_tags_are_admitted_not_guessed() {
        // Every tag would look missing. Saying so is better than a confident
        // wrong answer.
        let p = one_step_plan();
        let blobs = expected_blobs(&p);
        let dir = fake_repo("packed", &p, &blobs);
        std::fs::remove_dir_all(dir.join(".git/refs/tags")).unwrap();
        std::fs::write(dir.join(".git/packed-refs"), "# pack-refs with: peeled\n").unwrap();

        assert_eq!(repo_drift(&dir, &p, &blobs).unwrap(), RepoDrift::TagsPacked);
    }

    #[test]
    fn repo__git_internals_are_not_unexpected_files() {
        // `.git` is the repository, not its content. Counting it as drift
        // would make every built repo report as stale.
        let p = one_step_plan();
        let blobs = expected_blobs(&p);
        let dir = fake_repo("gitdir", &p, &blobs);
        std::fs::write(dir.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();

        assert_eq!(repo_drift(&dir, &p, &blobs).unwrap(), RepoDrift::InSync);
    }

    #[test]
    fn expected_tags__covers_every_step_and_one_end_tag_per_chapter() {
        let p = one_step_plan();
        assert_eq!(expected_tags(&p), vec!["step-001-first", "ch01-end"]);
    }
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used, clippy::expect_used)]
mod report_tests {
    use super::*;

    fn report(lock: LockDrift, repo_state: RepoDrift) -> StatusReport {
        StatusReport {
            repo: "r".to_string(),
            steps: 1,
            lock,
            repo_state,
        }
    }

    #[test]
    fn report__absent_artifacts_are_not_drift() {
        // The distinction that makes this usable in CI. Without it every
        // fresh checkout is a red build, and a red build nobody believes is
        // one they mute.
        let r = report(LockDrift::NeverPlanned, RepoDrift::NeverBuilt);
        assert!(!r.has_drift());
        let text = r.to_string();
        assert!(text.contains("never planned"), "{text}");
        assert!(text.contains("never built"), "{text}");
    }

    #[test]
    fn report__packed_tags_are_not_drift_either() {
        // An admission that a check could not run, not a claim it failed.
        let r = report(LockDrift::InSync, RepoDrift::TagsPacked);
        assert!(!r.has_drift());
        assert!(r.to_string().contains("cannot check"), "{r}");
    }

    #[test]
    fn report__a_stale_lock_is_drift() {
        let r = report(
            LockDrift::Stale {
                only_in_lock: vec!["001 a expect=pass".to_string()],
                only_in_book: vec!["001 a expect=test_fail".to_string()],
            },
            RepoDrift::InSync,
        );
        assert!(r.has_drift());
        let text = r.to_string();
        assert!(text.contains("- 001 a expect=pass"), "{text}");
        assert!(text.contains("+ 001 a expect=test_fail"), "{text}");
        assert!(text.contains("bower plan"), "the fix must be named: {text}");
    }

    #[test]
    fn report__a_stale_repo_is_drift() {
        let r = report(
            LockDrift::InSync,
            RepoDrift::Stale {
                missing_tags: vec!["step-001-a".to_string()],
                unexpected_tags: vec![],
                differing_files: vec!["src/lib.rs".to_string()],
                missing_files: vec![],
                unexpected_files: vec![],
            },
        );
        assert!(r.has_drift());
        let text = r.to_string();
        assert!(text.contains("missing tag  step-001-a"), "{text}");
        assert!(text.contains("differs      src/lib.rs"), "{text}");
    }

    #[test]
    fn report__long_lists_are_capped() {
        // A report longer than a screen is a report nobody reads.
        let many: Vec<String> = (0..12).map(|i| format!("file{i}.rs")).collect();
        let r = report(
            LockDrift::InSync,
            RepoDrift::Stale {
                missing_tags: vec![],
                unexpected_tags: vec![],
                differing_files: many,
                missing_files: vec![],
                unexpected_files: vec![],
            },
        );
        let text = r.to_string();
        assert!(text.contains("file4.rs"), "{text}");
        assert!(!text.contains("file5.rs"), "{text}");
        assert!(text.contains("and 7 more"), "{text}");
    }
}
