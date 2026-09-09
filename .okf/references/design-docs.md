---
type: Reference
title: Where the project's other documents live
description: A map of the in-repo documentation — EPICs, defect reports, backlog, technical debt — and what each is authoritative for.
tags: [reference, documentation, navigation]
timestamp: '2026-09-09T00:00:00Z'
---

# The map

| Path | What it is | Authoritative for |
|---|---|---|
| `bower-spec.md` | [The design specification](/references/bower-spec.md), Draft 0.2. | Anything not yet built. |
| `README.md` | The front door: crates, invariants, quick tour, every workflow. | Orientation. Carries one known stale detail — the elision leader. |
| `docs/EPIC-01…08_*.md` | Eight closed EPICs, one per phase-4 surface. | *Why* a component is shaped the way it is; the alternatives rejected. |
| `docs/DEFECT_Path_Traversal.md` | Path traversal via book-controlled `file=` paths. Fixed, regression-tested. | The threat model around `file=`. |
| `docs/DEFECT_Review_Findings.md` | Three bugs found by review, each fixed test-first. | What review caught that tests did not. |
| `BACKLOG.md` | Outstanding work, refreshed 2026-09-03. | In-flight and next-up work. Contains at least one stale claim. |
| `docs/TECHNICAL_DEBT.md` | 16 open items, 4 closed. | Known gaps, and deliberate allowances such as the MPL-2.0 crate. |
| `docs/superpowers/specs/` | Feature design notes, e.g. `2026-09-06-exercises-design.md`. | Detailed design for individual features. |

# Reading order for someone new

1. [What Bower is](/overview.md) — the inversion and why it matters.
2. `README.md` — the invariant list, which doubles as a feature tour.
3. [The domain model](/model/index.md) — the vocabulary everything else uses.
4. One EPIC, whichever component you are about to touch. They record the
   reasoning, not just the outcome.

# The EPICs are worth reading

They are unusually load-bearing for this project. EPIC-07, for example, records
that the PDF route uses Typst rather than LaTeX because Phase 0 *measured* LaTeX
as non-reproducible even with `SOURCE_DATE_EPOCH` — a decision that looks
arbitrary in the code and obvious in the document.

# Citations

[1] `docs/`, `README.md`, `BACKLOG.md`
