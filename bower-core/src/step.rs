//! Step grouping and ordering: blocks sharing a `step` id merge into one
//! commit-to-be; steps are sequenced in document order, bent only by
//! explicit `after=` constraints, with cycles rejected.

use std::collections::{BTreeMap, BTreeSet};

use crate::block::Block;
use crate::directive::Expect;
use crate::source::{Location, RepoName};
use crate::{BowerError, Errors};

/// A step's identity — the stable half of every generated tag name
/// (`step-012-rank-enum`), so authors should prefer explicit ids for steps
/// the book links to. Auto-generated ids are readable but shift when
/// content moves.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StepId(pub String);

impl std::fmt::Display for StepId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// One commit-to-be: a group of blocks applied together, with a subject,
/// an expectation, and ordering constraints.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Step {
    pub id: StepId,
    pub repo: RepoName,
    pub blocks: Vec<Block>,
    pub msg: String,
    pub expect: Expect,
    pub afters: Vec<String>,
    /// The book position of the step's first block — the anchor commits
    /// point back to.
    pub anchor: Location,
    /// Document-order index of the step's first block, the ordering tiebreak.
    pub doc_order: usize,
}

impl Step {
    /// Every file this step touches, deduplicated, in first-touch order.
    #[must_use]
    pub fn files(&self) -> Vec<String> {
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        for b in &self.blocks {
            for f in b.file.iter().chain(b.paths.iter()) {
                if seen.insert(f.clone()) {
                    out.push(f.clone());
                }
            }
        }
        out
    }
}

/// Group blocks into steps, preserving document order of first appearance.
/// Reports conflicting repos/expects within a shared step id and duplicate
/// file writes within a step.
#[must_use]
pub fn group(blocks: Vec<Block>, errors: &mut Errors) -> Vec<Step> {
    let mut steps: Vec<Step> = Vec::new();
    // (repo, explicit id) → index into `steps`
    let mut by_key: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut used_ids: BTreeSet<(String, String)> = BTreeSet::new();

    for block in blocks {
        if let Some(explicit) = block.step.clone() {
            let key = (block.repo.0.clone(), explicit.clone());
            if let Some(&idx) = by_key.get(&key) {
                merge_into(&mut steps[idx], block, errors);
            } else {
                used_ids.insert(key.clone());
                by_key.insert(key, steps.len());
                steps.push(new_step(StepId(explicit), block));
            }
        } else {
            let id = auto_id(&block, &mut used_ids);
            steps.push(new_step(id, block));
        }
    }

    for step in &steps {
        check_duplicate_files(step, errors);
    }
    steps
}

fn new_step(id: StepId, block: Block) -> Step {
    Step {
        id,
        repo: block.repo.clone(),
        msg: derive_msg(&block),
        expect: block.expect.unwrap_or_default(),
        afters: block.after.clone().into_iter().collect(),
        anchor: block.loc.clone(),
        doc_order: block.seq_in_book,
        blocks: vec![block],
    }
}

fn merge_into(step: &mut Step, block: Block, errors: &mut Errors) {
    if block.repo != step.repo {
        errors.push(BowerError::ConflictingRepoInStep {
            loc: block.loc.clone(),
            step: step.id.0.clone(),
        });
        return;
    }
    if let Some(e) = block.expect {
        if step
            .blocks
            .iter()
            .any(|b| b.expect.is_some_and(|prev| prev != e))
        {
            errors.push(BowerError::ConflictingExpectInStep {
                loc: block.loc.clone(),
                step: step.id.0.clone(),
            });
        } else {
            step.expect = e;
        }
    }
    if step.blocks.iter().all(|b| b.msg.is_none())
        && let Some(m) = &block.msg
    {
        step.msg.clone_from(m);
    }
    if let Some(a) = &block.after
        && !step.afters.contains(a)
    {
        step.afters.push(a.clone());
    }
    step.blocks.push(block);
}

fn derive_msg(block: &Block) -> String {
    if let Some(m) = &block.msg {
        return m.clone();
    }
    match &block.heading {
        Some(h) => format!("{}: {h}", block.chapter_stem),
        None => match &block.file {
            Some(f) => format!("{}: {f}", block.chapter_stem),
            None => format!("{}: step", block.chapter_stem),
        },
    }
}

