//! The sample book, `books/hello-playbook/`, planned and checked.
//!
//! The chapters are real files on disk, pulled in by `fixtures::hello_playbook()`
//! at compile time. If a chapter's directives drift, this test fails.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use bower_testkit::prelude::*;

const REPO: &str = "hello-playbook";

use bower_testkit::fixtures::{HELLO_PLAYBOOK_FINAL_PATHS, HELLO_PLAYBOOK_TAGS};
fn book_plan() -> BookPlan {
    let f = fixtures::hello_playbook();
    match plan(&f.book, &f.catalog) {
        Ok(p) => p,
        Err(errs) => panic!("the sample book must plan cleanly:\n{errs}"),
    }
}

fn tags(p: &BookPlan) -> Vec<String> {
    p.repo(REPO)
        .expect("the sample book targets `hello-playbook`")
        .steps
        .iter()
        .map(PlannedStep::tag)
        .collect()
}

#[test]
fn plans_cleanly_with_the_expected_tags() {
    let p = book_plan();
    assert_eq!(tags(&p), HELLO_PLAYBOOK_TAGS);
}

#[test]
fn records_the_two_deliberate_failures() {
    let p = book_plan();
    let repo = p.repo(REPO).expect("repo present");
    let expect_of = |id: &str| {
        repo.steps
            .iter()
            .find(|s| s.id.0 == id)
            .unwrap_or_else(|| panic!("no step `{id}`"))
            .expect
    };

    assert_eq!(expect_of("test-that-fails"), Expect::TestFail);
    assert_eq!(expect_of("wont-compile"), Expect::CompileFail);
    assert_eq!(expect_of("test-that-passes"), Expect::Pass);
    assert_eq!(expect_of("scratch-fixed"), Expect::Pass);
}


fn final_tree() -> TreeState {
    let p = book_plan();
    let repo = p.repo(REPO).expect("repo present");
    repo.steps.last().expect("the book has steps").tree.clone()
}

#[test]
fn final_tree_holds_exactly_the_expected_files() {
    let tree = final_tree();
    let paths: Vec<&str> = tree.paths().collect();
    assert_eq!(paths, HELLO_PLAYBOOK_FINAL_PATHS);
}

#[test]
fn no_region_marker_survives_into_any_tree() {
    let p = book_plan();
    for step in &p.repo(REPO).expect("repo present").steps {
        for path in step.tree.paths() {
            let Some(text) = step.tree.text(path) else {
                continue;
            };
            assert!(
                !text.contains("bower:"),
                "{} still carries a marker at {}",
                path,
                step.tag()
            );
        }
    }
}

#[test]
fn cargo_toml_and_lib_rs_match_their_goldens() {
    let tree = final_tree();

    assert_eq!(
        tree.text("Cargo.toml").expect("Cargo.toml exists"),
        concat!(
            "[package]\n",
            "name = \"hello-playbook\"\n",
            "version = \"0.1.0\"\n",
            "edition = \"2021\"\n",
            "rust-version = \"1.95\"\n",
            "license = \"MIT OR Apache-2.0 OR GPL-3.0-or-later\"\n",
            "\n",
            "[dependencies]\n",
            "\n",
            "[lints.rust]\n",
            "unsafe_code = \"forbid\"\n",
            "\n",
            "[lints.clippy]\n",
            "pedantic = { level = \"warn\", priority = -1 }\n",
            "unwrap_used = \"warn\"\n",
            "expect_used = \"warn\"\n",
        )
    );

    let lib = tree.text("src/lib.rs").expect("src/lib.rs exists");
    assert!(lib.starts_with("//! A greeting, and nothing else.\n\n/// Build a greeting"));
    assert!(lib.contains("format!(\"Hello, {}!\", name.trim())"));
    assert!(lib.contains("fn greet_ignores_stray_whitespace()"));
    assert!(!lib.contains("pub mod scratch;"));
    assert!(
        !lib.contains("\n\n\n"),
        "two blank lines in a row — `cargo fmt --check` would reject this:\n{lib}"
    );
}

#[test]
fn makefile_ends_with_the_full_gate() {
    let tree = final_tree();
    let mk = tree.text("Makefile").expect("Makefile exists");
    assert!(mk.contains("GATE := build fmt lint test audit"));
    assert!(mk.contains("audit:\n\t./bin/security-scan"));
    assert!(mk.contains(".DEFAULT_GOAL := ayce"));
}

#[test]
fn planning_twice_is_byte_identical() {
    assert_eq!(book_plan(), book_plan());
}
