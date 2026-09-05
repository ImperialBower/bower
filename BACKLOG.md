# Backlog

> Refreshed 3 September 2026. Eight EPICs closed: replay, verification, the
> mdBook preprocessor, status, push, publish (html + epub), PDF via Typst, and
> the site branch. Since then, three unnumbered pieces of work landed on top:
> covers, a verify defect fix, and releases. `hello-playbook` is live at
> <https://github.com/abstecker/hello-playbook> with its book served from
> `gh-pages`.
> Items marked 🤖 were proposed by automation — review before acting on them.
> Debt detail lives in [`docs/TECHNICAL_DEBT.md`](docs/TECHNICAL_DEBT.md).

## In flight

- **Migration — *Rust for Failures*** (spec § 11 Phase 5). The book lives in
  `books/rust4failures/`, beside the sample, and publishes both its code and
  its rendered site to `folkengine/rust4failures`. Chapter 1 is written and
  green: five steps, one of them a declared `compile_fail`, all five verified
  against a real compiler (`make failures`). Next is the chapter map — the
  teaching order over `pkcore`'s `DIARY.md`, which is deliberately not the
  order the work was done in. Nothing has been pushed to GitHub yet.

## Next up — unwritten EPICs

| Work | Source | Note |
|---|---|---|
| **`--target ipynb`** | spec § 15 | The fourth publishing target. Needs play cells, which nothing renders yet. |
| **Editions** | spec § 13 M2 | A published edition as a pinned triple: book commit, `bower.lock`, repo tags. A reader of the 1.0 epub follows 1.0 links forever while `main` moves on. **`[book] version` is now the first brick of this** — when Editions is written, it should own that key. |
| **Authoring bridges** | spec § 14 | Obsidian and Scrivener. Explicitly scoped only after Phase 5. |

## Known gaps

Carried from the EPIC corrigenda. Detail and file references in
[`docs/TECHNICAL_DEBT.md`](docs/TECHNICAL_DEBT.md).

- [ ] `bower verify` does not run the book's own gate — the defect it found was caught by eye, not by the tool
- [ ] `GitHubForge` is untested, releases included — every `gh` and `git push` call is the acknowledged last inch. It has now cost two real defects (the lease, and the garbled release refusal); `push_branch_args` is the first piece pulled out into something testable
- [ ] No stderr snapshots for `compile_fail` (spec § 12 Q3, decided failure-only)
- [ ] No parallel verification (spec § 6 calls it embarrassingly parallel; 20 steps take 16s sequentially)
- [ ] Play cells (spec § 15) are neither rendered nor verified
- [ ] Nothing reports whether a *published artifact* is current — `status` covers the lock, the repo, and the site, but not the PDF or epub
- [ ] An SVG epub cover is legal but unevenly supported; no reader has been tested

## Open questions — decisions, not code

From `bower-spec.md` § 12. Six are closed (gix, failure-only snapshots, crate
names, repo layout, repo CI, block library); these four are not.

| # | Question |
|---|---|
| 7 | Wheel distribution for notebooks |
| 8 | `expect` for play cells |
| 9 | JupyterLite / pyodide |
| 10 | `devenv.nix` — hand-authored per repo, or derived? |

Decided 4 September 2026: **(4)** yes, generated repos get GitHub Actions that
re-verify on push; **(6)** the block library is core, not an extension.

**(2)** was decided 4 September as one repo per book and **reversed 5
September**: every book's source lives in this workspace under `books/`.
Standing up the first real book settled it by trying the other answer first.
Generated repos are unaffected — each book still publishes to its own GitHub
repository. See `bower-spec.md` § 12.

## Recently fixed

- **A pushed site branch was not a served site.** `bower push` created and
  force-pushed `gh-pages` and stopped there, so on a repository nobody had
  configured by hand GitHub kept its `build_type: "workflow"` default, pointed
  at `main`, and never built once. Everything the tool said was true — the
  branch really was published, `status` really was in sync — while the URL
  404'd indefinitely. Fixed 5 September 2026: `Forge::enable_pages` points
  Pages at the site branch and requests the one build that the branch push
  could not trigger, but only for a branch that run created, and never on a
  repository already serving something. The decision is pure and tested;
  only the `gh api` call is the last inch.

- **`bower push` could publish a repository once and never again.** The branch
  push ran `git push --force-with-lease <url> main:main`. A bare
  `--force-with-lease` compares against a *remote-tracking* ref, and a push to
  a URL has none, so git answered `stale info` and refused — every push after
  a repository's first one, which is the only kind that matters, since a
  generated repo is republished whenever the book changes. The first push to an
  empty remote succeeded, which is why `hello-playbook` shipped and nothing
  looked wrong. Fixed 5 September 2026: `remote_head` reads the branch with
  `git ls-remote` and the lease carries that value explicitly. Still a lease —
  a remote that moved between the read and the push is still refused, proven
  against a local bare repository before the change was written.

