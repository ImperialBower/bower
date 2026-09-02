//! `bower publish` — one render, several targets (spec § 13 M1).
//!
//! Publishing is a pure fold from a book and its plan to a *render plan*, with
//! renderers as replaceable I/O at the edges — the same relationship
//! `TreeState` has with git. This module owns the fold; the renderers live
//! beside it.
//!
//! As of EPIC-06 Phase 0 it owns [`Target`] alone, which is the one thing the
//! render actually varies on.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use bower_core::prelude::{BookPlan, BookSource};
use serde::Deserialize;

use crate::config::LinkTemplates;
use crate::render;

/// What is being produced.
///
/// Threaded through the render because the elision rule differs, and only
/// because of that. A boolean would do the job today; an enum is right anyway,
/// because spec § 13 names `pdf` and `ipynb` as the next two rungs and a `bool`
/// called `is_html` is a variable that stops being true.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Target {
    /// mdBook HTML. Rust fences keep the eye-toggle a reader expands in place.
    Html,
    /// pandoc epub. No toggle exists anywhere in an epub, so every elision —
    /// Rust included — collapses to a comment naming what was left out.
    Epub,
}

impl Target {
    /// Whether this target can hide code behind a toggle the reader expands.
    ///
    /// True only for [`Target::Html`], and even there only Rust fences use it:
    /// mdBook's hidden-line rule is a Rust feature, not a markdown one.
    #[must_use]
    pub fn has_hidden_lines(self) -> bool {
        matches!(self, Self::Html)
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Html => "html",
            Self::Epub => "epub",
        })
    }
}

impl std::str::FromStr for Target {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "html" => Ok(Self::Html),
            "epub" => Ok(Self::Epub),
            other => Err(format!(
                "unknown target `{other}` — this build knows html and epub \
                 (pdf and ipynb are spec § 13's next rungs, not built yet)"
            )),
        }
    }
}

/// Why a publish could not proceed.
#[derive(Debug)]
pub enum PublishError {
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
}

impl fmt::Display for PublishError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => write!(f, "cannot read {}: {source}", path.display()),
            Self::Parse { path, source } => write!(f, "cannot parse {}: {source}", path.display()),
        }
    }
}

impl std::error::Error for PublishError {}

/// The book's identity, as an epub's metadata needs it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookMeta {
    pub title: String,
    pub authors: Vec<String>,
    pub language: String,
}

impl BookMeta {
    /// Read `[book]` out of `book.toml`.
    ///
    /// From mdBook's file, not a second copy in `bower.toml`: the book's title
    /// and authors are already written down once, and two places to say who
    /// wrote a book is one place to get it wrong.
    ///
    /// # Errors
    ///
    /// [`PublishError`] if `book.toml` is missing or unparseable. An epub with
    /// no title is not worth producing, so this fails rather than defaulting.
    pub fn load(book_root: &Path) -> Result<Self, PublishError> {
        let path = book_root.join("book.toml");
        let text = std::fs::read_to_string(&path).map_err(|source| PublishError::Read {
            path: path.clone(),
            source,
        })?;
        let wire: WireBookToml =
            toml::from_str(&text).map_err(|source| PublishError::Parse { path, source })?;
        Ok(Self {
            title: wire.book.title,
            authors: wire.book.authors,
            language: wire.book.language,
        })
    }
}

/// `book.toml` is mdBook's file, full of keys this crate does not own —
/// `src`, `[output.html]`, `[preprocessor.bower]`. So, uniquely among this
/// crate's wire structs, it does **not** carry `deny_unknown_fields`: rejecting
/// a key mdBook added would break every book on the next mdBook release.
#[derive(Deserialize)]
struct WireBookToml {
    book: WireBookTable,
}

#[derive(Deserialize)]
struct WireBookTable {
    title: String,
    #[serde(default)]
    authors: Vec<String>,
    #[serde(default = "default_language")]
    language: String,
}

fn default_language() -> String {
    "en".to_string()
}

/// One chapter, rendered for one target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedChapter {
    /// Book-relative and `src/`-prefixed, as everywhere else in this crate.
    pub path: String,
    /// The chapter's first heading, or its filename if it has none.
    pub title: String,
    pub markdown: String,
}

/// A whole book, rendered for one target.
///
/// Pure: no I/O, no renderer, nothing written. This is to publishing what
/// `TreeState` is to git — and it is what lets a test assert that the epub
/// elides a Rust block while the HTML keeps its toggle, with no renderer
/// installed and no binary artifact to inspect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderPlan {
    pub target: Target,
    pub meta: BookMeta,
    pub chapters: Vec<RenderedChapter>,
}

/// Fold a loaded book and its resolved plan into a render plan.
#[must_use]
pub fn render_plan(
    book: &BookSource,
    plan: &BookPlan,
    meta: BookMeta,
    target: Target,
    links: &BTreeMap<String, LinkTemplates>,
) -> RenderPlan {
    RenderPlan {
        target,
        meta,
        chapters: book
            .chapters
            .iter()
            .map(|c| RenderedChapter {
                path: c.path.clone(),
                title: heading_of(&c.text).unwrap_or_else(|| c.path.clone()),
                markdown: render::chapter(&c.text, &c.path, plan, links, target),
            })
            .collect(),
    }
}

