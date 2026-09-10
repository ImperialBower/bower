#![allow(non_snake_case)]
use bower_spike_branches::*;

#[test]
fn plan__branch_forks_from_nearest_preceding_main_step() {
    let p = plan(&fixtures::rank_saga()).unwrap();
    let lookup = &p.steps[1];
    assert_eq!(lookup.line, Line::Branch("try/lookup-table".into()));
    assert_eq!(lookup.parents, vec![1]);
    let broken = &p.steps[3];
    assert_eq!(broken.parents, vec![3], "forks from lib-doc, the main head at that point");
    let fixed = &p.steps[4];
    assert_eq!(fixed.parents, vec![4], "continues its own branch");
}

#[test]
fn plan__explicit_from_forks_earlier() {
    let p = plan(&fixtures::explicit_from()).unwrap();
    assert_eq!(p.steps[3].parents, vec![1]);
    assert_eq!(p.branches[1].forked_from, 1);
    // The branch tree does not carry main's later change to lib.rs …
    assert_eq!(p.steps[3].tree.0["src/lib.rs"], format!("{}\n", fixtures::LIB));
    // … but the merge tree does.
    assert_eq!(p.steps[5].tree.0["src/lib.rs"], format!("{}\n", fixtures::LIB_DOC));
}

#[test]
fn plan__merge_has_two_parents_and_is_on_main() {
    let p = plan(&fixtures::rank_saga()).unwrap();
    let m = &p.steps[5];
    assert_eq!(m.line, Line::Main);
    assert_eq!(m.parents, vec![3, 5], "[main head, branch head]");
    assert_eq!(m.merges.as_deref(), Some("from-char"));
}

#[test]
fn plan__merge_tree_is_branch_blocks_over_main() {
    let p = plan(&fixtures::rank_saga()).unwrap();
    let m = &p.steps[5];
    // main's change since the fork is kept …
    assert_eq!(m.tree.0["src/lib.rs"], format!("{}\n", fixtures::LIB_DOC));
    // … and the branch's final content lands on top.
    assert!(m.tree.0["src/rank.rs"].contains("_ => Rank::Blank"));
    assert!(!m.tree.0["src/rank.rs"].contains("RANKS"), "the abandoned branch never reaches main");
}

#[test]
fn plan__unmerged_branch_survives_with_an_open_pr() {
    let p = plan(&fixtures::rank_saga()).unwrap();
    let b = &p.branches[0];
    assert_eq!(b.name, "try/lookup-table");
    assert_eq!(b.merged_at, None);
    assert_eq!(b.pr.as_ref().unwrap().state, PrState::Open);
    let b = &p.branches[1];
    assert_eq!(b.merged_at, Some(6));
    assert_eq!(b.pr.as_ref().unwrap().state, PrState::Merged);
}

#[test]
fn plan__conflict_without_resolution_is_an_error() {
    let es = plan(&fixtures::conflict_unresolved()).unwrap_err();
    assert_eq!(es, vec![Error::MergeConflict { step: "merge-from-char".into(), branch: "from-char".into(), file: "src/rank.rs".into() }]);
}

#[test]
fn plan__resolution_block_clears_the_conflict() {
    let p = plan(&fixtures::conflict_resolved()).unwrap();
    assert!(p.steps[5].tree.0["src/rank.rs"].starts_with("// resolved by hand"));
}

#[test]
fn plan__step_after_merge_is_refused() {
    let es = plan(&fixtures::step_after_merge()).unwrap_err();
    assert!(matches!(&es[0], Error::BranchAlreadyMerged { step, branch, merged_at }
        if step == "from-char-late" && branch == "from-char" && merged_at == "merge-from-char"));
}

#[test]
fn plan__merge_of_unknown_branch_is_refused() {
    let es = plan(&fixtures::merge_unknown()).unwrap_err();
    assert!(matches!(&es[0], Error::MergeUnknownBranch { .. }));
}

