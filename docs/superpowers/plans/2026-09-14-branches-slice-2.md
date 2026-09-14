# Branches, Slice 2 (EPIC-09) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `bower push` puts every pull request a book declares on the forge — open PRs opened or edited, merged PRs opened on a stepping stone and then merged by the push of main — through a pure, printed schedule that a dry run shows and `--execute` carries out.

**Architecture:** A new pure module `bower/src/schedule.rs` turns the plan, the remote's branch heads, the built repo's branch heads, and the forge's PRs into an ordered list of `Move`s and a `PrAction` per declared PR. `plan_push` reads the remote heads and PRs after the marker gate and carries the schedule; `push::execute` runs it through seven small `Forge` methods that replace `Forge::push`, chaining main's leases and putting main back on its head if it stops on a stone. `GitHubForge` shells out to `git ls-remote`, `gh pr list --jq`, `gh pr create`, `gh pr edit`, and `gh api -X PATCH`; its parsing is pure and tested.

**Tech Stack:** Rust 2024 workspace (`rust-version = "1.98.1"`); the `bower` crate (`gix`, `clap`, `toml`, `time`); `bower-testkit` as a dev-dependency. No new dependency: the base binary parses no JSON (`serde_json` belongs to the preprocessor feature alone).

**Spec:** `docs/EPIC-09_Branches.md` — **Decisions 20–26**, Work Items **3c–3g**, open questions 1, 2, 7, 8, and exit criterion 9. The GitHub behaviour behind them is recorded in `docs/spikes/pr-remote/`.

## Global Constraints

