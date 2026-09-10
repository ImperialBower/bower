//! Spike: branches, merges, and pull requests in a Bower plan.
//!
//! Scope of the spike (mirrors `bower-core` shapes, not its parser):
//!
//! - a `Step` as `step::group` + `step::order` would hand it over, carrying
//!   the four proposed keys — `branch=`, `from=`, `merge=`, `pr=`;
//! - the **fold**: one tree per line (main + every branch), merges computed as
//!   *the branch's blocks re-applied over main*, conflicts as plan-time errors;
//! - the **replay**: every step becomes a commit whose parents are plan values,
//!   so a merge is a two-parent commit and a branch is a ref — deterministic,
//!   byte-identical SHAs across runs.
//!
//! Deliberately absent: markdown parsing, regions, display markers, the forge.
//! Those are `bower-core` / `bower` concerns the EPIC maps onto real symbols.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Create,
    Replace,
    Append,
    Delete,
    /// `op="none"` — a step that carries only a message (or only a merge).
    Prose,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expect {
    Pass,
    CompileFail,
    TestFail,
    None,
}

impl fmt::Display for Expect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Expect::Pass => "pass",
            Expect::CompileFail => "compile_fail",
            Expect::TestFail => "test_fail",
            Expect::None => "none",
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub file: Option<String>,
    pub op: Op,
    pub lines: Vec<String>,
}

/// `pr="Title"` on a branch step, with an optional fenced markdown body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pr {
    pub title: String,
    pub body: Vec<String>,
}

/// One ordered step, as the kernel's grouping + ordering would produce it.
///
/// `branch`, `from`, `merge`, and `pr` are the four new directive keys.
#[derive(Clone, Debug)]
pub struct Step {
    pub id: String,
    pub msg: String,
    pub expect: Expect,
    pub blocks: Vec<Block>,
    /// `branch="name"` — this step lives on that branch, not on main.
    pub branch: Option<String>,
    /// `from="step-id"` — fork the branch at that main step instead of the
    /// nearest preceding one. Only meaningful on a branch's first step.
    pub from: Option<String>,
    /// `merge="name"` — this step is a merge of that branch into main. Any
    /// blocks it carries are applied *after* the merge, as the resolution.
    pub merge: Option<String>,
    /// `pr="Title"` — declare a pull request for this step's branch.
    pub pr: Option<Pr>,
}

/// Which line of history a planned step sits on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Line {
    Main,
    Branch(String),
}

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Line::Main => f.write_str("main"),
            Line::Branch(b) => f.write_str(b),
        }
    }
}

/// The whole file tree after a step. Same shape as `bower_core::TreeState`
/// (text-only here).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TreeState(pub BTreeMap<String, String>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// A step named a branch that already has a merge step behind it.
    BranchAlreadyMerged { step: String, branch: String, merged_at: String },
    /// `merge=` names a branch no earlier step created.
    MergeUnknownBranch { step: String, branch: String },
    /// A merge step also declared `branch=`; v1 merges into main only.
    MergeOnBranch { step: String },
    /// `from=` names a step that does not exist.
    UnknownFrom { step: String, from: String },
    /// `from=` names a step that is not on main.
    FromNotOnMain { step: String, from: String },
    /// Both sides touched `file` since the fork and the merge step carries no
    /// whole-file resolution for it.
    MergeConflict { step: String, branch: String, file: String },
    /// A block op that the current tree refuses (replace of a missing file, …).
    Apply { step: String, file: String, reason: &'static str },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BranchAlreadyMerged { step, branch, merged_at } => write!(
                f,
                "step `{step}`: branch `{branch}` was already merged at step `{merged_at}`"
            ),
            Error::MergeUnknownBranch { step, branch } => {
                write!(f, "step `{step}`: merge of unknown branch `{branch}`")
            }
            Error::MergeOnBranch { step } => {
                write!(f, "step `{step}`: a merge step cannot itself be on a branch")
            }
            Error::UnknownFrom { step, from } => {
                write!(f, "step `{step}`: from=\"{from}\" names no step")
            }
            Error::FromNotOnMain { step, from } => {
                write!(f, "step `{step}`: from=\"{from}\" is not a main step")
            }
            Error::MergeConflict { step, branch, file } => write!(
                f,
                "step `{step}`: merging `{branch}` conflicts on `{file}` — main changed it since the fork; add a whole-file resolution block to the merge step"
            ),
            Error::Apply { step, file, reason } => {
                write!(f, "step `{step}`: `{file}`: {reason}")
            }
        }
    }
}