#[test]
fn plan__merging_twice_is_refused() {
    let es = plan(&fixtures::merge_twice()).unwrap_err();
    assert!(matches!(&es[0], Error::BranchAlreadyMerged { step, .. } if step == "merge-again"));
}

#[test]
fn plan__from_must_name_a_main_step() {
    let es = plan(&fixtures::from_not_on_main()).unwrap_err();
    assert!(matches!(&es[0], Error::FromNotOnMain { .. }));
}

#[test]
fn plan__lock_text_shows_lines_parents_and_branches() {
    let p = plan(&fixtures::rank_saga()).unwrap();
    let t = lock_text(&p);
    assert!(t.contains("006 merge-from-char expect=pass line=main parents=003,005 merges=from-char"));
    assert!(t.contains("try/lookup-table from=001 head=002 merged=no pr=\"Try a lookup table for ranks\" state=open"));
    assert!(t.contains("from-char from=003 head=005 merged=006 pr=\"Rank::from(char)\" state=merged"));
}

fn tmp(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("bower-spike-{name}-{}", std::process::id()))
}

#[test]
fn replay__is_byte_identical_across_runs() {
    let p = plan(&fixtures::rank_saga()).unwrap();
    let a = replay::run(&p, &tmp("a")).unwrap();
    let b = replay::run(&p, &tmp("b")).unwrap();
    assert_eq!(a.refs, b.refs);
    assert_eq!(a.commits, 7);
}

#[test]
fn replay__merge_commit_has_two_parents_and_branches_are_refs() {
    let p = plan(&fixtures::rank_saga()).unwrap();
    let dir = tmp("c");
    let r = replay::run(&p, &dir).unwrap();
    let out = std::process::Command::new("git").arg("-C").arg(&dir)
        .args(["rev-list", "--parents", "-n", "1", "main"]).output().unwrap();
    let line = String::from_utf8(out.stdout).unwrap();
    let parts: Vec<&str> = line.split_whitespace().collect();
    assert_eq!(parts.len(), 3, "merge commit: self + two parents");
    assert_eq!(parts[1], r.refs["refs/tags/step-003-lib-doc"]);
    assert_eq!(parts[2], r.refs["refs/tags/step-005-from-char-fixed"]);
    assert_eq!(r.refs["refs/heads/try/lookup-table"], r.refs["refs/tags/step-002-lookup-table"]);
    assert_eq!(r.refs["refs/heads/from-char"], r.refs["refs/tags/step-005-from-char-fixed"]);
}

#[test]
fn replay__a_change_on_the_branch_changes_only_what_descends_from_it() {
    let base = plan(&fixtures::rank_saga()).unwrap();
    let mut edited = fixtures::rank_saga();
    edited[4].blocks[0].lines.push("// edited on the branch".into());
    let changed = plan(&edited).unwrap();
    let a = replay::run(&base, &tmp("d")).unwrap();
    let b = replay::run(&changed, &tmp("e")).unwrap();
    for t in ["step-001-rank-enum", "step-002-lookup-table", "step-003-lib-doc", "step-004-from-char-broken"] {
        assert_eq!(a.refs[&format!("refs/tags/{t}")], b.refs[&format!("refs/tags/{t}")], "{t} untouched");
    }
    assert_ne!(a.refs["refs/tags/step-005-from-char-fixed"], b.refs["refs/tags/step-005-from-char-fixed"]);
    assert_ne!(a.refs["refs/heads/main"], b.refs["refs/heads/main"], "the merge descends from the edit");
    assert_eq!(a.refs["refs/heads/try/lookup-table"], b.refs["refs/heads/try/lookup-table"]);
}

#[test]
fn plan__a_main_edit_before_the_fork_is_not_a_conflict() {
    let mut v = fixtures::conflict_unresolved();
    v[3].from = None; // fork after main's edit instead
    let p = plan(&v).unwrap();
    // The branch simply builds on main's edit …
    assert!(p.steps[3].tree.0["src/rank.rs"].starts_with("// main touched this"));
    // … and the merge is clean.
    assert_eq!(p.steps[5].parents, vec![3, 5]);
}
