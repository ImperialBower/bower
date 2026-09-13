//! EPIC-09 in the kernel: lines of history, merges, and conflicts, planned
//! from real chapters. The thirteen `plan__` tests the spike had
//! (`docs/spikes/spike-branches/tests/spike.rs`, now deleted) are here under
//! the same names; the rest are new.

#![allow(non_snake_case, clippy::unwrap_used, clippy::expect_used)]

use bower_core::Errors;
use bower_testkit::prelude::*;

fn catalog() -> RepoCatalog {
    RepoCatalog::from_names(&["failers"])
}

fn plan_of(text: &str) -> Result<RepoPlan, Errors> {
    let book = BookSource::from_chapters(vec![Chapter::new("ch01-saga.md", text)]);
    plan(&book, &catalog()).map(|mut p| p.repos.remove(0))
}

fn saga() -> RepoPlan {
    plan_of(fixtures::BRANCH_SAGA).unwrap()
}

/// The saga with each `(from, to)` replaced once. Panics if the saga no longer
/// contains `from`, so a fixture edit cannot quietly turn a test into a no-op.
fn edited(edits: &[(&str, &str)]) -> String {
    let mut text = fixtures::BRANCH_SAGA.to_string();
    for (from, to) in edits {
        assert!(text.contains(from), "the saga no longer contains {from:?}");
        text = text.replacen(from, to, 1);
    }
    text
}

/// Fork the pull-request branch at the enum step, before main documented the
/// crate.
const EXPLICIT_FROM: (&str, &str) = (
    "step=\"from-char-broken\" branch=\"from-char\"",
    "step=\"from-char-broken\" branch=\"from-char\" from=\"rank-enum\"",
);

/// Main's middle step edits `src/rank.rs` instead of `src/lib.rs`.
const MAIN_TOUCHES_RANK: (&str, &str) = (
    "step=\"lib-doc\" file=\"src/lib.rs\" op=\"replace\" msg=\"ch01: document the crate\" -->\n```rust\n//! A deck of cards, built the failing way.\npub mod rank;\n```",
    "step=\"lib-doc\" file=\"src/rank.rs\" op=\"replace\" msg=\"ch01: main touches rank.rs\" -->\n```rust\n// main touched this\n#[derive(Clone, Copy, Debug, Eq, PartialEq)]\npub enum Rank { Ace, King, Queen, Blank }\n```",
);

/// The merge step replaces `src/rank.rs`: the resolution is a step.
const RESOLVED: (&str, &str) = (
    "step=\"merge-from-char\" merge=\"from-char\" op=\"none\" msg=\"ch02: merge from-char\" -->\n",
    "step=\"merge-from-char\" merge=\"from-char\" file=\"src/rank.rs\" op=\"replace\" msg=\"ch02: merge from-char, resolving rank.rs\" -->\n```rust\n// resolved by hand\n```\n",
);

const LIB: &str = "pub mod rank;\n";
const LIB_DOC: &str = "//! A deck of cards, built the failing way.\npub mod rank;\n";

#[test]
fn plan__branch_forks_from_nearest_preceding_main_step() {
    let p = saga();
    let lookup = &p.steps[1];
    assert_eq!(lookup.line, Line::Branch("try/lookup-table".into()));
    assert_eq!(lookup.parents, vec![1]);
    assert_eq!(
        p.steps[3].parents,
        vec![3],
        "forks from lib-doc, the main head at that point"
    );
    assert_eq!(p.steps[4].parents, vec![4], "continues its own branch");
}

#[test]
fn plan__explicit_from_forks_earlier() {
    let p = plan_of(&edited(&[EXPLICIT_FROM])).unwrap();
    assert_eq!(p.steps[3].parents, vec![1]);
    assert_eq!(p.branches[1].forked_from, 1);
    // The branch tree does not carry main's later change to lib.rs …
    assert_eq!(p.steps[3].tree.text("src/lib.rs"), Some(LIB));
    // … but the merge tree does.
    assert_eq!(p.steps[5].tree.text("src/lib.rs"), Some(LIB_DOC));
}

