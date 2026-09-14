//! The kernel's invariants, proven over arbitrary valid books:
//!
//! 1. **Determinism** — equal inputs, equal plans, byte for byte.
//! 2. **Generated validity** — every book the generator can produce plans
//!    cleanly (the generator walks only the legal state space).
//! 3. **Line-map exactness** — for every displayed span with a range, the
//!    lines at that range in the materialized tree are exactly the span's
//!    text. This is the property that makes line-anchored source links
//!    trustworthy.
//! 4. **Lock stability** — lock text is a pure function of the plan.
//! 5. **Normalization is idempotent** and blind to colour and line endings.
//! 6. **Recording round-trips** — a rewritten fence plans back to exactly the
//!    lines written, and nothing outside the fence moves.
//! 7. **Branching is a fold** — a merge with nothing new on main since the
//!    fork has exactly the branch head's tree.
//! 8. **Parents come first** — every parent of a step is an earlier step.
//! 9. **A branch head is its line's last step.**

#![allow(clippy::unwrap_used, clippy::expect_used)]

use bower_testkit::prelude::*;
use proptest::prelude::*;

proptest! {
    #[test]
    fn generated_books_plan_cleanly((book, catalog) in arb_book()) {
        let result = plan(&book, &catalog);
        prop_assert!(result.is_ok(), "generated book failed:\n{}", result.unwrap_err());
    }

    #[test]
    fn plan_is_deterministic((book, catalog) in arb_book()) {
        let a = plan(&book, &catalog).unwrap();
        let b = plan(&book, &catalog).unwrap();
        prop_assert_eq!(a, b);
    }

    #[test]
    fn line_map_is_exact((book, catalog) in arb_book()) {
        let p = plan(&book, &catalog).unwrap();
        for repo in &p.repos {
            for step in &repo.steps {
                for display in &step.displays {
                    for (span, range) in display.spans.iter().zip(&display.ranges) {
                        let Some(range) = range else { continue };
                        let text = step.tree.text(&range.file).unwrap();
                        let lines: Vec<&str> = text.lines().collect();
                        let window: Vec<&str> =
                            lines[range.start - 1..range.end].to_vec();
                        let keep = catalog.spec(&repo.repo).keep_region_markers;
                        let expected = span.mapped_lines(keep);
                        prop_assert_eq!(window, expected);
                    }
                }
            }
        }
    }

    #[test]
    fn lock_text_is_deterministic((book, catalog) in arb_book()) {
        let a = lock_text(&plan(&book, &catalog).unwrap());
        let b = lock_text(&plan(&book, &catalog).unwrap());
        prop_assert_eq!(a, b);
    }

    #[test]
    fn normalize_is_idempotent(raw in arb_raw_output()) {
        let once = normalize(&raw, &Scrub::default());
        let twice = normalize(&once.join("\n"), &Scrub::default());
        prop_assert_eq!(once, twice);
    }

    #[test]
    fn normalize_ignores_ansi_and_crlf(lines in arb_plain_lines()) {
        let plain = lines.join("\n");
        let dressed = lines
            .iter()
            .map(|l| format!("\x1b[1m{l}\x1b[0m"))
            .collect::<Vec<_>>()
            .join("\r\n");
        prop_assert_eq!(
            normalize(&dressed, &Scrub::default()),
            normalize(&plain, &Scrub::default())
        );
    }

    #[test]
    fn rewrite_round_trips_through_plan(raw in arb_raw_output()) {
        let live = normalize(&raw, &Scrub::default());
        // Line 8 is the output directive.
        let chapter = concat!(
            "# Out\n\n",
            "<!-- bower repo=\"gen\" file=\"a.rs\" -->\n```rust\nx\n```\n\n",
            "<!-- bower repo=\"gen\" output=\"check\" -->\n```text\n```\n\n",
            "The end.\n",
        );
        let text = rewrite(chapter, 8, &live);
        let book = BookSource::from_chapters(vec![Chapter::new("out.md", &text)]);
        let p = plan(&book, &RepoCatalog::from_names(&["gen"])).unwrap();
        prop_assert_eq!(&p.repos[0].steps[0].outputs[0].lines, &live);
    }

    #[test]
    fn rewrite_touches_nothing_outside_the_fence(
        live in arb_plain_lines(),
        before in arb_words(),
        after in arb_words(),
    ) {
        let head: String = before.iter().map(|l| format!("{l}\n")).collect::<String>()
            + "<!-- bower repo=\"gen\" output=\"check\" -->\n";
        let tail: String = "\n".to_string()
            + &after.iter().map(|l| format!("{l}\n")).collect::<String>();
        let text = format!("{head}```text\nold\n```{tail}");
        let out = rewrite(&text, before.len() + 1, &live);
        prop_assert!(out.starts_with(&head), "{out}");
        prop_assert!(out.ends_with(&tail), "{out}");
    }

    #[test]
    fn generated_branch_books_plan_cleanly((book, catalog) in arb_branch_book()) {
        let result = plan(&book, &catalog);
        prop_assert!(result.is_ok(), "generated book failed:\n{}", result.unwrap_err());
    }

    #[test]
    fn a_merge_with_nothing_new_on_main_is_the_branch_head((book, catalog) in arb_branch_book()) {
        let p = plan(&book, &catalog).unwrap();
        let repo = &p.repos[0];
        let side = &repo.branches[0];
        if let Some(at) = side.merged_at {
            let main_moved = repo.steps.iter().any(|s| {
                s.line == Line::Main && s.merges.is_none() && s.seq > side.forked_from && s.seq < at
            });
            if !main_moved {
                prop_assert_eq!(&repo.steps[at - 1].tree, &repo.steps[side.head - 1].tree);
            }
        }
    }

    #[test]
    fn every_parent_comes_before_its_child((book, catalog) in arb_branch_book()) {
        let p = plan(&book, &catalog).unwrap();
        for s in &p.repos[0].steps {
            prop_assert!(!s.parents.is_empty());
            for parent in &s.parents {
                prop_assert!(*parent < s.seq, "step {} has parent {}", s.seq, parent);
            }
        }
    }

    #[test]
    fn a_branch_head_is_the_last_step_on_its_line((book, catalog) in arb_branch_book()) {
        let p = plan(&book, &catalog).unwrap();
        let repo = &p.repos[0];
        for b in &repo.branches {
            let last = repo
                .steps
                .iter()
                .filter(|s| s.line == Line::Branch(b.name.clone()))
                .map(|s| s.seq)
                .max();
            prop_assert_eq!(Some(b.head), last);
        }
    }
}

#[test]
fn line_map_is_exact_on_the_fixture_corpus() {
    for f in fixtures::valid() {
        let p = plan(&f.book, &f.catalog).unwrap();
        for repo in &p.repos {
            for step in &repo.steps {
                for display in &step.displays {
                    for (span, range) in display.spans.iter().zip(&display.ranges) {
                        let Some(range) = range else { continue };
                        let text = step.tree.text(&range.file).unwrap();
                        let lines: Vec<&str> = text.lines().collect();
                        let window = &lines[range.start - 1..range.end];
                        let keep = f.catalog.spec(&repo.repo).keep_region_markers;
                        let expected = span.mapped_lines(keep);
                        assert_eq!(window, expected, "fixture `{}` step `{}`", f.name, step.id);
                    }
                }
            }
        }
    }
}
