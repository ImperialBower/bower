//! Reading a book off disk into the value the kernel consumes.
//!
//! The kernel never opens a file (`bower-core/src/lib.rs:11`); this module does
//! all of it and hands over a [`BookSource`]. Reading order is `SUMMARY.md`
//! order, because document order is the default step order.
//!
//! Chapter paths are stored **book-root-relative** — `src/ch01-a-repo.md`, not
//! `ch01-a-repo.md` — because every `Book-Source` commit trailer prints one, and
//! a trailer has to name a path a reader can actually open.

use std::fmt;
use std::path::{Path, PathBuf};

use bower_core::prelude::{BookSource, Chapter};

/// Reads an mdBook directory.
pub struct BookLoader {
    root: PathBuf,
}

/// Why a book could not be loaded.
#[derive(Debug)]
pub enum LoadError {
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    /// `SUMMARY.md` listed no chapters — almost always a malformed summary
    /// rather than a deliberately empty book, so it is an error, not an
    /// empty plan.
    NoChapters { path: PathBuf },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => write!(f, "cannot read {}: {source}", path.display()),
            Self::NoChapters { path } => {
                write!(f, "{} lists no chapters", path.display())
            }
        }
    }
}

impl std::error::Error for LoadError {}

impl BookLoader {
    #[must_use]
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    /// Read `src/SUMMARY.md` and every chapter it links, in order.
    ///
    /// # Errors
    ///
    /// Returns [`LoadError`] if the summary or any chapter cannot be read, or
    /// if the summary links no chapters.
    pub fn load(&self) -> Result<BookSource, LoadError> {
        let summary_path = self.root.join("src").join("SUMMARY.md");
        let summary = read(&summary_path)?;

        let links = chapter_links(&summary);
        if links.is_empty() {
            return Err(LoadError::NoChapters { path: summary_path });
        }

        let mut chapters = Vec::with_capacity(links.len());
        for link in links {
            let on_disk = self.root.join("src").join(&link);
            let text = read(&on_disk)?;
            chapters.push(Chapter::new(&format!("src/{link}"), &text));
        }

        // The block library (`include=`) and binary assets (`op="copy"`) are
        // left empty: no chapter in any current book uses them, and inventing
        // a directory convention before a book needs one would be guesswork.
        Ok(BookSource::from_chapters(chapters))
    }
}

fn read(path: &Path) -> Result<String, LoadError> {
    std::fs::read_to_string(path).map_err(|source| LoadError::Read {
        path: path.to_path_buf(),
        source,
    })
}

/// Every local `.md` target of a markdown link, in document order, without
/// repeats. Deliberately not a full markdown parser: `SUMMARY.md` is a list of
/// links by definition, and a chapter listed twice would be planned twice.
fn chapter_links(summary: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in summary.lines() {
        let mut rest = line;
        while let Some(i) = rest.find("](") {
            let after = &rest[i + 2..];
            let Some(j) = after.find(')') else { break };
            let target = after[..j].trim();
            if is_markdown(target)
                && !target.contains("://")
                && !out.iter().any(|seen| seen == target)
            {
                out.push(target.to_string());
            }
            rest = &after[j + 1..];
        }
    }
    out
}

/// A `.md` target, case-insensitively — `SUMMARY.md` is hand-written, and
/// `Chapter.MD` is a typo worth tolerating rather than silently dropping.
fn is_markdown(target: &str) -> bool {
    Path::new(target)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("md"))
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod loader_tests {
    use super::*;

    fn sample_book_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("books")
            .join("hello-playbook")
    }

    #[test]
    fn links__empty_summary_yields_nothing() {
        // The precondition for `LoadError::NoChapters`, tested here because
        // reaching that variant through `load` would need a temporary book.
        assert!(chapter_links("# Summary\n\nNo links here.\n").is_empty());
    }

    #[test]
    fn links__are_in_document_order_without_repeats() {
        let summary = concat!(
            "# Summary\n\n",
            "- [One](ch01.md)\n",
            "- [Two](ch02.md)\n",
            "- [One again](ch01.md)\n",
            "- [External](https://example.invalid/x.md)\n",
            "- [Not markdown](image.png)\n",
            "- [Shouty extension](ch03.MD)\n",
        );
        assert_eq!(
            chapter_links(summary),
            vec!["ch01.md", "ch02.md", "ch03.MD"]
        );
    }

    #[test]
    fn loader__reads_hello_playbook_in_summary_order() {
        let book = BookLoader::new(&sample_book_root()).load().unwrap();
        let paths: Vec<&str> = book.chapters.iter().map(|c| c.path.as_str()).collect();
        assert_eq!(
            paths,
            vec![
                "src/ch01-a-repo-that-builds.md",
                "src/ch02-the-gate.md",
                "src/ch03-lints-and-format.md",
                "src/ch04-tests-and-failing-on-purpose.md",
                "src/ch05-supply-chain.md",
                "src/ch06-ci.md",
            ]
        );
    }

    #[test]
    fn loader__matches_the_testkit_fixture_exactly() {
        // The one test that stops the on-disk loader and the `include_str!`
        // fixture from drifting into two different books.
        let loaded = BookLoader::new(&sample_book_root()).load().unwrap();
        assert_eq!(loaded, bower_testkit::fixtures::hello_playbook().book);
    }

    #[test]
    fn loader__missing_book_is_an_error() {
        let err = BookLoader::new(Path::new("/nonexistent-book"))
            .load()
            .unwrap_err();
        assert!(matches!(err, LoadError::Read { .. }), "{err:?}");
    }
}
