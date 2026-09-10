# bower-voice-spike

A zero-dependency spike of the **voice kernel** proposed in
[`../../EPIC-09_Voice.md`](../../EPIC-09_Voice.md): the book as the single
source of truth for how it is read aloud.

It is deliberately **not** a member of the bower workspace, so `make ayce`
never sees it. Run it on its own:

```bash
cd spikes/bower-voice-spike
cargo test
cargo clippy --all-targets -- -D warnings
cargo tree -e normal          # one line: the kernel has no dependencies
```

Built and verified 10 September 2026 with rustc 1.95 (the workspace pins
1.98.1; nothing here needs anything newer than `u32::isqrt`, 1.84).

## What it proves

| Claim | Test |
|---|---|
| A `<!-- voice … -->` cue is an HTML comment, so the reader's book is untouched and `bower-core` never sees it. | `cues__are_html_comments_so_a_reader_never_sees_them`, `bower_directives__and_other_comments_are_ignored` |
| One-shot, `begin`/`end` span, and overlay scoping do what the module docs say. | `one_shot__directs_exactly_one_paragraph`, `span__covers_every_paragraph_until_end_and_a_one_shot_overlays_it` |
| Code fences are passages of kind `Code`, never scanned for cues. | `cue__in_a_code_fence_is_not_a_cue` |
| Every paragraph has a speaker, cued or not, so the breakdown covers the whole book. | `unmarked__paragraph_is_the_narrator_by_default`, `totals__count_prose_and_headings_but_not_code` |
| Every `VoiceError` variant is producible from a named fixture. | `errors__every_variant_is_producible_from_a_fixture` |
| The palette fit grades reach, names the gap vector, breaks ties stably, and flags collisions **only** between voices that share a chapter. | `fit__places_every_designed_voice_and_grades_the_reach`, `fit__flags_two_close_voices_only_where_they_share_a_chapter` |
| Script, breakdown text and fit text are deterministic. | `script__is_deterministic`, `text__is_stable`, `fit__text_is_stable_and_names_the_gap` |

41 tests. Three mutations were applied by hand and each was caught: dropping
the co-occurrence rule from collisions (1 test fails), making the scanner
fence-blind (5 fail), and letting a one-shot cue leak into following
paragraphs (6 fail).

## Module map

| File | Owns |
|---|---|
| `lib.rs` | `VoiceError`, `Errors`, prelude |
| `source.rs` | `Chapter`, `Location`, `LineRange` — copies of `bower-core`'s, to stay zero-dep |
| `cue.rs` | The `voice` grammar: `CueDirective`, `CueForm`, `Pace`, `Energy`, `Pause` |
| `cast.rs` | `Cast`, `Character`, `Tones`, `Lexicon` — the voice bible as a value |
| `script.rs` | The fold: chapters + cast → `Script` of `Passage`s |
| `breakdown.rs` | The producer's sheet, and its stable text |
| `palette.rs` | `Axes`, `VoiceRegion`, `Palette`, `fit()` → `FitReport`, and its stable text |
