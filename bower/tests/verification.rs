//! `bower verify` against the sample book.
//!
//! The fast cases live in the default gate; the exhaustive twenty-step sweep is
//! `#[ignore]`d, because a compiler run per step is a tax on every `cargo test`.
//! Run the sweep with:
//!
//! ```text
//! cargo test -p bower --test verification -- --ignored
//! ```

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
    let dir = std::env::temp_dir().join(format!("bower-verify-{case}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// Run `bower verify` against `book`, returning the raw output so a test can
/// assert on the exit status as well as the text.
fn verify(book: &Path, work: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bower"))
        .arg("--book")
        .arg(book)
        .arg("verify")
        .arg("--work")
        .arg(work)
        .args(extra)
        .output()
        .expect("the bower binary must run")
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap();
        // mdBook's render output is not part of the book.
        if name == "book" {
            continue;
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
fn upholds_the_two_deliberate_failures() {
    let work = scratch("deliberate");
    for step in ["test-that-fails", "wont-compile"] {
        let out = verify(&book_root(), &work, &["--step", step]);
        assert!(
            out.status.success(),
            "step `{step}` should have been upheld:\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("ok  "), "{stdout}");
        assert!(stdout.contains("every claim holds"), "{stdout}");
    }
}

#[test]
fn reports_a_wrong_expectation_as_broken() {
    // Without this test, a verifier hardcoded to return `Upheld` would pass
    // every other test in this file.
    let liar = scratch("liar-book");
    copy_dir(&book_root(), &liar);

    let chapter = liar
        .join("src")
        .join("ch04-tests-and-failing-on-purpose.md");
    let text = std::fs::read_to_string(&chapter).unwrap();
    let lie = text.replace(r#"expect="test_fail""#, r#"expect="pass""#);
    assert_ne!(lie, text, "the fixture must actually change");
    std::fs::write(&chapter, lie).unwrap();

    let out = verify(&liar, &scratch("liar-work"), &["--step", "test-that-fails"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        !out.status.success(),
        "a false claim must fail the build:\n{stdout}"
    );
    assert!(stdout.contains("FAIL"), "{stdout}");
    assert!(
        stderr.contains("ch04-tests-and-failing-on-purpose.md"),
        "the report must name the chapter:\n{stderr}"
    );
    assert!(
        stderr.contains("its tests failed"),
        "the report must say what happened:\n{stderr}"
    );
    assert!(
        stderr.contains("cargo test"),
        "the report must name the command:\n{stderr}"
    );
}

#[test]
fn unknown_step_is_an_error() {
    let out = verify(
        &book_root(),
        &scratch("unknown"),
        &["--step", "no-such-step"],
    );
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("no step named `no-such-step`"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The full sweep. Ignored by default; see this file's header for the command.
#[test]
#[ignore = "runs a compiler per step; see the module docs for the command"]
fn upholds_every_step() {
    let out = verify(&book_root(), &scratch("full"), &[]);
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        out.status.success(),
        "some claim in the book is not true:\n{stdout}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("20 step(s)"), "{stdout}");
    assert_eq!(stdout.matches("ok  ").count(), 20, "{stdout}");
    assert!(stdout.contains("every claim holds"), "{stdout}");
}
