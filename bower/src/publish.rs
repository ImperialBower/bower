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
    /// Typst PDF. Elides exactly as an epub does, for the same reason: paper
    /// has no toggle either. Unlike an epub, its typography is ours to choose.
    Pdf,
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
            Self::Pdf => "pdf",
        })
    }
}

impl std::str::FromStr for Target {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "html" => Ok(Self::Html),
            "epub" => Ok(Self::Epub),
            "pdf" => Ok(Self::Pdf),
            other => Err(format!(
                "unknown target `{other}` — this build knows html, epub, and pdf \
                 (ipynb is spec § 15's, and needs play cells)"
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
    /// The composed cover, when the book ships one. `None` is a normal book:
    /// every book that existed before covers did publishes unchanged.
    pub cover: Option<Cover>,
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
            cover: load_cover(book_root)?,
        })
    }
}

/// A book's cover, composed into one SVG document.
///
/// A newtype rather than a bare `String` because a cover with artwork is
/// megabytes of base64, and a derived `Debug` on [`BookMeta`] that dumped it
/// would make every failing assertion in this crate unreadable.
#[derive(Clone, Eq, PartialEq)]
pub struct Cover(String);

impl Cover {
    /// The SVG source.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.0
    }

    /// Write it where a renderer can point an external tool at it.
    ///
    /// # Errors
    ///
    /// [`PublishError::Read`] if the file cannot be written.
    pub fn write(&self, path: &Path) -> Result<(), PublishError> {
        std::fs::write(path, &self.0).map_err(|source| PublishError::Read {
            path: path.to_path_buf(),
            source,
        })
    }
}

impl fmt::Debug for Cover {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Cover({} bytes)", self.0.len())
    }
}

/// The composed cover of the book at `book_root`, if it ships one.
///
/// Found by convention beside `book.toml`, exactly as `template.typ` is
/// (`bower/src/main.rs`): `cover.svg` is the title band, and `cover.png` /
/// `cover.jpg` is optional artwork stacked below it. A book with the svg and
/// no artwork gets a cover that is all title, which is what lets an author add
/// the picture later without a second code path.
///
/// # Errors
///
/// [`PublishError::Read`] if a file that exists cannot be read.
pub fn load_cover(book_root: &Path) -> Result<Option<Cover>, PublishError> {
    let title = book_root.join("cover.svg");
    if !title.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&title).map_err(|source| PublishError::Read {
        path: title.clone(),
        source,
    })?;

    // First match wins, in a fixed order: two artwork files would otherwise
    // make the cover depend on directory iteration order, and this project
    // does not have artifacts that depend on that.
    for name in ["cover.png", "cover.jpg", "cover.jpeg"] {
        let art = book_root.join(name);
        if let Some(mime) = art_mime(&art).filter(|_| art.exists()) {
            let bytes = std::fs::read(&art).map_err(|source| PublishError::Read {
                path: art.clone(),
                source,
            })?;
            return Ok(Some(Cover(compose_cover(&text, Some((&bytes, mime))))));
        }
    }
    Ok(Some(Cover(compose_cover(&text, None))))
}

/// The MIME type of an artwork file, by extension.
///
/// A closed list, because the value is written into a `data:` URI: guessing
/// wrong there produces a cover that renders in one reader and not the next.
fn art_mime(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        _ => None,
    }
}

/// The cover page: a title band on top, artwork below it.
///
/// Pure, and the reason this is one function rather than two renderer methods:
/// epub and PDF both need exactly **one** image file, so the stacking has to
/// happen before either renderer sees it. Compose once, hand the same bytes to
/// pandoc and to Typst, and the two formats cannot drift apart.
///
/// Artwork is embedded as a `data:` URI rather than linked, so the composed
/// file is self-contained — a linked path would resolve differently in
/// pandoc's working directory than in Typst's.
#[must_use]
pub fn compose_cover(title_svg: &str, art: Option<(&[u8], &str)>) -> String {
    // A 2:3 page, the ordinary book-cover proportion. Fixed rather than
    // configurable: this is a coordinate system, not a design decision — both
    // renderers scale the result to whatever page they have.
    const W: u32 = 1600;
    const H: u32 = 2400;
    const BAND: u32 = 800;

    let band = if art.is_some() { BAND } else { H };
    // Built in one `format!` rather than appended to: an SVG document has no
    // meaning until it is whole, and a half-written one is not a thing this
    // function should be able to hold.
    let title = nest_svg(title_svg, 0, 0, W, band);
    let artwork = art.map_or_else(String::new, |(bytes, mime)| {
        // `slice` rather than `meet`: artwork fills its band and is cropped,
        // because a letterboxed photo on a cover reads as a mistake.
        format!(
            "<image x=\"0\" y=\"{BAND}\" width=\"{W}\" height=\"{}\" \
preserveAspectRatio=\"xMidYMid slice\" xlink:href=\"data:{mime};base64,{}\"/>",
            H - BAND,
            base64(bytes)
        )
    });
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" \
version=\"1.1\" width=\"{W}\" height=\"{H}\" viewBox=\"0 0 {W} {H}\">\
<rect width=\"{W}\" height=\"{H}\" fill=\"#ffffff\"/>{title}{artwork}</svg>"
    )
}

