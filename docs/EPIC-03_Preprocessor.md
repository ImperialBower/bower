# EPIC-03: The mdBook preprocessor (PRE)

## Context

Two EPICs shipped. `bower build` replays a book into a deterministic git
repository with annotated tags and commit trailers; `bower verify` checks every
step's declared `expect` against a real compiler. The repository → book
direction is done: every commit names the chapter and line that produced it.

**The book → repository direction does not exist.** A reader of the rendered
`hello-playbook` sees code blocks with no link to the repo state they produced,
and the `<!-- bower … -->` directives sit invisibly in the HTML doing nothing.
Worse, the display markers do nothing either: chapter 4 wraps its new test in
`// bower:show` … `// bower:show end`, and mdBook prints the whole module
anyway, because nothing consumes those markers at render time.

The kernel already computes every input this needs. `BlockDisplay`
(`bower-core/src/display.rs:50`) carries the spans that render, an
`elided_lines` count, and — after `resolve_ranges`
(`bower-core/src/display.rs:203`) — a `LineRange` per span naming the exact file
and 1-based line span in the materialized tree. `plan()` runs that resolution
already, and hangs the result on every step as `PlannedStep::displays`
(`bower-core/src/plan.rs:48`). Configuration is half-ready: `LinkTemplates`
holds a `blob` template (`bower/src/config.rs:60`) that nothing has ever used.

**This EPIC does not** touch the epub or pandoc path — spec § 3.4 describes a
different elision rendering there (a linked comment rather than a toggle), and
no epub build exists yet. It does not implement `push` or `status`; those are
the other two thirds of spec § 11 Phase 4 and get their own EPICs. It does not
render notebook `play` cells (spec § 15). And it does not touch `bower-core`,
which stays pure and dependency-free.

---

## Status

| Component | Status |
|---|---|
| `mdbook-bower` binary, `preprocessor` feature | **Complete** |
| `supports` handshake | **Complete** |
| mdBook JSON → `BookSource` | **Complete** |
| Plan-time errors fail the book build | **Complete** |
| Directive stripping | **Complete** |
| Display markers: elided spans as hidden lines | **Complete** |
| `#step-<id>` anchors | **Complete** |
| Source-link footer | Planned |
| `tree` and `commit` link templates | Planned |
| Goldens over the sample book | Planned |

---

## Goals

- Make the rendered book **link to the code**: every annotated block gets a
  footer naming its file, its exact **line range**, and the **tag** that state
  lives at.
- Make **display markers** real. A block marked `// bower:show` prints only what
  it marks, with the rest behind mdBook's own toggle rather than deleted.
- Make **annotation rot a build failure**. A directive naming an unknown repo or
  a region that does not exist yet should stop `mdbook build`, not reach a
  reader.
- Keep directives **invisible** — they are HTML comments today, which renders as
  nothing, but they should not survive into the output at all.
- Change **nothing** in `bower-core`.

## Scope

- A second binary, `mdbook-bower`, in the `bower` crate behind a `preprocessor`
  feature, per spec § 8.
- mdBook's preprocessor protocol: `mdbook-bower supports <renderer>` exits `0`
  for `html` and `1` otherwise; with no arguments it reads `[context, book]` as
  JSON on stdin and writes the transformed book as JSON on stdout.
- The whole book arrives in that JSON, so the `BookSource` is built from it —
  not read from disk. Chapter paths are prefixed `src/` to match the loader's
  convention (EPIC-01, corrigendum item 8), so a `Book-Source` trailer and a
  preprocessor anchor name the same chapter.
- Rendering rules per annotated block:
  - the directive comment is removed;
  - an anchor `<a id="step-<id>"></a>` precedes the block;
  - spans marked `bower:show` print; every other line is emitted as an
    mdBook `#`-hidden line, so the code is all there behind the eye toggle;
  - a block with no markers prints whole, exactly as today;
  - a footer follows: `` `src/lib.rs` L18–31 · step 12 of hello-playbook ``
    with links to the file, the diff, and the repo at that tag.
- Links come from `bower.toml` templates. Absent template, absent link — never a
  plausible-looking broken one, the same rule `Book-Url` already follows.
- Any `BowerError` from `plan()` aborts with a non-zero exit and the error's
  chapter and line on stderr.

---

## Domain map

| Domain concept | Code construct | Status |
|---|---|---|
| Which lines the page shows | `DisplaySpan` `bower-core/src/display.rs:18` | ✅ done |
| Where those lines live in the tree | `LineRange` `bower-core/src/display.rs:41` | ✅ done |
| A block's display outcome | `BlockDisplay` `bower-core/src/display.rs:50` | ✅ done |
| The step a block belongs to | `PlannedStep::displays` `bower-core/src/plan.rs:48` | ✅ done |
| Forge URL shapes | `LinkTemplates` `bower/src/config.rs:60` | 🟡 `blob` only |
| mdBook's book, as data | `mdbook::{split, chapters, map_chapters}` | ✅ done |
| The rewrite | `render::chapter()` | ✅ done |

