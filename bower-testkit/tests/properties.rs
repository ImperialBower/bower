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
