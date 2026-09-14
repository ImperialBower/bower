//! What `bower push` will do, in order, decided before any of it happens
//! (EPIC-09 Decisions 20–26).
//!
//! Pure. The plan, what the remote's branches point at, what the built
//! repository's branches point at, and what the forge says about pull
//! requests go in; an ordered list of [`Move`]s and a [`PrAction`] per
//! declared pull request come out. The dry run prints the schedule and
//! `push::execute` carries it out; nothing here touches the network.

use std::collections::BTreeMap;
use std::fmt::{self, Write as _};

use bower_core::prelude::{PlannedStep, PullRequest, RepoPlan};

use crate::publish::digest;
use crate::replay::{BRANCH, last_main_seq};

/// The main line's short name — what a pull request's base is, and what a
/// fresh repository's default branch becomes (Decision 26).
pub const MAIN: &str = "main";

/// A pull request's state as the forge reports it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ForgePrState {
    Open,
    Merged,
    Closed,
}

/// One pull request as the forge reports it — only what a decision needs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForgePr {
    pub number: u64,
    /// Its head branch.
    pub branch: String,
    pub state: ForgePrState,
    /// The commit its head points at on the forge.
    pub head: String,
    /// The `bower-pr:` digest in its body, when it has one (Decision 23).
    pub digest: Option<String>,
}

/// What a push does about one declared pull request (Decision 22).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrAction {
    /// No PR on the forge: one is opened — on a stepping stone when the book
    /// has already merged the branch (Decision 20).
    Created {
        stone: Option<String>,
    },
    /// An open PR whose title or description differs.
    Updated {
        number: u64,
    },
    Unchanged {
        number: u64,
    },
    /// Merged already. `moved` when its head is no longer the branch's head —
    /// the drift a rebuild leaves, reported and never repaired.
    LeftMerged {
        number: u64,
        moved: bool,
    },
    /// Closed by a person. Bower never reopens a PR.
    LeftClosed {
        number: u64,
    },
    /// A merged branch whose PR cannot be opened.
    NotOpened {
        reason: String,
    },
}

impl fmt::Display for PrAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created { stone: None } => f.write_str("create"),
            Self::Created { stone: Some(s) } => write!(f, "create, on the stepping stone {s}"),
            Self::Updated { number } => write!(f, "update #{number}"),
            Self::Unchanged { number } => write!(f, "unchanged #{number}"),
            Self::LeftMerged {
                number,
                moved: false,
            } => write!(f, "merged #{number}, left alone"),
            Self::LeftMerged {
                number,
                moved: true,
            } => write!(
                f,
                "merged #{number}, left alone; its head is no longer the branch's"
            ),
            Self::LeftClosed { number } => write!(f, "closed #{number}, left alone"),
            Self::NotOpened { reason } => write!(f, "not opened — {reason}"),
        }
    }
}

/// One thing a push does to the remote.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Move {
    /// Push the book's branch to the remote branch of the same name.
    Branch { name: String },
    /// Move the remote's main to the commit tagged `at` — its head, or a
    /// stepping stone for `stone_for`'s PR.
    Main {
        at: String,
        stone_for: Option<String>,
    },
    /// Make main the repository's default branch (Decision 26).
    DefaultMain,
    OpenPr {
        branch: String,
        title: String,
        body: String,
    },
    EditPr {
        number: u64,
        branch: String,
        title: String,
        body: String,
    },
    /// Push every tag, forced. Always last.
    Tags,
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Branch { name } => write!(f, "branch {name}"),
            Self::Main {
                at,
                stone_for: None,
            } => write!(f, "main → {at}"),
            Self::Main {
                at,
                stone_for: Some(b),
            } => write!(f, "main → {at} (stepping stone for {b})"),
            Self::DefaultMain => write!(f, "default branch → {MAIN}"),
            Self::OpenPr { branch, title, .. } => write!(f, "open PR {branch} \"{title}\""),
            Self::EditPr {
                number,
                branch,
                title,
                ..
            } => write!(f, "edit PR #{number} {branch} \"{title}\""),
            Self::Tags => f.write_str("tags"),
        }
    }
}

/// Everything a push will do, in order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Schedule {
    pub moves: Vec<Move>,
    /// Every declared PR and what the push does about it, in branch order.
    pub prs: Vec<(String, PrAction)>,
    /// The tag of main's last step — where main must end up, and where
    /// `push::execute` puts it back if it stops on a stone.
    pub head: Option<String>,
}

