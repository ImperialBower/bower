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
use bower::publish::{render_plan, BookMeta, RenderPlan, Target};
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
    render_plan(&book, &resolved, meta, target, &links)
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
        ch4.contains("full file: https://github.com/ImperialBower/hello-playbook/blob/"),
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
fn both_targets_carry_every_chapter_and_its_metadata() {
    for target in [Target::Html, Target::Epub] {
        let p = plan_for(target);
        assert_eq!(p.target, target);
        assert_eq!(p.meta.title, "Hello, Playbook");
        assert_eq!(
            p.chapters.len(),
            6,
            "a dropped chapter is one nobody misses"
        );
        assert_eq!(p.chapters[0].title, "A repo that builds");
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

    let epub = out.join("hello-playbook.epub");
    assert!(epub.exists(), "no epub was written");

    // An epub is a zip whose first entry is an uncompressed `mimetype`.
    let bytes = std::fs::read(&epub).unwrap();
    assert!(bytes.starts_with(b"PK"), "not a zip");
    assert!(
        String::from_utf8_lossy(&bytes[..200]).contains("application/epub+zip"),
        "the mimetype entry is missing or not first"
    );
    assert!(bytes.len() > 10_000, "suspiciously small: {}", bytes.len());
}
