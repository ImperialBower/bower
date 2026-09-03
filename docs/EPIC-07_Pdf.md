# EPIC-07: `--target pdf` — Typst, and a PDF that is the same twice (PDF)

## Context

EPIC-06 shipped `bower publish` with two targets. `Target`
(`bower/src/publish.rs:28`) has two variants, `Renderer`
(`bower/src/publish.rs:230`) has three implementations, and adding a third
target is now a matter of one enum variant, one renderer, and one elision rule.

Spec § 13 M1 names `pdf` alongside them, with "PDF via Typst or LaTeX with
proper code typography". EPIC-06 deferred the choice rather than tossing a coin.
**This EPIC makes it: Typst.**

### The spike that decided it

Run against this machine on 2 September 2026, on the sample book's own rendered
chapters:

| Question | LaTeX (`xelatex`) | Typst |
|---|---|---|
| Installed here | yes, TeX Live 2026 | **yes** — 0.15.1, see below |
| Footprint | **7.5 GB** | **43 MB**, one binary |
| `pandoc --to` support | `pdf`, `latex` | `typst`, in pandoc 3.11 |
| Two runs, identical bytes | **no** | **no** |
| ... with `SOURCE_DATE_EPOCH` | **still no** | **IDENTICAL** |

The last row is the whole decision, and Phase 0 measured both halves of it.

The determinism result is the one that decided it. Two identical `pandoc → pdf
→ xelatex` runs produced different SHAs; pinning `SOURCE_DATE_EPOCH` **and**
`FORCE_SOURCE_DATE` did not fix it. This project's headline invariant is that
the same book in produces byte-identical output — proven for the plan
(`plan_is_deterministic`) and for git SHAs
(`hello_playbook_is_byte_identical_across_runs`). A target that cannot make that
promise is a hole in the argument, not merely a slower path.

Second reason: this repository audits its dependencies and argues about a 30 MB
footprint in `docs/TECHNICAL_DEBT.md`. Seven and a half gigabytes of TeX to
produce a PDF is hard to say in the same breath.

The spike also found a bug already shipping: `⋯` (U+22EF) is absent from Latin
Modern Mono, so the elision rendered with no ellipsis at all. Fixed in EPIC-06's
corrigendum item 7 before this EPIC was written.

**This EPIC does not** remove the LaTeX option — `Renderer` is exactly the seam
that makes `LatexRenderer` an afternoon if Typst's code typography disappoints,
and work item 4c records what would have to be true to justify it. It does not
build `--target ipynb` (spec § 15, needs play cells). It does not touch
`bower-core`, which stays pure and dependency-free — a seventh EPIC running.

---

## Status

| Component | Status |
|---|---|
| Typst installed and spiked | **Complete** |
| `Target::Pdf` + its elision rule | **Complete** |
| `TypstRenderer` | **Complete** |
| Reproducibility, asserted | **Complete** |
| Code typography: the template | **Complete** |
| Goldens + `make pdf` | **Complete** |

---

## Goals

- A **PDF** of the sample book, from `bower publish --target pdf`.
- **Byte-identical across runs**, asserted by a test — the promise every other
  output in this project already keeps.
- **Code that reads like code**: monospace, syntax-highlighted, no mid-token
  line breaks, and line numbers that agree with the `L18–L21` in every footer.
- One more **variant and one more renderer**, with the render engine untouched —
  the claim EPIC-06 made about its own architecture, tested by using it.

## Scope

### The elision rule

| Target | Elision |
|---|---|
| `html` | `rust` fences keep mdBook's toggle; others get a comment |
| `epub` | every language gets a comment |
| **`pdf`** | **every language gets a comment**, same as epub |

A PDF has no toggle, exactly like an epub. So `Target::has_hidden_lines`
(`bower/src/publish.rs:28`) returns `false` for `Pdf` and **no other code
changes** — which is the whole point of having threaded a target through in
EPIC-06 Phase 0.

### Rules

- `bower publish --target pdf [-o DIR]`, producing `<slug>.pdf` beside where
  `--target epub` puts `<slug>.epub` (`publish::slug`, `bower/src/publish.rs:270`).
- Route: `pandoc --to typst` for the markdown conversion, then `typst compile`.
  Not `pandoc --to pdf`, which would reach for LaTeX.
- `typst` missing is named with its install command before anything is written,
  the way `pandoc` and `mdbook` already are.
- Two runs of an unchanged book produce **identical bytes**. If Typst turns out
  to need a pinned timestamp to manage that, pin it from `bower.toml`'s existing
  `epoch` (`bower/src/config.rs:24`) — the same value that already makes commit
  SHAs reproducible. One epoch, every artifact.

---

## Domain map