impl Schedule {
    /// The branches this push sends, in order.
    #[must_use]
    pub fn branches(&self) -> Vec<&str> {
        self.moves
            .iter()
            .filter_map(|m| match m {
                Move::Branch { name } => Some(name.as_str()),
                _ => None,
            })
            .collect()
    }
}

/// The digest a PR's body carries, of its title and description (Decision
/// 23). A digest rather than the text: GitHub may rewrite a body's line
/// endings, and the base binary parses no JSON to read a body back with.
#[must_use]
pub fn pr_digest(pr: &PullRequest) -> String {
    digest(&format!("{}\n{}", pr.title, pr.body.join("\n")))
}

/// The body Bower writes for `pr`: the book's description, then a footer
/// saying where the PR came from and how it merges, then the digest.
#[must_use]
pub fn pr_body(pr: &PullRequest, book: &str) -> String {
    let mut out = String::new();
    if !pr.body.is_empty() {
        out.push_str(&pr.body.join("\n"));
        // A blank line first: `---` straight under a paragraph is a heading.
        out.push_str("\n\n");
    }
    let _ = write!(
        out,
        "---\nOpened by Bower from the book *{book}*. It is merged by a push to main, never on the forge.\n<!-- bower-pr: {} -->\n",
        pr_digest(pr)
    );
    out
}

/// What a push does about `pr`, given what the forge holds for its branch
/// (Decision 22). An open PR outranks a merged one, and a merged one a
/// closed one. `local_head` is the built repository's branch head.
#[must_use]
pub fn pr_action(pr: &PullRequest, on_forge: &[ForgePr], local_head: Option<&str>) -> PrAction {
    let mine = || on_forge.iter().filter(|f| f.branch == pr.branch);
    let latest = |state| mine().filter(|f| f.state == state).max_by_key(|f| f.number);

    if let Some(open) = latest(ForgePrState::Open) {
        return if open.digest.as_deref() == Some(pr_digest(pr).as_str()) {
            PrAction::Unchanged {
                number: open.number,
            }
        } else {
            PrAction::Updated {
                number: open.number,
            }
        };
    }
    if let Some(merged) = latest(ForgePrState::Merged) {
        return PrAction::LeftMerged {
            number: merged.number,
            moved: local_head.is_some_and(|h| h != merged.head),
        };
    }
    if let Some(closed) = latest(ForgePrState::Closed) {
        return PrAction::LeftClosed {
            number: closed.number,
        };
    }
    PrAction::Created { stone: None }
}

/// The step tag main stands on while a merged branch's PR is opened: the
/// merge step's main parent (Decision 20). `None` when that parent is the
/// scaffolding commit, which has no tag.
fn stone_for(plan: &RepoPlan, merged_at: usize) -> Option<String> {
    let merge = plan.steps.iter().find(|s| s.seq == merged_at)?;
    let parent = *merge.parents.first()?;
    plan.steps
        .iter()
        .find(|s| s.seq == parent)
        .map(PlannedStep::tag)
}