- **Git is the user's.** Never run a state-changing git command against this repository (`~/.claude/CLAUDE.md`). Branch: `feat/branches-slice-2`, created by the user. Every "Commit" step means: stop and print the exact `git add … && git commit -m "…"` for the user. Never run `git push`, `gh pr create`, `gh pr edit`, `gh api -X PATCH`, or `bower push --execute` — the live run (Task 4) is the user's. Reading (`git status`, `gh pr list`, `gh api` GETs) is fine.
- **Gate for every task:** `cargo fmt --all --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `make docs`. `make docs` catches private doc links that clippy does not.
- **Never** close, merge, reopen, or delete a PR, and add no flag that would (`--merge-pr`, `--close-pr` stay absent). The marker gate is untouched: a refused push reads no PR and writes nothing.
- **A book without branches** prints exactly the dry run it prints today (Decision 13): the `branches`, `schedule`, and `pr` lines appear only when the book has a branch. Its execution is main to its head, then tags — plus `DefaultMain` on a remote with no main (Decision 26).
- **Branches before main is load-bearing** (open question 2): GitHub closes an open PR when main is force-pushed to a history its head shares no commits with. No move may push main to the rebuilt history before the book's branches.
- **Exact values** (copy verbatim):
  - Main's short name: `MAIN = "main"`; its ref stays `replay::BRANCH` (`refs/heads/main`).
  - PR body: the description lines joined with `\n`, then (only when there is a description) a blank line, then `---`, then `Opened by Bower from the book *<book>*. It is merged by a push to main, never on the forge.`, then `<!-- bower-pr: <digest> -->`, then a trailing newline. `<digest>` is `publish::digest(format!("{title}\n{body joined with \n}"))`.
  - `gh pr list` arguments: `pr list -R <repo> --base main --state all --limit 1000 --json number,state,headRefName,headRefOid,body --jq <PR_LIST_JQ>`, where `PR_LIST_JQ` is `.[] | [(.number|tostring), .state, .headRefName, .headRefOid, ((.body // "") | (capture("bower-pr: (?<d>[0-9a-f]+)").d // "-"))] | join("\t")` — one tab-separated line per PR; states `OPEN`, `MERGED`, `CLOSED`; digest `-` means none.
  - Default branch: `api -X PATCH repos/<repo> -f default_branch=main`.
  - Move display: `branch <name>`; `main → <tag>`; `main → <tag> (stepping stone for <branch>)`; `default branch → main`; `open PR <branch> "<title>"`; `edit PR #<n> <branch> "<title>"`; `tags`.
  - PR action display: `create`; `create, on the stepping stone <tag>`; `update #<n>`; `unchanged #<n>`; `merged #<n>, left alone`; `merged #<n>, left alone; its head is no longer the branch's`; `closed #<n>, left alone`; `not opened — <reason>`.
  - Dry-run block (only with branches): `  branches  <a>, <b>`; then `  schedule  1. <move>` and `            <i>. <move>` for the rest (12 spaces); then one `  pr        <branch> — <action>` per declared PR.
- **Lints:** pedantic; test modules `#[allow(non_snake_case, clippy::unwrap_used)]`; test names `subject__does_thing`.

---

## File Structure

| File | Responsibility |
|---|---|
| `bower/src/schedule.rs` (new) | `ForgePr`, `ForgePrState`, `PrAction`, `Move`, `Schedule`, `MAIN`, `pr_digest`, `pr_body`, `pr_action`, `schedule`. Pure. |
| `bower/src/lib.rs` | `pub mod schedule`. |
| `bower/src/forge.rs` | Seven `Forge` methods in place of `push`; `FakeForge` records them; `GitHubForge` implements them; pure `push_ref_args`, `parse_ls_remote`, `pr_list_args`, `PR_LIST_JQ`, `parse_pr_list`, `pr_number_from_url`, `default_branch_args`. `push`, `push_commands`, `push_branch_args`, `remote_head`, `pushed_branches` are removed in Task 3. |
| `bower/src/push.rs` | `plan_push` reads heads and PRs after the gate; `PushPlan::Ready` carries `schedule` and `remote_heads`; `execute`, `ExecuteError`, `schedule_lines`. |
| `bower/src/main.rs` | `report_push` prints `schedule_lines` and calls `execute`. |
| `bower/tests/push.rs` | The no-override test gains `--merge-pr`, `--close-pr`. |
| `docs/EPIC-09_Branches.md`, `.okf/model/pull-request.md`, `BACKLOG.md`, `docs/TECHNICAL_DEBT.md` | Status, corrigendum, docs (Task 4). |

Task order: the pure schedule (1), the forge's new methods (2), execute and the push wiring (3), docs and the live run (4).

---

### Task 1: The schedule — pure (EPIC Work Item 3c; Decisions 20–23, 26)

**Files:**
- Create: `bower/src/schedule.rs`
- Modify: `bower/src/lib.rs`

**Interfaces:**
- Consumes: `bower_core::prelude::{PullRequest, RepoPlan, PlannedStep}`; `crate::publish::digest(&str) -> String`; `crate::replay::{BRANCH, last_main_seq}`.
- Produces (all `pub`, in `bower::schedule`):
  - `pub const MAIN: &str = "main";`
  - `enum ForgePrState { Open, Merged, Closed }`
  - `struct ForgePr { number: u64, branch: String, state: ForgePrState, head: String, digest: Option<String> }`
  - `enum PrAction { Created { stone: Option<String> }, Updated { number: u64 }, Unchanged { number: u64 }, LeftMerged { number: u64, moved: bool }, LeftClosed { number: u64 }, NotOpened { reason: String } }` with `Display`
  - `enum Move { Branch { name: String }, Main { at: String, stone_for: Option<String> }, DefaultMain, OpenPr { branch: String, title: String, body: String }, EditPr { number: u64, branch: String, title: String, body: String }, Tags }` with `Display`
  - `struct Schedule { moves: Vec<Move>, prs: Vec<(String, PrAction)>, head: Option<String> }` and `Schedule::branches(&self) -> Vec<&str>`
  - `fn pr_digest(pr: &PullRequest) -> String`, `fn pr_body(pr: &PullRequest, book: &str) -> String`
  - `fn pr_action(pr: &PullRequest, on_forge: &[ForgePr], local_head: Option<&str>) -> PrAction`
  - `fn schedule(plan: &RepoPlan, remote_heads: &BTreeMap<String, String>, local_heads: &BTreeMap<String, String>, on_forge: &[ForgePr], book: &str) -> Schedule` — `remote_heads` keys are full refs (`refs/heads/main`); `local_heads` keys are branch names.

- [ ] **Step 1: Write the failing tests**

Create `bower/src/schedule.rs` with only the test module for now, and add `pub mod schedule;` to `bower/src/lib.rs` (after `pub mod replay;`):

```rust
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
        assert_eq!(action.to_string(), "create, on the stepping stone step-003-lib-doc");
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
            forge_pr(1, "try/lookup-table", ForgePrState::Open, Some(pr_digest(&lookup))),
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
                ("try/lookup-table".to_string(), PrAction::Unchanged { number: 1 }),
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
        let same = [forge_pr(7, "try/lookup-table", ForgePrState::Open, Some(pr_digest(&pr)))];
        assert_eq!(pr_action(&pr, &same, None), PrAction::Unchanged { number: 7 });
        let other = [forge_pr(7, "try/lookup-table", ForgePrState::Open, Some("0".repeat(16)))];
        assert_eq!(pr_action(&pr, &other, None), PrAction::Updated { number: 7 });
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
        assert_eq!(pr_action(&pr, &closed, None), PrAction::LeftClosed { number: 4 });
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
        assert!(!shapes(&s.moves).iter().any(|m| m.starts_with("open PR")), "{:?}", s.moves);
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
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p bower --lib schedule`
Expected: FAIL to compile — `cannot find type Move`, `cannot find function schedule`.

- [ ] **Step 3: Write the module**

Above the test module in `bower/src/schedule.rs`:

```rust
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
    Created { stone: Option<String> },
    /// An open PR whose title or description differs.
    Updated { number: u64 },
    Unchanged { number: u64 },
    /// Merged already. `moved` when its head is no longer the branch's head —
    /// the drift a rebuild leaves, reported and never repaired.
    LeftMerged { number: u64, moved: bool },
    /// Closed by a person. Bower never reopens a PR.
    LeftClosed { number: u64 },
    /// A merged branch whose PR cannot be opened.
    NotOpened { reason: String },
}

impl fmt::Display for PrAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Created { stone: None } => f.write_str("create"),
            Self::Created { stone: Some(s) } => write!(f, "create, on the stepping stone {s}"),
            Self::Updated { number } => write!(f, "update #{number}"),
            Self::Unchanged { number } => write!(f, "unchanged #{number}"),
            Self::LeftMerged { number, moved: false } => write!(f, "merged #{number}, left alone"),
            Self::LeftMerged { number, moved: true } => write!(
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
            Self::Main { at, stone_for: None } => write!(f, "main → {at}"),
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
```

`last_main_seq` is `pub` in `bower/src/replay.rs` already (EPIC-09 slice 1). If clippy flags `too_many_lines` on `schedule`, split the PR loop into a private helper `decide_prs(plan, on_forge, local_heads, book) -> (Vec<(String, PrAction)>, Vec<(usize, String, String, Move)>, Vec<Move>)` — same logic, record it as a deviation.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p bower --lib schedule`
Expected: PASS, 12 tests.

- [ ] **Step 5: Gate**

Run: `cargo fmt --all --check && cargo test --workspace && cargo clippy --workspace --all-targets --all-features -- -D warnings && make docs`
Expected: all green.

- [ ] **Step 6: Commit**

```bash
git add bower/src/schedule.rs bower/src/lib.rs
git commit -m "feat(push): the push schedule — stepping stones and a PR action per declared PR"
```

---

### Task 2: The forge's new methods (EPIC Work Item 3d; Decisions 21–23, 26)

Added **alongside** `Forge::push`, which Task 3 removes once nothing calls it — so this task compiles and ships on its own.

**Files:**
- Modify: `bower/src/forge.rs` (trait, `FakeForge`, `GitHubForge`, pure helpers, tests)

**Interfaces:**
- Consumes: `crate::schedule::{ForgePr, ForgePrState, MAIN}` (Task 1).
- Produces, on `trait Forge`:
  - `fn remote_heads(&self, repo: &str) -> Result<BTreeMap<String, String>, ForgeError>` — full ref → SHA, e.g. `refs/heads/main`.
  - `fn pull_requests(&self, repo: &str) -> Result<Vec<ForgePr>, ForgeError>` — every PR with base `main`, any state.
  - `fn push_ref(&self, dir: &Path, repo: &str, src: &str, dst: &str, lease: Option<&str>) -> Result<String, ForgeError>` — pushes the commit `src` names in `dir` to the remote ref `dst`; leases against `lease` when `Some`, plain push when `None`; returns the SHA pushed.
  - `fn push_tags(&self, dir: &Path, repo: &str) -> Result<usize, ForgeError>` — every tag, forced; returns the local tag count.
  - `fn set_default_branch(&self, repo: &str, branch: &str) -> Result<(), ForgeError>`
  - `fn open_pull_request(&self, repo: &str, branch: &str, title: &str, body: &str) -> Result<u64, ForgeError>` — base `main`; returns the PR number.
  - `fn edit_pull_request(&self, repo: &str, number: u64, title: &str, body: &str) -> Result<(), ForgeError>`
- Pure, `pub`: `push_ref_args(url, src, dst, lease) -> Vec<String>`, `parse_ls_remote(&str) -> BTreeMap<String, String>`, `PR_LIST_JQ: &str`, `pr_list_args(repo) -> Vec<String>`, `parse_pr_list(&str) -> Vec<ForgePr>`, `pr_number_from_url(&str) -> Option<u64>`, `default_branch_args(repo, branch) -> Vec<String>`.
- `FakeForge`: new `pub heads: BTreeMap<String, String>` (populated with `refs/heads/main` when constructed with `RemoteState::HasContent`), `pub prs: Vec<ForgePr>`, `pub fail_on: Option<String>` (any recorded call starting with it fails); records `remote_heads`, `pull_requests`, `push_ref <src> -> <dst> lease=<lease or ->`, `push_tags`, `set_default_branch <branch>`, `open_pr <branch>`, `edit_pr #<n>`; `open_pull_request` returns 101, 102, …; `push_ref` returns `sha:<src>`. `mutated()` also counts `set_default_branch`, `open_pr`, `edit_pr`.

- [ ] **Step 1: Write the failing tests for the pure helpers**

In `bower/src/forge.rs`, inside `mod forge_tests`, add:

```rust
    #[test]
    fn push_ref_args__lease_against_the_value_read() {
        assert_eq!(
            push_ref_args(URL, "abc123", "refs/heads/main", Some("def456")),
            [
                "push",
                "--force-with-lease=refs/heads/main:def456",
                URL,
                "abc123:refs/heads/main",
            ]
        );
    }

    #[test]
    fn push_ref_args__a_ref_that_is_not_there_forces_nothing() {
        let args = push_ref_args(URL, "abc123", "refs/heads/try/x", None);
        assert_eq!(args, ["push", URL, "abc123:refs/heads/try/x"]);
        assert!(!args.iter().any(|a| a.contains("force")), "{args:?}");
    }

    #[test]
    fn parse_ls_remote__reads_every_head() {
        let text = "1111111111111111111111111111111111111111\trefs/heads/main\n2222222222222222222222222222222222222222\trefs/heads/try/shout\n";
        let heads = parse_ls_remote(text);
        assert_eq!(heads.len(), 2);
        assert_eq!(heads["refs/heads/main"], "1".repeat(40));
        assert_eq!(heads["refs/heads/try/shout"], "2".repeat(40));
        assert!(parse_ls_remote("").is_empty(), "an empty repository has no heads");
    }

    #[test]
    fn pr_list_args__ask_for_every_state_against_main() {
        let args = pr_list_args("o/n");
        assert_eq!(&args[..5], ["pr", "list", "-R", "o/n", "--base"]);
        assert!(args.contains(&"main".to_string()), "{args:?}");
        assert!(args.windows(2).any(|w| w == ["--state", "all"]), "{args:?}");
        assert!(args.windows(2).any(|w| w[0] == "--jq" && w[1] == PR_LIST_JQ), "{args:?}");
    }

    #[test]
    fn parse_pr_list__reads_the_jq_lines() {
        // The exact shape `gh pr list --jq PR_LIST_JQ` printed against
        // abstecker/bower-sandbox on 14 September 2026.
        let text = concat!(
            "4\tMERGED\tfeat/merge-me\ta53df691b8873e11ee76dcb9a6a30ded39fcd8cc\t-\n",
            "3\tOPEN\ttry/abandon\t2c754e9e638914b95dadaf478990a9c084b19d6f\t0123456789abcdef\n",
            "1\tCLOSED\ttry/old\tb907fbfa529ed2c9dc1133728fb94ca7f9b04b3f\t-\n",
        );
        let prs = parse_pr_list(text);
        assert_eq!(prs.len(), 3);
        assert_eq!(prs[0].number, 4);
        assert_eq!(prs[0].state, ForgePrState::Merged);
        assert_eq!(prs[0].branch, "feat/merge-me");
        assert_eq!(prs[0].digest, None);
        assert_eq!(prs[1].state, ForgePrState::Open);
        assert_eq!(prs[1].digest.as_deref(), Some("0123456789abcdef"));
        assert_eq!(prs[2].state, ForgePrState::Closed);
        // A line that is not ours is skipped, never guessed at.
        assert!(parse_pr_list("garbage\n").is_empty());
    }

    #[test]
    fn pr_number_from_url__reads_the_last_segment() {
        assert_eq!(
            pr_number_from_url("https://github.com/o/n/pull/42\n"),
            Some(42)
        );
        assert_eq!(pr_number_from_url("https://github.com/o/n/issues"), None);
    }

    #[test]
    fn default_branch_args__patch_the_repository() {
        assert_eq!(
            default_branch_args("o/n", "main"),
            ["api", "-X", "PATCH", "repos/o/n", "-f", "default_branch=main"]
        );
    }

    #[test]
    fn fake_forge__records_the_new_calls_and_can_fail_on_one() {
        let mut forge = FakeForge::new(RemoteState::HasContent, None);
        assert!(forge.heads.contains_key("refs/heads/main"));
        let dir = Path::new("/nowhere");
        assert_eq!(
            forge
                .push_ref(dir, "o/n", "step-001-a", "refs/heads/main", Some("x"))
                .unwrap(),
            "sha:step-001-a"
        );
        assert_eq!(forge.open_pull_request("o/n", "b", "T", "B").unwrap(), 101);
        assert_eq!(forge.open_pull_request("o/n", "c", "T", "B").unwrap(), 102);
        assert_eq!(
            forge.calls(),
            [
                "push_ref step-001-a -> refs/heads/main lease=x",
                "open_pr b",
                "open_pr c"
            ]
        );
        assert!(forge.mutated());

        forge.fail_on = Some("open_pr".to_string());
        assert!(forge.open_pull_request("o/n", "d", "T", "B").is_err());
    }
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p bower --lib forge`
Expected: FAIL to compile — `cannot find function push_ref_args`.

- [ ] **Step 3: Add the pure helpers**

In `bower/src/forge.rs`, add `use std::collections::BTreeMap;` and `use crate::schedule::{ForgePr, ForgePrState, MAIN};` to the imports, and add below `push_branch_args`:

```rust
/// The arguments that push commit `src` to the remote ref `dst`, leased
/// against `lease` — what the remote held when Bower last looked, or the
/// commit Bower itself pushed there a moment ago (EPIC-09 Decision 21).
///
/// `src` is a SHA, resolved in the built repository before the push, so a
/// tag name can never be misread as a branch. A ref that is not there yet
/// needs no force and has nothing to protect.
#[must_use]
pub fn push_ref_args(url: &str, src: &str, dst: &str, lease: Option<&str>) -> Vec<String> {
    let refspec = format!("{src}:{dst}");
    match lease {
        None => vec!["push".into(), url.to_string(), refspec],
        Some(sha) => vec![
            "push".into(),
            format!("--force-with-lease={dst}:{sha}"),
            url.to_string(),
            refspec,
        ],
    }
}

/// `git ls-remote --heads` output, as full ref → SHA. An empty repository
/// prints nothing, which is an empty map.
#[must_use]
pub fn parse_ls_remote(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter_map(|l| {
            let (sha, name) = l.split_once('\t')?;
            Some((name.trim().to_string(), sha.trim().to_string()))
        })
        .collect()
}

/// The `jq` program `gh pr list` runs: one tab-separated line per PR —
/// number, state, head branch, head SHA, and the `bower-pr:` digest or `-`.
/// A filter rather than JSON, because the base binary parses no JSON.
pub const PR_LIST_JQ: &str = r#".[] | [(.number|tostring), .state, .headRefName, .headRefOid, ((.body // "") | (capture("bower-pr: (?<d>[0-9a-f]+)").d // "-"))] | join("\t")"#;

/// The `gh` arguments that list every PR against main, in any state.
#[must_use]
pub fn pr_list_args(repo: &str) -> Vec<String> {
    [
        "pr", "list", "-R", repo, "--base", MAIN, "--state", "all", "--limit", "1000", "--json",
        "number,state,headRefName,headRefOid,body", "--jq", PR_LIST_JQ,
    ]
    .iter()
    .map(ToString::to_string)
    .collect()
}

/// Read [`PR_LIST_JQ`]'s lines. A line that does not have its shape is
/// skipped rather than guessed at.
#[must_use]
pub fn parse_pr_list(text: &str) -> Vec<ForgePr> {
    text.lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            let [number, state, branch, head, digest] = f.as_slice() else {
                return None;
            };
            let state = match *state {
                "OPEN" => ForgePrState::Open,
                "MERGED" => ForgePrState::Merged,
                "CLOSED" => ForgePrState::Closed,
                _ => return None,
            };
            Some(ForgePr {
                number: number.parse().ok()?,
                branch: (*branch).to_string(),
                state,
                head: (*head).to_string(),
                digest: (*digest != "-").then(|| (*digest).to_string()),
            })
        })
        .collect()
}

/// The PR number at the end of the URL `gh pr create` prints.
#[must_use]
pub fn pr_number_from_url(url: &str) -> Option<u64> {
    let url = url.trim();
    let (rest, number) = url.rsplit_once('/')?;
    rest.ends_with("/pull").then(|| number.parse().ok())?
}

/// The `gh` arguments that make `branch` the repository's default
/// (EPIC-09 Decision 26).
#[must_use]
pub fn default_branch_args(repo: &str, branch: &str) -> Vec<String> {
    vec![
        "api".into(),
        "-X".into(),
        "PATCH".into(),
        format!("repos/{repo}"),
        "-f".into(),
        format!("default_branch={branch}"),
    ]
}

/// Run `git -C dir args…`: success, stdout, stderr.
fn git_in(dir: &Path, args: &[String]) -> Result<(bool, String, String), ForgeError> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|e| ForgeError::Failed {
            what: format!("git {}", args.join(" ")),
            stderr: e.to_string(),
        })?;
    Ok((
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    ))
}
```

Run: `cargo test -p bower --lib forge` — the pure tests pass; `fake_forge__…` still fails to compile.

- [ ] **Step 4: Add the trait methods**

Add to `trait Forge`, after `push`:

```rust
    /// Every branch head on `owner/name`, as full ref → SHA. Empty for an
    /// empty repository. Read-only; `plan_push` calls it after the gate.
    ///
    /// # Errors
    ///
    /// If the remote cannot be read. An unread remote is never "no heads".
    fn remote_heads(&self, repo: &str) -> Result<BTreeMap<String, String>, ForgeError>;

    /// Every pull request against main, in any state (EPIC-09 Decision 22).
    /// Read-only; `plan_push` calls it after the gate.
    ///
    /// # Errors
    ///
    /// If the forge cannot be read.
    fn pull_requests(&self, repo: &str) -> Result<Vec<ForgePr>, ForgeError>;

    /// Push the commit `src` names in `dir` to the remote ref `dst`, leased
    /// against `lease`, or a plain push when `None`. Returns the SHA pushed,
    /// which the next push of the same ref leases against.
    ///
    /// # Errors
    ///
    /// If `src` does not resolve, or the push is refused — a lease trip
    /// included.
    fn push_ref(
        &self,
        dir: &Path,
        repo: &str,
        src: &str,
        dst: &str,
        lease: Option<&str>,
    ) -> Result<String, ForgeError>;

    /// Push every tag, forced. Returns how many tags `dir` holds.
    ///
    /// # Errors
    ///
    /// If the push is refused.
    fn push_tags(&self, dir: &Path, repo: &str) -> Result<usize, ForgeError>;

    /// Make `branch` the repository's default (EPIC-09 Decision 26). Only
    /// ever scheduled for a remote that had no main.
    ///
    /// # Errors
    ///
    /// If the forge refuses.
    fn set_default_branch(&self, repo: &str, branch: &str) -> Result<(), ForgeError>;

    /// Open a PR from `branch` into main. Returns its number.
    ///
    /// # Errors
    ///
    /// If the forge refuses — for instance, a branch with no commits ahead.
    fn open_pull_request(
        &self,
        repo: &str,
        branch: &str,
        title: &str,
        body: &str,
    ) -> Result<u64, ForgeError>;

    /// Set an open PR's title and body.
    ///
    /// # Errors
    ///
    /// If the forge refuses.
    fn edit_pull_request(
        &self,
        repo: &str,
        number: u64,
        title: &str,
        body: &str,
    ) -> Result<(), ForgeError>;
```

In `FakeForge`: add the fields

```rust
    /// The remote's branch heads, as `remote_heads` reports them.
    pub heads: BTreeMap<String, String>,
    /// The forge's PRs, as `pull_requests` reports them.
    pub prs: Vec<ForgePr>,
    /// When set, any call whose record starts with it fails — how a test
    /// makes one move of a schedule fail.
    pub fail_on: Option<String>,
    next_pr: std::cell::Cell<u64>,
```

initialised in both constructors — `heads` as `BTreeMap::from([(crate::replay::BRANCH.to_string(), "0".repeat(40))])` when `state == RemoteState::HasContent`, else empty; `prs: Vec::new()`, `fail_on: None`, `next_pr: std::cell::Cell::new(101)`. Add a helper and extend `mutated`:

```rust
    /// Record `what`, then fail if `fail_on` names it.
    fn call(&self, what: &str) -> Result<(), ForgeError> {
        self.record(what);
        match &self.fail_on {
            Some(prefix) if what.starts_with(prefix.as_str()) => Err(ForgeError::Failed {
                what: what.to_string(),
                stderr: "refused by FakeForge".to_string(),
            }),
            _ => Ok(()),
        }
    }
```

and in `mutated`: `c.starts_with("push") || c.starts_with("create") || c.starts_with("release") || c.starts_with("set_default_branch") || c.starts_with("open_pr") || c.starts_with("edit_pr")`.

`impl Forge for FakeForge`:

```rust
    fn remote_heads(&self, repo: &str) -> Result<BTreeMap<String, String>, ForgeError> {
        self.call("remote_heads")?;
        self.reachable(repo)?;
        Ok(self.heads.clone())
    }

    fn pull_requests(&self, repo: &str) -> Result<Vec<ForgePr>, ForgeError> {
        self.call("pull_requests")?;
        self.reachable(repo)?;
        Ok(self.prs.clone())
    }

    fn push_ref(
        &self,
        _dir: &Path,
        _repo: &str,
        src: &str,
        dst: &str,
        lease: Option<&str>,
    ) -> Result<String, ForgeError> {
        self.call(&format!(
            "push_ref {src} -> {dst} lease={}",
            lease.unwrap_or("-")
        ))?;
        Ok(format!("sha:{src}"))
    }

    fn push_tags(&self, _dir: &Path, _repo: &str) -> Result<usize, ForgeError> {
        self.call("push_tags")?;
        Ok(0)
    }

    fn set_default_branch(&self, _repo: &str, branch: &str) -> Result<(), ForgeError> {
        self.call(&format!("set_default_branch {branch}"))
    }

    fn open_pull_request(
        &self,
        _repo: &str,
        branch: &str,
        _title: &str,
        _body: &str,
    ) -> Result<u64, ForgeError> {
        self.call(&format!("open_pr {branch}"))?;
        let n = self.next_pr.get();
        self.next_pr.set(n + 1);
        Ok(n)
    }

    fn edit_pull_request(
        &self,
        _repo: &str,
        number: u64,
        _title: &str,
        _body: &str,
    ) -> Result<(), ForgeError> {
        self.call(&format!("edit_pr #{number}"))
    }
```

`impl Forge for GitHubForge` — the last inch, untested by design; every argument list comes from a tested pure function:

```rust
    fn remote_heads(&self, repo: &str) -> Result<BTreeMap<String, String>, ForgeError> {
        let out = std::process::Command::new("git")
            .args(["ls-remote", "--heads", &remote_url(repo)])
            .output()
            .map_err(|e| ForgeError::Failed {
                what: "git ls-remote --heads".to_string(),
                stderr: e.to_string(),
            })?;
        if !out.status.success() {
            return Err(ForgeError::Unreachable {
                repo: repo.to_string(),
                reason: String::from_utf8_lossy(&out.stderr).trim().to_string(),
            });
        }
        Ok(parse_ls_remote(&String::from_utf8_lossy(&out.stdout)))
    }

    fn pull_requests(&self, repo: &str) -> Result<Vec<ForgePr>, ForgeError> {
        let args = pr_list_args(repo);
        let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
        let (ok, body, stderr) = gh(&borrowed)?;
        if !ok {
            return Err(ForgeError::Unreachable {
                repo: repo.to_string(),
                reason: stderr.trim().to_string(),
            });
        }
        Ok(parse_pr_list(&body))
    }

    fn push_ref(
        &self,
        dir: &Path,
        repo: &str,
        src: &str,
        dst: &str,
        lease: Option<&str>,
    ) -> Result<String, ForgeError> {
        // Resolve first: the refspec carries a SHA, and the SHA is what the
        // next push of `dst` leases against.
        let (ok, stdout, stderr) =
            git_in(dir, &["rev-parse".into(), format!("{src}^{{commit}}")])?;
        if !ok {
            return Err(ForgeError::Failed {
                what: format!("git rev-parse {src}"),
                stderr: stderr.trim().to_string(),
            });
        }
        let sha = stdout.trim().to_string();
        let args = push_ref_args(&remote_url(repo), &sha, dst, lease);
        let (ok, _, stderr) = git_in(dir, &args)?;
        if !ok {
            return Err(ForgeError::Failed {
                what: format!("git push {src} → {dst}"),
                stderr: stderr.trim().to_string(),
            });
        }
        Ok(sha)
    }

    fn push_tags(&self, dir: &Path, repo: &str) -> Result<usize, ForgeError> {
        // Plainly forced: a replay recreates every tag with the same name and
        // a new SHA, and the gate has already said this remote is ours.
        let args: Vec<String> = vec![
            "push".into(),
            "--force".into(),
            remote_url(repo),
            "--tags".into(),
        ];
        let (ok, _, stderr) = git_in(dir, &args)?;
        if !ok {
            return Err(ForgeError::Failed {
                what: "git push --force --tags".to_string(),
                stderr: stderr.trim().to_string(),
            });
        }
        Ok(count(dir, &["tag", "--list"]))
    }

    fn set_default_branch(&self, repo: &str, branch: &str) -> Result<(), ForgeError> {
        let args = default_branch_args(repo, branch);
        let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
        let (ok, _, stderr) = gh(&borrowed)?;
        if ok {
            Ok(())
        } else {
            Err(ForgeError::Failed {
                what: format!("gh api -X PATCH repos/{repo} default_branch={branch}"),
                stderr: stderr.trim().to_string(),
            })
        }
    }

    fn open_pull_request(
        &self,
        repo: &str,
        branch: &str,
        title: &str,
        body: &str,
    ) -> Result<u64, ForgeError> {
        // `--flag=value`, so a body that starts with `---` (Decision 23's
        // footer, when the book gives no description) is never read as a flag.
        let (title, body) = (format!("--title={title}"), format!("--body={body}"));
        let (ok, stdout, stderr) = gh(&[
            "pr", "create", "-R", repo, "--base", MAIN, "--head", branch, &title, &body,
        ])?;
        if !ok {
            return Err(ForgeError::Failed {
                what: format!("gh pr create --head {branch}"),
                stderr: stderr.trim().to_string(),
            });
        }
        pr_number_from_url(&stdout).ok_or_else(|| ForgeError::Failed {
            what: format!("gh pr create --head {branch}"),
            stderr: format!("no PR number in `{}`", stdout.trim()),
        })
    }

    fn edit_pull_request(
        &self,
        repo: &str,
        number: u64,
        title: &str,
        body: &str,
    ) -> Result<(), ForgeError> {
        let n = number.to_string();
        let (title, body) = (format!("--title={title}"), format!("--body={body}"));
        let (ok, _, stderr) = gh(&["pr", "edit", &n, "-R", repo, &title, &body])?;
        if ok {
            Ok(())
        } else {
            Err(ForgeError::Failed {
                what: format!("gh pr edit #{number}"),
                stderr: stderr.trim().to_string(),
            })
        }
    }
```

- [ ] **Step 5: Run the tests**

Run: `cargo test -p bower --lib forge`
Expected: PASS — the eight new tests and every existing one (`Forge::push` and `push_commands` still exist, untouched).

- [ ] **Step 6: Gate**

Run: `cargo fmt --all --check && cargo test --workspace && cargo clippy --workspace --all-targets --all-features -- -D warnings && make docs`
Expected: all green. If rustfmt wraps the long argument arrays, run `cargo fmt --all` and re-run the gate.

- [ ] **Step 7: Commit**

```bash
git add bower/src/forge.rs
git commit -m "feat(forge): read heads and PRs, push one ref, open and edit PRs, set the default branch"
```

---

### Task 3: `execute`, and the push wired to the schedule (EPIC Work Items 3d–3f; Decision 21)

**Files:**
- Modify: `bower/src/push.rs` (`PushPlan::Ready`, `plan_push`, `execute`, `ExecuteError`, `schedule_lines`, `local_branch_heads`, tests)
- Modify: `bower/src/main.rs` (`report_push`)
- Modify: `bower/src/forge.rs` (remove `Forge::push`, both impls, `push_commands`, `push_branch_args`, `remote_head`, `FakeForge::pushed_branches` and its field, and their tests)
- Modify: `bower/tests/push.rs` (the no-override test)

**Interfaces:**
- Consumes: `schedule::{Schedule, Move, schedule, MAIN}` (Task 1); the seven `Forge` methods (Task 2).
- Produces:
  - `PushPlan::Ready { repo, remote, branch, tags, schedule: Schedule, remote_heads: BTreeMap<String, String>, create, site, release }` — `branches` and `main_first` are gone; the schedule carries both.
  - `pub fn execute(forge: &dyn Forge, dir: &Path, remote: &str, schedule: &Schedule, remote_heads: &BTreeMap<String, String>) -> Result<Vec<String>, ExecuteError>` — one line per move done.
  - `pub struct ExecuteError { pub done: Vec<String>, pub error: ForgeError }` with `Display`.
  - `pub fn schedule_lines(schedule: &Schedule) -> Vec<String>` — the dry-run block; empty for a book without branches.

- [ ] **Step 1: Write the failing tests**

In `bower/src/push.rs`, inside `mod plan_tests`, add `use crate::schedule::{ForgePr, ForgePrState, Move, Schedule, schedule};` and `use std::collections::BTreeMap;`, then:

```rust
    /// A schedule with a stepping stone: two branches, one merged.
    fn stone_schedule() -> Schedule {
        let f = bower_testkit::fixtures::branch_saga();
        let p = resolve(&f.book, &f.catalog).unwrap().repos.remove(0);
        let heads = BTreeMap::from([(BRANCH.to_string(), "old".to_string())]);
        schedule(&p, &heads, &BTreeMap::new(), &[], "book")
    }

    #[test]
    fn execute__runs_moves_in_order_and_chains_main_leases() {
        let forge = FakeForge::new(RemoteState::HasContent, None);
        let s = stone_schedule();
        let heads = BTreeMap::from([(BRANCH.to_string(), "old".to_string())]);
        let done = execute(&forge, Path::new("/built"), REMOTE, &s, &heads).unwrap();
        assert_eq!(
            forge.calls(),
            [
                "push_ref refs/heads/try/lookup-table -> refs/heads/try/lookup-table lease=-",
                "push_ref refs/heads/from-char -> refs/heads/from-char lease=-",
                // The first push of main leases against what the remote held …
                "push_ref step-003-lib-doc -> refs/heads/main lease=old",
                "open_pr from-char",
                // … every later one against the stone Bower just pushed.
                "push_ref step-006-merge-from-char -> refs/heads/main lease=sha:step-003-lib-doc",
                "open_pr try/lookup-table",
                "push_tags",
            ]
        );
        assert_eq!(done.len(), 7);
        assert!(done[3].ends_with("→ #101"), "{done:?}");
    }

    #[test]
    fn execute__stops_at_the_first_failure_and_names_what_went_out() {
        let mut forge = FakeForge::new(RemoteState::HasContent, None);
        forge.fail_on = Some("push_ref refs/heads/from-char".to_string());
        let heads = BTreeMap::from([(BRANCH.to_string(), "old".to_string())]);
        let err = execute(&forge, Path::new("/built"), REMOTE, &stone_schedule(), &heads).unwrap_err();
        assert_eq!(err.done, ["branch try/lookup-table"]);
        assert!(err.to_string().contains("already done: branch try/lookup-table"), "{err}");
        assert!(!forge.calls().iter().any(|c| c.contains("refs/heads/main")), "{:?}", forge.calls());
    }

    #[test]
    fn execute__a_failure_after_a_stone_puts_main_back_on_its_head() {
        // A stone has no STEPS.md: a remote left on one fails the gate forever.
        let mut forge = FakeForge::new(RemoteState::HasContent, None);
        forge.fail_on = Some("open_pr from-char".to_string());
        let heads = BTreeMap::from([(BRANCH.to_string(), "old".to_string())]);
        let err = execute(&forge, Path::new("/built"), REMOTE, &stone_schedule(), &heads).unwrap_err();
        assert_eq!(
            forge.calls().last().unwrap(),
            "push_ref step-006-merge-from-char -> refs/heads/main lease=sha:step-003-lib-doc"
        );
        assert!(err.to_string().contains("main put back on step-006-merge-from-char"), "{err}");
    }

    #[test]
    fn schedule_lines__are_empty_for_a_straight_line() {
        let p = one_step_plan();
        let heads = BTreeMap::from([(BRANCH.to_string(), "old".to_string())]);
        assert!(schedule_lines(&schedule(&p, &heads, &BTreeMap::new(), &[], "book")).is_empty());
    }

    #[test]
    fn schedule_lines__number_every_move_and_name_every_pr() {
        let lines = schedule_lines(&stone_schedule());
        assert_eq!(lines[0], "  branches  try/lookup-table, from-char");
        assert_eq!(lines[1], "  schedule  1. branch try/lookup-table");
        assert_eq!(
            lines[3],
            "            3. main → step-003-lib-doc (stepping stone for from-char)"
        );
        assert!(
            lines.contains(&"  pr        from-char — create, on the stepping stone step-003-lib-doc".to_string()),
            "{lines:#?}"
        );
    }

    #[test]
    fn push__gate_refusal_reads_and_creates_no_pr() {
        let forge = FakeForge::new(RemoteState::HasContent, None);
        let root = book_root("pr-refused");
        let p = branch_plan_with_pr();
        let cfg = config(Some(REMOTE));
        let dir = built("pr-refused-repo", &p, &root, &cfg);

        let got = plan_push(&forge, &cfg, FP, &p, &dir, Path::new("/no-site"), &root).unwrap();
        assert!(matches!(got, PushPlan::Blocked { .. }), "{got:?}");
        assert!(!forge.calls().iter().any(|c| c == "pull_requests" || c == "remote_heads"), "{:?}", forge.calls());
        assert!(!forge.mutated(), "{:?}", forge.calls());
    }

    #[test]
    fn plan__reads_heads_and_prs_after_the_gate_and_carries_the_schedule() {
        let mut forge = FakeForge::new(RemoteState::HasContent, Some(ours()));
        forge.prs = vec![ForgePr {
            number: 5,
            branch: "try/side".to_string(),
            state: ForgePrState::Merged,
            head: "0".to_string(),
            digest: None,
        }];
        let root = book_root("pr-ready");
        let p = branch_plan_with_pr();
        let cfg = config(Some(REMOTE));
        let dir = built("pr-ready-repo", &p, &root, &cfg);

        let got = plan_push(&forge, &cfg, FP, &p, &dir, Path::new("/no-site"), &root).unwrap();
        let PushPlan::Ready { schedule, .. } = got else {
            panic!("expected Ready, got {got:?}");
        };
        assert_eq!(schedule.branches(), ["try/side"]);
        assert!(!schedule.moves.contains(&Move::DefaultMain), "main exists already");
        assert_eq!(schedule.prs[0].1.to_string(), "merged #5, left alone");
        assert!(!forge.mutated(), "{:?}", forge.calls());
    }
```

Add the helper beside `branch_plan()`:

```rust
    /// `branch_plan()`, with a PR declared for its branch.
    fn branch_plan_with_pr() -> RepoPlan {
        let text = concat!(
            "<!-- bower repo=\"r\" step=\"base\" file=\"src/lib.rs\" -->\n```rust\npub fn f() {}\n```\n",
            "<!-- bower repo=\"r\" step=\"side\" branch=\"try/side\" file=\"src/side.rs\" pr=\"A side\" -->\n```rust\npub fn g() {}\n```\n",
        );
        let book = BookSource::from_chapters(vec![Chapter::new("src/ch01.md", text)]);
        resolve(&book, &RepoCatalog::from_names(&["r"]))
            .unwrap()
            .repos
            .remove(0)
    }
```

Rewrite the existing tests that read the removed fields, keeping their names and intent:
- `plan__lists_every_branch`: destructure `PushPlan::Ready { schedule, .. }`; `assert_eq!(schedule.branches(), ["try/side"]);`.
- `push__gate_refusal_pushes_no_branch`: replace the `pushed_branches()` assertion with `assert!(!forge.calls().iter().any(|c| c.starts_with("push_ref")), "{:?}", forge.calls());`.
- The fresh-remote test that asserted `branches.is_empty()` and `main_first`: `assert!(schedule.branches().is_empty());` and `assert!(schedule.moves.contains(&Move::DefaultMain), "an absent remote has no main yet");`.
- The our-own-remote test that asserted `!main_first`: `assert!(!schedule.moves.contains(&Move::DefaultMain), "main already exists on this remote: {schedule:?}");`.
- `plan__an_empty_remote_also_needs_main_first`: `assert!(schedule.moves.contains(&Move::DefaultMain), "main is still absent from it: {schedule:?}");`.

In `bower/tests/push.rs` `dry_run_is_the_default_and_no_flag_overrides_the_guard`, extend the list: `for escape in ["--force", "--override", "--skip", "--no-verify", "--yes", "--merge-pr", "--close-pr"]`, and rename nothing. Add a sibling so the EPIC's test name exists:

```rust
#[test]
fn push__has_no_merge_or_close_flag() {
    // EPIC-09 Decision 22: Bower never merges or closes a PR on the forge,
    // and offers no way to.
    let out = bower(&["push", "--help"]);
    let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
    for flag in ["--merge-pr", "--close-pr", "--merge", "--close"] {
        assert!(!text.contains(flag), "`{flag}`: {text}");
    }
}
```

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p bower`
Expected: FAIL to compile — `cannot find function execute`, `no field schedule on PushPlan::Ready`.

- [ ] **Step 3: Carry the schedule in the plan**

In `bower/src/push.rs`: add `use std::collections::BTreeMap;` and `use crate::schedule::{Move, Schedule, schedule};` (plus `MAIN` if used), drop `expected_branches` from the `replay` import if unused.

In `PushPlan::Ready`, replace `branches: Vec<String>` and `main_first: bool` (and their doc comments) with:

```rust
        /// Everything the push will do, in order (EPIC-09 Decision 21): the
        /// dry run prints it and `execute` carries it out.
        schedule: Schedule,
        /// What the remote's branches pointed at when planned — the first
        /// lease of every ref `execute` pushes.
        remote_heads: BTreeMap<String, String>,
```

If clippy's `large_enum_variant` fires, box `schedule` (`Box<Schedule>`) as `release` already is, and record it as a deviation.

In `plan_push`, after the release decision and before `Ok(PushPlan::Ready { … })`:

```rust
    // Read only now, after the gate: a remote that is not ours has nothing
    // of its PRs read, let alone written (Decision 21).
    let remote_heads = if state == RemoteState::Absent {
        BTreeMap::new()
    } else {
        forge.remote_heads(&remote).map_err(PushError::Forge)?
    };
    let on_forge = if state == RemoteState::HasContent && plan.branches.iter().any(|b| b.pr.is_some()) {
        forge.pull_requests(&remote).map_err(PushError::Forge)?
    } else {
        Vec::new()
    };
    let schedule = schedule(
        plan,
        &remote_heads,
        &local_branch_heads(dir, plan),
        &on_forge,
        &book,
    );
```

and in the `Ready` literal replace `branches: …` and `main_first: …` with `schedule,` and `remote_heads,`.

Add below `plan_push`:

```rust
/// The built repository's branch heads, read as loose refs — how a replay
/// leaves them (`status` reads them the same way). A branch whose ref is
/// missing is simply absent from the map.
fn local_branch_heads(dir: &Path, plan: &RepoPlan) -> BTreeMap<String, String> {
    plan.branches
        .iter()
        .filter_map(|b| {
            let path = dir.join(".git").join("refs").join("heads").join(&b.name);
            let sha = std::fs::read_to_string(path).ok()?;
            Some((b.name.clone(), sha.trim().to_string()))
        })
        .collect()
}
```

- [ ] **Step 4: `execute` and the dry-run lines**

Add to `bower/src/push.rs`:

```rust
/// A push that stopped partway: what went out, and why it stopped.
#[derive(Debug)]
pub struct ExecuteError {
    pub done: Vec<String>,
    pub error: ForgeError,
}

impl fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.error)?;
        if !self.done.is_empty() {
            write!(f, "\n  already done: {}", self.done.join("; "))?;
        }
        Ok(())
    }
}

