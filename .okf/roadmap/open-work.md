---
type: Roadmap
title: Open work — backlog and technical debt
description: A snapshot of what is outstanding as of 2026-09-09, with pointers to the two files that are authoritative and change often.
tags: [roadmap, backlog, technical-debt]
timestamp: '2026-09-09T00:00:00Z'
---

> **This is a snapshot, not a source of truth.** `BACKLOG.md` and
> `docs/TECHNICAL_DEBT.md` are the live documents and move faster than this
> concept does. Read them before acting on anything here — and note that
> `BACKLOG.md` already carries at least one
> [stale claim](/books/rust4failures.md) about book size.

# In flight

Migrating [*Rust for Failures*](/books/rust4failures.md) — spec §11
[Phase 5](/roadmap/phases.md). Next concrete task: the chapter map over
*pkcore*'s `DIARY.md`.

# Next up — unwritten EPICs

- `--target ipynb` — see [the notebook target](/roadmap/notebook-target.md).
  Blocked on nothing technical; the kernel half is built.
- **Editions** — M2 of [the publishing ladder](/roadmap/publishing-ladder.md).
  Should take ownership of the `[book] version` key.
- **Authoring bridges** — see [authoring bridges](/roadmap/authoring-bridges.md).

# Technical debt worth knowing about

Sixteen open items, four closed. The ones that shape how much to trust the tool:

| Item | Why it matters |
|---|---|
| **`bower verify` does not run the book's own gate.** | A real defect was caught by eye rather than by tool as a result. |
| **`GitHubForge` is untested**, releases included. | Has already cost two real defects. It is the one component that touches the network. |
| **The push gate identifies a book by directory basename.** | The [marker guard](/commands/push.md) is otherwise the strongest safety rule in the system; this is its weak joint. |
| **`bower push` ignores lock drift.** | It checks site staleness but not whether the plan is current. |
| **Verification is sequential.** | The spec calls it embarrassingly parallel; each tree state is independent. |
| **Nothing reports whether a published artifact is current.** | `status` covers the local site directory, not the remote. |
| **One copyleft crate in the tree** — `uluru`, MPL-2.0, via `gix-pack`. | Allowed deliberately in `deny.toml` and recorded, not accidental. |

# Still-open spec questions

Four of ten, all in unbuilt territory: wheel distribution for notebooks,
`expect` for play cells, JupyterLite/pyodide, and whether `devenv.nix` is
hand-authored or derived from `bower.toml` + `rust-toolchain.toml`.

# A documentation drift worth fixing

`README.md` documents the elision leader as `// ⋯ 9 lines elided`. The code
emits ASCII `...`, and a test named
`elision__is_ascii_so_every_font_can_render_it` pins it that way — the Unicode
character was removed because a font lacked the glyph and xelatex dropped it
silently. See [display markers](/model/display-markers.md).

# Citations

[1] `BACKLOG.md` (refreshed 2026-09-03), `docs/TECHNICAL_DEBT.md`
