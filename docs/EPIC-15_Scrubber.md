# EPIC-15: The scrubber — time-travel over the tree (SCRB)

## Context

Eight EPICs shipped, and the book → repo direction is link-shaped: every
annotated block gets a footer naming its file, its line range, its step, and —
when the repo has a remote — a `diff` and a `browse` link (`render::footer`,
`bower/src/render.rs:286`). Each link lands on **one** state. A reader following
the *shape* of the code — `src/lib.rs` growing through chapter 4, the test that
fails at step 011 and passes at step 012 — opens twenty forge tabs and diffs in
their head. A reader looking at a line cannot ask the question the book exists
to answer: **which paragraph wrote this?**

Idea **A3** (`docs/bower-ideas.md:449`, Part E rank 4) answers it: a static
per-repo payload emitted by `bower publish --target html`, and a small widget
that slides through every step. The kernel already computes most of the inputs:

- Every `PlannedStep` carries its **complete materialized tree**
  (`bower-core/src/plan.rs:50`), its tag (`plan.rs:108`), its expectation, and
  its anchor (`plan.rs:46`). A tree is a `BTreeMap<String, FileBody>`
  (`bower-core/src/tree.rs:24`), so iteration is deterministic for free.
- `BlockDisplay::ranges` (`bower-core/src/display.rs:58`) say where each shown
  span sits in its step's tree.
- The page carries a `#step-<id>` anchor per step (`render.rs:72`), and
  `html_name` (`bower/src/trailers.rs:117`) maps a chapter to the page mdBook
  emits.
- The site is **static-only**: `bower push` force-pushes the rendered directory
  to a branch (`Forge::push_tree`, `bower/src/forge.rs:127`) beside a
  `.nojekyll` (`bower/src/publish.rs:903`). There is no server to ask, and
  nothing here needs one.

What the idea assumed and the code does not have:

- **There is no reverse line map.** A3 calls the (path, lines) → anchor map
  one "the preprocessor already computes for § 3.4's footers." It does not.
  `resolve_ranges` (`display.rs:203`) computes the *forward* map — block →
  lines, at the block's own step, by first-match window search
  (`display.rs:241`) — and nothing inverts it. It is also only true at that
  step: the next `append` or `region` shifts every line below it. "Written by"
  at step 017 is **line provenance carried across steps** — blame — and blame
  needs a line diff the kernel does not have.
- **The preprocessor cannot write files.** A3 puts the widget "in
  `mdbook-bower`", which reads the book on stdin and writes it to stdout
  (`bower/src/mdbook_bower.rs:70`). Files beside the HTML are written by
  `MdBookRenderer::render` after `mdbook build`, as `.nojekyll` and
  `.bower-site` already are (`publish.rs:961`).
- **The kernel's tree is not the git tree.** Scaffolding is read from disk and
  laid under every step (`bower/src/replay.rs:106`, `replay.rs:308`);
  `STEPS.md` exists only in the final tree (`replay.rs:330`). Measured on a
  scratch `bower build` of `books/hello-playbook` at HEAD: 30 unique blobs,
  65,506 bytes — the template's seven files are 56,072 and `STEPS.md` 4,382.
  What the *book* wrote is **21 unique blobs, 5,033 bytes, 11 paths, 26
  hunks** across 20 steps.
- **There is no front-end code in the repository.** `git ls-files` finds no
  `.js` and no `.html`. The only styling is a 43-line `theme/step-meta.css`,
  duplicated per book and wired through `additional-css` in each `book.toml`.
  This EPIC writes the project's first JavaScript.
- **The kernel refuses serialization and dependencies.** "No format crate
  appears in any signature" (`bower-core/src/lib.rs:13`), and `make purity`
  fails if `bower-core` grows a dependency. `bower` has `serde_json` only
  behind the `preprocessor` feature (`bower/Cargo.toml:31`); `make minimal`
  builds without it. Git blob ids live in `gix`, at the edge.
