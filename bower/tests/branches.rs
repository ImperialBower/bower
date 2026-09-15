//! EPIC-09 at the edge: a plan with branches, replayed into a real
//! repository. Driven through the library, because the branch saga is a
//! testkit fixture rather than a book on disk. The first three tests are the
//! spike's (`docs/spikes/spike-branches/tests/spike.rs`), ported by name.
//!
//! Refs and parents are read back with `git` itself, so these tests cannot
//! share a bug with the `gix` code that wrote them.

#![allow(non_snake_case, clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use bower::config::BookConfig;
use bower::replay::Replayer;
use bower_core::prelude::{BookSource, Chapter, RepoCatalog, RepoPlan, plan};
use bower_testkit::fixtures;

fn config() -> BookConfig {
    BookConfig::parse(concat!(
        "[book]\nepoch = 2026-09-01T00:00:00Z\n\n",
        "[identity]\nname = \"N\"\nemail = \"e@example.invalid\"\n\n",
        "[repos.failers]\n",
    ))
    .unwrap()
}

/// One book root for every test. Its directory name is the book's name in
/// every `Book-Source` trailer, so two replays that must agree byte for byte
/// have to share it. It holds no template: these repos have no scaffolding.
fn book_root() -> PathBuf {
    let dir = std::env::temp_dir().join("bower-branches-book");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn plan_of(text: &str) -> RepoPlan {
    let book = BookSource::from_chapters(vec![Chapter::new("ch01-saga.md", text)]);
    plan(&book, &RepoCatalog::from_names(&["failers"]))
        .unwrap()
        .repos
        .remove(0)
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

/// Replay `plan` into a fresh directory; return it and every ref → the commit
/// it names (an annotated tag peeled to its commit).
fn replay(plan: &RepoPlan, case: &str) -> (PathBuf, BTreeMap<String, String>) {
    let cfg = config();
    let out = std::env::temp_dir().join(format!("bower-branches-{case}"));
    let _ = std::fs::remove_dir_all(&out);
    Replayer {
        config: &cfg,
        book_root: &book_root(),
        out_dir: &out,
    }
    .run(plan)
    .unwrap();
    let refs = git(
        &out,
        &[
            "for-each-ref",
            "--format=%(refname) %(objectname) %(*objectname)",
        ],
    )
    .lines()
    .map(|l| {
        let mut words = l.split_whitespace();
        let name = words.next().unwrap().to_string();
        let object = words.next().unwrap();
        (name, words.next().unwrap_or(object).to_string())
    })
    .collect();
    (out, refs)
}

#[test]
fn replay__is_byte_identical_across_runs() {
    let p = plan_of(fixtures::BRANCH_SAGA);
    let (_, a) = replay(&p, "run-a");
    let (_, b) = replay(&p, "run-b");
    assert_eq!(a, b);
    assert!(a.contains_key("refs/heads/try/lookup-table"), "{a:?}");
}

#[test]
fn replay__merge_commit_has_two_parents_and_branches_are_refs() {
    let (dir, refs) = replay(&plan_of(fixtures::BRANCH_SAGA), "graph");
    let line = git(
        &dir,
        &["rev-list", "--parents", "-n", "1", "refs/heads/main"],
    );
    let parts: Vec<&str> = line.split_whitespace().collect();
    assert_eq!(parts.len(), 3, "the merge and its two parents: {line}");
    assert_eq!(parts[1], refs["refs/tags/step-003-lib-doc"]);
    assert_eq!(parts[2], refs["refs/tags/step-005-from-char-fixed"]);
    assert_eq!(
        refs["refs/heads/try/lookup-table"],
        refs["refs/tags/step-002-lookup-table"]
    );
    assert_eq!(
        refs["refs/heads/from-char"],
        refs["refs/tags/step-005-from-char-fixed"]
    );
    assert_eq!(
        refs["refs/heads/main"],
        refs["refs/tags/step-006-merge-from-char"]
    );
}

#[test]
fn replay__a_change_on_the_branch_changes_only_what_descends_from_it() {
    let base = plan_of(fixtures::BRANCH_SAGA);
    let edited_text = fixtures::BRANCH_SAGA.replacen(
        "_ => Rank::Blank }",
        "_ => Rank::Blank } // edited on the branch",
        1,
    );
    assert_ne!(edited_text, fixtures::BRANCH_SAGA);
    let edited = plan_of(&edited_text);

    let (_, a) = replay(&base, "blast-a");
    let (_, b) = replay(&edited, "blast-b");
    for tag in [
        "step-001-rank-enum",
        "step-002-lookup-table",
        "step-003-lib-doc",
        "step-004-from-char-broken",
    ] {
        let r = format!("refs/tags/{tag}");
        assert_eq!(a[&r], b[&r], "{tag} does not descend from the edit");
    }
    assert_ne!(
        a["refs/tags/step-005-from-char-fixed"],
        b["refs/tags/step-005-from-char-fixed"]
    );
    assert_ne!(
        a["refs/heads/main"], b["refs/heads/main"],
        "the merge descends from the edit"
    );
    assert_eq!(
        a["refs/heads/try/lookup-table"],
        b["refs/heads/try/lookup-table"]
    );
}

/// The saga, then one more step on a new branch: a book whose last step in
/// plan order is not on main.
fn ends_on_a_branch() -> RepoPlan {
    plan_of(&format!(
        "{}{}",
        fixtures::BRANCH_SAGA,
        "<!-- bower repo=\"failers\" step=\"after\" branch=\"try/after\" file=\"src/after.rs\" -->\n```rust\n// on a branch\n```\n",
    ))
}

#[test]
fn replay__worktree_is_main_when_the_book_ends_on_a_branch() {
    let (dir, _) = replay(&ends_on_a_branch(), "ends-on-branch");
    assert_eq!(
        git(&dir, &["symbolic-ref", "HEAD"]).trim(),
        "refs/heads/main"
    );
    assert!(
        !dir.join("src/after.rs").exists(),
        "the working tree is a branch's, not main's"
    );
    assert!(
        git(&dir, &["show", "refs/heads/main:STEPS.md"]).contains("`step-007-after`"),
        "STEPS.md lists every step, and lives on main"
    );
    assert!(
        git(&dir, &["status", "--porcelain"]).trim().is_empty(),
        "the index matches main's tree"
    );
}

#[test]
fn replay__a_branch_off_the_last_main_step_changes_only_its_own_files() {
    // `STEPS.md` and `PULLS.md` land on the last main commit. A branch that
    // forks from it inherits them, the same as any other file; building its
    // tree from the scaffolding and its own files alone showed both deleted —
    // in its first commit, its `[diff]` link, and any PR opened for it.
    let (dir, _) = replay(&ends_on_a_branch(), "branch-off-last-main");
    let changed = git(
        &dir,
        &[
            "diff",
            "--name-status",
            "step-006-merge-from-char",
            "step-007-after",
        ],
    );
    assert_eq!(changed, "A\tsrc/after.rs\n");
}

#[test]
fn replay__chapter_end_tag_never_points_at_a_branch() {
    let (_, refs) = replay(&ends_on_a_branch(), "chapter-end");
    assert_eq!(
        refs["refs/tags/ch01-saga-end"],
        refs["refs/tags/step-006-merge-from-char"]
    );
}

#[test]
fn replay__trailers_name_the_line_and_the_merge() {
    let (dir, _) = replay(&plan_of(fixtures::BRANCH_SAGA), "trailers");
    let msg = |tag: &str| git(&dir, &["log", "-1", "--format=%B", tag]);
    assert!(
        msg("step-002-lookup-table").contains("Bower-Line: try/lookup-table\n"),
        "{}",
        msg("step-002-lookup-table")
    );
    assert!(
        msg("step-006-merge-from-char").contains("Bower-Merges: from-char\n"),
        "{}",
        msg("step-006-merge-from-char")
    );
    let main = msg("step-003-lib-doc");
    assert!(
        !main.contains("Bower-Line") && !main.contains("Bower-Merges"),
        "{main}"
    );
}

#[test]
fn replay__pulls_md_lists_every_declared_pr() {
    let (dir, _) = replay(&plan_of(fixtures::BRANCH_SAGA), "pulls");
    let pulls = std::fs::read_to_string(dir.join("PULLS.md")).unwrap();
    assert!(
        pulls.contains(
            "| `try/lookup-table` | open | Try a lookup table for ranks | `step-002-lookup-table` | — |"
        ),
        "{pulls}"
    );
    assert!(
        pulls.contains(
            "| `from-char` | merged | Rank::from(char) | `step-005-from-char-fixed` | `step-006-merge-from-char` |"
        ),
        "{pulls}"
    );
    assert!(
        pulls.contains("Faster? Maybe. Correct? The test says no."),
        "{pulls}"
    );
}

#[test]
fn replay__a_linear_book_has_one_parent_and_no_line_trailers() {
    // Decision 13: a straight line's commits are what they were before
    // branches existed.
    let f = fixtures::rank_saga();
    let p = plan(&f.book, &f.catalog).unwrap().repos.remove(0);
    let (dir, _) = replay(&p, "linear");
    for step in &p.steps {
        let tag = step.tag();
        let listed = git(&dir, &["rev-list", "--parents", "-n", "1", &tag]);
        // No scaffolding here, so the first commit is a root.
        let want = if step.seq == 1 { 1 } else { 2 };
        assert_eq!(listed.split_whitespace().count(), want, "{tag}: {listed}");
        let msg = git(&dir, &["log", "-1", "--format=%B", &tag]);
        assert!(
            !msg.contains("Bower-Line") && !msg.contains("Bower-Merges"),
            "{tag}: {msg}"
        );
    }
    assert!(!dir.join("PULLS.md").exists());
}