#[test]
fn plan__merge_has_two_parents_and_is_on_main() {
    let p = saga();
    let m = &p.steps[5];
    assert_eq!(m.line, Line::Main);
    assert_eq!(m.parents, vec![3, 5], "[main head, branch head]");
    assert_eq!(m.merges.as_deref(), Some("from-char"));
}

#[test]
fn plan__merge_tree_is_branch_blocks_over_main() {
    let p = saga();
    let m = &p.steps[5];
    // Main's change since the fork is kept …
    assert_eq!(m.tree.text("src/lib.rs"), Some(LIB_DOC));
    // … and the branch's final content lands on top.
    let rank = m.tree.text("src/rank.rs").unwrap();
    assert!(rank.contains("_ => Rank::Blank"), "{rank}");
    assert!(
        !rank.contains("RANKS"),
        "the abandoned branch never reaches main: {rank}"
    );
}

#[test]
fn plan__conflict_without_resolution_is_an_error() {
    let errs = plan_of(&edited(&[EXPLICIT_FROM, MAIN_TOUCHES_RANK])).unwrap_err();
    assert_eq!(errs.len(), 1, "{errs}");
    assert!(
        matches!(
            &errs.0[0],
            BowerError::MergeConflict { step, branch, file, region: None, .. }
                if step == "merge-from-char" && branch == "from-char" && file == "src/rank.rs"
        ),
        "{errs}"
    );
}

#[test]
fn plan__resolution_block_clears_the_conflict() {
    let p = plan_of(&edited(&[EXPLICIT_FROM, MAIN_TOUCHES_RANK, RESOLVED])).unwrap();
    let rank = p.steps[5].tree.text("src/rank.rs").unwrap();
    assert!(rank.starts_with("// resolved by hand"), "{rank}");
}

#[test]
fn plan__a_main_edit_before_the_fork_is_not_a_conflict() {
    // Without `from=`, the branch forks after main's edit and builds on it.
    let p = plan_of(&edited(&[MAIN_TOUCHES_RANK])).unwrap();
    let branch_rank = p.steps[3].tree.text("src/rank.rs").unwrap();
    assert!(
        branch_rank.starts_with("// main touched this"),
        "{branch_rank}"
    );
    assert_eq!(p.steps[5].parents, vec![3, 5], "and the merge is clean");
}

#[test]
fn plan__step_after_merge_is_refused() {
    let late = format!(
        "{}{}",
        fixtures::BRANCH_SAGA,
        "<!-- bower repo=\"failers\" step=\"from-char-late\" branch=\"from-char\" file=\"src/rank.rs\" op=\"append\" msg=\"ch02: one more on a closed branch\" -->\n```rust\n// too late\n```\n",
    );
    let errs = plan_of(&late).unwrap_err();
    assert!(
        matches!(
            &errs.0[0],
            BowerError::BranchAlreadyMerged { step, branch, merged_at, .. }
                if step == "from-char-late" && branch == "from-char" && merged_at == "merge-from-char"
        ),
        "{errs}"
    );
}

#[test]
fn plan__merge_of_unknown_branch_is_refused() {
    let errs = plan_of(&edited(&[(
        "merge=\"from-char\"",
        "merge=\"no-such-branch\"",
    )]))
    .unwrap_err();
    assert!(
        matches!(&errs.0[0], BowerError::MergeUnknownBranch { branch, .. } if branch == "no-such-branch"),
        "{errs}"
    );
}