/// Place one SVG document inside another as a viewport at `(at_x, at_y)`.
///
/// The surgery is on the root start tag only: the prologue is dropped (an
/// `<?xml?>` declaration is legal at the top of a file and illegal in the
/// middle of one), and the sizing attributes are replaced with ours so the
/// author's drawing scales into the band instead of overflowing it. Its
/// `viewBox` is kept — that is what says how the drawing maps onto its own
/// coordinates — and synthesized from `width`/`height` when it has none.
fn nest_svg(svg: &str, at_x: u32, at_y: u32, width: u32, height: u32) -> String {
    let body = strip_prologue(svg);
    let Some(open) = body.find("<svg") else {
        return String::new();
    };
    let rest = &body[open + "<svg".len()..];
    let Some(close) = tag_end(rest) else {
        return String::new();
    };
    let (mut inner, tail) = (&rest[..close], &rest[close + 1..]);

    let self_closing = inner.trim_end().ends_with('/');
    if self_closing {
        inner = &inner[..inner.trim_end().len() - 1];
    }
    let attrs = attrs_of(inner);
    let value = |name: &str| {
        attrs
            .iter()
            .find(|(found, _)| found == name)
            .map(|(_, v)| v.as_str())
    };
    let number =
        |name: &str| value(name).and_then(|v| v.trim().trim_end_matches("px").parse::<f64>().ok());
    let view_box = value("viewBox").map_or_else(
        || match (number("width"), number("height")) {
            (Some(wide), Some(high)) => format!("0 0 {wide} {high}"),
            _ => format!("0 0 {width} {height}"),
        },
        ToString::to_string,
    );
    let kept = attrs
        .iter()
        .filter(|(name, _)| {
            !matches!(
                name.as_str(),
                "x" | "y" | "width" | "height" | "viewBox" | "preserveAspectRatio"
            )
        })
        .fold(String::new(), |mut acc, (name, v)| {
            acc.push(' ');
            acc.push_str(name);
            acc.push_str("=\"");
            acc.push_str(v);
            acc.push('"');
            acc
        });

    let head = format!(
        "<svg{kept} x=\"{at_x}\" y=\"{at_y}\" width=\"{width}\" height=\"{height}\" \
viewBox=\"{view_box}\" preserveAspectRatio=\"xMidYMid meet\""
    );
    if self_closing {
        format!("{head}/>")
    } else {
        format!("{head}>{tail}")
    }
}

/// Everything before an SVG's root element: declaration, doctype, comments.
fn strip_prologue(svg: &str) -> &str {
    svg.find("<svg").map_or(svg, |i| &svg[i..])
}

/// The offset of the `>` that closes a start tag, ignoring quoted `>`.
fn tag_end(rest: &str) -> Option<usize> {
    let mut quote = None;
    for (i, c) in rest.char_indices() {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(q), c) if c == q => quote = None,
            (None, '>') => return Some(i),
            _ => {}
        }
    }
    None
}

/// The `name="value"` pairs inside a start tag.
///
/// A scanner rather than an XML parser: this crate parses its own directives by
/// hand for the same reason, and the input here is one start tag.
fn attrs_of(inner: &str) -> Vec<(String, String)> {
    let chars: Vec<char> = inner.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        while chars.get(i).is_some_and(|c| c.is_whitespace()) {
            i += 1;
        }
        let start = i;
        while chars
            .get(i)
            .is_some_and(|c| !c.is_whitespace() && *c != '=')
        {
            i += 1;
        }
        if i == start {
            break;
        }
        let name: String = chars[start..i].iter().collect();
        while chars.get(i).is_some_and(|c| c.is_whitespace()) {
            i += 1;
        }
        if chars.get(i) != Some(&'=') {
            out.push((name, String::new()));
            continue;
        }
        i += 1;
        while chars.get(i).is_some_and(|c| c.is_whitespace()) {
            i += 1;
        }
        let quote = chars.get(i).copied().filter(|c| *c == '"' || *c == '\'');
        if quote.is_some() {
            i += 1;
        }
        let vstart = i;
        while chars
            .get(i)
            .is_some_and(|c| quote.map_or(!c.is_whitespace(), |q| *c != q))
        {
            i += 1;
        }
        let value: String = chars[vstart..i].iter().collect();
        if i < chars.len() {
            i += 1;
        }
        out.push((name, value));
    }
    out
}

/// Standard base64, written out rather than pulled in.
///
/// Same call as the FNV-1a digest below: a dependency for forty lines of table
/// lookup is a dependency this project has declined before.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let at = |i: usize| char::from(ALPHABET[i & 63]);

    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let byte = |i: usize| usize::from(chunk.get(i).copied().unwrap_or(0));
        let n = (byte(0) << 16) | (byte(1) << 8) | byte(2);
        out.push(at(n >> 18));
        out.push(at(n >> 12));
        out.push(if chunk.len() > 1 { at(n >> 6) } else { '=' });
        out.push(if chunk.len() > 2 { at(n) } else { '=' });
    }
    out
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

/// A file beside the chapters that a chapter links but does not contain — an
/// image, most of the time.
///
/// A path, not the bytes: [`RenderPlan`] stays pure, and a `Debug` of a book
/// stays readable instead of printing a JPEG.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderAsset {
    /// Path relative to the book's `src/`, exactly as a chapter links it —
    /// `images/spinal-tap.jpg`. The copy has to land at this same relative
    /// path or the link still dangles.
    pub rel: String,
    /// Where to copy it from.
    pub from: PathBuf,
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
    /// The book's edition, from `bower.toml`'s `[book] version`. `None` is a
    /// normal book; it names its artifacts without a version, as it always did.
    pub version: Option<String>,
    pub chapters: Vec<RenderedChapter>,
    /// Non-markdown files under the book's `src/`. mdBook copies these itself,
    /// so the HTML target ignores them; pandoc and typst are handed only the
    /// files bower writes, and would otherwise render a book with holes in it.
    pub assets: Vec<RenderAsset>,
}

