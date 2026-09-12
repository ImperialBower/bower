//! Corpus tests: every valid fixture plans cleanly, every broken fixture
//! produces exactly the error family its name claims, and the corpus as a
//! whole achieves full state coverage.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use bower_testkit::prelude::*;

#[test]
fn every_valid_fixture_plans_cleanly() {
    for f in fixtures::valid() {
        let result = plan(&f.book, &f.catalog);
        assert!(
            result.is_ok(),
            "fixture `{}` failed to plan:\n{}",
            f.name,
            result.unwrap_err()
        );
    }
}

#[test]
fn every_broken_fixture_produces_its_named_error() {
    for (variant, f) in fixtures::broken() {
        let errs = plan(&f.book, &f.catalog)
            .err()
            .unwrap_or_else(|| panic!("broken fixture `{variant}` planned cleanly"));
        let hit = errs.0.iter().any(|e| {
            // Debug form starts with the variant name.
            format!("{e:?}").starts_with(variant)
        });
        assert!(hit, "fixture `{variant}` produced other errors: {errs}");
    }
}

#[test]
fn corpus_state_coverage_is_complete() {
    let mut corpus = fixtures::valid();
    corpus.extend(fixtures::broken().into_iter().map(|(_, f)| f));
    let report = CoverageReport::measure(&corpus);
    assert!(report.is_complete(), "coverage gaps:\n{report}");
    // The exercise axis is a set of its own: an exercise on every `expect`.
    assert!(
        report.missing_exercise_expects().is_empty(),
        "coverage gaps:\n{report}"
    );
    assert!(
        report.missing_capture_expects().is_empty(),
        "coverage gaps:\n{report}"
    );
}

#[test]
fn captured_outputs_bind_every_legal_pair() {
    let f = fixtures::captured_outputs();
    let p = plan(&f.book, &f.catalog).unwrap();
    let steps = &p.repo("failers").unwrap().steps;
    let captures = |id: &str| {
        steps
            .iter()
            .find(|s| s.id.0 == id)
            .unwrap()
            .outputs
            .iter()
            .map(|o| o.capture)
            .collect::<Vec<_>>()
    };

    // `green`'s verify block sits last in the book and reaches back by name.
    assert_eq!(captures("green"), vec![Capture::Check, Capture::Verify]);
    assert_eq!(captures("broken"), vec![Capture::Check]);
    assert_eq!(captures("red"), vec![Capture::Check, Capture::Verify]);

    assert_eq!(
        steps[1].outputs[0].lines,
        vec![
            "error[E0308]: mismatched types".to_string(),
            ELISION.to_string()
        ]
    );
    assert!(
        steps[0].outputs[0].lines.is_empty(),
        "an empty fence is not recorded yet"
    );
    // Output blocks never reach a tree.
    assert_eq!(
        steps[2].tree.paths().collect::<Vec<_>>(),
        vec!["src/lib.rs"]
    );
}

#[test]
fn rank_saga_replays_the_diary_story() {
    let f = fixtures::rank_saga();
    let p = plan(&f.book, &f.catalog).unwrap();
    let repo = p.repo("failers").unwrap();

    assert_eq!(repo.steps.len(), 3);
    assert_eq!(repo.steps[0].tag(), "step-001-rank-enum");
    assert_eq!(repo.steps[1].expect, Expect::CompileFail);
    // The fix step groups two blocks (region + tests) into one commit.
    assert_eq!(repo.steps[2].id.0, "from-char");
    assert_eq!(repo.steps[2].displays.len(), 2);

    let final_rank = repo.steps[2].tree.text("src/rank.rs").unwrap();
    assert!(final_rank.contains("mod rank_tests"));
    assert!(
        !final_rank.contains("bower:begin"),
        "markers must be stripped"
    );
    // The broken step's tree is preserved at its own tag, not erased by the fix.
    let broken = repo.steps[1].tree.text("src/rank.rs").unwrap();
    assert!(broken.contains("match c { 'A' => Rank::Ace }"));
}

#[test]
fn display_textures_render_and_map() {
    let f = fixtures::display_textures();
    let p = plan(&f.book, &f.catalog).unwrap();
    let repo = p.repo("failers").unwrap();

    // Anonymous span: one span shown, boilerplate elided, range anchored.
    let show = &repo.steps[0].displays[0];
    assert_eq!(show.spans.len(), 1);
    assert_eq!(
        show.spans[0].visible_lines,
        vec!["pub fn the_point() {}".to_string()]
    );
    assert_eq!(show.elided_lines, 2);
    let range = show.ranges[0].clone().unwrap();
    assert_eq!(range.file, "src/show.rs");

    // Named spans filtered by show="two": `one` elided, `two` rendered.
    let named = &repo.steps[1].displays[0];
    assert_eq!(named.spans.len(), 1);
    assert_eq!(named.spans[0].name.as_deref(), Some("two"));
    assert_eq!(named.elided_lines, 1);

    // hidden="false": the hidden line never reaches the repo.
    let silent = repo.steps[2].tree.text("src/silent.rs").unwrap();
    assert_eq!(silent, "fn in_repo() {}\n");
}

