# EPIC-06: `bower publish` — one render, several targets (PUB2)

## Context

Five EPICs shipped. A book becomes a deterministic git repository, every
declared `expect` is checked by a real compiler, drift is reported, publishing
is guarded, and `mdbook-bower` rewrites chapters at render time.

**But there is exactly one output: mdBook HTML, and only as a side effect of
running `mdbook` yourself.** There is no `bower publish`, no epub, no PDF. The
word "epub" appears twice in the codebase, both times in a comment explaining
why something was *not* done that way.

The render logic already exists and already knows about more than one target.
`render::body_lines` (`bower/src/render.rs:138`) elides differently by language
because mdBook's hidden-line toggle only works for Rust: a `rust` fence gets
`# `-prefixed lines the reader expands in place, and every other language gets
`# ⋯ 11 lines elided`. That second form *is* the epub rendering spec § 3.4
describes — built, working, and rendered into nothing, because no epub build
exists. `docs/TECHNICAL_DEBT.md` has carried it as "designed but unbuilt" since
EPIC-03.

Everything else the render needs is present: `BlockDisplay`
(`bower-core/src/display.rs:50`) carries the spans, the elision count, and
resolved `LineRange`s; `PlannedStep::displays` (`bower-core/src/plan.rs:48`)
hangs that off every step; `render::footer` (`bower/src/render.rs:228`) already
emits absolute forge URLs, which is what a reader on a Kindle needs.

**Scope, and why it is not all of M1.** Spec § 13 M1 names four targets —
`html`, `epub`, `pdf`, `ipynb`. That is not one kata. This EPIC does **html and
epub**, the two whose renderers are already installed and whose output shapes
are settled. **PDF is deferred**: it needs a typography decision (Typst is not
installed here; `xelatex` is) that deserves its own EPIC rather than a coin
toss. **`ipynb` is deferred**: it depends on play cells (spec § 15), which
nothing renders yet.

**This EPIC does not** implement editions (M2), author tooling (M3), the imprint
(M4), or a cover image. It does not touch `bower-core`, which stays pure and
dependency-free — a sixth EPIC in a row.

---

## Status

| Component | Status |
|---|---|
| `Target` — html and epub | **Complete** |
| Per-target elision in `body_lines` | **Complete** |
| `RenderPlan` — the pure fold | Planned |
| `Renderer` trait + `FakeRenderer` | Planned |
| Book metadata from `book.toml` | Planned |
| `PandocRenderer` — the epub | Planned |
| `MdBookRenderer` — the html | Planned |
| `bower publish --target` CLI | Planned |
| Goldens against the fake renderer | Planned |

---

## Goals

- **One render, several targets.** The same display-marker engine feeds every
  output, so html and epub cannot disagree about what a page shows.
- Keep the fold **pure**: book plus plan in, a `RenderPlan` out, no I/O. The
  renderers are replaceable edges, exactly as `TreeState` relates to git.
- Make the **elision honest per target**. A toggle exists in mdBook HTML and
  nowhere else; epub gets a comment naming how much was left out *and a link to
  the full file*, which is what spec § 3.4 asks for and what the current
  implementation omits.
- Ship an **epub a reader can actually open**, from one command.
- Change **nothing** in `bower-core`.

## Scope

### Targets

| Target | Renderer | Elision |
|---|---|---|
| `html` | `mdbook build`, via the existing `mdbook-bower` preprocessor | `rust` fences use mdBook's hidden lines; other languages get a comment |
| `epub` | `pandoc`, over rendered markdown this crate writes | **every** language gets a comment, because there is no toggle anywhere in an epub |

The difference is one rule, and it is the reason a target has to be threaded
through the render rather than inferred from the fence.

### Rules

- `bower publish --target html|epub [--repo R] [-o DIR]`. `--target` is
  required; there is no default, because silently producing the wrong artifact
  is worse than asking.
- The elision comment carries a link when the book declares a `blob` template:
  `// ⋯ 14 lines elided — full file: <url>`. Spec § 3.4 specifies that link, and
  without a target to render into it was never added.
- Book metadata — title, authors, language — comes from **`book.toml`**, which
  is where mdBook already keeps it. Not a second copy in `bower.toml`.
- A missing renderer binary is named, with its install command, before anything
  is written.
- The output directory is emptied first, the way `build` does: a publish is a
  build artifact, not an accumulation.