impl TreeState {
    fn apply(&mut self, step: &str, b: &Block, errors: &mut Vec<Error>) {
        let file = match &b.file {
            Some(f) => f.clone(),
            None => return,
        };
        let text = b.lines.join("\n") + "\n";
        let err = |reason| Error::Apply {
            step: step.to_string(),
            file: file.clone(),
            reason,
        };
        match b.op {
            Op::Create => {
                if self.0.contains_key(&file) {
                    errors.push(err("create of a file that already exists"));
                } else {
                    self.0.insert(file.clone(), text);
                }
            }
            Op::Replace => {
                if self.0.contains_key(&file) {
                    self.0.insert(file.clone(), text);
                } else {
                    errors.push(err("replace of a file that does not exist"));
                }
            }
            Op::Append => match self.0.get_mut(&file) {
                Some(body) => body.push_str(&text),
                None => errors.push(err("append to a file that does not exist")),
            },
            Op::Delete => {
                if self.0.remove(&file).is_none() {
                    errors.push(err("delete of a file that does not exist"));
                }
            }
            Op::Prose => {}
        }
    }
}

/// A step after planning: its line, its parents, its complete tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedStep {
    pub seq: usize,
    pub id: String,
    pub msg: String,
    pub expect: Expect,
    pub line: Line,
    /// Parent step seqs; `0` is the scaffolding root. One parent normally,
    /// two for a merge (`[main_head, branch_head]`).
    pub parents: Vec<usize>,
    /// `Some(branch)` when this step is the merge of that branch.
    pub merges: Option<String>,
    pub tree: TreeState,
}

