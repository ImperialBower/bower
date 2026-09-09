---
type: CLI Command
title: bower publish
description: Folds the book into one render plan and hands it to a renderer — html, epub, or pdf — with every external tool checked by name before anything is written.
tags: [command, rendering, pdf, epub]
timestamp: '2026-09-09T00:00:00Z'
---

# Usage

```
bower --book <DIR> publish --target <html|epub|pdf> [-o|--out <DIR>]
```

`--target` is **required**; `--out` defaults to `published`.

# The render plan

Publishing is a pure fold, the same discipline the kernel applies to trees:

```rust
RenderPlan { target, meta, version, chapters, assets }
```

`RenderedChapter { path, title, markdown }`, where the title comes from the
first `#` ATX heading, else the path. Because the fold is pure, most of
`bower/tests/publish.rs` asserts against the plan with **no renderer installed**.

All targets share one display-marker engine and differ in exactly one rule — see
[display markers and elision](/model/display-markers.md).

# Preflight: every tool named before anything is written

The `Renderer` trait is `preflight()` + `render()`, and `preflight()` runs
before any output. `tool_present` probes `<tool> --version`.

| Target | Needs | Install hint given on failure |
|---|---|---|
| `html` | `mdbook`, then `mdbook-bower` on `PATH` | `cargo install mdbook`; `cargo build -p bower && export PATH="$PWD/target/debug:$PATH"` |
| `epub` | `pandoc` | `brew install pandoc` |
| `pdf` | `pandoc`, `typst`, plus a font check | `brew install typst` |

Because `book.toml` declares `[preprocessor.bower]`, an `html` build without
`mdbook-bower` **fails** rather than rendering a book whose directives were
never applied.

# The PDF route, and why it is not LaTeX

`pandoc --to typst`, then `typst compile` — not `pandoc --to pdf`. EPIC-07 Phase
0 measured LaTeX as non-reproducible even with `SOURCE_DATE_EPOCH` and
`FORCE_SOURCE_DATE`; Typst is byte-identical with `SOURCE_DATE_EPOCH` alone.

That epoch is `bower.toml`'s `epoch` — the same value that pins commit SHAs. One
configured instant makes the git history and the PDF reproducible together.

`template.typ` is found by convention beside `book.toml` and is **prepended** as
a preamble, since Typst's `#set` rules apply forward. Every font it names is
checked against `typst fonts` output first; a miss is reported as a missing
tool. That check exists because silent font substitution had already hidden a
real bug — see [display markers](/model/display-markers.md).

# Covers

Two files by convention beside `book.toml`: `cover.svg` (the title band —
required, no svg means no cover) and optional artwork, matched in the fixed
order `cover.png`, `cover.jpg`, `cover.jpeg` so directory iteration cannot
change the artifact.

`compose_cover` stacks them into **one 1600×2400 SVG** (an 800px title band over
1600px of artwork), with the artwork embedded as a `data:` URI so the result is
self-contained — pandoc's and Typst's working directories differ. Artwork uses
`preserveAspectRatio="xMidYMid slice"`, so it crops rather than letterboxes.

Both renderers receive the same bytes — pandoc as `--epub-cover-image`, Typst as
a full-bleed first page with the page counter reset afterwards — so the epub and
the PDF cannot show different covers. The composition is a pure function, so it
is tested with no renderer installed.

# Artifact naming

`slug(title)` plus `_{version}` when the book declares one:

```
rust-for-failures_0.1.0.epub
hello-playbook.epub
```

# Citations

[1] `bower/src/publish.rs`, `bower/src/render.rs`
[2] `docs/EPIC-06_Publish.md`, `docs/EPIC-07_Pdf.md`, [`bower-spec.md` §13](/references/bower-spec.md)
