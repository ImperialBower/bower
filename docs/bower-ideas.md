# Bower — Enhancement Concepts

**Reader experience, distribution & imprint, and Bower as a product**
Concept doc 0.1 · 10 September 2026 · ImperialBower
Companion to *Bower — Design Specification* draft 0.2

> **Status, 10 September 2026.** Ranks 1–7 of Part E are now EPICs:
> A1 → [EPIC-11](EPIC-11_Diagnostics.md), A9 → [EPIC-12](EPIC-12_Back_Matter.md),
> A4 → [EPIC-13](EPIC-13_Exercises.md), A5 → [EPIC-14](EPIC-14_Follow.md),
> A3 → [EPIC-15](EPIC-15_Scrubber.md), A6 → [EPIC-09](EPIC-09_Branches.md),
> B1 + B3 + B4 + C4 → [EPIC-16](EPIC-16_Editions.md). The EPICs win where they
> differ from this doc. The rest are still ideas.

---

## 0. Purpose and the admission rule

The spec (draft 0.2) settles the core: book source in, deterministic plan and
tree states out, git replay and verification at the edges, with a maturity
ladder (§ 13), authoring bridges (§ 14), a notebook target (§ 15), and reader
environments (§ 16). This document goes looking for what *else* the same
machinery can produce, weighted toward three directions: what the reader
holds in their hands, how the books travel, and what Bower is if it is a
product rather than a private build tool.

Every idea here had to pass one admission rule, in the spirit of § 14.4:
**it must be a pure function of things the kernel already computes** — the
`Plan`, the `TreeState` sequence, the link map, the `expect` matrix, and the
verification results — or extend the annotation vocabulary by at most one
key. New targets, not new sources. An idea that needed a second source of
truth was cut. This keeps every enhancement auditable the way the repos are:
same book, same output, byte for byte.

Each idea is written up the same way — *what*, *mechanism*, *thesis fit*,
*cost*, *spec placement*, *prior art* — and Part E ranks them. Nothing here
is a commitment; the spec's build plan (§ 11) is unchanged, and Part E says
which phase each idea would attach to.

### 0.1 Summary

| Id | Idea | Direction | Cost | Attaches to |
|---|---|---|---|---|
| A1 | Captured diagnostics as verified book content | Reader | S | Phase 3 |
| A2 | Zero-install play: wasm components in the browser | Reader | L | Phase 4+ |
| A3 | The scrubber: time-travel over the tree | Reader | M | Phase 4 |
| A4 | Exercises: hand the reader the failing state | Reader | M | Phase 3 |
| A5 | `bower follow`: the reader's CLI | Reader | M | Phase 2–3 |
| A6 | The road not taken: verified alternate branches | Reader | M | Phase 2 |
| A7 | Aquascope and quizzes: integrate, don't build | Reader | S | Phase 4 |
| A8 | Print and epub as first-class readers | Reader | S–M | M1 |
| A9 | Generated back matter: the failure index | Reader | S | M1 |
| B1 | Step permalinks, citations, DOIs | Imprint | S | M2 |
| B2 | Verified translations | Imprint | M | M2 |
| B3 | Errata and contributions as a product surface | Imprint | S | M2 |
| B4 | The living edition and the frozen edition | Imprint | M | M2 |
| B5 | Slides and workshops from the same plan | Imprint | M | M1 |
| B6 | Fitting the storefronts | Imprint | S | M1 |
| B7 | The concept index and the rustdoc rendition | Imprint | S–M | M4 |
| C1 | Bower for other authors | Product | S–M | after Phase 5 |
| C2 | The Bower PR bot | Product | M | M3 |
| C3 | Classroom | Product | M | after A4 |
| C4 | "Bower-verified" as a claim, not a mark | Product | S | M2 |
| C5 | Bundle and pricing shapes | Product | — | M2 |
| C6 | Bower Press, hosted | Product | L | far |
| C7 | Reader signals, opt-in | Product | S | after A5 |

---

## Part A — Reader experience

### A1. The compiler is a co-author: captured diagnostics as book content

**What.** In a book about controlled failing, the compiler's message *is*
the content of a `compile_fail` step, and today it would be pasted by hand
and rot silently. Instead, `bower verify --record` captures stderr and test
output at every step, and the book embeds it by reference.

**Mechanism.** One new directive form:

```markdown
<!-- bower output step="rank-from-char-naive" kind="stderr" -->
```text
error[E0004]: non-exhaustive patterns: `'Z'` not covered
  --> src/rank.rs:14:15
```​
```