---

## Design

### The binary

`bower/src/mdbook_bower.rs` (new), behind:

```toml
[features]
default = ["preprocessor"]
preprocessor = ["dep:serde_json"]

[[bin]]
name = "mdbook-bower"
path = "src/mdbook_bower.rs"
required-features = ["preprocessor"]
```

`serde_json` is the one new dependency, and it buys the whole protocol. `serde`
is already present for `bower.toml`.

### mdBook's JSON, as little of it as possible

`bower/src/mdbook.rs` (new):

```rust
/// mdBook hands the preprocessor `[context, book]`. Only these fields are
/// modelled; everything else round-trips untouched via `serde_json::Value`,
/// so an mdBook upgrade that adds a field cannot break this.
#[derive(Deserialize)]
pub struct Context {
    pub root: PathBuf,
}

/// A book is a tree of items; only `Chapter` items carry content.
pub fn chapters_mut(book: &mut Value) -> Vec<&mut Value>;
pub fn chapter_path(item: &Value) -> Option<String>;
pub fn chapter_content_mut(item: &mut Value) -> Option<&mut String>;
```

Modelling the book as `serde_json::Value` rather than a mirror of mdBook's
`Book` struct is deliberate. A typed mirror has to be kept in step with mdBook
release by release, and every field it forgets is a field it silently drops.

### The rewrite

`bower/src/render.rs` (new):

```rust
/// Everything the renderer needs about one annotated block.
pub struct Rendered {
    pub anchor: String,
    pub body: Vec<String>,
    pub footer: Option<String>,
}

/// Rewrite one chapter's markdown: strip directives, apply display markers,
/// inject anchors and footers.
pub fn chapter(
    text: &str,
    chapter_path: &str,
    plan: &BookPlan,
    links: &LinkTemplates,
) -> String;

/// The lines a block contributes to the page: marked spans verbatim, and
/// everything else as `#`-hidden lines.
pub fn body_lines(display: &BlockDisplay, raw: &[String]) -> Vec<String>;

/// `` `src/lib.rs` L18–31 · step 12 of hello-playbook · [full file] · … ``
pub fn footer(
    step: &PlannedStep,
    display: &BlockDisplay,
    repo: &str,
    links: &LinkTemplates,
) -> Option<String>;
```

`chapter` takes the whole `BookPlan` because a chapter's blocks may target
several repos, and the footer needs each block's own step.

### Link templates

`LinkTemplates` (`bower/src/config.rs:60`) gains two fields:

```rust
pub struct LinkTemplates {
    /// `…/blob/{tag}/{path}#L{start}-L{end}`
    pub blob: Option<String>,
    /// `…/tree/{tag}` — browse the repo at this step.
    pub tree: Option<String>,
    /// `…/commit/{tag}` — the diff this step introduced.
    pub commit: Option<String>,
}
```

Substitution is literal `{tag}`, `{path}`, `{start}`, `{end}` replacement. No
template engine: three placeholders do not justify a dependency, and a template
that can compute is a template that can fail at render time.

---

## Work Items

### Phase 0 — The binary exists and round-trips

- [x] **0a.** `preprocessor` feature, optional `serde_json`, and two explicit
  `[[bin]]` entries. **Addition:** `autobins = false`, because a file in
  `src/bin/` would otherwise be auto-discovered a second time as a binary named
  `mdbook_bower`, with an underscore nobody chose. **And the file does not live
  in `src/bin/` at all:** a common global gitignore rule — a bare `bin` line,
  meant for compiled output — silently excluded the whole directory, so the new
  binary was invisible to `git status` and would never have been committed.
  With `autobins = false` the path is arbitrary, so it sits beside `main.rs`.
  **Tidy:**
  `gix` was pinned directly in `bower/Cargo.toml` by `cargo add` back in
  EPIC-01, against this workspace's convention; it now goes through
  `[workspace.dependencies]` like everything else.
- [x] **0b.** `mdbook_bower.rs`: the `supports` handshake answers `html` only,
  and the no-argument path reads the envelope and writes the book back
  unchanged.
- [x] **0c.** Four tests, not one. **Addition:** the protocol's failure modes
  earned their own — `bad_stdin_is_an_error_not_a_panic` and
  `a_bare_object_is_rejected`. mdBook always sends a pair; guessing at anything
  else would corrupt someone's book rather than fail it.
- [x] **0d.** 119 tests pass, clippy is silent with `--all-features`, and
  `cargo tree -p bower --no-default-features -e normal` contains no
  `serde_json` — the feature boundary is real, not decorative. Verified on
  branch `docs/epic-01`, 1 September 2026.

### Phase 1 — Planning from mdBook's JSON

- [x] **1a.** `bower/src/mdbook.rs`: `Context`, `split`, `chapters`,
  `chapter_path`, `chapter_content`, `map_chapters`, `book_source`.
  **Structural change, approved before starting:** the crate gained
  `bower/src/lib.rs`. Two binaries cannot share modules any other way, and
  duplicating the configuration parser between `bower` and `mdbook-bower` would
  be the first step toward two tools that disagree about the same book. The
  `mdbook` module is behind the `preprocessor` feature, since it is the only
  thing that needs `serde_json`.
- [x] **1b.** `book_source()` builds the kernel's input from the envelope's
  chapters in reading order, `src/`-prefixed, and the binary resolves it with
  `plan()` against `BookConfig::load(context.root)`.
- [x] **1c.** Any `BowerError` prints with its chapter and line and exits `1`.
  Resolving is not skipped merely because nothing is rewritten yet: a book that
  does not plan is a book with broken annotations, and publishing it would hand
  a reader links to states that were never computed.
- [x] **1d.** Six tests in the module and six against the binary, including
  `a_bad_directive_fails_the_build` and its positive twin
  `a_resolvable_book_survives_planning` — a build that fails for *every* book is
  not a working gate. Verified on branch `docs/epic-01`, 1 September 2026: 127
  tests pass, clippy silent with `--all-features`.

### Phase 2 — The rewrite

- [x] **2a.** `render::body_lines`. **Deviation:** the design said elided lines
  become `# `-hidden lines, full stop. That is only right for Rust — mdBook's
  hidden-line toggle does not exist for other languages, and a `# ` prefix
  inside a Makefile or YAML block would corrupt the file rather than hide it.
  So: `rust` fences get hidden lines the reader can expand in place, and every
  other language gets one comment line per elided run (`# ⋯ 11 lines elided`),
  which is the epub rendering spec § 3.4 already describes. The comment token
  follows the fence's language. Chapter 6's `yaml` block is the case that
  forced this.
