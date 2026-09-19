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
    /// Part titles, in reading order. Empty for a book without parts. The
    /// planner never reads them: a part groups chapters for a reader and
    /// means nothing to a generated repository.
    pub parts: Vec<PartMark>,
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

    /// The same book with its parts, checked against its chapters: every mark
    /// has a title, names a chapter the book holds, and opens after the mark
    /// ahead of it.
    ///
    /// # Errors
    ///
    /// Returns the first [`PartError`] found, in mark order.
    pub fn with_parts(self, parts: Vec<PartMark>) -> Result<Self, PartError> {
        let mut previous: Option<(usize, &PartMark)> = None;
        for mark in &parts {
            if mark.title.trim().is_empty() {
                return Err(PartError::Untitled {
                    first: mark.first.clone(),
                });
            }
            let Some(at) = self.chapters.iter().position(|c| c.path == mark.first) else {
                return Err(PartError::UnknownChapter {
                    title: mark.title.clone(),
                    first: mark.first.clone(),
                });
            };
            if let Some((before, earlier)) = previous {
                if at == before {
                    return Err(PartError::Shared {
                        first: mark.first.clone(),
                        titles: (earlier.title.clone(), mark.title.clone()),
                    });
                }
                if at < before {
                    return Err(PartError::OutOfOrder {
                        title: mark.title.clone(),
                    });
                }
            }
            previous = Some((at, mark));
        }
        Ok(Self { parts, ..self })
    }

    /// The mark that opens at this chapter, if one does.
    #[must_use]
    pub fn part_at(&self, chapter: &str) -> Option<&PartMark> {
        self.parts.iter().find(|p| p.first == chapter)
    }
}

/// Where a part opens: a title, and the chapter it sits in front of. It does
/// not say where the part closes, because mdBook does not either.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PartMark {
    /// The author's markdown, as written after `# ` in `SUMMARY.md`.
    pub title: String,
    /// Path of the first chapter under the title, `src/`-prefixed.
    pub first: String,
}

impl PartMark {
    #[must_use]
    pub fn new(title: &str, first: &str) -> Self {
        Self {
            title: title.to_string(),
            first: first.to_string(),
        }
    }
}

/// Why a set of part marks does not fit a book.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PartError {
    /// `first` names no chapter in the book.
    UnknownChapter { title: String, first: String },
    /// A mark opens before the mark ahead of it.
    OutOfOrder { title: String },
    /// Two marks open at one chapter, so the first part is empty.
    Shared {
        first: String,
        titles: (String, String),
    },
    /// A title of nothing but whitespace.
    Untitled { first: String },
}

impl std::fmt::Display for PartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownChapter { title, first } => {
                write!(
                    f,
                    "part `{title}` opens at `{first}`, which is not a chapter of the book"
                )
            }
            Self::OutOfOrder { title } => {
                write!(f, "part `{title}` opens before the part ahead of it")
            }
            Self::Shared { first, titles } => write!(
                f,
                "parts `{}` and `{}` both open at `{first}`, so `{}` is empty",
                titles.0, titles.1, titles.0
            ),
            Self::Untitled { first } => write!(f, "the part opening at `{first}` has no title"),
        }
    }
}

impl std::error::Error for PartError {}

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

    fn three() -> BookSource {
        BookSource::from_chapters(vec![
            Chapter::new("src/a.md", ""),
            Chapter::new("src/b.md", ""),
            Chapter::new("src/c.md", ""),
        ])
    }

    #[test]
    fn parts__default_is_empty() {
        assert!(BookSource::default().parts.is_empty());
        assert!(three().parts.is_empty());
    }

    #[test]
    fn with_parts__accepts_marks_in_order() {
        let marks = vec![
            PartMark::new("One", "src/a.md"),
            PartMark::new("Two", "src/c.md"),
        ];
        let book = three().with_parts(marks.clone()).unwrap();
        assert_eq!(book.parts, marks);
        assert_eq!(book.chapters, three().chapters);
    }

    #[test]
    fn with_parts__refuses_an_unknown_chapter() {
        let err = three()
            .with_parts(vec![PartMark::new("One", "src/z.md")])
            .unwrap_err();
        assert_eq!(
            err,
            PartError::UnknownChapter {
                title: "One".to_string(),
                first: "src/z.md".to_string()
            }
        );
    }

    #[test]
    fn with_parts__refuses_marks_out_of_order() {
        let err = three()
            .with_parts(vec![
                PartMark::new("One", "src/c.md"),
                PartMark::new("Two", "src/a.md"),
            ])
            .unwrap_err();
        assert_eq!(
            err,
            PartError::OutOfOrder {
                title: "Two".to_string()
            }
        );
    }

    #[test]
    fn with_parts__refuses_two_marks_at_one_chapter() {
        let err = three()
            .with_parts(vec![
                PartMark::new("One", "src/b.md"),
                PartMark::new("Two", "src/b.md"),
            ])
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "parts `One` and `Two` both open at `src/b.md`, so `One` is empty"
        );
    }

    #[test]
    fn with_parts__refuses_a_blank_title() {
        let err = three()
            .with_parts(vec![PartMark::new(" \t", "src/a.md")])
            .unwrap_err();
        assert_eq!(
            err,
            PartError::Untitled {
                first: "src/a.md".to_string()
            }
        );
    }

    #[test]
    fn part_at__finds_only_the_opening_chapter() {
        let book = three()
            .with_parts(vec![PartMark::new("One", "src/b.md")])
            .unwrap();
        assert_eq!(book.part_at("src/b.md").unwrap().title, "One");
        assert!(book.part_at("src/a.md").is_none());
        assert!(book.part_at("src/c.md").is_none());
    }

    #[test]
    fn catalog__contains_and_spec() {
        let cat = RepoCatalog::from_names(&["failers"]);
        assert!(cat.contains("failers"));
        assert!(!cat.contains("clock"));
        assert!(!cat.spec(&RepoName::new("failers")).keep_region_markers);
    }
}