impl std::error::Error for ExecuteError {}

/// Carry out a schedule, move by move (EPIC-09 Decision 21).
///
/// Every ref is leased: a branch against what the remote held when planned,
/// main against that for its first push and then against the commit Bower
/// itself pushed there. It stops at the first failure and names what went
/// out — and if main is standing on a stepping stone when it stops, it first
/// moves main back to its head. A stone is an older commit with no
/// `STEPS.md`, and a remote left there would fail the marker gate on every
/// later push.
///
/// # Errors
///
/// [`ExecuteError`] carrying the moves already done and the forge's error.
pub fn execute(
    forge: &dyn Forge,
    dir: &Path,
    remote: &str,
    schedule: &Schedule,
    remote_heads: &BTreeMap<String, String>,
) -> Result<Vec<String>, ExecuteError> {
    let mut done = Vec::new();
    let mut main_lease: Option<String> = remote_heads.get(BRANCH).cloned();
    let mut main_at: Option<String> = None;

    for mv in &schedule.moves {
        let result = match mv {
            Move::Branch { name } => {
                let full = format!("refs/heads/{name}");
                forge
                    .push_ref(dir, remote, &full, &full, remote_heads.get(&full).map(String::as_str))
                    .map(|_| mv.to_string())
            }
            Move::Main { at, .. } => forge
                .push_ref(dir, remote, at, BRANCH, main_lease.as_deref())
                .map(|sha| {
                    main_lease = Some(sha);
                    main_at = Some(at.clone());
                    mv.to_string()
                }),
            Move::DefaultMain => forge
                .set_default_branch(remote, crate::schedule::MAIN)
                .map(|()| mv.to_string()),
            Move::OpenPr {
                branch,
                title,
                body,
            } => forge
                .open_pull_request(remote, branch, title, body)
                .map(|n| format!("{mv} → #{n}")),
            Move::EditPr {
                number,
                title,
                body,
                ..
            } => forge
                .edit_pull_request(remote, *number, title, body)
                .map(|()| mv.to_string()),
            Move::Tags => forge
                .push_tags(dir, remote)
                .map(|n| format!("tags — {n}")),
        };
        match result {
            Ok(line) => done.push(line),
            Err(error) => {
                // Stranded on a stone: put main back on its head first.
                if let (Some(at), Some(head)) = (&main_at, &schedule.head)
                    && at != head
                    && forge
                        .push_ref(dir, remote, head, BRANCH, main_lease.as_deref())
                        .is_ok()
                {
                    done.push(format!("main put back on {head}"));
                }
                return Err(ExecuteError { done, error });
            }
        }
    }
    Ok(done)
}

