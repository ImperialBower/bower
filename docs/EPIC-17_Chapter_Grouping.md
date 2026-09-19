# EPIC-17: Parts — grouping chapters under a title (PARTS)

## Summary

- **Builds:** parts: an author writes `# Part I: The repo` in `SUMMARY.md`,
  and the chapters under it are grouped in the HTML, the epub, and the PDF.
- **Why:** the HTML already shows parts, because mdBook draws them. The epub
  and the PDF drop them without a word, so one book reads three ways.
- **Shape:** a `PartMark { title, first }` beside the flat chapter list, never
  around it. The loader reads it, the planner ignores it, and the two pandoc
  renderers write one divider file per part.
- **Proves it:** the sample book gains two parts, its epub `nav.xhtml` lists
  both, and every SHA `make build` produces is unchanged.
- **Status:** Planned, filed 19 September 2026, nothing started. Rewritten the
  same day after an evaluation against the code; see the footer.

---

## Context

A book is a flat list of chapters. `BookSource.chapters` is a `Vec<Chapter>`
in reading order (`bower-core/src/source.rs:10-17`), and the loader fills it
from `SUMMARY.md` with `chapter_links` (`bower/src/loader.rs:107-125`), which
looks for `](` on each line and keeps every local `.md` target. It is
"deliberately not a full markdown parser" (`loader.rs:105`). It reads no
headings, no indentation, and no separators, so a nested chapter is flattened
and a heading is skipped.

mdBook already has parts. An `# H1` inside `SUMMARY.md` is a *part title* for
the numbered chapters that follow it, and the first H1, `# Summary`, is the
file's own title and is ignored. Three facts follow, all measured on
19 September 2026 on a copy of `hello-playbook` with two part titles added
(mdBook 0.4.52, pandoc 3.11):

- **HTML works today, with no code.** `bower publish --target html` shells
  out to `mdbook build` (`bower/src/publish.rs:926-954`). The build wrote
  `<li class="part-title">Part I: The repo</li>` into `toc.html`, and
  `mdbook-bower` still ran on every chapter. The loader skipped the heading
  lines because they hold no link. The preprocessor skips part items because
  `collect` follows only `Chapter` items (`bower/src/mdbook.rs:82-92`), and
  its doc comment says so: "Separators and parts carry none and are skipped"
  (`mdbook.rs:72-75`).
- **The epub drops parts silently.** `render_plan` maps `book.chapters`
  straight into `RenderPlan.chapters` (`publish.rs:514-522`), and
  `write_chapters` hands pandoc one numbered file per chapter
  (`publish.rs:647-672`). The part titles never leave `SUMMARY.md`. The
  spiked epub's `nav.xhtml` held no "Part I".
- **The PDF drops them the same way.** `TypstRenderer` calls the same
  `write_chapters` (`publish.rs:1047`).

One more gap sits beside these. `site_fingerprint` hashes the lock text and
each chapter's path and text (`publish.rs:854-861`). It never sees
`SUMMARY.md`. Chapter order is covered, because the paths are hashed in
order. A part title is not, so an author who renames a part is told the
published site is in sync when it is not.

This EPIC does **not** change the plan, the lock, a step, a tag, a commit, or
a SHA. A part groups chapters for a reader and means nothing to a generated
repository. `Location` stays a chapter path and a line (`source.rs:58-62`).
The preprocessor is left alone.

---

## Status

| Component | Status |
|---|---|
| Tests pinning what already works (HTML parts, loader skips headings) | **Planned** |
| Kernel: `PartMark`, `BookSource.parts`, `with_parts`, `PartError` | **Planned** |
| Testkit: `arb_parts`, the plan-ignores-parts property | **Planned** |
| Loader: `summary` reads part titles; `LoadError::EmptyPart` | **Planned** |
| Render: `RenderPlan.parts`, `part_divider`, `write_chapters` interleaves | **Planned** |
| `site_fingerprint` covers part marks | **Planned** |
| Sample book gains two parts; fixture follows; slow lanes | **Planned** |
| README, spec, backlog | **Planned** |