---

## Domain map

| Domain concept | Code construct | Status |
|---|---|---|
| Which lines a page shows | `BlockDisplay` `bower-core/src/display.rs:50` | ✅ done |
| How much was left out | `elided_lines` `bower-core/src/display.rs:55` | ✅ done |
| Where the full file lives | `LineRange` + `footer` `bower/src/render.rs:228` | ✅ done |
| A chapter, rewritten | `render::chapter` `bower/src/render.rs:21` | ✅ takes a `Target` |
| Which output is wanted | `Target` | ✅ done |
| The whole book, rendered | `RenderPlan` | ❌ absent |
| A thing that writes artifacts | `Renderer` | ❌ absent |
| The book's identity | `BookMeta` from `book.toml` | ❌ absent |

---

## Design

### `Target` — the one thing that varies

`bower/src/publish.rs` (new):

```rust
/// What is being produced. Threaded through the render because the elision
/// rule differs, and only because of that.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Target {
    /// mdBook HTML. Rust fences keep the eye-toggle.
    Html,
    /// pandoc epub. No toggle exists anywhere, so every elision is a comment.
    Epub,
}

impl Target {
    /// Whether this target can hide code behind a toggle the reader expands.
    /// True only for `Html`, and only Rust fences use it even then.
    #[must_use]
    pub fn has_hidden_lines(self) -> bool {
        matches!(self, Self::Html)
    }
}
```

A boolean would do the job today. An enum is right anyway: `Pdf` and `Ipynb` are
named in spec § 13 and will land here, and a `bool` called `is_html` is a
variable that stops being true.

### `render::body_lines` grows a target

`bower/src/render.rs:138` currently decides by fence language alone:

```rust
// today
let rustish = info.split([',', ' ']).next() == Some("rust");
// after
let hidden_lines = target.has_hidden_lines() && info.split([',', ' ']).next() == Some("rust");
```

and the elision comment gains the link spec § 3.4 asks for:

```rust
// `// ⋯ 14 lines elided — full file: https://…/blob/step-011-…/src/lib.rs`
fn elision(comment: &str, n: usize, full_file: Option<&str>) -> String;
```

`full_file` is `None` when the book declares no `blob` template — the same rule
`Book-Url` and the footer already follow: no link at all beats a plausible
broken one.

### `RenderPlan` — the pure fold

```rust
/// A whole book, rendered for one target. Pure: no I/O, no renderer, nothing
/// written. This is to publishing what `TreeState` is to git.
pub struct RenderPlan {
    pub target: Target,
    pub meta: BookMeta,
    pub chapters: Vec<RenderedChapter>,
}

pub struct RenderedChapter {
    /// Book-relative, `src/`-prefixed, as everywhere else in this crate.
    pub path: String,
    pub title: String,
    pub markdown: String,
}

/// Title, authors, language — from `book.toml`, where mdBook already keeps it.
pub struct BookMeta {
    pub title: String,
    pub authors: Vec<String>,
    pub language: String,
}

/// Fold a loaded book and its plan into a render plan.
pub fn render_plan(
    book: &BookSource,
    plan: &BookPlan,
    meta: BookMeta,
    target: Target,
    links: &BTreeMap<String, LinkTemplates>,
) -> RenderPlan;
```

Splitting the fold from the renderers is the whole architectural claim of spec
§ 13, and it is what makes the goldens possible: a test asserts on markdown,
without pandoc installed and without reading a binary epub.

### `Renderer` — the edge

```rust
/// Something that turns a `RenderPlan` into files.
///
/// Shaped like `Forge` (`bower/src/forge.rs:57`) and for the same reason: the
/// decisions are tested against a fake, and only the shelling-out is not.
pub trait Renderer {
    /// The binaries this renderer needs, checked before anything is written.
    fn preflight(&self) -> Result<(), PublishError>;
    fn render(&self, plan: &RenderPlan, out: &Path) -> Result<Artifact, PublishError>;
}

