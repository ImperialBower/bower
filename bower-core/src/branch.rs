//! Lines of history (EPIC-09): which line a step's commit sits on, the fold
//! that keeps one tree per line, the pull requests a book declares, and the
//! rules a branch name must follow.

use std::collections::{BTreeMap, BTreeSet};

use crate::step::{Step, Touch, TouchKind, composes, touches};
use crate::tree::TreeState;
use crate::{BowerError, Errors, block::Block};

/// Which line of history a step's commit sits on.
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Line {
    #[default]
    Main,
    Branch(String),
}

impl std::fmt::Display for Line {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Main => f.write_str("main"),
            Self::Branch(b) => f.write_str(b),
        }
    }
}

/// One branch of a repo's plan: where it forked, where it ends, and the step
/// that merged it, if one did.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BranchSummary {
    pub name: String,
    /// The main step it forked at; `0` is the scaffolding.
    pub forked_from: usize,
    /// Its last step: where `refs/heads/<name>` points.
    pub head: usize,
    pub merged_at: Option<usize>,
}

/// What the fold decided for one step: its line, its parents, the branch it
/// merges, and its tree before materialization.
#[derive(Clone, Debug)]
pub struct Folded {
    pub line: Line,
    /// Parent seqs; `0` is the scaffolding. Two for a merge: main's head, then
    /// the branch's.
    pub parents: Vec<usize>,
    pub merges: Option<String>,
    pub tree: TreeState,
}

struct BranchState {
    forked_from: usize,
    head: usize,
    tree: TreeState,
    /// Every block applied on the branch since the fork, in order: what a
    /// merge re-applies over main.
    blocks: Vec<Block>,
    touched: Vec<Touch>,
    merged_at: Option<usize>,
}

/// One repo's fold over lines of history (Decisions 2–4). Pure: steps in plan
/// order in, one [`Folded`] out per step, errors collected.
pub struct Fold<'a> {
    assets: &'a BTreeMap<String, Vec<u8>>,
    main: TreeState,
    main_head: usize,
    /// Main's tree after each main step, for `from=`.
    main_trees: BTreeMap<usize, TreeState>,
    /// What main changed, and at which step.
    main_touches: Vec<(usize, Touch)>,
    branches: BTreeMap<String, BranchState>,
    /// Branch names, in the order each first appears.
    order: Vec<String>,
    /// Every step folded so far: id → (seq, line).
    seen: BTreeMap<String, (usize, Line)>,
    /// Step ids by seq, to name a merge in an error.
    ids: BTreeMap<usize, String>,
}

impl<'a> Fold<'a> {
    #[must_use]
    pub fn new(assets: &'a BTreeMap<String, Vec<u8>>) -> Self {
        Self {
            assets,
            main: TreeState::default(),
            main_head: 0,
            main_trees: BTreeMap::new(),
            main_touches: Vec::new(),
            branches: BTreeMap::new(),
            order: Vec::new(),
            seen: BTreeMap::new(),
            ids: BTreeMap::new(),
        }
    }

    /// Fold `step`, the `seq`-th of its repo.
    ///
    /// A step the fold refuses is reported and comes back as main's current
    /// state, applying nothing, so one mistake is one error rather than a
    /// cascade of `FileNotCreated`s after it. The plan fails either way.
    pub fn step(&mut self, seq: usize, step: &Step, errors: &mut Errors) -> Folded {
        let folded = if step.branch.is_none() && step.from.is_some() {
            // `from=` on a main or merge step: nothing reads it there (only a
            // branch's first step does), so it is a typo, not a no-op.
            errors.push(BowerError::FromOnLaterStep {
                loc: step.anchor.clone(),
                step: step.id.0.clone(),
                branch: "main".to_string(),
            });
            None
        } else if let Some(target) = &step.merge {
            self.merge(seq, step, target, errors)
        } else if let Some(name) = &step.branch {
            self.on_branch(seq, step, name, errors)
        } else {
            Some(self.on_main(seq, step, errors))
        };
        let folded = folded.unwrap_or_else(|| Folded {
            line: Line::Main,
            parents: vec![self.main_head],
            merges: None,
            tree: self.main.clone(),
        });
        self.seen
            .insert(step.id.0.clone(), (seq, folded.line.clone()));
        self.ids.insert(seq, step.id.0.clone());
        folded
    }