---

## Goals

- Make a **part** a value the book carries, read from the one place mdBook
  already reads it.
- Make the **three targets agree**. A part the HTML shows is a part the epub
  and the PDF show.
- Keep the **chapter list flat**. About thirty call sites index
  `book.chapters` or `plan.chapters`, and none of them should move.
- Keep the **kernel's plan blind to parts**, and prove it with a property.
- Keep a **book without parts byte for byte** what it is today: the same
  scratch files, the same epub, the same PDF, the same fingerprint.

## Scope

### Things

| Thing | What it is |
|---|---|
| **Part title** | An `# H1` line in `SUMMARY.md`, other than the file's own title. Author's markdown, kept as written. |
| **Part mark** | A title and the path of the first chapter under it. It says where a part opens. It does not say where it closes, because mdBook does not either. |
| **Summary title** | The first H1, when nothing but blank lines comes before it. `# Summary`. Never a part. |
| **Divider** | One generated markdown file per part, handed to pandoc between two chapters. |
| **Partless book** | A book whose `SUMMARY.md` has no part title. Every book that exists today. |

### Business requirements

1. Parts come from `SUMMARY.md` and nowhere else. No directory layout, no
   `bower.toml` key, no directive.
2. Bower reads a part title exactly where mdBook does, so the HTML and the
   other two targets never disagree about what is a part.
3. A part mark names a chapter the book holds. Marks are in reading order,
   and no two marks name the same chapter.
4. A part with no chapter under it is refused at load time, by title. It is
   almost always a heading left behind after a chapter moved.
5. The plan is a function of chapters alone. Two books that differ only in
   their parts produce the same `BookPlan` and the same lock text.
6. A partless book renders exactly as it does today, in every target.
7. A changed part title, a moved part, or a removed part changes the site
   fingerprint. A partless book keeps the fingerprint it has now.

The business logic is § Design and § Work Items.

### What the reader gets

- In the epub's table of contents: *Part I: The repo*, then its chapters,
  then *Part II: The proof*, then its chapters. Each part title opens a page
  of its own.
- In the PDF: each part title on a page of its own, before its first chapter.
- In the HTML: what mdBook draws today, now pinned by a test.

### Not in scope

- **A second level** (books that hold parts that hold chapters). mdBook has
  one level. An author who wants "Book One" writes it as a part title. Open
  question 2.
- **A nested table of contents**, where chapters sit *under* their part. It
  was measured and it works, at a price. Decision 5 and open question 1.
- **`part_of(chapter)`.** Nothing in this EPIC asks which part a chapter is
  in, and the answer needs a rule for where a part ends. EPIC-12 or EPIC-14
  can add it when one of them needs it.
- **Part tags** (`part-1-end`, beside `<chapter>-end` at
  `bower/src/replay.rs:168`). Open question 3.
- **Nested chapters and separators.** The loader flattens one and skips the
  other today, and keeps doing so.
- **The preprocessor.** `mdbook::book_source` (`mdbook.rs:147-154`) leaves
  `parts` empty. It renders one chapter at a time and mdBook owns the rest.
- **`--target ipynb`**, which does not exist yet.

---

## Decisions

1. **mdBook's syntax, not a new one.** A part is an `# H1` in `SUMMARY.md`.
   The HTML target already honours it with no code, so any other spelling
   would make the HTML disagree with the epub. An unlinked list item
   (`- Part One`) is not a part in mdBook. It is a draft chapter.
2. **Marks beside the list, not a tree around it.** `BookSource` keeps
   `chapters: Vec<Chapter>` and gains `parts: Vec<PartMark>`. The obvious
   alternative, `parts: Vec<Part { title, chapters }>`, was rejected for
   three reasons. It rewrites about thirty call sites (`publish.rs:515`,
   `:856`, `main.rs:296`, `block.rs:84`, `tests/site.rs:176`,
   `tests/publish.rs:73-127`, and the rest) for a thing the planner never
   reads. It needs a part with an empty title to hold the chapters that come
   before the first part, and an empty string standing for "no part" is a
   state that should not be writable. And it is not mdBook's model: mdBook's
   `PartTitle` is a sibling of the chapters in one flat item list.
