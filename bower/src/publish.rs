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
    /// A renderer's binary is not installed.
    MissingTool { tool: String, install: String },
    /// The renderer ran and failed.
    Failed { what: String, stderr: String },
}

impl fmt::Display for PublishError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => write!(f, "cannot read {}: {source}", path.display()),
            Self::Parse { path, source } => write!(f, "cannot parse {}: {source}", path.display()),
            Self::MissingTool { tool, install } => {
                write!(f, "`{tool}` is not installed; try: {install}")
            }
            Self::Failed { what, stderr } => write!(f, "{what} failed: {stderr}"),
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

/// What a render produced.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Artifact {
    pub path: PathBuf,
    pub bytes: u64,
}

/// Something that turns a [`RenderPlan`] into files.
///
/// Shaped like `Forge` (`bower/src/forge.rs:57`) and for the same reason: the
/// decisions are tested against a fake, and only the shelling-out is not.
pub trait Renderer {
    /// The binaries this renderer needs, checked before anything is written.
    ///
    /// # Errors
    ///
    /// [`PublishError::MissingTool`], naming the install command.
    fn preflight(&self) -> Result<(), PublishError>;

    /// Produce the artifact under `out`.
    ///
    /// # Errors
    ///
    /// [`PublishError`] if writing fails or the renderer refuses the input.
    fn render(&self, plan: &RenderPlan, out: &Path) -> Result<Artifact, PublishError>;
}

