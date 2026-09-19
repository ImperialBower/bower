# Backlog

> Refreshed 13 September 2026, after EPIC-11 (captured diagnostics) merged as
> PR #5 (`d8f24a4`). Nine EPICs closed: replay, verification, the mdBook
> preprocessor, status, push, publish (html + epub), PDF via Typst, the site
> branch, and captured diagnostics. Three unnumbered pieces of work landed on top:
> covers, a verify defect fix, and releases. `hello-playbook` is live at
> <https://github.com/abstecker/hello-playbook> with its book served from
> `gh-pages`.
> 
> Filed 9 September 2026: [a design for self-hosted
> forges](docs/DESIGN_Forges.md) — one Forgejo container per book, so a
> generated repo, its releases, its site, and its CI can exist with GitHub
> out of the loop.
> 
> Filed 10 September 2026: two EPICs with spikes behind them —
> [EPIC-09 Branches](docs/EPIC-09_Branches.md) and
> [EPIC-10 Voice](docs/EPIC-10_Voice.md) — a Jupyter proof of concept for
> `--target ipynb`, and [a concept doc of 23 enhancement
> ideas](docs/bower-ideas.md), folded in below under § Ideas.
> Items marked 🤖 were proposed by automation — review before acting on them.
> Debt detail lives in [`docs/TECHNICAL_DEBT.md`](docs/TECHNICAL_DEBT.md).
> Re-checked 14 September 2026: EPIC-09 is shipped — slice 1 in PR #6, slice
> 2 in PR #7, the post-merge review's three fixes in PR #8. The same day, every
> EPIC was cut loose from *Rust for Failures*: an EPIC is proven on
> `hello-playbook` alone, and neither `make build` nor CI builds the real book.
> Its former EPIC work items are under § In flight.
> 
> Re-checked 15 September 2026 against EPIC-09 (PR #10 open,
> `docs/epic-09-followups`): nothing in the EPIC is unfinished. What it left
> out is under § Deferred from EPIC-09, its Forgejo question is
> `DESIGN_Forges.md` § 11 question 5, and its thirteen open debt items are
> summarized under § Known gaps.
> 
> 18 September 2026: Fixed tech-debt item 9 (branch names unescaped in link
> templates) — `branch_link` now escapes both link text and URL sides. Twelve
> EPIC-09 debt items remain open; see [`docs/TECHNICAL_DEBT.md`](docs/TECHNICAL_DEBT.md).
> 
> 18 September 2026, later: `bower verify` runs steps on four workers
> (`1715314`, fixed in `d2f04c6`), which closes the *no parallel
> verification* gap. The sample book went from 28–32 s to 15–17 s. Three 🤖
> notes on the worker pool are in the debt doc. Later that day `verify` gained
> a per-command `timeout` (default ten minutes), `--jobs N`, and a stop on the
> first error. That closed the timeout finding and two of the three notes.

## In flight

- **Migration — *Rust for Failures*** (spec § 11 Phase 5). The book lives in
  `books/rust4failures/`, beside the sample, and publishes both its code and
  its rendered site to `abstecker/rust4failures`. `SUMMARY.md` lists
  `ch01-local_development.md` (five steps, one a declared `compile_fail`) and
  `ch02-cicd.md` (one step), all verified against a real compiler
  (`make failures`, run by hand — no gate and no EPIC depends on this book),
  with `toolchain = "1.98.1"` pinned since EPIC-11.
  `ch03-rank.md` is being listed (14 September 2026) but does not resolve yet:
  it edits a `mods` region of `src/lib.rs` and a `dev-dependencies` region of
  `Cargo.toml` that the rewritten ch01 no longer creates, and its `pass` steps
  sit on ch01's still-failing kata. Book work that EPICs used to carry, now
  the book's own: the `from-char` branch and PR (was EPIC-09 4b), a failure
  index (was EPIC-12 5b), and a hidden solution if the chapter map wants one
  (dropped from EPIC-13 5b). Next is the chapter map — the teaching order over
  `pkcore`'s `DIARY.md`, which is deliberately not the order the work was done
  in.
  🤖 *Check before a push:* two `REJECTED_` drafts are listed in `SUMMARY.md`,
  so they would render into the published site.

## Next up — not started

