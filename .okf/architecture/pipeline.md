---
type: Architecture
title: The pipeline
description: Book markdown becomes blocks, steps, a plan, and tree states — all pure — and only then meets git, the compiler, and the renderers.
tags: [architecture, dataflow]
timestamp: '2026-09-09T00:00:00Z'
---

# The data flow

```
book source (.md)
      │  parse annotations
      ▼
  Vec<Block> ──group by (repo, step)──▶ Vec<Step> ──order──▶ BookPlan   (pure)
      │                                                          │
      │                                                          ▼
      │                                                fold: TreeState per step   (pure)
      │                                                          │
      ▼                                                          ▼
 link map (step ⇄ book anchor)                          git replay + verify (I/O)
      │                                                          │
      ▼                                                          ▼
 mdbook-bower preprocessor                          local repo, tags, optional push
 (injects links into rendered book)                 (commit trailers link back)
```

Everything above the "git replay" line is the [domain kernel](/architecture/domain-kernel.md).

# The stages

| Stage | Owned by | Produces |
|---|---|---|
| Fence-aware chapter scan, directive↔fence pairing, `include=` resolution | `block.rs` | `Block`, `BlockContent` |
| Grouping and [ordering](/model/ordering.md) | `step.rs` | `Step` (internal) |
| Tree fold, op semantics, region engine | `tree.rs` | [`TreeState`](/model/tree-state.md) |
| Which spans render, and where they land in the tree | `display.rs` | `DisplaySpan`, `LineRange`, `BlockDisplay` |
| The front door: `plan()`, `lock_text()` | `plan.rs` | [`BookPlan`](/model/plan.md) |

# The two directions of linking

The pipeline is built so that page and code point at each other, and neither
link can go stale silently.

- **Book → repo.** Annotated [tags](/model/step.md) (`step-012-rank-enum`) plus
  line-anchored blob URLs, recomputed on every build from the same fold that
  produced the tree.
- **Repo → book.** Commit trailers on every generated commit, plus a generated
  `STEPS.md` at the repo root — the repo's own table of contents back into the
  book.

```
ch03: Introduce the Rank enum

Book-Source: rust4failures/src/ch03-ranks.md#step-rank-enum
Book-Url: https://…/ch03-ranks.html#step-rank-enum
Bower-Step: rust4failures/012
Generated-By: bower v0.1.0
```

`Book-Url` appears only when the book declares a `site`. `STEPS.md` doubles as
the safety marker that [`bower push`](/commands/push.md) checks before it will
force-push over anything.

# Consumers, not extensions

The CLI, the [preprocessor](/architecture/mdbook-bower.md), and the publishing
pipeline are all *consumers* of the kernel. That is why `ShowMark` and
`show_marker()` are public: a second copy of the marker grammar living in a
consumer is exactly the drift this crate exists to prevent.

# Citations

[1] `bower-core/src/`, `bower/src/replay.rs`, `bower/src/trailers.rs`
[2] [`bower-spec.md` §2.2, §5.2](/references/bower-spec.md)