| Domain concept | Code construct | Status |
|---|---|---|
| Which output is wanted | `Target` `bower/src/publish.rs:28` | ✅ done |
| Whether a toggle exists | `has_hidden_lines` | ✅ done |
| The book, rendered | `RenderPlan` `bower/src/publish.rs:177` | ✅ done |
| A thing that writes artifacts | `Renderer` `bower/src/publish.rs:230` | ✅ done |
| The PDF renderer | `TypstRenderer` | ✅ done |
| Page and code styling | `template.typ` | ✅ done |

---

## Design

### `Target::Pdf`

```rust
pub enum Target {
    Html,
    Epub,
    /// Typst. Like an epub, a PDF has no toggle: every elided span is a
    /// comment. Unlike an epub, its typography is ours to choose, which is
    /// what `template.typ` is for.
    Pdf,
}
```

`has_hidden_lines` returns `false`. `FromStr` and `Display` gain a case. That is
the entire render-engine change, and if it turns out to be more, EPIC-06's
architecture claim was wrong and this EPIC should say so.

### `TypstRenderer`

`bower/src/publish.rs`:

```rust
/// The PDF, via `pandoc --to typst` and `typst compile`.
///
/// Two steps rather than `pandoc --to pdf`, because that route reaches for
/// LaTeX — 7.5 GB of it, and measurably non-reproducible (see Context).
pub struct TypstRenderer {
    /// A `.typ` file setting page size, fonts, and code styling. `None` uses
    /// Typst's defaults, which are legible but not tuned for code.
    pub template: Option<PathBuf>,
}

impl Renderer for TypstRenderer {
    fn preflight(&self) -> Result<(), PublishError>;   // needs pandoc and typst
    fn render(&self, plan: &RenderPlan, out: &Path) -> Result<Artifact, PublishError>;
}
```

`render` writes chapters to a scratch directory exactly as `PandocRenderer`
already does (`bower/src/publish.rs:292`), converts each to `.typ`, concatenates
under the template, and compiles. **The scratch-writing is worth extracting**
rather than copying: `PandocRenderer` and `TypstRenderer` want the same numbered
chapter files, and two copies of that loop is two chances to order them
differently.

### The template

`books/hello-playbook/template.typ` (new), or a default compiled in:

```typst
#set page(paper: "us-letter", margin: 2.2cm)
#set text(font: "Libertinus Serif", size: 10.5pt)
#show raw: set text(font: "DejaVu Sans Mono", size: 9pt)
#show raw.where(block: true): block.with(fill: luma(248), inset: 8pt, radius: 3pt)
```

Font availability is a real risk and work item 2c is about facing it: a template
naming a font nobody has degrades silently, which is precisely how the `⋯` bug
survived to a spike. The renderer checks the named fonts against
`typst fonts` and refuses rather than substituting.

### Reproducibility

Typst records a creation timestamp. `SOURCE_DATE_EPOCH` is honoured by
`typst compile`; if it proves not to be, the fallback is `--creation-timestamp`,
fed from `bower.toml`'s `epoch`. Work item 0d settles which, by measurement
rather than by hope — and if **neither** yields identical bytes, that is a
finding worth the EPIC on its own, and the honest response is to record it and
reconsider LaTeX, not to quietly drop the promise.

---

## Work Items

### Phase 0 — Install, spike, decide for real

- [x] **0a.** **typst 0.15.1, a 43 MB binary.** Two corrections to this EPIC's
  own Context, both mine:
  1. It said typst was **not installed**. It was — since 17 July — but its
     Homebrew symlink was missing, so `command -v typst` correctly said no.
     `brew install` relinked it. The probe was right; my conclusion from it was
     wrong.
  2. It estimated **~30 MB**. The measurement is **43 MB**. The argument is
     unaffected — 43 MB against 7.5 GB is still a factor of 174 — but the
     number in the table is now one I took rather than one I guessed.
- [x] **0b.** `pandoc --from markdown --to typst` produced an 18.9 KB `.typ`,
  and `typst compile` produced a **144 KB PDF, exit 0, no warnings**. Compare
  with the LaTeX spike, which emitted six missing-glyph warnings.
- [x] **0c.** Everything arrives: the marked test, both elision forms, the
  `full file:` link, and the footer reading
  `src/lib.rs · L18–L21 · step 011 of hello-playbook · diff · browse`. **Zero
  directive lines.** **One finding:** a long URL inside a code block wraps
  mid-URL — `…/hello-playbook/` then `blob/step-011-…` on the next line. Legible
  but ugly, and it is the template's problem to solve (work item 4a), not the
  renderer's.
