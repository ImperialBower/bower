---
type: Domain Concept
title: Ordering
description: Steps run in document order by default; `after=` bends it through a stable topological sort, and cycles are a plan-time error.
tags: [model, kernel]
timestamp: '2026-09-09T00:00:00Z'
---

# Default: document order

Books are read front to back, so steps are sequenced by chapter order in
`SUMMARY.md`, then block position within the chapter, filtered per repo. The
common case is zero-config.

# The exception: `after=`

An explicit `step` id plus `after="step-id"` handles the cases document order
cannot express — a chapter that interleaves two repos, an appendix that patches
an early step.

`order()` sorts by document position, then runs a **stable topological sort**:
among the steps that are ready, the one earliest in the book goes first.

That stability is the point. Adding a single `after` constraint never reshuffles
unrelated steps, so a one-line authoring change produces a one-line
[`bower.lock`](/config/bower-lock.md) diff rather than a renumbering of the
whole book.

# Failure modes

| Error | Cause |
|---|---|
| `OrphanAfter` | `after=` names a step id that does not exist in that repo. |
| `OrderingCycle` | The constraints form a cycle; no valid order exists. |

`OrderingCycle` is the one [error](/model/errors.md) with **no** `Location` —
a cycle is a property of a set of steps, not of a line — so it carries the repo
name and the ids in the cycle instead.

# Why the lock file exists

The resolved order is written to [`bower.lock`](/config/bower-lock.md), a
human-readable manifest of (seq, id, expect, anchor, files) per repo. Reordering
therefore shows up in diffs and code review, rather than as a silent history
rewrite on the next `bower build`.

# Citations

[1] `bower-core/src/step.rs`
[2] [`bower-spec.md` §4](/references/bower-spec.md)
