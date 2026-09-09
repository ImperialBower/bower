---
type: Crate
title: bower-core
description: The domain kernel — parses directives, groups and orders steps, folds tree states, computes line maps, and reports every error in one located pass. Zero dependencies.
resource: https://github.com/ImperialBower/bower/tree/main/bower-core
tags: [crate, kernel, rust]
timestamp: '2026-09-09T00:00:00Z'
---

# What it is

> The Bower domain kernel: parse annotated book sources into deterministic repo
> plans and tree states. Pure — no I/O, no git, no serialization in the public
> API.

See [the domain kernel pattern](/architecture/domain-kernel.md) for why it is
shaped this way.

# Module layout

| File | Owns |
|---|---|
| `lib.rs` | Crate docs, `BowerError`, the `Errors` collector, `prelude`. |
| `source.rs` | Caller-supplied input: `BookSource`, `Chapter`, `Location`, `RepoName`, `RepoSpec`, `RepoCatalog`. |
| `directive.rs` | The `<!-- bower … -->` grammar: `Op`, `Expect`, `Directive`, and the key-value pair parser. |
| `block.rs` | Fence-aware chapter scan, directive↔fence pairing, `include=` resolution, required-key validation. `Block`, `BlockContent`. |
| `step.rs` | Grouping and ordering. `StepId`, `Step`, `group()`, `order()`. |
| `tree.rs` | The tree fold, op semantics, region engine, marker syntax. `TreeState`, `FileBody`, `ShowMark`, `show_marker()`, `assemble()`. |
| `display.rs` | Which span renders and where it lands. `DisplaySpan`, `LineRange`, `BlockDisplay`, `analyze()`, `resolve_ranges()`. |
| `plan.rs` | The front door: `plan()`, `lock_text()`, `BookPlan`, `RepoPlan`, `PlannedStep`, `PlayCell`, `Exercise`, `ExerciseForm`. |

# The prelude

`use bower_core::prelude::*;` is the one import a consumer needs. It exports
`BowerError`; `Block`, `BlockContent`; `Directive`, `Expect`, `Op`;
`BlockDisplay`, `DisplaySpan`, `LineRange`; `BookPlan`, `RepoPlan`,
`PlannedStep`, `PlayCell`, `Exercise`, `ExerciseForm`, `lock_text`, `plan`;
`BookSource`, `Chapter`, `Location`, `RepoCatalog`, `RepoName`, `RepoSpec`;
`StepId`; `TreeState`, `FileBody`, `ShowMark`, `show_marker`.

Two naming traps worth knowing:

- The top-level plan type is **`BookPlan`**, not `Plan`. `plan` is the function.
- The public step type is **`PlannedStep`**. The internal `Step` is not
  exported.

# Dependencies

**None.** The `[dependencies]` table is empty by design and CI asserts it stays
that way. `rstest` is the only dev-dependency.

# Lint posture

```rust
#![warn(clippy::pedantic, clippy::unwrap_used, clippy::expect_used)]
```

with two documented allowances: `module_name_repetitions` (because `RepoPlan`
and `RepoName` read better qualified) and `missing_panics_doc` (because library
code has no panic paths).

# Citations

[1] `bower-core/src/lib.rs`, `bower-core/Cargo.toml`
[2] [`bower-spec.md` §8](/references/bower-spec.md)