The fenced body is *generated*: `bower verify --record` rewrites it in the
book source, normalized (paths relative to the repo root, timings stripped,
ANSI stripped, the `Compiling`/`Finished` chatter dropped). A subsequent
plain `bower verify` compares the live output to the embedded text and
fails on drift — the same discipline as `insta` snapshots and the Rust
Book's per-listing `output.txt` files. Normalization is a pure function in
`bower-core` with its own textures in the testkit (a diagnostic with a
multi-line note, a test panic with a backtrace, a warning-only step).
`kind` covers `stderr`, `stdout`, `test`, and `check` (`cargo check` only).

This absorbs spec open question 3: failure-only stays the default for
`expect`, and an `output` block is the snapshot opt-in — but as *visible
book content* rather than a hidden fixture, which is the only kind of
snapshot an author will keep honest.

**Thesis fit.** Highest of anything in this document. *Rust for Failers*
says the compiler is the first observability tool; this makes every
diagnostic the book prints an executable claim, verified on every build,
and the day rustc rewords E0004 the book build fails before a reader
notices. For *Controllability* the captured test output is telemetry from
the book's own plant.

**Cost.** S. A directive key, a normalizer, a `--record` flag on the
verifier, and the preprocessor rendering the block as a diagnostic (with
the rustc error code linked to the error index, which A9 then exploits).

**Spec placement.** § 3.2 key table (`output` as a block kind alongside
`notebook`); § 6 verification table gains the drift check; retires open
question 3.

**Prior art.** The Rust Book's `listings/*/output.txt` with
`tools/update-rustc.sh`; `insta`; `trybuild`'s `.stderr` files.

### A2. Zero-install play: wasm components in the browser

**What.** § 15 makes chapters playable through Python and Jupyter, and
§ 15.4 notes that a browser-only notebook is blocked on pyodide loading
native wheels. There is a second road that does not go through Python at
all: the chapter-end state compiled as a WebAssembly *component*, run in
the HTML book itself.

**Mechanism.** The domain-kernel boundary is already a WIT world in the
pkcore ecosystem, and § 15.2 already makes the bindings crate book content.
The same move, one crate over: the repo template ships a `wit/` world and a
`component/` crate; chapters grow the world step by step, as ordinary
blocks. At each chapter-end tag, CI builds the component and `jco
transpile` turns it into an ES module with a JS binding. The rendered
chapter loads that module and play cells written in JavaScript run
against it, in-page, with no toolchain, no Nix, no kernel.

Play cells gain one key, `lang`, defaulting to `python`. A `lang="js"`
cell renders live in HTML; a Python cell renders live in the notebook
target and as a static "try it" aside in HTML, exactly as § 15.1 already
specifies. The kernel's job is unchanged — bind the cell to its step,
refuse tree-affecting keys — and it does not care what language the cell
is in. That is the point: the play surface is whatever the WIT world
exports, and any language jco or wasmtime can host is a valid reader.

**Thesis fit.** This *is* the domain-kernel argument, demonstrated by the
book about it: because the kernel has a language-neutral boundary, the
same tree state is playable from Python, from a browser, from Ruby
(`pkrb`), without the book forking. The pyodide route by contrast makes
the browser wait on Python's packaging story.

**Cost.** L. Component build in CI per chapter-end tag, a jco step in
`publish`, a small in-page runner (a JS cell with a run button and an
output pane — deliberately not a full REPL), and the authoring cost of a
WIT world in every repo that wants it. Only repos that opt in pay it.
Component-model support in browsers is a moving target; jco's transpile
path has been stable since its 1.0 and is the dependency to pin.

**Spec placement.** § 15 gains a subsection "15.5 The web track" parallel
to the play track; § 12 open question 9 is answered by "go around."

**Prior art.** jco transpile; the `pkcore.py` and `pkrb` patterns already
in the ecosystem; Observable and Pluto notebooks for the in-page cell
feel.

### A3. The scrubber: time-travel over the tree

**What.** The generated repo has a hundred tagged states, and the book
links into them one at a time. A reader following the *shape* of the code
wants to slide through them: watch `src/rank.rs` grow from chapter 1 to
chapter 9, see the diff between any two steps, and — the reverse link —
click a line and be told which paragraph wrote it.

**Mechanism.** `bower publish --target html` emits one static JSON per
repo: the tree at each step as content-addressed blobs plus a step →
(path → blob) map, and the reverse line map (path, line range) → book
anchor that the preprocessor already computes for § 3.4's footers. A
widget in `mdbook-bower` renders: file tree, step slider with chapter
ticks, full-file or diff-against-previous view, and a "written by"
gutter. No server; the JSON is a build artifact like everything else, and
its size is bounded by the same argument that makes replay cheap (small
files, a hundred steps).

Two smaller wins fall out. **Diff-reading mode:** a per-reader toggle on
every annotated block to show it as the diff against the previous tree
state rather than as prose-selected spans — some readers think in diffs.
**The file biography:** a per-file page listing every step that touched
it, which is also the natural home for A9's file index.

