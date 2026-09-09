---
type: Playbook
title: CI — the fast lane and the slow lane
description: ci.yml splits the gate into five parallel jobs on every PR; slow.yml runs the tool-dependent lanes on main only, with fonts pinned by hand.
tags: [operations, ci, github-actions]
timestamp: '2026-09-09T00:00:00Z'
---

# Two workflows

| Workflow | Triggers | Contents |
|---|---|---|
| `ci.yml` | push to `main`, **every PR**, Mondays 06:30 UTC, dispatch | Five parallel jobs, each calling the Makefile target it mirrors. |
| `slow.yml` | push to `main`, Mondays 06:00 UTC, dispatch — **not on PRs** | The tool-dependent lanes. |

Concurrency cancels superseded runs.

# The fast lane

`ci.yml` splits [`make ayce`](/operations/make-ayce.md) into five jobs — `fmt`
(via `make fmt-check`), `test` (`make build` then `make test`), `lint`, `docs`,
and `security` (installing `cargo-audit` and `cargo-deny`, then
`make security-scan`).

Each job calls the Makefile rather than reimplementing it, so the gate has one
definition and a contributor's local sweep is the same sweep.

# The slow lane

`slow.yml` installs `typst`, `mdbook`, `cargo-audit`, `cargo-deny`, `pandoc`,
and **Libertinus 7.051 pinned from a GitHub release** — Ubuntu 24.04 has no
`fonts-libertinus` package. It sets `TYPST_FONT_PATHS`, asserts `typst fonts`
can see `Libertinus Serif` and `DejaVu Sans Mono`, then runs
`make check-dependencies`, `make slow`, `make book`, and `make failures`.

**`mdbook` is deliberately unpinned**, so that an envelope rename like mdBook
0.4 → 0.5 is *reported* rather than hidden. See
[the preprocessor](/architecture/mdbook-bower.md), which reads both shapes.

# What is `#[ignore]`d, and why

Every ignored test is tool- or compiler-dependent — none is ignored for being
flaky or slow to fix:

| Location | Needs |
|---|---|
| `bower/tests/preprocessor.rs` | `mdbook` and `mdbook-bower` on `PATH` |
| `bower/tests/verification.rs` | A real compiler, per step |
| `bower/tests/publish.rs` | pandoc; pandoc + typst |
| `bower/src/publish.rs` (4 unit tests) | pandoc, typst, and the pinned fonts |

# The supporting scripts

- `bin/check-dependencies` — one definition of what the repo needs, each with
  its install command. Required: `cargo`, `git`, `pandoc`, `typst`,
  `cargo-audit`, `cargo-deny`. Optional (warn only): `mdbook`, `gh`.
- `bin/security-scan` — `cargo audit`, then
  `cargo deny check advisories bans licenses sources`.
- `bin/bookfiles` — walks the `<!-- bower … -->` markers under `books/` in
  reading order. `bookfiles <repo>` lists steps; `bookfiles <repo> <step>` lists
  the files that step writes.

# License policy

`deny.toml` allows exactly `Apache-2.0`, `MIT`, `Unicode-3.0`, `BSD-3-Clause`,
`Zlib`, and `MPL-2.0`. The last is there for a single crate — `uluru`, pulled in
via `gix-pack` — and is recorded as a deliberate decision in
`docs/TECHNICAL_DEBT.md` rather than left as an unexplained allowance. Unknown
registries and git sources are denied; multiple versions warn.

# Citations

[1] `.github/workflows/ci.yml`, `.github/workflows/slow.yml`, `bin/`, `deny.toml`
