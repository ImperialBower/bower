---
type: Domain Concept
title: Pull request
description: A plan value first, a forge artifact second — a branch's declared PR, deterministic in the repo whether or not a forge is involved.
tags: [model, kernel, github, roadmap]
timestamp: '2026-09-13T00:00:00Z'
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

# Not yet

Slice 1 stops at the plan value and `PULLS.md`. Ensuring the PR actually
exists on a forge — `Forge::ensure_pull_request`, creating or updating it
without ever closing or merging one Bower did not create — is EPIC-09 slice
2, which waits on open questions about merged-PR detection and regeneration
churn being answered against a real remote.

# Citations

[1] `bower-core/src/branch.rs`, `bower/src/trailers.rs`
[2] `docs/EPIC-09_Branches.md`
