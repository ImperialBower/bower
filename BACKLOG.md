# Backlog

> Refreshed 2 September 2026. Eight EPICs closed: replay, verification, the
> mdBook preprocessor, status, push, publish (html + epub), PDF via Typst, and
> the site branch. `hello-playbook` is live at
> <https://github.com/abstecker/hello-playbook> with its book served from
> `gh-pages`.
> Items marked 🤖 were proposed by automation — review before acting on them.
> Debt detail lives in [`docs/TECHNICAL_DEBT.md`](docs/TECHNICAL_DEBT.md).

## In flight

_Nothing. EPIC-03 shipped._


## Next up — unwritten EPICs

| Work | Source | Note |
|---|---|---|
| **Migration** | spec § 11 Phase 5 | Stand up the real *Rust for Failers* mdBook, move the DIARY and doc-comment material chapter by chapter, generate `failers` for real. This is the phase that proves the whole tool. |
| **`--target ipynb`** | spec § 15 | The fourth publishing target. Needs play cells, which nothing renders yet. |
| **Editions** | spec § 13 M2 | A published edition as a pinned triple: book commit, `bower.lock`, repo tags. A reader of the 1.0 epub follows 1.0 links forever while `main` moves on. |
| **Authoring bridges** | spec § 14 | Obsidian and Scrivener. Explicitly scoped only after Phase 5. |

## Known gaps

Carried from the EPIC corrigenda. Detail and file references in
[`docs/TECHNICAL_DEBT.md`](docs/TECHNICAL_DEBT.md).

- [ ] `bower verify` does not run the book's own gate — the defect it found was caught by eye, not by the tool
- [ ] `bower verify`'s default `--work target/bower-verify` is inside this cargo workspace, so `cargo check` refuses every step's tree ("current package believes it's in a workspace when it's not") and all 20 claims report false. Passing `--work` outside the workspace works. `make ship-hello` does exactly that; the CLI default should too. The verification test suite never saw it because it uses `std::env::temp_dir()` (`bower/tests/verification.rs:24`)
- [ ] No stderr snapshots for `compile_fail` (spec § 12 Q3, decided failure-only)
- [ ] No parallel verification (spec § 6 calls it embarrassingly parallel; 20 steps take 16s sequentially)
- [ ] Play cells (spec § 15) are neither rendered nor verified
- [ ] The epub/pandoc elision rendering is designed but unbuilt (spec § 3.4)

## Open questions — decisions, not code

From `bower-spec.md` § 12. Three are closed (gix, failure-only snapshots, crate
names); these six are not.

| # | Question |
|---|---|
| 2 | One repo per book, or one workspace holding both books? |
| 4 | Should generated repos carry GitHub Actions that re-verify on push? |
| 6 | Block library — core or extension? |
| 7 | Wheel distribution for notebooks |
| 8 | `expect` for play cells |
| 9 | JupyterLite / pyodide |
| 10 | `devenv.nix` — hand-authored per repo, or derived? |

## Recently fixed

- **[DEFECT: three review findings](docs/DEFECT_Review_Findings.md)** — the
  scaffolding was deleted at step 1 (every generated repo shipped with no
  licence), `verify --step` failed on multi-repo books, and the `show=` key was
  ignored at render time.

- **[DEFECT: path traversal](docs/DEFECT_Path_Traversal.md)** — book-controlled
  `file="…"` and `SUMMARY.md` links reached `Path::join` unvalidated. Found in
  review, reproduced against the real binary, fixed at the I/O boundary, and
  covered by eleven tests.

## Recently completed

- **[EPIC-01 — replay](docs/EPIC-01_Replay.md)** — 11/11 components. Deterministic git replay, tags, trailers, `STEPS.md`.
- **[EPIC-02 — verification](docs/EPIC-02_Verification.md)** — 8/8 components. `bower verify` against a real compiler.
- **[EPIC-03 — the mdBook preprocessor](docs/EPIC-03_Preprocessor.md)** — 10/10 components. Directives stripped, display markers applied, anchors and line-anchored footers injected.
- **[EPIC-04 — `bower status`](docs/EPIC-04_Status.md)** — 5/5 components. Book ⇄ lock ⇄ repo drift, named and exit-coded.
- **[EPIC-05 — `bower push`](docs/EPIC-05_Push.md)** — 6/6 components. Publishing, behind a guard with no override.
- **[EPIC-06 — `bower publish`](docs/EPIC-06_Publish.md)** — 9/9 components. One render plan, two targets: mdBook HTML and a pandoc epub.
- **[EPIC-07 — `--target pdf`](docs/EPIC-07_Pdf.md)** — 6/6 components. Typst, chosen by measurement; the PDF is byte-identical across runs.
- **[EPIC-08 — the site branch](docs/EPIC-08_Site.md)** — 6/6 components. `bower push` ships the rendered book, gated like the code branch, and `status` catches a stale site.
- **The sample book** — `books/hello-playbook/`, six chapters, twenty steps.

## Health

| Signal | State |
|---|---|
| Tests | 270 passing, 0 failing (`make ayce` green from clean) |
| Clippy | 0 warnings, pedantic, `--all-features` |
| Kernel purity | `cargo tree -p bower-core -e normal` prints one line |
| Code markers | none — no `TODO`, `FIXME`, `HACK`, or `XXX` anywhere |
| Open GitHub issues | none |
| `make ayce` | green: clean, fmt, build, test, lint, security-scan, docs |
