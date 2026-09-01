//! Input types: the book source the kernel consumes and the repo catalog it
//! validates against. The caller (CLI, preprocessor, test) does all reading
//! from disk; the kernel only ever sees these values.

use std::collections::BTreeMap;

/// A whole book, as text. Chapters are in reading order — the order of
/// `SUMMARY.md` — because document order is the default step order.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BookSource {
    /// Chapters in reading order.
    pub chapters: Vec<Chapter>,
    /// The block library (§ 14.3): path → file text, for `include=` keys.
    pub library: BTreeMap<String, String>,
    /// Binary assets for `op="copy"`: book-relative path → bytes.
    pub assets: BTreeMap<String, Vec<u8>>,
}

impl BookSource {
    /// A book from chapters alone — the common fixture shape.
    #[must_use]
    pub fn from_chapters(chapters: Vec<Chapter>) -> Self {
        Self {
            chapters,
            ..Self::default()
        }
    }
}

/// One chapter: its book-relative path and its full markdown text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Chapter {
    pub path: String,
    pub text: String,
}

impl Chapter {
    #[must_use]
    pub fn new(path: &str, text: &str) -> Self {
        Self {
            path: path.to_string(),
            text: text.to_string(),
        }
    }

    /// The chapter's file stem, used when deriving commit subjects
    /// (`ch01-ranks.md` → `ch01-ranks`).
    #[must_use]
    pub fn stem(&self) -> &str {
        let name = self.path.rsplit('/').next().unwrap_or(&self.path);
        name.strip_suffix(".md").unwrap_or(name)
    }
}

/// A position in the book source: chapter path plus 1-based line number.
/// Every error and every planned step carries one — this is the anchor the
/// rendered book and the commit trailers both point at.
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Location {
    pub chapter: String,
    pub line: usize,
}

impl Location {
    #[must_use]
    pub fn new(chapter: &str, line: usize) -> Self {
        Self {
            chapter: chapter.to_string(),
            line,
        }
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.chapter, self.line)
    }
}

/// A target repository's name, as used in `repo="…"` directives.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RepoName(pub String);

impl RepoName {
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }
}

impl std::fmt::Display for RepoName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Per-repo settings that change how the kernel folds and materializes
/// trees. The replay layer owns everything else (remotes, identities,
/// verify commands); only what affects pure computation lives here.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RepoSpec {
    /// Leave `// bower:begin/end` region markers in the materialized tree.
    pub keep_region_markers: bool,
}

/// The declared repos a book may target. Directives naming anything else
/// are an error — annotation typos surface at plan time, not push time.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RepoCatalog(pub BTreeMap<RepoName, RepoSpec>);

impl RepoCatalog {
    /// A catalog of default-spec repos from names alone.
    #[must_use]
    pub fn from_names(names: &[&str]) -> Self {
        Self(
            names
                .iter()
                .map(|n| (RepoName::new(n), RepoSpec::default()))
                .collect(),
        )
    }

    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.0.keys().any(|r| r.0 == name)
    }

    #[must_use]
    pub fn spec(&self, name: &RepoName) -> RepoSpec {
        self.0.get(name).copied().unwrap_or_default()
    }
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod source_tests {
    use super::*;

    #[test]
    fn chapter__stem_strips_dirs_and_extension() {
        assert_eq!(Chapter::new("src/ch01-ranks.md", "").stem(), "ch01-ranks");
        assert_eq!(Chapter::new("intro.md", "").stem(), "intro");
    }

    #[test]
    fn location__display_is_grep_shaped() {
        assert_eq!(
            Location::new("src/ch03.md", 42).to_string(),
            "src/ch03.md:42"
        );
    }

    #[test]
    fn catalog__contains_and_spec() {
        let cat = RepoCatalog::from_names(&["failers"]);
        assert!(cat.contains("failers"));
        assert!(!cat.contains("clock"));
        assert!(!cat.spec(&RepoName::new("failers")).keep_region_markers);
    }
}
