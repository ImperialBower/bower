//! `bower publish` — the render plan, and the artifacts it produces.
//!
//! The fast cases assert on the **plan**, with no renderer installed and no
//! binary artifact to inspect. That is what the pure fold buys: the claim that
//! epub and html share one display engine is checked in milliseconds.
//!
//! The end-to-end epub is `#[ignore]`d, because it needs pandoc. Run it with:
//!
//! ```text
//! cargo test -p bower --test publish -- --ignored
//! ```

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use bower::config::BookConfig;
use bower::loader::BookLoader;
use bower::publish::{BookMeta, RenderPlan, Target, artifact_name, book_assets, render_plan};
use bower_core::prelude::plan;

fn book_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("books")
        .join("hello-playbook")
}

fn scratch(case: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bower-publish-{case}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// What the book itself says a rendered artifact is called.
///
/// Asked of the plan rather than written out here: the name carries the book's
/// version, so `hello-playbook.epub` became `hello-playbook_0.1.0.epub` the day
/// `bower.toml` declared one — and a pasted-in name inside an `#[ignore]`d test
/// reports that as a failure long after the change that caused it.
fn artifact(target: Target, ext: &str) -> String {
    let plan = plan_for(target);
    artifact_name(&plan.meta.title, plan.version.as_deref(), ext)
}

fn plan_for(target: Target) -> RenderPlan {
    let cfg = BookConfig::load(&book_root()).unwrap();
    let book = BookLoader::new(&book_root()).load().unwrap();
    let resolved = plan(&book, &cfg.catalog()).unwrap();
    let meta = BookMeta::load(&book_root()).unwrap();
    let links: BTreeMap<_, _> = cfg
        .repos
        .iter()
        .map(|(n, r)| (n.clone(), r.links.clone()))
        .collect();
    render_plan(
        &book,
        &resolved,
        meta,
        target,
        &links,
        cfg.version.clone(),
        book_assets(&book_root()),
    )
}

#[test]
fn epub_plan_elides_chapter_four() {
    // The golden, over real chapters, with no renderer installed.
    let epub = plan_for(Target::Epub);
    let ch4 = &epub.chapters[3].markdown;

    assert!(
        ch4.contains("lines elided"),
        "the epub must say what it left out"
    );
    assert!(
        ch4.contains("full file: https://github.com/abstecker/hello-playbook/blob/"),
        "spec § 3.4 asks for the link"
    );
    assert!(
        !ch4.contains("# mod tests {"),
        "an epub hidden line is a comment nobody can expand"
    );
}

#[test]
fn html_plan_keeps_the_toggle() {
    let html = plan_for(Target::Html);
    let ch4 = &html.chapters[3].markdown;

    assert!(
        ch4.contains("# mod tests {"),
        "html keeps mdBook's hidden lines"
    );
    assert!(
        !ch4.contains("lines elided"),
        "and needs no elision comment"
    );
}

#[test]
fn epub_plan_carries_the_exercise_without_a_toggle() {
    let ch4 = &plan_for(Target::Epub).chapters[3].markdown;
    assert!(ch4.contains("> **Your turn: Make the test pass**"), "{ch4}");
    assert!(!ch4.contains("<details"), "{ch4}");
    assert!(!ch4.contains("step-exercise"), "{ch4}");

    let html = &plan_for(Target::Html).chapters[3].markdown;
    assert!(html.contains("<details class=\"step-answer\">"), "{html}");
}

#[test]
fn both_targets_carry_every_chapter_and_its_metadata() {
    for target in [Target::Html, Target::Epub] {
        let p = plan_for(target);
        assert_eq!(p.target, target);
        assert_eq!(p.meta.title, "Hello, Playbook");
        assert_eq!(
            p.chapters.len(),
            7,
            "a dropped chapter is one nobody misses"
        );
        assert_eq!(p.chapters[0].title, "A repo that builds");
        assert_eq!(p.chapters[6].title, "Appendix: credits");
        assert!(
            p.meta.cover.is_some(),
            "the sample book ships a cover, and every target carries it"
        );
    }
}

#[test]
fn no_directive_line_reaches_either_target() {
    for target in [Target::Html, Target::Epub] {
        for chapter in plan_for(target).chapters {
            for line in chapter.markdown.lines() {
                assert!(
                    !line.trim_start().starts_with("<!-- bower"),
                    "{} leaked a directive: {line}",
                    chapter.path
                );
            }
        }
    }
}

/// End to end through pandoc. Ignored: needs pandoc installed. See the module
/// docs for the command.
#[test]
#[ignore = "needs pandoc installed"]
fn publish_epub_produces_a_readable_book() {
    let out = scratch("epub");
    let result = Command::new(env!("CARGO_BIN_EXE_bower"))
        .arg("--book")
        .arg(book_root())
        .args(["publish", "--target", "epub", "-o"])
        .arg(&out)
        .output()
        .expect("the bower binary must run");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );

    let epub = out.join(artifact(Target::Epub, "epub"));
    assert!(epub.exists(), "no epub was written");

    // An epub is a zip whose first entry is an uncompressed `mimetype`.
    let bytes = std::fs::read(&epub).unwrap();
    assert!(bytes.starts_with(b"PK"), "not a zip");
    assert!(
        String::from_utf8_lossy(&bytes[..200]).contains("application/epub+zip"),
        "the mimetype entry is missing or not first"
    );
    assert!(bytes.len() > 10_000, "suspiciously small: {}", bytes.len());

    // A zip stores its entry names in the clear, so the cover can be found
    // without unpacking. Both must be there: pandoc writes `cover.xhtml` only
    // when it was handed a cover image, and the image itself is what the
    // reader actually sees.
    let names = String::from_utf8_lossy(&bytes);
    assert!(names.contains("cover.xhtml"), "the epub has no cover page");
    assert!(
        names.contains("media/file0.svg"),
        "the composed cover image is not in the epub"
    );
}

