# EPIC-12: Generated back matter — the failure index (IDX)

## Context

A book about failing should be indexed by its failures. Idea A9
(`docs/bower-ideas.md:360`, ranked second in Part E at line 803) asks for
four indices as back matter — failures by rustc error code and by test name,
steps, files, play cells — each "a pure fold over the plan and A1's captured
diagnostics". A8 (`docs/bower-ideas.md:334`) wants the same indices in print,
where they carry the cross-references a reader cannot click.

Everything the first three folds need is already computed. `plan()`
(`bower-core/src/plan.rs:123`) returns one `RepoPlan` per repo in catalog
order (`plan.rs:136`). Each `PlannedStep` (`plan.rs:40`) carries its `seq`,
`id`, `msg`, `expect`, book `anchor`, the `files` it touches (`plan.rs:47`),
the complete materialized `tree` after it (`plan.rs:50`, built at
`plan.rs:149`), and its `exercise` (`plan.rs:55`). `PlannedStep::tag`
(`plan.rs:108`) is the only tag format. `Expect` (`directive.rs:92`) already
separates `CompileFail` and `TestFail` from `Pass` and `Skip`.

One reader-facing index ships today, in the wrong place for a reader.
`trailers::steps_md` (`bower/src/trailers.rs:73`) writes `STEPS.md` — tag,
expect, subject, source link — into the **final tree of each generated
repo** (`bower/src/replay.rs:330`, inserted at `replay.rs:342`). It lives in
the code, not the book; it links the site by URL (`trailers.rs:110`), not the
page by anchor; and a book that feeds two repos has two of them.

What the idea doc did not know, because it was written as if only Phase 1 had
shipped:

- **There is no single place `bower publish` emits a book.** Epub and PDF are
  fed `RenderPlan::chapters` (`bower/src/publish.rs:485`, folded at
  `publish.rs:500`), one per `SUMMARY.md` entry the loader read
  (`bower/src/loader.rs:68`). HTML is not: `MdBookRenderer::render` ignores
  the plan (`publish.rs:935`, `_plan`) and runs `mdbook`, which re-runs
  `mdbook-bower` (`bower/src/mdbook_bower.rs:88`). The one function all three
  targets pass through is `render::chapter` (`bower/src/render.rs:23`), called
  at `publish.rs:519` and `mdbook_bower.rs:98`.
- **The PDF has no step anchors at all.** `render::chapter` injects a raw
  `<a id="step-…">` for every target (`render.rs:72`). pandoc 3.11's Typst
  writer drops raw HTML, and Typst refuses a link to a missing label —
  `label <step-x> does not exist in the document`, exit 1. Checked on this
  machine (pandoc 3.11, typst 0.15.1) while drafting. The epub keeps the raw
  anchor and pandoc rewrites `#step-x` to `ch001.xhtml#step-x`, so epub
  links work. Today nothing in any chapter links a step anchor, which is the
  only reason the PDF has not broken yet. An index is the first thing that
  would.
- **Test names are not in the plan, and verification forgets them.**
  `verify::Outcome` (`bower/src/verify.rs:43`) keeps stderr for the failure
  report only; an upheld `test_fail` stores nothing about *which* test
  failed. The tree knows which `#[test]` functions exist, not which one fails.
  An index "by test name" needs captured output, and that is EPIC-11.
- **Play cells half exist.** The kernel parses `notebook="play"`
  (`directive.rs:229`), binds cells to steps (`plan.rs:262`) and counts them
  in the lock (`plan.rs:384`). No book uses one, no target renders one
  (`BACKLOG.md:87`), and `--target ipynb` is refused by name
  (`publish.rs:70`). The play-index fold would be ten lines, but it would
  link to a notebook nobody can open yet.

**This EPIC does not** capture diagnostics (EPIC-11 does), render play cells
or build `--target ipynb`, add margin tags or QR codes (the rest of A8),
index prose concepts (B7), or change `STEPS.md` by a byte. It adds no config
key to `bower.toml` and nothing to `bower.lock`. It does not touch `verify`,
`build`, `status`, or `push`. `bower-core` gains a module and stays
dependency-free.

---

## Status

