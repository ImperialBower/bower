# Backlog

> Refreshed 10 September 2026. Eight EPICs closed: replay, verification, the
> mdBook preprocessor, status, push, publish (html + epub), PDF via Typst, and
> the site branch. Since then, three unnumbered pieces of work landed on top:
> covers, a verify defect fix, and releases. `hello-playbook` is live at
> <https://github.com/abstecker/hello-playbook> with its book served from
> `gh-pages`.
> Filed 9 September 2026: [a design for self-hosted
> forges](docs/DESIGN_Forges.md) — one Forgejo container per book, so a
> generated repo, its releases, its site, and its CI can exist with GitHub
> out of the loop.
> Filed 10 September 2026: two EPICs with spikes behind them —
> [EPIC-09 Branches](docs/EPIC-09_Branches.md) and
> [EPIC-10 Voice](docs/EPIC-10_Voice.md) — a Jupyter proof of concept for
> `--target ipynb`, and [a concept doc of 23 enhancement
> ideas](docs/bower-ideas.md), folded in below under § Ideas.
> Items marked 🤖 were proposed by automation — review before acting on them.
> Debt detail lives in [`docs/TECHNICAL_DEBT.md`](docs/TECHNICAL_DEBT.md).

## In flight

- **Migration — *Rust for Failures*** (spec § 11 Phase 5). The book lives in
  `books/rust4failures/`, beside the sample, and publishes both its code and
  its rendered site to `abstecker/rust4failures`. Chapter 1 is written and
  green: five steps, one of them a declared `compile_fail`, all five verified
  against a real compiler (`make failures`). Next is the chapter map — the
  teaching order over `pkcore`'s `DIARY.md`, which is deliberately not the
  order the work was done in. Nothing has been pushed to GitHub yet.

## Next up — not started

| Work | Source | Note |
|---|---|---|
| **Branches** | [`docs/EPIC-09_Branches.md`](docs/EPIC-09_Branches.md) | History with a shape: four directive keys (`branch=`, `from=`, `merge=`, `pr=`) put steps on a branch, merge it, and declare a pull request. A merge is a fold; a conflict is a plan-time `MergeConflict`; parents are plan values, so replay stays byte-identical. Five phases (kernel → replay/status → render → push/PRs → book), all Planned. Spike at `docs/spikes/spike-branches/` — 16 tests. Six open questions (§ Open questions). Absorbs idea A6. PR state on a forge is Phase 3's last inch — the Forges design would be its first real test target. |
| **`--target ipynb`** | spec § 15 | The fourth publishing target. Needs play cells, which nothing renders yet. A Python proof of concept at `docs/spikes/bower-jupyter/bower-ipynb-poc/` renders one chapter to a notebook and verifies it headless, against the `pkcore.py` wheel, not a book-grown `bindings/` crate. It exercises the three play-cell plan errors and `expect="raises"` (open question 8). |
| **Editions** | spec § 13 M2 | A published edition as a pinned triple: book commit, `bower.lock`, repo tags. A reader of the 1.0 epub follows 1.0 links forever while `main` moves on. **`[book] version` is now the first brick of this** — when Editions is written, it should own that key. |
| **Authoring bridges** | spec § 14 | Obsidian and Scrivener. Explicitly scoped only after Phase 5. |
| **Forges** | [`docs/DESIGN_Forges.md`](docs/DESIGN_Forges.md) | A `forgejo` `Forge` kind, named forges in `bower.toml`, per-forge link templates and rendering, and `bower forge up/down/status` over a three-service compose file. Also the first way to test the `Forge` trait end to end — the known gap below. Design written, not scheduled; four open questions of its own (§ 11). |
| **Voice** | [`docs/EPIC-10_Voice.md`](docs/EPIC-10_Voice.md) | The book as the source of truth for the ear: `<!-- voice … -->` cues in the prose, a `bower-voice` kernel beside `bower-core` that folds them into a narrator's script, a breakdown, and a palette fit (which characters a narrator can reach, and which two collide). Spike at `docs/spikes/bower-voice-spike/` — 41 tests, three mutations caught. Filed 10 September 2026; Phase 1 not started; five open questions (§ Open questions). |

## Ideas — not yet EPICs

From [`docs/bower-ideas.md`](docs/bower-ideas.md) (concept doc 0.1, 10
September 2026). Nothing there is a commitment. Its admission rule: every idea
is a pure function of what the kernel already computes, or adds at most one
annotation key. The doc was drafted as if only Phase 1 were built. Read its
"Attaches to" column against what has shipped since.

Its ranked shortlist, mapped onto this backlog:

| Rank | Idea | Where it lands here |
|---|---|---|
| 1 | **A1** captured diagnostics — `<!-- bower output … -->`, `verify --record`, drift fails the build | Closes the *stderr snapshots* gap below and retires spec Q3. The doc's recommended first move. |
| 2 | **A9** generated back matter — the failure index (by rustc error code), step, file and play indices | Wants A1 first. "A weekend once A1 exists." |
| 3 | **A4 + A5** exercises (`exercise=`, `solution=`) and `bower follow`, the reader's CLI | A4 half closes the *hidden solution* gap below. A5 is the seed of C3 (classroom). |
| 4 | **A3** the scrubber — a step slider over each repo's tree, with a "written by" gutter | The project's first front-end code. |
| 5 | **A6** verified alternate branches | **Now [EPIC-09](docs/EPIC-09_Branches.md).** EPIC-09 answers the doc's Part F Q2: one plan per repo, branches carried on `RepoPlan`. |
| 6 | **B1 + B3 + C4** step permalinks and DOIs, errata feeds, the verification receipt | Belong to **Editions** above. |
| 7 | **B4** living vs frozen edition — a per-step toolchain board | Belongs to **Editions**. Wants parallel verification (gap below). |
| 8 | **A2** wasm components in the browser | Answers open question 9 by going around pyodide. |
| 9–14 | B5 slides and workshops, B2 verified translations, C1 + C2 other authors and a PR bot, C3 classroom, A7/A8/B6/B7/C5/C7, C6 hosted | Wait for their trigger (a talk date, a translator, a second author). A8 is half done: the PDF and epub ship, the margin tags and QR codes do not. |