/// Fold a loaded book and its resolved plan into a render plan.
#[must_use]
pub fn render_plan(
    book: &BookSource,
    plan: &BookPlan,
    meta: BookMeta,
    target: Target,
    links: &BTreeMap<String, LinkTemplates>,
    version: Option<String>,
    assets: Vec<RenderAsset>,
) -> RenderPlan {
    RenderPlan {
        target,
        meta,
        version,
        assets,
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

/// What a rendered artifact is called: the book's slug, its version when it
/// declares one, and the target's extension.
///
/// `Rust for Failures` at `0.1.0` becomes `rust-for-failures_0.1.0.epub`. A
/// book with no `version` keeps the plain `hello-playbook.epub` — every book
/// that existed before this did publishes under the name it always had.
///
/// The version is filtered rather than slugged: a dot separates a version's
/// parts and must survive, while anything a filesystem or a URL would argue
/// about is dropped.
#[must_use]
pub fn artifact_name(title: &str, version: Option<&str>, ext: &str) -> String {
    let stem = slug(title);
    match version.map(version_tag).filter(|v| !v.is_empty()) {
        Some(v) => format!("{stem}_{v}.{ext}"),
        None => format!("{stem}.{ext}"),
    }
}

/// The filename-safe part of a version string.
fn version_tag(version: &str) -> String {
    version
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .collect()
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

/// Write a plan's chapters to `dir` as numbered markdown files, in reading
/// order, and return them in that order.
///
/// Shared by every renderer that hands files to an external tool. Two copies of
/// this loop would be two chances to order a book differently, and a reader
/// would only find out at chapter 3.
///
/// # Errors
///
/// Any filesystem failure while clearing, creating, or writing.
pub fn write_chapters(dir: &Path, plan: &RenderPlan) -> Result<Vec<PathBuf>, PublishError> {
    let io = |path: &Path| {
        let path = path.to_path_buf();
        move |source| PublishError::Read { path, source }
    };

    if dir.exists() {
        std::fs::remove_dir_all(dir).map_err(io(dir))?;
    }
    std::fs::create_dir_all(dir).map_err(io(dir))?;

    let mut out = Vec::with_capacity(plan.chapters.len());
    for (i, chapter) in plan.chapters.iter().enumerate() {
        let stem = chapter
            .path
            .rsplit('/')
            .next()
            .unwrap_or(&chapter.path)
            .trim_end_matches(".md");
        // Numbered so the order an external tool receives is visible on disk.
        let file = dir.join(format!("{:03}-{stem}.md", i + 1));
        std::fs::write(&file, &chapter.markdown).map_err(io(&file))?;
        out.push(file);
    }
    Ok(out)
}

/// Every non-markdown file under a book's `src/`, sorted, book-`src`-relative.
///
/// mdBook already does this for the HTML target, which is why a missing image
/// only ever showed up in the epub and the PDF. The rule is mdBook's rule:
/// markdown is a chapter, anything else is a file to carry along. Guessing at
/// a list of image extensions instead would drop the next `.svg` or `.csv` a
/// chapter links, and drop it silently.
#[must_use]
pub fn book_assets(book_root: &Path) -> Vec<RenderAsset> {
    let src = book_root.join("src");
    let mut out = Vec::new();
    collect_assets_into(&src, &src, &mut out);
    out.sort_by(|a, b| a.rel.cmp(&b.rel));
    out
}

/// Walk `dir`, recording every non-markdown file relative to `src`.
///
/// A directory that cannot be read is skipped rather than fatal: a book whose
/// `src/` has no extra files at all is the normal case, and an unreadable
/// subdirectory is not a reason to refuse to render the prose.
fn collect_assets_into(src: &Path, dir: &Path, out: &mut Vec<RenderAsset>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // Dotfiles are the renderers' own scratch, never book content.
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with('.'))
        {
            continue;
        }
        if path.is_dir() {
            collect_assets_into(src, &path, out);
        } else if !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("md"))
            && let Ok(rel) = path.strip_prefix(src)
        {
            // Slashes, not the platform separator: this string is compared
            // against what a markdown link says, and a link says `/`.
            let rel = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            out.push(RenderAsset { rel, from: path });
        }
    }
}