    /// Every branch, in the order its first step appeared.
    #[must_use]
    pub fn finish(self) -> Vec<BranchSummary> {
        self.order
            .iter()
            .filter_map(|name| {
                self.branches.get(name).map(|b| BranchSummary {
                    name: name.clone(),
                    forked_from: b.forked_from,
                    head: b.head,
                    merged_at: b.merged_at,
                })
            })
            .collect()
    }

    fn on_main(&mut self, seq: usize, step: &Step, errors: &mut Errors) -> Folded {
        let mut tree = self.main.clone();
        for b in &step.blocks {
            tree.apply_block(b, &step.id.0, self.assets, errors);
        }
        self.main_touches
            .extend(step.blocks.iter().flat_map(touches).map(|t| (seq, t)));
        let parents = vec![self.main_head];
        self.advance_main(seq, &tree);
        Folded {
            line: Line::Main,
            parents,
            merges: None,
            tree,
        }
    }

    fn on_branch(
        &mut self,
        seq: usize,
        step: &Step,
        name: &str,
        errors: &mut Errors,
    ) -> Option<Folded> {
        if let Some(b) = self.branches.get(name) {
            if let Some(at) = b.merged_at {
                errors.push(BowerError::BranchAlreadyMerged {
                    loc: step.anchor.clone(),
                    step: step.id.0.clone(),
                    branch: name.to_string(),
                    merged_at: self.ids.get(&at).cloned().unwrap_or_default(),
                });
                return None;
            }
            if step.from.is_some() {
                errors.push(BowerError::FromOnLaterStep {
                    loc: step.anchor.clone(),
                    step: step.id.0.clone(),
                    branch: name.to_string(),
                });
                return None;
            }
        } else {
            let (forked_from, tree) = self.fork_point(step, errors)?;
            self.order.push(name.to_string());
            self.branches.insert(
                name.to_string(),
                BranchState {
                    forked_from,
                    head: forked_from,
                    tree,
                    blocks: Vec::new(),
                    touched: Vec::new(),
                    merged_at: None,
                },
            );
        }

        let b = self.branches.get_mut(name)?;
        let mut tree = b.tree.clone();
        for blk in &step.blocks {
            tree.apply_block(blk, &step.id.0, self.assets, errors);
        }
        b.blocks.extend(step.blocks.iter().cloned());
        b.touched.extend(step.blocks.iter().flat_map(touches));
        let parents = vec![b.head];
        b.head = seq;
        b.tree.clone_from(&tree);
        Some(Folded {
            line: Line::Branch(name.to_string()),
            parents,
            merges: None,
            tree,
        })
    }

    /// Where a branch's first step forks: main as it stands (Decision 2), or
    /// the main step `from=` names.
    fn fork_point(&self, step: &Step, errors: &mut Errors) -> Option<(usize, TreeState)> {
        let Some(from) = &step.from else {
            return Some((self.main_head, self.main.clone()));
        };
        match self.seen.get(from) {
            None => {
                errors.push(BowerError::UnknownFrom {
                    loc: step.anchor.clone(),
                    step: step.id.0.clone(),
                    from: from.clone(),
                });
                None
            }
            Some((_, Line::Branch(_))) => {
                errors.push(BowerError::FromNotOnMain {
                    loc: step.anchor.clone(),
                    step: step.id.0.clone(),
                    from: from.clone(),
                });
                None
            }
            Some((seq, Line::Main)) => {
                Some((*seq, self.main_trees.get(seq).cloned().unwrap_or_default()))
            }
        }
    }

