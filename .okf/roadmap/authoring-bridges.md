---
type: Roadmap
title: Authoring bridges — Obsidian and Scrivener
description: Obsidian is a natural two-way fit; Scrivener is a one-way street that needs a design accommodation to be safe around code.
tags: [roadmap, authoring, obsidian, scrivener]
timestamp: '2026-09-09T00:00:00Z'
---

# The honest summary

**Obsidian is a natural, cheap, two-way fit. Scrivener is a one-way street** that
needs the [block library](/decisions/block-library-is-core.md) to be safe around
code.

# Obsidian — the vault is the book source

Obsidian and Bower already speak the same language: plain markdown on disk.
Integration is mostly *removing accidental friction*, in three tiers.

| Tier | Work | Status |
|---|---|---|
| **1** | Open the book source as a vault. [Directives](/model/directive.md) are HTML comments — invisible in reading view, editable in source mode. Everything round-trips. | **Works today, zero code** |
| **2** | Adapters in the preprocessor: wikilinks (`[[ch03-ranks]]`) translated to mdBook links, and `SUMMARY.md` generated from folder order or a frontmatter `order:` key. Both are small pure functions in `bower-core`, useful to non-Obsidian authors too. | Not built |
| **3** | A thin Obsidian plugin: live directive validation, a step badge in the gutter, "jump to generated file". All intelligence stays in `bower-core`; the plugin shells out to `bower plan --json`. | Not built, optional |

The vault's graph view over `[[step-id]]` links is a free visualization of the
book's dependency structure.

# Scrivener — compile in, never round-trip

Scrivener's project format is an RTF-based package with an XML index. It is not
a source of truth Bower can parse, and pretending otherwise would put an
irreversible, lossy format at the head of the pipeline.

- **Path A (recommended)** — draft prose in Scrivener, compile to
  pandoc-flavored markdown, and let Bower consume that as read-only input.
  Strictly one-directional; edits flow back by editing in Scrivener and
  recompiling.
- **Path B** — external folder sync. Tempting and fragile: sync conflicts,
  filename mangling. Supported only as a variant of Path A: prose sync, code
  elsewhere.

# The hazard being designed around

Scrivener's smart punctuation silently turns `"` into `"` inside anything it
considers prose, and **a single curly quote in a code block is a compile error
two chapters later.**

Hence the rule — *code never lives in Scrivener* — and hence the block library,
which lets a chapter reference a block by an easily-typed include line while the
code is authored in a real editor.

The spec also proposes enforcing this in `bower verify`, by refusing unexpected
Unicode in code. That check is not built.

# The invariant that makes all of it safe

> **Bower's contract is with the markdown on disk, never with the editor.**

Obsidian, Scrivener-compiled output, Zed, or `ed` — the kernel sees text, the
[plan](/model/plan.md) is a pure function of it, and `bower verify` is the
arbiter no editor can charm. Authoring tools may be adopted, mixed, and
abandoned without a migration.

The same invariant extends to targets: every target renders from the same
markdown on disk, so adding one never forks the book.

# Citations

[1] [`bower-spec.md` §14](/references/bower-spec.md)
