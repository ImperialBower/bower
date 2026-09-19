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

use bower_core::prelude::{BookSource, Chapter, PartError, PartMark};

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
    /// A `SUMMARY.md` link points outside the book. Refused rather than
    /// skipped: a book that asks to read `/etc/passwd` is not a book with a
    /// typo, and quietly omitting the chapter would hide that.
    UnsafeLink { link: String, reason: String },
    /// A part title with no chapter under it — almost always a heading left
    /// behind after a chapter moved. mdBook would draw a bare label; Bower
    /// refuses it, so the epub and the PDF never get a part page with
    /// nothing behind it.
    EmptyPart { title: String },
    /// The part titles do not fit the chapters.
    Parts(PartError),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => write!(f, "cannot read {}: {source}", path.display()),
            Self::NoChapters { path } => {
                write!(f, "{} lists no chapters", path.display())
            }
            Self::UnsafeLink { link, reason } => {
                write!(f, "SUMMARY.md links outside the book: `{link}` — {reason}")
            }
            Self::EmptyPart { title } => write!(
                f,
                "SUMMARY.md: part `{title}` has no chapter under it; link one, or remove the title"
            ),
            Self::Parts(e) => write!(f, "SUMMARY.md: {e}"),
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
    /// Returns [`LoadError`] if the summary or any chapter cannot be read, if
    /// the summary links no chapters, or if a part title has no chapter
    /// under it.
    pub fn load(&self) -> Result<BookSource, LoadError> {
        let summary_path = self.root.join("src").join("SUMMARY.md");
        let summary_text = read(&summary_path)?;

        let Summary { links, parts } = summary(&summary_text);
        if links.is_empty() {
            return Err(LoadError::NoChapters { path: summary_path });
        }
        let mut marks = Vec::with_capacity(parts.len());
        for (title, first) in parts {
            let Some(first) = first else {
                return Err(LoadError::EmptyPart { title });
            };
            marks.push(PartMark {
                title,
                first: format!("src/{first}"),
            });
        }

        let mut chapters = Vec::with_capacity(links.len());
        for link in links {
            // A `SUMMARY.md` is book content, so its links are as untrusted as
            // any `file="…"` directive.
            crate::materialize::check_path(&link).map_err(|e| LoadError::UnsafeLink {
                link: link.clone(),
                reason: e.to_string(),
            })?;
            let on_disk = self.root.join("src").join(&link);
            let text = read(&on_disk)?;
            chapters.push(Chapter::new(&format!("src/{link}"), &text));
        }

        // The block library (`include=`) and binary assets (`op="copy"`) are
        // left empty: no chapter in any current book uses them, and inventing
        // a directory convention before a book needs one would be guesswork.
        BookSource::from_chapters(chapters)
            .with_parts(marks)
            .map_err(LoadError::Parts)
    }
}

fn read(path: &Path) -> Result<String, LoadError> {
    std::fs::read_to_string(path).map_err(|source| LoadError::Read {
        path: path.to_path_buf(),
        source,
    })
}

/// What `SUMMARY.md` says: the chapters, and where each part opens.
#[derive(Debug, Default, Eq, PartialEq)]
struct Summary {
    links: Vec<String>,
    /// A title, and the link that followed it. `None` is an empty part.
    parts: Vec<(String, Option<String>)>,
}

/// Every local `.md` target of a markdown link, in document order, without
/// repeats, and every part title. Deliberately not a full markdown parser:
/// `SUMMARY.md` is a list of links by definition, and a chapter listed twice
/// would be planned twice.
///
/// A part title is an `# H1`, read where mdBook reads one (measured against
/// mdBook 0.4.52): the first H1 is the summary's own title when nothing but
/// blank lines comes before it, and every later one is a part title. A title
/// is pending until the next new link claims it; one still pending at the
/// next title, or at the end of the file, is an empty part.
fn summary(text: &str) -> Summary {
    let mut out = Summary::default();
    let mut pending: Option<String> = None;
    let mut only_blanks = true;
    for line in text.lines() {
        if let Some(title) = h1(line)
            && !only_blanks
        {
            if let Some(empty) = pending.take() {
                out.parts.push((empty, None));
            }
            pending = Some(title.to_string());
        }
        only_blanks &= line.trim().is_empty();
        let mut rest = line;
        while let Some(i) = rest.find("](") {
            let after = &rest[i + 2..];
            let Some(j) = after.find(')') else { break };
            let target = after[..j].trim();
            if is_markdown(target)
                && !target.contains("://")
                && !out.links.iter().any(|seen| seen == target)
            {
                out.links.push(target.to_string());
                if let Some(title) = pending.take() {
                    out.parts.push((title, Some(target.to_string())));
                }
            }
            rest = &after[j + 1..];
        }
    }
    if let Some(empty) = pending {
        out.parts.push((empty, None));
    }
    out
}

/// The text of an ATX level-one heading: up to three spaces, `#`, then a
/// space or the end of the line. A closing run of `#` is dropped, as
/// `CommonMark` drops it. `#Title` and `## Title` are not level-one headings.
fn h1(line: &str) -> Option<&str> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 {
        return None;
    }
    let rest = line[indent..].strip_prefix('#')?;
    if !(rest.is_empty() || rest.starts_with([' ', '\t'])) {
        return None;
    }
    let title = rest.trim();
    let closed = title.trim_end_matches('#');
    Some(if closed.is_empty() || closed.ends_with([' ', '\t']) {
        closed.trim_end()
    } else {
        title
    })
}

