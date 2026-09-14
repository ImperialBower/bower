---
type: Domain Concept
title: Pull request
description: A plan value first, a forge artifact second — a branch's declared PR, deterministic in the repo whether or not a forge is involved.
tags: [model, kernel, github]
timestamp: '2026-09-14T00:00:00Z'
---

# What it is

A pull request belongs to a [branch](/model/branch.md), not a step. The plan
carries `PullRequest { loc, branch, title, body, state }`, with `state`
derived rather than authored: `PrState::Merged` once the branch has a merge
step, `PrState::Open` otherwise (EPIC-09 Decision 8). A book that never
declares `pr=` gets no `PullRequest` values and nothing new prints anywhere —
a plan value first means the PR exists and has a state before any forge is
asked about it.

# Two forms

The key form rides on a branch step's own directive, with no body:

```markdown
<!-- bower repo="failers" step="try-lookup" branch="try/lookup-table" file="src/rank.rs" pr="Try a lookup table for ranks" -->
```rust
// a lookup table instead of a match
```
```

The block form is its own directive, carrying `repo=`, `branch=`, and `pr=`,
followed by a fenced markdown body that becomes the description — bound by
its `branch=` name, not through the step index that locates ordinary blocks,
because that would find a step and then have to check its line against the
block's `branch=` separately. A block naming a branch no step is on is
`PrUnknownBranch`; a second `pr=` for one branch, in either form, is
`PrDuplicate`; `pr=` on a step that is not on a branch is `PrWithoutBranch`.

# Where it lives

`bower.lock` prints a `[<repo>.branches]` table only for a repo that has a
branch, one row per branch: its fork point, its head, whether and where it
merged, and its PR's title and state when it declares one. The same
information becomes `PULLS.md` in the generated repository — a table, then
each PR's title, branch, state, and description — deterministic whether or
not a forge is ever involved, and generated only for a repo that declares at
least one PR.

# On the forge

`bower push` opens a PR per declared branch through a schedule the dry run
prints — `schedule(plan, remote heads, local heads, forge PRs, book)`
(`bower/src/schedule.rs`, EPIC-09 Decisions 20–22). For each declared PR it
looks up the forge's PRs by head branch, in any state, and decides `Created`,
`Updated`, `Unchanged`, `LeftMerged`, `LeftClosed`, or `NotOpened`. Bower
only ever acts on PRs from branches of the repository itself: a PR from a
fork is filtered out when the list is read (`isCrossRepository`), so a
reader's fork PR from a branch that happens to share the book's branch name is
never edited, and a closed one never blocks the book's own PR. A branch
the book has already merged has no commits ahead of main by the time a first
publish reaches it, so its PR is opened on a stepping stone: main moves back
to the merge's main parent just long enough to open the PR, then moves on;
the forge marks the PR merged once main passes the merge again (Decision 20).
Stones go in merge order.

The book's branches and main's first move go out in one atomic `git push`
(`Forge::push_refs`): every ref or none. After a rebuild that changes every
SHA, an open PR survives only when its branch and main move together — the
`pr-remote` spike's Q2b — while a force-push of main alone to a history the
PR's head shares no commits with makes GitHub close it. The dry run says so
under the numbered schedule: `(moves 1–<k> go out as one atomic push)`.

An open PR is updated when its title or description differs from what the
book now declares, and left `Unchanged` when it does not. A merged or closed
PR is left alone outright — Bower never merges, closes, reopens, or deletes a
PR, and has no flag to (Decision 22); a merged PR whose head has since moved
is reported, not repaired.

A PR's body is the book's description, then a footer — "Opened by Bower from
the book *\<name\>*. It is merged by a push to main, never on the forge." —
and an HTML comment carrying an FNV digest of the title and description,
`<!-- bower-pr: <digest> -->`. `Updated` versus `Unchanged` compares digests,
read back with `gh pr list --jq`, rather than bodies: GitHub may rewrite a
body's line endings, and the base binary parses no JSON outside the
preprocessor. A body with no digest — one a person wrote, or edited away —
always reads as different (Decision 23).

# Citations

[1] `bower-core/src/branch.rs`, `bower/src/trailers.rs`, `bower/src/schedule.rs`, `bower/src/push.rs`, `bower/src/forge.rs`
[2] `docs/EPIC-09_Branches.md`