| Component | Status |
|---|---|
| Kernel: `index=` key, `IndexPlacement` on `BookPlan`, three errors | **Planned** |
| Kernel: `back_matter` fold — step spine, failure rows, file biographies | **Planned** |
| Kernel: `Chapter::title` (moved from `publish::heading_of`) | **Planned** |
| Testkit: back-matter textures and two properties | **Planned** |
| Render: step index and failure index (by chapter and step) | **Planned** |
| Render: pandoc-native step anchors outside HTML | **Planned** |
| Render: file index — the biography of a file | **Planned** |
| Print: page numbers in the PDF index | **Planned** |
| Failure index by rustc error code and failing test (needs EPIC-11) | **Planned** |
| Sample book back matter and goldens | **Planned** |
| Play index (gated on `--target ipynb`) | **Planned** |

---

## Goals

- Give every Bower book a **failure index**: every declared failure, where it
  sits in the book, and — once EPIC-11 lands — the **rustc error code** and
  **failing test** it shows, so a reader who just hit E0502 in their own code
  can find the chapter that failed the same way on purpose.
- Give it a **step index** (every tag, in order, linked into the book) and a
  **file index** (every path, and the steps that created, changed, and
  deleted it).
- Keep every index a **pure fold** in the kernel: same book, same index, byte
  for byte. Rendering is the edge's job.
- One **substitution point** for every target, so html, epub, and pdf agree
  about what the index says and differ only in how a link is written.
- Ship in two halves: everything the plan already knows now; the error-code
  and test-name columns when EPIC-11 does.

## Scope

### Things

| Thing | What it is |
|---|---|
| **Placement** | Where the author put an index: an `index=` directive in a chapter `SUMMARY.md` lists. |
| **Step reference** | One planned step, as an index row names it: repo, seq, id, tag, subject, expect, chapter, anchor. |
| **The spine** | Every step reference, in book order. The other indices point into it. |
| **Failure row** | A spine entry whose `expect` is `compile_fail` or `test_fail`, plus the evidence EPIC-11 captures for it. |
| **Error code** | `E0308` — a rustc diagnostic's bracketed code, read out of captured stderr. |
| **File biography** | One path in one repo, and the events in its life: created, changed, deleted. |

### Business requirements

1. An index appears where the author places it, under the heading the author
   writes above it, in a chapter `SUMMARY.md` lists — and nowhere else.
2. The step index lists every step exactly once, per repo, in `seq` order.
3. The failure index lists exactly the `compile_fail` and `test_fail` steps.
   `expect="none"` is not a failure; it was never checked.
4. A file biography records a change only when the committed bytes changed:
   what `git log -- <path>` would show, not which directives mentioned the
   path.
5. Every row links to the book anchor of its step, and — when the repo
   declares a `tree` template — to the tag.
6. Every link resolves in every target. A PDF that fails `typst compile`
   because of an index is a defect, not a rendering choice.
7. A failure with no error code — a cargo manifest error, a test panic — is
   listed, never dropped.
8. The indices are derived: no author ever edits one, and no drift check is
   needed because nothing can drift.

### What the reader gets

```markdown
# Failure index

| Step | Expect | Chapter | What fails |
|---|---|---|---|
| [`step-011-test-that-fails`](ch04-tests-and-failing-on-purpose.md#step-test-that-fails) | test_fail | Tests, and failing on purpose | test: greet should ignore stray whitespace (failing) · exercise |
| [`step-013-wont-compile`](ch04-tests-and-failing-on-purpose.md#step-wont-compile) | compile_fail | Tests, and failing on purpose | feat: scratch module (does not compile) |
```

Those are the sample book's two failures (`books/hello-playbook/bower.lock`,
steps 011 and 013). With EPIC-11 the same page grows a section keyed by code;
step 013's `let n: u32 = "42";` would be listed under E0308 once its stderr
is captured — no index claims a code before then.

```markdown
### `src/scratch.rs` — hello-playbook

| | Step | Chapter | Subject |
|---|---|---|---|
| created | `step-013-wont-compile` | Tests, and failing on purpose | feat: scratch module (does not compile) |
| changed | `step-014-scratch-fixed` | Tests, and failing on purpose | fix: parse the text instead of pretending it is a number |
| deleted | `step-020-drop-scratch` | CI | chore: remove the scratch module |
```

