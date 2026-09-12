---
type: Domain Concept
title: The bower directive
description: An HTML comment carrying TOML-flavored key-values, placed immediately before a fenced code block, that binds that block to a file in a target repository.
tags: [model, syntax, authoring]
timestamp: '2026-09-12T00:00:00Z'
---

# What it is

A directive is an HTML comment immediately preceding a fenced block, carrying
TOML-flavored key-values:

```markdown
<!-- bower repo="rust4failures" file="src/rank.rs" -->
```

The canonical directive word is `bower`; `bf` is accepted as a short alias for
drafting speed.

# Why an HTML comment

Three design constraints forced the choice, and HTML comments satisfy all three:

1. **Invisible in every renderer already in use** — mdBook HTML, pandoc → epub,
   and plain GitHub markdown preview.
2. **Must not fight mdBook's own conventions** — info-string flags
   (`rust,ignore`, `no_run`) and the `#`-hidden-line convention. Bower
   *exploits* them rather than competing with them. See [assembly](/model/assembly.md).
3. **Writable by hand mid-sentence** while drafting, without breaking flow.

This is also why the pandoc/epub path keeps working with no special support:
directives are simply comments.

# The key set

| Key | Required | Default | Meaning |
|---|---|---|---|
| `repo` | yes | — | Target repo name, declared in [`bower.toml`](/config/bower-toml.md) |
| `file` | yes* | — | Path inside the repo this block writes |
| `op` | no | `create` | `create` \| `replace` \| `region` \| `append` \| `delete` \| `copy` \| `none` |
| `step` | no | auto | Explicit [step](/model/step.md) id for grouping/ordering |
| `after` | no | — | Order this step after the named step id. See [ordering](/model/ordering.md) |
| `msg` | no | derived | Commit message subject for the step |
| `expect` | no | `pass` | See [expectations](/model/expectation.md) |
| `region` | with `op="region"` | — | Named region in the file to replace |
| `src` | with `op="copy"` | — | Book-relative asset path (binary files, fixtures) |
| `paths` | with multi-file `op="delete"` | — | Comma-separated path list; entries are trimmed and empties dropped |
| `hidden` | no | `true` | Whether mdBook `#`-hidden lines are included in the repo file |
| `show` | no | all marked | Named [display span(s)](/model/display-markers.md) to render |
| `include` | no | — | Pull block content from the block library instead of inline |
| `notebook` | no | — | `play`: a live [notebook cell](/model/play-cell.md), not repo content |
| `exercise` | no | — | A "your turn" task on this step. See [exercises](/model/exercise.md) |
| `output` | no | — | `check` \| `verify`: what that command printed at the bound step. See [captured output](/model/captured-output.md) |

\* `file` is not required for `op="delete"` steps declared with `paths=[…]`, nor
for pure-narrative steps (`op="none"`) that exist only to carry a commit message
— a refactoring the book describes but does not show in full.

# The `op` values

`op` selects how the block's text becomes file content in the folded
[tree state](/model/tree-state.md).

- `create` — write a new file (the default).
- `replace` — overwrite a file that already exists.
- `region` — replace exactly the text between named markers. See [assembly](/model/assembly.md).
- `append` — add to the bottom of an existing file. Covers "now add the test
  module" without requiring markers.
- `delete` — remove a path.
- `copy` — copy a book-relative asset named by `src` (binary files, fixtures).
- `none` — a prose-only step that exists only to carry a commit message.

# Multi-file steps

One step is one commit, but a commit often touches several files — a new module,
plus its `mod` declaration, plus `Cargo.toml`. Blocks that share an explicit
`step` id merge into one step:

```markdown
<!-- bower repo="rust4failures" step="rank-enum" file="src/rank.rs" -->
…block…
<!-- bower repo="rust4failures" step="rank-enum" file="src/lib.rs" op="region" region="mods" -->
…block…
```

Blocks without a `step` id get an auto-generated one and form single-block steps.

# The block library

A block may live in a companion file under the book source instead of inline,
referenced by `include`:

```markdown
<!-- bower include="blocks/ch01/rank-enum.md" -->
```

The library file holds the directive defaults and the fence; the chapter holds
one line. This was decided as a **core, recommended** way to write a block —
not a fallback. See [the block library decision](/decisions/block-library-is-core.md)
and [authoring bridges](/roadmap/authoring-bridges.md).

# Citations

[1] [`bower-spec.md` §3.1–§3.2, §3.5, §14.3](/references/bower-spec.md)