pub struct Artifact {
    pub path: PathBuf,
    pub bytes: u64,
}
```

`PandocRenderer` writes each chapter's markdown to a scratch directory in
`SUMMARY.md` order and invokes `pandoc` with the metadata and an output path.
`MdBookRenderer` shells to `mdbook build`, which already routes through
`mdbook-bower`.

**A wrinkle worth stating.** `MdBookRenderer` does not consume the `RenderPlan`
in the way `PandocRenderer` does: mdBook re-runs the preprocessor itself, so the
render happens inside `mdbook-bower` rather than in `bower publish`. The plan is
still built and still checked — a book that will not render is caught before
`mdbook` is invoked — but the html path renders twice. Unifying that properly
means teaching `mdbook-bower` to read a prepared plan, which is deliberately not
in this EPIC. `FakeRenderer` records what it was handed, so the asymmetry is
visible in tests rather than hidden.

---

## Work Items

### Phase 0 — Target, and the elision it changes

- [x] **0a.** `bower/src/publish.rs`: `Target`, `has_hidden_lines`, `Display`,
  and `FromStr`. **Deviation:** `PublishError` was not written — it has no
  caller until Phase 2, and a library's public error type with nothing raising
  it is the dead-code shape EPIC-02 taught this project to avoid. **Addition:**
  `FromStr` names the built targets in its own error, so `--target pdf` says
  which two exist rather than only that it is wrong.
- [x] **0b.** `Target` threaded through `render::body_lines` and
  `render::chapter`; `mdbook_bower.rs` passes `Target::Html`.
- [x] **0c.** `full_file_url()` plus the `— full file: <url>` suffix.
  **Deviation:** this is a new function, not another `subst` call. The `blob`
  template addresses a *line range*; a reader following an elision wants the
  file, so the template's `#L{start}-L{end}` fragment is dropped. One turns into
  the other only by truncation, which deserved a named function and a comment.
- [x] **0d.** Four tests as designed.
- [x] **0e.** Verified, with one honest correction. The item said the HTML must
  be unchanged "byte for byte", which item 0c makes false on purpose: chapter 6's
  YAML elision now carries the link. What must not change is the *behaviour*, and
  it did not — chapter 4 still renders 31 `class="boring"` hidden lines and zero
  elisions, and the book still carries 20 anchors. Ten preprocessor goldens pass.
  Verified on `main`, 2 September 2026: 219 tests, `make ayce` green.

### Phase 1 — The pure fold

- [ ] **1a.** `BookMeta` and a reader for `book.toml`'s `[book]` table.
- [ ] **1b.** `RenderedChapter`, `RenderPlan`, `render_plan()`.
- [ ] **1c.** Tests: `render_plan__covers_every_chapter_in_reading_order`,
  `render_plan__epub_and_html_differ_only_in_elision`,
  `meta__comes_from_book_toml`, `meta__missing_book_toml_is_an_error`.

### Phase 2 — The renderers

- [ ] **2a.** `Renderer`, `Artifact`, and `FakeRenderer` — public, recording
  every call, for the same reason `FakeForge` is (EPIC-05, corrigendum).
- [ ] **2b.** `PandocRenderer`: preflight for `pandoc`, write chapters to a
  scratch directory in order, invoke pandoc with title, authors, and language.
- [ ] **2c.** `MdBookRenderer`: preflight for `mdbook` **and** `mdbook-bower`
  on `PATH`, then `mdbook build`.
- [ ] **2d.** Tests: `renderer__preflight_names_the_install_command`,
  `fake__records_the_plan_it_was_given`.

### Phase 3 — The command

- [ ] **3a.** `bower publish --target html|epub [--repo R] [-o DIR]` in
  `bower/src/main.rs`, reusing `resolve()`.
- [ ] **3b.** Empty the output directory first, and report the artifact's path
  and size.
- [ ] **3c.** Exit non-zero when the book does not resolve, a renderer is
  missing, or the renderer fails.

### Phase 4 — Goldens and documentation

- [ ] **4a.** `bower/tests/publish.rs`: build a `RenderPlan` for both targets
  over the real sample chapters and assert the epub plan elides chapter 4's Rust
  block while the html plan keeps its hidden lines. **No renderer needed** —
  this is what the pure fold buys.
- [ ] **4b.** An `#[ignore]`d end-to-end test: `bower publish --target epub`
  produces a file `pandoc` accepts, skipped cleanly when pandoc is absent.
- [ ] **4c.** `make epub` and `make html` targets calling `bower publish`,
  replacing the note in `README.md` that says no epub exists.
- [ ] **4d.** Close the "epub elision rendering is designed but unbuilt" item in
  `docs/TECHNICAL_DEBT.md`; flip Status rows; append the corrigendum.