### Not in scope

Concept and glossary indices (B7), cross-book indices (M4), a play index
before `--target ipynb` exists, lint names as index keys (open question 5),
and editing `STEPS.md`.

---

## Decisions

1. **Placement is authored, content is derived.** An author writes
   `<!-- bower index="failures" -->` in a chapter that `SUMMARY.md` lists.
   The loader already reads every listed chapter (`loader.rs:85`), mdBook
   already puts it in the sidebar and search, and `render::chapter` is the one
   function every target runs, so substituting there puts the index in all
   three targets with one code path. The alternative — the preprocessor
   injecting a chapter into mdBook's JSON and `render_plan` appending one — is
   two injection mechanisms that could disagree, and the author would lose
   control of where the index goes and what it is called.
2. **One key, per the admission rule.** The idea doc admits an enhancement
   that "extend[s] the annotation vocabulary by at most one key"
   (`docs/bower-ideas.md:21`). `index=` is that key. Values: `failures`,
   `steps`, `files`. `repo=` may narrow it; nothing else may ride along.
3. **Placements live in the plan; index content does not.** `BookPlan`
   gains `indices: Vec<IndexPlacement>`, validated at plan time and read by
   the renderer by line, the same way exercises are (`render.rs:596`). The
   content is `back_matter(book, plan)`, a function of a plan, sitting beside
   `lock_text` (`plan.rs:369`) rather than inside `plan()`. Nothing in any
   repo depends on either, so `bower.lock` does not change and `make plan`
   produces no churn.
4. **The spine is the step index; everything else points into it.** A failure
   row and a file event hold an index into `BackMatter::steps`, not a copy of
   the step. One definition of how a step is named, linked, and titled.
5. **A biography is a tree diff, not an op log.** `PlannedStep::files`
   (`step.rs:45`) lists paths a step *mentioned*: a region edit that leaves
   the bytes as they were is in it, and a prose step lists nothing
   (`002 hello-runs … files=` in the lock). Each `tree` is the materialized
   tree git commits (`plan.rs:149`), so comparing a step's tree with its
   parent's is exactly what `git log -- <path>` shows. Bower has no rename op,
   so there is no rename to follow. A path deleted and later re-created gets a
   second `Created`. Scaffolding (`replay.rs:106`) and `STEPS.md`
   (`replay.rs:342`) are not in any planned tree and are not in the index: the
   file index is what the book wrote.
6. **"Parent" is a function, not `seq - 1`.** Today a step's parent is the
   previous step of its repo. EPIC-09 gives steps `parents` and lines; the fold
   takes its parent from one helper, so a branch step diffs against its own
   line, a merge against main, and the change is one function.
7. **Test names and error codes come only from captured output.** The
   tempting shortcut — scan the step's tree for `#[test] fn` names it
   introduced — reports tests that exist, not tests that fail. Step 002 of
   `rust4failures` is a `test_fail` whose `op="replace"` rewrites the whole of
   `src/lib.rs` (`books/rust4failures/src/ch01-local_development.md:216`); it
   fails because the implementation changed under a test, and a diff of
   `#[test]` names cannot see that. An index that guesses is an index that lies, in the one book
   whose thesis is that the compiler, not the author, says what failed. Until
   EPIC-11, the failure index is keyed by chapter and step, and says so.
8. **Links are written per target; anchors too.** HTML links a relative
   `chapter.md#step-<id>`, which mdBook rewrites to `.html`. Epub and PDF
   are one pandoc document, so they link `#step-<id>`. Outside HTML,
   `render::chapter` writes the anchor as a pandoc span, `[]{#step-<id>}`,
   instead of raw HTML: pandoc turns it into an epub `id` and a Typst label
   `<step-<id>>`, both checked on pandoc 3.11. HTML keeps `<a id>` byte for
   byte, so the sidebar, the search index, and every `Book-Url` trailer
   (`trailers.rs:30`) are untouched.