- [x] **0d.** **The decisive result.** Two plain runs produced different SHAs.
  Two runs with `SOURCE_DATE_EPOCH` pinned produced **identical bytes**. LaTeX
  stayed different even with `SOURCE_DATE_EPOCH` *and* `FORCE_SOURCE_DATE` set.
  Work item 3a is therefore settled by measurement: pin `SOURCE_DATE_EPOCH`,
  fed from `bower.toml`'s `epoch`.
- [x] **0e.** Not needed. Typst passed 0b and 0d, so the LaTeX option stays
  closed — with work item 4c recording what would re-open it. Verified on branch
  `review2`, 2 September 2026.

### Phase 1 — The target

- [x] **1a.** `Target::Pdf`, `has_hidden_lines() == false`, `Display`,
  `FromStr`.
- [x] **1b.** Tests: `target__pdf_has_no_toggle`,
  `target__round_trips_through_its_name` extended to three variants, and
  `render_plan__pdf_matches_epub_markdown` — the two targets that share an
  elision rule must produce the same markdown, which is the architecture claim
  restated.
- [x] **1c.** Confirm the html and epub goldens are untouched.

### Phase 2 — The renderer

- [x] **2a.** Extract the numbered-chapter scratch writer out of
  `PandocRenderer` (`bower/src/publish.rs:292`) so both renderers share one
  definition of chapter order.
- [x] **2b.** `TypstRenderer`: preflight for `pandoc` **and** `typst`, convert,
  compile, report the artifact.
- [x] **2c.** Refuse a template naming a font `typst fonts` does not list. A
  silently substituted font is how the `⋯` bug survived.
- [x] **2d.** Tests: `typst__preflight_names_both_tools`,
  `scratch__numbers_chapters_in_reading_order`.

### Phase 3 — Reproducibility, asserted

- [x] **3a.** Whatever 0d found — `SOURCE_DATE_EPOCH`, `--creation-timestamp`
  from `bower.toml`'s `epoch`, or both — implemented in `TypstRenderer`.
- [x] **3b.** `bower/tests/publish.rs`: `pdf_is_byte_identical_across_runs`,
  `#[ignore]`d because it needs typst, and named in the module docs like the
  other slow lanes.
- [x] **3c.** `make slow` gains it, so it runs where the other ignored lanes do.

### Phase 4 — Typography, goldens, documentation

- [x] **4a.** `template.typ` with page, body font, and code styling; the fonts
  it names checked in preflight.
- [x] **4b.** `make pdf`, `README.md`, and `BACKLOG.md`.
- [x] **4c.** Record in the corrigendum what would justify revisiting LaTeX:
  Typst's highlighting failing a language a real book uses, or a typographic
  need its templating cannot express. `Renderer` keeps that door open; this
  writes down when to walk through it.
- [x] **4d.** Flip Status rows; append the corrigendum.

---

## Test Plan

- `target__pdf_has_no_toggle` — the one render rule this target adds.
- `render_plan__pdf_matches_epub_markdown` — pdf and epub share an elision rule,
  so their markdown must be identical. If this ever fails, two targets have
  quietly grown two engines.
- `target__round_trips_through_its_name` — extended to three.
- `typst__preflight_names_both_tools` — the failure a reader will hit first.
- `scratch__numbers_chapters_in_reading_order` — shared by both renderers after
  2a, so a bug here would silently reorder an epub too.
- `pdf_is_byte_identical_across_runs` — the promise this EPIC exists to keep.
- The existing `epub_plan_elides_chapter_four` and `html_plan_keeps_the_toggle`
  must not move.

## Key Files

| File | Role |
|---|---|
| `bower/src/publish.rs:28` | `Target` gains `Pdf` |
| `bower/src/publish.rs:292` | scratch writer extracted from `PandocRenderer` |
| `bower/src/publish.rs` | new `TypstRenderer` |
| `books/hello-playbook/template.typ` | new; page and code styling |
| `bower/tests/publish.rs` | the reproducibility golden |
| `Makefile` | `pdf`, and `slow` gains the golden |

## Reuse (do NOT recreate)

- `bower/src/publish.rs:177` — `RenderPlan` is the input. The PDF target adds no
  fold of its own.
- `bower/src/publish.rs:230` — `Renderer` and `Artifact`. A third implementation,
  not a third pattern.
- `bower/src/publish.rs:250` — `tool_present` already answers "is this
  installed", and `elision__is_ascii…` already exists because it did not.
- `bower/src/publish.rs:270` — `slug` names the file.
- `bower/src/render.rs:151` — `body_lines` needs no change: `Pdf` simply reports
  no hidden lines.
- `bower/src/config.rs:24` — `epoch` is already the project's one source of
  reproducible time. A second timestamp would be a second answer.

## Compatibility

- **Preserves** the html and epub output exactly, and `bower-core`'s purity.
- **Adds** one enum variant, one renderer, one runtime binary (`typst`), and one
  template file.
