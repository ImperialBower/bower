//! The kernel's front door: [`plan`] takes a book and a catalog and
//! returns everything the replay layer, the preprocessor, and the
//! verifier need — or every error it could find, located.

use std::collections::BTreeMap;

use crate::block::Block;
use crate::branch::{self, BranchSummary, Fold, Line};
use crate::capture;
use crate::directive::{Capture, Expect, Op};
use crate::display::{self, BlockDisplay};
use crate::source::{BookSource, Location, RepoCatalog, RepoName};
use crate::step::{self, Step, StepId};
use crate::tree::TreeState;
use crate::{BowerError, Errors, block};

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
    /// Every branch the repo's steps sit on, in the order each first appears
    /// (EPIC-09). Empty for a straight line.
    pub branches: Vec<BranchSummary>,
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
    /// The "your turn" point on this step, if the book declares one.
    pub exercise: Option<Exercise>,
    /// What the compiler said at this step, as the book records it: every
    /// `output="…"` block bound here, in document order (EPIC-11).
    pub outputs: Vec<CapturedOutput>,
    /// The line this step's commit sits on (EPIC-09).
    pub line: Line,
    /// Parent seqs: `0` is the scaffolding commit. One, or two for a merge —
    /// main's head, then the branch's.
    pub parents: Vec<usize>,
    /// The branch this step merges, when it is a merge.
    pub merges: Option<String>,
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

/// One `output="…"` block, bound to a step. Never part of any tree, so
/// recording one moves no SHA (EPIC-11 Decision 9).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapturedOutput {
    /// The directive: where errors point and where `--record` writes.
    pub loc: Location,
    pub capture: Capture,
    /// The author's fence info string, kept on rewrite.
    pub info: String,
    /// The fence body, tidied (`capture::tidy`). Empty means "not recorded
    /// yet".
    pub lines: Vec<String>,
}

/// One "your turn" point, bound to a step. Never part of any tree. See
/// `docs/superpowers/specs/2026-09-06-exercises-design.md`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Exercise {
    /// The directive that declared it — where errors point.
    pub loc: Location,
    pub form: ExerciseForm,
    /// Where the box renders: the block form's own directive, or the key
    /// form's step's last block in book order.
    pub at: Location,
    /// The `exercise="…"` value: one imperative line.
    pub task: String,
    /// The block form's fenced body, verbatim markdown. Empty for the key
    /// form.
    pub detail: Vec<String>,
    /// The step that carries the answer: the next step of the same repo.
    pub answer: StepId,
}

/// How an exercise was written: a key on a tree block's directive, or its
/// own directive with a fenced body.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExerciseForm {
    Key,
    Block,
}

impl std::fmt::Display for ExerciseForm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Key => "key",
            Self::Block => "block",
        })
    }
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

    let (pr_blocks, rest): (Vec<_>, Vec<_>) = blocks.into_iter().partition(|b| b.pr_block);
    let (play_blocks, rest): (Vec<_>, Vec<_>) = rest.into_iter().partition(|b| b.play);
    let (output_blocks, rest): (Vec<_>, Vec<_>) =
        rest.into_iter().partition(|b| b.output.is_some());
    let (exercise_blocks, code_blocks): (Vec<_>, Vec<_>) =
        rest.into_iter().partition(|b| b.exercise_block);

    let steps = step::group(code_blocks, &mut errors);

    let mut repos = Vec::new();
    for (name, spec) in &catalog.0 {
        let mine: Vec<Step> = steps.iter().filter(|s| &s.repo == name).cloned().collect();
        if mine.is_empty() {
            continue;
        }
        let ordered = step::order(name, mine, &mut errors);

        let mut fold = Fold::new(&book.assets);
        let mut planned = Vec::new();
        for (i, s) in ordered.iter().enumerate() {
            let seq = i + 1;
            let folded = fold.step(seq, s, &mut errors);
            let materialized = folded.tree.materialized(spec.keep_region_markers);

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
                seq,
                id: s.id.clone(),
                msg: s.msg.clone(),
                expect: s.expect,
                anchor: s.anchor.clone(),
                files: s.files(),
                tree: materialized,
                displays,
                play_cells: Vec::new(),
                exercise: None,
                outputs: Vec::new(),
                line: folded.line,
                parents: folded.parents,
                merges: folded.merges,
            });
        }
        let mut branches = fold.finish();
        branch::bind_prs(&mut branches, &ordered, &pr_blocks, name, &mut errors);
        check_branch_refs(&branches, &planned, &mut errors);

        let index = StepIndex::new(&ordered, &planned);
        bind_play_cells(&play_blocks, name, &index, &mut planned, &mut errors);
        bind_exercises(
            &exercise_blocks,
            name,
            &ordered,
            &index,
            &mut planned,
            &mut errors,
        );
        bind_outputs(&output_blocks, name, &index, &mut planned, &mut errors);

        repos.push(RepoPlan {
            repo: name.clone(),
            steps: planned,
            branches,
        });
    }

    if errors.is_empty() {
        Ok(BookPlan { repos })
    } else {
        Err(errors)
    }
}

