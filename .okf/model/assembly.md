---
type: Domain Concept
title: Assembly — fragments vs. full files
description: The two mechanisms that reconcile "the book wants to show 10 lines" with "the repo needs the whole compiling file" — mdBook hidden lines and named regions.
tags: [model, kernel, authoring]
timestamp: '2026-09-09T00:00:00Z'
---

# The central tension

The book wants to show ten lines. The repository needs the whole compiling file.
Assembly is how one annotated block satisfies both. It decides **what ends up in
the repo**; [display markers](/model/display-markers.md) are the orthogonal
question of what ends up on the page.

# Mechanism 1 — mdBook hidden lines

In a `rust` block, lines starting with `# ` are hidden by mdBook's renderer but
are part of the block. The repo file gets *all* lines with `# ` stripped; the
rendered book shows only the visible ones. One block is simultaneously the
honest full file and the pedagogical fragment.

Detection is narrow on purpose: hidden-line handling applies **only** when the
fence info string's first token is `rust`. A leading `# ` or a lone `#` is
hidden; `#[` and `#!` are ordinary Rust and are left alone. Setting
`hidden="false"` keeps those lines out of the repo entirely.

Good for small files and a stray line or two. It does not scale — prefixing
every boilerplate line with `# ` is worse than the problem it solves.

# Mechanism 2 — named regions

For files that grow across chapters, the first `create` establishes region
markers as ordinary comments:

```rust
// bower:begin from_char
// bower:end from_char
```

A later block with `op="region" region="from_char"` replaces exactly the text
between the markers. The reader sees only the new code; the kernel computes the
full resulting file.

Markers are recognized after `//`, `#`, `--`, `;`, or bare, and `bf:` works as
an alias for `bower:`. They **stay in the held tree** so later steps can find
them, and are stripped at `materialized()` unless the repo sets
`keep_region_markers`. See [tree state](/model/tree-state.md).

Two failure modes are named errors: `RegionMissing` (the markers are not there)
and `RegionUnbalanced` (a `begin` without its `end`).

# `append` — the third way

`op="append"` covers the common "now add the test module at the bottom" move
without requiring any markers at all. Within one [step](/model/step.md),
appends and *distinct* regions may share a file; whole-file ops may not.

# Why two mechanisms and not one

They are complementary, and the spec is explicit that the overlap is deliberate.
Hidden lines keep a block copy-paste-runnable while drafting. Regions scale to
files that grow over many chapters. Neither subsumes the other.

# Citations

[1] `bower-core/src/tree.rs`
[2] [`bower-spec.md` §3.3](/references/bower-spec.md)
