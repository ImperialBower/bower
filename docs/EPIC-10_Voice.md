# EPIC-10: Voice — the book as the source of truth for the ear (VOICE)

## Summary

- **Builds:** `bower-voice`, a second kernel, plus `voices.toml`, `bower script`
  and `bower voices`: the narrator's script, cast, and breakdown derived from
  cues in the manuscript.
- **Why:** an audiobook's working documents are kept by hand beside the book
  and go stale silently, the drift Bower already removed for code.
- **Shape:** a `<!-- voice … -->` comment directs the next paragraph or span;
  `script()`, `breakdown()`, and `fit()` are pure folds over a `Palette`.
- **Proves it:** the spike's 41 tests, and `cargo tree -p bower-voice` printing
  exactly two lines.
- **Status:** Phase 0 spike complete at `docs/spikes/bower-voice-spike/`;
  Phases 1 to 5 not started, filed 10 September 2026.

---

## Context

Bower's thesis is an inversion: the book is the single source of truth and the
repository is a build artifact of it (`bower-spec.md` § 0–§ 2). Eight EPICs
have proven that for code — a fenced block annotated with `<!-- bower … -->`
becomes a file, a step, a commit, a tag, and a line-anchored link, and every
one of those is derived, deterministic, and checked.

An audiobook of the same book is produced today the way example repos used to
be: by hand, beside the source, drifting from it. A narrator reads the
manuscript and builds three documents nobody versions — a **character bible**
(who speaks, how they sound), a **pronunciation guide** (`pkcore` is
*pee-kay-core*), and a **breakdown** (which voices appear in which chapter,
how long the finished audio will run). A producer keeps a fourth: notes on
which characters the narrator can and cannot do. None of it links back to a
line of the book. Edit chapter 3 and all four are stale, silently, exactly the
failure class this project exists to remove.

The prior-art survey of 10 September 2026 found the *audio* half of "autotune
for narrators" crowded — Antares Throat models the vocal tract, ElevenLabs and
Altered re-voice a performance, ByteDance's VoiceShop edits age and gender
while preserving identity — and the *authoring* half empty. Nothing lets an
author mark a passage for the ear in the manuscript, nothing derives the
narrator's documents from those marks, and nothing maps a narrator's range as
data a cast can be checked against. The nearest thing is the clinical *voice
range profile* (pitch × loudness), which no casting workflow uses.

This EPIC adds that authoring half to Bower as a second domain kernel,
`bower-voice`, beside `bower-core`. Same discipline: the cue is an HTML
comment the reader never sees; the script, breakdown and palette fit are pure
folds over chapter text; the CLI, preprocessor and renderers are consumers.

**This EPIC does not** process audio. It produces the *control input* audio
processing would need — a per-character gap vector saying how far a voice sits
outside the narrator's range — and stops. It does not choose or integrate a
speech-to-speech engine; that is a later EPIC with its own I/O boundary. It
does not touch `bower-core`'s public API except to expose the key-value
grammar its directive parser already owns (Decision 3). It does not change
any existing command's behaviour: a book with no cues and no `voices.toml` is
a normal book.

A zero-dependency spike was built first and is checked in at
`docs/spikes/bower-voice-spike/`: 41 tests, clippy pedantic clean, three hand
mutations each caught. Its `README.md` maps claim to test.

---

## Status

| Component | Status |
|---|---|
| Spike — cue grammar, cast, script fold, breakdown, palette fit | **Complete** (`docs/spikes/bower-voice-spike/`) |
| `bower-voice` crate (Phase 1) | Not started |
| `bower-testkit` voice corpus and coverage (Phase 2) | Not started |
| `voices.toml`, `bower script`, `bower voices` (Phase 3) | Not started |
| Narrator script as a render target (Phase 4) | Not started |
| Palette study and the processing seam (Phase 5) | Not started |

---

## Goals

- **The manuscript carries the direction.** An author or producer marks who
  speaks and how — speaker, tone, pace, energy, a pause, a note — inline, in
  a comment the reader never sees, with the same hand that writes the prose.
- **The narrator's documents are derived.** Script, character bible,
  pronunciation guide and breakdown are outputs of one pure fold, regenerated
  every build, and each line of the script points at the chapter and line it
  came from.