3. **The mark names its chapter by path.** `first` is `src/ch04-….md`,
   "book-relative and `src/`-prefixed, as everywhere else in this crate"
   (`publish.rs:456`). An index would be shorter and would go stale the
   moment a test edits `chapters`.
4. **Parts live on `BookSource`, and the planner ignores them.** They are
   book source: they come from the same file chapter order comes from.
   `render_plan` and `site_fingerprint` already take `&BookSource`
   (`publish.rs:500-508`, `:854`), so no signature changes and no call site
   in `main.rs` (`:633`, `:681`, `:776`) moves. `BookSource` derives
   `Default` and `from_chapters` fills the rest with `..Self::default()`
   (`source.rs:22-27`), so its 52 callers compile untouched.
   `plan__ignores_parts` holds requirement 5.
5. **A divider, not a heading shift.** Both shapes were spiked through
   pandoc 3.11 on 19 September 2026.
   - *Divider.* One file, `# Part I: The repo`, between two chapter files.
     The epub gave the part its own page (`ch001.xhtml`) and a table of
     contents entry, as a sibling of the chapters. No chapter's markdown
     changes. This is also exactly what mdBook's sidebar shows: a label in a
     flat list.
   - *Nested.* The same file, every chapter's headings pushed down one level,
     and `--split-level=2`. The table of contents nested correctly. The
     price: pandoc's own `--shift-heading-level-by` would move the part too,
     and running each chapter through pandoc to shift it reflowed the prose
     and turned Bower's `<a id="step-…">` anchors into raw-HTML spans. So the
     shift would have to be Bower's own and fence-aware, the sample
     template's `heading.where(level: 1)` rule
     (`books/hello-playbook/template.typ:26`) would start styling parts
     instead of chapters, and chapters outside any part would have to stay
     unshifted.

   The divider ships. Nesting is open question 1.
6. **One divider text for both targets.** The divider wraps its heading in
   raw Typst page breaks, written as pandoc raw blocks. Measured: the Typst
   writer emits `#pagebreak(weak: true)` on each side of
   `#heading(level: 1, numbering: none)[Part I: The repo]`, and the epub
   writer drops the raw blocks entirely. `part_divider` is one pure function
   with no `match` on `Target`. The HTML target never sees a divider:
   `MdBookRenderer` builds from the book root, not from `write_chapters`.
7. **One counter names the scratch files.** `write_chapters` numbers files so
   "the order an external tool receives is visible on disk"
   (`publish.rs:666`). Dividers join that count: `001-part.md`,
   `002-ch01-a-repo-that-builds.md`. A partless book writes no divider, so
   its files keep today's names, and requirement 6 holds without a special
   case.
8. **An empty part is refused, not rendered.** mdBook accepts a part title
   with nothing under it and draws a bare label. Bower refuses it with
   `LoadError::EmptyPart`, naming the title, in the house manner: "annotation
   typos surface at plan time, not push time" (`source.rs:106-107`). A title
   whose only chapter is a repeat link counts as empty, because
   `chapter_links` drops repeats (`loader.rs:117`).
9. **The fingerprint grows only when parts exist.** `site_fingerprint`
   appends one `part` record per mark, after the chapters. With no marks it
   appends nothing, so no published site turns stale on the day this ships.
   EPIC-16 puts this fingerprint in an edition's pin (its decision 1), so a
   renamed part after a cut will read as `Moved`, which is right.
10. **Titles are the author's markdown.** A title is written into the divider
    as it stands. Two parts may share a title. The scratch file name never
    uses it, so there is nothing to slug and nothing to collide.

---

## Domain map