impl PlannedStep {
    pub fn tag(&self) -> String {
        format!("step-{:03}-{}", self.seq, self.id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrState {
    Open,
    Merged,
}

/// Everything the forge needs to make a pull request exist — and nothing it
/// would have to invent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PullRequest {
    pub branch: String,
    pub title: String,
    pub body: Vec<String>,
    pub state: PrState,
}

/// One branch of the plan, summarised for the lock file and the forge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchSummary {
    pub name: String,
    pub forked_from: usize,
    pub head: usize,
    pub merged_at: Option<usize>,
    pub pr: Option<PullRequest>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoPlan {
    pub steps: Vec<PlannedStep>,
    pub branches: Vec<BranchSummary>,
}

struct BranchState {
    forked_from: usize,
    head: usize,
    tree: TreeState,
    /// Every block applied on the branch since the fork, in order — what a
    /// merge replays over main.
    blocks: Vec<Block>,
    touched: BTreeSet<String>,
    merged_at: Option<usize>,
    pr: Option<Pr>,
}

fn files_of(blocks: &[Block]) -> impl Iterator<Item = &String> {
    blocks.iter().filter_map(|b| b.file.as_ref())
}

/// The fold. Pure: ordered steps in, plan or errors out.
pub fn plan(steps: &[Step]) -> Result<RepoPlan, Vec<Error>> {
    let mut errors = Vec::new();
    let mut planned: Vec<PlannedStep> = Vec::new();
    let mut main_tree = TreeState::default();
    let mut main_head = 0_usize;
    // (seq of the main step, file) — what main has changed, and when.
    let mut main_touches: Vec<(usize, String)> = Vec::new();
    let mut branches: BTreeMap<String, BranchState> = BTreeMap::new();
    let mut branch_order: Vec<String> = Vec::new();

    for (i, step) in steps.iter().enumerate() {
        let seq = i + 1;

        if let Some(target) = &step.merge {
            if step.branch.is_some() {
                errors.push(Error::MergeOnBranch { step: step.id.clone() });
                continue;
            }
            let b = match branches.get_mut(target) {
                Some(b) => b,
                None => {
                    errors.push(Error::MergeUnknownBranch {
                        step: step.id.clone(),
                        branch: target.clone(),
                    });
                    continue;
                }
            };
            if let Some(at) = b.merged_at {
                errors.push(Error::BranchAlreadyMerged {
                    step: step.id.clone(),
                    branch: target.clone(),
                    merged_at: planned[at - 1].id.clone(),
                });
                continue;
            }
            // Conflict rule: a file both sides changed since the fork is a
            // conflict unless the merge step replaces it outright.
            let resolved: BTreeSet<&String> = step
                .blocks
                .iter()
                .filter(|x| matches!(x.op, Op::Create | Op::Replace | Op::Delete))
                .filter_map(|x| x.file.as_ref())
                .collect();
            let main_since: BTreeSet<&String> = main_touches
                .iter()
                .filter(|(s, _)| *s > b.forked_from)
                .map(|(_, f)| f)
                .collect();
            let mut conflicted = false;
            for file in b.touched.iter().filter(|f| main_since.contains(f) && !resolved.contains(f)) {
                conflicted = true;
                errors.push(Error::MergeConflict {
                    step: step.id.clone(),
                    branch: target.clone(),
                    file: file.clone(),
                });
            }
            if conflicted {
                continue;
            }
            // The merge tree: the branch's blocks, replayed over main as it
            // stands now, then the resolution blocks.
            let mut tree = main_tree.clone();
            let mut replay_errors = Vec::new();
            for blk in &b.blocks {
                tree.apply(&step.id, blk, &mut replay_errors);
            }
            for blk in step.blocks.iter().filter(|x| !resolved.contains(x.file.as_ref().unwrap_or(&String::new()))) {
                tree.apply(&step.id, blk, &mut replay_errors);
            }
            // Resolution blocks: whole-file, so order does not matter.
            for blk in step.blocks.iter().filter(|x| resolved.contains(x.file.as_ref().unwrap_or(&String::new()))) {
                let mut scratch = Vec::new();
                // A resolving `replace` may land on a file the branch created
                // over main: treat it as an overwrite either way.
                match blk.op {
                    Op::Replace | Op::Create => {
                        tree.0.insert(blk.file.clone().unwrap_or_default(), blk.lines.join("\n") + "\n");
                    }
                    _ => tree.apply(&step.id, blk, &mut scratch),
                }
                replay_errors.extend(scratch);
            }
            if !replay_errors.is_empty() {
                errors.extend(replay_errors);
                continue;
            }
            let parents = vec![main_head, b.head];
            b.merged_at = Some(seq);
            let touched: Vec<String> = b.touched.iter().cloned().collect();
            main_touches.extend(touched.into_iter().map(|f| (seq, f)));
            main_touches.extend(files_of(&step.blocks).map(|f| (seq, f.clone())));
            main_tree = tree.clone();
            main_head = seq;
            planned.push(PlannedStep {
                seq,
                id: step.id.clone(),
                msg: step.msg.clone(),
                expect: step.expect,
                line: Line::Main,
                parents,
                merges: Some(target.clone()),
                tree,
            });
            continue;
        }

        if let Some(name) = &step.branch {
            if !branches.contains_key(name) {
                // First step of the branch: decide the fork point.
                let (forked_from, tree) = match &step.from {
                    None => (main_head, main_tree.clone()),
                    Some(from) => match planned.iter().find(|p| &p.id == from) {
                        None => {
                            errors.push(Error::UnknownFrom {
                                step: step.id.clone(),
                                from: from.clone(),
                            });
                            continue;
                        }
                        Some(p) if p.line != Line::Main => {
                            errors.push(Error::FromNotOnMain {
                                step: step.id.clone(),
                                from: from.clone(),
                            });
                            continue;
                        }
                        Some(p) => (p.seq, p.tree.clone()),
                    },
                };
                branch_order.push(name.clone());
                branches.insert(
                    name.clone(),
                    BranchState {
                        forked_from,
                        head: forked_from,
                        tree,
                        blocks: Vec::new(),
                        touched: BTreeSet::new(),
                        merged_at: None,
                        pr: None,
                    },
                );
            }
            let b = branches.get_mut(name).expect("inserted above");
            if let Some(at) = b.merged_at {
                errors.push(Error::BranchAlreadyMerged {
                    step: step.id.clone(),
                    branch: name.clone(),
                    merged_at: planned[at - 1].id.clone(),
                });
                continue;
            }
            let mut tree = b.tree.clone();
            for blk in &step.blocks {
                tree.apply(&step.id, blk, &mut errors);
            }
            if step.pr.is_some() {
                b.pr = step.pr.clone();
            }
            b.blocks.extend(step.blocks.iter().cloned());
            b.touched.extend(files_of(&step.blocks).cloned());
            let parents = vec![b.head];
            b.head = seq;
            b.tree = tree.clone();
            planned.push(PlannedStep {
                seq,
                id: step.id.clone(),
                msg: step.msg.clone(),
                expect: step.expect,
                line: Line::Branch(name.clone()),
                parents,
                merges: None,
                tree,
            });
            continue;
        }

        // An ordinary main step.
        let mut tree = main_tree.clone();
        for blk in &step.blocks {
            tree.apply(&step.id, blk, &mut errors);
        }
        main_touches.extend(files_of(&step.blocks).map(|f| (seq, f.clone())));
        let parents = vec![main_head];
        main_head = seq;
        main_tree = tree.clone();
        planned.push(PlannedStep {
            seq,
            id: step.id.clone(),
            msg: step.msg.clone(),
            expect: step.expect,
            line: Line::Main,
            parents,
            merges: None,
            tree,
        });
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    let summaries = branch_order
        .iter()
        .map(|name| {
            let b = &branches[name];
            BranchSummary {
                name: name.clone(),
                forked_from: b.forked_from,
                head: b.head,
                merged_at: b.merged_at,
                pr: b.pr.as_ref().map(|pr| PullRequest {
                    branch: name.clone(),
                    title: pr.title.clone(),
                    body: pr.body.clone(),
                    state: if b.merged_at.is_some() { PrState::Merged } else { PrState::Open },
                }),
            }
        })
        .collect();
    Ok(RepoPlan { steps: planned, branches: summaries })
}

/// The `bower.lock` shape this feature wants: line and parents per step, then
/// a branch table. Pure string out.
pub fn lock_text(plan: &RepoPlan) -> String {
    let mut out = String::from("[failers]\n");
    for s in &plan.steps {
        let parents: Vec<String> = s.parents.iter().map(|p| format!("{p:03}")).collect();
        out.push_str(&format!(
            "{:03} {} expect={} line={} parents={}",
            s.seq,
            s.id,
            s.expect,
            s.line,
            parents.join(",")
        ));
        if let Some(m) = &s.merges {
            out.push_str(&format!(" merges={m}"));
        }
        out.push('\n');
    }
    if !plan.branches.is_empty() {
        out.push_str("\n[failers.branches]\n");
        for b in &plan.branches {
            out.push_str(&format!(
                "{} from={:03} head={:03} merged={}",
                b.name,
                b.forked_from,
                b.head,
                match b.merged_at {
                    Some(s) => format!("{s:03}"),
                    None => "no".to_string(),
                }
            ));
            if let Some(pr) = &b.pr {
                out.push_str(&format!(
                    " pr={:?} state={}",
                    pr.title,
                    match pr.state {
                        PrState::Open => "open",
                        PrState::Merged => "merged",
                    }
                ));
            }
            out.push('\n');
        }
    }
    out
}

pub mod replay;
pub mod fixtures;