fn auto_id(block: &Block, used: &mut BTreeSet<(String, String)>) -> StepId {
    let base = block
        .msg
        .as_deref()
        .or(block.heading.as_deref())
        .or(block.file.as_deref())
        .unwrap_or("step");
    let slug = format!("{}-{}", slugify(&block.chapter_stem), slugify(base));
    let repo = block.repo.0.clone();
    if used.insert((repo.clone(), slug.clone())) {
        return StepId(slug);
    }
    let mut n = 2_usize;
    loop {
        let candidate = format!("{slug}-{n}");
        if used.insert((repo.clone(), candidate.clone())) {
            return StepId(candidate);
        }
        n += 1;
    }
}

fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_dash = true;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "step".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Two blocks in one step may touch the same file only when the touches
/// compose: distinct-region ops and appends. Whole-file writes (create,
/// replace, delete, copy) never share a file with anything else in the
/// same step, and two region ops on the same region are a conflict.
fn check_duplicate_files(step: &Step, errors: &mut Errors) {
    use crate::directive::Op;

    let mut touches: BTreeMap<&String, Vec<(&Block, Op)>> = BTreeMap::new();
    for b in &step.blocks {
        for f in b.file.iter().chain(b.paths.iter()) {
            touches.entry(f).or_default().push((b, b.op));
        }
    }
    for (file, list) in touches {
        if list.len() < 2 {
            continue;
        }
        let whole_file = list
            .iter()
            .any(|(_, op)| matches!(op, Op::Create | Op::Replace | Op::Delete | Op::Copy));
        let mut regions: BTreeSet<&str> = BTreeSet::new();
        let region_clash = list.iter().any(|(b, op)| {
            *op == Op::Region && b.region.as_deref().is_some_and(|r| !regions.insert(r))
        });
        if whole_file || region_clash {
            let offender = list[1].0;
            errors.push(BowerError::DuplicateFileInStep {
                loc: offender.loc.clone(),
                step: step.id.0.clone(),
                file: file.clone(),
            });
        }
    }
}