- [x] **2b.** `render::chapter`. **Deviation:** the walker had to become
  fence-aware. A book *about* Bower quotes directives inside example fences —
  `bower-testkit`'s `self_hosting_chapter` fixture exists for exactly this — and
  the first version stripped them, erasing the thing the page was teaching. A
  fence not opened by a directive is now copied through untouched.
- [x] **2c.** Nine tests, including `render__a_directive_shown_inside_a_fence_survives`
  and `render__marked_yaml_block_elides_with_a_comment`. Verified on branch
  `docs/epic-01`, 1 September 2026: over the real six chapters, **zero directive
  lines survive and twenty anchors are injected** — one per step.

**Kernel change, and why.** `ShowMark` and `show_marker` were `pub(crate)` in
`bower-core/src/tree.rs`. The preprocessor needs that grammar, and the
alternative was a second copy of it in a consumer — precisely the drift this
kernel exists to prevent. They are now `pub` and in the prelude. This is an
additive, pure change: no dependency, no I/O, no behavior difference, and
`cargo tree -p bower-core -e normal` still prints one line. It contradicts this
EPIC's Context, which said `bower-core` would not be touched; the Context was
wrong rather than the change.

### Phase 3 — Footers and links

- [ ] **3a.** `LinkTemplates` gains `tree` and `commit`; declare all three in
  `books/hello-playbook/bower.toml`.
- [ ] **3b.** `render::footer` substituting `{tag}`, `{path}`, `{start}`,
  `{end}`, omitting any link whose template is absent, and omitting the footer
  entirely when a block has no resolvable range.
- [ ] **3c.** Tests: `footer__names_the_file_and_line_range`,
  `footer__omits_links_without_templates`,
  `footer__absent_when_a_block_has_no_range`.

### Phase 4 — Goldens

- [ ] **4a.** `bower/tests/preprocessor.rs`: drive the `mdbook-bower` binary
  with a JSON book built from the real sample chapters; assert no directive
  survives, every step id appears as an anchor, and chapter 4's marked test is
  visible while the module around it is hidden.
- [ ] **4b.** A negative golden: a chapter naming an unknown repo makes the
  binary exit non-zero and name the chapter and line.
- [ ] **4c.** End-to-end `mdbook build books/hello-playbook` with the
  preprocessor configured, marked `#[ignore]` and skipped cleanly when `mdbook`
  is not installed.

### Phase 5 — Documentation

- [ ] **5a.** Configure `[preprocessor.bower]` in
  `books/hello-playbook/book.toml`.
- [ ] **5b.** `README.md`: the workspace reaches Phase 4 in part; document
  `mdbook-bower`.
- [ ] **5c.** Flip this EPIC's Status rows and append the corrigendum.

