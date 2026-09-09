---
type: Domain Concept
title: Display markers and elision
description: Which part of a block the rendered book shows — and the one rule where HTML and epub/PDF are allowed to differ.
tags: [model, kernel, rendering]
timestamp: '2026-09-09T00:00:00Z'
---

# What they are

[Assembly](/model/assembly.md) decides what reaches the repo. Display markers
decide what reaches the *page*. The full code is always present — in the block,
in the generated repo, and in HTML one click away — but the render prints only
the spans that earn their place.

```rust
// bower:show
impl From<char> for Rank {
    fn from(c: char) -> Self { /* … */ }
}
// bower:show end
```

# Rules

- **No markers → the whole block renders.** Small examples pay no tax.
- **Any marker present → only marked spans render.** Everything else is elided.
- Spans can be named — `// bower:show begin from_char` … `// bower:show end` —
  so one full-file block can be reused across chapters, with the directive key
  `show="from_char"` selecting a different slice each time.
- Display markers are **always stripped** from the generated repo tree. Unlike
  region markers, this is not configurable. They are book concerns, not code.

Malformed spans are named errors: `UnclosedShowSpan`, `NestedShowSpan`,
`OrphanShowEnd`, and `UnknownShowSpan` for a `show=` naming a span the block
does not define.

# The one rule that differs per target

Everything about rendering is shared between targets except this. In
`render::body_lines`:

```rust
let rustish = target.has_hidden_lines()
    && info.split([',', ' ']).next() == Some("rust");
```

`has_hidden_lines()` is true **only** for `Target::Html`.

- **HTML + a `rust` fence** → elided lines become mdBook hidden lines, so the
  reader gets the standard eye-toggle and the code is literally all there,
  expandable inline.
- **Everything else** (epub, PDF, and non-Rust fences) → the elided run
  collapses to a single comment line.

# The exact elision string

```rust
format!("{comment} ... {elided} {plural} elided{link}")
```

`plural` is `line`/`lines`; `link` is `" — full file: {url}"` or empty.

```
# ... 1 line elided
// ... 9 lines elided — full file: https://github.com/…/src/rank.rs
```

The comment token is `//` for rust, c, cpp, go, java, js, ts and json5; `--` for
sql, lua and haskell; `#` otherwise.

> **The leader is ASCII `...`, not `⋯`.** A test —
> `elision__is_ascii_so_every_font_can_render_it` — pins it. The Unicode
> midline ellipsis U+22EF was removed because Latin Modern Mono has no glyph for
> it and xelatex dropped it *silently*. `README.md` still documents the old
> `⋯` form and is stale on this point.

The elision link deliberately strips the URL template's `#fragment`, so it
points at the whole file rather than at the span the reader cannot see.

# Line-anchored links

Because replay is deterministic, Bower knows the exact path *and line range* of
every displayed span in the generated tree at that step. `DisplaySpan` and
`LineRange` carry that, and the preprocessor turns it into
`…/blob/step-012-rank-enum/src/rank.rs#L18-L31`. The line map is recomputed on
every build, so a stale anchor is a build error rather than a reader's
discovery. See [invariants](/architecture/invariants.md).

If a displayed span cannot be located in the materialized tree, the kernel
raises `SpanNotInTree` — documented as "a kernel invariant violation surfaced as
an error rather than a panic."

# Citations

[1] `bower-core/src/display.rs`, `bower/src/render.rs`
[2] [`bower-spec.md` §3.4](/references/bower-spec.md)