- **Range is data.** A narrator's palette — the voices they can produce and
  how far each bends — is a value. The fit of a cast against it is a report:
  which characters are in range, which are a stretch, which are out of
  reach and by how much on which axis, and which two voices are too close to
  tell apart in a chapter they share.
- **Errors are located and collected.** An unknown speaker, an unknown tone,
  a cue with no paragraph, an unclosed span — every one names its chapter and
  line, and one pass reports all of them, the rule `BowerError` set.
- **Purity is asserted.** `bower-voice` depends on `bower-core` and nothing
  else; CI proves it the way `make purity` proves `bower-core` has nothing.

## Scope

### Authoring

A cue is a single-line HTML comment whose first word is `voice` (drafting
alias `vo`). Three forms:

```markdown
<!-- voice speaker="rosa" tone="warm,tired" pause="beat" -->
"Sit down, kid. Nobody bites before midnight."

<!-- voice begin speaker="dealer" pace="slow" note="never looks up" -->
"Blinds are up."

"Ante."
<!-- voice end -->
```

| Key | Values | Meaning |
|---|---|---|
| `speaker` | a cast id or alias | Who reads the passage. Absent means the narrator. |
| `tone` | comma list from the tone vocabulary | How it is coloured. Absent means the character's default tones. |
| `pace` | `slow` \| `measured` \| `brisk` \| `rushed` | |
| `energy` | `whisper` \| `soft` \| `level` \| `loud` \| `shout` | |
| `pause` | `beat` \| `breath` \| `long` | A hold *before* the passage. |
| `note` | free text | Direction the vocabulary cannot carry: `"she is lying"`. |

Scoping, in one place (`bower-voice/src/script.rs` module docs):

- A **one-shot** cue applies to the next paragraph and nothing else. Two cues
  with no paragraph between them: the first is `CueWithoutText`.