/// The dry run's schedule block (EPIC-09 exit criterion 6): the branches, the
/// numbered moves, and one line per declared PR. Empty for a book without
/// branches, whose report is unchanged (Decision 13).
#[must_use]
pub fn schedule_lines(schedule: &Schedule) -> Vec<String> {
    let branches = schedule.branches();
    if branches.is_empty() {
        return Vec::new();
    }
    let mut out = vec![format!("  branches  {}", branches.join(", "))];
    for (i, mv) in schedule.moves.iter().enumerate() {
        let lead = if i == 0 { "  schedule  " } else { "            " };
        out.push(format!("{lead}{}. {mv}", i + 1));
    }
    for (branch, action) in &schedule.prs {
        out.push(format!("  pr        {branch} — {action}"));
    }
    out
}
```

- [ ] **Step 5: The report**

In `bower/src/main.rs`, import `use bower::push::{execute, schedule_lines};` (beside `PushPlan`, `plan_push`). In `report_push`'s `PushPlan::Ready { … }` pattern replace `branches, main_first,` with `schedule, remote_heads,`; replace the `if !branches.is_empty() { … }` block with:

```rust
            // Only when there are branches: a straight line's report is
            // unchanged (Decision 13).
            for line in schedule_lines(&schedule) {
                println!("{line}");
            }