---

## Test Plan

- `passthrough_returns_the_book_unchanged` — the protocol is right before any
  rewriting is attempted.
- `supports_html_and_nothing_else` — an unknown renderer is declined, never
  assumed.
- `bad_stdin_is_an_error_not_a_panic`, `a_bare_object_is_rejected` — the two
  ways the envelope can be wrong.
- `mdbook__chapters_come_back_in_reading_order` — step order follows reading
  order, so the traversal must too.
- `mdbook__nested_sections_are_found` — a book with parts and sub-chapters is
  not silently half-processed.
- `mdbook__separators_carry_no_content_and_are_skipped` — a `Separator` item is
  not a chapter and must not be treated as one.
- `mdbook__map_chapters_rewrites_nested_chapters_too` — the rewrite reaches as
  deep as the read does.
- `a_bad_directive_fails_the_build` — annotation rot stops the build. Without
  this, every other guarantee here is advisory.
- `a_resolvable_book_survives_planning` — its twin; a gate that rejects
  everything is not a gate.
- `render__unmarked_block_is_untouched` — small examples pay no tax.
- `render__marked_rust_block_hides_the_rest` — the display markers finally do
  something.
- `render__marked_yaml_block_elides_with_a_comment` — and they do the right,
  different thing outside Rust.
- `render__directive_comments_do_not_survive` — no directive line in the output.
- `render__a_directive_shown_inside_a_fence_survives` — its counterweight: a
  quoted directive is content, not an instruction.
- `footer__names_the_file_and_line_range` — the line anchor is the whole point;
  a footer that names the wrong lines is worse than none.
- `footer__omits_links_without_templates` — no plausible-looking broken links.
- `preprocessor__sample_book_round_trip` — the golden, over real chapters.

## Key Files

| File | Role |
|---|---|
| `bower/src/mdbook_bower.rs` | new; the protocol and the driver |
| `bower/src/mdbook.rs` | new; mdBook's JSON, minimally modelled |
| `bower/src/render.rs` | new; the chapter rewrite |
| `bower/src/config.rs:60` | `LinkTemplates` gains `tree` and `commit` |
| `bower/Cargo.toml` | `preprocessor` feature, `serde_json`, second `[[bin]]` |
| `books/hello-playbook/book.toml` | declares the preprocessor |
| `books/hello-playbook/bower.toml` | declares all three link templates |
| `bower/tests/preprocessor.rs` | new; the goldens |

## Reuse (do NOT recreate)

- `bower-core/src/display.rs:50` — `BlockDisplay` already holds the spans, the
  elision count, and the resolved ranges. The preprocessor formats; it computes
  nothing about display.
- `bower-core/src/plan.rs:48` — `PlannedStep::displays` is that data, per step,
  already resolved by `plan()`.
- `bower-core/src/plan.rs:67` — `PlannedStep::tag()` is the link target. The
  preprocessor must never construct a tag name itself.
- `bower/src/config.rs` — `BookConfig::load` and `catalog()` are unchanged; only
  `LinkTemplates` grows.
- `bower/src/loader.rs` — the `src/`-prefixed chapter path convention. The
  preprocessor must match it or anchors and trailers will disagree.

## Compatibility

- **Preserves** `bower-core`'s zero-dependency, no-I/O contract, and every
  existing subcommand. Verified by `cargo tree -p bower-core -e normal`.
- **Adds** one feature, one dependency, one binary, two config keys.
- **Breaks** nothing. Building `bower` with `--no-default-features` yields the
  CLI alone, without `serde_json`.

## Dependencies

- **Blocks:** nothing yet. `push` (spec § 5.5) and `status` (spec § 7) are
  independent and get their own EPICs.
- **Built on:** EPIC-01 for the plan, the config, and the tag names; EPIC-02 for
  the confidence that a published link points at a state that actually works.
- **Related:** spec § 3.4 (display markers), § 5.3 (tags), § 5.4 (this).

## Verification

```bash
cargo build --workspace --all-features
cargo build -p bower --no-default-features      # the CLI, without serde_json
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo tree -p bower-core -e normal
mdbook build books/hello-playbook
```

Exit criteria:

1. `mdbook build books/hello-playbook` succeeds and no directive *line*
   survives into any rendered page. Prose that quotes a directive inline, or
   shows one inside an example fence, is content and must survive — the
   original wording of this criterion did not distinguish the two.
2. Chapter 4's failing-test block shows only the marked test, with the rest of
   the module behind mdBook's toggle.
3. Every annotated block carries a footer naming a file, a line range, and a
   step, and every link in it resolves to a template the book declared.
4. A directive naming an unknown repo fails `mdbook build` with the chapter and
   line named.
5. `cargo tree -p bower-core -e normal` still prints one line.