#[test]
fn plan__merging_twice_is_refused() {
    let again = format!(
        "{}{}",
        fixtures::BRANCH_SAGA,
        "<!-- bower repo=\"failers\" step=\"merge-again\" merge=\"from-char\" op=\"none\" msg=\"ch02: merge from-char again\" -->\n",
    );
    let errs = plan_of(&again).unwrap_err();
    assert!(
        matches!(&errs.0[0], BowerError::BranchAlreadyMerged { step, .. } if step == "merge-again"),
        "{errs}"
    );
}

#[test]
fn plan__from_must_name_a_main_step() {
    let errs = plan_of(&edited(&[(
        "step=\"from-char-broken\" branch=\"from-char\"",
        "step=\"from-char-broken\" branch=\"from-char\" from=\"lookup-table\"",
    )]))
    .unwrap_err();
    assert!(
        matches!(&errs.0[0], BowerError::FromNotOnMain { from, .. } if from == "lookup-table"),
        "{errs}"
    );
}

#[test]
fn plan__from_on_a_main_step_is_refused() {
    let errs = plan_of(concat!(
        "<!-- bower repo=\"failers\" step=\"base\" file=\"a.rs\" -->\n```rust\nx\n```\n",
        "<!-- bower repo=\"failers\" step=\"later\" file=\"a.rs\" op=\"replace\" from=\"base\" -->\n```rust\ny\n```\n",
    ))
    .unwrap_err();
    assert!(
        matches!(
            &errs.0[0],
            BowerError::FromOnLaterStep { step, branch, .. }
                if step == "later" && branch == "main"
        ),
        "{errs}"
    );
}

#[test]
fn plan__from_on_a_merge_step_is_refused() {
    let errs = plan_of(&edited(&[(
        "step=\"merge-from-char\" merge=\"from-char\" op=\"none\" msg=\"ch02: merge from-char\" -->\n",
        "step=\"merge-from-char\" merge=\"from-char\" op=\"none\" from=\"rank-enum\" msg=\"ch02: merge from-char\" -->\n",
    )]))
    .unwrap_err();
    assert!(
        matches!(
            &errs.0[0],
            BowerError::FromOnLaterStep { step, branch, .. }
                if step == "merge-from-char" && branch == "main"
        ),
        "{errs}"
    );
}

#[test]
fn plan__a_resolution_does_not_drop_the_rest_of_a_branch_block() {
    // The branch's one block deletes both a.rs and b.rs; the merge step
    // resolves only a.rs. b.rs's deletion is not on the resolved list, so it
    // must still land — the whole block is not dropped because part of it
    // was resolved (a regression this test pins).
    let p = plan_of(concat!(
        "<!-- bower repo=\"failers\" step=\"base\" file=\"a.rs\" -->\n```rust\na1\n```\n",
        "<!-- bower repo=\"failers\" step=\"base\" file=\"b.rs\" -->\n```rust\nb1\n```\n",
        "<!-- bower repo=\"failers\" step=\"side\" branch=\"b\" op=\"delete\" file=\"a.rs\" paths=\"b.rs\" -->\n",
        "<!-- bower repo=\"failers\" step=\"later\" file=\"a.rs\" op=\"replace\" -->\n```rust\nmain2\n```\n",
        "<!-- bower repo=\"failers\" step=\"join\" merge=\"b\" file=\"a.rs\" op=\"replace\" msg=\"merge b\" -->\n```rust\nresolved\n```\n",
    ))
    .unwrap();
    let m = &p.steps[3];
    assert_eq!(m.tree.text("a.rs"), Some("resolved\n"));
    assert_eq!(m.tree.get("b.rs"), None, "b.rs's deletion must survive");
}

// New in the kernel: what the spike could not see — names, `from=` in the
// wrong place, regions, appends, and the summaries a replay and a lock read.

#[test]
fn plan__branch_named_main_is_refused() {
    let errs =
        plan_of("<!-- bower repo=\"failers\" file=\"a.rs\" branch=\"main\" -->\n```rust\nx\n```\n")
            .unwrap_err();
    assert!(
        matches!(&errs.0[0], BowerError::InvalidBranchName { name, .. } if name == "main"),
        "{errs}"
    );
}