9. **Print gets page numbers.** In the PDF each row adds
   `` `#context counter(page).at(<step-<id>>).first()`{=typst} `` — raw Typst
   inline, which pandoc passes through. A sketch compiled and printed the right
   page (`pdftotext`: "on page 1" for a label one page back). The printed
   index is the one place a print reader follows a reference, so it is the
   one place page numbers are worth their line of Typst.
10. **An empty index says so.** A book with no declared failures renders "This
    book declares no failures." under the placement, not an empty table, and
    not an error — a book is allowed to be about something else.
11. **Error codes link the rustc error index.** `E0308` links
    `https://doc.rust-lang.org/error_codes/E0308.html`, a constant in the render
    layer (open question 1). The idea doc names it as the link target
    (`docs/bower-ideas.md:389`).
12. **`STEPS.md` stays the repo's; the step index is the book's.** They list
    the same rows for different readers. `steps_md` moves onto the spine only
    if `hello_playbook_is_byte_identical_across_runs`
    (`bower/tests/determinism.rs:94`) and every tag SHA stay identical; a
    changed `STEPS.md` changes the last commit of every generated repo.

---

## Domain map

| Domain concept | Code construct | Status |
|---|---|---|
| Where an index goes | `IndexPlacement` on `BookPlan` `plan.rs:18` | 🔴 new |
| The `index=` key | `Directive::index` `directive.rs:138` | 🔴 new |
| A chapter's title | `publish::heading_of` `publish.rs:531` → `Chapter::title` | 🟡 moves into the kernel |
| A step, as an index names it | `StepRef` over `PlannedStep` `plan.rs:40` | 🟡 fields exist |
| The tag | `PlannedStep::tag` `plan.rs:108` | ✅ reuse |
| Which steps fail | `Expect::{CompileFail, TestFail}` `directive.rs:92` | ✅ reuse |
| A file's state at a step | `TreeState` `tree.rs:24`, `PlannedStep::tree` | ✅ reuse |
| A file's biography | `FileBiography` — diff of consecutive trees | 🔴 new |
| An error code / failing test | `ErrorCode`, `failing_tests` over EPIC-11 output | 🔴 new, gated |
| The substitution point | `render::chapter` `render.rs:23` | 🟡 grows a placeholder arm |
| A link to a tag | `subst` `render.rs:548`, `LinkTemplates::tree` `config.rs:85` | ✅ reuse |
| A step anchor | `<a id>` `render.rs:72` | 🟡 pandoc span outside HTML |
| The repo's own step list | `trailers::steps_md` `trailers.rs:73` | ✅ unchanged |
| Play cells by step | `PlayCell` `plan.rs:61` | 🟡 fold trivial; no target |

---

## Design

### The key — `bower-core/src/directive.rs`, `block.rs`, `plan.rs`

`Directive` gains `index: Option<IndexKind>`, parsed beside `notebook`
(`directive.rs:229`); an unknown value is the existing `BadValue`. In
`block::resolve` (`block.rs:206`) an index directive is its own branch, like a
play cell (`block.rs:249`): `repo` becomes optional (it is required at
`block.rs:243` for everything else), no fence is expected, and any tree key,
`step`, `expect`, `exercise`, or `notebook` is refused. `plan()` partitions
index blocks out beside play and exercise blocks (`plan.rs:129`) and records
them; they never join a step.

```rust
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum IndexKind { Failures, Steps, Files }

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexPlacement {
    pub loc: Location,
    pub kind: IndexKind,
    /// `None`: every repo the book feeds, in catalog order.
    pub repo: Option<RepoName>,
}

pub struct BookPlan {
    pub repos: Vec<RepoPlan>,
    pub indices: Vec<IndexPlacement>,   // new; document order
}
```

New `BowerError` variants (`lib.rs:63`, `#[non_exhaustive]` at `lib.rs:62`),
each with a `Location`: `IndexConflictingKeys` (mirrors
`PlayCellConflictingKeys`, `lib.rs:162`), `IndexDuplicate { kind }` — the same
kind and repo placed twice, reported at the second — and `IndexOnEmptyRepo` —
`repo=` names a catalog repo the book never feeds, so the index would be
empty by construction. An unknown `repo=` is the existing `UnknownRepo`.