/// Whether a binary can be run at all.
///
/// Pure enough to test: point it at a name nothing could plausibly install.
#[must_use]
pub fn tool_present(tool: &str) -> bool {
    std::process::Command::new(tool)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn need(tool: &str, install: &str) -> Result<(), PublishError> {
    if tool_present(tool) {
        Ok(())
    } else {
        Err(PublishError::MissingTool {
            tool: tool.to_string(),
            install: install.to_string(),
        })
    }
}

/// A filename-safe form of a book's title: `Hello, Playbook` → `hello-playbook`.
#[must_use]
pub fn slug(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut dash = false;
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    // A title of pure punctuation would otherwise name a file `-`.
    let trimmed = out.trim_end_matches('-');
    if trimmed.is_empty() {
        "book".to_string()
    } else {
        trimmed.to_string()
    }
}

/// The epub, via pandoc.
pub struct PandocRenderer;

impl Renderer for PandocRenderer {
    fn preflight(&self) -> Result<(), PublishError> {
        need("pandoc", "brew install pandoc")
    }

    fn render(&self, plan: &RenderPlan, out: &Path) -> Result<Artifact, PublishError> {
        let io = |path: &Path| {
            let path = path.to_path_buf();
            move |source| PublishError::Read { path, source }
        };

        std::fs::create_dir_all(out).map_err(io(out))?;
        // Chapters go to a scratch directory as numbered files: pandoc reads
        // them in the order it is given, and a numeric prefix makes that order
        // visible if anyone looks.
        let src = out.join(".chapters");
        if src.exists() {
            std::fs::remove_dir_all(&src).map_err(io(&src))?;
        }
        std::fs::create_dir_all(&src).map_err(io(&src))?;

        let mut inputs = Vec::with_capacity(plan.chapters.len());
        for (i, chapter) in plan.chapters.iter().enumerate() {
            let stem = chapter
                .path
                .rsplit('/')
                .next()
                .unwrap_or(&chapter.path)
                .trim_end_matches(".md");
            let file = src.join(format!("{:03}-{stem}.md", i + 1));
            std::fs::write(&file, &chapter.markdown).map_err(io(&file))?;
            inputs.push(file);
        }

        let artifact = out.join(format!("{}.epub", slug(&plan.meta.title)));
        let mut cmd = std::process::Command::new("pandoc");
        cmd.arg("--from")
            .arg("markdown")
            .arg("--to")
            .arg("epub")
            .arg("--toc")
            .arg("--metadata")
            .arg(format!("title={}", plan.meta.title))
            .arg("--metadata")
            .arg(format!("lang={}", plan.meta.language));
        for author in &plan.meta.authors {
            cmd.arg("--metadata").arg(format!("author={author}"));
        }
        cmd.arg("-o").arg(&artifact).args(&inputs);

        let output = cmd.output().map_err(|e| PublishError::Failed {
            what: "pandoc".to_string(),
            stderr: e.to_string(),
        })?;
        if !output.status.success() {
            return Err(PublishError::Failed {
                what: "pandoc".to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }

        let bytes = std::fs::metadata(&artifact).map_err(io(&artifact))?.len();
        Ok(Artifact {
            path: artifact,
            bytes,
        })
    }
}

/// The HTML, via mdBook.
///
/// This renderer does **not** consume the `RenderPlan` the way `PandocRenderer`
/// does: mdBook re-runs `mdbook-bower` itself, so the render happens inside the
/// preprocessor rather than here. The plan is still built and still checked
/// first — a book that will not resolve fails before `mdbook` is invoked — but
/// the HTML path renders twice. Unifying that means teaching `mdbook-bower` to
/// read a prepared plan, and is deliberately not in EPIC-06.
pub struct MdBookRenderer {
    /// mdBook needs the source directory, not the plan. Hence the field.
    pub book_root: PathBuf,
}

impl Renderer for MdBookRenderer {
    fn preflight(&self) -> Result<(), PublishError> {
        need("mdbook", "cargo install mdbook")?;
        // `book.toml` declares `[preprocessor.bower]`, so mdBook will fail
        // without this on PATH — and failing here says why.
        need(
            "mdbook-bower",
            "cargo build -p bower && export PATH=\"$PWD/target/debug:$PATH\"",
        )
    }

    fn render(&self, _plan: &RenderPlan, out: &Path) -> Result<Artifact, PublishError> {
        // mdBook resolves `-d` **relative to the book root**, so a relative
        // `out` lands inside the book rather than where the caller asked.
        let dest = std::path::absolute(out).map_err(|source| PublishError::Read {
            path: out.to_path_buf(),
            source,
        })?;
        let output = std::process::Command::new("mdbook")
            .arg("build")
            .arg(&self.book_root)
            .arg("-d")
            .arg(&dest)
            .output()
            .map_err(|e| PublishError::Failed {
                what: "mdbook build".to_string(),
                stderr: e.to_string(),
            })?;
        if !output.status.success() {
            return Err(PublishError::Failed {
                what: "mdbook build".to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        // Reporting 0 bytes for a file that is not there would hide exactly
        // the bug that `-d` relativity caused.
        let index = dest.join("index.html");
        let bytes = std::fs::metadata(&index).map_err(|source| PublishError::Read {
            path: index.clone(),
            source,
        })?;
        Ok(Artifact {
            path: index,
            bytes: bytes.len(),
        })
    }
}

/// A [`Renderer`] that writes nothing and remembers what it was handed.
///
/// Public for the same reason `FakeForge` is: an integration test needs it, and
/// the assertions that matter are about what a renderer was *given*, which only
/// something that records can answer.
pub struct FakeRenderer {
    seen: std::cell::RefCell<Vec<RenderPlan>>,
}

impl Default for FakeRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeRenderer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            seen: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// Every plan this renderer was asked to render, in order.
    #[must_use]
    pub fn seen(&self) -> Vec<RenderPlan> {
        self.seen.borrow().clone()
    }
}

impl Renderer for FakeRenderer {
    fn preflight(&self) -> Result<(), PublishError> {
        Ok(())
    }

    fn render(&self, plan: &RenderPlan, out: &Path) -> Result<Artifact, PublishError> {
        self.seen.borrow_mut().push(plan.clone());
        Ok(Artifact {
            path: out.join("fake"),
            bytes: 0,
        })
    }
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
        assert_eq!(
            heading_of("# Real Title\n\nprose"),
            Some("Real Title".into())
        );
        // A `##` is a section within the chapter, not the chapter.
        assert_eq!(heading_of("## Subsection\n"), None);
        assert_eq!(heading_of("no heading at all"), None);
    }

    #[test]
    fn tool_present__says_no_to_something_nobody_installs() {
        assert!(!tool_present("bower-no-such-tool-exists"));
        // pandoc is this repo's documented publishing dependency.
        assert!(
            tool_present("pandoc"),
            "pandoc is needed to publish an epub"
        );
    }

    #[test]
    fn renderer__preflight_names_the_install_command() {
        // The failure a reader will actually hit.
        let err = PublishError::MissingTool {
            tool: "pandoc".into(),
            install: "brew install pandoc".into(),
        };
        let text = err.to_string();
        assert!(text.contains("pandoc"), "{text}");
        assert!(text.contains("brew install pandoc"), "{text}");
    }

    #[test]
    fn slug__is_filename_safe() {
        assert_eq!(slug("Hello, Playbook"), "hello-playbook");
        assert_eq!(slug("Rust for Failers!"), "rust-for-failers");
        assert_eq!(slug("  spaced  out  "), "spaced-out");
        // A title of pure punctuation would otherwise name a file `-`.
        assert_eq!(slug("!!!"), "book");
    }

    #[test]
    fn fake__records_the_plan_it_was_given() {
        let (book, plan) = sample_plan();
        let meta = BookMeta::load(&sample_root()).unwrap();
        let rp = render_plan(&book, &plan, meta, Target::Epub, &links());

        let fake = FakeRenderer::new();
        fake.preflight().unwrap();
        let artifact = fake.render(&rp, Path::new("/tmp/nowhere")).unwrap();

        assert_eq!(fake.seen().len(), 1);
        assert_eq!(fake.seen()[0].target, Target::Epub);
        assert_eq!(fake.seen()[0].chapters.len(), 6);
        assert_eq!(artifact.bytes, 0, "a fake writes nothing");
    }

    #[test]
    fn mdbook_renderer__keeps_the_book_root_because_mdbook_needs_it() {
        // Recorded as a test because it is the one asymmetry in this design:
        // mdBook re-runs the preprocessor, so it needs the source directory
        // rather than the plan.
        let r = MdBookRenderer {
            book_root: sample_root(),
        };
        assert!(r.book_root.join("book.toml").exists());
    }
}