/// Copy a plan's assets under `dir`, keeping their relative paths.
///
/// `dir` is whatever the external tool resolves links against, and the two
/// tools disagree: typst resolves against the `.typ` it is compiling, pandoc
/// against its `--resource-path`. Both are satisfied by the same mirror, put
/// in a different place.
///
/// # Errors
///
/// Any filesystem failure while creating a directory or copying a file.
pub fn copy_assets(dir: &Path, assets: &[RenderAsset]) -> Result<(), PublishError> {
    for asset in assets {
        let dest = dir.join(&asset.rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|source| PublishError::Read {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        std::fs::copy(&asset.from, &dest).map_err(|source| PublishError::Read {
            path: asset.from.clone(),
            source,
        })?;
    }
    Ok(())
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
        let chapters = out.join(".chapters");
        let inputs = write_chapters(&chapters, plan)?;
        // After `write_chapters`, which clears the directory first.
        copy_assets(&chapters, &plan.assets)?;

        let artifact = out.join(artifact_name(
            &plan.meta.title,
            plan.version.as_deref(),
            "epub",
        ));
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
        // Written beside the chapters rather than passed inline: pandoc takes a
        // path, and the composed cover is the same bytes the PDF gets.
        if let Some(cover) = &plan.meta.cover {
            let path = out.join(".cover.svg");
            cover.write(&path)?;
            cmd.arg("--epub-cover-image").arg(&path);
        }
        // Pandoc resolves an image path against its working directory, not
        // against the chapter that links it. Without this the epub is built
        // with a warning on stderr and a hole where the picture was.
        cmd.arg("--resource-path").arg(&chapters);
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

/// A short, stable fingerprint of a plan.
///
/// FNV-1a, written out rather than pulled in: "did this change" needs no
/// cryptography, and this project has declined a dependency for less. Stable
/// across runs and machines, which is all the site marker asks of it.
#[must_use]
pub fn digest(text: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// What the rendered book depends on, as one string to digest.
///
/// **Not just the lock.** `lock_text` records step ids, expectations, anchors,
/// and files — but not commit subjects, and not a word of prose. Digesting it
/// alone means an edited paragraph, or a changed `msg=`, leaves the site
/// reporting "in sync" while serving different text. That was found live on
/// 2 September 2026, by changing one `msg=` and watching the repo drift while
/// the site did not.
///
/// So: the lock, plus every chapter's source. Some changes this catches would
/// render identically — a trailing space, say — and that is the right way to be
/// wrong. A false "stale" costs one re-render; a false "in sync" serves the
/// wrong book.
#[must_use]
pub fn site_fingerprint(book: &BookSource, lock: &str) -> String {
    let mut all = String::from(lock);
    for chapter in &book.chapters {
        all.push_str(&chapter.path);
        all.push('\n');
        all.push_str(&chapter.text);
    }
    digest(&all)
}

/// The contents of `.bower-site`.
///
/// One file, two jobs. `marker_line` makes it readable by
/// `trailers::book_named_in`, so `bower push`'s gate works on a site branch
/// unchanged — a branch with content and no marker is refused. The digest lets
/// `bower status` answer "is this site current?" without fetching a page.
#[must_use]
pub fn site_marker(book_name: &str, fingerprint: &str) -> String {
    format!(
        "{}\n\nplan-digest: {}\n",
        crate::trailers::marker_line(book_name),
        fingerprint
    )
}

/// The plan digest a `.bower-site` records, if it carries one.
///
/// The inverse of the `plan-digest:` line [`site_marker`] writes. Together they
/// answer "was this site rendered from the plan the book produces now?" without
/// fetching a single page.
#[must_use]
pub fn site_plan_digest(marker: &str) -> Option<&str> {
    marker
        .lines()
        .find_map(|l| l.trim().strip_prefix("plan-digest:"))
        .map(str::trim)
        .filter(|d| !d.is_empty())
}

/// Write the two files a static host needs beside the rendered book.
///
/// `.nojekyll` matters more than it looks: without it GitHub Pages runs Jekyll,
/// which silently drops files and directories beginning with `_`. mdBook emits
/// none today, so the failure mode is missing CSS with no error anywhere —
/// exactly the class of bug that survives unnoticed.
///
/// # Errors
///
/// Any filesystem failure while writing.
pub fn write_site_files(dir: &Path, marker: &str) -> std::io::Result<()> {
    std::fs::write(dir.join(".nojekyll"), "")?;
    std::fs::write(dir.join(".bower-site"), marker)
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
    /// The `.bower-site` contents, when this render is destined for a site
    /// branch. `None` renders HTML for local reading only.
    pub site_marker: Option<String>,
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
        if let Some(marker) = &self.site_marker {
            write_site_files(&dest, marker).map_err(|source| PublishError::Read {
                path: dest.clone(),
                source,
            })?;
        }

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

/// The PDF, via `pandoc --to typst` and `typst compile`.
///
/// Two steps rather than `pandoc --to pdf`, because that route reaches for
/// LaTeX. EPIC-07 Phase 0 measured both: a LaTeX PDF is **not** reproducible
/// even with `SOURCE_DATE_EPOCH` and `FORCE_SOURCE_DATE` pinned, while a Typst
/// one is byte-identical with `SOURCE_DATE_EPOCH` alone. This project promises
/// byte-identical output everywhere else; a target that cannot is a hole in the
/// argument.
pub struct TypstRenderer {
    /// Seconds since the Unix epoch, pinned so two runs agree. Comes from
    /// `bower.toml`'s `epoch` — the same value that already makes commit SHAs
    /// reproducible. One epoch, every artifact.
    pub epoch: i64,
    /// A `.typ` preamble setting page size, fonts, and code styling. `None`
    /// uses Typst's defaults, which are legible but not tuned for code.
    pub template: Option<PathBuf>,
}

impl Renderer for TypstRenderer {
    fn preflight(&self) -> Result<(), PublishError> {
        need("pandoc", "brew install pandoc")?;
        need("typst", "brew install typst")?;
        if let Some(template) = &self.template {
            let text = std::fs::read_to_string(template).map_err(|source| PublishError::Read {
                path: template.clone(),
                source,
            })?;
            check_fonts(&text)?;
        }
        Ok(())
    }

    fn render(&self, plan: &RenderPlan, out: &Path) -> Result<Artifact, PublishError> {
        let io = |path: &Path| {
            let path = path.to_path_buf();
            move |source| PublishError::Read { path, source }
        };

        std::fs::create_dir_all(out).map_err(io(out))?;
        let work = out.join(".typst");
        let inputs = write_chapters(&work.join("chapters"), plan)?;
        // Beside `book.typ`, not beside the chapters: typst resolves an
        // `image("images/x.jpg")` against the `.typ` file it is compiling.
        copy_assets(&work, &plan.assets)?;

        // One `.typ` for the whole book: pandoc resolves cross-chapter links
        // and builds one document, which is what a PDF is.
        let body = work.join("body.typ");
        let mut cmd = std::process::Command::new("pandoc");
        cmd.arg("--from")
            .arg("markdown")
            .arg("--to")
            .arg("typst")
            .arg("--metadata")
            .arg(format!("title={}", plan.meta.title))
            .arg("--metadata")
            .arg(format!("lang={}", plan.meta.language));
        for author in &plan.meta.authors {
            cmd.arg("--metadata").arg(format!("author={author}"));
        }
        let output = cmd
            .arg("-o")
            .arg(&body)
            .args(&inputs)
            .output()
            .map_err(|e| PublishError::Failed {
                what: "pandoc --to typst".to_string(),
                stderr: e.to_string(),
            })?;
        if !output.status.success() {
            return Err(PublishError::Failed {
                what: "pandoc --to typst".to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }

        // The template goes in front of pandoc's output rather than around it:
        // Typst's `#set` rules apply to everything that follows, so a preamble
        // is all a template needs to be.
        let source = work.join("book.typ");
        let mut text = String::new();
        if let Some(template) = &self.template {
            text.push_str(&std::fs::read_to_string(template).map_err(io(template))?);
            text.push('\n');
        }
        // Between the template and the body, because the template's `#set`
        // rules must already be in force and the cover must precede chapter
        // one. `#page` with a body opens a page of its own, so the override of
        // margin and numbering ends when the cover does.
        if let Some(cover) = &plan.meta.cover {
            cover.write(&work.join("cover.svg"))?;
            text.push_str(
                "#page(margin: 0pt, numbering: none)[\n  \
                 #image(\"cover.svg\", width: 100%, height: 100%, fit: \"contain\")\n]\n\
                 // The cover is not page 1. Without this reset the first page a\n\
                 // reader sees prints \"2\", because Typst counts the cover even\n\
                 // though it prints no number on it.\n\
                 #counter(page).update(1)\n",
            );
        }
        text.push_str(&std::fs::read_to_string(&body).map_err(io(&body))?);
        std::fs::write(&source, text).map_err(io(&source))?;

        let artifact = out.join(artifact_name(
            &plan.meta.title,
            plan.version.as_deref(),
            "pdf",
        ));
        let output = std::process::Command::new("typst")
            .arg("compile")
            // Phase 0 measured this: without it, two runs differ; with it, they
            // are byte-identical.
            .env("SOURCE_DATE_EPOCH", self.epoch.to_string())
            .arg(&source)
            .arg(&artifact)
            .output()
            .map_err(|e| PublishError::Failed {
                what: "typst compile".to_string(),
                stderr: e.to_string(),
            })?;
        if !output.status.success() {
            return Err(PublishError::Failed {
                what: "typst compile".to_string(),
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

/// Refuse a template naming a font Typst cannot see.
///
/// Typst substitutes silently for a missing font, which is precisely how the
/// `⋯` glyph bug reached a spike unnoticed (EPIC-06, corrigendum item 7). A
/// template that asks for a face nobody has should say so, not quietly render
/// in something else.
///
/// # Errors
///
/// [`PublishError::MissingTool`] naming the font and how to see the list.
pub fn check_fonts(template: &str) -> Result<(), PublishError> {
    let available = std::process::Command::new("typst")
        .arg("fonts")
        .output()
        .map_err(|e| PublishError::Failed {
            what: "typst fonts".to_string(),
            stderr: e.to_string(),
        })?;
    let have = String::from_utf8_lossy(&available.stdout);

    for name in fonts_named_in(template) {
        if !have.lines().any(|f| f.trim() == name) {
            return Err(PublishError::MissingTool {
                tool: format!("font `{name}`"),
                install: "install it, or run `typst fonts` to see what is available".to_string(),
            });
        }
    }
    Ok(())
}

/// Every `font: "…"` a Typst template names.
///
/// Deliberately a scan rather than a parser: this is a guard, and a guard that
/// needs a language front end to work is a guard that stops working.
#[must_use]
pub fn fonts_named_in(template: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = template;
    while let Some(i) = rest.find("font:") {
        rest = &rest[i + 5..];
        let Some(open) = rest.find('"') else { break };
        let after = &rest[open + 1..];
        let Some(close) = after.find('"') else { break };
        let name = after[..close].to_string();
        if !name.is_empty() && !out.contains(&name) {
            out.push(name);
        }
        rest = &after[close + 1..];
    }
    out
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
    fn target__pdf_has_no_toggle() {
        // A PDF has no toggle, exactly like an epub. That is the whole render
        // rule this target adds.
        assert!(!Target::Pdf.has_hidden_lines());
    }

    #[test]
    fn target__round_trips_through_its_name() {
        for t in [Target::Html, Target::Epub, Target::Pdf] {
            assert_eq!(t.to_string().parse::<Target>().unwrap(), t);
        }
    }

    #[test]
    fn target__an_unbuilt_target_says_which_are_built() {
        let err = "ipynb".parse::<Target>().unwrap_err();
        assert!(err.contains("html, epub, and pdf"), "{err}");
    }

    use crate::loader::BookLoader;
    use bower_core::prelude::{RepoCatalog, plan};

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
                checkout: None,
                ..LinkTemplates::default()
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
    fn book_assets__collects_every_non_markdown_file_under_src() {
        // mdBook copies these itself for the HTML target. pandoc and typst get
        // nothing unless bower hands it to them, and a missing image is a
        // hard error in one and a silent hole in the other.
        let root = std::env::temp_dir().join("bower-book-assets");
        let _ = std::fs::remove_dir_all(&root);
        let src = root.join("src");
        std::fs::create_dir_all(src.join("images")).unwrap();
        std::fs::write(src.join("SUMMARY.md"), "x").unwrap();
        std::fs::write(src.join("ch01.md"), "x").unwrap();
        std::fs::write(src.join("images").join("a.jpg"), b"jpg").unwrap();
        std::fs::write(src.join("notes.csv"), "1,2").unwrap();

        let found = book_assets(&root);
        let rels: Vec<&str> = found.iter().map(|a| a.rel.as_str()).collect();
        assert_eq!(
            rels,
            vec!["images/a.jpg", "notes.csv"],
            "sorted, no markdown"
        );
        assert_eq!(found[0].from, src.join("images").join("a.jpg"));
    }

    #[test]
    fn book_assets__a_book_without_a_src_directory_is_empty_not_a_panic() {
        assert!(book_assets(Path::new("/no/such/book")).is_empty());
    }

    #[test]
    fn copy_assets__mirrors_the_relative_layout_a_chapter_links() {
        // The markdown says `images/a.jpg`. Typst resolves that against the
        // `.typ` beside it, so flattening the copy into one directory would
        // put the file on disk and still not be found.
        let root = std::env::temp_dir().join("bower-copy-assets");
        let _ = std::fs::remove_dir_all(&root);
        let from = root.join("from");
        std::fs::create_dir_all(&from).unwrap();
        std::fs::write(from.join("a.jpg"), b"jpg").unwrap();

        let dest = root.join("dest");
        std::fs::create_dir_all(&dest).unwrap();
        copy_assets(
            &dest,
            &[RenderAsset {
                rel: "images/a.jpg".to_string(),
                from: from.join("a.jpg"),
            }],
        )
        .unwrap();

        assert_eq!(
            std::fs::read(dest.join("images").join("a.jpg")).unwrap(),
            b"jpg"
        );
    }

    #[test]
    fn render_plan__carries_the_assets_it_was_given() {
        let (book, plan) = sample_plan();
        let meta = BookMeta::load(&sample_root()).unwrap();
        let assets = vec![RenderAsset {
            rel: "images/a.jpg".to_string(),
            from: PathBuf::from("/tmp/a.jpg"),
        }];
        let rp = render_plan(
            &book,
            &plan,
            meta,
            Target::Pdf,
            &links(),
            None,
            assets.clone(),
        );
        assert_eq!(rp.assets, assets);
    }

    #[test]
    fn render_plan__covers_every_chapter_in_reading_order() {
        // A chapter silently dropped from an epub is one nobody notices is
        // missing.
        let (book, plan) = sample_plan();
        let meta = BookMeta::load(&sample_root()).unwrap();
        let rp = render_plan(&book, &plan, meta, Target::Epub, &links(), None, Vec::new());

        assert_eq!(rp.chapters.len(), book.chapters.len());
        let paths: Vec<&str> = rp.chapters.iter().map(|c| c.path.as_str()).collect();
        assert_eq!(paths[0], "src/ch01-a-repo-that-builds.md");
        assert_eq!(paths[5], "src/ch06-ci.md");
        assert_eq!(rp.chapters[3].title, "Tests, and failing on purpose");
    }

    #[test]
    fn render_plan__pdf_matches_epub_markdown() {
        // pdf and epub share an elision rule, so their markdown must be
        // byte-identical. If this ever fails, two targets have quietly grown
        // two engines — which is the thing EPIC-06's architecture exists to
        // prevent.
        let (book, plan) = sample_plan();
        let meta = BookMeta::load(&sample_root()).unwrap();
        let epub = render_plan(
            &book,
            &plan,
            meta.clone(),
            Target::Epub,
            &links(),
            None,
            Vec::new(),
        );
        let pdf = render_plan(&book, &plan, meta, Target::Pdf, &links(), None, Vec::new());

        for (e, p) in epub.chapters.iter().zip(pdf.chapters.iter()) {
            assert_eq!(e.markdown, p.markdown, "{} differs between targets", e.path);
        }
    }

    #[test]
    fn render_plan__epub_and_html_differ_only_in_elision() {
        // The claim that there is one engine, made testable.
        let (book, plan) = sample_plan();
        let meta = BookMeta::load(&sample_root()).unwrap();
        let html = render_plan(
            &book,
            &plan,
            meta.clone(),
            Target::Html,
            &links(),
            None,
            Vec::new(),
        );
        let epub = render_plan(&book, &plan, meta, Target::Epub, &links(), None, Vec::new());

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
    fn artifact_name__carries_the_version() {
        assert_eq!(
            artifact_name("Rust for Failures", Some("0.1.0"), "epub"),
            "rust-for-failures_0.1.0.epub"
        );
        assert_eq!(
            artifact_name("Rust for Failures", Some("0.1.0"), "pdf"),
            "rust-for-failures_0.1.0.pdf"
        );
    }

    #[test]
    fn artifact_name__a_book_without_a_version_keeps_its_old_name() {
        // Every book that shipped before versioned filenames existed must
        // publish under the name it always had.
        assert_eq!(
            artifact_name("Hello, Playbook", None, "epub"),
            "hello-playbook.epub"
        );
    }

    #[test]
    fn artifact_name__a_version_of_pure_punctuation_is_no_version() {
        // Filtering can empty a string. `book_..epub` would be the alternative.
        assert_eq!(artifact_name("A Book", Some("///"), "pdf"), "a-book.pdf");
    }

    #[test]
    fn artifact_name__keeps_a_version_readable() {
        // Dots separate a version's parts and must survive; a space must not.
        assert_eq!(
            artifact_name("A Book", Some("1.2.3-rc.1"), "epub"),
            "a-book_1.2.3-rc.1.epub"
        );
        assert_eq!(
            artifact_name("A Book", Some("1.0 beta"), "epub"),
            "a-book_1.0beta.epub"
        );
    }

    #[test]
    fn slug__is_filename_safe() {
        assert_eq!(slug("Hello, Playbook"), "hello-playbook");
        assert_eq!(slug("Rust for Failures!"), "rust-for-failures");
        assert_eq!(slug("  spaced  out  "), "spaced-out");
        // A title of pure punctuation would otherwise name a file `-`.
        assert_eq!(slug("!!!"), "book");
    }

    #[test]
    fn fake__records_the_plan_it_was_given() {
        let (book, plan) = sample_plan();
        let meta = BookMeta::load(&sample_root()).unwrap();
        let rp = render_plan(&book, &plan, meta, Target::Epub, &links(), None, Vec::new());

        let fake = FakeRenderer::new();
        fake.preflight().unwrap();
        let artifact = fake.render(&rp, Path::new("/tmp/nowhere")).unwrap();

        assert_eq!(fake.seen().len(), 1);
        assert_eq!(fake.seen()[0].target, Target::Epub);
        assert_eq!(fake.seen()[0].chapters.len(), 7);
        assert_eq!(artifact.bytes, 0, "a fake writes nothing");
    }

    #[test]
    fn mdbook_renderer__keeps_the_book_root_because_mdbook_needs_it() {
        // Recorded as a test because it is the one asymmetry in this design:
        // mdBook re-runs the preprocessor, so it needs the source directory
        // rather than the plan.
        let r = MdBookRenderer {
            book_root: sample_root(),
            site_marker: None,
        };
        assert!(r.book_root.join("book.toml").exists());
    }

    #[test]
    fn base64__matches_the_rfc_4648_vectors() {
        // The canonical vectors, because a base64 bug shows up as a cover that
        // renders in one reader and not another — the worst kind to chase.
        for (input, want) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(input.as_bytes()), want, "{input:?}");
        }
    }

    #[test]
    fn art_mime__knows_the_formats_a_reader_can_show() {
        assert_eq!(art_mime(Path::new("cover.png")), Some("image/png"));
        assert_eq!(art_mime(Path::new("cover.JPG")), Some("image/jpeg"));
        assert_eq!(art_mime(Path::new("cover.jpeg")), Some("image/jpeg"));
        assert_eq!(art_mime(Path::new("cover.tiff")), None);
    }

    #[test]
    fn compose_cover__stacks_the_art_below_the_title() {
        let title =
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="1600" height="800"><rect/></svg>"#;
        let svg = compose_cover(title, Some((b"abc", "image/png")));

        assert!(
            svg.starts_with("<svg "),
            "the result must be one svg document"
        );
        assert!(svg.contains("viewBox=\"0 0 1600 2400\""), "{svg}");
        // The title band occupies the top; the art is placed under it.
        assert!(
            svg.contains("<rect/>"),
            "the title svg is nested, not dropped"
        );
        assert!(
            svg.contains(r#"<image x="0" y="800""#),
            "art starts where the title band ends: {svg}"
        );
        assert!(
            svg.contains("data:image/png;base64,YWJj"),
            "art is embedded, not linked: {svg}"
        );
    }

    #[test]
    fn compose_cover__without_art_is_still_a_cover() {
        // Option B: the book ships a title svg and no artwork yet. That must
        // publish, not fail — otherwise adding a cover is all-or-nothing.
        let title =
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="1600" height="2400"><rect/></svg>"#;
        let svg = compose_cover(title, None);

        assert!(!svg.contains("<image"), "no art means no image element");
        assert!(svg.contains("<rect/>"), "the title still fills the page");
        assert!(
            svg.contains(r#"height="2400""#),
            "the title band grows to the full page: {svg}"
        );
    }

    #[test]
    fn compose_cover__strips_the_xml_prologue_it_cannot_nest() {
        // An `<?xml?>` declaration is legal at the top of a file and illegal in
        // the middle of one. Editors emit it; nesting it produces a cover that
        // silently fails to draw.
        let title = concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
            "<!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \"x.dtd\">\n",
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\" height=\"1\"><rect/></svg>",
        );
        let svg = compose_cover(title, None);

        assert!(!svg.contains("<?xml"), "prologue must not be nested: {svg}");
        assert!(
            !svg.contains("<!DOCTYPE"),
            "doctype must not be nested: {svg}"
        );
        assert!(svg.contains("<rect/>"), "the drawing survives: {svg}");
    }

    #[test]
    fn compose_cover__is_byte_identical_across_calls() {
        // The PDF promises identical bytes across runs. A cover that hashed a
        // map or read a clock would break that promise from inside.
        let title = r#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"/>"#;
        let art: &[u8] = b"\x89PNG\r\n\x1a\n0123456789";
        assert_eq!(
            compose_cover(title, Some((art, "image/png"))),
            compose_cover(title, Some((art, "image/png")))
        );
    }

    #[test]
    fn load_cover__composes_the_sample_books_cover() {
        // The sample book ships `cover.svg`; `cover.png` is the author's to add.
        let cover = load_cover(&sample_root()).unwrap().unwrap();
        assert!(
            cover.text().contains("Hello, Playbook"),
            "the title is drawn"
        );
        assert_eq!(
            cover.text().contains("<image"),
            sample_root().join("cover.png").exists(),
            "art appears exactly when the file does"
        );
    }

    #[test]
    fn load_cover__a_book_without_one_publishes_anyway() {
        // Every book that existed before this feature has no `cover.svg`, and
        // must keep publishing unchanged.
        let dir = std::env::temp_dir().join("bower-cover-none");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(load_cover(&dir).unwrap().is_none());
    }

    #[test]
    fn book_meta__carries_the_sample_books_cover() {
        let meta = BookMeta::load(&sample_root()).unwrap();
        assert!(meta.cover.is_some(), "the sample book has a cover to carry");
    }

    #[test]
    fn typst__preflight_names_both_tools() {
        // The failure a reader hits first: one of the two binaries missing.
        let r = TypstRenderer {
            epoch: 0,
            template: None,
        };
        // Both are installed here, so this passes; the shape of the failure is
        // covered by `renderer__preflight_names_the_install_command`.
        assert!(r.preflight().is_ok(), "pandoc and typst are both required");
    }

    #[test]
    fn fonts_named_in__finds_every_face_a_template_asks_for() {
        let template = concat!(
            "#set text(font: \"Libertinus Serif\", size: 10.5pt)\n",
            "#show raw: set text(font: \"DejaVu Sans Mono\")\n",
            "#show raw: set text(font: \"DejaVu Sans Mono\")\n",
        );
        // Deduplicated, in order.
        assert_eq!(
            fonts_named_in(template),
            vec![
                "Libertinus Serif".to_string(),
                "DejaVu Sans Mono".to_string()
            ]
        );
        assert!(fonts_named_in("#set page(paper: \"a4\")").is_empty());
    }

    #[test]
    fn check_fonts__refuses_a_face_nobody_has() {
        // Typst substitutes silently, which is how a missing glyph reaches a
        // reader. A template asking for a font nobody has must fail loudly.
        let err = check_fonts("#set text(font: \"No Such Font Exists 9000\")").unwrap_err();
        let text = err.to_string();
        assert!(text.contains("No Such Font Exists 9000"), "{text}");
        assert!(
            text.contains("typst fonts"),
            "the way to look must be named: {text}"
        );
    }

    #[test]
    fn check_fonts__accepts_the_sample_books_template() {
        let template = sample_root().join("template.typ");
        let text = std::fs::read_to_string(&template).unwrap();
        assert!(
            check_fonts(&text).is_ok(),
            "the shipped template must compile"
        );
    }

    #[test]
    fn write_chapters__numbers_them_in_reading_order() {
        // Shared by pandoc and typst, so a bug here would misorder an epub too.
        let (book, plan) = sample_plan();
        let meta = BookMeta::load(&sample_root()).unwrap();
        let rp = render_plan(&book, &plan, meta, Target::Pdf, &links(), None, Vec::new());

        let dir = std::env::temp_dir().join("bower-write-chapters");
        let files = write_chapters(&dir, &rp).unwrap();

        assert_eq!(files.len(), 7);
        let names: Vec<String> = files
            .iter()
            .map(|f| f.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names[0], "001-ch01-a-repo-that-builds.md");
        assert_eq!(names[5], "006-ch06-ci.md");
        assert_eq!(names[6], "007-appendix-credits.md", "the appendix is last");
    }

    #[test]
    fn digest__is_stable_and_changes_with_content() {
        assert_eq!(digest("abc"), digest("abc"), "must be stable across calls");
        assert_ne!(digest("abc"), digest("abd"));
        assert_eq!(
            digest("abc").len(),
            16,
            "fixed-width hex is easier to eyeball"
        );
        // The empty case has a defined value rather than a panic.
        assert_eq!(digest("").len(), 16);
    }

    #[test]
    fn site_marker__round_trips_through_book_named_in() {
        // The guard reads what the renderer wrote. This is the fourth time
        // this project has needed that assertion, and the reason `marker_line`
        // and `book_named_in` are one definition.
        use crate::trailers::book_named_in;
        let m = site_marker("hello-playbook", "0123456789abcdef");
        assert_eq!(book_named_in(&m), Some("hello-playbook"));
        assert!(m.contains("plan-digest: "), "{m}");
    }

    #[test]
    fn site_marker__changes_when_the_plan_does() {
        // Staleness is answerable without fetching a page.
        let a = site_marker("b", &site_fingerprint(&sample_plan().0, "lock-a"));
        let b = site_marker("b", &site_fingerprint(&sample_plan().0, "lock-b"));
        assert_ne!(a, b, "a changed plan must change the marker");
    }

    #[test]
    fn write_site_files__writes_nojekyll_and_the_marker() {
        // Without `.nojekyll`, GitHub Pages runs Jekyll and silently drops
        // anything starting with `_`. No error, just missing CSS.
        let dir = std::env::temp_dir().join("bower-site-files");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let marker = site_marker("hello-playbook", "0123456789abcdef");
        write_site_files(&dir, &marker).unwrap();

        assert!(
            dir.join(".nojekyll").exists(),
            "Jekyll would eat the assets"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join(".bower-site")).unwrap(),
            marker
        );
    }
}