/// A chapter's first ATX heading, if it has one.
///
/// Only a top-level `# `: a `##` is a section within the chapter, and naming a
/// chapter after its second subsection would be worse than naming it after its
/// file.
fn heading_of(text: &str) -> Option<String> {
    text.lines()
        .find_map(|l| l.strip_prefix("# "))
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod publish_tests {
    use super::*;

    #[test]
    fn target__only_html_hides_lines() {
        assert!(Target::Html.has_hidden_lines());
        assert!(!Target::Epub.has_hidden_lines());
    }

    #[test]
    fn target__round_trips_through_its_name() {
        for t in [Target::Html, Target::Epub] {
            assert_eq!(t.to_string().parse::<Target>().unwrap(), t);
        }
    }

    #[test]
    fn target__an_unbuilt_target_says_which_are_built() {
        let err = "pdf".parse::<Target>().unwrap_err();
        assert!(err.contains("html and epub"), "{err}");
    }

    use crate::loader::BookLoader;
    use bower_core::prelude::{plan, RepoCatalog};

    fn sample_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("books")
            .join("hello-playbook")
    }

    fn sample_plan() -> (BookSource, BookPlan) {
        let book = BookLoader::new(&sample_root()).load().unwrap();
        let plan = plan(&book, &RepoCatalog::from_names(&["hello-playbook"])).unwrap();
        (book, plan)
    }

    fn links() -> BTreeMap<String, LinkTemplates> {
        let mut m = BTreeMap::new();
        m.insert(
            "hello-playbook".to_string(),
            LinkTemplates {
                blob: Some("https://x.invalid/blob/{tag}/{path}#L{start}-L{end}".to_string()),
                tree: None,
                commit: None,
            },
        );
        m
    }

    #[test]
    fn meta__comes_from_book_toml() {
        let meta = BookMeta::load(&sample_root()).unwrap();
        assert_eq!(meta.title, "Hello, Playbook");
        assert_eq!(meta.authors, vec!["ImperialBower".to_string()]);
        assert_eq!(meta.language, "en");
    }

    #[test]
    fn meta__missing_book_toml_is_an_error() {
        // An epub with no title is not worth producing.
        let err = BookMeta::load(Path::new("/nonexistent-book")).unwrap_err();
        assert!(matches!(err, PublishError::Read { .. }), "{err:?}");
    }

    #[test]
    fn meta__ignores_the_keys_mdbook_owns() {
        // `book.toml` carries `src`, `[output.html]`, `[preprocessor.bower]`.
        // Rejecting a key mdBook adds would break every book on its next
        // release, so this wire struct alone does not deny unknown fields.
        let meta = BookMeta::load(&sample_root());
        assert!(meta.is_ok(), "{meta:?}");
    }

    #[test]
    fn render_plan__covers_every_chapter_in_reading_order() {
        // A chapter silently dropped from an epub is one nobody notices is
        // missing.
        let (book, plan) = sample_plan();
        let meta = BookMeta::load(&sample_root()).unwrap();
        let rp = render_plan(&book, &plan, meta, Target::Epub, &links());

        assert_eq!(rp.chapters.len(), book.chapters.len());
        let paths: Vec<&str> = rp.chapters.iter().map(|c| c.path.as_str()).collect();
        assert_eq!(paths[0], "src/ch01-a-repo-that-builds.md");
        assert_eq!(paths[5], "src/ch06-ci.md");
        assert_eq!(rp.chapters[3].title, "Tests, and failing on purpose");
    }

    #[test]
    fn render_plan__epub_and_html_differ_only_in_elision() {
        // The claim that there is one engine, made testable.
        let (book, plan) = sample_plan();
        let meta = BookMeta::load(&sample_root()).unwrap();
        let html = render_plan(&book, &plan, meta.clone(), Target::Html, &links());
        let epub = render_plan(&book, &plan, meta, Target::Epub, &links());

        // Chapters without display markers render identically.
        assert_eq!(
            html.chapters[0].markdown, epub.chapters[0].markdown,
            "an unmarked chapter must not vary by target"
        );

        // Chapter 4 is the marked one, and only there do they diverge.
        let h4 = &html.chapters[3].markdown;
        let e4 = &epub.chapters[3].markdown;
        assert_ne!(h4, e4);
        assert!(h4.contains("# mod tests {"), "html keeps the toggle");
        assert!(!e4.contains("# mod tests {"), "epub has no toggle to keep");
        assert!(e4.contains("lines elided"), "epub says what it left out");
        assert!(e4.contains("full file: https://x.invalid/blob/"), "{e4}");
    }

    #[test]
    fn heading__falls_back_to_the_path() {
        assert_eq!(heading_of("# Real Title\n\nprose"), Some("Real Title".into()));
        // A `##` is a section within the chapter, not the chapter.
        assert_eq!(heading_of("## Subsection\n"), None);
        assert_eq!(heading_of("no heading at all"), None);
    }
}
