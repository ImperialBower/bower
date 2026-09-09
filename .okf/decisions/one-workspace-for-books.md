---
type: Decision
title: One workspace for every book — a reversal
description: Decided one-repo-per-book on 2026-09-04, reversed to a single workspace on 2026-09-05 after trying the other answer first.
tags: [decision, repository-layout, reversal]
timestamp: '2026-09-05T00:00:00Z'
---

# The question

Should each book's *source* live in its own repository, or should every book
share one workspace with the tool?

# The decision, and its reversal

- **4 September 2026** — decided: one repo per book.
- **5 September 2026** — **reversed**: one workspace.

Every book's source — chapters, [`bower.toml`](/config/bower-toml.md), block
library — lives in this repository under `books/`, beside the sample.

# Why it was reversed

Standing up the first real book settled the question by trying the other answer
first. A separate source repository:

- duplicates the toolchain, the Makefile, and the `.gitignore`;
- puts a **version skew** between a book and the `bower` that renders it.

In one workspace, a change to the kernel and the chapter that exercises it are
one commit, and `make failures` is the whole loop.

This is worth remembering as a pattern: the reversal was cheap because it was
made after building something, not before.

# What it does *not* change

How a book's **generated** repositories work — that was never in question. Each
book still publishes to its own GitHub repository, receiving the code on its
default branch and the rendered book on `site_branch`. See
[`bower push`](/commands/push.md).

# What it enables later

M4 of the [publishing ladder](/roadmap/publishing-ladder.md) — "the imprint" —
assumes a shared workspace: cross-book step links resolved within the workspace
rather than across repositories, a catalog page built by reading `books/`, and
one `bower publish --all` over the whole estate.

# Citations

[1] [`bower-spec.md` §12 Q2, §13 M4](/references/bower-spec.md)