### The fold — `bower-core/src/backmatter.rs` (new)

```rust
/// One step, as every index names it. Built once; pointed at, never copied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepRef {
    pub repo: RepoName,
    pub seq: usize,
    pub id: StepId,
    pub tag: String,            // PlannedStep::tag()
    pub msg: String,
    pub expect: Expect,
    pub anchor: Location,
    pub chapter_title: String,  // Chapter::title of anchor.chapter
    pub exercise: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileEvent { Created, Changed, Deleted }

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileBiography {
    pub repo: RepoName,
    pub path: String,
    /// (event, index into `BackMatter::steps`), in seq order.
    pub events: Vec<(FileEvent, usize)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Failure {
    pub step: usize,                 // into `steps`
    pub evidence: Vec<Evidence>,     // empty until EPIC-11 (Phase 4)
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BackMatter {
    pub steps: Vec<StepRef>,         // repos in catalog order, then seq
    pub failures: Vec<Failure>,      // spine order
    pub files: Vec<FileBiography>,   // repo, then path (BTreeMap order)
}

#[must_use]
pub fn back_matter(book: &BookSource, plan: &BookPlan) -> BackMatter;
```

`files` walks each repo's steps with the previous tree in hand — the empty
`TreeState::default()` before step 1, exactly where `plan()` starts
(`plan.rs:143`) — and compares keys and bodies. Binary bodies from
`op="copy"` compare as bytes. The cost is one pass over trees the plan
already holds.

`Chapter::title` (`source.rs:32`) is the first `# ` heading, else the path:
the rule `publish::heading_of` applies at `publish.rs:531`, moved so that the
kernel and the render plan cannot title a chapter two ways. `heading_of`'s
test (`publish.rs:1475`) moves with it.

### Render — `bower/src/render.rs`

`render::chapter` gains a `&BackMatter` argument, built once per render at
its two call sites (`publish.rs:519`, `mdbook_bower.rs:98`). An
`indices_by_line` map, shaped like `exercises_by_line` (`render.rs:596`),
tells the directive arm to emit the index where the directive was, instead
of dropping it. The three renderers are pure functions returning markdown
lines:

```rust
pub fn step_index(bm: &BackMatter, repo: Option<&RepoName>, at: &str,
                  links: &BTreeMap<String, LinkTemplates>, target: Target) -> Vec<String>;
pub fn failure_index(/* same */) -> Vec<String>;
pub fn file_index(/* same */) -> Vec<String>;
```

`at` is the placement's chapter, from which an HTML link to another chapter is
made relative. Tag links go through `subst` (`render.rs:548`) with the repo's
`tree` template; a repo without one gets the tag as code, never a guessed URL
— the rule `footer` already follows (`render.rs:286`). Multi-repo books get a
`###` section per repo. The anchor arm at `render.rs:72` becomes per target:
`<a id>` for `Target::Html`, `[]{#step-<id>}` otherwise — a predicate on
`Target` beside `has_hidden_lines` (`publish.rs:45`).

### Captured evidence — after EPIC-11

EPIC-11 puts normalised compiler and test output into the book source. This
EPIC assumes one thing of it: per step, zero or more `(kind, text)` bodies the
kernel can read off the plan. From those, two pure parsers:

```rust
pub struct ErrorCode(pub String);           // "E0308"
pub struct Evidence { pub codes: Vec<ErrorCode>, pub tests: Vec<String>, pub headline: String }

pub fn error_codes(stderr: &str) -> Vec<ErrorCode>;   // `error[E....]`, deduplicated, in order
pub fn failing_tests(output: &str) -> Vec<String>;    // libtest's `test path::name ... FAILED`
```

`headline` is the first `error…:` line, trimmed — the idea's "one-line
summary from the captured diagnostic". `BackMatter` gains `by_code()` and
`by_test()` views; a failure with evidence but no code lands under "No error
code" (requirement 7). If EPIC-11's shape differs, the parsers do not; only
the adapter that feeds them moves.

### Testkit — `bower-testkit`