**Thesis fit.** Observability of the artifact. The scrubber makes the
claim "every intermediate state is a state the tests visited" (§ 6)
something the reader can see rather than take on faith.

**Cost.** M. The JSON is a fold over `TreeState`s (pure, testable); the
widget is the first real front-end code in the project and should stay
small and dependency-free.

**Spec placement.** § 5.4 (preprocessor outputs) and § 13 M1.

**Prior art.** Learn Git Branching; GitHub's blame view; the "see the
diff" links Crafting Interpreters puts under every snippet.

### A4. Exercises: hand the reader the failing state

**What.** *Rust for Failers* has a move the book can ask the reader to
make: here is a test that fails, make it pass. An exercise is a step whose
tests are present and whose implementation is not.

**Mechanism.** Two keys on an existing step: `exercise = true` and
`solution = "<step-id>"`. An exercise step is verified as `expect =
"test_fail"` with the named tests failing (the reader's starting state is
a controlled failure); its solution step is an ordinary step, verified as
`pass`, that follows it in the plan. Plan-time errors: an exercise whose
solution does not exist, or whose solution precedes it, or whose tests are
not present in its own tree. Both states are tags in the generated repo,
so a reader can `git checkout step-031-exercise-from-char` and work, and
diff against `step-032-solution-from-char` when done. The book renders the
exercise with its tests shown and the solution collapsed under a toggle,
with a link to the solution tag.

`bower verify` checks both directions — the exercise fails for the stated
reason, the solution passes — so the book can never publish an exercise
whose tests are wrong or a solution that does not solve it.

**Thesis fit.** The reader's job in every exercise is to make a controlled
failure stop for the right reason. That is the book's thesis handed over
as a task.

**Cost.** M. Kernel: two keys and three errors. Replay: nothing new.
Preprocessor: the collapsed-solution rendering. The larger cost is
authorial — writing good failing tests is the work — and that cost is the
book's, not the tool's.

**Spec placement.** § 3.2 key table; § 6 verification table (`exercise`
row); § 9 worked example gets a fourth step.

**Prior art.** Rustlings, Exercism, and the Rust Book's "here's a test,
go" moments; the difference is that here the exercise and solution are
verified as adjacent states of one repo.

### A5. `bower follow` — the reader's CLI

**What.** The spec's `bower` has an author face. Readers who want to follow
along get "clone and checkout tags," which is fine for the reader who is
comfortable in git and a wall for the one who is not. A reader face on the
same binary makes the generated repo a guided path.

**Mechanism.** Reader-side subcommands, all driven by the generated repo's
own metadata (`STEPS.md`, tags, commit trailers — nothing the reader has
to download separately):

```
bower follow start [--from step-id]   # clone or reset to a step, open its book anchor
bower follow next                     # advance one step: show the diff, apply it, run verify
bower follow next --hands-on          # apply only tests + scaffolding, leave the impl to me
bower follow check                    # run this step's verification against my tree
bower follow diff                     # my tree vs the canonical tree at this step
bower follow where                    # step, chapter, and the book URL to reopen
```

`--hands-on` is A4 generalized: any step can be followed as an exercise by
applying only the files matching a per-repo pattern (tests, `Cargo.toml`,
region markers) and letting the reader write the rest; `check` then runs
the canonical verification against what they wrote. A reader's fork can
carry a GitHub Actions workflow that runs `follow check` on push — a
progress badge for the follow-along reader, and the seed of C3.

**Thesis fit.** The controllability of the reader's own environment: they
always know which state they are in, can diff against the known-good
state, and can run the book's own verifier on their work.

**Cost.** M. Mostly plumbing over `gix`; no kernel change. One open
choice (Part F, 6) is whether the reader face ships as a separate,
dependency-light binary so that installing it is not the first hurdle.

**Spec placement.** § 7 CLI surface; § 16.3 (the follow-along reader's
path alongside devenv).

**Prior art.** `git-tutor`-style guided histories; Rustlings' `watch`
loop.

### A6. The road not taken: verified alternate branches

**What.** Both books argue by counterfactual: had we done it the naive way,
here is what breaks. Today that is an aside in prose. It can be a real git
branch, verified to fail.

**Mechanism.** A step with `branch = "naive"` forks from its parent step
(the nearest preceding mainline step of its repo, or an explicit `after`)
onto a branch `alt/<parent>-naive` in the generated repo, with its own
`expect` — typically `compile_fail` or `test_fail`, and A1's captured
output showing exactly how. Branch steps may chain (`after` another branch
step) but the mainline can never depend on a branch step; the plan
becomes a tree with one trunk, and the cycle check becomes a tree check.
The book renders the branch as an aside with links to the branch tip and
its diff from the fork point; the scrubber (A3) shows it as a fork in the
slider.

**Thesis fit.** Controlled failing includes the failures the author chose
not to ship. A branch that is verified to fail is an argument that cannot
go stale.