#[test]
fn plan__from_on_a_later_step_is_refused() {
    let errs = plan_of(&edited(&[(
        "step=\"from-char-fixed\" branch=\"from-char\"",
        "step=\"from-char-fixed\" branch=\"from-char\" from=\"rank-enum\"",
    )]))
    .unwrap_err();
    assert!(
        matches!(
            &errs.0[0],
            BowerError::FromOnLaterStep { step, branch, .. }
                if step == "from-char-fixed" && branch == "from-char"
        ),
        "{errs}"
    );
}

const REGIONS: &str = concat!(
    "<!-- bower repo=\"failers\" step=\"base\" file=\"a.rs\" -->\n",
    "```rust\n// bower:begin one\n// bower:end one\n// bower:begin two\n// bower:end two\n```\n",
    "<!-- bower repo=\"failers\" step=\"side\" branch=\"b\" file=\"a.rs\" op=\"region\" region=\"one\" -->\n",
    "```rust\nfn one() {}\n```\n",
    "<!-- bower repo=\"failers\" step=\"main-two\" file=\"a.rs\" op=\"region\" region=\"two\" -->\n",
    "```rust\nfn two() {}\n```\n",
    "<!-- bower repo=\"failers\" step=\"join\" merge=\"b\" op=\"none\" msg=\"merge b\" -->\n",
);

#[test]
fn plan__conflict_is_per_region_not_per_file() {
    // Distinct regions of one file compose: both sides' work survives.
    let p = plan_of(REGIONS).unwrap();
    assert_eq!(
        p.steps[3].tree.text("a.rs"),
        Some("fn one() {}\nfn two() {}\n")
    );

    // The same region on both sides does not.
    let clash = REGIONS.replacen("region=\"two\"", "region=\"one\"", 1);
    let errs = plan_of(&clash).unwrap_err();
    assert!(
        matches!(
            &errs.0[0],
            BowerError::MergeConflict { file, region: Some(r), .. } if file == "a.rs" && r == "one"
        ),
        "{errs}"
    );
}

#[test]
fn plan__two_appends_compose_at_a_merge() {
    let p = plan_of(concat!(
        "<!-- bower repo=\"failers\" step=\"base\" file=\"a.rs\" -->\n```rust\nstart\n```\n",
        "<!-- bower repo=\"failers\" step=\"side\" branch=\"b\" file=\"a.rs\" op=\"append\" -->\n```rust\nfrom the branch\n```\n",
        "<!-- bower repo=\"failers\" step=\"more\" file=\"a.rs\" op=\"append\" -->\n```rust\nfrom main\n```\n",
        "<!-- bower repo=\"failers\" step=\"join\" merge=\"b\" op=\"none\" msg=\"merge b\" -->\n",
    ))
    .unwrap();
    // Main's append first, then the branch's re-applied over it (Decision 3).
    assert_eq!(
        p.steps[3].tree.text("a.rs"),
        Some("start\nfrom main\nfrom the branch\n")
    );
}

#[test]
fn plan__branches_are_summarized_in_the_order_they_appear() {
    let p = saga();
    let got: Vec<(&str, usize, usize, Option<usize>)> = p
        .branches
        .iter()
        .map(|b| (b.name.as_str(), b.forked_from, b.head, b.merged_at))
        .collect();
    assert_eq!(
        got,
        vec![
            ("try/lookup-table", 1, 2, None),
            ("from-char", 3, 5, Some(6))
        ]
    );
}