A `back_matter` fixture: one file created, changed, deleted, and re-created;
a region edit that writes the bytes already there; a prose step; a
`compile_fail`, a `test_fail`, and an `expect="none"`; two repos; each
`index=` form, and one of each new error in `broken()`. Coverage gains the
`IndexPlacement` mechanism (`coverage.rs:16`). Properties over `arb_book`
(`generators.rs:52`): a biography, replayed, says a path is alive at step
*n* exactly when `tree.get(path)` is `Some` there; every `step` index a
failure or event holds is in bounds.

---

## Work Items

### Phase 0 — Kernel

- [ ] **0a.** `IndexKind`, `Directive::index`, the `resolve` branch, the
  partition in `plan()`, `BookPlan::indices`, three error variants with
  `Display` arms and `location()`.
- [ ] **0b.** `Chapter::title`; `publish::heading_of` deleted in favour of it.
- [ ] **0c.** `backmatter.rs`: `StepRef`, the spine, `Failure` rows,
  `FileBiography` by tree diff, parent from one helper (Decision 6). Prelude
  exports.
- [ ] **0d.** Testkit fixture, broken fixtures, coverage mechanism, the two
  properties. `make purity` still reports no dependencies.

### Phase 1 — Step and failure indices

- [ ] **1a.** `indices_by_line`; the directive arm emits instead of dropping;
  `render::chapter` takes `&BackMatter`; both call sites build it once.
- [ ] **1b.** `step_index` and `failure_index`, with relative HTML links and
  fragment links elsewhere; the empty sentence (Decision 10).
- [ ] **1c.** Anchors as pandoc spans outside HTML; `sample_book_anchors_every_step`
  (`bower/tests/preprocessor.rs:227`) unchanged and green.
- [ ] **1d.** Goldens against `FakeRenderer` for html and epub render plans.

### Phase 2 — The file index

- [ ] **2a.** `file_index`: one `###` per path, grouped by repo, rows linking
  each event's step.
- [ ] **2b.** Golden: `src/scratch.rs` reads created 013, changed 014,
  deleted 020 in the rendered sample book.

### Phase 3 — Print

- [ ] **3a.** Page numbers in `Target::Pdf` rows (Decision 9).
- [ ] **3b.** Slow lane: render the sample book's PDF, `typst compile` exits 0,
  and `pdf_is_byte_identical_across_runs` (`bower/tests/publish.rs:235`) still
  holds with the index in.
- [ ] **3c.** Epub: the index chapter is in pandoc's `--toc`
  (`publish.rs:785`) and every `#step-` link resolves to an `id` in the
  package.

### Phase 4 — Error codes and failing tests (after EPIC-11)

- [ ] **4a.** `error_codes` and `failing_tests`, with textures: a multi-code
  diagnostic, a `warning[…]` beside an `error[…]`, a cargo manifest error with
  no code, a panicking test with a backtrace.
- [ ] **4b.** `Evidence` on `Failure`; `by_code`, `by_test`, the "No error
  code" bucket.
- [ ] **4c.** `failure_index` grows the by-code section, each code linking the
  rustc error index, and a failing-test column.

### Phase 5 — Book and docs

- [ ] **5a.** `books/hello-playbook/src/back-matter.md` with all three
  placements, listed in `SUMMARY.md`; the testkit's `include_str!` fixture and
  `loader__reads_hello_playbook_in_summary_order` (`loader.rs:172`) updated.
- [ ] **5b.** A failure index in `books/rust4failures` — the book the index
  exists for.
- [ ] **5c.** `.okf/model/` pages for placement and back matter; the key in
  the directive table; `BACKLOG.md` ideas row for A9 points here.
- [ ] **5d.** Flip Status rows; append the corrigendum.

### Phase 6 — Play index (gated on `--target ipynb`)

- [ ] **6a.** `IndexKind::Play` and a `plays` view over `PlannedStep::play_cells`,
  once a target renders a play cell for the index to link. Until then
  `index="play"` is a `BadValue`.

---

## Test Plan

