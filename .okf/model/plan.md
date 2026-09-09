---
type: Domain Concept
title: Plan
description: The fully-resolved, ordered list of steps per repository — a pure value, and the kernel's single front door.
tags: [model, kernel]
timestamp: '2026-09-09T00:00:00Z'
---

# What it is

A plan is the fully-resolved, ordered list of [steps](/model/step.md) for a book
— **a pure value**. Given the same [`BookSource`](/model/book.md) and
`RepoCatalog`, `plan()` returns an identical `BookPlan`, down to every tree byte.

```rust
use bower_core::prelude::*;

let plan = plan(&book, &catalog)?;
let step = &plan.repo("failures").unwrap().steps[0];
assert_eq!(step.tag(), "step-001-ch01-hello");
```

# The types

- `BookPlan { repos: Vec<RepoPlan> }`, with a `.repo(name)` lookup. Repos appear
  in **catalog order**, and a repo the book never feeds is simply skipped rather
  than appearing empty.
- `RepoPlan { repo: RepoName, steps: Vec<PlannedStep> }`.

The top-level type is `BookPlan`, not `Plan` — the free function is `plan()`.

# What planning does

```
blocks ──group by (repo, step id)──▶ steps ──order──▶ fold each step's TreeState
```

1. Scan chapters fence-aware, pair each [directive](/model/directive.md) with its
   fence, resolve `include=` against the block library, validate required keys.
2. [Group](/model/step.md) blocks into steps.
3. [Order](/model/ordering.md) them.
4. Fold each step into a complete [`TreeState`](/model/tree-state.md).
5. Compute [display spans](/model/display-markers.md) and their line ranges in
   that tree.
6. Bind [play cells](/model/play-cell.md) and [exercises](/model/exercise.md).

Every failure along the way is collected, not thrown — see [errors](/model/errors.md).

# The lock text

`lock_text()` renders the plan as [`bower.lock`](/config/bower-lock.md). It lives
in the kernel and is **the only definition of that format** — there is no
parser. `bower status` compares *rendered text*, because a parser would be a
second definition free to disagree with the first.

# Citations

[1] `bower-core/src/plan.rs`
[2] [`bower-spec.md` §2.1, §4](/references/bower-spec.md)
