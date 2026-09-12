//! `normalize` against real output: every texture reduces to the lines a
//! reader of the book should see, and nothing that varies by machine or run.

#![allow(non_snake_case, clippy::unwrap_used, clippy::expect_used)]

use bower_testkit::prelude::*;
use bower_testkit::textures::{self, scrub};

fn v(lines: &[&str]) -> Vec<String> {
    lines.iter().map(ToString::to_string).collect()
}

#[test]
fn texture__e0004_keeps_the_spans_and_notes() {
    assert_eq!(
        normalize(textures::E0004, &scrub()),
        v(&[
            "error[E0004]: non-exhaustive patterns: `'\\0'..='@'`, `'B'..='J'`, `'L'..='`'` and 3 more not covered",
            "  --> src/lib.rs:8:15",
            "   |",
            " 8 |         match c {",
            "   |               ^ patterns `'\\0'..='@'`, `'B'..='J'`, `'L'..='`'` and 3 more not covered",
            "   |",
            "   = note: the matched value is of type `char`",
            "help: ensure that all possible cases are being handled by adding a match arm with a wildcard pattern as shown, or multiple match arms",
            "   |",
            "10 ~             'K' | 'k' => Rank::KING,",
            "11 ~             _ => todo!(),",
            "   |",
            "",
            "For more information about this error, try `rustc --explain E0004`.",
            "error: could not compile `rank` (lib) due to 1 previous error",
        ])
    );
}

#[test]
fn texture__colour_and_crlf_change_nothing() {
    let plain = normalize(textures::E0004, &scrub());
    assert_eq!(normalize(textures::E0004_ANSI, &scrub()), plain);
    assert_eq!(normalize(&textures::e0004_crlf(), &scrub()), plain);
}

#[test]
fn texture__the_test_panic_loses_its_thread_id_clock_and_paths() {
    let lines = normalize(textures::TEST_PANIC, &scrub());
    assert_eq!(lines[0], "running 2 tests");
    assert!(
        lines.contains(
            &"thread 'tests::greet_ignores_stray_whitespace' panicked at src/lib.rs:20:9:"
                .to_string()
        ),
        "{lines:#?}"
    );
    assert!(
        lines.contains(
            &"test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out"
                .to_string()
        ),
        "{lines:#?}"
    );
    assert_eq!(
        lines.last().unwrap(),
        "error: test failed, to rerun pass `--lib`"
    );
    assert!(
        !lines.iter().any(|l| l.contains(textures::SCRATCH)),
        "{lines:#?}"
    );
    assert!(
        !lines
            .iter()
            .any(|l| l.contains("Compiling") || l.contains("Running")),
        "{lines:#?}"
    );
}

#[test]
fn texture__a_warning_only_check_keeps_the_warning() {
    let lines = normalize(textures::WARNING_ONLY, &scrub());
    assert_eq!(
        lines[0],
        "warning: method `hello__world` should have a snake case name"
    );
    assert_eq!(
        lines.last().unwrap(),
        "warning: `rust4failures` (lib) generated 1 warning"
    );
}

#[test]
fn texture__the_manifest_error_names_a_relative_path() {
    assert_eq!(
        normalize(textures::MANIFEST_ERROR, &scrub())[0],
        "error: failed to parse manifest at `Cargo.toml`"
    );
}

#[test]
fn texture__two_runs_of_one_suite_normalize_alike() {
    assert_eq!(
        normalize(textures::TIMED_PASS_SHUFFLED, &scrub()),
        normalize(textures::TIMED_PASS, &scrub())
    );
}

#[test]
fn texture__error_codes_come_from_the_headline() {
    assert_eq!(
        error_codes(&normalize(textures::E0004, &scrub())),
        vec!["E0004"]
    );
    assert_eq!(
        error_codes(&normalize(textures::E0308, &scrub())),
        vec!["E0308"]
    );
    assert!(error_codes(&normalize(textures::TEST_PANIC, &scrub())).is_empty());
}