| Domain concept | Code construct | Status |
|---|---|---|
| Chapter order | `BookSource.chapters`, `source.rs:12` | ✅ unchanged |
| Where a part opens | `PartMark`, `BookSource.parts` | 🔴 new |
| Marks that make sense | `BookSource::with_parts`, `PartError` | 🔴 new |
| Reading `SUMMARY.md` | `summary`; today `chapter_links`, `loader.rs:107` | 🟡 learns H1 |
| A heading with no chapters | `LoadError::EmptyPart`, beside `NoChapters` at `loader.rs:28` | 🔴 new |
| Parts in HTML | mdBook's `part-title`, via `publish.rs:926-954` | ✅ works, untested |
| Parts in the epub and PDF | `RenderPlan.parts`, `part_divider`, `write_chapters` at `publish.rs:647` | 🔴 new |
| Change detection | `site_fingerprint`, `publish.rs:854` | 🟡 plus marks |
| The preprocessor's view | `mdbook::collect`, `mdbook.rs:82` | ✅ unchanged |

---

## Design

### Kernel — `bower-core/src/source.rs`

```rust
/// Where a part opens: a title, and the chapter it sits in front of.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PartMark {
    /// The author's markdown, as written after `# ` in `SUMMARY.md`.
    pub title: String,
    /// Path of the first chapter under the title, `src/`-prefixed.
    pub first: String,
}

pub struct BookSource {
    pub chapters: Vec<Chapter>,
    /// Part titles, in reading order. Empty for a book without parts.
    pub parts: Vec<PartMark>,
    pub library: BTreeMap<String, String>,
    pub assets: BTreeMap<String, Vec<u8>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PartError {
    /// `first` names no chapter in the book.
    UnknownChapter { title: String, first: String },
    /// A mark opens before the mark ahead of it.
    OutOfOrder { title: String },
    /// Two marks open at one chapter, so the first part is empty.
    Shared { first: String, titles: (String, String) },
    /// A title of nothing but whitespace.
    Untitled { first: String },
}

impl BookSource {
    /// The same book with its parts, checked against its chapters.
    pub fn with_parts(self, parts: Vec<PartMark>) -> Result<Self, PartError>;