```

and replace the `match forge.push(dir, &remote, &branch, &branches) { … }` block with:

```rust
            match execute(forge, dir, &remote, &schedule, &remote_heads) {
                Ok(done) => {
                    for line in done {
                        println!("  pushed    {line}");
                    }
                }
                Err(e) => {
                    eprintln!("bower: {e}");
                    return false;
                }
            }
```

- [ ] **Step 6: Remove what nothing calls any more**

In `bower/src/forge.rs`, delete: `Forge::push` from the trait and from both impls; `push_commands`; `push_branch_args`; `remote_head`; `FakeForge::pushed_branches` (method and field); and the tests of those (`push__branches_go_before_main`, `push_commands__…`, `push_branch_args__…`). The lease rules those tests pinned are pinned by Task 2's `push_ref_args__…` tests and Step 1's `execute__…` tests. Update doc comments that mention `Forge::push` or `push_commands` to name `push::execute` and `schedule::schedule`. Keep `count` (`push_tags` uses it) and `PushOutcome` (`push_tree` uses it).

The EPIC's slice-1 test name `push__branches_go_before_main` goes with `push_commands`; its guarantee now lives in `schedule__an_existing_remote_pushes_branches_then_prs_then_main_then_tags` — Task 4 notes it in the corrigendum.

- [ ] **Step 7: Run the tests**

Run: `cargo test -p bower`
Expected: PASS.

- [ ] **Step 8: Gate**

Run: `cargo fmt --all --check && cargo test --workspace && cargo clippy --workspace --all-targets --all-features -- -D warnings && make docs`
Expected: all green.

- [ ] **Step 9: Commit**

```bash
git add bower/src/push.rs bower/src/main.rs bower/src/forge.rs bower/tests/push.rs
git commit -m "feat(push): plan and execute the schedule — stepping stones, PRs, main put back if a push stops on a stone"
```

---

### Task 4: Docs, the EPIC, and the live run (EPIC Work Item 3g; exit criterion 9)

**Files:**
- Modify: `docs/EPIC-09_Branches.md`, `.okf/model/pull-request.md`, `BACKLOG.md`, `docs/TECHNICAL_DEBT.md`, `.okf/log.md`

- [ ] **Step 1: The docs**

- `docs/EPIC-09_Branches.md`: set the slice-2 `push` Status row to **Done**; tick Work Items 3c–3f (3g after the live run); add a `## Corrigendum — as built, slice 2` section after slice 1's, listing each deviation the tasks reported (file and why) plus: "`push__branches_go_before_main` tested `push_commands`, which slice 2 removed; its guarantee — branches before main — is now `schedule__an_existing_remote_pushes_branches_then_prs_then_main_then_tags`." Update the Summary's Status line to "Slice 2 built on `feat/branches-slice-2`, 14 September 2026; the live run (exit criterion 9) is the last step."
- `.okf/model/pull-request.md`: replace the "Not yet" section with **On the forge**: `bower push` opens a PR per declared branch through a schedule the dry run prints; a merged branch's PR is opened on a stepping stone and merged by the push of main; open PRs are updated when their digest differs; merged and closed PRs are left alone; Bower never merges, closes, reopens, or deletes a PR; the body's footer and `bower-pr:` digest (Decision 23). Check each statement against `bower/src/schedule.rs`.
- `docs/TECHNICAL_DEBT.md`: close (strike through, in the file's style, with the date and the reason) "`report_push`'s forwarding of branches to `Forge::push` is covered only by the compiler" — `execute` runs a real schedule through `FakeForge` in tests now. Add, as tracked debt: "The push gate reads `STEPS.md` from the repository's default branch; Decision 26 now makes that main on a fresh remote, but a repository whose owner chose another default is still read there" (if the existing entry says this, update it instead of adding one).
- `BACKLOG.md`: the Branches row — slice 2 built, live run pending; `.okf/log.md` a `2026-09-14` entry.

Run the `okf:validate` skill over `.okf/` (or its checker script) and fix what it reports.

- [ ] **Step 2: Gate**

Run: `cargo fmt --all --check && make ayce && make slow`
Expected: green.

- [ ] **Step 3: Commit**

```bash
git add docs/EPIC-09_Branches.md .okf BACKLOG.md docs/TECHNICAL_DEBT.md
git commit -m "docs: EPIC-09 slice 2 built — the push schedule, stepping stones, PRs on the forge"
```

- [ ] **Step 4: The live run — the user's** (exit criterion 9)

The controller hands the user these commands and reads the results back with read-only `gh` calls:

```bash
make ship-hello                       # dry run: read the schedule block
make ship-hello-execute               # the real push
gh pr list -R abstecker/hello-playbook --state all
make ship-hello                       # second dry run
```

Expected:
- The first dry run's schedule for `abstecker/hello-playbook` (main exists; no PRs yet): `branch try/shout`, `branch feat/greet-many`, `main → step-024-changelog (stepping stone for feat/greet-many)`, `open PR feat/greet-many "…"`, `main → step-025-merge-greet-many`, `open PR try/shout "…"`, `tags`.
- After the execute: `try/shout`'s PR OPEN; `feat/greet-many`'s PR MERGED (allow GitHub ~10 s), its merge commit Bower's; `main` at `step-025`; `STEPS.md` on main.
- The second dry run: `pr try/shout — unchanged #…`, `pr feat/greet-many — merged #…, left alone`; no `open PR` move.

Then tick Work Item 3g and flip the Status line to "Slice 2 shipped", and give the user the commit for that one-line change.

---

## Exit criteria (from the EPIC), and the task that proves each

| # | Criterion | Proved by |
|---|---|---|
| 6 (slice 2) | The dry run shows the numbered schedule, every PR and its action | Task 3 `schedule_lines__number_every_move_and_name_every_pr`; Task 4 Step 4 |
| 9 | `--execute` leaves `try/shout`'s PR open and `feat/greet-many`'s merged, opened on a stone; a second push opens nothing | Task 4 Step 4 |
| — | A refused gate reads and writes no PR | Task 3 `push__gate_refusal_reads_and_creates_no_pr` |
| — | No flag merges or closes a PR | Task 3 `push__has_no_merge_or_close_flag` |
| — | A book without branches pushes and reports as before | Task 1 `schedule__a_straight_line_is_main_then_tags`; Task 3 `schedule_lines__are_empty_for_a_straight_line` |