- **Breaks** nothing.

## Dependencies

- **Blocks:** nothing. Editions (spec § 13 M2) will want to pin a PDF, but do
  not need one to exist.
- **Built on:** EPIC-06, which built `Target`, `RenderPlan`, and `Renderer`
  specifically so a third target would be small. This EPIC is the test of that.
- **Related:** spec § 13 M1; § 3.4 (per-target elision); EPIC-06 corrigendum
  item 7, the `⋯` bug this EPIC's spike found.

## Verification

```bash
typst --version
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo tree -p bower-core -e normal
cargo run -p bower -- --book books/hello-playbook publish --target pdf -o published
cargo run -p bower -- --book books/hello-playbook publish --target pdf -o published2
shasum published/hello-playbook.pdf published2/hello-playbook.pdf   # must match
cargo test -p bower --test publish -- --ignored
```

Exit criteria:

1. `publish --target pdf` produces a PDF a reader can open, with the book's
   title and authors.
2. Two runs produce **byte-identical** files, asserted by a test.
3. Code blocks are monospace and syntax-highlighted, with no mid-token line
   breaks, and every elision reads `... N lines elided — full file: <url>`.
4. The html and epub outputs are unchanged.
5. A missing `typst` fails with its install command named, before anything is
   written.
6. `cargo tree -p bower-core -e normal` still prints one line.


---

## Implementation corrigendum

Recorded 2 September 2026, branch `review2`.

### 1. Two facts in this EPIC's own Context were wrong

Both mine, both corrected by Phase 0's measurement. Typst was described as **not
installed** — it had been since July, but a missing Homebrew symlink meant
`command -v typst` correctly answered no; `brew install` relinked it. And the
footprint was estimated at **~30 MB**; it is **43 MB**. The argument survives
both — 43 MB against 7.5 GB is a factor of 174 — but a table of guesses is not
evidence, and this one now holds measurements.

### 2. EPIC-06's architecture claim held

Adding a third target required **one enum variant and three match arms**: the
variant itself, `Display`, `FromStr`, and the CLI's renderer selection. The
render engine — `body_lines`, `chapter`, the whole display-marker path — needed
**no change at all**. `Target::Pdf` simply reports no hidden lines, and
`render_plan__pdf_matches_epub_markdown` asserts the two targets that share an
elision rule produce byte-identical markdown.

The compiler said so plainly: after adding the variant, the only error in the
workspace was the CLI's non-exhaustive `match`.

### 3. Phase 1 could not stand alone, and did not pretend to

Adding `Target::Pdf` leaves the CLI's `match` non-exhaustive, so Phase 1 in
isolation would have shipped a stub arm saying "not built yet". That arm existed
for about four minutes. The phases were carried through together, as EPIC-02 and
EPIC-05 each concluded before them — a variant and the code that handles it are
one unit.

### 4. The template is a preamble, not a wrapper

Typst's `#set` rules apply to everything that follows, so a template is simply
prepended to pandoc's output. That is simpler than LaTeX's document-class
machinery, and it is a fair part of why Typst was the right call.

### 5. The font guard came from the `⋯` bug

`check_fonts` refuses a template naming a face `typst fonts` cannot list,
because Typst substitutes **silently** — which is exactly how a missing glyph
reached a spike unnoticed in EPIC-06. The scanner is deliberately a scan and not
a parser: a guard that needs a language front end is a guard that stops working.

### Phase status summary

| Phase | Status | Notes |
|---|---|---|
| 0 (install, spike, decide) | Shipped | item 1; the decision measured, not assumed |
| 1 (the target) | Shipped | item 2, 3 |
| 2 (the renderer) | Shipped | items 4, 5 |
| 3 (reproducibility) | Shipped | `SOURCE_DATE_EPOCH` from `bower.toml`'s epoch |
| 4 (typography, docs) | Shipped | |

### Still open after this EPIC

- **A long URL wraps mid-URL inside a code block** — `…/hello-playbook/` then
  `blob/step-011-…`. Found in Phase 0c, legible but ugly. The template can
  address it; nothing does yet.
- **`TypstRenderer` is untested**, the same last inch as `GitHubForge` and
  `PandocRenderer`. The decisions are tested against `FakeRenderer`, and the
  pure parts — `fonts_named_in`, `write_chapters`, `slug` — were extracted and
  are. `pdf_is_byte_identical_across_runs` covers the whole path end to end, in
  the ignored lane.
- **`--target ipynb`** remains, and still needs play cells (spec § 15).
- **LaTeX stays reachable.** `Renderer` is the seam. What would justify walking
  through it: Typst failing to highlight a language a real book uses, or a
  typographic need its templating cannot express. Neither has happened.