- **"Scrub" is taken.** EPIC-11 names its normalizer's prefix table `Scrub`.
  The kernel value here is a `Timeline`; *scrubber* names only the widget.

**This EPIC does not** add a server, a search index, or a fetch to any host
but the book's own; does not diff arbitrary step pairs (Decision 6); does not
ship anything to epub or PDF, which render byte-for-byte as today; does not
embed scaffolding *content*; does not render EPIC-12's file index — it shares
its fold; and does not change `plan()`, `bower.lock`, any generated repository
byte, or replay, verify, status, and push beyond one input to the site
fingerprint.

---

## Status

| Component | Status |
|---|---|
| Kernel: `diff` — deterministic line diff | **Planned** |
| Kernel: `timeline` — interned blobs, per-step changes, blame | **Planned** |
| Kernel: `FileBiography` reused from EPIC-12's fold | **Planned** |
| Testkit: history textures, the reconstruction property | **Planned** |
| Edge: `bower/src/scrubber.rs` — payload text, schema v1 | **Planned** |
| Publish: payload and widget written beside the HTML | **Planned** |
| Render: footer data attributes, `scrub` link, bootstrap (HTML only) | **Planned** |
| Widget: tree, slider with chapter ticks, file/diff view, written-by gutter | **Planned** |
| Diff-reading mode | **Planned** |
| File biography mount (in EPIC-12's file index) | **Planned** |
| Branches as forks in the slider (after EPIC-09) | **Planned** |
| Sample book and slow-lane golden | **Planned** |

---

## Goals

- A reader can **slide through every step** of a repo, in the book, with no
  forge, no clone, and no server.
- Every line in view answers **"written by"** — step, chapter, and a link to
  the paragraph.
- The payload is a **pure fold** over the `RepoPlan`: tested in the kernel,
  byte-identical across runs, never a second source of truth.
- The widget is **small, dependency-free, and dumb**: it draws what the kernel
  computed and computes nothing a test would want to check.
- Every other target **degrades cleanly**: epub and PDF are unchanged; HTML
  without JavaScript still has working links.

## Scope

### Things

| Thing | Meaning |
|---|---|
| **Blob** | One distinct file content in a repo's history, interned once. |
| **Change** | What a step did to one path — a `FileEvent` (EPIC-12), the new blob, the edit script. |
| **Edit script** | A run-length line diff: keep *n*, delete *n*, insert *n*. |
| **Origin** | The step that last wrote a line. |
| **Blame** | Run-length origins for every line of a file after a change. |
| **Chapter tick** | A chapter's last step — where `<chapter>-end` points. |

### Business requirements

1. Replaying the changes from an empty tree reproduces **every**
   `PlannedStep::tree` exactly.
2. A line's origin is the step that inserted it; a line a step leaves
   untouched keeps its origin, even under `op="replace"`. That is git blame's
   rule, so the gutter and the forge agree.
3. Equal contents share one blob; binary files carry a size, never bytes.
4. Equal inputs give a byte-identical payload.
5. Only `Target::Html` gets data attributes, the `scrub` link, or the
   bootstrap. Epub and PDF render exactly as today.
6. With no script, every `scrub` link still lands somewhere true.
7. A repo with no steps gets no payload.

---

## Decisions

1. **Blame is a fold over diffs, and the diff is the kernel's.** Provenance
   cannot come from the step-local forward map (`display.rs:203`).
   `diff::lines(old, new)` — Myers, deletions before insertions on ties — runs
   once per changed path per step; `Keep` runs inherit origins, `Insert` runs
   take the step's `seq`. The same scripts *are* the diff view: one diff in
   the project, not a Rust one for blame and a JavaScript one for display.
2. **Changes come from EPIC-12's biographies, not a second tree comparison.**
   EPIC-12 Decision 5 already walks consecutive trees into `FileBiography`
   events, with the parent from one helper (its Decision 6). The timeline
   turns each `(FileEvent, step)` into a `Change` and adds the blob, script,
   and blame. Whichever EPIC lands first writes `FileEvent`,
   `FileBiography`, and the parent helper in `bower-core/src/backmatter.rs`,
   with EPIC-12's shapes.
3. **Blobs are interned, not hashed.** Ids are dense integers in first-seen
   order (step, then path). The kernel has no SHA-1 and must not grow one;
   the payload is not a git object; two digits beat forty hex characters.
4. **Changes, not snapshots.** A step records only what changed against its
   parent; the widget replays forward. Full maps cost 20 × 11 entries for
   `hello-playbook` and 100 × 40 for a real book. Requirement 1 is a property
   over `arb_book` (`bower-testkit/src/generators.rs:52`).
5. **The kernel returns a value; the edge writes text.** The writer is
   hand-rolled — an escaper and five shapes — because `make minimal` builds
   `bower` without `serde_json`, and the project has declined a dependency for
   less (`digest`, `publish.rs:831`; `base64`, `publish.rs:415`). A
   dev-dependency round-trip (`bower/Cargo.toml:48`) keeps it honest.
6. **Diff against the parent only.** A3's *What* says "any two steps"; its
   *Mechanism* says "diff-against-previous". Blame needs the latter anyway.
   Arbitrary pairs need a JavaScript diff or a quadratic payload — open
   question 1.
7. **One file per repo, loadable by `<script>`.** `bower/<repo>.timeline.js`
   is `bowerTimeline(` on its first line, `);` on its last, JSON between. A
   script tag works on `file://` — where an author previews `make book` and
   `fetch` is refused — and on any static host, with no CORS.
8. **Scaffolding is paths, not content.** The kernel never sees the template;
   the edge passes its paths, and the widget lists them greyed, linked to the
   forge `tree` URL when there is one. Content would multiply the sample's
   payload six-fold to show licences. `STEPS.md` is no step's content
   (`replay.rs:326`) and is omitted — EPIC-12 Decision 5 draws the same line.
9. **The bootstrap reads mdBook's `path_to_root`.** The preprocessor appends
   one inline `<script>` to each HTML chapter that has a footer; it loads
   `path_to_root + "bower/scrubber.js"`. mdBook defines that global for its
   own search script (present in the installed v0.4.52). Nested chapters and
   `print.html` resolve without the book editing `book.toml`; absent the
   global, the bootstrap falls back to a prefix computed from the chapter
   path. Plain `mdbook build` gets the tag and no files: nothing loads, links
   still work.
10. **The no-JavaScript `scrub` link is a real page.** Its `href` is the
    path's entry in EPIC-12's file index when the book has one, else the full
    file at the step (`full_file_url`, `render.rs:245`), else the step's own
    anchor. The widget intercepts the click; nothing depends on it.
11. **The widget never builds HTML from book text.** Code goes in by
    `textContent`. A Rust test fails if the embedded script contains
    `innerHTML`, `eval`, or an absolute URL. A book about Rust is full of `<`.
12. **The widget joins the site fingerprint.** `site_fingerprint`
    (`publish.rs:854`) covers lock and chapter sources, hence the payload —
    but not `scrubber.js`. A bower upgrade that changed it would leave
    `status` saying "in sync", the direction `docs/TECHNICAL_DEBT.md` refuses.
    `digest(SCRUBBER_JS)` joins the input.
13. **Exercises keep their fold.** An answer step folds shut in HTML
    (`fold_label`, `render.rs:497`). The slider marks exercise and answer
    steps and never auto-advances past an exercise; reaching the answer is
    the reader's click, as opening the fold is.

---

## Domain map

| Domain concept | Code construct | Status |
|---|---|---|
| A step's full tree | `PlannedStep::tree` `plan.rs:50` | 🟢 exists |
| Where a block's lines sit, at its step | `BlockDisplay::ranges` `display.rs:58` | 🟢 forward only |
| A line diff | `diff::lines` | 🔴 new |
| A file's biography | `FileBiography` (EPIC-12, `backmatter.rs`) | 🔴 new, shared |
| A line's origin at a later step | `Change::blame` | 🔴 new |
| Paragraph anchor | `#step-<id>` `render.rs:72` | 🟢 exists |
| Chapter → page | `html_name` `trailers.rs:117` | 🟡 private; made `pub` |
| Chapter title | `heading_of` `publish.rs:531` | 🟢 edge |
| Chapter tick | `chapter_stem` `replay.rs:396`; `-end` tags `replay.rs:146` | 🟢 edge |
| Scaffolding | `scaffolding` `replay.rs:308` | 🟢 edge; paths only |
| Files beside the HTML | `write_site_files` `publish.rs:903` | 🟡 pattern to follow |
| The footer | `footer` `render.rs:286` | 🟡 gains a target |
| The payload, as text | `bower/src/scrubber.rs` | 🔴 new |
| The widget | `bower/assets/scrubber.{js,css}` | 🔴 new |

---

## Design

### Kernel — `bower-core/src/diff.rs` (new)

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Edit { Keep(usize), Delete(usize), Insert(usize) }

/// Myers' O(ND) line diff; deletions before insertions on ties; adjacent
/// runs of one kind merged. Pure.
#[must_use]
pub fn lines(old: &str, new: &str) -> Vec<Edit>;
```

Lines split by `str::lines`, as `resolve_ranges` does (`display.rs:216`), so a
gutter line and a footer's `L18–L21` count the same lines. The tie rule is
pinned by a test: it decides which of two equal-cost scripts the reader sees,
and a changed script is a changed payload.

### Kernel — `bower-core/src/timeline.rs` (new)

```rust
pub struct BlobId(pub usize);
pub enum Blob { Text(String), Binary { bytes: usize } }

pub struct Timeline {
    pub repo: RepoName,
    pub blobs: Vec<Blob>,           // index is the BlobId
    pub scaffold: Vec<String>,      // paths, from the caller, sorted
    pub steps: Vec<TimelineStep>,
}

pub struct TimelineStep {
    pub seq: usize,
    pub id: StepId,
    pub tag: String,                // PlannedStep::tag
    pub expect: Expect,
    pub anchor: Location,
    pub parents: Vec<usize>,        // from EPIC-12's parent helper
    pub exercise: bool,
    pub changes: Vec<Change>,       // path order
}

pub struct Change {
    pub path: String,
    pub event: FileEvent,           // EPIC-12
    pub blob: Option<BlobId>,       // None when Deleted
    pub edits: Vec<Edit>,           // empty for binary and Deleted
    pub blame: Vec<(usize, usize)>, // (run length, origin seq); text only
}

#[must_use]
pub fn timeline(repo: &RepoPlan, biographies: &[FileBiography], scaffold: &[String]) -> Timeline;

impl Timeline {
    /// The tree at `seq`, by replaying changes — what the widget does.
    #[must_use]
    pub fn tree_at(&self, seq: usize) -> BTreeMap<String, BlobId>;
    /// The blame of `path` at `seq`: the latest change to it at or before.
    #[must_use]
    pub fn blame_at(&self, seq: usize, path: &str) -> Option<&[(usize, usize)]>;
}
```

Blame lives on `Change` because it moves only when the file does; the widget
looks up the latest change at or before the cursor. `tree_at` and `blame_at`
let the property and the golden assert, in Rust, exactly what the widget does
in JavaScript.

### Edge — `bower/src/scrubber.rs` (new)

```rust
pub struct ChapterTick { pub path: String, pub html: String, pub title: String, pub end: usize }

/// `bowerTimeline(\n{…}\n);\n` — schema v1, keys in a fixed order.
#[must_use]
pub fn payload(t: &Timeline, book: &str, chapters: &[ChapterTick]) -> String;

pub const SCRUBBER_JS: &str = include_str!("../assets/scrubber.js");
pub const SCRUBBER_CSS: &str = include_str!("../assets/scrubber.css");
```

```json
{"bower_timeline":1,"book":"hello-playbook","repo":"hello-playbook",
 "chapters":[{"path":"src/ch01-a-repo-that-builds.md","html":"ch01-a-repo-that-builds.html","title":"A repo that builds","end":2}],
 "scaffold":[".gitignore","CODE_OF_CONDUCT.md","LICENSE-MIT"],
 "blobs":["[package]\nname = \"hello-playbook\"\n…",{"binary":1043}],
 "steps":[{"seq":1,"id":"cargo-init","tag":"step-001-cargo-init","expect":"pass",
   "anchor":"ch01-a-repo-that-builds.html#step-cargo-init","parents":[0],
   "changes":[{"path":"Cargo.toml","event":"created","blob":0,"edits":[["+",6]],"blame":[[6,1]]}]}]}
```

**Size.** From the measured inputs: 5,033 bytes of blob text, about 5.6 KB
escaped; 20 steps of metadata near 3 KB; 26 hunks' scripts and blame runs
under 2 KB; chapters and scaffold paths under 1 KB — **about 11 KB for
`hello-playbook`**, a few KB as Pages serves it gzipped. Scaffolding content
would add 56 KB (Decision 8). A book's payload grows with what it *writes*.

### Publish — `bower/src/publish.rs`, `bower/src/main.rs`

`MdBookRenderer` (`publish.rs:916`) gains `extra: Vec<(String, Vec<u8>)>`,
written under `dest` after `mdbook build`, where `write_site_files` runs
(`publish.rs:961`). `run_publish` (`main.rs:555`) fills it for `Target::Html`
only: `bower/scrubber.js`, `bower/scrubber.css`, and one
`bower/<repo>.timeline.js` per `RepoPlan`. Every path goes through
`check_path` (`bower/src/materialize.rs:87`), the guard
`DEFECT_Path_Traversal.md` put at every write. `RenderPlan` is untouched, so
`PandocRenderer` and `TypstRenderer` cannot see any of it.

The HTML render happens twice (EPIC-06, still open; `publish.rs:909`): the
payload comes from `run_publish`'s plan, the page from the preprocessor's.
They agree because `plan()` is deterministic (`plan.rs:116`) and both read
the same loader convention (`bower/src/mdbook.rs:95`). The slow golden checks
it rather than trusting it.

### Render — `bower/src/render.rs`

`footer` gains `target: Target`. For HTML the span carries `data-repo`,
`data-step`, `data-path`, and `data-lines="18-21"`, and the parts gain a
`[scrub](…)` link with the Decision 10 `href`. For epub and PDF the output is
byte-identical to today. `chapter` appends the bootstrap for HTML in any
chapter with at least one footer; the sample's prose-only appendix gets none.

### Widget — `bower/assets/scrubber.js` (new)

Hand-written ES2017; no build step, no npm, no framework; **16 KB
unminified**, asserted on `SCRUBBER_JS.len()`. It:

- loads `bower/<repo>.timeline.js` on first use, never on page load;
- turns each `.step-meta[data-repo]` `scrub` link into a toggle that opens one
  panel per page at that step, file, and range;
- draws the **file tree** at the cursor (scaffold greyed), the **slider** with
  a tick per chapter end and a mark per `compile_fail`, `test_fail`, and
  exercise step, and the **file** or **diff** view;
- draws the **written-by gutter**: each blame run a band labelled
  `step 011 · ch04`, linked to its page and `#step-<id>`;
- takes ←/→, Home/End, and keeps `#scrub=<repo>/<seq>/<path>` in the URL, so a
  scrubber state is a link.

`scrubber.css` uses mdBook's theme variables, as `step-meta.css` does, so every
built-in theme works without a book touching its own.

### Diff-reading mode

For every footer whose step has a `Change` for its `data-path`, the widget
adds a `prose | diff` toggle above the fence and remembers the choice in
`localStorage` — per reader, not per book. Diff mode shows the step's edit
script over the old and new blobs in place of the block, for display only.
Print has no toggle and gets no diff.

### File biography

EPIC-12's file index is the text twin: one anchored section per path, in every
target. In HTML, this EPIC adds `<div class="bower-scrub" data-repo data-path>`
to each section, and the widget fills it with a scrubber limited to that
file's changes. Without EPIC-12 there is no biography page, and Decision 10's
fallback applies.

### Branches — after EPIC-09

EPIC-09 keeps `seq` global across lines and gives steps `line` and `parents`
(its Decisions 5 and § Design). The payload carries `parents` from v1, so the
schema keeps its shape. The slider draws main as the rail and each branch as a
spur leaving at its fork tick and, if merged, rejoining at the merge; ←/→
follow the cursor's line. Diffs are against `parents[0]`. A merge's blame
diffs against both parents: lines kept from the first keep its origin, then
lines kept from the second, then the merge's own `seq`.

---

## Work Items

### Phase 0 — Kernel: the diff and the shared biography

- [ ] **0a.** `bower-core/src/diff.rs`: `Edit`, `lines`, the tie rule, the
  apply-yields-new property.
- [ ] **0b.** `FileEvent`, `FileBiography`, and the parent helper in
  `bower-core/src/backmatter.rs` — reused if EPIC-12 landed them, written to
  its shapes if not.
- [ ] **0c.** Export through `prelude` (`bower-core/src/lib.rs:39`);
  `make purity` still prints one line.

### Phase 1 — Kernel: the timeline

- [ ] **1a.** `bower-core/src/timeline.rs`: interning, changes from
  biographies, `exercise` from `PlannedStep::exercise` (`plan.rs:55`).
- [ ] **1b.** Blame as a fold over `diff::lines`; binary carries a size.
- [ ] **1c.** `Timeline::tree_at`, `Timeline::blame_at`.
- [ ] **1d.** Testkit: a `history` axis in `bower-testkit/src/coverage.rs` —
  created, changed, deleted, re-created, binary — and a fixture per state the
  corpus lacks.
- [ ] **1e.** The reconstruction property over `arb_book`.

### Phase 2 — Edge: the payload and the files

- [ ] **2a.** `bower/src/scrubber.rs`: `payload`, the escaper, schema v1;
  `html_name` made `pub` (`trailers.rs:117`), not copied.
- [ ] **2b.** `MdBookRenderer::extra` through `check_path`; `run_publish`
  fills it for HTML only.
- [ ] **2c.** `digest(SCRUBBER_JS)` joins `site_fingerprint`'s input.

### Phase 3 — Render hooks

- [ ] **3a.** `footer` takes a `Target`; data attributes and the `scrub` link
  for HTML; the Decision 10 `href` chain.
- [ ] **3b.** The bootstrap, HTML only, only in chapters with a footer.
- [ ] **3c.** Preprocessor golden: one bootstrap per code chapter of the
  sample book, none in the appendix.

### Phase 4 — The widget

- [ ] **4a.** `bower/assets/scrubber.js` and `.css`: panel, tree, slider,
  ticks, file view, gutter, keyboard, fragment.
- [ ] **4b.** Rust guards on the embedded script: budget, no `innerHTML`, no
  `eval`, no absolute URL.
- [ ] **4c.** The hand checklist below, in two browsers, over `file://` and
  the published site.

### Phase 5 — Diff-reading mode and the biography

- [ ] **5a.** The `prose | diff` toggle, `localStorage`-backed.
- [ ] **5b.** The biography mount inside EPIC-12's file index, once it exists.

### Phase 6 — Branches (gated on EPIC-09 Phase 1)

- [ ] **6a.** `parents` and `line` from the plan into the payload.
- [ ] **6b.** Two-parent blame on merges.
- [ ] **6c.** Spurs in the slider; ←/→ follow the line.

### Phase 7 — Book and docs

- [ ] **7a.** One paragraph in `books/hello-playbook` chapter 4 pointing the
  reader at the scrubber, beside step 011.
- [ ] **7b.** `.okf/model/timeline.md` beside `.okf/model/tree-state.md`;
  `BACKLOG.md` row moved out of § Ideas; `README.md`.
- [ ] **7c.** Flip Status rows; append the corrigendum.

---

## Test Plan

Kernel:
`diff__identical_texts_are_one_keep`,
`diff__deletions_come_before_insertions_on_a_tie`,
`diff__applying_the_script_to_old_yields_new` (property),
`diff__counts_lines_as_resolve_ranges_does`,
`timeline__changes_replay_to_every_planned_tree` (property, Requirement 1),
`timeline__changes_agree_with_back_matter_biographies`,
`timeline__a_step_with_no_files_has_no_changes` (sample step 002),
`timeline__a_deleted_path_leaves_the_tree` (sample step 020, `src/scratch.rs`),
`timeline__identical_contents_share_one_blob`,
`timeline__binary_is_a_size_not_bytes`,
`timeline__replace_keeps_the_origin_of_unchanged_lines`,
`timeline__append_blames_only_the_appended_lines`,
`timeline__region_fill_blames_only_the_region`,
`timeline__scaffold_is_paths_only`,
`timeline__is_deterministic`.

Edge:
`payload__round_trips_through_serde_json`,
`payload__escapes_every_control_character`,
`payload__is_byte_identical_across_runs`,
`payload__strips_to_json_by_its_first_and_last_line`,
`footer__html_carries_data_attributes_and_a_scrub_link`,
`footer__epub_and_pdf_are_unchanged`,
`render__bootstrap_only_in_html_chapters_with_a_footer`,
`render__scrub_link_falls_back_to_the_full_file`,
`publish__html_writes_payload_and_widget_beside_index`,
`publish__epub_and_pdf_write_no_scrubber`,
`site_fingerprint__changes_with_the_widget`,
`widget__stays_under_budget_and_never_uses_inner_html`.

Slow lane (`#[ignore]`, real `mdbook build`):
`sample_book_every_footer_resolves_in_its_payload` — every `data-step` and
`data-path` in the rendered HTML names a step and path the payload has, and
every `data-lines` range lies inside that blob at that step.

## Key Files

| File | Role |
|---|---|
| `bower-core/src/diff.rs` | new: the one line diff |
| `bower-core/src/timeline.rs` | new: blobs, changes, blame |
| `bower-core/src/backmatter.rs` | EPIC-12's; `FileBiography` and the parent helper |
| `bower-testkit/src/coverage.rs`, `fixtures.rs` | the `history` axis, textures |
| `bower/src/scrubber.rs` | new: payload text, embedded assets |
| `bower/assets/scrubber.js`, `scrubber.css` | new: the widget |
| `bower/src/render.rs` | footer target, data attributes, link, bootstrap |
| `bower/src/publish.rs` | `MdBookRenderer::extra`; fingerprint input |
| `bower/src/main.rs` | `run_publish` fills `extra` for HTML |
| `bower/src/trailers.rs` | `html_name` made public |
| `bower/tests/preprocessor.rs`, `publish.rs` | goldens and the slow lane |

## Reuse (do NOT recreate)

- `plan.rs:50` `PlannedStep::tree` — the fold's input; never re-apply blocks.
- EPIC-12's `FileBiography` and parent helper — the only tree comparison and
  the only definition of "parent".
- `plan.rs:108` `PlannedStep::tag` — the only tag format.
- `trailers.rs:117` `html_name` — the only chapter → page rule.
- `replay.rs:396` `chapter_stem` — ticks follow the rule the `-end` tags use.
- `replay.rs:308` `scaffolding` — the answer `build`, `verify`, `status`, and
  `push` already share.
- `render.rs:245` `full_file_url` — the no-JavaScript fallback.
- `publish.rs:531` `heading_of` — tick titles.
- `publish.rs:903` `write_site_files` — the pattern for files beside the HTML.
- `materialize.rs:87` `check_path` — every write.
- `publish.rs:831` `digest` — the fingerprint addition.

## Compatibility

- **Preserves** `plan()`, `bower.lock`, every generated SHA, and every epub
  and PDF byte. `cargo tree -p bower-core -e normal` prints one line;
  `make minimal` builds.
- **Adds** two kernel modules, one edge module, two embedded assets, three
  files under `bower/` per HTML render, four data attributes and a link per
  HTML footer.
- **Changes** `site_fingerprint` for every book, once: the first `status`
  after upgrading calls the site stale, which is true.
- **Breaks** nothing; `footer`'s new parameter is crate-internal.

## Dependencies

- **Built on:** EPIC-03 (footers, anchors), EPIC-06 (`Target`,
  `MdBookRenderer`), EPIC-08 (the static site, its fingerprint).
- **Shares a fold with:** EPIC-12 — `FileBiography` is its file index and this
  EPIC's per-file view; whichever lands second reuses it.
- **Coordinates with:** EPIC-09 (Phase 6 waits on `line` and `parents`);
  EPIC-13 (Decision 13, exercise and answer marks).
- **Related:** EPIC-11 — a `compile_fail` tick could show captured stderr once
  there is some; mind the `Scrub` name. EPIC-14 — `bower follow` diffs a real
  clone with `git`; the scrubber is its in-browser twin, and if `follow` ever
  wants a kernel diff it is `diff::lines`. EPIC-16 — a frozen edition's site
  carries its own payload, so pinning costs nothing.

## Verification

```bash
make test
make lint                      # clippy pedantic, then `make purity`
make minimal                   # bower without serde_json still builds
make plan                      # the locks do not move
make book                      # html, with mdbook-bower on PATH
ls books/hello-playbook/book/bower/
make slow                      # the real-mdbook golden
make epub pdf                  # unchanged artifacts
```

By hand, in two browsers, over `file://` and the published site: open
chapter 4, click `scrub` on step 011's footer; the panel opens on `src/lib.rs`
at L18–L21; ← shows step 010; the `step 011` band links to
`#step-test-that-fails`; step 020 shows `src/scratch.rs` deleted; the diff
toggle survives a reload; with JavaScript off, `scrub` opens a real page.

Exit criteria:

1. `books/hello-playbook/book/bower/` holds `scrubber.js`, `scrubber.css`, and
   `hello-playbook.timeline.js`, the last under 16 KB.
2. Replaying the payload reproduces every `PlannedStep::tree`, over the
   fixture corpus and `arb_book`.
3. Every line of every file at every step names an origin, and a line
   untouched since step *k* names *k*.
4. The sample book's epub and PDF are byte-identical before and after.
5. Two publishes give byte-identical payloads; `bower-core` still has no
   dependencies.

---

## Open questions — decisions, not code

| # | Question |
|---|---|
| 1 | **Arbitrary-pair diffs.** A3 promises "the diff between any two steps." A widget-side diff is a second implementation of `diff::lines`; precomputed pairs are quadratic. Parent only, or a JavaScript diff held to the kernel's by a shared golden? |
| 2 | **Scaffolding content.** Paths keep the sample at 11 KB; content adds 56 KB of licences and makes the tree honest. Per-repo opt-in, or never? EPIC-12's Q6 asks the same of the file index; answer both at once. |
| 3 | **Raw JSON too.** EPIC-14 or a classroom tool may want plain `.json`. Two files, or document "strip the first and last line"? |
| 4 | **The gutter as a check.** Footer ranges come from a first-match search (`display.rs:241`). Blame can tell whether a shown span's lines were written by its step. Warn when none were — or is that an honest `op="replace"` of unchanged lines, never a warning? |
| 5 | **`path_to_root` is mdBook's, not ours.** Decision 9 leans on a global mdBook defines for search. Pin it with the slow lane, or use only the preprocessor-computed prefix? |
| 6 | **Opting out.** No key in v1, per the admission rule. A repo with a large generated file may want `scrub = false`; add it when a book asks. |

---

*Measured 10 September 2026 against `ImperialBower/bower` @ HEAD ("docs: file
the forge design and add it to the backlog"): a scratch `bower build` of
`books/hello-playbook` — 21 commits, 26 tags, 30 unique blobs — and the
installed mdBook v0.4.52. Nothing in this EPIC is built.*
