---
type: Domain Concept
title: Step
description: One commit-to-be in one target repository, grouping one or more annotated blocks, carrying its own complete tree state.
tags: [model, kernel, git]
timestamp: '2026-09-09T00:00:00Z'
---

# What it is

A step is **one commit-to-be in one target repo**. It groups one or more
annotated blocks. It is the unit the book teaches in, the unit git commits, and
the unit tags point at.

The public type is `PlannedStep` (the internal `Step` is not exported):

| Field | Meaning |
|---|---|
| `seq` | 1-based position within its repo's [plan](/model/plan.md). |
| `id` | Stable step id — explicit from `step=`, or derived. |
| `msg` | Commit message subject. |
| `expect` | The [expectation](/model/expectation.md) verification will check. |
| `anchor` | The [`Location`](/model/book.md) in the chapter that produced it. |
| `files` | Paths this step touches, deduped in first-touch order. |
| `tree` | The complete [`TreeState`](/model/tree-state.md) **after** this step. |
| `displays` | Per-block [display](/model/display-markers.md) information. |
| `play_cells` | Bound [play cells](/model/play-cell.md). |
| `exercise` | `Option<Exercise>` — at most one. See [exercises](/model/exercise.md). |

Note that `tree` is the *whole* tree, not a diff. Every step carries a complete,
materializable file tree, which is what makes verification of any single step
possible without replaying its predecessors.

# Grouping

Blocks sharing `(repo, explicit step id)` merge into one step; the group keeps
the document position of its first appearance. Blocks with no `step=` each
become their own single-block step.

Within one step, whole-file ops (`create`, `replace`, `delete`, `copy`) may
never share a file with anything else, and two `region` ops may not target the
same region — that is `DuplicateFileInStep`. Distinct regions and appends
compose fine.

# Auto ids

When no `step=` is given, the id is `slugify(chapter_stem)-slugify(base)`, where
`base` is the first available of `msg`, the nearest heading, `file`, or the
literal `"step"`. Collisions get a numeric suffix:

```
ch01-ranks
ch01-ranks-2
ch01-ranks-3
```

Slugify lowercases ASCII alphanumerics and collapses everything else to single
dashes, trimmed; an empty result becomes `"step"`.

# Derived commit subjects

With no `msg=`, the subject is built from the chapter stem:

```
{chapter_stem}: {heading}      e.g.  ch03-ranks: Introduce the Rank enum
{chapter_stem}: {file}         when there is no heading
{chapter_stem}: step           when there is neither
```

# Tags — the stable link into the repo

SHAs change whenever an earlier step changes, so the book never links to a SHA.
Every step gets an annotated tag, formatted in exactly one place
(`bower-core/src/plan.rs`):

```rust
format!("step-{:03}-{}", self.seq, self.id)   // step-012-rank-enum
```

Each chapter boundary additionally gets a `<chapter-stem>-end` tag. Tags are
recreated on every regeneration; their names are stable for as long as step ids
are. This is why renaming a step id is a breaking change for any published book.

# Citations

[1] `bower-core/src/step.rs`, `bower-core/src/plan.rs`
[2] [`bower-spec.md` §2.1, §3.5, §5.3](/references/bower-spec.md)
