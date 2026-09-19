# EPIC-17: Chapter Grouping — Parts and Books (CHGRP)

## Summary

- **Builds:** chapter grouping: organize chapters into **parts** (or "books"), with a `Part` type and a hierarchical structure in `SUMMARY.md` — e.g., `Part 1 (chapters 1–3)`, `Part 2 (chapters 4–6)`.
- **Why:** books are currently a flat list of chapters; readers and authors need to group related chapters into larger narrative units for clarity and navigation.
- **Shape:** a new `Part { title: String, chapters: Vec<Chapter> }` struct, replacing the flat `Vec<Chapter>` in `BookSource`, and a `SUMMARY.md` syntax that maps to parts (mdBook's `- Part Title` heading pattern).
- **Proves it:** parsing a SUMMARY.md with part markers extracts parts correctly, and a part appears in HTML/EPUB/PDF output with its chapters nested inside, verified by a rendered test book.
- **Status:** Planned, filed 19 September 2026, nothing started.

---

## Context

Currently, `BookSource` (`bower-core/src/source.rs:10-27`) holds a flat `Vec<Chapter>`, and `SUMMARY.md` lists chapters in reading order with no grouping semantics. The kernel preserves order (`Chapter` as a tuple of `path` and `text`, with no grouping field), and all downstream rendering — HTML via `mdbook-bower`, EPUB, PDF — treats chapters as independent units. The book's narrative structure is implicit in chapter ordering and file-naming conventions, not explicit in the type system.

The feature does **not** change:
- How chapters are verified or stepped through (steps remain chapter-relative via `Location`)
- How `bower verify` or `bower follow` work (ordering and iteration patterns adapt but semantics unchanged)
- Public APIs for external tool authors (the Feature-Gated design below preserves backward compat)

---

## Status

| Component | Status |
|---|---|
| `Part` type definition | Planned |
| `SUMMARY.md` parsing (part markers) | Planned |
| `BookSource` refactor to hold `Vec<Part>` | Planned |
| Kernel iteration: adapt chapter lookups for part structure | Planned |
| Rendering: parts in HTML output | Planned |
| Rendering: parts in EPUB | Planned |
| Rendering: parts in PDF | Planned |
| `mdbook-bower` preprocessor adaptation | Planned |

---

## Goals

- **Model** chapters as grouped within parts, making hierarchy explicit in the type system.
- **Parse** SUMMARY.md with mdBook's part heading syntax (`- Part Title` or equivalent) to extract part structure from author text.
- **Preserve** all existing step/verification/navigation semantics; a part is a **logical grouping only**, not a new domain entity with rules of its own.
- **Render** parts visibly in all output formats (HTML, EPUB, PDF), with parts as collapsible/expandable sections or visually distinct blocks.
- **Maintain** API compatibility; users who write tools against `bower-core` should be able to opt into part-aware behavior.

---

## Scope

- A `Part` must have a title and a non-empty list of chapters.
- Part structure comes **only** from `SUMMARY.md`, never from inferred file layout (no automatic grouping by directory).
- Parts must preserve reading order: chapters are iterated in part order, then by chapter order within part.
- Steps continue to reference chapters by path; part membership is **not** part of a step's identity (i.e., `Location` stays `Location { chapter, line }`).
- EPUB and PDF rendering must show part boundaries (e.g., a page break and a part title).
- Part titles are not subject to Bower's own markdown processing (they are author-provided strings, not parsed for directives).

---

## Domain map

| Domain concept | Code construct | Status |
|---|---|---|
| A named group of chapters | `Part { title: String, chapters: Vec<Chapter> }` | 🟡 to be added |
| Hierarchical book structure | `BookSource { parts: Vec<Part>, library, assets }` | 🟡 to be refactored |
| Part syntax in author text | SUMMARY.md part markers | 🟡 to be parsed |
| Chapter lookup by path | Updated `BookSource::chapter_by_path(…)` | 🟡 to adapt |
| Iteration over all chapters | Flatten `parts` in iterator | 🟡 to adapt |
| Rendering with part headers | `render.rs`, `publish.rs` | 🟡 to adapt |

---

## Design

### `Part` type

`bower-core/src/source.rs` (added):

```rust
/// A named group of chapters.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Part {
    /// The part's title (e.g., "Part 1: Foundations").
    pub title: String,
    /// Chapters in reading order within this part.
    pub chapters: Vec<Chapter>,
}

impl Part {
    /// A part from a title and chapters.
    #[must_use]
    pub fn new(title: &str, chapters: Vec<Chapter>) -> Self {
        Self {
            title: title.to_string(),
            chapters,
        }
    }
}
```

**Rationale:** A part is a container of chapters with a title. It mirrors the current `Chapter` design (simple fields, no computed state), and its sole responsibility is grouping. This keeps the change localized and testable.

### `BookSource` refactor

`bower-core/src/source.rs` (refactored):

```rust
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BookSource {
    /// Chapters organized into parts (reading order).
    pub parts: Vec<Part>,
    /// The block library (§ 14.3): path → file text, for `include=` keys.
    pub library: BTreeMap<String, String>,
    /// Binary assets for `op="copy"`: book-relative path → bytes.
    pub assets: BTreeMap<String, Vec<u8>>,
}

impl BookSource {
    /// A book from chapters alone — common fixture shape (all chapters in one implicit part).
    #[must_use]
    pub fn from_chapters(chapters: Vec<Chapter>) -> Self {
        Self {
            parts: vec![Part::new("", chapters)],  // unnamed part for backward compat
            ..Self::default()
        }
    }

    /// A book from explicit parts.
    #[must_use]
    pub fn from_parts(parts: Vec<Part>) -> Self {
        Self {
            parts,
            ..Self::default()
        }
    }

    /// All chapters in reading order (flattened from parts).
    #[must_use]
    pub fn all_chapters(&self) -> Vec<&Chapter> {
        self.parts
            .iter()
            .flat_map(|p| p.chapters.iter())
            .collect()
    }

    /// Find a chapter by path.
    #[must_use]
    pub fn chapter_by_path(&self, path: &str) -> Option<&Chapter> {
        self.all_chapters()
            .into_iter()
            .find(|ch| ch.path == path)
    }
}
```

**Rationale:** `from_chapters` maintains the existing API: tests and fixture code that don't care about parts will work unchanged. `all_chapters()` and `chapter_by_path()` flatten the structure for callers that iterate or search; internal kernel code that cares only about chapter order remains unaffected. An unnamed first part preserves test semantics.

### `SUMMARY.md` parsing

`bower/src/book_parser.rs` (new or refactored parsing logic):

```rust
/// Parse SUMMARY.md into parts and chapters.
/// Part syntax: a markdown heading `- Part Title` followed by indented list items for chapters.
/// Example:
///   - Part 1: Foundations
///     - [Intro](ch01-intro.md)
///     - [Basics](ch02-basics.md)
///   - Part 2: Advanced
///     - [Patterns](ch03-patterns.md)
pub fn parse_summary(text: &str) -> Result<Vec<Part>, ParseError> {
    // Pseudocode:
    // 1. Scan lines; detect part markers (`- Part Title` at indent 0).
    // 2. For each part, collect indented chapters (`- [Title](path.md)`).
    // 3. Return Vec<Part> in order.
}
```

**Rationale:** mdBook's SUMMARY.md syntax already supports part headings (a top-level `- Part Title` is treated as a section header). Reusing this syntax lowers author burden and aligns with mdBook conventions. If parsing gets complex, split it into a dedicated module after Phase 1.

---

## Work Items

### Phase 0 — Prerequisites & feature gating

- [ ] **0a.** Add `Part` type to `bower-core/src/source.rs` with unit tests (new, empty, equality).
- [ ] **0b.** Add `from_parts` constructor to `BookSource`; confirm existing tests still pass.
- [ ] **0c.** Confirm `cargo check --features=<default>` is green; no breaking changes yet (new types only).

### Phase 1 — `BookSource` refactor

- [ ] **1a.** Refactor `BookSource::chapters` to `parts: Vec<Part>`. Add `all_chapters()` and `chapter_by_path()`.
- [ ] **1b.** Update `from_chapters()` to wrap chapters in a single unnamed part (for backward compat in tests).
- [ ] **1c.** Add unit tests: `all_chapters()` flattens parts, `chapter_by_path()` finds across parts, order preserved.
- [ ] **1d.** Run `make docs` (to catch private doc link breaks); confirm all existing tests still pass.

### Phase 2 — Kernel iteration & chapter lookups

- [ ] **2a.** Update `bower-core/src/lib.rs` (kernel step extraction): replace `for ch in &book.chapters` with `for ch in book.all_chapters()`.
- [ ] **2b.** Update `Location::chapter` semantics: still refers to chapter path (no part prefix), but verify lookups work across parts.
- [ ] **2c.** Add integration tests: a step in a chapter inside a part is found and rendered correctly.
- [ ] **2d.** Run full test suite; confirm `cargo test --all-features` passes.

### Phase 3 — `SUMMARY.md` parsing

- [ ] **3a.** Implement part marker detection in `SUMMARY.md` parsing (a new function or adapter in the existing parser). Test with a synthetic SUMMARY.md (two parts, three chapters in each).
- [ ] **3b.** Update the CLI or book loader to parse parts and construct `BookSource::from_parts()` when parts are found; fall back to `from_chapters()` if none.
- [ ] **3c.** Unit tests: parse a multi-part SUMMARY.md, verify part count, chapter order, and names.
- [ ] **3d.** Confirm `cargo test --all-features` passes.

### Phase 4 — HTML rendering with parts

- [ ] **4a.** Update `bower/src/render.rs` to iterate parts and render a part header before chapters (e.g., `<h1 class="part">Part 1: Foundations</h1>`).
- [ ] **4b.** Update `mdbook-bower` preprocessor to inject part headers into the mdBook `book.json` structure so `mdbook build` sees parts.
- [ ] **4c.** Test: run `make book` on the sample book with parts; verify HTML shows part titles and chapter nesting.

### Phase 5 — EPUB rendering with parts

- [ ] **5a.** Update EPUB rendering (`bower/src/publish.rs`, EPUB builder) to emit part metadata (NCX landmarks or a dedicated part container in the OPF).
- [ ] **5b.** Test: generate an EPUB from the sample book and verify part structure appears (use an EPUB reader or unzip and inspect `content.opf` and `toc.ncx`).

### Phase 6 — PDF rendering with parts

- [ ] **6a.** Update PDF rendering (via Typst, `bower/src/publish.rs`) to emit a part title page or heading before each part's chapters.
- [ ] **6b.** Test: generate a PDF and verify parts appear as visually distinct blocks (page breaks, headings).

### Phase 7 — Documentation & backlog update

- [ ] **7a.** Update `README.md` with a SUMMARY.md part example.
- [ ] **7b.** Close EPIC-17; reconcile BACKLOG.md against shipped work.

---

## Test Plan

- **`part__new_holds_title_and_chapters`** — a part's title and chapters are stored and returned.
- **`book_source__from_chapters_wraps_in_unnamed_part`** — backward compat: `from_chapters()` still works and chapters are flattened correctly.
- **`book_source__all_chapters_flattens_parts`** — `all_chapters()` returns a flat vec in reading order.
- **`book_source__chapter_by_path_finds_across_parts`** — chapter lookup works across part boundaries.
- **`summary_parse__extracts_parts_from_markdown`** — a multi-part SUMMARY.md is parsed into correct part count and chapter order.
- **`render__html_shows_part_headers`** — HTML output includes `<h1 class="part">` before each part's chapters.
- **`epub__part_metadata_in_opf`** — EPUB's `content.opf` reflects part structure.
- **`pdf__part_titles_rendered`** — PDF shows part headings.

---

## Key Files

| File | Role |
|---|---|
| `bower-core/src/source.rs` | `Part` type definition, `BookSource` refactor |
| `bower/src/book_parser.rs` | SUMMARY.md parsing (new or refactored) |
| `bower-core/src/lib.rs` | Step extraction, chapter iteration |
| `bower/src/render.rs` | HTML part rendering |
| `bower/src/publish.rs` | EPUB, PDF part metadata and rendering |
| `bower/src/main.rs` (preprocessor) | `mdbook-bower` part injection |

---

## Reuse (do NOT recreate)

- `bower-core/src/source.rs:Chapter` — the existing chapter struct; reuse as-is.
- `mdBook`'s own part syntax in SUMMARY.md — follow their convention.
- Existing iteration patterns in `bower-core/src/lib.rs` — refactor to use `all_chapters()` rather than rewriting the whole pass.

---

## Compatibility

- **Preserves:** `Chapter` type, `Location` identity (still chapter path, not part-qualified). Tests and fixtures using `from_chapters()` work unchanged.
- **Adds:** `Part` type, `from_parts()` constructor, `all_chapters()`, `chapter_by_path()` helpers. `SUMMARY.md` part syntax.
- **Breaks:** nothing if the design above holds. A minimal internal refactor (iteration only).

---

## Dependencies

- **Blocks:** EPIC-17 should not block anything; it is a capability add.
- **Built on:** `bower-core` and the existing mdBook integration (`EPIC-08_Site`).
- **Related:** EPIC-12 (back matter, which may benefit from part-aware indexing), EPIC-14 (`bower follow`, which may show part context).

---

## Verification

```bash
# Phase 0: types added, no breakage
cargo check --all-features
cargo test --all-features

# Phase 1: refactor complete
cargo test --all-features test::part
cargo test --all-features test::book_source

# Phase 3: parsing works
cargo test --all-features test::summary_parse

# Phase 4: HTML rendering
make book
grep -q '<h1 class="part">' target/book/index.html

# Phase 5: EPUB structure
unzip -p target/book.epub content.opf | grep -q '<metadata'  # EPUB present

# Phase 6: PDF visible
[ -f target/book.pdf ]  # PDF exists (visual inspection required)

# Final: all tests pass
cargo test --all-features
cargo clippy --all-features -- -D warnings
```

Exit criteria:

1. All tests pass; `cargo check --all-features` is clean.
2. `make book` produces HTML with part headers in the correct order.
3. An EPUB built from a multi-part book preserves part structure.
4. A PDF shows part boundaries (manual verification).
5. No breaking changes to public APIs (verified by checking `from_chapters()` and step lookups still work).

---

## Implementation corrigendum

*To be filled after shipping.*