#[test]
fn op_sampler_folds_every_op() {
    let f = fixtures::op_sampler();
    let p = plan(&f.book, &f.catalog).unwrap();
    let repo = p.repo("failers").unwrap();
    let last = repo.steps.last().unwrap();
    // After the delete step, only b.rs survives.
    assert_eq!(last.tree.paths().collect::<Vec<_>>(), vec!["b.rs"]);
    // The copy step held real bytes while it lived.
    let copy_step = repo
        .steps
        .iter()
        .find(|s| s.files == vec!["logo.png".to_string()])
        .unwrap();
    assert!(matches!(
        copy_step.tree.get("logo.png"),
        Some(FileBody::Binary(_))
    ));
    // The prose step touched nothing and skipped verification.
    let prose = repo.steps.iter().find(|s| s.files.is_empty()).unwrap();
    assert_eq!(prose.expect, Expect::Skip);
    assert_eq!(prose.msg, "a refactor the book narrates");
}

#[test]
fn multi_repo_after_bends_order() {
    let f = fixtures::multi_repo();
    let p = plan(&f.book, &f.catalog).unwrap();
    let failers = p.repo("failers").unwrap();
    assert_eq!(failers.steps[0].id.0, "late");
    assert_eq!(failers.steps[1].id.0, "early");
    assert_eq!(p.repo("clock").unwrap().steps.len(), 1);
}

#[test]
fn include_library_merges_defaults_and_overrides() {
    let f = fixtures::include_library();
    let p = plan(&f.book, &f.catalog).unwrap();
    let step = &p.repo("failers").unwrap().steps[0];
    assert_eq!(step.msg, "Rank enum — the café edition ♠");
    assert_eq!(
        step.tree.text("src/rank.rs").unwrap(),
        "pub enum Rank { Ace }\n"
    );
}

#[test]
fn notebook_play_binds_cells_to_steps() {
    let f = fixtures::notebook_play();
    let p = plan(&f.book, &f.catalog).unwrap();
    let repo = p.repo("failers").unwrap();

    let rank = repo.steps.iter().find(|s| s.id.0 == "rank").unwrap();
    // The doc-order cell binds to the step just built; the explicit
    // step="rank" cell reaches back past the `lo` step.
    assert_eq!(rank.play_cells.len(), 2);
    assert_eq!(rank.play_cells[0].info, "python");
    assert_eq!(
        rank.play_cells[0].lines,
        vec![
            "from failers import Rank".to_string(),
            "Rank.from_char(\"A\")".to_string()
        ]
    );

    let lo = repo.steps.iter().find(|s| s.id.0 == "lo").unwrap();
    assert!(lo.play_cells.is_empty());

    // Play cells never touch trees, and the lock records them.
    assert!(rank.tree.text("src/rank.rs").unwrap().contains("enum Rank"));
    assert!(lock_text(&p).contains("play=2"));
}

#[test]
fn exercise_forms_bind_both_forms_on_every_expect() {
    let f = fixtures::exercise_forms();
    let p = plan(&f.book, &f.catalog).unwrap();
    let steps = &p.repo("failers").unwrap().steps;
    let by_id = |id: &str| steps.iter().find(|s| s.id.0 == id).unwrap();

    let broken = by_id("broken").exercise.as_ref().unwrap();
    assert_eq!(broken.form, ExerciseForm::Key);
    assert_eq!(broken.answer.0, "fixed");

    // The positional block form binds to the step just built.
    let fixed = by_id("fixed").exercise.as_ref().unwrap();
    assert_eq!(fixed.form, ExerciseForm::Block);
    assert_eq!(fixed.detail[0], "Parse text instead.");
    assert_eq!(fixed.answer.0, "tested");

    // The explicit `step=` block form reaches back past `green`.
    let tested = by_id("tested").exercise.as_ref().unwrap();
    assert_eq!(tested.task, "Make the test pass");
    assert_eq!(tested.answer.0, "green");
    assert!(by_id("green").exercise.is_none());

    // A prose step can carry one too; its answer is whatever follows.
    assert_eq!(by_id("narrated").exercise.as_ref().unwrap().answer.0, "end");
    assert!(by_id("end").exercise.is_none());

    let lock = lock_text(&p);
    assert!(lock.contains("exercise form=key answer=fixed"), "{lock}");
    assert!(lock.contains("exercise form=block answer=tested"), "{lock}");
}

#[test]
fn self_hosting_chapter_ignores_fenced_directives() {
    let f = fixtures::self_hosting_chapter();
    let p = plan(&f.book, &f.catalog).unwrap();
    let repo = p.repo("failers").unwrap();
    assert_eq!(repo.steps.len(), 1);
    assert!(repo.steps[0].tree.text("real.rs").is_some());
    assert!(repo.steps[0].tree.text("would-be-a-bug.rs").is_none());
}