Kernel: `directive__index_key_parses_three_kinds`,
`directive__index_play_is_a_bad_value`,
`plan__index_with_tree_keys_is_refused`,
`plan__index_placed_twice_is_refused_at_the_second`,
`plan__index_on_a_repo_the_book_never_feeds_is_refused`,
`plan__index_never_joins_a_step`, `lock_text__is_unchanged_by_a_placement`,
`chapter__title_is_the_first_h1_else_the_path`,
`backmatter__spine_lists_every_step_once_in_seq_order`,
`backmatter__failures_are_exactly_compile_fail_and_test_fail`,
`backmatter__expect_none_is_not_a_failure`,
`backmatter__scratch_rs_is_created_changed_deleted`,
`backmatter__a_touch_that_changes_no_bytes_is_not_a_change`,
`backmatter__a_prose_step_has_no_file_events`,
`backmatter__a_recreated_file_is_created_twice`,
`backmatter__repos_are_in_catalog_order`, `backmatter__is_deterministic`.
Properties: `prop__biography_replays_to_tree_presence`,
`prop__every_step_pointer_is_in_bounds`.

Render: `render__index_placement_becomes_the_index`,
`render__html_links_are_relative_md_with_the_step_anchor`,
`render__epub_and_pdf_link_by_fragment`,
`render__anchors_are_pandoc_spans_outside_html`,
`render__html_anchor_is_unchanged`,
`render__empty_failure_index_says_so`,
`render__a_repo_without_a_tree_template_gets_no_tag_link`.

Integration: `publish__back_matter_reaches_every_target_plan`,
`publish__pdf_with_an_index_compiles` (`#[ignore]`, needs typst),
`publish__every_epub_step_link_resolves` (`#[ignore]`, needs pandoc),
`hello_playbook_is_byte_identical_across_runs` unchanged.

Phase 4: `error_codes__reads_every_bracketed_code_once`,
`error_codes__none_in_a_manifest_error`,
`failing_tests__reads_libtest_failed_lines`,
`backmatter__uncoded_failures_are_listed_not_dropped`.

## Key Files

| File | Role |
|---|---|
| `bower-core/src/backmatter.rs` | new; the fold, `StepRef`, `FileBiography`, `Failure`, parsers |
| `bower-core/src/directive.rs`, `block.rs` | `index=`; the index branch in `resolve` |
| `bower-core/src/plan.rs` | partition; `BookPlan::indices` |
| `bower-core/src/source.rs` | `Chapter::title` |
| `bower-core/src/lib.rs` | three error variants; prelude |
| `bower-testkit/src/{fixtures,coverage}.rs` | textures, mechanism, properties |
| `bower/src/render.rs` | placement arm, three index renderers, per-target anchors |
| `bower/src/publish.rs` | builds `BackMatter`; loses `heading_of` |
| `bower/src/mdbook_bower.rs` | builds `BackMatter` for HTML |
| `books/hello-playbook/src/back-matter.md` | the sample placements |

## Reuse (do NOT recreate)

- `plan.rs:108` `PlannedStep::tag` — the only tag format; `StepRef::tag` calls it.
- `plan.rs:149` materialized trees — the biography diffs these, never
  re-folds blocks.
- `render.rs:548` `subst` and `config.rs:85` `LinkTemplates` — tag links;
  no new template.
- `render.rs:596` `exercises_by_line` — the shape of `indices_by_line`.
- `block.rs:249` the play-cell branch — the shape of the index branch.
- `publish.rs:531` `heading_of` — becomes `Chapter::title`; do not write a
  second title rule.
- `trailers.rs:73` `steps_md` — its columns are the step index's columns;
  leave its bytes alone (Decision 12).

## Compatibility

- **Preserves** every existing book: no placement, no index, and HTML bytes
  identical. `bower.lock`, `STEPS.md`, every repo SHA, and `bower.toml` are
  unchanged.
- **Adds** one directive key, three errors, one kernel module, a field on
  `BookPlan`, a parameter on `render::chapter`.
- **Changes** epub and PDF bytes once, for every book: step anchors become
  pandoc spans. The PDF stays byte-identical across runs; the site
  fingerprint (`publish.rs:854`) is unaffected because HTML does not change.
