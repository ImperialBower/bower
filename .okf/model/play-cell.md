---
type: Domain Concept
title: Play cell
description: A Python cell bound to a step for the notebook target — live to run and mutate, and never part of any repository tree.
tags: [model, kernel, notebooks, roadmap]
timestamp: '2026-09-09T00:00:00Z'
---

# What it is

Reading about controlled failing is one thing; poking it is another. A play cell
is a block kind that becomes a **live code cell** in the notebook target:

```markdown
<!-- bower repo="rust4failures" notebook="play" -->
```python
from failures import Rank
Rank.from_char("A")   # now break it: what does 'Z' do?
```
```

`notebook` accepts exactly one value, `"play"`. Anything else is `BadValue`.

# Binding

A play cell attaches to the **nearest preceding [step](/model/step.md) of its
repo** in document order — you play with what you just built — or to an explicit
`step="rank-enum"`. Bound cells land on `PlannedStep.play_cells`.

# It never touches the tree

This is the defining constraint. A play cell carrying any tree-affecting key
(`file`, `op`, `region`, `src`, `paths`) is `PlayCellConflictingKeys` — refused
as a category confusion rather than quietly ignored. Two further errors cover
the rest: `PlayCellUnbound` (nothing to attach to) and `PlayCellUnknownStep`.

# One source, every target

| Target | What a play cell becomes |
|---|---|
| `ipynb` | A live, runnable cell. |
| HTML | A "try it" aside under the step, linking to the notebook. |
| epub / PDF | Dropped or footnoted, per book config. |

# Status

**The kernel half is built and the renderer is not.** Play cells are parsed,
bound into the plan, and error-checked today; `--target ipynb` is not
implemented — its `FromStr` error names it as spec §15's target, needing play
cells. It is on the [roadmap](/roadmap/notebook-target.md), where the open
questions about wheel distribution and `expect` for play cells also live.

# Why Bower will not generate the bindings

The execution surface is a PyO3/maturin binding crate wrapping the chapter's
Rust. Bower deliberately does **not** generate that code — auto-generated
bindings are exactly the unauditable machinery both books argue against. The
binding crate is authored *in the book* like everything else, growing step by
step and visible at every tag.

# Citations

[1] `bower-core/src/plan.rs`, `bower-core/src/lib.rs`
[2] [`bower-spec.md` §15](/references/bower-spec.md)
