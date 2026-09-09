---
type: Decision
title: The block library is core, not a Scrivener fallback
description: Decided 2026-09-04 — `include=` is a first-class, recommended way to write a block; chapters may mix inline and included blocks freely.
tags: [decision, authoring, syntax]
timestamp: '2026-09-04T00:00:00Z'
---

# The question

A block may live in a companion file under the book source instead of inline in
the chapter:

```markdown
<!-- bower include="blocks/ch01/rank-enum.md" -->
```

Is that a core feature, or an escape hatch reached for only when
Scrivener-style authoring needs it?

# The decision

**Core.** Decided 4 September 2026. `include` is a first-class, recommended way
to write a block, and chapters may mix inline and included blocks freely.

# What it buys

- **Editor safety.** Prose drafted in a tool with smart punctuation can mention
  a block by include line — easily typed, immune to curly quotes — while the
  code is authored in a real editor with rust-analyzer. See
  [authoring bridges](/roadmap/authoring-bridges.md) for the hazard this
  addresses.
- **Obsidian ergonomics.** Block files are notes; embedding and backlinking work
  on them natively.
- **Reuse.** One block, displayed in two chapters with different `show=` slices,
  without duplication.

# What it costs

One more level of indirection, and the drafting flow loses "the code is right
there in the prose".

# The framing that makes it safe

The block library is still *book source* — same directory, same git history,
same single source of truth. As the spec puts it: **this is not a second home
for code, just a second room.**

# In the implementation

`include=` resolves against `BookSource.library` in `block.rs`, and never
chains — an included directive's own `include` is not followed. Two errors cover
it: `IncludeMissing` and `IncludeMalformed` (an entry that did not contain
exactly one block). The testkit ships an `include_library` fixture, and
`Mechanism::Include` is one of the ten states the
[coverage report](/architecture/crate-bower-testkit.md) must cover.

# Citations

[1] [`bower-spec.md` §12 Q6, §14.3](/references/bower-spec.md)
[2] `bower-core/src/block.rs`