#[test]
fn plan__unmerged_branch_survives_with_an_open_pr() {
    let p = saga();
    let b = &p.branches[0];
    assert_eq!(b.name, "try/lookup-table");
    assert_eq!(b.merged_at, None);
    let pr = b.pr.as_ref().unwrap();
    assert_eq!(pr.state, PrState::Open);
    assert_eq!(pr.title, "Try a lookup table for ranks");
    assert_eq!(
        pr.body,
        vec!["Faster? Maybe. Correct? The test says no.".to_string()]
    );
    let b = &p.branches[1];
    assert_eq!(b.merged_at, Some(6));
    assert_eq!(b.pr.as_ref().unwrap().state, PrState::Merged);
}

#[test]
fn plan__a_pr_block_is_never_a_step() {
    assert_eq!(saga().steps.len(), 6);
}

#[test]
fn plan__pr_key_form_has_no_body() {
    let p = plan_of(concat!(
        "<!-- bower repo=\"failers\" step=\"base\" file=\"a.rs\" -->\n```rust\nx\n```\n",
        "<!-- bower repo=\"failers\" step=\"side\" branch=\"b\" file=\"b.rs\" pr=\"Key form\" -->\n```rust\ny\n```\n",
    ))
    .unwrap();
    let pr = p.branches[0].pr.as_ref().unwrap();
    assert_eq!(pr.title, "Key form");
    assert!(pr.body.is_empty());
    assert_eq!(pr.state, PrState::Open);
}

#[test]
fn plan__pr_on_a_main_step_is_refused() {
    let errs =
        plan_of("<!-- bower repo=\"failers\" file=\"a.rs\" pr=\"On main\" -->\n```rust\nx\n```\n")
            .unwrap_err();
    assert!(
        matches!(&errs.0[0], BowerError::PrWithoutBranch { loc } if loc.line == 1),
        "{errs}"
    );
}

#[test]
fn plan__two_prs_on_one_branch_is_refused() {
    let text = format!(
        "{}{}",
        fixtures::BRANCH_SAGA,
        "<!-- bower repo=\"failers\" branch=\"from-char\" pr=\"Again\" -->\n```markdown\nSecond.\n```\n",
    );
    // The directive is the fourth line from the end: directive, fence,
    // "Second.", fence.
    let line = text.lines().count() - 3;
    let errs = plan_of(&text).unwrap_err();
    assert!(
        matches!(&errs.0[0], BowerError::PrDuplicate { branch, loc } if branch == "from-char" && loc.line == line),
        "{errs}"
    );
}

#[test]
fn plan__pr_block_for_an_unknown_branch_is_refused() {
    let text = format!(
        "{}{}",
        fixtures::BRANCH_SAGA,
        "<!-- bower repo=\"failers\" branch=\"ghost\" pr=\"Nowhere\" -->\n```markdown\nx\n```\n",
    );
    let errs = plan_of(&text).unwrap_err();
    assert!(
        matches!(&errs.0[0], BowerError::PrUnknownBranch { branch, .. } if branch == "ghost"),
        "{errs}"
    );
}

#[test]
fn plan__lock_text_shows_lines_parents_and_branches() {
    let t = lock_text(&plan(&fixtures::branch_saga().book, &catalog()).unwrap());
    assert!(
        t.contains(" files=src/rank.rs line=try/lookup-table parents=001\n"),
        "{t}"
    );
    assert!(t.contains(" files= parents=003,005 merges=from-char\n"), "{t}");
    let lib_doc = t.lines().find(|l| l.starts_with("003 lib-doc ")).unwrap();
    assert!(
        lib_doc.ends_with(" files=src/lib.rs"),
        "a main step's line gains nothing: {lib_doc}"
    );
    assert!(t.contains("\n\n[failers.branches]\n"), "{t}");
    assert!(
        t.contains(concat!(
            "try/lookup-table from=001 head=002 merged=no pr=\"Try a lookup table for ranks\" state=open\n",
            "    | Faster? Maybe. Correct? The test says no.\n",
        )),
        "{t}"
    );
    assert!(
        t.contains("from-char from=003 head=005 merged=006 pr=\"Rank::from(char)\" state=merged\n"),
        "{t}"
    );
}