/// Decide the whole push (Decisions 20–22, 26).
///
/// On a remote with main: every branch, then — per merged PR that needs a
/// stone, in merge order — main to the stone and the PR opened, then main to
/// its head, then the other PRs opened or edited, then the tags. The other
/// PRs wait for main's head: a PR opens only against a base that shares its
/// history, and after a rebuild the remote's old main shares none. On a
/// remote with no main, main goes first — to the first stone or its head —
/// and becomes the default branch, before any branch.
#[must_use]
pub fn schedule(
    plan: &RepoPlan,
    remote_heads: &BTreeMap<String, String>,
    local_heads: &BTreeMap<String, String>,
    on_forge: &[ForgePr],
    book: &str,
) -> Schedule {
    let head = last_main_seq(plan)
        .and_then(|seq| plan.steps.iter().find(|s| s.seq == seq))
        .map(PlannedStep::tag);

    let mut prs = Vec::new();
    // (merge seq, stone tag, branch, the PR to open on it)
    let mut stones: Vec<(usize, String, String, Move)> = Vec::new();
    let mut later = Vec::new();
    for b in &plan.branches {
        let Some(pr) = &b.pr else { continue };
        let mut action = pr_action(pr, on_forge, local_heads.get(&b.name).map(String::as_str));
        let body = pr_body(pr, book);
        match (&action, b.merged_at) {
            (PrAction::Created { .. }, None) => later.push(Move::OpenPr {
                branch: b.name.clone(),
                title: pr.title.clone(),
                body,
            }),
            (PrAction::Created { .. }, Some(at)) => match stone_for(plan, at) {
                Some(tag) => {
                    stones.push((
                        at,
                        tag.clone(),
                        b.name.clone(),
                        Move::OpenPr {
                            branch: b.name.clone(),
                            title: pr.title.clone(),
                            body,
                        },
                    ));
                    action = PrAction::Created { stone: Some(tag) };
                }
                None => {
                    action = PrAction::NotOpened {
                        reason: "its merge's main parent is the scaffolding, so main has no step to stand on first".to_string(),
                    };
                }
            },
            (PrAction::Updated { number }, _) => later.push(Move::EditPr {
                number: *number,
                branch: b.name.clone(),
                title: pr.title.clone(),
                body,
            }),
            _ => {}
        }
        prs.push((b.name.clone(), action));
    }
    stones.sort_by_key(|(at, ..)| *at);

    let branches = plan.branches.iter().map(|b| Move::Branch {
        name: b.name.clone(),
    });
    let mut moves = Vec::new();
    let mut main_at: Option<String> = None;
    if remote_heads.contains_key(BRANCH) {
        moves.extend(branches);
    } else {
        let first = stones
            .first()
            .map(|(_, tag, branch, _)| (tag.clone(), Some(branch.clone())))
            .or_else(|| head.clone().map(|h| (h, None)));
        if let Some((at, stone_for)) = first {
            moves.push(Move::Main {
                at: at.clone(),
                stone_for,
            });
            moves.push(Move::DefaultMain);
            main_at = Some(at);
        }
        moves.extend(branches);
    }
    for (_, tag, branch, open) in stones {
        if main_at.as_deref() != Some(tag.as_str()) {
            moves.push(Move::Main {
                at: tag.clone(),
                stone_for: Some(branch),
            });
            main_at = Some(tag);
        }
        moves.push(open);
    }
    if let Some(h) = &head
        && main_at.as_deref() != Some(h.as_str())
    {
        moves.push(Move::Main {
            at: h.clone(),
            stone_for: None,
        });
    }
    moves.extend(later);
    moves.push(Move::Tags);

    Schedule { moves, prs, head }
}

#[cfg(test)]
#[allow(non_snake_case, clippy::unwrap_used)]
mod schedule_tests {
    use super::*;
    use bower_core::prelude::{BookSource, Chapter, RepoCatalog, plan};
    use bower_testkit::fixtures;

    /// The branch saga: `try/lookup-table` (open PR, never merged) and
    /// `from-char` (PR, merged at step 6, whose main parent is step 3).
    fn saga() -> RepoPlan {
        let f = fixtures::branch_saga();
        plan(&f.book, &f.catalog).unwrap().repos.remove(0)
    }

    fn plan_of(text: &str) -> RepoPlan {
        let book = BookSource::from_chapters(vec![Chapter::new("ch01.md", text)]);
        plan(&book, &RepoCatalog::from_names(&["failers"]))
            .unwrap()
            .repos
            .remove(0)
    }

    fn existing() -> BTreeMap<String, String> {
        BTreeMap::from([(BRANCH.to_string(), "0".repeat(40))])
    }

    fn none() -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    fn pr_of(p: &RepoPlan, branch: &str) -> PullRequest {
        p.branches
            .iter()
            .find(|b| b.name == branch)
            .and_then(|b| b.pr.clone())
            .unwrap()
    }

    fn forge_pr(number: u64, branch: &str, state: ForgePrState, digest: Option<String>) -> ForgePr {
        ForgePr {
            number,
            branch: branch.to_string(),
            state,
            head: "f".repeat(40),
            digest,
        }
    }