- A **span** sets an ambient direction for every paragraph until `voice end`;
  a one-shot inside a span overlays it (its keys win, the span's fill in).
- **Headings** are always the narrator, even inside a span.
- **Fenced code** is a passage of kind `Code`, never scanned for cues, and
  never given a speaker. A programming book read aloud has to decide what to
  do with its code — skip it, summarise it, read it — and the breakdown counts
  it so somebody can. That decision is Open Question 2.
- `<!-- bower … -->` directives and every other HTML comment are ignored.
  `Directive::is_directive_line` (`bower-core/src/directive.rs:164`) accepts
  only `bower` and `bf`, so the two grammars cannot collide.

Every paragraph becomes a passage whether or not it is cued. An unannotated
book has a complete script — the narrator, level, measured, in their default
tones — which is what makes the breakdown's word counts honest.

### Configuration — `voices.toml`

Beside `bower.toml`, read by the CLI, projected into kernel values exactly as
`BookConfig::catalog()` (`bower/src/config.rs:247`) projects the repo catalog.
Absent means: this book has no voice work, and that is not an error.

```toml
[tones]
# Extends the standard vocabulary. `replace = true` drops the standard set.
add = ["sardonic", "hushed"]

[narrator]
name = "Christoph"
tones = ["dry"]

[cast.rosa]
name        = "Rosa Alvarez"
aliases     = ["mrs-alvarez"]
description = "sixties, ex-dealer, never raises her voice"
tones       = ["warm"]
target      = { pitch = 45, age = 70, weight = 55, energy = 30 }

[cast.kid]
name   = "The Kid"
tones  = ["bright"]
target = { pitch = 75, age = 20, weight = 25, energy = 80 }

[lexicon]
pkcore = { say = "pee-kay-core" }
gix    = { say = "gicks", ipa = "ɡɪks" }

# One palette per narrator who might read this book. Measured or declared —
# the kernel does not care which; Phase 5 is about making it measured.
[palette.christoph]
own      = { pitch = 40, age = 50, weight = 60, energy = 40, radius = 15 }
young    = { pitch = 70, age = 25, weight = 30, energy = 70, radius = 12 }
old-man  = { pitch = 30, age = 80, weight = 70, energy = 30, radius = 10 }
```

Only what changes a pure computation crosses into the kernel: ids, aliases,
tones, targets, the lexicon, the palette. Reference-recording paths, session
notes, studio booking — shell concerns, when they exist at all.

### Commands

| Command | What it does |
|---|---|
| `bower script [--chapter <stem>] [--speaker <id>] [--target md\|html\|pdf]` | The narrator's script: every passage in reading order with its direction, each line anchored to `chapter:line`. `--speaker` filters to one voice's lines — the pickup list. |
| `bower voices` | The breakdown (words, code lines, finished-time estimate, per-chapter voices, per-character load and first appearance) followed by the character bible and the pronunciation guide. |
| `bower voices --fit <narrator>` | The fit report for one declared palette: placements, reach, gap vectors, collisions, undesigned characters. |
| `bower status` | Gains a `voices` row when `voices.toml` exists: the breakdown text is digested like the site marker, so an edited chapter reports the script stale. |

`bower plan`, `build`, `verify`, `publish`, `push` are unchanged. `bower
verify` gains cue validation as a *non-fatal* pass only if Open Question 1
is decided that way; the default is that `bower script` and `bower voices`
fail loudly on their own, the rule `mdbook-bower` follows.

### Rules

- A cue naming an unknown speaker is an error **and** the passage falls back
  to the narrator, so the script is still complete and the error is still
  reported. Never drop a paragraph.
- A tone outside the vocabulary is an error and is dropped from the passage;
  if nothing survives, the character's defaults apply.
- The narrator is always in the cast. If `voices.toml` declares no
  `[narrator]`, a default one is added with tone `neutral`.
- The narrator is never placed by the fit. The palette *is* the narrator; an
  undesigned narrator is the normal case, not a gap in the bible.
- A collision is reported only between two designed characters who **share a
  chapter**. Two identical voices that never meet are not a problem.
- Every report text is deterministic — `BTreeMap` everywhere, no clock, no
  environment — so it can be digested and diffed.

---

## Domain map

| Domain concept | Code construct | Status |
|---|---|---|
| The cue | `CueDirective`, `CueForm`, `Pace`, `Energy`, `Pause` — `bower-voice/src/cue.rs` | spike ✅ |
| Where in the book | `bower_core::source::{Chapter, Location}` (reused, Decision 2) | exists |
| The voice bible | `Cast`, `Character`, `Tones`, `Lexicon`, `Pronunciation` — `cast.rs` | spike ✅ |
| One unit of reading | `Passage { kind, lines_at, speaker, tones, pace, energy, pause, note, cue_at, text, words }` — `script.rs` | spike ✅ |
| The script | `script(&[Chapter], &Cast) -> (Script, Errors)` | spike ✅ |
| The producer's sheet | `breakdown(&Script, &Cast) -> Breakdown`, `breakdown_text()` — `breakdown.rs` | spike ✅ |
| A narrator's range | `Palette`, `VoiceRegion { center: Axes, radius }` — `palette.rs` | spike ✅ |
| Where a voice sits | `Axes { pitch, age, weight, energy }`, each `0..=100` | spike ✅ |
| Can the narrator get there | `fit() -> FitReport`, `Reach::{InRange, Stretch{gap}, OutOfReach{gap}}`, `Collision` | spike ✅ |
| What audio processing would need | `Gap` — signed per-axis delta | spike ✅ |
| Every failure, located | `VoiceError` (14 variants), `Errors` | spike ✅ |
| The config file | `VoicesConfig` in `bower/src/config.rs`, `voices()` projection | not started |
| Generated corpus and state coverage | `bower-testkit::voice::{fixtures, generators, coverage}` | not started |

---

## Decisions

1. **A second kernel crate, not a module of `bower-core`.** `bower-voice` is
   its own crate with its own error enum. The two kernels answer different
   questions over the same chapters — *what does the repo look like at this
   step* versus *how is this paragraph read* — and neither needs the other's
   plan. Keeping them apart keeps `bower-core`'s purity check trivially true
   (`cargo tree -p bower-core -e normal` stays one line) and lets the voice
   work move at its own pace without touching a kernel eight EPICs depend on.

2. **`bower-voice` depends on `bower-core`, for exactly three things:**
   `Chapter`, `Location`, and the key-value grammar. One definition of "where
   in the book" is the anti-drift rule this project applies to itself
   (`.okf/architecture/pipeline.md`: "a second copy of the marker grammar
   living in a consumer is exactly the drift this crate exists to prevent").
   The purity assertion for the new crate is therefore `cargo tree -p
   bower-voice -e normal` printing **two** lines, `bower-voice` and
   `bower-core`, and nothing else. The spike carries local copies of
   `Chapter`/`Location` only to stay standalone; Phase 1 deletes them.

3. **Expose `bower-core`'s key-value parser.** `KeyValues` in
   `bower-core/src/directive.rs` is private. Phase 1 makes a `pub fn
   key_values(rest: &str, loc: &Location) -> (Vec<(String, String)>, Errors)`
   beside it — the one small change to `bower-core` this EPIC makes — so the
   cue grammar and the directive grammar cannot disagree about quoting. The
   error variants stay per-kernel (`VoiceError::CueParse` wraps the reason
   string), because a voice error is not a Bower error.

4. **The cue is an HTML comment with a different first word.** Same three
   constraints the directive satisfied (`.okf/model/directive.md`): invisible
   in every renderer in use, no fight with mdBook conventions, writable
   mid-sentence. The fourth, new constraint: `bower-core` must not see it.
   `is_directive_line` already accepts only `bower`/`bf`; the spike test
   `bower_directives__and_other_comments_are_ignored` pins the converse.
   **No renderer needs to learn anything for the reader's book to stay
   clean.** The preprocessor already strips nothing but `bower` lines and the
   eye never sees a comment anyway.

5. **Paragraph is the unit, not the sentence or the quotation.** A one-shot
   cue applies to the next paragraph; multi-paragraph speech uses a span.
   Inline quotation binding (`"…," <!-- vo speaker="rosa" --> she said.`)
   was considered and deferred to Open Question 3: it needs a quotation
   grammar the kernel does not have, and in practice dialogue-heavy
   manuscripts already put one speaker per paragraph. Start with the rule
   that is unambiguous.

6. **The tone vocabulary is controlled, extensible per book.** `wrly` in
   chapter 12 must be a build error, not a mystery to the narrator six weeks
   later, and the breakdown counts tones per character, which needs a stable
   set. The standard set is fifteen words; `[tones] add` extends it,
   `replace = true` replaces it.

7. **Four axes, `0..=100`, integer distance.** Pitch, age impression,
   weight, energy — the dimensions a producer actually argues about, and the
   ones a pitch/formant stage can move. Euclidean distance with `u32::isqrt`
   so the number is identical on every machine. The clinical *voice range
   profile*'s two axes (pitch × loudness) are a strict subset. These are
   declared values in this EPIC; whether they are *measured* from a
   recording is Phase 5's question, and the kernel does not care.

8. **Reach has three grades and a gap.** Inside the radius is in range;
   inside twice the radius is a stretch the producer should hear about;
   beyond that is out of reach, and the report says how far on which axis.
   The gap vector is the seam to audio processing: a later EPIC can consume
   `Reach::OutOfReach { gap }` as a pitch/formant target without this crate
   knowing such a stage exists.

9. **Collisions require co-occurrence.** Two voices closer than
   `COLLISION_DISTANCE` (15, a starting value) are a collision only in a
   chapter they share. The spike's mutation test proves the rule has teeth:
   drop it and `fit__flags_two_close_voices_only_where_they_share_a_chapter`
   fails.

10. **The narrator is exempt from the fit.** The palette describes the
    narrator; placing them against it is circular. An undesigned narrator is
    not reported as undesigned.

11. **Code is counted, not spoken.** Fenced blocks become `PassageKind::Code`
    with no speaker. Every programming audiobook has to decide what to do
    with code, and the honest thing the tool can do today is say how much
    there is (per chapter, in lines) and leave the decision to Open Question 2.

12. **`bower-testkit` grows a `voice` module rather than a fourth crate.**
    One corpus, one coverage report, one `corpus_state_coverage_is_complete`
    invariant. The testkit already depends on `bower-core`; adding
    `bower-voice` keeps the "controllability half of the kernel" in one
    place. Reversible if the module outgrows the crate.

13. **Industry rule of thumb for runtime, named as such.** 9,300 words per
    finished hour, ACX's figure, as a documented constant. Studio time is two
    to four times finished time and the report says so rather than guessing
    a multiplier.

---

## Design

### The fold

```
chapters (bower-core::Chapter)      voices.toml ─projection─▶ Cast, Palette
      │                                              │
      ▼  fence-aware scan, cue binding               │
  Script { chapters: [ChapterScript { passages }] } ◀┘        (pure)
      │
      ├──▶ breakdown(&Script, &Cast) ─▶ Breakdown ─▶ breakdown_text()   (pure)
      │
      └──▶ fit(&Palette, &Cast, &Script) ─▶ FitReport ─▶ fit_text()    (pure)
                                                    │
                                        bower script / bower voices / status   (I/O)
```

`script()` never fails: errors are collected on the side and every paragraph
still lands in the script with a speaker. `breakdown()` never fails: a script
is always countable. `fit()` refuses an invalid palette (`PaletteEmpty`,
`DuplicateRegion`) because "nearest region" would be meaningless over one.

### Binding, in `script.rs`

The scanner is one state machine with three pieces of state: `pending`, a
one-shot cue waiting for its paragraph; `ambient`, the open span's direction
(or defaults); and the paragraph accumulator. The rules in *Scope → Authoring*
are the transitions. A cue left `pending` at a heading, a fence, another cue,
a `begin`, an `end`, or end-of-chapter is `CueWithoutText` at the cue's line.
A `begin` inside a span is `NestedSpan`; an `end` outside one is `OrphanEnd`;
a span still open at end-of-chapter is `UnclosedSpan` at its `begin`.

Direction resolves in `direct()`: the cue's key wins, else the ambient's, else
the default. Speaker resolution goes through `Cast::resolve`, which follows
aliases to the canonical id. Tones are filtered against the vocabulary and
each miss is an `UnknownTone`; an empty result falls back to the character's
`default_tones`.

### The fit, in `palette.rs`

Nearest region by integer Euclidean distance, ties broken by region name so
the answer never depends on declaration order. Reach graded against the
region's radius. Co-occurrence computed from the script — the set of chapters
each speaker appears in — and intersected pairwise for designed characters
within `COLLISION_DISTANCE`.

### Determinism and the digest

`breakdown_text()` and `fit_text()` are the canonical serialisations, in the
`lock_text()` tradition (`bower-core/src/plan.rs`): the kernel renders them,
nothing parses them. `bower status` digests `breakdown_text()` with the same
FNV-1a `digest()` that `.bower-site` uses (`bower/src/publish.rs`) and
compares — a stale script is visible without rendering it.

### What the CLI adds

- `bower/src/config.rs`: `VoicesConfig` and `BookConfig::voices() ->
  Option<(Cast, BTreeMap<String, Palette>)>`. A test pins that only kernel
  settings cross, the way `config__catalog_carries_only_kernel_settings`
  pins `RepoSpec`.
- `bower/src/script.rs` (new): renders a `Script` as markdown — one heading
  per chapter, one block per passage with a `speaker · tones · pace · energy`
  header line, the `pause` and `note` when present, and a `chapter:line`
  trailer. HTML and PDF reuse the existing `Target` machinery
  (`bower/src/publish.rs:28`); the script is prose, not code, so no elision
  rules apply.
- `bower/src/main.rs`: `Command::Script` and `Command::Voices`.
- `bower/src/status.rs`: a `VoicesDrift` beside `SiteDrift`, `NotConfigured`
  is not drift (the EPIC-04 rule).

---

## Work Items

### Phase 0 — Spike ✅

- [x] **0a.** Zero-dependency crate at `docs/spikes/bower-voice-spike/`, not a
  workspace member (its `Cargo.toml` declares `[workspace]` so cargo does not
  climb). 41 tests; `cargo clippy --all-targets -- -D warnings` clean;
  `RUSTDOCFLAGS=-D warnings cargo doc` clean; `cargo tree -e normal` one line.
- [x] **0b.** Every `VoiceError` variant has a fixture that produces it
  (`errors__every_variant_is_producible_from_a_fixture`).
- [x] **0c.** Three mutations applied by hand, each caught: collisions without
  co-occurrence (1 test), a fence-blind scanner (5), a leaking one-shot (6).
- [x] **0d.** `README.md` maps each claim to the test that proves it.

**What the spike changed about the design.** Two things the plan did not
have: the narrator exemption from the fit (Decision 10) fell out of the first
run of `fit__places_every_designed_voice…`, which reported the narrator as
undesigned; and the tie-break on region name (Design → The fit) fell out of a
fixture where Rosa sat exactly 23 from two regions. Both are now tested.

### Phase 1 — The `bower-voice` crate

- [ ] **1a.** `bower-voice/` in the workspace, `[dependencies] bower-core`,
  edition 2024, the `bower-core` lint posture. Move the spike's modules;
  delete `source.rs` and import `bower_core::source::{Chapter, Location}`.
  `LineRange` — reuse `bower_core::display::LineRange` if its shape fits,
  else keep a local one and record why.
- [ ] **1b.** `bower-core/src/directive.rs`: `pub fn key_values(...)`
  (Decision 3). Its existing tests move with it; `Directive::parse` calls it.
- [ ] **1c.** `cue.rs` calls `bower_core::directive::key_values` and wraps
  each parse error as `VoiceError::CueParse`. Delete the spike's copy.
- [ ] **1d.** `Makefile` `purity` target gains a second assertion:
  `cargo tree -p bower-voice -e normal` is exactly two lines. CI's
  `clippy & purity` lane runs it for free.
- [ ] **1e.** `.okf/architecture/crate-bower-voice.md` and
  `.okf/model/cue.md`, `.okf/model/passage.md`, `.okf/model/palette.md`, in
  the bundle's frontmatter style.

### Phase 2 — Controllability

- [ ] **2a.** `bower-testkit/src/voice/fixtures.rs`: `valid()` — at least
  `unannotated`, `one_shots`, `spans_and_overlays`, `aliases`,
  `code_heavy`, `two_narrators`; `broken()` — one per `VoiceError` variant,
  named for it, following `fixtures.rs:547`.
- [ ] **2b.** `voice/generators.rs`: `arb_voiced_book()` — arbitrary chapters
  with arbitrary cues over a generated cast; **every generated book must
  script cleanly**, the rule `arb_book` already follows.
- [ ] **2c.** `voice/coverage.rs`: `VoiceCoverage { forms, paces, energies,
  pauses, kinds, reaches, errors }` and `voice_corpus_state_coverage_is_complete`.
  `CueForm::all()`, `Pace::all()`, `Energy::all()`, `Pause::all()`,
  `PassageKind::all()`, `Reach::all()` exist for exactly this.
- [ ] **2d.** Properties: determinism of `script`, `breakdown_text`,
  `fit_text`; every passage's `lines_at` covers exactly its `text` in the
  chapter (the line-map property, for prose); the sum of passage words
  equals the chapter's non-code word count.

### Phase 3 — Configuration and commands

- [ ] **3a.** `VoicesConfig` in `bower/src/config.rs`; `BookConfig::voices()`
  projection; `config__voices_carries_only_kernel_settings`.
- [ ] **3b.** `Command::Voices` — breakdown, bible, lexicon, `--fit`.
- [ ] **3c.** `Command::Script` — `--chapter`, `--speaker`, `--target md`.
- [ ] **3d.** `VoicesDrift` in `bower status`; `NotConfigured` is not drift.
- [ ] **3e.** Goldens in `bower/tests/voices.rs` against a fixture book with
  a `voices.toml`; a book without one behaves exactly as today.

### Phase 4 — The script as a render

- [ ] **4a.** `--target html` and `--target pdf` for `bower script`, through
  `MdBookRenderer` and the Typst path, with a stylesheet that reads well on a
  music stand: large type, speaker in the margin, `note` set off.
- [ ] **4b.** `--speaker <id>` produces the pickup list — every line for one
  voice with its `chapter:line`, so a re-record session has an agenda.
- [ ] **4c.** Update `README.md`, `BACKLOG.md`, `.okf/roadmap/`.

### Phase 5 — Measured palettes and the processing seam (design only)

- [ ] **5a.** A design note (not code): how a palette's `Axes` could be
  *measured* from a narrator's structured read — pitch from f0, weight from
  spectral tilt, age and energy from a listener panel — so `voices.toml`'s
  `[palette]` is generated rather than guessed. I/O, a shell concern.
- [ ] **5b.** A design note: how `Reach::OutOfReach { gap }` maps onto a
  pitch/formant stage's parameters, what stays human, and what ACX's
  disclosure rules say about processed human narration (unclear as of
  July 2026; verify before building).

---

## Test Plan

Every row is a test the spike already has or Phase 2 names.

- `cues__are_html_comments_so_a_reader_never_sees_them` — strip cue lines and
  the chapter is the chapter; no renderer learns anything.
- `bower_directives__and_other_comments_are_ignored` — the two grammars
  cannot collide.
- `one_shot__directs_exactly_one_paragraph`,
  `span__covers_every_paragraph_until_end_and_a_one_shot_overlays_it` — the
  scoping rules as stated.
- `cue__in_a_code_fence_is_not_a_cue` — a book that quotes cues as examples
  survives its own scanner.
- `unmarked__paragraph_is_the_narrator_by_default` — the script is complete.
- `unknown_speaker__is_reported_and_falls_back_to_the_narrator` — never drop a
  paragraph.
- `errors__every_variant_is_producible_from_a_fixture` — no unreachable
  error.
- `fit__places_every_designed_voice_and_grades_the_reach` — reach grades and
  the gap vector, including the stable tie-break.
- `fit__flags_two_close_voices_only_where_they_share_a_chapter` — the
  collision rule, with its mutation counterpart.
- `fit__refuses_an_invalid_palette_rather_than_guessing`.
- `script__is_deterministic`, `text__is_stable`,
  `fit__text_is_stable_and_names_the_gap`.
- Phase 2: `voice_corpus_state_coverage_is_complete`,
  `every_generated_voiced_book_scripts_cleanly`,
  `passage_line_ranges_cover_exactly_their_text`.
- Phase 3: `config__voices_carries_only_kernel_settings`,
  `a_book_without_voices_toml_is_a_normal_book`,
  `voices__an_edited_chapter_is_drift_and_names_the_fix`.

## Key Files

| File | Role |
|---|---|
| `docs/spikes/bower-voice-spike/` | The spike; promoted, not copied, in Phase 1 |
| `bower-voice/src/{lib,cue,cast,script,breakdown,palette}.rs` | The kernel |
| `bower-core/src/directive.rs:300` | `KeyValues` → `pub fn key_values` |
| `bower-testkit/src/voice/{fixtures,generators,coverage}.rs` | Controllability |
| `bower/src/config.rs:27` | `BookConfig` gains `voices()` |
| `bower/src/script.rs` | new; the script renderer |
| `bower/src/main.rs:48` | `Command::{Script, Voices}` |
| `bower/src/status.rs:99` | `VoicesDrift` beside `SiteDrift` |
| `Makefile` | `purity` asserts two crates |
| `books/*/voices.toml` | Per-book cast, lexicon, palettes |

## Reuse (do NOT recreate)

- `bower-core/src/source.rs` — `Chapter`, `Location`. One definition of
  "where in the book".
- `bower-core/src/directive.rs` — the `key="value"` grammar, once exposed.
  The spike's `KeyValues` is a placeholder and is deleted in 1c.
- `bower-core/src/block.rs` `fence_width` / `skip_fence` — the fence rule.
  Either expose them or accept a documented copy; the spike copies. Prefer
  exposing: a fence that `bower-core` skips and `bower-voice` reads would be
  a real bug.
- `bower/src/publish.rs` `digest()` — the FNV-1a digest `.bower-site` uses;
  `VoicesDrift` hashes `breakdown_text()` with it.
- `bower/src/status.rs` — the `NotConfigured`-is-not-drift rule and the row
  shape.
- `bower-testkit/src/coverage.rs` — the `CoverageReport` pattern; the voice
  report follows its shape.

## Compatibility

- **Preserves** every existing command. A book with no cues and no
  `voices.toml` produces byte-identical plans, locks, repos and renders.
- **Adds** one crate, one config file, two commands, one `status` row, one
  `pub fn` on `bower-core`.
- **Breaks** nothing. `bower-core`'s dependency count stays zero.

## Dependencies

- **Built on:** EPIC-03 for the comment-is-invisible argument; EPIC-04 for
  the drift-row rule; EPIC-06/07 for the render targets Phase 4 reuses;
  EPIC-08 for the digest.
- **Blocks:** any audio-processing EPIC — it consumes `Gap` and needs the fit
  to exist first.
- **Related:** the publishing ladder M3 (author tooling) — `bower script` is
  author tooling; authoring bridges (Obsidian sees cues as comments, same as
  directives; Scrivener's smart quotes cannot corrupt a cue because a cue
  carries no code).

## Verification

```bash
# Phase 0 (now)
cd docs/spikes/bower-voice-spike
cargo test                                   # 41 passed
cargo clippy --all-targets -- -D warnings    # clean
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo tree -e normal                         # one line

# Phase 1+
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo tree -p bower-core  -e normal          # one line
cargo tree -p bower-voice -e normal          # two lines: bower-voice, bower-core
cargo run -p bower -- --book books/rust4failures voices
cargo run -p bower -- --book books/rust4failures voices --fit christoph
cargo run -p bower -- --book books/rust4failures script --chapter ch01-local_development
cargo run -p bower -- --book books/rust4failures status
```

Exit criteria:

1. `bower voices` on a book with a `voices.toml` prints the breakdown, bible
   and lexicon; on a book without one, says so and exits 0.
2. `bower voices --fit <narrator>` grades every designed character and names
   the gap for every stretch and out-of-reach placement.
3. `bower script --speaker rosa` lists exactly Rosa's passages with
   `chapter:line` anchors that point at the paragraphs a reader can find.
4. Editing a cued paragraph and re-running `status` reports `voices STALE`.
5. A cue naming an unknown speaker fails `bower script` naming chapter and
   line, and the same book still `bower build`s — the two kernels are
   independent.
6. `cargo tree -p bower-core -e normal` still prints one line;
   `cargo tree -p bower-voice -e normal` prints two.
7. `voice_corpus_state_coverage_is_complete` is green.

---

## Open questions — decisions, not code

| # | Question | Leaning |
|---|---|---|
| 1 | Should `bower verify` run cue validation, or is `bower script` failing loudly enough? | Leave `verify` alone. It verifies code; a voice error is not a broken step. Revisit if a cued book ships with a bad cue nobody ran `script` on. |
| 2 | What does an audiobook do with a fenced code block? | Out of scope for the kernel; the breakdown counts code lines so the producer can decide per book. A `code = "skip" \| "summarise" \| "read"` key in `voices.toml` is the likely shape when it is needed. |
| 3 | Inline quotation binding — a cue mid-paragraph applying to the next quoted string? | Deferred (Decision 5). Needs a quotation grammar; straight vs curly quotes alone is a hazard the authoring-bridges work already names. |
| 4 | Should `Axes` carry a fifth axis — placement (nasal/chest/head) or accent? | Not yet. Four is what a producer argues about and what a formant stage can move. Accent is categorical and belongs on `Character`, not on the axes. |
| 5 | Where do reference recordings live? | Shell-side, by path, never in the kernel. Possibly `[cast.rosa] reference = "voices/rosa.wav"` read only by a future audio EPIC. |

---

## Prior art (for the record)

Surveyed 10 September 2026. Audio-side tools are mature and converging on
speech-to-speech; nothing does the authoring side.

- DSP vocal design: Antares Throat (physical vocal-tract model), Soundtoys
  Little AlterBoy, Waves Vocal Bender — music-production tools, no narrator
  ergonomics.
- Neural re-voicing: ElevenLabs Voice Changer (lists "consistent character
  voices across recording sessions" as a use case), Altered Studio (voice
  actors performing into other voices; dubbing), Respeecher marketplace.
- Identity-preserving attribute editing: ByteDance VoiceShop (age, gender,
  accent, style; unreleased), *Controlling your Attributes in Voice* (2025).
- Directly on point academically: Sini et al., *Inter- and Intra-speaker
  Voice Conversion using Audiobooks*, LREC 2022 — one narrator's narration
  vs. character speech as two speakers.
- Range mapping: the clinical voice range profile / phonetogram (pitch ×
  loudness) and speech range profile; nothing in casting practice.
- Patent to read: US 8,370,151 (K-NFB Reading Technology, 2009) — assigning
  voices to user-selected text portions, primarily TTS.
- Platform constraint: ACX sanctions AI narration only through Audible's own
  pipeline and is silent on AI-processed human narration as of July 2026.
