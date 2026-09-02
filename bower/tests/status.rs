//! `bower status` against the sample book.
//!
//! These drive the installed binary and check the exit code as well as the
//! text, because the exit code is what makes the command usable in CI and it is
//! the easiest thing to get quietly wrong.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn book_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("books")
        .join("hello-playbook")
}

fn scratch(case: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bower-statustest-{case}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn bower(book: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bower"))
        .arg("--book")
        .arg(book)
        .args(args)
        .output()
        .expect("the bower binary must run")
}

fn status(book: &Path, out: &Path) -> (bool, String) {
    let o = bower(book, &["status", "-o", out.to_str().unwrap()]);
    (
        o.status.success(),
        String::from_utf8_lossy(&o.stdout).into_owned(),
    )
}

fn build(book: &Path, out: &Path) {
    let o = bower(book, &["build", "-o", out.to_str().unwrap()]);
    assert!(
        o.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&o.stderr)
    );
}

/// A writable copy of the sample book, so a test can edit chapters.
fn copy_book(case: &str) -> PathBuf {
    let dir = scratch(case);
    copy_dir(&book_root(), &dir);
    dir
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap();
        if name == "book" {
            continue; // mdBook render output is not part of the book
        }
        let target = to.join(name);
        if path.is_dir() {
            copy_dir(&path, &target);
        } else {
            std::fs::copy(&path, &target).unwrap();
        }
    }
}

#[test]
fn a_freshly_built_book_is_clean() {
    let out = scratch("clean-repo");
    build(&book_root(), &out);

    let (ok, text) = status(&book_root(), &out);
    assert!(ok, "a freshly built book must exit 0:\n{text}");
    assert!(text.contains("lock      in sync"), "{text}");
    assert!(text.contains("repo      in sync"), "{text}");
}

#[test]
fn never_planned_and_never_built_is_not_drift() {
    // The distinction that makes this usable in CI: a fresh checkout that
    // nobody has planned or built is honest, not broken.
    let book = copy_book("virgin");
    std::fs::remove_file(book.join("bower.lock")).unwrap();

    let (ok, text) = status(&book, Path::new("/nonexistent-repo-dir"));
    assert!(ok, "absent artifacts must exit 0:\n{text}");
    assert!(text.contains("never planned"), "{text}");
    assert!(text.contains("never built"), "{text}");
}

#[test]
fn an_edited_chapter_makes_the_lock_stale() {
    let book = copy_book("edited");
    let chapter = book
        .join("src")
        .join("ch04-tests-and-failing-on-purpose.md");
    let text = std::fs::read_to_string(&chapter).unwrap();
    let edited = text.replace(r#"expect="test_fail""#, r#"expect="pass""#);
    assert_ne!(edited, text, "the fixture must actually change");
    std::fs::write(&chapter, edited).unwrap();

    let (ok, out) = status(&book, Path::new("/nonexistent-repo-dir"));
    assert!(!ok, "a stale lock must exit non-zero:\n{out}");
    assert!(out.contains("lock      STALE"), "{out}");
    // Both sides, so the reader can see what changed rather than that it did.
    assert!(
        out.contains("- 011 test-that-fails expect=test_fail"),
        "{out}"
    );
    assert!(out.contains("+ 011 test-that-fails expect=pass"), "{out}");
}

#[test]
fn a_deleted_file_is_reported() {
    let out = scratch("deleted-file");
    build(&book_root(), &out);
    std::fs::remove_file(out.join("Makefile")).unwrap();

    let (ok, text) = status(&book_root(), &out);
    assert!(!ok, "a missing file must exit non-zero:\n{text}");
    assert!(text.contains("repo      STALE"), "{text}");
    assert!(text.contains("missing      Makefile"), "{text}");
}

#[test]
fn a_tampered_file_and_a_dropped_tag_are_both_named() {
    let out = scratch("tampered");
    build(&book_root(), &out);
    std::fs::write(out.join("src").join("lib.rs"), "tampered\n").unwrap();
    std::fs::write(out.join("STOWAWAY.md"), "not from the book\n").unwrap();
    std::fs::remove_file(out.join(".git/refs/tags/step-011-test-that-fails")).unwrap();

    let (ok, text) = status(&book_root(), &out);
    assert!(!ok, "{text}");
    assert!(text.contains("differs      src/lib.rs"), "{text}");
    assert!(text.contains("extra        STOWAWAY.md"), "{text}");
    assert!(
        text.contains("missing tag  step-011-test-that-fails"),
        "{text}"
    );
}

#[test]
fn rebuilding_clears_the_drift() {
    // The report's advice has to work. If `bower build` does not restore a
    // clean status, the command is telling people to do something useless.
    let out = scratch("rebuild");
    build(&book_root(), &out);
    std::fs::remove_file(out.join("Makefile")).unwrap();
    assert!(!status(&book_root(), &out).0);

    build(&book_root(), &out);
    let (ok, text) = status(&book_root(), &out);
    assert!(ok, "rebuilding must clear the drift:\n{text}");
    assert!(text.contains("repo      in sync"), "{text}");
}
