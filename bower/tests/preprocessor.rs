//! `mdbook-bower` — the preprocessor, driven the way mdBook drives it.
//!
//! These tests speak the real protocol over stdin and stdout rather than
//! calling into the crate, because the protocol is most of what can go wrong:
//! a preprocessor that mangles the JSON envelope fails in ways that look like
//! mdBook bugs rather than Bower ones.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// The sample book's root — where `bower.toml` declares the repo catalog the
/// preprocessor resolves against.
fn book_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("books")
        .join("hello-playbook")
}

/// A minimal mdBook `[context, book]` envelope with one chapter.
fn envelope(chapter_content: &str) -> String {
    serde_json::json!([
        {
            "root": book_root(),
            "config": {},
            "renderer": "html",
            "mdbook_version": "0.4.0"
        },
        {
            "sections": [
                {
                    "Chapter": {
                        "name": "One",
                        "content": chapter_content,
                        "number": [1],
                        "sub_items": [],
                        "path": "ch01.md",
                        "source_path": "ch01.md",
                        "parent_names": []
                    }
                }
            ],
            "__non_exhaustive": null
        }
    ])
    .to_string()
}

fn run(args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mdbook-bower"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the mdbook-bower binary must run");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn supports_html_and_nothing_else() {
    assert!(run(&["supports", "html"], "").status.success());
    assert!(!run(&["supports", "epub"], "").status.success());
    // A renderer mdBook has not invented yet must be declined, not assumed.
    assert!(!run(&["supports", ""], "").status.success());
}

#[test]
fn passthrough_returns_the_book_unchanged() {
    let input = envelope("# One\n\nSome prose.\n");
    let out = run(&[], &input);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let sent: serde_json::Value = serde_json::from_str(&input).unwrap();
    let got: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();

    // mdBook expects the book back — the second half of the envelope, alone.
    assert_eq!(got, sent[1]);
}

#[test]
fn bad_stdin_is_an_error_not_a_panic() {
    let out = run(&[], "not json at all");
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("not valid JSON"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn a_bare_object_is_rejected() {
    // mdBook always sends a pair. Anything else means the protocol changed
    // under us, and guessing would corrupt someone's book.
    let out = run(&[], r#"{"sections": []}"#);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("[context, book]"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn a_bad_directive_fails_the_build() {
    // Annotation rot must stop `mdbook build`. Without this, every other
    // guarantee the preprocessor makes is advisory.
    let bad = concat!(
        "# One\n\n",
        "<!-- bower repo=\"no-such-repo\" file=\"src/lib.rs\" -->\n",
        "```rust\npub fn x() {}\n```\n",
    );
    let out = run(&[], &envelope(bad));

    assert!(!out.status.success(), "a book that does not resolve must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("does not resolve"), "{stderr}");
    assert!(stderr.contains("no-such-repo"), "{stderr}");
    assert!(stderr.contains("src/ch01.md"), "the error must name the chapter: {stderr}");
}

#[test]
fn a_resolvable_book_survives_planning() {
    // The sample book's own catalog, and a directive that uses it.
    let good = concat!(
        "# One\n\n",
        "<!-- bower repo=\"hello-playbook\" file=\"src/lib.rs\" -->\n",
        "```rust\npub fn x() {}\n```\n",
    );
    let out = run(&[], &envelope(good));
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
