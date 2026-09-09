---
type: Overview
title: What Bower is
description: Bower inverts the usual relationship between a book and its example code — the book is the source of truth, and the git repository is a build artifact of the book.
resource: https://github.com/ImperialBower/bower
tags: [bower, overview, thesis]
timestamp: '2026-09-09T00:00:00Z'
---

# The inversion

Most technical books quote their example repository. Snippets are copied out of
source files into prose, and from that moment they drift.

Bower reverses the arrow. Annotated fenced code blocks **in the book's markdown**
are the single source of truth. Bower parses those annotations, resolves them
into an ordered [plan](/model/plan.md), and replays the plan as a real git
repository — one commit per teaching [step](/model/step.md).

```
the book stops quoting the repo;
the repo becomes a build artifact of the book.
```

Edit an example in chapter 3 and re-run: the repository recreates itself with
the change threaded through every subsequent commit.

# The three problems it solves

The [design spec](/references/bower-spec.md) names the problems precisely.

| Problem | Bower's answer |
|---|---|
| **No single home.** Snippets are duplicated between prose and source, and drift. | The block in the chapter *is* the file in the repo. |
| **No replayability.** A reader cannot follow along, because the book's teaching order is not any real repo's git history. | The plan is replayed from an empty tree in the book's order. |
| **No stable cross-references.** Nothing links "this paragraph" to "this exact state of the code" and survives an edit. | Annotated tags per step, plus line-anchored blob links recomputed on every build. |

# Why the name

In euchre, the *right bower* is the card that controls the game. The book is the
right bower here — everything else is derived from it. ImperialBower is the
house.

# What makes it trustworthy

Two properties do the load-bearing work, and both are enforced by tests rather
than asserted in prose. See [invariants](/architecture/invariants.md).

- **Determinism.** Equal book in, byte-identical plan — and byte-identical commit
  SHAs — out. Achieved by fixing author/committer identity and deriving every
  timestamp from a configured epoch. See [replay determinism](/architecture/invariants.md).
- **Verified failure.** A step may declare that it *should* fail to compile or
  *should* fail its tests, and [`bower verify`](/commands/verify.md) asserts the
  failure really happens. This is the thesis of *Rust for Failures* enforced by
  the book's own toolchain.

# The shape of the system

Everything upstream of git replay is a pure [domain kernel](/architecture/domain-kernel.md);
git, the filesystem, and the renderers are I/O at the edges. See
[the pipeline](/architecture/pipeline.md) for the full data flow, and
[crate layout](/architecture/index.md) for how that maps onto four crates.

# Where to go next

- The vocabulary: [the domain model](/model/index.md).
- The command surface: [commands](/commands/index.md).
- The two books it drives: [books](/books/index.md).

# Citations

[1] [`bower-spec.md` §0–§2](/references/bower-spec.md) — Draft 0.2, 31 August 2026.
[2] [`README.md`](https://github.com/ImperialBower/bower/blob/main/README.md)