/// Order one repo's steps: document order, bent by `after=` constraints.
/// A stable topological sort — among ready steps, the one earliest in the
/// book goes first, so adding one `after` never reshuffles the rest.
#[must_use]
pub fn order(repo: &RepoName, mut steps: Vec<Step>, errors: &mut Errors) -> Vec<Step> {
    steps.sort_by_key(|s| s.doc_order);
    let index: BTreeMap<String, usize> = steps
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.0.clone(), i))
        .collect();

    let mut blocked_by: Vec<Vec<usize>> = vec![Vec::new(); steps.len()];
    for (i, step) in steps.iter().enumerate() {
        for after in &step.afters {
            match index.get(after) {
                Some(&j) => blocked_by[i].push(j),
                None => errors.push(BowerError::OrphanAfter {
                    loc: step.anchor.clone(),
                    step: step.id.0.clone(),
                    after: after.clone(),
                }),
            }
        }
    }

    let mut placed = vec![false; steps.len()];
    let mut out_indices = Vec::with_capacity(steps.len());
    while out_indices.len() < steps.len() {
        let next =
            (0..steps.len()).find(|&i| !placed[i] && blocked_by[i].iter().all(|&j| placed[j]));
        if let Some(i) = next {
            placed[i] = true;
            out_indices.push(i);
        } else {
            let stuck: Vec<String> = (0..steps.len())
                .filter(|&i| !placed[i])
                .map(|i| steps[i].id.0.clone())
                .collect();
            errors.push(BowerError::OrderingCycle {
                repo: repo.0.clone(),
                steps: stuck,
            });
            return Vec::new();
        }
    }

    let mut by_index: BTreeMap<usize, Step> = steps.into_iter().enumerate().collect();
    out_indices
        .into_iter()
        .filter_map(|i| by_index.remove(&i))
        .collect()
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod step_tests {
    use super::*;
    use crate::block::BlockContent;
    use crate::directive::Op;

    fn block(seq: usize, step: Option<&str>, file: &str) -> Block {
        Block {
            loc: Location::new("ch01.md", seq + 1),
            repo: RepoName::new("failers"),
            op: Op::Create,
            expect: None,
            hidden: true,
            file: Some(file.to_string()),
            paths: vec![],
            region: None,
            src: None,
            show: None,
            step: step.map(ToString::to_string),
            msg: None,
            after: None,
            heading: Some("Ranks".to_string()),
            chapter_stem: "ch01".to_string(),
            content: BlockContent::default(),
            seq_in_book: seq,
            play: false,
            exercise: None,
            exercise_block: false,
            output: None,
        }
    }

    #[test]
    fn group__explicit_ids_merge_autos_stand_alone() {
        let mut errors = Errors::default();
        let steps = group(
            vec![
                block(0, Some("rank"), "a.rs"),
                block(1, None, "b.rs"),
                block(2, Some("rank"), "c.rs"),
            ],
            &mut errors,
        );
        assert!(errors.is_empty(), "{errors}");
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].blocks.len(), 2);
        assert_eq!(
            steps[0].files(),
            vec!["a.rs".to_string(), "c.rs".to_string()]
        );
    }

    #[test]
    fn group__msg_derived_from_heading() {
        let mut errors = Errors::default();
        let steps = group(vec![block(0, None, "a.rs")], &mut errors);
        assert_eq!(steps[0].msg, "ch01: Ranks");
    }

    #[test]
    fn group__duplicate_file_in_step_is_an_error() {
        let mut errors = Errors::default();
        let _ = group(
            vec![block(0, Some("s"), "a.rs"), block(1, Some("s"), "a.rs")],
            &mut errors,
        );
        assert!(matches!(
            errors.0[0],
            BowerError::DuplicateFileInStep { .. }
        ));
    }

    #[test]
    fn group__auto_ids_deduplicate() {
        let mut errors = Errors::default();
        let steps = group(
            vec![block(0, None, "a.rs"), block(1, None, "b.rs")],
            &mut errors,
        );
        assert_eq!(steps[0].id.0, "ch01-ranks");
        assert_eq!(steps[1].id.0, "ch01-ranks-2");
    }

    #[test]
    fn order__default_is_document_order() {
        let mut errors = Errors::default();
        let steps = group(
            vec![block(0, None, "a.rs"), block(1, None, "b.rs")],
            &mut errors,
        );
        let ordered = order(&RepoName::new("failers"), steps, &mut errors);
        assert!(errors.is_empty(), "{errors}");
        assert_eq!(ordered[0].doc_order, 0);
        assert_eq!(ordered[1].doc_order, 1);
    }

    #[test]
    fn order__after_pulls_a_step_later() {
        let mut errors = Errors::default();
        let mut early = block(0, Some("early"), "a.rs");
        early.after = Some("late".to_string());
        let late = block(1, Some("late"), "b.rs");
        let steps = group(vec![early, late], &mut errors);
        let ordered = order(&RepoName::new("failers"), steps, &mut errors);
        assert!(errors.is_empty(), "{errors}");
        assert_eq!(ordered[0].id.0, "late");
        assert_eq!(ordered[1].id.0, "early");
    }

    #[test]
    fn order__cycle_is_an_error() {
        let mut errors = Errors::default();
        let mut a = block(0, Some("a"), "a.rs");
        a.after = Some("b".to_string());
        let mut b = block(1, Some("b"), "b.rs");
        b.after = Some("a".to_string());
        let steps = group(vec![a, b], &mut errors);
        let _ = order(&RepoName::new("failers"), steps, &mut errors);
        assert!(
            errors
                .0
                .iter()
                .any(|e| matches!(e, BowerError::OrderingCycle { .. }))
        );
    }

    #[test]
    fn order__orphan_after_is_an_error() {
        let mut errors = Errors::default();
        let mut a = block(0, Some("a"), "a.rs");
        a.after = Some("ghost".to_string());
        let steps = group(vec![a], &mut errors);
        let _ = order(&RepoName::new("failers"), steps, &mut errors);
        assert!(matches!(errors.0[0], BowerError::OrphanAfter { .. }));
    }
}