- **The book's name changed with the spelling of its path.** `book_name` took
  the book root's last path segment, and `--book` defaults to `.`, which has no
  last segment. Running Bower from inside a book — the normal way — fell back
  instead, and the five call sites do not pass the same fallback: `STEPS.md`
  got the target repo's name and `.bower-site` got the literal `"book"`, so
  `status` called the site stale forever. Invisible in `hello-playbook`, whose
  directory and target repo share one name; found 5 September 2026 on the first
  real Phase 5 book, where they differ. Fixed by normalizing the root with
  `std::path::absolute` before naming it — no filesystem access, so an
  unrendered book still has a name and a symlinked one keeps the name the
  reader typed.

- **`bower verify` blamed the book for its own scratch tree.** The default
  `--work target/bower-verify` sat inside the book's cargo workspace, so cargo
  refused every step's tree and all twenty true claims reported false. No test
  saw it: every test passed `--work` explicitly. Fixed 3 September 2026 — the
  default moved under the system temp directory, and a nested work directory now
  stops with a named error instead of judging the book.

- **[DEFECT: three review findings](docs/DEFECT_Review_Findings.md)** — the
  scaffolding was deleted at step 1 (every generated repo shipped with no
  licence), `verify --step` failed on multi-repo books, and the `show=` key was
  ignored at render time.

- **[DEFECT: path traversal](docs/DEFECT_Path_Traversal.md)** — book-controlled
  `file="…"` and `SUMMARY.md` links reached `Path::join` unvalidated. Found in
  review, reproduced against the real binary, fixed at the I/O boundary, and
  covered by eleven tests.

## Recently completed

- **Releases** (3 September 2026) — `bower push` hangs a GitHub release off
  `[book] version` and attaches every `.pdf`/`.epub` in the repo's `assets`
  directory. Decided locally, published last, refused when half-configured.
  Never yet run against real GitHub.
- **Covers** (3 September 2026) — `cover.svg` plus optional `cover.png`,
  composed into one self-contained SVG before either renderer sees it, so the
  epub and the PDF cannot show different covers. The sample book gained an
  `Appendix: credits` chapter carrying its cover photograph's CC BY-SA 3.0
  attribution.
- **`make ship-hello` / `make ship-hello-execute`** (3 September 2026) — the
  whole sample-book sequence, split so `--execute` is written down once.
- **[EPIC-01 — replay](docs/EPIC-01_Replay.md)** — 11/11 components. Deterministic git replay, tags, trailers, `STEPS.md`.
- **[EPIC-02 — verification](docs/EPIC-02_Verification.md)** — 8/8 components. `bower verify` against a real compiler.
- **[EPIC-03 — the mdBook preprocessor](docs/EPIC-03_Preprocessor.md)** — 10/10 components. Directives stripped, display markers applied, anchors and line-anchored footers injected.
- **[EPIC-04 — `bower status`](docs/EPIC-04_Status.md)** — 5/5 components. Book ⇄ lock ⇄ repo drift, named and exit-coded.
- **[EPIC-05 — `bower push`](docs/EPIC-05_Push.md)** — 6/6 components. Publishing, behind a guard with no override.
- **[EPIC-06 — `bower publish`](docs/EPIC-06_Publish.md)** — 9/9 components. One render plan, two targets: mdBook HTML and a pandoc epub.
- **[EPIC-07 — `--target pdf`](docs/EPIC-07_Pdf.md)** — 6/6 components. Typst, chosen by measurement; the PDF is byte-identical across runs.
- **[EPIC-08 — the site branch](docs/EPIC-08_Site.md)** — 6/6 components. `bower push` ships the rendered book, gated like the code branch, and `status` catches a stale site.
- **The sample book** — `books/hello-playbook/`, seven chapters, twenty steps.

## Health

| Signal | State |
|---|---|
| Tests | 297 passing, 0 failing (`make ayce` green from clean) |
| Slow lanes | 6 `#[ignore]`d, all green (`make slow`) |
| Clippy | 0 warnings, pedantic, `--all-features` |
| Kernel purity | `cargo tree -p bower-core -e normal` prints one line |
| Code markers | none — no `TODO`, `FIXME`, `HACK`, or `XXX` anywhere |
| Open GitHub issues | none |
| `make ayce` | green: clean, fmt, build, test, lint, security-scan, docs |