/// Refuse a repo's branches that would collide once they are git refs
/// (EPIC-09): one a `/`-prefix of another (`refs/heads/a` is a file where
/// `refs/heads/a/b` needs a directory), or two that are equal ignoring ASCII
/// case but not identical (a case-insensitive filesystem resolves both to one
/// ref). `branch_name_problem` already refuses anything under `main/` or
/// spelled like `main`, so this only ever compares branch to branch.
///
/// Reported once per colliding pair, at the branch whose first step comes
/// later in plan order — the earlier branch is the one already there.
fn check_branch_refs(branches: &[BranchSummary], planned: &[PlannedStep], errors: &mut Errors) {
    let first_step = |name: &str| -> Option<&PlannedStep> {
        planned
            .iter()
            .filter(|s| s.line == Line::Branch(name.to_string()))
            .min_by_key(|s| s.seq)
    };
    for (i, a) in branches.iter().enumerate() {
        for b in &branches[..i] {
            let collides = a.name.starts_with(&format!("{}/", b.name))
                || b.name.starts_with(&format!("{}/", a.name))
                || (a.name.eq_ignore_ascii_case(&b.name) && a.name != b.name);
            if !collides {
                continue;
            }
            // The later branch in plan order is the one that collides with
            // what was already there.
            let (later, other) = match (first_step(&a.name), first_step(&b.name)) {
                (Some(sa), Some(sb)) if sb.seq < sa.seq => (a, b),
                (Some(_), Some(_)) => (b, a),
                _ => (a, b),
            };
            let Some(step) = first_step(&later.name) else {
                continue;
            };
            errors.push(BowerError::InvalidBranchName {
                loc: step.anchor.clone(),
                name: later.name.clone(),
                reason: format!("it collides with branch `{}` as a git ref", other.name),
            });
        }
    }
}

/// Where a non-tree block (a play cell, a block-form exercise) attaches: step
/// ids, and the document position of every code block, for the two binding
/// rules of spec § 15.1 — an explicit `step=`, else the nearest preceding
/// code block of the repo.
struct StepIndex {
    by_id: BTreeMap<String, usize>,
    /// (document position of a code block) → index of its planned step,
    /// sorted by position.
    by_doc_pos: Vec<(usize, usize)>,
}

/// Why a non-tree block could not be bound; the caller names the error.
enum Unbindable {
    NoPrecedingStep,
    UnknownStep(String),
}

impl StepIndex {
    fn new(ordered: &[Step], planned: &[PlannedStep]) -> Self {
        let by_id: BTreeMap<String, usize> = planned
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id.0.clone(), i))
            .collect();
        let mut by_doc_pos: Vec<(usize, usize)> = Vec::new();
        for s in ordered {
            if let Some(&idx) = by_id.get(&s.id.0) {
                for b in &s.blocks {
                    by_doc_pos.push((b.seq_in_book, idx));
                }
            }
        }
        by_doc_pos.sort_unstable();
        Self { by_id, by_doc_pos }
    }

    fn locate(&self, explicit: Option<&str>, seq_in_book: usize) -> Result<usize, Unbindable> {
        match explicit {
            Some(id) => self
                .by_id
                .get(id)
                .copied()
                .ok_or_else(|| Unbindable::UnknownStep(id.to_string())),
            None => self
                .by_doc_pos
                .iter()
                .take_while(|(pos, _)| *pos < seq_in_book)
                .last()
                .map(|&(_, idx)| idx)
                .ok_or(Unbindable::NoPrecedingStep),
        }
    }
}