#[test]
fn plan__exercise_on_a_merged_head_answers_with_the_merge() {
    let p = plan_of(&edited(&[(
        "msg=\"ch02: From<char>, every arm answered\"",
        "msg=\"ch02: From<char>, every arm answered\" exercise=\"Merge it\"",
    )]))
    .unwrap();
    let x = p.steps[4].exercise.as_ref().unwrap();
    assert_eq!(x.answer, StepId("merge-from-char".into()));
}

#[test]
fn plan__exercise_on_a_main_step_is_answered_on_main() {
    // The next step in the book is on a branch; the answer is the next one on
    // main.
    let p = plan_of(&edited(&[(
        "msg=\"ch01: the Rank enum\"",
        "msg=\"ch01: the Rank enum\" exercise=\"Document the crate\"",
    )]))
    .unwrap();
    let x = p.steps[0].exercise.as_ref().unwrap();
    assert_eq!(x.answer, StepId("lib-doc".into()));
}

#[test]
fn plan__exercise_on_an_unmerged_head_has_no_answer() {
    let errs = plan_of(&edited(&[(
        "msg=\"ch01: try a lookup table instead\"",
        "msg=\"ch01: try a lookup table instead\" exercise=\"Make the test pass\"",
    )]))
    .unwrap_err();
    assert!(
        matches!(&errs.0[0], BowerError::ExerciseWithoutAnswer { step, .. } if step == "lookup-table"),
        "{errs}"
    );
}

#[test]
fn plan__branch_names_that_collide_as_refs_are_refused() {
    let errs =
        plan_of("<!-- bower repo=\"failers\" file=\"a.rs\" branch=\"main/x\" -->\n```rust\nx\n```\n")
            .unwrap_err();
    assert!(
        matches!(&errs.0[0], BowerError::InvalidBranchName { name, .. } if name == "main/x"),
        "{errs}"
    );

    let errs = plan_of(concat!(
        "<!-- bower repo=\"failers\" step=\"one\" file=\"a.rs\" branch=\"a\" -->\n```rust\nx\n```\n",
        "<!-- bower repo=\"failers\" step=\"two\" file=\"b.rs\" branch=\"a/b\" -->\n```rust\ny\n```\n",
    ))
    .unwrap_err();
    assert!(
        matches!(&errs.0[0], BowerError::InvalidBranchName { name, .. } if name == "a/b"),
        "{errs}"
    );

    let errs = plan_of(concat!(
        "<!-- bower repo=\"failers\" step=\"one\" file=\"a.rs\" branch=\"Foo\" -->\n```rust\nx\n```\n",
        "<!-- bower repo=\"failers\" step=\"two\" file=\"b.rs\" branch=\"foo\" -->\n```rust\ny\n```\n",
    ))
    .unwrap_err();
    assert!(
        matches!(&errs.0[0], BowerError::InvalidBranchName { name, .. } if name == "foo"),
        "{errs}"
    );

    // Siblings under one prefix do not collide.
    let p = plan_of(concat!(
        "<!-- bower repo=\"failers\" step=\"one\" file=\"a.rs\" branch=\"try/x\" -->\n```rust\nx\n```\n",
        "<!-- bower repo=\"failers\" step=\"two\" file=\"b.rs\" branch=\"try/y\" -->\n```rust\ny\n```\n",
    ))
    .unwrap();
    assert_eq!(p.branches.len(), 2);
}

#[test]
fn plan__a_straight_line_has_one_parent_each_and_no_branches() {
    let f = fixtures::rank_saga();
    let p = plan(&f.book, &f.catalog).unwrap();
    let repo = &p.repos[0];
    assert!(repo.branches.is_empty());
    for s in &repo.steps {
        assert_eq!(s.line, Line::Main);
        assert_eq!(s.parents, vec![s.seq - 1], "step {}", s.seq);
        assert_eq!(s.merges, None);
    }
}
