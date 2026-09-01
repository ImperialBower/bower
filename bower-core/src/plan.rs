//! The kernel's front door: [`plan`] takes a book and a catalog and
//! returns everything the replay layer, the preprocessor, and the
//! verifier need — or every error it could find, located.

use crate::directive::{Expect, Op};
use crate::display::{self, BlockDisplay};
use crate::source::{BookSource, Location, RepoCatalog, RepoName};
use crate::step::{self, Step, StepId};
use crate::tree::TreeState;
use crate::{block, BowerError, Errors};

/// The resolved plan for a whole book: one [`RepoPlan`] per repo the book
/// actually feeds, in catalog order.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BookPlan {
    pub repos: Vec<RepoPlan>,
}

impl BookPlan {
    #[must_use]
    pub fn repo(&self, name: &str) -> Option<&RepoPlan> {
        self.repos.iter().find(|r| r.repo.0 == name)
    }
}

/// One repo's ordered steps, each carrying its full materialized tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepoPlan {
    pub repo: RepoName,
    pub steps: Vec<PlannedStep>,
}

/// One commit-to-be, fully resolved: subject, expectation, book anchor,
/// the files it touches, the complete tree after it, and the display
/// data (spans + line ranges) for every block it contains.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedStep {
    /// 1-based position in the repo's history.
    pub seq: usize,
    pub id: StepId,
    pub msg: String,
    pub expect: Expect,
    pub anchor: Location,
    pub files: Vec<String>,
    /// The materialized tree after this step — what the replay layer
    /// writes and commits.
    pub tree: TreeState,
    pub displays: Vec<BlockDisplay>,
    /// Live notebook cells bound to this step (§ 15), in document order.
    pub play_cells: Vec<PlayCell>,
}

/// One `notebook="play"` cell: live code the ipynb target renders as an
/// executable cell against this step's state. Never part of any tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayCell {
    pub loc: Location,
    /// The fence info string (`python`, typically).
    pub info: String,
    pub lines: Vec<String>,
}

impl PlannedStep {
    /// The stable tag name the replay layer creates for this step and the
    /// rendered book links to: `step-012-rank-enum`.
    #[must_use]
    pub fn tag(&self) -> String {
        format!("step-{:03}-{}", self.seq, self.id)
    }
}

/// Resolve a book against a repo catalog: extract, group, order, fold,
/// and line-map — one pass, all errors collected.
///
/// Determinism contract: equal inputs produce an equal [`BookPlan`],
/// down to every tree byte and line range.
///
/// # Errors
///
/// Returns every [`crate::BowerError`] found anywhere in the pass; a plan
/// is only produced when there are none.
pub fn plan(book: &BookSource, catalog: &RepoCatalog) -> Result<BookPlan, Errors> {
    let mut errors = Errors::default();

    let (blocks, extract_errors) = block::extract(book, catalog);
    errors.extend(extract_errors);

    let (play_blocks, code_blocks): (Vec<_>, Vec<_>) = blocks.into_iter().partition(|b| b.play);

    let steps = step::group(code_blocks, &mut errors);

    let mut repos = Vec::new();
    for (name, spec) in &catalog.0 {
        let mine: Vec<Step> = steps.iter().filter(|s| &s.repo == name).cloned().collect();
        if mine.is_empty() {
            continue;
        }
        let ordered = step::order(name, mine, &mut errors);

        let mut tree = TreeState::default();
        let mut planned = Vec::new();
        for (i, s) in ordered.iter().enumerate() {
            for b in &s.blocks {
                tree.apply_block(b, &s.id.0, &book.assets, &mut errors);
            }
            let materialized = tree.materialized(spec.keep_region_markers);

            let mut displays = Vec::new();
            for b in &s.blocks {
                if matches!(b.op, Op::Prose | Op::Delete | Op::Copy) {
                    continue;
                }
                let mut d = display::analyze(b, &mut errors);
                display::resolve_ranges(
                    &mut d,
                    &materialized,
                    spec.keep_region_markers,
                    &s.id.0,
                    &mut errors,
                );
                displays.push(d);
            }

            planned.push(PlannedStep {
                seq: i + 1,
                id: s.id.clone(),
                msg: s.msg.clone(),
                expect: s.expect,
                anchor: s.anchor.clone(),
                files: s.files(),
                tree: materialized,
                displays,
                play_cells: Vec::new(),
            });
        }

        bind_play_cells(&play_blocks, name, &ordered, &mut planned, &mut errors);

        repos.push(RepoPlan {
            repo: name.clone(),
            steps: planned,
        });
    }

    if errors.is_empty() {
        Ok(BookPlan { repos })
    } else {
        Err(errors)
    }
}

