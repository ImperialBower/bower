//! Caller-supplied input. **Spike-local copies** of `bower_core::source::Chapter`
//! and `Location`; the real crate imports them so the book has one notion of
//! "where".

/// One chapter: its path (for locations) and its markdown text.
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

    /// `src/ch03-rank.md` → `ch03-rank`.
    #[must_use]
    pub fn stem(&self) -> &str {
        let base = self.path.rsplit('/').next().unwrap_or(&self.path);
        base.strip_suffix(".md").unwrap_or(base)
    }
}

/// A 1-based chapter/line pair whose `Display` is grep-shaped: `src/ch03.md:42`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
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

/// An inclusive 1-based line range inside a chapter.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LineRange {
    pub start: usize,
    pub end: usize,
}

impl std::fmt::Display for LineRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.start == self.end {
            write!(f, "L{}", self.start)
        } else {
            write!(f, "L{}-L{}", self.start, self.end)
        }
    }
}