    /// The mark that opens at this chapter, if one does.
    #[must_use]
    pub fn part_at(&self, chapter: &str) -> Option<&PartMark>;
}
```

`parts` is public, as every field on `BookSource` is, and `with_parts` is
the checked way in. The loader and the fixtures use it. `part_at` is the one
question the renderers ask. No dependency is added, so `make purity`
(`Makefile:49`) still passes.

### Testkit — `bower-testkit`

`arb_parts(&BookSource)` draws a valid set of marks over a generated book.
One property: for any `arb_book()` (`generators.rs:54`) and any `arb_parts`
over it, `plan` and the lock text equal those of the same book without parts.
`hello_playbook()` (`fixtures.rs:96`) gains `.with_parts(…)` in Phase 4, in
the same change that edits the sample's `SUMMARY.md`, because
`loader__matches_the_testkit_fixture_exactly` (`loader.rs:191`) compares the
two with `==` and will fail until both move.

### Loader — `bower/src/loader.rs`

```rust
/// What `SUMMARY.md` says: the chapters, and where each part opens.
struct Summary {
    links: Vec<String>,
    /// A title, and the link that followed it. `None` is an empty part.
    parts: Vec<(String, Option<String>)>,
}

fn summary(text: &str) -> Summary;
```

`summary` keeps `chapter_links`' rules and its "not a full markdown parser"
stance, and adds one. A line that starts with `# ` is a heading. The first
heading is the summary's own title when no link and no other heading comes
before it; every later one is a part title, pending until the next new link
claims it. A title still pending at the next title, or at the end of the
file, is an empty part. `chapter_links` stays as `summary(text).links`, so
its two tests (`loader.rs:148`, `:155`) stand as written. `load` maps each
claimed title to `PartMark { title, first: format!("src/{link}") }` and ends
with `from_chapters(chapters).with_parts(marks)`.

The title rule copies mdBook's, and Phase 0 pins it against a real build
rather than against memory of mdBook's source.

### Render — `bower/src/publish.rs`

```rust
/// A part title, placed for a renderer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedPart {
    pub title: String,
    /// Path of the `RenderedChapter` this part comes before.
    pub before: String,
}

pub struct RenderPlan {
    // … as today …
    pub chapters: Vec<RenderedChapter>,
    pub parts: Vec<RenderedPart>,
}

/// The markdown handed to pandoc for one part title.
#[must_use]
pub fn part_divider(title: &str) -> String;
```

`render_plan` copies `book.parts` across. `part_divider` returns a raw Typst
page break, the heading, and a second page break (decision 6).
`write_chapters` checks `plan.parts` before each chapter, writes the divider
first when one opens there, and returns the paths in the order written.
`PandocRenderer` and `TypstRenderer` pass that list to pandoc as they do now
(`publish.rs:804`, `:1070`), so neither renderer changes. `FakeRenderer`
(`publish.rs:1201`) records the plan, so tests read `seen()[0].parts` with no
tool installed.

`site_fingerprint` appends, after the chapter loop and only when
`book.parts` is not empty, the word `part`, the title, and `first` for each
mark, newline-separated.

---

## Work Items

### Phase 0 — Pin what is already true

- [ ] **0a.** `links__skip_part_headings` in `loader.rs`: a summary with two
  `# ` part titles yields the same links as one without. Passes today.
- [ ] **0b.** Slow lane, beside `mdbook_build_succeeds_with_the_preprocessor`
  (`bower/tests/preprocessor.rs:333`): build a temporary book whose summary
  has two part titles, and assert `toc.html` holds two `part-title` items and
  a chapter page still carries a `step-` anchor. Passes today.
- [ ] **0c.** In the same test, a summary with no `# Summary` line and a part
  title first. Record what mdBook does with that first H1, and make
  `summary`'s title rule match it. If mdBook disagrees with § Design, the
  design changes, not the test.

### Phase 1 — Kernel and testkit

- [ ] **1a.** `PartMark`, `BookSource.parts`, `PartError` with `Display`,
  `with_parts`, `part_at`. Export from the prelude in `bower-core/src/lib.rs`.
- [ ] **1b.** Unit tests, one per `PartError` variant, plus
  `parts__default_is_empty` and `part_at__finds_only_the_opening_chapter`.
- [ ] **1c.** Testkit: `arb_parts` and `plan__ignores_parts`. Confirm
  `make purity`.

### Phase 2 — Loader

- [ ] **2a.** `summary`, with `chapter_links` rewritten over it.
- [ ] **2b.** `LoadError::EmptyPart { title }` and `LoadError::Parts(PartError)`
  with their `Display` arms, beside `NoChapters` (`loader.rs:28-46`).
- [ ] **2c.** `load` builds marks and calls `with_parts`. The sample book has
  no parts yet, so `loader__matches_the_testkit_fixture_exactly` still
  passes.

### Phase 3 — Render and fingerprint

- [ ] **3a.** `RenderedPart`, `RenderPlan.parts`, and the copy in
  `render_plan`.
- [ ] **3b.** `part_divider`, and `write_chapters` interleaving with one
  counter.
- [ ] **3c.** `site_fingerprint` appends marks. Add the fingerprint tests
  beside `bower/tests/site.rs:174-177`.

### Phase 4 — The sample book

- [ ] **4a.** `books/hello-playbook/src/SUMMARY.md` gains `# Part I: The
  repo` before ch01 and `# Part II: The proof` before ch04 (titles are open
  question 4). `hello_playbook()` gains the matching `with_parts` in the same
  change.
- [ ] **4b.** `make build`, then confirm `bower.lock` has no diff and
  `bower/tests/determinism.rs` passes unedited.
- [ ] **4c.** Slow lanes: extend `publish_epub_produces_a_readable_book`
  (`bower/tests/publish.rs:154`) to read `EPUB/nav.xhtml` and find both
  titles in order, each before its first chapter. `pdf_opens_on_the_cover`
  (`:198`) must still pass.

### Phase 5 — Docs

- [ ] **5a.** README: a parts paragraph with a four-line `SUMMARY.md`.
  `bower-spec.md` § 4 (Ordering, `:270`): parts do not affect step order.
- [ ] **5b.** `BACKLOG.md`: flip the EPIC-17 row. Run `make` (the `docs`
  target catches private doc links that clippy misses).

---

## Test Plan

Kernel: `parts__default_is_empty`, `with_parts__accepts_marks_in_order`,
`with_parts__refuses_an_unknown_chapter`, `with_parts__refuses_marks_out_of_order`,
`with_parts__refuses_two_marks_at_one_chapter`,
`with_parts__refuses_a_blank_title`,
`part_at__finds_only_the_opening_chapter`. Property: `plan__ignores_parts`.

Loader: `links__skip_part_headings`,
`summary__first_heading_is_the_title_not_a_part`,
`summary__a_part_opens_at_the_next_new_link`,
`summary__a_part_before_a_repeat_link_is_empty`,
`summary__two_titles_in_a_row_leave_the_first_empty`,
`summary__a_trailing_title_is_empty`, `summary__a_deeper_heading_is_not_a_part`,
`loader__refuses_an_empty_part_by_title`.

Render: `render_plan__carries_parts_in_order`,
`divider__is_a_heading_between_two_typst_page_breaks`,
`write_chapters__puts_a_divider_before_its_chapter`,
`write_chapters__a_partless_book_writes_todays_names`,
`fingerprint__a_partless_book_is_unchanged` (pinned to today's value for the
sample), `fingerprint__a_renamed_part_changes_it`,
`fingerprint__a_moved_part_changes_it`.

Slow: `mdbook_build_shows_part_titles`, and the `nav.xhtml` assertions in
`publish_epub_produces_a_readable_book`.

Each of these is a change in behaviour that fails a test which passes today
once the code moves: the fixture equality in 4a, the partless names in 3b, the
pinned fingerprint in 3c.

## Key Files

| File | Role |
|---|---|
| `bower-core/src/source.rs` | `PartMark`, `BookSource.parts`, `with_parts`, `PartError` |
| `bower-core/src/lib.rs` | prelude exports |
| `bower-testkit/src/generators.rs` | `arb_parts`, the property |
| `bower-testkit/src/fixtures.rs` | `hello_playbook()` gains its parts |
| `bower/src/loader.rs` | `summary`, `EmptyPart`, marks into `load` |
| `bower/src/publish.rs` | `RenderedPart`, `part_divider`, `write_chapters`, `site_fingerprint` |
| `bower/tests/{preprocessor,publish,site}.rs` | the slow lanes and the fingerprint tests |
| `books/hello-playbook/src/SUMMARY.md` | two part titles |
| `README.md`, `bower-spec.md`, `BACKLOG.md` | docs |

## Reuse (do NOT recreate)

- `chapter_links` and `is_markdown` (`loader.rs:107-133`): the link rules,
  the repeat rule, and the `.MD` tolerance stay as they are.
- `materialize::check_path` (`loader.rs:81`): a mark's `first` is a link that
  already passed it.
- `write_chapters` (`publish.rs:647`): "two copies of this loop would be two
  chances to order a book differently". Dividers go in this loop, not in
  each renderer.
- `FakeRenderer` (`publish.rs:1201`): part placement is asserted on the plan,
  with no pandoc.
- `mdbook::collect` (`mdbook.rs:82`): already skips part items. Leave it.
- mdBook itself, for every pixel of HTML.

## Compatibility

- **Preserves** every partless book byte for byte: scratch file names, epub,
  PDF, and site fingerprint. Preserves every plan, lock, tag, and SHA for
  every book, parts or not. `from_chapters`, `Chapter`, `Location`, and
  `RenderPlan.chapters` keep their shapes.
- **Adds** one public field to `BookSource` and one to `RenderPlan`. Code
  that builds either with a full struct literal must name the new field. In
  this workspace that is `render_plan` alone; the three `BookSource {` hits
  outside `source.rs` (`mdbook.rs:147`, `plan.rs:868`, `block.rs:487`) are
  return types, not literals.
- **Changes** one thing an author can see: a `SUMMARY.md` with a part title
  and no chapter under it now fails to load. Before, it loaded and the HTML
  showed a bare label.
- **Breaks** nothing in `bower-core`'s purity.

## Dependencies

- **Built on:** EPIC-03 (the preprocessor's untyped pass-through), EPIC-06
  and EPIC-07 (`RenderPlan`, `write_chapters`, the two pandoc renderers),
  EPIC-08 (`site_fingerprint`).
- **Coordinates with:** **EPIC-12**: its indices can group by part, and would
  add `part_of`. **EPIC-14**: `bower follow where` could name the part.
  **EPIC-16**: the fingerprint is in the pin, so part edits after a cut are
  `Moved`.
- **Blocks:** nothing.

## Verification

```bash
make build                     # includes `make plan`; bower.lock must not change
git diff --exit-code books/hello-playbook/bower.lock
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
make purity
make docs
make slow                      # the mdbook, epub and PDF lanes (`Makefile:64-69`)
make book epub pdf
grep -c 'class="part-title"' books/hello-playbook/book/toc.html   # 2
unzip -p books/hello-playbook/published/*.epub EPUB/nav.xhtml | grep -c 'Part I'
```

Exit criteria:

1. The sample book's HTML, epub, and PDF each show both parts, each before
   its first chapter.
2. `bower.lock` is unchanged and `bower/tests/determinism.rs` passes
   unedited: no SHA moved.
3. `plan__ignores_parts` passes over generated books.
4. A partless book writes the scratch file names and the fingerprint it
   writes today, asserted against pinned values.
5. A part title with no chapter fails `bower build` with the title in the
   message.
6. `cargo tree -p bower-core -e normal` prints one line.

---

## Open questions — decisions, not code

| # | Question |
|---|---|
| 1 | **A nested table of contents.** The divider makes a part a sibling of its chapters, as mdBook's sidebar does. A true nest was measured and works with `--split-level=2` and chapters pushed down one heading level. It needs a fence-aware `shift_headings` of Bower's own, a template that styles two levels, and a rule for chapters outside any part. Is the nest worth that, or is the label enough? The lean is the label, until a reader asks. |
| 2 | **Two levels.** "Books" that hold parts. mdBook has no second level, so the HTML could not show it without a theme change. Does any book need it, or does a part titled "Book One" do? |
| 3 | **Part tags.** `ch03-end` exists (`replay.rs:168`). Should a part's last chapter also get `part-1-end`? It would be the first tag derived from `SUMMARY.md` text, it changes `expected_tags`, and it needs the where-does-a-part-end rule this EPIC avoids. The lean is no. |
| 4 | **The sample's part titles.** "The repo" and "The proof" are placeholders from the spike. The appendix is a list item, so it lands under Part II. Should it move below a `---` as a suffix chapter, get an `# Appendix` part of its own, or stay? |
| 5 | **The PDF's part page.** A level-1 heading alone on a page, in the template's 17pt (`template.typ:26`). Should `part_divider` emit a styled block instead, or should the template own that through a `#show` rule Bower documents? |
| 6 | **Refusing what mdBook accepts.** Decision 8 makes `bower build` fail on a summary `mdbook build` takes. Is that the right side to err on, or should it be a warning on stderr? |

---

*Drafted 19 September 2026 against `ImperialBower/bower` @ `1c10cf2` ("Added
EPIC-17 Grouping"), replacing that commit's first draft after an evaluation
found it built on a part syntax mdBook does not have and on an HTML renderer
Bower does not own. Spiked the same day on a scratch copy of `hello-playbook`
with mdBook 0.4.52, pandoc 3.11, and Typst: the HTML build, the epub's
`nav.xhtml` with and without a divider, the nested variant, and the raw-Typst
page break. Nothing from the spike is in the repository.*