| Work | Source | Note |
|---|---|---|
| **`--target ipynb`** | spec § 15 | The fourth publishing target. Needs play cells, which nothing renders yet. A Python proof of concept at `docs/spikes/bower-jupyter/bower-ipynb-poc/` renders one chapter to a notebook and verifies it headless, against the `pkcore.py` wheel, not a book-grown `bindings/` crate. It exercises the three play-cell plan errors and `expect="raises"` (open question 8). |
| **Back matter** | [`docs/EPIC-12_Back_Matter.md`](docs/EPIC-12_Back_Matter.md) | Idea A9. A step index, a failure index, and a file index, placed by an author-written `<!-- bower index="…" -->`. Built on EPIC-11 for error codes and failing test names. The step, by-step failure, and file indices can ship first. Found that the PDF drops step anchors today, so Phase 3 fixes print links. Seven phases; seven open questions. |
| **Exercises, finished** | [`docs/EPIC-13_Exercises.md`](docs/EPIC-13_Exercises.md) | Idea A4. Exercises already ship (PR #4, `fdcd09b`). This EPIC adds `tests=`, `solution=`, and `reveal=`. Verify then checks that exactly the named tests fail, then pass (today any non-zero exit counts). `reveal="never"` closes the *hidden solution* gap below. Six phases; six open questions. |
| **`bower follow`** | [`docs/EPIC-14_Follow.md`](docs/EPIC-14_Follow.md) | Idea A5. A light `bower-follow` binary for readers: `start`, `next`, `next --hands-on`, `check`, `diff`, `where`. Built on EPIC-13. A generated repo cannot rebuild its own verification today, so Phase 0 writes a machine-readable `.bower/steps` file into the final tree. Five phases; seven open questions. |
| **The scrubber** | [`docs/EPIC-15_Scrubber.md`](docs/EPIC-15_Scrubber.md) | Idea A3. About 11 KB of JSON per repo, and a widget under 16 KB with no dependencies, in the HTML book only. It is the project's first JavaScript. The reverse line map the idea assumed does not exist, so the kernel gains a line diff. Shares EPIC-12's file history. Eight phases; six open questions. |
| **Editions** | [`docs/EPIC-16_Editions.md`](docs/EPIC-16_Editions.md) | Spec § 13 M2 with ideas B1, B3, B4 and C4. `bower edition cut` writes one file that is both the pin and the verification receipt. Adds `edition-<id>/…` step tags that never move, and releases that only ever gain files. Owns `[book] version`. Found that today's `--clobber` and `push --force --tags` would overwrite a frozen edition. B4's toolchain board and B1's DOIs are deferred. Six phases; eight open questions. |
| **Authoring bridges** | spec § 14 | Obsidian and Scrivener. Explicitly scoped only after Phase 5. |
| **Forges** | [`docs/DESIGN_Forges.md`](docs/DESIGN_Forges.md) | A `forgejo` `Forge` kind, named forges in `bower.toml`, per-forge link templates and rendering, and `bower forge up/down/status` over a three-service compose file. Also the first way to test the `Forge` trait end to end — the known gap below. Design written, not scheduled; five open questions of its own (§ 11), one of them EPIC-09's: whether Forgejo marks a PR merged when main is pushed past it. |
| **Voice** | [`docs/EPIC-10_Voice.md`](docs/EPIC-10_Voice.md) | The book as the source of truth for the ear: `<!-- voice … -->` cues in the prose, a `bower-voice` kernel beside `bower-core` that folds them into a narrator's script, a breakdown, and a palette fit (which characters a narrator can reach, and which two collide). Spike at `docs/spikes/bower-voice-spike/` — 41 tests, three mutations caught. Filed 10 September 2026; Phase 1 not started; five open questions (§ Open questions). |

## Ideas — not yet EPICs

From [`docs/bower-ideas.md`](docs/bower-ideas.md) (concept doc 0.1, 10
September 2026). Nothing there is a commitment. Its admission rule: every idea
is a pure function of what the kernel already computes, or adds at most one
annotation key. The doc was drafted as if only Phase 1 were built. Read its
"Attaches to" column against what has shipped since.

Ranks 1–7 of its shortlist are now EPICs. EPIC-11 has shipped; the rest are
Planned. Each EPIC records where the idea met shipped code and changed shape:

| Rank | Idea | EPIC |
|---|---|---|
| 1 | **A1** captured diagnostics | [EPIC-11](docs/EPIC-11_Diagnostics.md) — shipped |
| 2 | **A9** generated back matter | [EPIC-12](docs/EPIC-12_Back_Matter.md) |
| 3 | **A4 + A5** exercises and `bower follow` | [EPIC-13](docs/EPIC-13_Exercises.md), [EPIC-14](docs/EPIC-14_Follow.md) |
| 4 | **A3** the scrubber | [EPIC-15](docs/EPIC-15_Scrubber.md) |
| 5 | **A6** verified alternate branches | [EPIC-09](docs/EPIC-09_Branches.md) |
| 6–7 | **B1 + B3 + B4 + C4** permalinks, errata, toolchain board, receipt | [EPIC-16](docs/EPIC-16_Editions.md). B1's DOIs and B4's board are deferred inside it. |

The rest stay ideas:

| Rank | Idea | Where it lands here |
|---|---|---|
| 8 | **A2** wasm components in the browser | Answers open question 9 by going around pyodide. |
| 9–14 | B5 slides and workshops, B2 verified translations, C1 + C2 other authors and a PR bot, C3 classroom, A7/A8/B6/B7/C5/C7, C6 hosted | Wait for their trigger (a talk date, a translator, a second author). A8 is half done: the PDF and epub ship, the margin tags and QR codes do not. |

The doc's Part F carries eight open questions of its own. Q7 is the big one:
is Bower-for-others a goal, or a side effect?

### Deferred from EPIC-09

What [EPIC-09](docs/EPIC-09_Branches.md) left out on purpose, each waiting for
a chapter that needs it. Any of them is an EPIC of its own when it comes, proven
on `hello-playbook`.

| Deferred | Today | Trigger, and what it costs |
|---|---|---|
| **Branches off branches** | `FromNotOnMain`: `from=` must name a main step (Decision 19) | A chapter that forks an experiment from an experiment. The fold generalises — main becomes one more `BranchState` — so the cost is in the lock and the render, not the kernel. |
| **Merges into branches** | `MergeOnBranch`: a merge sits on main (Decision 19) | A chapter that folds one branch into another before main. Lifts with the one above. |
| **Rebases** | Not modelled; a branch's parents are fixed plan values | A chapter about rewriting history. Every SHA after the rebase point moves, so replay's blast-radius golden needs a rebase case. |
| **Closing a PR without merging** | An abandoned branch's PR stays open (open question 3, Decision 25) | A chapter that needs "reviewed, declined" — a *Failures* story. Likely a `pr_state="closed"` key; cheap now that Decision 8 stands. |
| **Review comments as book content** | Not modelled (open question 4) | A real chapter that teaches through a PR conversation. A second body of prose the book must own. |
| **Merging on the forge** | Bower makes every merge commit itself, deterministically (Decision 8) | Probably never: a forge-side merge commit cannot be byte-reproducible. Revisit only if a forge refuses to mark a PR merged by a plain push — see the Forgejo question in [`DESIGN_Forges.md`](docs/DESIGN_Forges.md) § 11. |

## Spikes

Throwaway code that settled a design before an EPIC was written. None is a
workspace member, so `make ayce` never sees them.

| Spike | Behind | State |
|---|---|---|
| [`docs/spikes/bower-voice-spike/`](docs/spikes/bower-voice-spike/) | EPIC-10 | 41 tests green. Promoted, not copied, into `bower-voice/` in Phase 1. |
| [`docs/spikes/bower-jupyter/bower-ipynb-poc/`](docs/spikes/bower-jupyter/bower-ipynb-poc/) | `--target ipynb` | Python and Docker. Not run in this refresh — it needs `pkcore.py` and Jupyter. |
| [`docs/spikes/pr-remote/`](docs/spikes/pr-remote/) | EPIC-09 slice 2 | Two runs against `abstecker/bower-sandbox`, 14 September 2026, settled open questions 1 and 2 for GitHub. The EPIC shipped; the spike stays as the harness for the Forgejo half of question 1, but `run.sh` speaks only `gh`, so a Forgejo run needs its own client first. |

## Known gaps

Carried from the EPIC corrigenda. Detail and file references in
[`docs/TECHNICAL_DEBT.md`](docs/TECHNICAL_DEBT.md).

- [ ] `bower verify` does not run the book's own gate — the defect it found was caught by eye, not by the tool
- [ ] `GitHubForge` is untested, releases included — every `gh` and `git push` call is the acknowledged last inch. It has now cost two real defects (the lease, and the garbled release refusal); `push_ref_args` and `push_refs_args` (which replaced `push_branch_args` in EPIC-09 slice 2) are the first pieces pulled out into something testable
- [ ] Play cells (spec § 15) are neither rendered nor verified
- [ ] Exercises have no place for a *reader's* answer — a per-exercise link to a Discussion or similar (exercises spec § 2) — EPIC-14's `follow check` judges a reader's answer but gives it no place; this narrows the gap and does not close it
- [ ] Exercises cannot carry a hidden solution the book does not print; the answer is always the next step (exercises spec § 2) — EPIC-13's `reveal="never"` closes this
- [ ] Nothing reports whether a *published artifact* is current — `status` covers the lock, the repo, and the site, but not the PDF or epub
- [ ] An SVG epub cover is legal but unevenly supported; no reader has been tested
- [ ] 🤖 Two notes on the verify worker pool: Ctrl-C no longer reaches a running step (the price of the timeout's process group), and old work directories keep a stale `target/`
- [ ] 🤖 Ten open findings from the 15 September 2026 deep review ([`docs/TECHNICAL_DEBT.md`](docs/TECHNICAL_DEBT.md) § Automated review findings). The worst: `verify --record` rewriting chapters in place (the timeout finding closed 18 September)
- [ ] EPIC-09 left thirteen debt items. Two can strand a live repository: a push interrupted while main stands on a stepping stone leaves the remote failing the marker gate for good, and the gate reads `STEPS.md` from the forge's default branch rather than `main`. Most of the rest are edges no book reaches yet: an all-branch book, a branch named like a `-end` tag, unescaped `|` and `)` in branch names and PR titles, and one mistake reported twice. One is nearly free since slice 2: naming remote branches the book dropped, because `schedule` already reads every remote head.

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

- **One failed Pages call left a site unserved for good.** `bower push`
  enabled Pages only for a site branch that run had created. So if the branch
  push worked and the `gh api` call after it failed, every later push found
  the branch already there and never tried again. Nothing reported it either:
  the dry run's `pages` line was gated the same way. That is the "pushed but
  never served" defect of 5 September, reached through a partial failure.
  Fixed 15 September 2026: the plan reads Pages once the gates pass. It
  enables Pages for a new branch as before, or for an existing one Pages is
  not serving yet, and the dry run says which. A branch that is already served
  gets no extra call. Found by the deep automated review.

- **A `site_branch` git read as an option could delete the remote's refs.**
  `site_branch` was never validated, and the site push passed it to `git push`
  as a bare argument. With `site_branch = "--mirror"`, that became a mirror
  push, which deleted every branch and tag the site did not carry. Found by the
  deep automated review and reproduced against a local bare repository. Fixed
  15 September 2026 with two guards: `bower push` refuses any site branch name
  a chapter's own branch could not have, before any forge call, and the site
  push names the branch only inside a `HEAD:refs/heads/<branch>` refspec.

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

- **Parallel verification** (18 September 2026, `1715314` + `d2f04c6`) —
  four workers, each with its own target directory, results in document
  order, and the toolchain probe run one at a time. About 2× on the sample
  book. The first commit's workers held the queue lock for a whole step, so
  they took turns; a review caught it before the branch merged. Then:
  `timeout = <seconds>` per repo (a killed step is broken, whatever it
  claimed), `--jobs N` (default: cores, at most four), and a stop on the
  first error.
- **[EPIC-09 — branches, merges, and pull requests, slice 2](docs/EPIC-09_Branches.md)**
  (14 September 2026, PR #7; review fixes in PR #8) — `bower push` puts every
  declared pull request on the forge through a pure schedule the dry run
  prints: branches and main's first move in one atomic push, a merged branch's
  PR opened on a stepping stone and merged by the push of main, open PRs
  edited when their digest differs, merged and closed PRs left alone, fork PRs
  never touched, and main put back on its head if a push stops on a stone.
  Live on `abstecker/hello-playbook`: `feat/greet-many`'s PR merged by Bower's
  merge commit, `try/shout`'s left open, and a second push opens nothing.
- **[EPIC-09 — branches, merges, and pull requests, slice 1](docs/EPIC-09_Branches.md)**
  (13 September 2026, `feat/branches`) — four directive keys (`branch=`,
  `from=`, `merge=`, `pr=`), a per-line fold, `MergeConflict` at region
  granularity, branch refs and `Bower-Line`/`Bower-Merges` trailers, `PULLS.md`,
  branch drift in `status`, the footer's branch and compare links, a merge's
  own `step-meta` line, and `bower push` publishing every branch with its own
  lease. The sample book's `ch07-try-it-on-a-branch.md` carries one unmerged
  branch and one two-parent merge. Slice 2 (the forge's pull requests) followed
  the next day.
- **[EPIC-11 — captured diagnostics](docs/EPIC-11_Diagnostics.md)** (12 September 2026, merged as PR #5) — 9/9 components. `output="check"|"verify"` records what the compiler said as book content; `verify` fails on drift, `--record` writes it, `[...]` trims it. Found and fixed: `verify` under `make` ignored every tree's toolchain pin.
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
| Tests | 604 passing, 0 failing (`cargo test --workspace`, 18 September 2026) |
| Slow lanes | 11 `#[ignore]`d; all green 18 September 2026 (`make slow`, the 25-step sweep in about 17 s) |
| Clippy | 0 warnings, pedantic, `--all-features` |
| Kernel purity | `cargo tree -p bower-core -e normal` prints one line |
| Code markers | none — no `TODO`, `FIXME`, `HACK`, or `XXX` anywhere |
| Open GitHub issues | none |
| `make ayce` | green: clean, fmt, build, test, lint, security-scan, docs |
| CI | GitHub Actions: `ci.yml` (the `ayce` lanes, in parallel) on every push and PR; `slow.yml` (the `#[ignore]`d lanes and the sample book rendered) on `main`, weekly, and on demand |
