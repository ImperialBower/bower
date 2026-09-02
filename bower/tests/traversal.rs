//! A book is data, and `file="…"` is book-controlled. These are the escapes
//! that worked before the guard existed, kept as regression tests.
//!
//! Found in review, reproduced, then fixed. `bower verify` was the exploitable
//! path: it writes each step's tree to disk with no git anywhere in the loop.
//! `bower build` was protected only incidentally, by `gix` refusing `..` as a
//! tree filename — a protection that would have vanished with a change of git
//! backend, and never covered absolute paths at all.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn scratch(case: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bower-traversal-{case}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// A one-chapter book whose single directive writes to `target`.
fn evil_book(dir: &Path, target: &str) {
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("bower.toml"),
        concat!(
            "[book]\nepoch = 2026-09-01T00:00:00Z\n\n",
            "[identity]\nname = \"T\"\nemail = \"t@example.invalid\"\n\n",
            "[repos.evil]\ncheck = \"true\"\nverify = \"true\"\n",
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("src").join("SUMMARY.md"),
        "# Summary\n\n- [One](ch01.md)\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("src").join("ch01.md"),
        format!(
            "# Escape\n\n<!-- bower repo=\"evil\" step=\"escape\" file=\"{target}\" -->\n\n\
             ```text\nshould never be written\n```\n"
        ),
    )
    .unwrap();
}

fn bower(book: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bower"))
        .arg("--book")
        .arg(book)
        .args(args)
        .output()
        .expect("the bower binary must run")
}

#[test]
fn verify_refuses_a_relative_escape() {
    let root = scratch("verify-rel");
    let book = root.join("book");
    evil_book(&book, "../../ESCAPED.txt");

    let out = bower(
        &book,
        &["verify", "--work", root.join("work").to_str().unwrap()],
    );
    assert!(!out.status.success(), "the escape must fail the command");
    assert!(
        !root.join("ESCAPED.txt").exists(),
        "a file was written outside the work directory"
    );
}

#[test]
fn verify_refuses_an_absolute_escape() {
    // The worse variant: `Path::join` discards its base when given an absolute
    // path, so this escapes with no `..` to notice.
    let root = scratch("verify-abs");
    let book = root.join("book");
    let target = root.join("ABSOLUTE.txt");
    evil_book(&book, target.to_str().unwrap());

    let out = bower(
        &book,
        &["verify", "--work", root.join("work").to_str().unwrap()],
    );
    assert!(!out.status.success(), "the escape must fail the command");
    assert!(!target.exists(), "a file was written to an absolute path");
}

#[test]
fn build_refuses_an_absolute_escape() {
    // `gix` never protected against this one.
    let root = scratch("build-abs");
    let book = root.join("book");
    let target = root.join("ABSOLUTE.txt");
    evil_book(&book, target.to_str().unwrap());

    let out = bower(&book, &["build", "-o", root.join("out").to_str().unwrap()]);
    assert!(!out.status.success(), "the escape must fail the command");
    assert!(!target.exists(), "a file was written to an absolute path");
}

#[test]
fn build_refuses_a_relative_escape_by_our_own_rule() {
    let root = scratch("build-rel");
    let book = root.join("book");
    evil_book(&book, "../../ESCAPED.txt");

    let out = bower(&book, &["build", "-o", root.join("out").to_str().unwrap()]);
    assert!(!out.status.success());
    assert!(!root.join("ESCAPED.txt").exists());
    // Ours, not `gix`'s incidental filename validation — so the guarantee
    // survives swapping the git backend.
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unsafe path in book"),
        "the refusal should be ours: {stderr}"
    );
}

#[test]
fn plan_refuses_a_summary_link_that_climbs_out() {
    let root = scratch("plan-summary");
    let book = root.join("book");
    evil_book(&book, "src/lib.rs");
    std::fs::write(
        book.join("src").join("SUMMARY.md"),
        "# Summary\n\n- [Out](../../../../../../etc/hosts.md)\n",
    )
    .unwrap();

    let out = bower(&book, &["plan"]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("links outside the book"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