The doc's Part F carries eight open questions of its own. Q7 is the big one:
is Bower-for-others a goal, or a side effect?

## Spikes

Throwaway code that settled a design before an EPIC was written. None is a
workspace member, so `make ayce` never sees them.

| Spike | Behind | State |
|---|---|---|
| [`docs/spikes/spike-branches/`](docs/spikes/spike-branches/) | EPIC-09 | 16 tests green. Delete after EPIC-09 Phase 0 (EPIC-09 Q6). |
| [`docs/spikes/bower-voice-spike/`](docs/spikes/bower-voice-spike/) | EPIC-10 | 41 tests green. Promoted, not copied, into `bower-voice/` in Phase 1. |
| [`docs/spikes/bower-jupyter/bower-ipynb-poc/`](docs/spikes/bower-jupyter/bower-ipynb-poc/) | `--target ipynb` | Python and Docker. Not run in this refresh — it needs `pkcore.py` and Jupyter. |

## Known gaps

Carried from the EPIC corrigenda. Detail and file references in
[`docs/TECHNICAL_DEBT.md`](docs/TECHNICAL_DEBT.md).

- [ ] `bower verify` does not run the book's own gate — the defect it found was caught by eye, not by the tool
- [ ] `GitHubForge` is untested, releases included — every `gh` and `git push` call is the acknowledged last inch. It has now cost two real defects (the lease, and the garbled release refusal); `push_branch_args` is the first piece pulled out into something testable
- [ ] No stderr snapshots for `compile_fail` (spec § 12 Q3, decided failure-only) — idea A1 reopens this as visible book content
- [ ] No parallel verification (spec § 6 calls it embarrassingly parallel; 20 steps take 16s sequentially)
- [ ] Play cells (spec § 15) are neither rendered nor verified
- [ ] Exercises have no place for a *reader's* answer — a per-exercise link to a Discussion or similar (exercises spec § 2)
- [ ] Exercises cannot carry a hidden solution the book does not print; the answer is always the next step (exercises spec § 2) — idea A4 half answers this: it collapses the solution, but the solution is still a step
- [ ] Nothing reports whether a *published artifact* is current — `status` covers the lock, the repo, and the site, but not the PDF or epub
- [ ] An SVG epub cover is legal but unevenly supported; no reader has been tested

## Open questions — decisions, not code

From `bower-spec.md` § 12. Six are closed (gix, failure-only snapshots, crate
names, repo layout, repo CI, block library); these four are not.

| # | Question |
|---|---|
| 7 | Wheel distribution for notebooks |
| 8 | `expect` for play cells — the Jupyter POC tried `expect="raises"` and it works |
| 9 | JupyterLite / pyodide — idea A2 proposes going around it with wasm |
| 10 | `devenv.nix` — hand-authored per repo, or derived? |

Decided 4 September 2026: **(4)** yes, generated repos get GitHub Actions that
re-verify on push; **(6)** the block library is core, not an extension.

**(2)** was decided 4 September as one repo per book and **reversed 5
September**: every book's source lives in this workspace under `books/`.
Standing up the first real book settled it by trying the other answer first.
Generated repos are unaffected — each book still publishes to its own GitHub
repository. See `bower-spec.md` § 12.

## Recently fixed

- **A book could render with every directive still on the page.** mdBook 0.5
  renamed the top-level key of the preprocessor envelope from `sections` to
  `items`. `bower::mdbook` read only `sections`, so under 0.5 it found no
  chapters, handed the book straight back, and mdBook reported a successful
  build of a book whose directives had never been applied — no error anywhere,
  on either side. Fixed 7 September 2026: both names are read, newest first,
  and the 0.5 envelope shape is a unit test rather than a version to keep up
  with. The `#[ignore]`d preprocessor test is what caught it, which is the
  argument for running the slow lanes in CI.

- **Two `#[ignore]`d publish tests had rotted.** They opened
  `hello-playbook.epub` and `hello-playbook.pdf`; an artifact has carried the
  book's version in its name since `[book] version` landed, so the files were
  `hello-playbook_0.1.0.epub` and `.pdf` and the tests failed on the one thing
  that had not gone wrong. Fixed 7 September 2026: they ask `artifact_name`
  what the file is called instead of repeating the answer.

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
| Tests | 358 passing, 0 failing (`make ayce` green from clean) |
| Slow lanes | 6 `#[ignore]`d, all green (`make slow`) |
| Clippy | 0 warnings, pedantic, `--all-features` |
| Kernel purity | `cargo tree -p bower-core -e normal` prints one line |
| Code markers | none — no `TODO`, `FIXME`, `HACK`, or `XXX` anywhere |
| Open GitHub issues | none |
| `make ayce` | green: clean, fmt, build, test, lint, security-scan, docs |
| CI | GitHub Actions: `ci.yml` (the `ayce` lanes, in parallel) on every push and PR; `slow.yml` (the `#[ignore]`d lanes, both books rendered) on `main`, weekly, and on demand |
