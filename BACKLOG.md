# Backlog

> Refreshed by the `/backlog` skill on 1 September 2026. `bower status`
> shipped after the first pass; EPIC-04 is done.
> Items marked 🤖 were proposed by automation — review before acting on them.
> Debt detail lives in [`docs/TECHNICAL_DEBT.md`](docs/TECHNICAL_DEBT.md).

## In flight

_Nothing. EPIC-03 shipped._


## Next up — unwritten EPICs

| Work | Source | Note |
|---|---|---|
| **Migration** | spec § 11 Phase 5 | Stand up the real *Rust for Failers* mdBook, move the DIARY and doc-comment material chapter by chapter, generate `failers` for real. This is the phase that proves the whole tool. |
| **Publishing maturity** | spec § 11 Phase 6+, § 13, § 14 | epub/PDF targets, notebook target, Obsidian and Scrivener bridges. Explicitly scoped only after Phase 5. |

## Known gaps

Carried from the EPIC corrigenda. Detail and file references in
[`docs/TECHNICAL_DEBT.md`](docs/TECHNICAL_DEBT.md).

- [ ] `bower verify` does not run the book's own gate — the defect it found was caught by eye, not by the tool
- [ ] Multi-repo books are untested; the code loops over repos but no book exercises it
- [ ] No stderr snapshots for `compile_fail` (spec § 12 Q3, decided failure-only)
- [ ] No parallel verification (spec § 6 calls it embarrassingly parallel; 20 steps take 16s sequentially)
- [ ] Play cells (spec § 15) are neither rendered nor verified
- [ ] The epub/pandoc elision rendering is designed but unbuilt (spec § 3.4)

## Open questions — decisions, not code

From `bower-spec.md` § 12. Two are closed; these seven are not.

| # | Question |
|---|---|
| 2 | One repo per book, or one workspace holding both books? |
| 4 | Should generated repos carry GitHub Actions that re-verify on push? |
| 5 | Crate naming on crates.io — is `bower` free? Fallbacks `bower-cli` / `imperial-bower`. **Check before publishing.** |
| 6 | Block library — core or extension? |
| 7 | Wheel distribution for notebooks |
| 8 | `expect` for play cells |
| 9 | JupyterLite / pyodide |
| 10 | `devenv.nix` — hand-authored per repo, or derived? |

## Recently completed

- **[EPIC-01 — replay](docs/EPIC-01_Replay.md)** — 11/11 components. Deterministic git replay, tags, trailers, `STEPS.md`.
- **[EPIC-02 — verification](docs/EPIC-02_Verification.md)** — 8/8 components. `bower verify` against a real compiler.
- **[EPIC-03 — the mdBook preprocessor](docs/EPIC-03_Preprocessor.md)** — 10/10 components. Directives stripped, display markers applied, anchors and line-anchored footers injected.
- **[EPIC-04 — `bower status`](docs/EPIC-04_Status.md)** — 5/5 components. Book ⇄ lock ⇄ repo drift, named and exit-coded.
- **[EPIC-05 — `bower push`](docs/EPIC-05_Push.md)** — 6/6 components. Publishing, behind a guard with no override.
- **The sample book** — `books/hello-playbook/`, six chapters, twenty steps.

## Health

| Signal | State |
|---|---|
| Tests | 194 passing, 0 failing (`make ayce` green from clean) |
| Clippy | 0 warnings, pedantic, `--all-features` |
| Kernel purity | `cargo tree -p bower-core -e normal` prints one line |
| Code markers | none — no `TODO`, `FIXME`, `HACK`, or `XXX` anywhere |
| Open GitHub issues | none |
| `make ayce` | green: clean, fmt, build, test, lint, security-scan, docs |