/// The cover reaches the PDF, and reaches it as the first page.
///
/// Ignored: needs pandoc and typst. See the module docs for the command.
#[test]
#[ignore = "needs pandoc and typst installed"]
fn pdf_opens_on_the_cover() {
    let out = scratch("pdf-cover");
    let result = Command::new(env!("CARGO_BIN_EXE_bower"))
        .arg("--book")
        .arg(book_root())
        .args(["publish", "--target", "pdf", "-o"])
        .arg(&out)
        .output()
        .expect("the bower binary must run");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );

    let work = out.join(".typst");
    let cover = std::fs::read_to_string(work.join("cover.svg")).expect("no cover was written");
    assert!(
        cover.contains("Hello, Playbook"),
        "the composed cover lost its title band"
    );

    let source = std::fs::read_to_string(work.join("book.typ")).unwrap();
    let page = source.find("#page(margin: 0pt").expect("no cover page");
    let body = source.find("= A repo that builds").expect("no chapter one");
    assert!(page < body, "the cover must precede chapter one");
    assert!(
        source.contains("#counter(page).update(1)"),
        "chapter one must print page 1, not page 2"
    );
}

/// The promise this project keeps everywhere else, kept here too.
///
/// Ignored: needs pandoc and typst. See the module docs for the command.
#[test]
#[ignore = "needs pandoc and typst installed"]
fn pdf_is_byte_identical_across_runs() {
    let a = scratch("pdf-a");
    let b = scratch("pdf-b");

    for out in [&a, &b] {
        let result = Command::new(env!("CARGO_BIN_EXE_bower"))
            .arg("--book")
            .arg(book_root())
            .args(["publish", "--target", "pdf", "-o"])
            .arg(out)
            .output()
            .expect("the bower binary must run");
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    let name = artifact(Target::Pdf, "pdf");
    let one = std::fs::read(a.join(&name)).unwrap();
    let two = std::fs::read(b.join(&name)).unwrap();
    assert!(one.starts_with(b"%PDF"), "not a pdf");
    assert_eq!(
        one, two,
        "two runs of an unchanged book must produce identical bytes"
    );
}

#[test]
fn pdf_plan_matches_the_epub_plan() {
    // Fast, no renderer: pdf and epub share an elision rule, so their markdown
    // must be identical. Two targets growing two engines would show here first.
    let epub = plan_for(Target::Epub);
    let pdf = plan_for(Target::Pdf);
    for (e, p) in epub.chapters.iter().zip(pdf.chapters.iter()) {
        assert_eq!(e.markdown, p.markdown, "{} differs", e.path);
    }
}
