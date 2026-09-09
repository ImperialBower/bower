---
type: Roadmap
title: The publishing ladder — M0 to M4
description: Repos are the first output target, not the last. Five rungs from the current state to "a publishing system that happens to have started as a repo generator".
tags: [roadmap, publishing, editions]
timestamp: '2026-09-09T00:00:00Z'
---

# The premise

The same kernel discipline extends outward: publishing stays a **pure fold from
book source to a render plan**, with renderers as replaceable I/O at the edges —
the way [`TreeState`](/model/tree-state.md) already relates to git. Each rung is
shippable on its own.

# The rungs

| Rung | Contents | Status |
|---|---|---|
| **M0 — this spec** | Generated repos, verified steps, mdBook HTML, the pandoc epub flow. | Done |
| **M1 — unified targets** | `publish --target html\|epub\|pdf\|ipynb` owning the whole render; one display-marker engine feeding all targets; per-target elision; PDF with proper code typography. | **Done except `ipynb`** |
| **M2 — editions** | A published edition as a *pinned triple*: book source commit, `bower.lock`, and the generated repo tags — stamped `edition-1.0` across book and repos. | Not started |
| **M3 — author tooling** | `bower new chapter`; a watch mode that rebuilds book + plan on save; `draft = true` chapters (verified but excluded from publish); sample extraction as a render plan over a subset of chapters. | Not started |
| **M4 — the imprint** | A common `bower.toml` starter and CI workflow each book inherits; cross-book step links resolved within the workspace; a catalog page built by reading `books/`; one `bower publish --all`. | Not started |

# M2 is the interesting one

An edition tag is never deleted or force-moved, so **a reader of the 1.0 epub
follows 1.0 links forever** while `main` moves on. That is the piece the current
design cannot yet honour: today's tags are recreated on every regeneration.

It also makes errata first-class. A correction regenerates `main`, and
`bower diff --edition 1.0` emits the reader-facing errata list — which chapters,
which steps, what changed — mechanically rather than from memory.

`[book] version` already exists as a free-string edition key, and the backlog
notes that M2 *should own it*.

M4 assumes [one workspace for every book](/decisions/one-workspace-for-books.md),
which is now the case.

# The boundary

What Bower does **not** aspire to: DRM, storefronts, or payment. Leanpub,
Gumroad and the rest remain the distribution channel. Bower's job ends at
producing verified, beautiful, deeply cross-linked artifacts.

# Citations

[1] [`bower-spec.md` §13](/references/bower-spec.md), `BACKLOG.md`
