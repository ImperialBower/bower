---
type: Decision
title: EPICs are proven on the sample book
description: No EPIC, and no gate, depends on Rust for Failures — hello-playbook and the testkit fixtures are the only books an EPIC may use.
tags: [decision, epics, books, ci]
timestamp: '2026-09-14T00:00:00Z'
---

# The question

May an EPIC's work items, exit criteria, or verification depend on *Rust for
Failures* — the real book — or only on the sample book?

# The decision

**Only the sample book** (14 September 2026). Every EPIC demonstrates and
proves its feature on [`hello-playbook`](/books/hello-playbook.md), plus
`bower-testkit` fixtures. *Rust for Failures* adopts a feature as its own book
work, tracked in `BACKLOG.md`, never as an EPIC's work item.

The gates follow: `make plan`, and so `make build`, plans `hello-playbook`
only, and `slow.yml` no longer runs `make failures`. The `failures*` targets
stay, run by hand.

# Why

EPIC-09's Work Item 4b — "the `from-char` PR in `ch03-rank.md`, once that
chapter is listed" — left a shipped EPIC unclosable, waiting on the real book's
authoring state. When the chapter was listed, it did not resolve against a
rewritten chapter 1, and because `make build` planned both books, the real
book's draft state would have turned bower's own CI red.

A book in progress changes for reasons that have nothing to do with the tool.
The sample book exists to be the tool's proof, and changes only when a feature
needs it.

# What moved

| Was | Now |
|---|---|
| EPIC-09 4b — the `from-char` PR | withdrawn; `hello-playbook` ch07 is the proof |
| EPIC-11 exit criterion 6 — `make failures` catches a reworded E0004 | `make slow` catches a reworded E0308 in `hello-playbook` |
| EPIC-12 5b — a failure index in the real book | withdrawn; 5a's `hello-playbook` placements are the proof |
| EPIC-13 5b, exit criterion 5 — the real book's hidden solution and unchanged SHAs | the sample book only; `rank_saga` holds the "unchanged" claim |
| EPIC-10 verification on the real book | on `hello-playbook` |

# Citations

[1] `docs/EPIC-09_Branches.md` (Decision 25, Work Item 4b), `Makefile`
(`plan`), `.github/workflows/slow.yml`
