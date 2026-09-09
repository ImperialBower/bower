---
type: Domain Concept
title: Tree state
description: The complete file tree of a repository after one step — a pure value, deterministic in iteration order, and the thing git actually commits.
tags: [model, kernel, determinism]
timestamp: '2026-09-09T00:00:00Z'
---

# What it is

```rust
pub struct TreeState(pub BTreeMap<String, FileBody>);

pub enum FileBody {
    Text(String),
    Binary(Vec<u8>),
}
```

The complete file tree of a repo **after** a [step](/model/step.md) — not a
diff. Every step carries one.

# Why a BTreeMap

Deliberate. A `BTreeMap` iterates in sorted key order, so everything downstream
— the git tree, the lock text, the rendered book — is deterministic without any
extra sorting pass. Swap in a `HashMap` and byte-identical SHAs stop being
achievable. See [invariants](/architecture/invariants.md).

# Text conventions

Text bodies are newline-joined with exactly one trailing newline; an empty file
is the empty string. Fixing this at the kernel boundary is what stops
"whitespace-only" diffs appearing between two runs of the same book.

# Methods that matter

| Method | Purpose |
|---|---|
| `text(path)` | The file's text, if it is text. |
| `get(path)` / `paths()` | Lookup and enumeration. |
| `apply_block(...)` | Fold one block in, per its `op`. See [assembly](/model/assembly.md). |
| `materialized(keep_region_markers)` | The tree as it should be written to disk. |

# Materialized vs. held

Two kinds of marker live in the tree, and they behave differently:

- **Region markers** (`// bower:begin name` / `// bower:end name`) **stay** in
  the held tree, because a later `op="region"` step has to find them. They are
  stripped at `materialized()` unless the repo sets `keep_region_markers`.
- **Display markers** (`// bower:show`) **never reach the repo at all** — they
  are dropped during assembly. They are book concerns, not code.

Leaving region markers in is itself a defensible choice for a generated repo: a
reader browsing the code can see where the book will edit next.

# Citations

[1] `bower-core/src/tree.rs`
[2] [`bower-spec.md` §2.1, §3.3, §3.4](/references/bower-spec.md)