- **Breaks** nothing in `bower-core`'s purity: `cargo tree -p bower-core -e
  normal` still prints one line.

## Dependencies

- **Built on:** EPIC-01 (tags, `STEPS.md`), EPIC-03 (`render::chapter`,
  anchors), EPIC-06 and EPIC-07 (`RenderPlan`, `Target`, the Typst path).
- **Built on (Phase 4):** EPIC-11 captured diagnostics. Phases 0–3 ship
  without it; the error-code and failing-test columns wait for it.
- **Coordinates with:** EPIC-09 — the spine gains a line column and
  biographies take parents from the plan (Decision 6); an unmerged branch's
  `test_fail` is exactly what a *Failures* index should list. EPIC-15 — the
  scrubber's per-file view draws the same `FileBiography`; whichever lands
  second reuses the fold.
- **Related:** EPIC-13 (exercise steps are flagged in both indices), EPIC-14
  (`bower follow` can jump by error code once Phase 4 lands), EPIC-16 (an
  edition's back matter is frozen with its edition), `--target ipynb`
  (Phase 6).

## Verification

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
make purity
make plan && git diff --exit-code books/*/bower.lock
make book && grep -c 'step-013-wont-compile' books/hello-playbook/book/back-matter.html
make epub
make pdf
make slow
```

Exit criteria:

1. The sample book's HTML, epub, and PDF each carry the three indices where
   `back-matter.md` places them, with the same rows.
2. Every step link in the index resolves: in HTML to the anchor mdBook serves,
   in the epub to an `id` in the package, in the PDF to a Typst label —
   `typst compile` exits 0.
3. The failure index lists steps 011 and 013 and nothing else; the biography
   of `src/scratch.rs` reads created 013, changed 014, deleted 020.
4. `bower.lock`, `STEPS.md`, and every generated SHA are byte-identical to
   before; `make plan` leaves the locks unchanged.
5. Two renders of the PDF are byte-identical.
6. `cargo tree -p bower-core -e normal` prints one line.
7. After EPIC-11 and Phase 4: step 013 appears under its captured error code,
   linked to the rustc error index, and step 011 under its failing test.

---

## Open questions — decisions, not code

| # | Question |
|---|---|
| 1 | **Which error index.** `doc.rust-lang.org/error_codes/` tracks stable; a book pinned to an older toolchain by `rust-toolchain.toml` may cite a code whose page has since been reworded. Link stable, link the pinned version, or make it a template on the book? |
| 2 | **Duplicate step ids across repos.** Anchors are `step-<id>` with no repo (`render.rs:72`); two repos sharing an id share an anchor, and Typst refuses a reference to a label that occurs twice. Namespacing anchors changes every `Book-Source` trailer (`trailers.rs:25`) and so every SHA. Refuse a placement over an ambiguous book at plan time, or namespace once and take the SHA churn? |
| 3 | **A test-name index before EPIC-11.** Decision 7 rejects inferring tests from the tree. If EPIC-11 slips, is "tests this step adds", labelled as that, worth shipping — or is a half-true column worse than none? |
| 4 | **Default placement.** Placement is opt-in per book. Should a future `bower new` (spec § 13 M3) scaffold `back-matter.md`, so a new book gets its failure index without asking? |
| 5 | **Lints as keys.** The sample book's gate turns warnings into failures; a clippy lint has no `E` code. Index `clippy::unwrap_used` beside `E0308` once EPIC-11 captures clippy output, or keep the index to rustc codes? |
| 6 | **Scaffolding in the file index.** Decision 5 leaves template files out because the book never wrote them. A reader asking "where did `LICENSE` come from" gets no answer. A "from the template" line per repo, or silence? |
| 7 | **The play index's first home.** Phase 6 waits for `--target ipynb`. If a book uses play cells before then, is an HTML "try it" list enough of a destination to ship it early? |

---

*Checked while drafting: pandoc 3.11 drops raw `<a id>` in Typst output and
Typst 0.15.1 refuses a link to the missing label; a pandoc span survives as a
label in both epub and Typst, and a raw-Typst page reference compiled and
printed the right page. Drafted 10 September 2026 against
`ImperialBower/bower` @ HEAD ("docs: file the forge design and add it to the
backlog").*
