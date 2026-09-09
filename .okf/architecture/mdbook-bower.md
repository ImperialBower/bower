---
type: Component
title: mdbook-bower — the preprocessor
description: The mdBook preprocessor that strips directives, applies display markers, injects step anchors, and hangs line-anchored links under every annotated block.
tags: [component, mdbook, rendering]
timestamp: '2026-09-09T00:00:00Z'
---

# Wiring

A second binary in the [`bower` crate](/architecture/crate-bower.md), behind the
default-on `preprocessor` feature. A book opts in through its `book.toml`:

```toml
[preprocessor.bower]
command = "mdbook-bower"
```

# The protocol

- `mdbook-bower supports <renderer>` exits 0 only for `html`.
- Bare invocation reads `[context, book]` JSON on stdin and writes the book to
  stdout.
- `--version` / `-V` prints `mdbook-bower {version}` — added because
  `tool_present` probes `--version`, and without it an installed binary looked
  missing.

**Both mdBook envelope shapes are read.** 0.4 sends `sections`, 0.5 sends
`items`; the code accepts either. Reading only one would silently return the
book untouched with every directive still on the page — a failure that looks
like success, which is exactly the class of bug this project designs against.

# What it does to a chapter

1. **Strips** `bower` directives from the output.
2. **Applies [display markers](/model/display-markers.md)** — in HTML, elided
   spans become mdBook hidden lines, so the reader gets the eye-toggle.
3. **Injects an anchor** `<a id="step-{id}"></a>` before the anchored block, so
   commit trailers and exercise links land on the exact block.
4. **Adds a header above the fence** — a `step-meta` span carrying a file link,
   one `L{start}–L{end}` link per shown span, `step {seq:03} of {repo}`, and
   `diff` / `browse` links.
5. **Adds a checkout line below the fence**, with the `$>` prompt *outside* the
   code span, so copying yields a runnable command.

Fences not opened by a directive pass through untouched — a book that quotes
directives as examples survives its own preprocessor.

When a link template is missing, it emits plain text. It never invents a URL.

# Failure is loud

`preprocess()` runs the full [plan](/model/plan.md). If the book does not
resolve, it prints `mdbook-bower: the book does not resolve:` followed by every
collected error, and exits failure — so `mdbook build` fails.

A directive naming an unknown repo therefore breaks the book build. Annotation
rot is caught by book CI rather than discovered by a reader.

# The double render

HTML is rendered twice on purpose: mdBook re-invokes `mdbook-bower` itself, so
`MdBookRenderer::render` ignores the `RenderPlan` it was handed — though the
plan is still built and checked first, which is what makes an unresolvable book
fail fast. EPIC-06 recorded the decision not to unify these paths.

# Citations

[1] `bower/src/mdbook.rs`, `bower/src/mdbook_bower.rs`, `bower/src/render.rs`
[2] [`bower-spec.md` §5.4](/references/bower-spec.md), `docs/EPIC-03_Preprocessor.md`
