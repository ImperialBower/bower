---
type: Architecture
title: The invariants
description: The properties the system promises, each with the test suite that proves it — determinism, line-map exactness, verified failure, visible drift.
tags: [architecture, invariants, testing]
timestamp: '2026-09-09T00:00:00Z'
---

# Why this list matters

These are claims with tests behind them, not aspirations. Each row names the
suite that would go red if the property stopped holding.

| Invariant | Proven by |
|---|---|
| **Determinism** — equal book in, byte-identical plan out. | `bower-testkit/tests/properties.rs` |
| **Line-map exactness** — every displayed span's `LineRange` covers exactly that span's text in the materialized tree. | `bower-testkit/tests/properties.rs` |
| **Errors, not surprises** — every broken fixture produces its named error, with chapter and line attached. | `bower-testkit/tests/corpus.rs` |
| **Full state coverage** — the corpus exercises every op, expectation, and mechanism, and the report proves it. | `corpus_state_coverage_is_complete` |
| **Replay determinism** — two `bower build` runs of an unchanged book produce identical SHAs, and a replay over a dirty directory matches one over an empty directory. | `bower/tests/determinism.rs` |
| **Declared failures really fail** — `compile_fail` does not compile; `test_fail` compiles but fails its tests. An untrue expectation fails the build, naming chapter and line. | `bower/tests/verification.rs` |
| **Drift is visible** — edit a chapter, delete a file, or drop a tag, and `bower status` names exactly what changed. | `bower/tests/status.rs` |
| **The page links to the code, by line** — every rendered block's footer names its file and line range at a tag, and those are the lines the reader just saw. | `bower/tests/preprocessor.rs` |
| **Every target shows the same thing** — html, epub and PDF render identically except for how an elided span is shown. | `bower/tests/publish.rs` |
| **The PDF is reproducible** — two runs of an unchanged book produce identical bytes. | `pdf_is_byte_identical_across_runs` |
| **A stale site is visible** — edit one paragraph and `bower status` reports the rendered book out of date, even when plan, lock and repo are unchanged. | `bower/tests/site.rs` |
| **Kernel purity** — `bower-core` has zero dependencies. | `make lint` → `purity` |

# How determinism is actually achieved

Three deliberate choices, each of which would break it if reversed:

1. **`BTreeMap` everywhere** in [`TreeState`](/model/tree-state.md), so iteration
   order is sorted rather than incidental.
2. **Synthetic timestamps.** `stamp(seq) = epoch + seq minutes`, formatted with
   the offset pinned to `+0000` by hand. No clock, no environment variable, no
   machine identity. The `epoch` comes from
   [`bower.toml`](/config/bower-toml.md).
3. **Fixed author and committer** from `[identity]`, the same signature for
   both roles.

The same `epoch` is passed to `typst compile` as `SOURCE_DATE_EPOCH`, which is
why the PDF is byte-identical too — one pinned value makes both the git history
and the rendered artifacts reproducible.

# The invariant behind the safety rules

Two behaviours look like caution and are really consequences of determinism:

- Replay always **starts from an empty tree** and rebuilds everything.
  Incremental replay is an explicit non-goal, because determinism makes full
  regeneration cheap and removes an entire class of drift bugs.
- The output directory is **removed and recreated**, never merged.

# A related discipline: don't share a library with what you inspect

[`bower status`](/commands/status.md) reads git tags as loose refs from
`.git/refs/tags` directly rather than through `gix`, on the reasoning that *"a
drift report that shares a library with the thing it inspects can share a bug
with it."* The trade-off is explicit: packed tags are reported as
`tags are packed; cannot check them here` rather than silently mis-read.

# Citations

[1] `bower/tests/`, `bower-testkit/tests/`, `bower/src/replay.rs`, `bower/src/status.rs`
[2] [`README.md`](https://github.com/ImperialBower/bower/blob/main/README.md), [`bower-spec.md` §5.1](/references/bower-spec.md)
