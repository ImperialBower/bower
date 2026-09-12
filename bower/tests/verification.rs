//! `bower verify` against the sample book.
//!
//! The fast cases live in the default gate; the exhaustive twenty-step sweep is
//! `#[ignore]`d, because a compiler run per step is a tax on every `cargo test`.
//! Run the sweep with:
//!
//! ```text
//! cargo test -p bower --test verification -- --ignored
//! ```

#![allow(non_snake_case, clippy::unwrap_used, clippy::expect_used)]

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
        // Both deliberate failures quote what the compiler said, and both
        // quotes still hold.
        assert!(stdout.contains("       ok   output="), "{stdout}");
        assert!(
            !stdout.contains("drft") && !stdout.contains("todo"),
            "{stdout}"
        );
        assert!(stdout.contains("every claim holds"), "{stdout}");
    }
}

fn chapter_of(book: &Path) -> PathBuf {
    book.join("src")
        .join("ch04-tests-and-failing-on-purpose.md")
}

/// The 1-based line of the first directive in `text` containing `needle`.
fn directive_line(text: &str, needle: &str) -> usize {
    text.lines()
        .position(|l| l.contains("<!-- bower") && l.contains(needle))
        .map(|i| i + 1)
        .expect("the directive is in the chapter")
}

/// The test that proves the output check can say no (EPIC-11 exit
/// criterion 2), as `reports_a_wrong_expectation_as_broken` does for `expect`.
#[test]
fn reports_a_drifted_diagnostic_as_broken() {
    let book = scratch("drift-book");
    copy_dir(&book_root(), &book);
    let chapter = chapter_of(&book);
    let text = std::fs::read_to_string(&chapter).unwrap();
    let edited = text.replacen(
        "error[E0308]: mismatched types",
        "error[E0308]: mismatched typos",
        1,
    );
    assert_ne!(edited, text, "the fixture must actually change");
    std::fs::write(&chapter, edited).unwrap();

    let out = verify(&book, &scratch("drift-work"), &["--step", "wont-compile"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        !out.status.success(),
        "a drifted output must fail:\n{stdout}"
    );
    assert!(stdout.contains("drft output=check"), "{stdout}");
    assert!(
        stderr.contains("ch04-tests-and-failing-on-purpose.md"),
        "the report must name the chapter:\n{stderr}"
    );
    assert!(stderr.contains("at line 1 of the recording"), "{stderr}");
    assert!(
        stderr.contains("mismatched typos") && stderr.contains("mismatched types"),
        "the report must show both sides:\n{stderr}"
    );
    assert!(
        stderr.contains("bower verify --record --step wont-compile"),
        "{stderr}"
    );
}

#[test]
fn record__rewrites_the_chapter_and_the_lock() {
    let book = scratch("record-book");
    copy_dir(&book_root(), &book);
    let chapter = chapter_of(&book);
    let original = std::fs::read_to_string(&chapter).unwrap();
    let line = directive_line(&original, "output=\"check\"");
    std::fs::write(&chapter, bower_core::prelude::rewrite(&original, line, &[])).unwrap();
    std::fs::write(book.join("bower.lock"), "stale\n").unwrap();

    let out = verify(
        &book,
        &scratch("record-work"),
        &["--step", "wont-compile", "--record"],
    );
    assert!(
        out.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // Recording on this machine restores exactly what the book holds, which
    // was recorded on another run (exit criterion 4, on one machine).
    assert_eq!(std::fs::read_to_string(&chapter).unwrap(), original);
    assert_eq!(
        std::fs::read_to_string(book.join("bower.lock")).unwrap(),
        std::fs::read_to_string(book_root().join("bower.lock")).unwrap(),
        "the lock must be rewritten, and consistent with the chapter"
    );
}

#[test]
fn record__is_a_no_op_when_nothing_changed() {
    let book = scratch("noop-book");
    copy_dir(&book_root(), &book);
    let chapter = chapter_of(&book);
    let before = std::fs::read_to_string(&chapter).unwrap();
    std::fs::write(book.join("bower.lock"), "untouched\n").unwrap();

    let out = verify(
        &book,
        &scratch("noop-work"),
        &["--step", "wont-compile", "--record"],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("nothing to record"), "{stdout}");
    assert_eq!(std::fs::read_to_string(&chapter).unwrap(), before);
    assert_eq!(
        std::fs::read_to_string(book.join("bower.lock")).unwrap(),
        "untouched\n",
        "nothing recorded, so the lock is not touched"
    );
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

/// The bug this project shipped for a while: `bower verify` wrote its scratch
/// package under `target/`, cargo refused to build anything inside its own
/// workspace, and the report said all twenty of the book's true claims were
/// false.
///
/// Not `#[ignore]`d, and deliberately so. Cargo rejects the package before
/// compiling a line, so this costs milliseconds — and the default gate is the
/// one place this failure had to be caught and was not.
#[test]
fn a_scratch_tree_inside_a_workspace_is_the_verifiers_fault_not_the_books() {
    // Inside this repository's own cargo workspace, which is exactly where the
    // old default `target/bower-verify` put it.
    let work = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("target")
        .join("bower-verify-nested");
    let _ = std::fs::remove_dir_all(&work);

    let out = verify(&book_root(), &work, &["--step", "cargo-init"]);
    let text = String::from_utf8_lossy(&out.stderr);

    assert!(!out.status.success(), "this cannot be reported as a pass");
    assert!(
        text.contains("inside another cargo workspace"),
        "the verifier must name its own problem: {text}"
    );
    assert!(text.contains("--work"), "and say how to fix it: {text}");
    assert!(
        !text.contains("claim"),
        "it must not blame the book: {text}"
    );
}

/// The default `--work` is usable — which is the half a unit test cannot prove.
///
/// `#[ignore]`d with the sweep: it runs a real compiler.
#[test]
#[ignore = "runs a compiler"]
fn the_default_work_dir_verifies_a_step() {
    let out = Command::new(env!("CARGO_BIN_EXE_bower"))
        .arg("--book")
        .arg(book_root())
        .args(["verify", "--step", "cargo-init"])
        .output()
        .expect("the bower binary must run");

    assert!(
        out.status.success(),
        "verify must work with no --work at all: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("cargo-init"));
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
    // Step rows start two spaces in; output rows start seven.
    let steps_ok = stdout.lines().filter(|l| l.starts_with("  ok ")).count();
    assert_eq!(steps_ok, 20, "{stdout}");
    assert_eq!(stdout.matches("       ok   output=").count(), 2, "{stdout}");
    assert!(stdout.contains("every claim holds"), "{stdout}");
}