    /// A merge (Decision 3): the branch's blocks since the fork, re-applied
    /// over main as it stands, then the merge step's own blocks. A whole-file
    /// block on the merge step is a resolution: its file skips the branch's
    /// blocks and takes the step's content.
    fn merge(
        &mut self,
        seq: usize,
        step: &Step,
        target: &str,
        errors: &mut Errors,
    ) -> Option<Folded> {
        if step.branch.is_some() {
            errors.push(BowerError::MergeOnBranch {
                loc: step.anchor.clone(),
                step: step.id.0.clone(),
            });
            return None;
        }
        let Some(b) = self.branches.get(target) else {
            errors.push(BowerError::MergeUnknownBranch {
                loc: step.anchor.clone(),
                step: step.id.0.clone(),
                branch: target.to_string(),
            });
            return None;
        };
        if let Some(at) = b.merged_at {
            errors.push(BowerError::BranchAlreadyMerged {
                loc: step.anchor.clone(),
                step: step.id.0.clone(),
                branch: target.to_string(),
                merged_at: self.ids.get(&at).cloned().unwrap_or_default(),
            });
            return None;
        }

        let resolved: BTreeSet<String> = step
            .blocks
            .iter()
            .flat_map(touches)
            .filter(|t| t.kind == TouchKind::Whole)
            .map(|t| t.file)
            .collect();
        let main_since: Vec<&Touch> = self
            .main_touches
            .iter()
            .filter(|(s, _)| *s > b.forked_from)
            .map(|(_, t)| t)
            .collect();
        let clashes = conflicts(&b.touched, &main_since, &resolved);
        if !clashes.is_empty() {
            for (file, region) in clashes {
                errors.push(BowerError::MergeConflict {
                    loc: step.anchor.clone(),
                    step: step.id.0.clone(),
                    branch: target.to_string(),
                    file,
                    region,
                });
            }
            return None;
        }

        let mut tree = self.main.clone();
        for blk in &b.blocks {
            if let Some(trimmed) = trim_resolved(blk, &resolved) {
                tree.apply_block(&trimmed, &step.id.0, self.assets, errors);
            }
        }
        for blk in &step.blocks {
            if touches(blk).iter().any(|t| t.kind == TouchKind::Whole) {
                tree.resolve_block(blk, &step.id.0, self.assets, errors);
            } else {
                tree.apply_block(blk, &step.id.0, self.assets, errors);
            }
        }

        let parents = vec![self.main_head, b.head];
        let branch_touches = b.touched.clone();
        if let Some(b) = self.branches.get_mut(target) {
            b.merged_at = Some(seq);
        }
        self.main_touches
            .extend(branch_touches.into_iter().map(|t| (seq, t)));
        self.main_touches
            .extend(step.blocks.iter().flat_map(touches).map(|t| (seq, t)));
        self.advance_main(seq, &tree);
        Some(Folded {
            line: Line::Main,
            parents,
            merges: Some(target.to_string()),
            tree,
        })
    }

    fn advance_main(&mut self, seq: usize, tree: &TreeState) {
        self.main_head = seq;
        self.main_trees.insert(seq, tree.clone());
        self.main.clone_from(tree);
    }
}

/// A branch block, as a merge re-applies it, with any path the merge step
/// resolves removed from `file`/`paths` — `None` when every path it touches
/// is resolved, so the whole block drops. A multi-path `delete` may touch
/// one resolved file and one that is not: dropping the whole block would
/// silently lose the delete of the file that was never resolved, so only
/// the resolved paths are trimmed away, never the block as a whole.
fn trim_resolved(block: &Block, resolved: &BTreeSet<String>) -> Option<Block> {
    let file_resolved = block.file.as_ref().is_some_and(|f| resolved.contains(f));
    if !file_resolved && block.paths.iter().all(|p| !resolved.contains(p)) {
        return Some(block.clone());
    }
    let mut out = block.clone();
    if file_resolved {
        out.file = None;
    }
    out.paths.retain(|p| !resolved.contains(p));
    if out.file.is_none() && out.paths.is_empty() {
        return None;
    }
    Some(out)
}