**Cost.** M. Kernel: `branch` key, tree-shaped plan, three new errors
(branch off a nonexistent parent, mainline depending on a branch, branch
name collision). Replay: branches and their tags. `bower.lock` gains a
parent column.

**Spec placement.** § 4 ordering; § 5.3 tags; § 6.1 error list.

**Prior art.** None close; the nearest is Crafting Interpreters' habit of
showing the wrong version first, in prose.

### A7. Aquascope and quizzes: integrate, don't build

**What.** Brown's cognitive-engineering-lab built two mdBook preprocessors
for their experimental edition of the Rust Book: Aquascope, which renders
ownership and borrow-check state inline for a code block, and mdbook-quiz.
Both are directly usable on a Bower book and both get better when the
code they annotate is verified.

**Mechanism.** For a `compile_fail` step whose diagnostic is a borrow
error, the preprocessor can emit the block in Aquascope's form as well as
Bower's, so the reader sees the permissions diagram next to the captured
diagnostic (A1). Aquascope pins a rustc version, which devenv (§ 16) makes
a non-issue — the book already pins one. Quizzes attach to steps by id so
that a question about step 12 links to step 12's tree and diff.

**Thesis fit.** Moderate; these are instruments for teaching Rust rather
than for the books' theses, but a book that verifies its failures is
exactly where a borrow-check visualizer belongs.

**Cost.** S for quizzes, S–M for Aquascope (its toolchain requirements
are the cost).