    fn shapes(moves: &[Move]) -> Vec<String> {
        moves.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn schedule__a_straight_line_is_main_then_tags() {
        let f = fixtures::rank_saga();
        let p = plan(&f.book, &f.catalog).unwrap().repos.remove(0);
        let s = schedule(&p, &existing(), &none(), &[], "book");
        assert_eq!(shapes(&s.moves), ["main → step-003-from-char", "tags"]);
        assert!(s.prs.is_empty());
        assert!(s.branches().is_empty());
    }

    #[test]
    fn schedule__an_existing_remote_pushes_branches_then_prs_then_main_then_tags() {
        let s = schedule(&saga(), &existing(), &none(), &[], "book");
        assert_eq!(
            shapes(&s.moves),
            [
                "branch try/lookup-table",
                "branch from-char",
                "main → step-003-lib-doc (stepping stone for from-char)",
                "open PR from-char \"Rank::from(char)\"",
                "main → step-006-merge-from-char",
                "open PR try/lookup-table \"Try a lookup table for ranks\"",
                "tags",
            ]
        );
    }

    #[test]
    fn schedule__a_fresh_remote_puts_main_first_and_makes_it_the_default() {
        let s = schedule(&saga(), &none(), &none(), &[], "book");
        assert_eq!(
            shapes(&s.moves),
            [
                "main → step-003-lib-doc (stepping stone for from-char)",
                "default branch → main",
                "branch try/lookup-table",
                "branch from-char",
                "open PR from-char \"Rank::from(char)\"",
                "main → step-006-merge-from-char",
                "open PR try/lookup-table \"Try a lookup table for ranks\"",
                "tags",
            ]
        );
        let f = fixtures::rank_saga();
        let straight = plan(&f.book, &f.catalog).unwrap().repos.remove(0);
        let s = schedule(&straight, &none(), &none(), &[], "book");
        assert_eq!(
            shapes(&s.moves),
            ["main → step-003-from-char", "default branch → main", "tags"]
        );
    }

    #[test]
    fn schedule__an_existing_remote_never_changes_its_default() {
        let s = schedule(&saga(), &existing(), &none(), &[], "book");
        assert!(!s.moves.contains(&Move::DefaultMain), "{:?}", s.moves);
    }

    #[test]
    fn schedule__a_merged_pr_without_a_forge_pr_gets_a_stepping_stone() {
        let s = schedule(&saga(), &existing(), &none(), &[], "book");
        let action = &s.prs.iter().find(|(b, _)| b == "from-char").unwrap().1;
        assert_eq!(
            action,
            &PrAction::Created {
                stone: Some("step-003-lib-doc".to_string())
            }
        );
        assert_eq!(
            action.to_string(),
            "create, on the stepping stone step-003-lib-doc"
        );
    }

    #[test]
    fn schedule__stones_go_in_merge_order() {
        // `early` is declared first but merged last: its stone comes second.
        let p = plan_of(concat!(
            "<!-- bower repo=\"failers\" step=\"base\" file=\"a.rs\" -->\n```rust\nx\n```\n",
            "<!-- bower repo=\"failers\" step=\"e1\" branch=\"early\" file=\"e.rs\" pr=\"Early\" -->\n```rust\ne\n```\n",
            "<!-- bower repo=\"failers\" step=\"l1\" branch=\"late\" file=\"l.rs\" pr=\"Late\" -->\n```rust\nl\n```\n",
            "<!-- bower repo=\"failers\" step=\"m1\" file=\"a.rs\" op=\"append\" -->\n```rust\ny\n```\n",
            "<!-- bower repo=\"failers\" step=\"join-late\" merge=\"late\" op=\"none\" msg=\"merge late\" -->\n",
            "<!-- bower repo=\"failers\" step=\"m2\" file=\"a.rs\" op=\"append\" -->\n```rust\nz\n```\n",
            "<!-- bower repo=\"failers\" step=\"join-early\" merge=\"early\" op=\"none\" msg=\"merge early\" -->\n",
        ));
        let s = schedule(&p, &existing(), &none(), &[], "book");
        let stones: Vec<String> = shapes(&s.moves)
            .into_iter()
            .filter(|m| m.contains("stepping stone"))
            .collect();
        assert_eq!(
            stones,
            [
                "main → step-004-m1 (stepping stone for late)",
                "main → step-006-m2 (stepping stone for early)",
            ]
        );
    }

    #[test]
    fn schedule__a_forge_pr_that_exists_needs_no_stone() {
        let p = saga();
        let lookup = pr_of(&p, "try/lookup-table");
        let on_forge = [
            forge_pr(
                1,
                "try/lookup-table",
                ForgePrState::Open,
                Some(pr_digest(&lookup)),
            ),
            forge_pr(2, "from-char", ForgePrState::Merged, None),
        ];
        let s = schedule(&p, &existing(), &none(), &on_forge, "book");
        assert_eq!(
            shapes(&s.moves),
            [
                "branch try/lookup-table",
                "branch from-char",
                "main → step-006-merge-from-char",
                "tags",
            ]
        );
        assert_eq!(
            s.prs,
            vec![
                (
                    "try/lookup-table".to_string(),
                    PrAction::Unchanged { number: 1 }
                ),
                (
                    "from-char".to_string(),
                    PrAction::LeftMerged {
                        number: 2,
                        moved: false
                    }
                ),
            ]
        );
    }

    #[test]
    fn pr_action__an_open_pr_is_updated_only_when_it_differs() {
        let p = saga();
        let pr = pr_of(&p, "try/lookup-table");
        let same = [forge_pr(
            7,
            "try/lookup-table",
            ForgePrState::Open,
            Some(pr_digest(&pr)),
        )];
        assert_eq!(
            pr_action(&pr, &same, None),
            PrAction::Unchanged { number: 7 }
        );
        let other = [forge_pr(
            7,
            "try/lookup-table",
            ForgePrState::Open,
            Some("0".repeat(16)),
        )];
        assert_eq!(
            pr_action(&pr, &other, None),
            PrAction::Updated { number: 7 }
        );
        // A body a person wrote carries no digest: it reads as different.
        let bare = [forge_pr(7, "try/lookup-table", ForgePrState::Open, None)];
        assert_eq!(pr_action(&pr, &bare, None), PrAction::Updated { number: 7 });
    }

    #[test]
    fn pr_action__a_merged_pr_is_left_alone_and_notes_a_moved_head() {
        let p = saga();
        let pr = pr_of(&p, "from-char");
        let merged = [forge_pr(2, "from-char", ForgePrState::Merged, None)];
        let head = "f".repeat(40);
        assert_eq!(
            pr_action(&pr, &merged, Some(&head)),
            PrAction::LeftMerged {
                number: 2,
                moved: false
            }
        );
        let moved = pr_action(&pr, &merged, Some("a1b2c3"));
        assert_eq!(
            moved,
            PrAction::LeftMerged {
                number: 2,
                moved: true
            }
        );
        assert_eq!(
            moved.to_string(),
            "merged #2, left alone; its head is no longer the branch's"
        );
    }

    #[test]
    fn pr_action__a_closed_pr_is_left_closed() {
        let p = saga();
        let pr = pr_of(&p, "try/lookup-table");
        let closed = [forge_pr(4, "try/lookup-table", ForgePrState::Closed, None)];
        assert_eq!(
            pr_action(&pr, &closed, None),
            PrAction::LeftClosed { number: 4 }
        );
        // An open PR outranks a closed one for the same branch.
        let both = [
            forge_pr(4, "try/lookup-table", ForgePrState::Closed, None),
            forge_pr(9, "try/lookup-table", ForgePrState::Open, None),
        ];
        assert_eq!(pr_action(&pr, &both, None), PrAction::Updated { number: 9 });
    }

    #[test]
    fn pr_action__a_stone_on_the_scaffolding_is_not_opened() {
        // The branch forks before any main step and is merged by the first
        // one: the merge's main parent is the scaffolding, which has no tag.
        let p = plan_of(concat!(
            "<!-- bower repo=\"failers\" step=\"b1\" branch=\"b\" file=\"b.rs\" pr=\"Early bird\" -->\n```rust\nb\n```\n",
            "<!-- bower repo=\"failers\" step=\"join\" merge=\"b\" op=\"none\" msg=\"merge b\" -->\n",
        ));
        let s = schedule(&p, &existing(), &none(), &[], "book");
        assert!(
            matches!(&s.prs[0].1, PrAction::NotOpened { .. }),
            "{:?}",
            s.prs
        );
        assert!(
            !shapes(&s.moves).iter().any(|m| m.starts_with("open PR")),
            "{:?}",
            s.moves
        );
    }

    #[test]
    fn pr_body__carries_the_description_the_footer_and_the_digest() {
        let p = saga();
        let pr = pr_of(&p, "try/lookup-table");
        let body = pr_body(&pr, "rank-book");
        assert_eq!(
            body,
            format!(
                "Faster? Maybe. Correct? The test says no.\n\n---\nOpened by Bower from the book *rank-book*. It is merged by a push to main, never on the forge.\n<!-- bower-pr: {} -->\n",
                pr_digest(&pr)
            )
        );
        assert_eq!(pr_digest(&pr).len(), 16);
    }
}