/// Every `(file, region)` where a branch touch and a main-since-fork touch on
/// one file fail to [`composes`], leaving out the files the merge step
/// resolves. A set, so each clash is reported once, in the same order every
/// run. The region is named only when both sides hit the same region.
fn conflicts(
    branch: &[Touch],
    main_since: &[&Touch],
    resolved: &BTreeSet<String>,
) -> BTreeSet<(String, Option<String>)> {
    let mut out = BTreeSet::new();
    for bt in branch.iter().filter(|t| !resolved.contains(&t.file)) {
        for mt in main_since.iter().filter(|m| m.file == bt.file) {
            if !composes(&bt.kind, &mt.kind) {
                let region = match (&bt.kind, &mt.kind) {
                    (TouchKind::Region(a), TouchKind::Region(_)) => Some(a.clone()),
                    _ => None,
                };
                out.insert((bt.file.clone(), region));
            }
        }
    }
    out
}

/// Why `name` cannot name a branch, or `None` when it can (Decision 6).
///
/// Git's ref-name rules (`git check-ref-format`), written out rather than
/// asked of git — the kernel runs no programs — plus Bower's own two: `main`
/// is the main line's name, and `step-…` is how step tags are named, so a
/// branch called that would be ambiguous in `git checkout`.
// Git's own `.lock` suffix check is a literal byte match, not a
// case-insensitive extension check — `clippy::case_sensitive_file_extension_comparisons`
// is meant for filesystem paths, not ref-name syntax, so it does not apply here.
#[allow(clippy::case_sensitive_file_extension_comparisons)]
#[must_use]
pub fn branch_name_problem(name: &str) -> Option<&'static str> {
    if name.is_empty() {
        return Some("it is empty");
    }
    if name == "main" || name == "HEAD" {
        return Some("the main line already has that name");
    }
    if name.starts_with("step-") {
        return Some("`step-` begins every step tag");
    }
    if name == "@" || name.contains("@{") {
        return Some("git reserves `@` and `@{`");
    }
    if name.starts_with(['-', '/']) || name.ends_with(['/', '.']) {
        return Some("it may not begin with `-` or `/`, or end with `/` or `.`");
    }
    if name.contains("..") || name.contains("//") {
        return Some("it may not contain `..` or `//`");
    }
    if name
        .chars()
        .any(|c| c.is_control() || c.is_whitespace() || "~^:?*[\\".contains(c))
    {
        return Some("it may not contain whitespace, control characters, or any of ~ ^ : ? * [ \\");
    }
    if name
        .split('/')
        .any(|part| part.starts_with('.') || part.ends_with(".lock"))
    {
        return Some("no part of it may begin with `.` or end with `.lock`");
    }
    None
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod branch_tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("try/lookup-table")]
    #[case("feat/greet-many")]
    #[case("from-char")]
    #[case("a.b")]
    fn branch_name_problem__accepts_ordinary_names(#[case] name: &str) {
        assert_eq!(branch_name_problem(name), None, "{name}");
    }

    #[rstest]
    #[case("")]
    #[case("main")]
    #[case("HEAD")]
    #[case("step-001-rank")]
    #[case("@")]
    #[case("a@{b")]
    #[case("-x")]
    #[case("/x")]
    #[case("x/")]
    #[case("x.")]
    #[case("a..b")]
    #[case("a//b")]
    #[case("a b")]
    #[case("a~b")]
    #[case("a^b")]
    #[case("a:b")]
    #[case("a?b")]
    #[case("a*b")]
    #[case("a[b")]
    #[case("a\\b")]
    #[case(".x")]
    #[case("a/.b")]
    #[case("x.lock")]
    #[case("a/x.lock/b")]
    fn branch_name_problem__refuses_what_git_or_bower_cannot_use(#[case] name: &str) {
        assert!(branch_name_problem(name).is_some(), "{name:?} was accepted");
    }
}