/// Every chapter link in `SUMMARY.md`, in order, without repeats.
#[cfg(test)]
fn chapter_links(text: &str) -> Vec<String> {
    summary(text).links
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

    fn parts(text: &str) -> Vec<(String, Option<String>)> {
        summary(text).parts
    }

    fn part(title: &str, first: Option<&str>) -> (String, Option<String>) {
        (title.to_string(), first.map(str::to_string))
    }

    #[test]
    fn links__skip_part_headings() {
        let flat = "# Summary\n\n- [One](ch01.md)\n- [Two](ch02.md)\n";
        let parted = concat!(
            "# Summary\n\n",
            "# Part I: Start\n\n- [One](ch01.md)\n\n",
            "# Part II: Finish\n\n- [Two](ch02.md)\n",
        );
        assert_eq!(chapter_links(parted), chapter_links(flat));
    }

    #[test]
    fn summary__first_heading_is_the_title_not_a_part() {
        // mdBook reads the first H1 as the title whatever it says, even
        // `# Part One` with no `# Summary` above it.
        assert!(parts("# Summary\n\n- [One](ch01.md)\n").is_empty());
        assert_eq!(
            parts("\n# Part One\n\n- [A](a.md)\n\n# Part Two\n\n- [B](b.md)\n"),
            vec![part("Part Two", Some("b.md"))]
        );
        // Anything but blank lines before it, and it is a part.
        assert_eq!(
            parts("Intro.\n\n# Part One\n\n- [A](a.md)\n"),
            vec![part("Part One", Some("a.md"))]
        );
    }

    #[test]
    fn summary__a_part_opens_at_the_next_new_link() {
        let text = concat!(
            "# Summary\n\n",
            "[Intro](intro.md)\n\n",
            "# Part I: Start ##\n\n- [One](ch01.md)\n- [Two](ch02.md)\n\n",
            "   # Part II: C#\n- [Three](ch03.md)\n",
        );
        assert_eq!(
            parts(text),
            vec![
                part("Part I: Start", Some("ch01.md")),
                part("Part II: C#", Some("ch03.md")),
            ]
        );
    }

    #[test]
    fn summary__a_part_before_a_repeat_link_is_empty() {
        let text = "# Summary\n\n- [One](ch01.md)\n\n# Again\n\n- [One](ch01.md)\n";
        assert_eq!(parts(text), vec![part("Again", None)]);
    }

    #[test]
    fn summary__two_titles_in_a_row_leave_the_first_empty() {
        let text = "# Summary\n\n# P1\n\n# P2\n\n- [A](a.md)\n";
        assert_eq!(
            parts(text),
            vec![part("P1", None), part("P2", Some("a.md"))]
        );
    }

    #[test]
    fn summary__a_trailing_title_is_empty() {
        let text = "# Summary\n\n- [A](a.md)\n\n# P1\n";
        assert_eq!(parts(text), vec![part("P1", None)]);
    }

    #[test]
    fn summary__a_deeper_heading_is_not_a_part() {
        let text = "# Summary\n\n## Sub\n\n#NoSpace\n\n    # Code\n\n- [A](a.md)\n";
        assert!(parts(text).is_empty());
    }

    #[test]
    fn loader__refuses_an_empty_part_by_title() {
        let dir = std::env::temp_dir().join("bower-loader-empty-part");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src").join("a.md"), "# A\n").unwrap();
        std::fs::write(
            dir.join("src").join("SUMMARY.md"),
            "# Summary\n\n# Part I: Moved away\n\n# Part II: Here\n\n- [A](a.md)\n",
        )
        .unwrap();

        let err = BookLoader::new(&dir).load().unwrap_err();
        assert!(matches!(err, LoadError::EmptyPart { .. }), "{err:?}");
        assert!(err.to_string().contains("`Part I: Moved away`"), "{err}");
    }

    #[test]
    fn loader__reads_part_marks_src_prefixed() {
        let dir = std::env::temp_dir().join("bower-loader-parts");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src").join("a.md"), "# A\n").unwrap();
        std::fs::write(dir.join("src").join("b.md"), "# B\n").unwrap();
        std::fs::write(
            dir.join("src").join("SUMMARY.md"),
            "# Summary\n\n- [A](a.md)\n\n# Part I: B\n\n- [B](b.md)\n",
        )
        .unwrap();

        let book = BookLoader::new(&dir).load().unwrap();
        assert_eq!(book.parts, vec![PartMark::new("Part I: B", "src/b.md")]);
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
                "src/ch07-try-it-on-a-branch.md",
                "src/appendix-credits.md",
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
    fn loader__refuses_a_summary_link_that_climbs_out() {
        let dir = std::env::temp_dir().join("bower-loader-escape");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("src").join("SUMMARY.md"),
            "# Summary\n\n- [Out](../../../../etc/hosts.md)\n",
        )
        .unwrap();

        let err = BookLoader::new(&dir).load().unwrap_err();
        assert!(matches!(err, LoadError::UnsafeLink { .. }), "{err:?}");
    }

    #[test]
    fn loader__missing_book_is_an_error() {
        let err = BookLoader::new(Path::new("/nonexistent-book"))
            .load()
            .unwrap_err();
        assert!(matches!(err, LoadError::Read { .. }), "{err:?}");
    }
}