/// Attach each play block of `repo` to its step: the explicit `step=` when
/// given, else the step containing the nearest preceding code block of the
/// same repo in document order — you play with what you just built.
fn bind_play_cells(
    play_blocks: &[Block],
    repo: &RepoName,
    index: &StepIndex,
    planned: &mut [PlannedStep],
    errors: &mut Errors,
) {
    for p in play_blocks.iter().filter(|p| &p.repo == repo) {
        match index.locate(p.step.as_deref(), p.seq_in_book) {
            Ok(idx) => planned[idx].play_cells.push(PlayCell {
                loc: p.loc.clone(),
                info: p.content.info.clone(),
                lines: p.content.lines.clone(),
            }),
            Err(Unbindable::UnknownStep(step)) => errors.push(BowerError::PlayCellUnknownStep {
                loc: p.loc.clone(),
                step,
            }),
            Err(Unbindable::NoPrecedingStep) => {
                errors.push(BowerError::PlayCellUnbound { loc: p.loc.clone() });
            }
        }
    }
}

/// Attach every exercise of `repo` to its step, in document order so a
/// duplicate is reported at the second one. Key forms ride on their own
/// step's blocks; block forms bind exactly as play cells do. The answer is
/// the next step on the exercise's line (EPIC-09 Decision 11), so the last
/// step of a line cannot carry one.
fn bind_exercises(
    exercise_blocks: &[Block],
    repo: &RepoName,
    ordered: &[Step],
    index: &StepIndex,
    planned: &mut [PlannedStep],
    errors: &mut Errors,
) {
    // (document position, declaring block, form, bound step index)
    let mut found: Vec<(usize, &Block, ExerciseForm, Option<usize>)> = Vec::new();

    for (idx, s) in ordered.iter().enumerate() {
        for b in s.blocks.iter().filter(|b| b.exercise.is_some()) {
            found.push((b.seq_in_book, b, ExerciseForm::Key, Some(idx)));
        }
    }
    for b in exercise_blocks.iter().filter(|b| &b.repo == repo) {
        let target = match index.locate(b.step.as_deref(), b.seq_in_book) {
            Ok(idx) => Some(idx),
            Err(Unbindable::UnknownStep(step)) => {
                errors.push(BowerError::ExerciseUnknownStep {
                    loc: b.loc.clone(),
                    step,
                });
                None
            }
            Err(Unbindable::NoPrecedingStep) => {
                errors.push(BowerError::ExerciseUnbound { loc: b.loc.clone() });
                None
            }
        };
        found.push((b.seq_in_book, b, ExerciseForm::Block, target));
    }
    found.sort_by_key(|(pos, ..)| *pos);

    for (_, b, form, target) in found {
        let Some(idx) = target else { continue };
        let step_id = planned[idx].id.0.clone();
        if planned[idx].exercise.is_some() {
            errors.push(BowerError::ExerciseDuplicate {
                loc: b.loc.clone(),
                step: step_id,
            });
            continue;
        }
        let Some(answer) = next_on_line(planned, idx).map(|s| s.id.clone()) else {
            errors.push(BowerError::ExerciseWithoutAnswer {
                loc: b.loc.clone(),
                step: step_id,
            });
            continue;
        };
        let at = match form {
            ExerciseForm::Block => b.loc.clone(),
            ExerciseForm::Key => ordered[idx]
                .blocks
                .iter()
                .max_by_key(|x| x.seq_in_book)
                .map_or_else(|| b.loc.clone(), |x| x.loc.clone()),
        };
        planned[idx].exercise = Some(Exercise {
            loc: b.loc.clone(),
            form,
            at,
            task: b.exercise.clone().unwrap_or_default(),
            detail: match form {
                ExerciseForm::Block => b.content.lines.clone(),
                ExerciseForm::Key => Vec::new(),
            },
            answer,
        });
    }
}

/// The step that answers an exercise on `planned[idx]` (EPIC-09 Decision 11):
/// the next step on the same line — or, for a branch's head, the step that
/// merges it. An unmerged head has none.
fn next_on_line(planned: &[PlannedStep], idx: usize) -> Option<&PlannedStep> {
    let here = planned.get(idx)?;
    planned.get(idx + 1..)?.iter().find(|s| {
        s.line == here.line
            || matches!(&here.line, Line::Branch(b) if s.merges.as_deref() == Some(b.as_str()))
    })
}

