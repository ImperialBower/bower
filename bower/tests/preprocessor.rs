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

    assert!(
        !out.status.success(),
        "a book that does not resolve must fail"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("does not resolve"), "{stderr}");
    assert!(stderr.contains("no-such-repo"), "{stderr}");
    assert!(
        stderr.contains("src/ch01.md"),
        "the error must name the chapter: {stderr}"
    );
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

/// The real six chapters, wrapped in an mdBook envelope.
fn real_book_envelope() -> String {
    let src = book_root().join("src");
    let order = [
        "ch01-a-repo-that-builds.md",
        "ch02-the-gate.md",
        "ch03-lints-and-format.md",
        "ch04-tests-and-failing-on-purpose.md",
        "ch05-supply-chain.md",
        "ch06-ci.md",
    ];
    let sections: Vec<serde_json::Value> = order
        .iter()
        .map(|name| {
            let content = std::fs::read_to_string(src.join(name)).unwrap();
            serde_json::json!({
                "Chapter": {
                    "name": name,
                    "content": content,
                    "path": name,
                    "sub_items": [],
                }
            })
        })
        .collect();
    serde_json::json!([
        { "root": book_root() },
        { "sections": sections }
    ])
    .to_string()
}

fn rendered_chapters() -> Vec<String> {
    let out = run(&[], &real_book_envelope());
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let book: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    book["sections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["Chapter"]["content"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn sample_book_keeps_no_directive_lines() {
    // Prose that *quotes* a directive is content and must survive; a directive
    // line is an instruction and must not.
    for (i, chapter) in rendered_chapters().iter().enumerate() {
        for (n, line) in chapter.lines().enumerate() {
            assert!(
                !line.trim_start().starts_with("<!-- bower"),
                "chapter {i}, line {}: {line}",
                n + 1
            );
        }
    }
    // The counterweight: chapter 1 discusses directives in prose.
    assert!(
        rendered_chapters()[0].contains("`<!-- bower"),
        "prose about directives must survive"
    );
}

#[test]
fn sample_book_anchors_every_step() {
    let joined = rendered_chapters().join("\n");
    for tag in bower_testkit::fixtures::HELLO_PLAYBOOK_TAGS {
        // `step-011-test-that-fails` → the anchor is the step id alone.
        let id = tag.splitn(3, '-').nth(2).unwrap();
        assert!(
            joined.contains(&format!("<a id=\"step-{id}\"></a>")),
            "no anchor for {id}"
        );
    }
}

#[test]
fn sample_book_applies_display_markers() {
    let ch04 = &rendered_chapters()[3];
    // The marked test prints; the module around it goes behind the toggle.
    assert!(
        ch04.contains("    fn greet_ignores_stray_whitespace() {"),
        "the marked test must be visible"
    );
    assert!(
        ch04.contains("# mod tests {"),
        "the unmarked module must be hidden"
    );
    // A *marker line* must not survive. Prose that names the markers must —
    // chapter 4 explains them a paragraph later, and deleting that sentence
    // would erase the lesson.
    for line in ch04.lines() {
        assert!(
            bower_core::prelude::show_marker(line).is_none(),
            "a marker line survived: {line}"
        );
    }
    assert!(
        ch04.contains("The `bower:show` markers decide"),
        "prose about the markers must survive"
    );

    // Chapter 6 is YAML, where `# ` hides nothing — it elides with a count.
    let ch06 = &rendered_chapters()[5];
    assert!(ch06.contains("lines elided"), "{ch06}");
}

#[test]
fn sample_book_footers_link_to_the_right_lines() {
    let ch04 = &rendered_chapters()[3];
    assert!(
        ch04.contains("blob/step-011-test-that-fails/src/lib.rs#L18-L21"),
        "the line anchor is the whole point"
    );
    assert!(ch04.contains("step 011 of hello-playbook"), "{ch04}");
}

/// End to end through mdBook itself. Ignored: it needs `mdbook` installed and
/// `mdbook-bower` on PATH. Run with:
///
/// ```text
/// cargo build -p bower
/// PATH="$PWD/target/debug:$PATH" cargo test -p bower --test preprocessor -- --ignored
/// ```
#[test]
#[ignore = "needs mdbook installed and mdbook-bower on PATH"]
fn mdbook_build_succeeds_with_the_preprocessor() {
    let out = Command::new("mdbook")
        .arg("build")
        .arg(book_root())
        .output()
        .expect("mdbook must be installed to run this test");
    assert!(
        out.status.success(),
        "mdbook build failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let html = book_root()
        .join("book")
        .join("ch04-tests-and-failing-on-purpose.html");
    let text = std::fs::read_to_string(&html).expect("chapter 4 must render");
    assert!(text.contains("greet_ignores_stray_whitespace"), "{html:?}");
    assert!(!text.contains("<!-- bower"), "a directive reached the page");
}