---

## Test Plan

- `target__only_html_hides_lines`, `target__round_trips_through_its_name`,
  `target__an_unbuilt_target_says_which_are_built` — the type itself.
- `target__epub_elides_rust_too` — the one behavioural difference between the
  targets, asserted directly.
- `target__html_keeps_the_rust_toggle` — its counterweight; the HTML must not
  regress while epub is being added.
- `elision__names_the_full_file_when_a_template_exists` — spec § 3.4's link,
  finally rendered.
- `elision__omits_the_link_without_one` — no plausible broken links, the rule
  `Book-Url` and `footer` already follow.
- `render_plan__covers_every_chapter_in_reading_order` — a chapter silently
  dropped from an epub is a chapter nobody notices is missing.
- `render_plan__epub_and_html_differ_only_in_elision` — the claim that there is
  one engine, made testable.
- `meta__missing_book_toml_is_an_error` — an epub with no title is not worth
  producing.
- `renderer__preflight_names_the_install_command` — the failure a reader will
  actually hit.
- `publish__epub_plan_elides_chapter_four` — the golden, over real chapters,
  with no renderer installed.

## Key Files

| File | Role |
|---|---|
| `bower/src/publish.rs` | new; `Target`, `RenderPlan`, `Renderer`, `FakeRenderer` |
| `bower/src/render.rs:21,138` | `chapter` and `body_lines` gain a `Target` |
| `bower/src/mdbook_bower.rs` | passes `Target::Html` |
| `bower/src/main.rs` | the `publish` subcommand |
| `books/hello-playbook/book.toml` | already carries the metadata; now read |
| `bower/tests/publish.rs` | new; the goldens |
| `Makefile` | `epub` and `html` targets |

## Reuse (do NOT recreate)

- `bower/src/render.rs:138` — `body_lines` already elides per language. It gains
  a target; it is not rewritten.
- `bower/src/render.rs:228` — `footer` already emits absolute forge URLs, which
  is exactly what an offline reader needs. Every target uses it unchanged.
- `bower-core/src/display.rs:55` — `elided_lines` is already counted. The epub
  comment prints the kernel's number, not its own.
- `bower/src/forge.rs:57,96` — `Forge` and `FakeForge` are the shape `Renderer`
  and `FakeRenderer` copy. Same reasoning, same testing strategy.
- `bower/src/loader.rs` — the book is already loaded in `SUMMARY.md` order.
  Publishing does not re-read it.

## Compatibility

- **Preserves** every existing command, the HTML output byte for byte
  (work item 0e), and `bower-core`'s purity.
- **Adds** one subcommand, one module, and two runtime binary dependencies
  (`pandoc`, and `mdbook` as before), each checked with a named install command.
- **Breaks** nothing.

## Dependencies

- **Blocks:** editions (spec § 13 M2) — an edition is a pinned triple of book
  commit, lock, and repo tags, and there is no point pinning what cannot be
  published.
- **Built on:** EPIC-03, which built the display-marker engine this folds over.
- **Related:** spec § 3.4 (per-target elision), § 13 M1 (the ladder rung this
  is), § 15 (the `ipynb` target, deferred).
- **Deferred siblings:** `--target pdf` needs a typography decision — Typst is
  not installed here, `xelatex` is — and `--target ipynb` needs play cells,
  which nothing renders. Each earns its own EPIC.

## Verification

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo tree -p bower-core -e normal
cargo run -p bower -- --book books/hello-playbook publish --target epub -o /tmp/pub
cargo run -p bower -- --book books/hello-playbook publish --target html -o /tmp/pub
cargo test -p bower --test publish -- --ignored     # needs pandoc
```

Exit criteria:

1. `publish --target epub` produces a file `pandoc` accepts and a reader can
   open, with the book's title and authors from `book.toml`.
2. Chapter 4's Rust block is elided with a comment in the epub and keeps its
   hidden-line toggle in the HTML — asserted on the `RenderPlan`, with no
   renderer installed.
3. Every elision comment in the epub names the full file's URL.
4. The HTML output's *behaviour* is unchanged from before this EPIC — the
   toggle, the anchors, the footers. The elision comment itself gains a link, by
   design (work item 0c).
5. A missing `pandoc` fails with its install command named, before anything is
   written.
6. `cargo tree -p bower-core -e normal` still prints one line.