/// Attach every output block of `repo` to its step, with the play-cell rule:
/// an explicit `step=`, else the nearest preceding code block. Blocks arrive in
/// document order, so a duplicate is reported at the second one. A capture the
/// verifier would never run at that step is refused here, at plan time: a
/// fence nothing could fill is a typo.
fn bind_outputs(
    output_blocks: &[Block],
    repo: &RepoName,
    index: &StepIndex,
    planned: &mut [PlannedStep],
    errors: &mut Errors,
) {
    for b in output_blocks.iter().filter(|b| &b.repo == repo) {
        let Some(capture) = b.output else { continue };
        let idx = match index.locate(b.step.as_deref(), b.seq_in_book) {
            Ok(idx) => idx,
            Err(Unbindable::UnknownStep(step)) => {
                errors.push(BowerError::OutputUnknownStep {
                    loc: b.loc.clone(),
                    step,
                });
                continue;
            }
            Err(Unbindable::NoPrecedingStep) => {
                errors.push(BowerError::OutputUnbound { loc: b.loc.clone() });
                continue;
            }
        };
        let step = &mut planned[idx];
        if !capture.runs_under(step.expect) {
            errors.push(BowerError::OutputNeverRuns {
                loc: b.loc.clone(),
                step: step.id.0.clone(),
                capture,
                expect: step.expect,
            });
            continue;
        }
        if step.outputs.iter().any(|o| o.capture == capture) {
            errors.push(BowerError::OutputDuplicate {
                loc: b.loc.clone(),
                step: step.id.0.clone(),
                capture,
            });
            continue;
        }
        step.outputs.push(CapturedOutput {
            loc: b.loc.clone(),
            capture,
            info: b.content.info.clone(),
            lines: capture::tidy(&b.content.lines),
        });
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
                "{:03} {} expect={} anchor={} files={}{}{}",
                s.seq,
                s.id,
                s.expect,
                s.anchor,
                s.files.join(","),
                if s.play_cells.is_empty() {
                    String::new()
                } else {
                    format!(" play={}", s.play_cells.len())
                },
                line_suffix(s)
            );
            if let Some(x) = &s.exercise {
                let _ = writeln!(
                    out,
                    "    exercise form={} answer={} task={:?}",
                    x.form, x.answer, x.task
                );
                for line in &x.detail {
                    let _ = writeln!(out, "    | {line}");
                }
            }
            // Every recorded line, so a rewording shows up in review as a
            // diff of the lock as well as of the chapter.
            for o in &s.outputs {
                let _ = writeln!(out, "    output {} at={}", o.capture, o.loc);
                for line in &o.lines {
                    let _ = writeln!(out, "    > {line}");
                }
            }
        }
        // Only for a repo that has one: a straight line's lock gains nothing.
        if !repo.branches.is_empty() {
            let _ = writeln!(out, "\n[{}.branches]", repo.repo);
            for b in &repo.branches {
                let merged = b
                    .merged_at
                    .map_or_else(|| "no".to_string(), |m| format!("{m:03}"));
                let pr = b.pr.as_ref().map_or_else(String::new, |p| {
                    format!(" pr={:?} state={}", p.title, p.state)
                });
                let _ = writeln!(
                    out,
                    "{} from={:03} head={:03} merged={merged}{pr}",
                    b.name, b.forked_from, b.head
                );
                // The description, line by line, so a rewording shows up in
                // review as a diff of the lock, as an exercise's detail does.
                for line in b.pr.iter().flat_map(|p| &p.body) {
                    let _ = writeln!(out, "    | {line}");
                }
            }
        }
    }
    out
}