/// Attach each play block of `repo` to its step: the explicit `step=` when
/// given, else the step containing the nearest preceding code block of the
/// same repo in document order — you play with what you just built.
fn bind_play_cells(
    play_blocks: &[crate::block::Block],
    repo: &RepoName,
    ordered: &[Step],
    planned: &mut [PlannedStep],
    errors: &mut Errors,
) {
    use std::collections::BTreeMap;

    let by_id: BTreeMap<String, usize> = planned
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.0.clone(), i))
        .collect();

    // (document position of a code block) → index of its planned step.
    let mut by_doc_pos: Vec<(usize, usize)> = Vec::new();
    for s in ordered {
        if let Some(&idx) = by_id.get(&s.id.0) {
            for b in &s.blocks {
                by_doc_pos.push((b.seq_in_book, idx));
            }
        }
    }
    by_doc_pos.sort_unstable();

    for p in play_blocks.iter().filter(|p| &p.repo == repo) {
        let target = if let Some(step_id) = &p.step {
            let found = by_id.get(step_id).copied();
            if found.is_none() {
                errors.push(BowerError::PlayCellUnknownStep {
                    loc: p.loc.clone(),
                    step: step_id.clone(),
                });
            }
            found
        } else {
            let preceding = by_doc_pos
                .iter()
                .take_while(|(pos, _)| *pos < p.seq_in_book)
                .last()
                .map(|&(_, idx)| idx);
            if preceding.is_none() {
                errors.push(BowerError::PlayCellUnbound { loc: p.loc.clone() });
            }
            preceding
        };
        if let Some(idx) = target {
            planned[idx].play_cells.push(PlayCell {
                loc: p.loc.clone(),
                info: p.content.info.clone(),
                lines: p.content.lines.clone(),
            });
        }
    }
}

/// Render the plan as `bower.lock` text: the human-readable manifest that
/// makes reordering visible in diffs and code review. Pure string out —
/// the caller writes the file.
#[must_use]
pub fn lock_text(plan: &BookPlan) -> String {
    use std::fmt::Write;

    let mut out = String::from("# bower.lock — generated; review, don't edit\n");
    for repo in &plan.repos {
        let _ = writeln!(out, "\n[{}]", repo.repo);
        for s in &repo.steps {
            let _ = writeln!(
                out,
                "{:03} {} expect={} anchor={} files={}{}",
                s.seq,
                s.id,
                s.expect,
                s.anchor,
                s.files.join(","),
                if s.play_cells.is_empty() {
                    String::new()
                } else {
                    format!(" play={}", s.play_cells.len())
                }
            );
        }
    }
    out
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod plan_tests {
    use super::*;
    use crate::source::Chapter;

    fn catalog() -> RepoCatalog {
        RepoCatalog::from_names(&["failers"])
    }

    fn ranks_chapter() -> Chapter {
        Chapter::new(
            "ch01-ranks.md",
            concat!(
                "# The Rank enum\n\n",
                "<!-- bower repo=\"failers\" file=\"src/rank.rs\" step=\"rank-enum\" -->\n",
                "```rust\n",
                "pub enum Rank { Ace }\n",
                "// bower:begin from_char\n",
                "// bower:end from_char\n",
                "```\n\n",
                "## From char\n\n",
                "<!-- bower repo=\"failers\" file=\"src/rank.rs\" op=\"region\" region=\"from_char\" expect=\"compile_fail\" -->\n",
                "```rust\n",
                "impl From<char> for Rank { }\n",
                "```\n",
            ),
        )
    }

    #[test]
    fn plan__two_steps_fold_and_anchor() {
        let book = BookSource::from_chapters(vec![ranks_chapter()]);
        let p = plan(&book, &catalog()).unwrap();
        let repo = p.repo("failers").unwrap();
        assert_eq!(repo.steps.len(), 2);

        let first = &repo.steps[0];
        assert_eq!(first.tag(), "step-001-rank-enum");
        assert_eq!(
            first.tree.text("src/rank.rs").unwrap(),
            "pub enum Rank { Ace }\n"
        );

        let second = &repo.steps[1];
        assert_eq!(second.expect, Expect::CompileFail);
        assert_eq!(
            second.tree.text("src/rank.rs").unwrap(),
            "pub enum Rank { Ace }\nimpl From<char> for Rank { }\n"
        );
        assert_eq!(second.msg, "ch01-ranks: From char");
    }

    #[test]
    fn plan__is_deterministic() {
        let book = BookSource::from_chapters(vec![ranks_chapter()]);
        let a = plan(&book, &catalog()).unwrap();
        let b = plan(&book, &catalog()).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn plan__line_ranges_land_on_the_region_content() {
        let book = BookSource::from_chapters(vec![ranks_chapter()]);
        let p = plan(&book, &catalog()).unwrap();
        let second = &p.repo("failers").unwrap().steps[1];
        let range = second.displays[0].ranges[0].clone().unwrap();
        assert_eq!(range.file, "src/rank.rs");
        assert_eq!((range.start, range.end), (2, 2));
    }

    #[test]
    fn plan__collects_errors_instead_of_stopping() {
        let book = BookSource::from_chapters(vec![Chapter::new(
            "bad.md",
            concat!(
                "<!-- bower repo=\"nope\" file=\"a.rs\" -->\n```rust\nx\n```\n",
                "<!-- bower repo=\"failers\" op=\"append\" file=\"never.rs\" -->\n```rust\ny\n```\n",
            ),
        )]);
        let errs = plan(&book, &catalog()).unwrap_err();
        assert!(errs.len() >= 2, "{errs}");
    }

    #[test]
    fn lock_text__is_stable_and_readable() {
        let book = BookSource::from_chapters(vec![ranks_chapter()]);
        let p = plan(&book, &catalog()).unwrap();
        let lock = lock_text(&p);
        assert!(lock.contains("[failers]"));
        assert!(lock.contains("001 rank-enum expect=pass anchor=ch01-ranks.md:3 files=src/rank.rs"));
        assert!(lock.contains("002"));
    }
}