**Spec placement.** § 5.4 as optional preprocessor companions; § 16.2
(Aquascope's pin joins the devenv).

**Prior art.** cognitive-engineering-lab/aquascope and the Brown edition
of the Rust Book.

### A8. Print and epub as first-class readers

**What.** § 3.4 already handles elision per target. Print and epub deserve
the rest: they cannot click, so the book has to carry the link in ink.

**Mechanism.** In the PDF target, every annotated block gets a margin tag
(`failers · step 12`) and a QR code resolving through `links.blob` to the
tag — so the print reader photographs the margin and lands on the tree.
Listing line numbers in print are the *repo's* line numbers, which the
line-anchored spans (§ 3.4) make free: a printed `L18–31` means `git show
step-012-rank-enum:src/rank.rs | sed -n 18,31p` reproduces the listing.
Epub gets the same margin tags as a footer line and, where the reading
system allows, the link. Both get A9's indices as back matter, which is
where the print reader's cross-references live.

**Thesis fit.** Stable links in both directions (§ 0) extended to the
medium with no links.

**Cost.** S–M. QR generation is a pure function of the link template;
margin typography is Typst work in M1.

**Spec placement.** § 13 M1 (per-target rules).

**Prior art.** Pragmatic Bookshelf's per-listing file names in print
(`ch03/rank.rs`), which is the same idea without the tag.

### A9. Generated back matter: the failure index

**What.** A book about failing should be indexed by its failures. Every
index below is a pure fold over the plan and A1's captured diagnostics, so
it costs nothing to keep correct.

**Mechanism.** `bower publish` emits, per book:

- **The failure index.** Every `compile_fail` and `test_fail` step,
  indexed by rustc error code (E0004, E0382, E0502 …) and by test name,
  with chapter, step, and a one-line summary from the captured
  diagnostic. A reader who has just hit E0502 in their own code opens the
  book at the index and finds the chapter that failed the same way on
  purpose.
- **The step index.** Every tag, in order, with its commit subject and
  chapter — `STEPS.md` rendered for readers.
- **The file index.** Every path in every repo, with the steps that
  created, changed, and (rarely) deleted it — the "biography of
  `src/rank.rs`" that A3's scrubber shows visually.
- **The play index** (if A2 or § 15 is in use). Every play cell by step.

**Thesis fit.** The failure index is the one nobody else can build: it
requires that failures be declared, verified, and captured, which is what
Bower makes routine.

**Cost.** S. Three folds and three render templates.

**Spec placement.** § 13 M1.

**Prior art.** The Rust compiler error index (as the link target); no
book indexes itself this way.

---

## Part B — Distribution and imprint

### B1. Step permalinks, citations, DOIs

**What.** § 13 M4 lets one book cite another by step id. The outside world
should be able to do the same, and be sure the citation still resolves in
ten years.

**Mechanism.** A resolver page per imprint, `https://…/s/<repo>/<step>`
(optionally `/s/<repo>/<step>@<edition>`), that redirects to the book
anchor, with the tree and diff links beside it — a pure function of the
link map. Each edition (M2) ships a `CITATION.cff` in the book source and
in every generated repo, and the edition tag on the book repo is turned
into a GitHub release, which Zenodo's integration mints a DOI for. The
generated repos' edition tags get the same treatment, so a step of code is
citable independently of the book. `bower publish --edition` writes the
CFF files and the release notes (from `bower diff --edition`); the DOI
mint is the one step that stays manual, because it is irreversible.

**Thesis fit.** Stable links in both directions, extended to links other
people make.

**Cost.** S. Two templates and a redirect table.

**Spec placement.** § 13 M2 (editions), M4 (imprint).

**Prior art.** Zenodo's GitHub release archiving; `CITATION.cff` as
GitHub renders it.

### B2. Verified translations

**What.** Translated technical books drift from the original because the
code and the prose are edited on different clocks. Under Bower, the code
is not in the translation at all: a translation is a second prose tree
that must produce the *identical* plan.

**Mechanism.** `src/` is the canonical language; `src/<lang>/` mirrors its
chapters. `bower plan --lang de` resolves the German tree and `bower
verify --lang de` asserts that its `bower.lock` and every `TreeState` are
byte-identical to the canonical ones — the translation cannot reorder,
drop, or alter a step, and if the canonical book changes, every
translation fails to build until it catches up, with `bower diff --lang
de` listing exactly which steps' surrounding prose moved. Commit subjects
(`msg`) translate through a per-language key (`msg.de`); code comments do
not translate by default (Part F, 4), which keeps the tree identical and
is, in practice, what translated Rust books do anyway. A translation is
published as an edition of its own (`edition-1.0-de`) pinned to the same
generated repos.

Compatibility note: mdBook's usual i18n route (`mdbook-i18n-helpers`)
extracts prose into gettext catalogs; Bower's directives are HTML comments
and would need to be excluded from extraction, and code blocks are already
excluded. Whether to go through gettext or through mirrored chapter files
is a Phase 5+ decision; the invariant — identical plan — holds either way.

**Thesis fit.** Determinism used as a contract between authors and
translators.

**Cost.** M. Mostly plumbing; the kernel already produces what needs
comparing.

**Spec placement.** § 13 M2; § 4 (a lock per language).

**Prior art.** The Rust Book's community translations, which drift for
exactly the reason this fixes; mdbook-i18n-helpers.

### B3. Errata and contributions as a product surface

**What.** § 13 M2 generates the errata list mechanically. The reader-facing
half is what makes it a product: a way to report, a way to subscribe, and
a way to contribute that does not route through the read-only repos.

**Mechanism.** Every annotated block's footer carries "report a problem at
this step," a prefilled issue link on the *book* repo carrying step id,
edition, rustc version from the devenv lock, and the tree link — so every
report arrives reproducible. Each edition publishes an Atom feed of errata
(from `bower diff --edition`), and the frozen epub's colophon carries a
QR/link to "errata for this edition." Contributions land as PRs to the
book source, which `bower verify` gates (C2 makes that visible); the
edition's release notes credit contributors from the git log. Readers'
alternate approaches can be submitted as A6 branches, which is the one
place community code can live without touching the mainline.

**Thesis fit.** The generated repos say "do not PR here"; this says where
to go instead, and makes every report carry its own reproduction.

**Cost.** S.

**Spec placement.** § 13 M2; § 5.4 footer.

**Prior art.** Pragmatic Bookshelf's errata pages; Manning's liveBook
comments, minus the hosting.

### B4. The living edition and the frozen edition

**What.** One source, two products: `main`, rebuilt on every push and
verified against current toolchains, and editions, pinned and immutable.
The interesting part is the space between them.

**Mechanism.** CI verifies `main` against a toolchain matrix — the
edition-pinned rustc, current stable, and beta — and publishes a per-step
board: this step passes on 1.94 (pinned), passes on stable, *fails on
beta with a changed diagnostic*. A beta failure is next month's erratum,
seen early. The board is the public answer to open question 4 (badges on
generated repos): not one badge per repo but one row per step, with the
captured diagnostic diff when something moves. The frozen edition's
readers see a one-line banner in the living HTML: "you are reading
edition 1.0; 3 errata since," linking to B3's feed.

**Thesis fit.** The book as an observable system. *Controllability*
argues that a system should report its own drift before its users do;
this is that argument applied to the book, in public.

**Cost.** M, all of it CI minutes — the matrix multiplies § 6's parallel
verification by the number of toolchains.

**Spec placement.** § 13 M2; retires open question 4.

**Prior art.** docs.rs build status; crater, in miniature.

### B5. Slides and workshops from the same plan

**What.** A talk series about these architectural choices is planned. A
talk about a book built this way should be built this way: every code
slide verified, every step linked.

**Mechanism.** `bower publish --target slides` renders a chapter (or a
`talk = "…"` selection of steps, a new chapter-level frontmatter key) as
a deck: one slide per step with the diff or the displayed spans, the
captured diagnostic (A1) on the failing steps, prose as speaker notes,
and the step's tree link as a footer. Typst's slide packages keep this
inside the M1 toolchain; a Marp/reveal.js output is a second template if
needed. A **workshop** is a chapter subset plus a start tag: `bower
publish --target workshop --from ch03-end --to ch05-end` emits the deck,
the exercises (A4) for that range, and a one-line setup (`bower follow
start --from ch03-end` inside the devenv). The half-day Rust workshop
becomes a build artifact of the book.

**Thesis fit.** Every rendition agrees about what is shown and linked
(§ 13 M1), now including the one the author says out loud.

**Cost.** M. A render plan over a step subset plus a deck template.

**Spec placement.** § 13 M1 (targets) and M3 (sample extraction
generalized to selection).

**Prior art.** Quarto's revealjs output from executable documents; Marp.

### B6. Fitting the storefronts

**What.** § 13 is explicit that Bower stops at artifacts and Leanpub,
Gumroad et al. do distribution. The artifacts should be shaped to drop
into those channels without a hand step.

**Mechanism.** Leanpub accepts author-uploaded PDF and EPUB, and also its
own Markua flavor of markdown; the PDF/EPUB route is the honest one
(Leanpub's generator will never run `bower`), so `bower publish --target
leanpub` is the existing PDF and EPUB targets plus a manifest and cover.
A `--target markua` is cheap — it is mostly markdown with different
admonition syntax — and would unlock Leanpub's in-progress publishing and
its per-version reader notifications for the *living* edition, at the
cost of Leanpub rendering the code rather than Typst. KDP and IngramSpark
take the PDF with print specs (trim, bleed, spine) as Typst parameters.
The rule stays § 13's: no DRM, no storefront in Bower, just artifacts
that fit.

**Thesis fit.** Neutral; this is logistics.

**Cost.** S.

**Spec placement.** § 13 M1.

**Prior art.** Leanpub's Markua manual and its upload-your-own-PDF flow.

### B7. The concept index and the rustdoc rendition

**What.** Two ways for the imprint's material to be found from outside
the books' tables of contents.

**Mechanism — concept index.** A `concept = "…"` key on headings or steps
(free text, many per step) folds into a per-concept page across all books:
"domain kernel" lists every step in *Failers* and *Controllability* that
touches it, with tree links. This is a folksonomy over steps — the
folkengine idea applied to the imprint's own catalog — and it stays a pure
fold: tags in, index out, no external service. Cross-book step links
(M4) get a human-readable layer.

**Mechanism — the rustdoc rendition.** The spec's § 1 problem was book
material living in doc comments by hand. Generated, it is a feature: at
each chapter-end tag, `bower publish --target rustdoc` writes the chapter's
prose into the generated repo's module docs (`//!`) as a *derived* layer
on a `docs` branch, so `cargo doc` on the generated repo — and docs.rs, if
the repo is published as a crate — renders the book alongside the API.
Nothing is hand-maintained; the branch is regenerated like everything
else.

**Thesis fit.** The concept index is the imprint's domain surface made
visible. The rustdoc rendition closes the loop on the problem that
started the project.

**Cost.** S for the index; M for rustdoc (doc-comment placement is a
per-file heuristic that needs textures).

**Spec placement.** § 13 M4 (catalog); § 13 M1 (targets).

**Prior art.** Rust's own `std` docs, which are a book in doc comments;
Obsidian's tag pages for the index's feel.

---

## Part C — Bower as a product

The honest framing first: Bower exists to ship two books, and Part C
should be sequenced after Phase 5 has produced one of them end to end.
But several product moves are cheap once the tool works for one author,
and one (C2) makes the author's own life better immediately.

### C1. Bower for other authors

**What.** Nothing in the kernel is Rust-specific: `verify` is a command,
`compile_fail` means "the check command fails," and the annotation format
is HTML comments in markdown. A `bower init` with per-language templates makes
that true in practice.

**Mechanism.** `bower init --lang rust|python|go|typescript` writes
`bower.toml`, a repo template (§ 3.6) with the language's scaffolding and
devenv, and a starter chapter with one create, one region, one
`compile_fail` (or the language's equivalent: a type-check or lint
failure) so the author sees the whole loop in five minutes. Per-language
verify commands are presets; the region-marker comment syntax comes from
the template. Distribution of the tool itself is the actual product
work: crates.io (name per open question 5), `cargo-binstall`, a Homebrew
tap, a Nix flake (which the devenv story wants anyway), and a GitHub
Action (`bower verify` on a book repo) in the Marketplace — the Action is
the discovery channel, because it is where an author looking for "verify
my book's code" will look.

**Thesis fit.** The domain-kernel claim that the core is delivery- and
language-agnostic, tested by handing it to someone else's language.

**Cost.** S–M. Templates are cheap; the packaging matrix is the tax.

**Spec placement.** § 7 (`init`), § 11 as a post-Phase 5 phase.

### C2. The Bower PR bot

**What.** § 13 M3's watch mode, socialized: every PR to a book repo gets
a comment explaining what it does to the plan and the repos, before
anyone reads the prose.

**Mechanism.** A GitHub App (or, more cheaply, an Action with
`pull_request` permissions) runs `bower plan` and `bower verify` on the
PR, diffs `bower.lock` against the base, and comments: steps added,
removed, reordered; which repos regenerate and how many commits change;
per-step verification results with A1 diagnostic diffs; and a link to a
preview deploy of the book. A status check per repo gates merge. For an
author working alone this is the second reader that catches "this
reorder silently rewrote forty commits."

**Thesis fit.** `bower.lock` was designed so reordering shows up in code
review (§ 4); this is the review.

**Cost.** M. The Action version is a day; the App is a service.

**Spec placement.** § 13 M3.

### C3. Classroom

**What.** A4's exercises plus A5's `follow check` are most of a course.
GitHub Classroom supplies the rest: per-student repos from a template and
autograding on push.

**Mechanism.** `bower classroom export --from ch03-end --to ch05-end`
emits an assignment template repo: the tree at the start tag, the
exercise steps' tests, a devenv, and an autograding workflow that runs
`bower follow check` at each exercise — so grading is the book's own
verifier. Instructors assign step ranges; students' repos are readers'
forks with a badge. The book itself remains the reading material, with
the workshop deck (B5) as the lecture.

**Thesis fit.** The reader's controlled failures, graded by the same
verifier that gated the author's.

**Cost.** M, nearly all of it in A4 and A5; the export is a template.

**Spec placement.** New § after § 16, or a C-series appendix; depends on
A4, A5, B5.

**Prior art.** GitHub Classroom autograding; Rustlings in the classroom.

### C4. "Bower-verified" as a claim, not a mark

**What.** Readers of technical books have learned that listings drift.
A book built by Bower can make a precise, checkable claim on its colophon:
every listing in this edition was generated from the source, every step
was verified on rustc X on date Y, and here is the log.

**Mechanism.** `bower publish --edition` writes a verification receipt
into the edition: a table of steps × expect × result × toolchain, the
devenv lock hash, and the book source commit — and links it from the
colophon and the imprint site. Anyone can rerun `bower verify` on the
tagged source in the pinned devenv and get the same table. That is the
whole "certification": reproducibility, not authority. A badge in the
README of each generated repo (open question 4) links to the same
receipt.

**Thesis fit.** Honest caveats about what is and is not tested, as a
published artifact rather than a sentence in a preface.

**Cost.** S.

**Spec placement.** § 13 M2.

### C5. Bundle and pricing shapes

**What.** Not a business plan — a note on what the artifacts make
possible, so the distribution choice is made deliberately.

**Mechanism.** The natural bundle is the book plus its generated repos,
notebooks, devenv, exercises, and (later) workshop deck, sold as an
edition and delivered as files; the Rust community norm — the living
HTML free, the frozen formats paid — fits the M2 split exactly and is
what B4's banner assumes. `draft = true` chapters (§ 13 M3) enable a
sponsorware or in-progress model without a second toolchain, and Leanpub
(B6) supplies pay-what-you-want and per-version notifications for free.
What Bower should not grow is any of the machinery those channels
already have.

**Thesis fit.** Neutral.

**Cost.** None in code; a decision.

**Spec placement.** § 13 (last paragraph).

### C6. Bower Press, hosted

**What.** The far rung: the M4 imprint as a service for other imprints —
hosted verification (C2), edition hosting with errata feeds (B3), the
per-step health board (B4), resolvers and DOIs (B1), reader progress (A5)
— for authors who want Bower's guarantees without running its CI.

**Mechanism.** Everything above, behind accounts. Named here so it is
not forgotten and so it is not mistaken for a near-term goal: it is worth
building only if C1 finds authors who want it, and the spec's "no
storefront" posture would need explicit revisiting.

**Cost.** L, and a different kind of project.

**Spec placement.** § 13, beyond M4, explicitly out of scope until C1
has evidence.

### C7. Reader signals, opt-in

**What.** A5's `bower follow check` knows when a reader is stuck at a
step. Aggregated and opt-in, that is the only telemetry an author of
*Controllability* could defend: which controlled failures readers
actually run, and where they stop.

**Mechanism.** `bower follow` asks once, defaults to off, and if enabled
sends (step id, result, toolchain) — no identity, no content — to a
static-friendly endpoint (a GitHub Discussions post per step, or a
one-line form POST) that B4's board can chart. The author sees "step 31
is where readers fall off"; nothing else.

**Thesis fit.** Contested, and that is the reason to write it down: the
book argues for observability of systems, not of readers. If it ships,
the design rule is that the signal is the same one the reader can see
locally, and nothing more.

**Cost.** S.

**Spec placement.** § 7 (`follow`), with the policy stated in the
generated README.

---

## Part D — Parked this round

Two directions were deliberately not weighted here but are recorded so
they are not lost.

**Verification and rigor.** Mutation-tested steps: each step's new tests
must kill a mutant introduced into that step's new code (`cargo-mutants`
over the diff), or the step's tests are not testing what the prose says
they test. The toolchain matrix is already B4; the verification receipt
is C4.

**LLM- and agent-native.** The book as an MCP server exposing steps,
trees, and diagnostics as resources; an *agent as reader* CI job — an
agent given chapter N's prose and the tree at `chN-1-end` must reach a
tree that passes `chN-end`'s verification, as a mechanical "does this
chapter actually teach it" check; steps as an agentic-coding benchmark,
each with a known-good next state; and steps exported as katas, which
the existing kata-generation skill could consume directly. The
agent-as-reader check is the one most likely to earn its way back into
scope, because it turns "is this chapter clear" into a build result.

---

## Part E — Ranked shortlist

Criteria, in order: leverage on the reader; thesis fit; cost; whether it
depends on a phase not yet built. Everything above the line is small
enough to be scoped into the existing build plan; everything below waits
for a real book.

| Rank | Idea | Why here | Phase |
|---|---|---|---|
| 1 | **A1** captured diagnostics | Smallest change with the largest effect on *Failers*; the compiler's words become verified content; retires open question 3 | Phase 3 |
| 2 | **A9** the failure index | Pure fold, unique to Bower-built books, and the best argument for A1 | M1 |
| 3 | **A4 + A5** exercises and `bower follow` | Turns reading into doing; A5 is also the seed of C3 | Phase 2–3 |
| 4 | **A3** the scrubber | The signature reader-facing surface; the first front-end code, so keep it small | Phase 4 |
| 5 | **A6** verified branches | Both books argue by counterfactual; a branch that fails on purpose cannot rot | Phase 2 |
| 6 | **B1 + B3 + C4** permalinks, errata, receipt | Three small M2 items that together make an edition a product | M2 |
| 7 | **B4** living vs frozen | The book observing its own drift; CI cost only | M2 |
| — | — | — | — |
| 8 | **A2** wasm in the browser | The thesis demo; large, and it wants the WIT world authored in the book first | Phase 4+ |
| 9 | **B5** slides and workshops | When the talk series has dates | M1+ |
| 10 | **B2** verified translations | When there is a translator | M2+ |
| 11 | **C1 + C2** other authors, PR bot | After Phase 5 proves the loop on one real book; C2 earlier if solo review pain shows up | post-5 |
| 12 | **C3** classroom | After A4/A5 and one workshop run | later |
| 13 | **A7, A8, B6, B7, C5, C7** | Cheap and worth doing when their rung arrives | various |
| 14 | **C6** Bower Press | Only with evidence from C1 | far |

The recommended immediate move is the top two: A1 changes what Phase 3
verifies, so it is cheaper to fold in now than to retrofit, and A9 is a
weekend once A1 exists.

---

## Part F — Open questions

1. **Play-cell language (A2).** A `lang` key defaulting to `python`, or a
   separate `web = true` flag? The kernel does not care; the question is
   what the author wants to type.
2. **Branch plans (A6).** One tree-shaped `Plan` with a trunk, or a
   `Plan` per branch keyed by its fork step? The tree keeps `bower.lock`
   in one file; per-branch plans keep the ordering code untouched.
3. **Diagnostic normalization (A1).** Which parts of rustc stderr are
   stable enough to snapshot — error code and primary message only, or
   the full rendering with spans? Suggest: full rendering by default,
   with a `match = "code"` escape hatch for steps that only care about
   the code.
4. **Translated code comments (B2).** Keep comments in the canonical
   language (identical trees, simpler contract) or allow a per-language
   comment overlay (better books, a second tree per language)? Suggest
   canonical-only for edition 1.
5. **Exercise representation (A4).** `exercise = true` plus `solution =`,
   or a distinct `op = "exercise"`? The former reuses the `test_fail`
   verification path unchanged.
6. **Reader binary (A5).** Same `bower` binary, or a dependency-light
   `bower-follow` so the reader's first command is not a Rust build?
   Affects nothing in the kernel; affects adoption a lot.
7. **Is Bower-for-others a goal (C1–C6)?** Or a side effect of doing the
   two books well? The answer decides whether Part C gets a phase or a
   footnote.
8. **Any reader signal at all (C7)?** Default no; write the policy down
   either way, because the *Controllability* reader will ask.

---

*Drafted against bower-spec.md draft 0.2 (Phase 1 built: `bower-core`,
`bower-testkit`). Admission rule: every idea is a pure function of the
existing `Plan`, `TreeState`s, link map, and verification results, or adds
at most one annotation key.*