/// A step's line in the lock, where it has one to state (EPIC-09 Decision 13):
/// a branch step's line and parent, a merge's parents and branch, and nothing
/// for a main step — so a straight line's lock is what it always was.
fn line_suffix(s: &PlannedStep) -> String {
    let parents = s
        .parents
        .iter()
        .map(|p| format!("{p:03}"))
        .collect::<Vec<_>>()
        .join(",");
    match (&s.line, &s.merges) {
        (Line::Branch(b), _) => format!(" line={b} parents={parents}"),
        (Line::Main, Some(m)) => format!(" parents={parents} merges={m}"),
        (Line::Main, None) => String::new(),
    }
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
        assert!(
            lock.contains("001 rank-enum expect=pass anchor=ch01-ranks.md:3 files=src/rank.rs")
        );
        assert!(lock.contains("002"));
    }

    #[test]
    fn plan__lock_text_of_a_linear_book_is_unchanged() {
        // Byte for byte what the lock said before branches existed (EPIC-09
        // Decision 13): no `line=`, no `parents=`, no branch table.
        let book = BookSource::from_chapters(vec![ranks_chapter()]);
        assert_eq!(
            lock_text(&plan(&book, &catalog()).unwrap()),
            concat!(
                "# bower.lock — generated; review, don't edit\n",
                "\n[failers]\n",
                "001 rank-enum expect=pass anchor=ch01-ranks.md:3 files=src/rank.rs\n",
                "002 ch01-ranks-from-char expect=compile_fail anchor=ch01-ranks.md:12 files=src/rank.rs\n",
            )
        );
    }

    fn exercise_chapter() -> Chapter {
        Chapter::new(
            "ch02-exercise.md",
            concat!(
                "# Broken\n\n",
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" step=\"broken\" expect=\"compile_fail\" exercise=\"Make this compile\" -->\n",
                "```rust\nfn x() -> u32 { \"42\" }\n```\n\n",
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" op=\"replace\" step=\"fixed\" -->\n",
                "```rust\nfn x() -> u32 { 42 }\n```\n\n",
                "<!-- bower repo=\"failers\" exercise=\"Do it without a literal\" -->\n",
                "```markdown\nParse it instead.\n\n- `str::parse` is one way.\n```\n\n",
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" op=\"append\" step=\"last\" -->\n",
                "```rust\n// the end\n```\n",
            ),
        )
    }

    #[test]
    fn exercise__key_form_binds_to_its_own_step_and_the_next_is_the_answer() {
        let book = BookSource::from_chapters(vec![exercise_chapter()]);
        let p = plan(&book, &catalog()).unwrap();
        let steps = &p.repo("failers").unwrap().steps;
        let x = steps[0].exercise.as_ref().unwrap();
        assert_eq!(x.form, ExerciseForm::Key);
        assert_eq!(x.task, "Make this compile");
        assert!(x.detail.is_empty());
        assert_eq!(x.answer, StepId("fixed".to_string()));
        assert_eq!(x.loc, Location::new("ch02-exercise.md", 3));
        assert_eq!(x.at, Location::new("ch02-exercise.md", 3));
    }

    #[test]
    fn exercise__block_form_binds_to_the_nearest_preceding_step_with_its_detail() {
        let book = BookSource::from_chapters(vec![exercise_chapter()]);
        let p = plan(&book, &catalog()).unwrap();
        let steps = &p.repo("failers").unwrap().steps;
        let x = steps[1].exercise.as_ref().unwrap();
        assert_eq!(x.form, ExerciseForm::Block);
        assert_eq!(x.task, "Do it without a literal");
        assert_eq!(x.at, Location::new("ch02-exercise.md", 13));
        assert_eq!(
            x.detail,
            vec![
                "Parse it instead.".to_string(),
                String::new(),
                "- `str::parse` is one way.".to_string(),
            ]
        );
        assert_eq!(x.answer, StepId("last".to_string()));
        assert!(steps[2].exercise.is_none());
        // The block form's fence never reaches a tree.
        assert_eq!(
            steps[2].tree.text("src/lib.rs").unwrap(),
            "fn x() -> u32 { 42 }\n// the end\n"
        );
    }

    #[test]
    fn exercise__key_form_on_a_multi_block_step_renders_after_the_last_block() {
        let book = BookSource::from_chapters(vec![Chapter::new(
            "ch.md",
            concat!(
                "<!-- bower repo=\"failers\" file=\"a.rs\" step=\"two\" exercise=\"Try\" -->\n```rust\nx\n```\n",
                "<!-- bower repo=\"failers\" file=\"b.rs\" step=\"two\" -->\n```rust\ny\n```\n",
                "<!-- bower repo=\"failers\" file=\"c.rs\" step=\"next\" -->\n```rust\nz\n```\n",
            ),
        )]);
        let p = plan(&book, &catalog()).unwrap();
        let x = p.repo("failers").unwrap().steps[0]
            .exercise
            .as_ref()
            .unwrap();
        assert_eq!(x.loc.line, 1);
        assert_eq!(x.at.line, 5);
    }

    #[test]
    fn exercise__duplicate_is_reported_at_the_second_one() {
        let book = BookSource::from_chapters(vec![Chapter::new(
            "ch.md",
            concat!(
                "<!-- bower repo=\"failers\" file=\"a.rs\" step=\"one\" exercise=\"First\" -->\n```rust\nx\n```\n",
                "<!-- bower repo=\"failers\" exercise=\"Second\" -->\n```markdown\nmore\n```\n",
                "<!-- bower repo=\"failers\" file=\"b.rs\" step=\"two\" -->\n```rust\ny\n```\n",
            ),
        )]);
        let errs = plan(&book, &catalog()).unwrap_err();
        assert_eq!(errs.len(), 1, "{errs}");
        assert!(
            matches!(&errs.0[0], BowerError::ExerciseDuplicate { loc, step } if loc.line == 5 && step == "one"),
            "{errs}"
        );
    }

    #[test]
    fn exercise__on_the_last_step_has_no_answer() {
        let book = BookSource::from_chapters(vec![Chapter::new(
            "ch.md",
            "<!-- bower repo=\"failers\" file=\"a.rs\" step=\"only\" exercise=\"Try\" -->\n```rust\nx\n```\n",
        )]);
        let errs = plan(&book, &catalog()).unwrap_err();
        assert!(
            matches!(&errs.0[0], BowerError::ExerciseWithoutAnswer { step, .. } if step == "only"),
            "{errs}"
        );
    }

    #[test]
    fn exercise__block_form_errors_mirror_play_cells() {
        let unbound = BookSource::from_chapters(vec![Chapter::new(
            "ch.md",
            "<!-- bower repo=\"failers\" exercise=\"Try\" -->\n```markdown\nx\n```\n<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n",
        )]);
        let errs = plan(&unbound, &catalog()).unwrap_err();
        assert!(
            matches!(errs.0[0], BowerError::ExerciseUnbound { .. }),
            "{errs}"
        );

        let ghost = BookSource::from_chapters(vec![Chapter::new(
            "ch.md",
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n<!-- bower repo=\"failers\" exercise=\"Try\" step=\"ghost\" -->\n```markdown\nx\n```\n",
        )]);
        let errs = plan(&ghost, &catalog()).unwrap_err();
        assert!(
            matches!(&errs.0[0], BowerError::ExerciseUnknownStep { step, .. } if step == "ghost"),
            "{errs}"
        );
    }

    #[test]
    fn lock_text__records_the_exercise_and_its_detail() {
        let book = BookSource::from_chapters(vec![exercise_chapter()]);
        let lock = lock_text(&plan(&book, &catalog()).unwrap());
        assert!(
            lock.contains("    exercise form=key answer=fixed task=\"Make this compile\"\n"),
            "{lock}"
        );
        assert!(
            lock.contains("    exercise form=block answer=last task=\"Do it without a literal\"\n"),
            "{lock}"
        );
        assert!(
            lock.contains("    | Parse it instead.\n    | \n    | - `str::parse` is one way.\n"),
            "{lock}"
        );
    }

    fn one(text: &str) -> BookSource {
        BookSource::from_chapters(vec![Chapter::new("ch.md", text)])
    }

    fn output_chapter() -> Chapter {
        Chapter::new(
            "ch03-outputs.md",
            concat!(
                "# Outputs\n\n", // 1-2
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" step=\"broken\" expect=\"compile_fail\" -->\n", // 3
                "```rust\nfn x() -> u32 { \"42\" }\n```\n\n", // 4-7
                "<!-- bower repo=\"failers\" output=\"check\" -->\n", // 8
                "```text\nerror[E0308]: mismatched types   \n[...]\n\n```\n\n", // 9-14
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" op=\"replace\" step=\"fixed\" -->\n", // 15
                "```rust\nfn x() -> u32 { 42 }\n```\n\n", // 16-19
                "<!-- bower repo=\"failers\" output=\"check\" step=\"fixed\" -->\n", // 20
                "```text\n```\n",                         // 21-22
            ),
        )
    }

    fn output_plan() -> BookPlan {
        plan(
            &BookSource::from_chapters(vec![output_chapter()]),
            &catalog(),
        )
        .unwrap()
    }

    #[test]
    fn plan__output_binds_to_the_nearest_preceding_step() {
        let p = output_plan();
        let o = &p.repo("failers").unwrap().steps[0].outputs[0];
        assert_eq!(o.capture, Capture::Check);
        assert_eq!(o.loc, Location::new("ch03-outputs.md", 8));
        assert_eq!(o.info, "text");
        // Tidied: the trailing spaces and the trailing blank line are gone.
        assert_eq!(
            o.lines,
            vec![
                "error[E0308]: mismatched types".to_string(),
                "[...]".to_string()
            ]
        );
    }

    #[test]
    fn plan__output_binds_explicitly_by_step() {
        let p = output_plan();
        let fixed = &p.repo("failers").unwrap().steps[1];
        assert_eq!(fixed.outputs.len(), 1);
        assert_eq!(fixed.outputs[0].loc.line, 20);
        assert!(
            fixed.outputs[0].lines.is_empty(),
            "an empty fence is not recorded yet"
        );
    }

    #[test]
    fn plan__output_never_touches_a_tree() {
        let p = output_plan();
        let steps = &p.repo("failers").unwrap().steps;
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].files, vec!["src/lib.rs".to_string()]);
        assert_eq!(
            steps[0].tree.text("src/lib.rs").unwrap(),
            "fn x() -> u32 { \"42\" }\n"
        );
    }

    #[test]
    fn plan__verify_output_on_compile_fail_never_runs() {
        let book = one(concat!(
            "<!-- bower repo=\"failers\" file=\"a.rs\" expect=\"compile_fail\" -->\n```rust\nx\n```\n",
            "<!-- bower repo=\"failers\" output=\"verify\" -->\n```text\n```\n",
        ));
        let errs = plan(&book, &catalog()).unwrap_err();
        assert!(
            matches!(
                &errs.0[0],
                BowerError::OutputNeverRuns { loc, capture: Capture::Verify, expect: Expect::CompileFail, .. }
                    if loc.line == 5
            ),
            "{errs}"
        );
    }

    #[test]
    fn plan__output_on_a_none_step_never_runs() {
        let book = one(concat!(
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n",
            "<!-- bower repo=\"failers\" op=\"none\" step=\"talk\" expect=\"none\" msg=\"m\" -->\n",
            "<!-- bower repo=\"failers\" output=\"check\" -->\n```text\n```\n",
        ));
        let errs = plan(&book, &catalog()).unwrap_err();
        assert!(
            matches!(
                &errs.0[0],
                BowerError::OutputNeverRuns { step, capture: Capture::Check, expect: Expect::Skip, .. }
                    if step == "talk"
            ),
            "{errs}"
        );
    }

    #[test]
    fn plan__duplicate_capture_is_reported_at_the_second() {
        let book = one(concat!(
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n",
            "<!-- bower repo=\"failers\" output=\"check\" -->\n```text\n```\n",
            "<!-- bower repo=\"failers\" output=\"check\" -->\n```text\n```\n",
        ));
        let errs = plan(&book, &catalog()).unwrap_err();
        assert_eq!(errs.len(), 1, "{errs}");
        assert!(
            matches!(&errs.0[0], BowerError::OutputDuplicate { loc, capture: Capture::Check, .. } if loc.line == 8),
            "{errs}"
        );
    }

    #[test]
    fn plan__output_binding_errors_mirror_play_cells() {
        let unbound = one(concat!(
            "<!-- bower repo=\"failers\" output=\"check\" -->\n```text\n```\n",
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n",
        ));
        let errs = plan(&unbound, &catalog()).unwrap_err();
        assert!(
            matches!(errs.0[0], BowerError::OutputUnbound { .. }),
            "{errs}"
        );

        let ghost = one(concat!(
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n",
            "<!-- bower repo=\"failers\" output=\"check\" step=\"ghost\" -->\n```text\n```\n",
        ));
        let errs = plan(&ghost, &catalog()).unwrap_err();
        assert!(
            matches!(&errs.0[0], BowerError::OutputUnknownStep { step, .. } if step == "ghost"),
            "{errs}"
        );
    }

    #[test]
    fn lock_text__records_outputs_line_by_line() {
        let lock = lock_text(&output_plan());
        assert!(
            lock.contains(concat!(
                "    output check at=ch03-outputs.md:8\n",
                "    > error[E0308]: mismatched types\n",
                "    > [...]\n",
            )),
            "{lock}"
        );
        assert!(
            lock.contains("    output check at=ch03-outputs.md:20\n"),
            "{lock}"
        );
    }
}
