//! The headline requirement (spec § 5.1): an unchanged book replays to
//! byte-identical SHAs, every time.
//!
//! These tests drive the **installed binary**, not the crate's internals. That
//! is deliberate: the artifact a reader runs is the artifact under test, and it
//! keeps the CLI's own argument handling inside the guarantee.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use bower_testkit::fixtures::{HELLO_PLAYBOOK_FINAL_PATHS, HELLO_PLAYBOOK_TAGS};

fn book_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("books")
        .join("hello-playbook")
}

/// A scratch directory unique to this case, removed on entry so every run
/// starts from nothing.
fn scratch(case: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bower-determinism-{case}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// Run `bower build` and return the HEAD id it reports.
fn build(out: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_bower"))
        .arg("--book")
        .arg(book_root())
        .arg("build")
        .arg("-o")
        .arg(out)
        .output()
        .expect("the bower binary must run");

    assert!(
        output.status.success(),
        "bower build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let head = stdout
        .split("HEAD ")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .expect("build must report a HEAD id");
    assert_eq!(head.len(), 40, "a HEAD id is 40 hex characters: {head}");
    head.to_string()
}

/// Loose refs under `.git/refs/tags`, read from disk rather than through a git
/// library, so the test cannot share a bug with the code it checks.
fn tags(out: &Path) -> BTreeSet<String> {
    let dir = out.join(".git").join("refs").join("tags");
    std::fs::read_dir(&dir)
        .expect("a replayed repo has tags")
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect()
}

/// Every file in the working tree, `.git` excluded, repo-relative.
fn worktree_files(out: &Path) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut stack = vec![out.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.file_name().is_some_and(|n| n == ".git") {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else {
                found.insert(
                    path.strip_prefix(out)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    found
}

#[test]
fn hello_playbook_is_byte_identical_across_runs() {
    let a = scratch("run-a");
    let b = scratch("run-b");

    let head_a = build(&a);
    let head_b = build(&b);

    assert_eq!(head_a, head_b, "two replays of one book must agree");
}

#[test]
fn starts_from_empty_even_over_a_dirty_directory() {
    let dir = scratch("dirty");
    let clean_head = build(&dir);

    // Junk a merging implementation would carry forward.
    std::fs::write(dir.join("STOWAWAY.md"), "should not survive").unwrap();
    std::fs::write(dir.join("Cargo.toml"), "garbage").unwrap();

    let second_head = build(&dir);

    assert_eq!(
        second_head, clean_head,
        "replay must not inherit the old tree"
    );
    assert!(
        !dir.join("STOWAWAY.md").exists(),
        "a replay that leaves strays behind is not a replay from empty"
    );
}

#[test]
fn tags_match_the_planned_tags() {
    let dir = scratch("tags");
    build(&dir);
    let found = tags(&dir);

    for tag in HELLO_PLAYBOOK_TAGS {
        assert!(found.contains(*tag), "missing tag {tag}");
    }
    // Twenty step tags plus one `<chapter>-end` tag per chapter.
    assert_eq!(
        found.len(),
        HELLO_PLAYBOOK_TAGS.len() + 6,
        "unexpected tag set: {found:?}"
    );
}

#[test]
fn final_worktree_is_the_kernels_tree_plus_scaffolding_and_steps_md() {
    let dir = scratch("worktree");
    build(&dir);

    let mut expected: BTreeSet<String> = HELLO_PLAYBOOK_FINAL_PATHS
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    // Two things the repository has that the kernel's tree does not: the
    // generated index back into the book, and the scaffolding the template
    // contributes at step 0 and every step thereafter.
    expected.insert("STEPS.md".to_string());
    for scaffold in [
        ".gitignore",
        "CODE_OF_CONDUCT.md",
        "CONTRIBUTING.md",
        "LICENSE-APACHE",
        "LICENSE-GPLv3",
        "LICENSE-MIT",
        "README.md",
        "SECURITY.md",
    ] {
        expected.insert(scaffold.to_string());
    }

    assert_eq!(worktree_files(&dir), expected);
}

#[test]
fn steps_md_indexes_every_step_back_into_the_book() {
    let dir = scratch("steps-md");
    build(&dir);

    let steps_md = std::fs::read_to_string(dir.join("STEPS.md")).unwrap();
    for tag in HELLO_PLAYBOOK_TAGS {
        assert!(steps_md.contains(tag), "STEPS.md omits {tag}");
    }
    assert!(steps_md.contains("#step-test-that-fails"));
    assert!(steps_md.contains("test_fail"));
}

#[test]
fn the_scaffolding_survives_every_step() {
    // The template is applied as step 0 and must persist. Each step's tree
    // *replaces* the tree, so a step built from the kernel's paths alone
    // silently deletes the licences, the README, and the .gitignore that the
    // scaffolding commit had just added.
    let dir = scratch("scaffolding");
    build(&dir);

    let files = worktree_files(&dir);
    for expected in [
        "LICENSE-MIT",
        "LICENSE-APACHE",
        "LICENSE-GPLv3",
        "README.md",
        ".gitignore",
        "CODE_OF_CONDUCT.md",
        "CONTRIBUTING.md",
        "SECURITY.md",
    ] {
        assert!(
            files.contains(expected),
            "the scaffolding lost `{expected}` — a generated repo with no licence"
        );
    }
}
