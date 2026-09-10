# EPIC-11: Captured diagnostics — the compiler as co-author (DIAG)

## Context

Eight EPICs shipped. Every step carries an `Expect`
(`bower-core/src/directive.rs:92`), and `bower verify` checks it against a real
compiler: the step's tree is written to a scratch directory
(`bower/src/verify.rs:183`), the repo's `check` command runs, and the `verify`
command runs after it unless the step declared `compile_fail`
(`verify.rs:201-206`). `verdict_for` (`verify.rs:232`) reduces the two
outcomes to `Upheld`, `Broken`, or `Skipped`. The claim is *that* it failed,
never *what it said*.

What it said is kept only to explain a broken step. `Outcome`
(`verify.rs:44`) holds `stderr` "for the failure report". `run`
(`verify.rs:289`) drops stdout (`verify.rs:311-315`). `run_verify` prints
eight stderr lines of a `Broken` step (`bower/src/main.rs:374`) and nothing
else. Spec § 12 Q3 closed this way on purpose: "failure-only"
(`bower-spec.md:523-528`), because "snapshots churn with every rustc release"
(`docs/EPIC-02_Verification.md:24-25`). It left the door open: "no book has
yet needed the stronger claim."

The book has now needed it twice, and pasted it by hand both times.
`books/rust4failures/src/ch03-rank.md:100-103` quotes the headline of an E0004
under a `compile_fail` step (`ch03-rank.md:73`) and says "the message is the
lesson". `books/rust4failures/src/ch01-local_development.md:262-268` quotes a
test panic under a `test_fail` step (`ch01-local_development.md:216`). Nothing
checks either one. The day rustc rewords E0004, `make failures` stays green,
even though `.github/workflows/slow.yml:130-133` already promises: "the day a
new compiler changes a diagnostic the book quotes, this is what says so." This
EPIC makes that sentence true.

The spec contradicts itself here. § 6 still says `compile_fail` may match "a
stored pattern (trybuild-style)" and `test_fail` means a *named* test fails
(`bower-spec.md:348-349`). Q3 says nothing is stored, and `verdict_for` names
no test (`verify.rs:261-270`). Idea A1 (`docs/bower-ideas.md:64-112`) resolves
it. Failure-only stays the default for `expect`. A captured output is the
opt-in, and it is visible book content rather than a hidden fixture. That is
the only kind of snapshot an author keeps honest.

**This EPIC does not** capture output for steps that do not ask for it. It
does not change the `expect` matrix or `verdict_for`. It does not verify on
several toolchains (B4, EPIC-16). It does not build the failure index
(EPIC-12). It does not make `bower status` run a compiler: `status` "is the
one command that touches nothing outside the machine"
(`docs/TECHNICAL_DEBT.md:216-217`). It does not parallelize verification
(`TECHNICAL_DEBT.md:63-68`). A book with no `output=` directive plans, builds,
verifies, and renders exactly as before.

---

## Status

| Component | Status |
|---|---|
| Edge prerequisite: `Outcome` keeps stdout; per-repo `toolchain` pin | **Planned** |
| Kernel: `output=` key, `Capture`, binding, five errors, lock text | **Planned** |
| Kernel: `capture` module — normalize, drift, error codes, rewrite | **Planned** |
| Testkit: output textures, fixtures, `capture × expect` axis, properties | **Planned** |
| Verify: drift check, report, exit code | **Planned** |
| Verify: `--record` rewrites chapters and `bower.lock` | **Planned** |
| Render: caption line and error-code link, all three targets | **Planned** |
| Books: the two hand-pasted outputs replaced; sample-book golden | **Planned** |
| Docs: spec § 3.2, § 6, § 12 Q3; `.okf`; backlog and debt | **Planned** |

---

## Goals

- Make **what the compiler says** at a step an **executable claim**, checked on
  every `bower verify`, the way `expect` already made *whether it fails* one.
- Keep it **visible**: the captured text is a fenced block in the chapter,
  rendered for the reader and diffed in review.
- Keep the kernel **pure**. Normalizing, comparing, and rewriting are string
  functions in `bower-core`. Running cargo and writing files stay in `bower`.
- Add **one directive key**, per the ideas doc's admission rule
  (`docs/bower-ideas.md:19-23`).
- Hand EPIC-12 the **error codes** as plan data.

## Scope

### Authoring — one key

| Key | Values | Meaning |
|---|---|---|
| `output="…"` | `check` \| `verify` | The fence holds what the repo's `check` (or `verify`) command printed at the bound step, normalized. `bower verify` compares; `bower verify --record` writes. |

````markdown
<!-- bower repo="rust4failures" step="from-char-broken" … expect="compile_fail" -->
```rust
…
```

The code is still rejected, and the message is the lesson:

<!-- bower repo="rust4failures" output="check" -->
```text
error[E0004]: non-exhaustive patterns: `'\0'..='/'`, `'1'`, `':'..='@'` and 9 more not covered
  --> src/rank.rs:14:15
…
```
````

An output block binds to a step the way a play cell does: an explicit `step=`
if given, else the step that owns the nearest preceding code block of that
repo. It never touches a tree. An empty fence is legal and means "not
recorded yet".

The reader gets the compiler's actual words under every failure the book
stages, captioned with the command, the step, and a link for each rustc error
code. The author gets a `verify` that fails when those words move, and
`verify --record [--step S]`, which rewrites the fences in place.

### The kata

**Things.** A *capture* (whose output: `check` or `verify`). An *output block*
(a directive, a fence, and a bound step). *Raw output* (a process's bytes).
*Normalized output* (lines that are stable across machines and runs). A
*drift* (the first line where the recorded and live outputs disagree). An
*error code* (`E0004`).

**Business requirements.**

1. An output block names a capture its step actually runs. `check` runs
   unless the step is `none`, and `verify` runs only on `pass` and `test_fail`
   (`verify.rs:191`, `verify.rs:202`).
2. An output is judged only when the step's claim holds.
3. Normalized output depends on what the compiler said, and not on the
   machine, path, clock, terminal, or thread scheduling.
4. A mismatch, or a fence that was never recorded, fails `bower verify`.
5. Recording changes only fence bodies and leaves the lock consistent.
6. An output block changes no tree, commit, tag, or SHA.

**Business logic.** The Design and Work Items below.

### Not in scope

Arbitrary third commands, regex or code-only matching (open question 1),
per-toolchain snapshots, a hidden `trybuild`-style `.stderr` tree, and outputs
for play cells (spec § 12 Q8).

---

## Decisions

1. **`output="check"`, not `<!-- bower output … -->`.** The idea's bare word
   (`docs/bower-ideas.md:74`) does not parse. After `bower`, every token must
   be `key="value"`, or `KeyValues` reports "expected key=\"value\""
   (`directive.rs:314-320`). The non-tree blocks that already exist use key
   form: `notebook="play"` (`directive.rs:229-239`) and `exercise="…"`
   (`directive.rs:228`).
2. **A capture names a command, not a stream.** The idea's four kinds
   (`stderr`, `stdout`, `test`, `check`; `bower-ideas.md:89`) overlap. The
   verifier already runs exactly two configured commands
   (`bower/src/config.rs:73-75`, defaults `config.rs:22-23`). Each capture
   takes that command's stderr, then its stdout, and normalizes the result.
   For `cargo check` that is the diagnostics. For `cargo test` it is warnings
   plus the test report, which libtest prints on stdout. The names follow
   `bower.toml` and assume no language.
3. **Five located plan-time errors.** `OutputUnbound` and `OutputUnknownStep`
   mirror play cells (`bower-core/src/lib.rs:154-158`).
   `OutputConflictingKeys` fires on tree keys, `notebook`, `exercise`,
   `expect`, or `include`. `OutputDuplicate` fires on two blocks for one
   `(step, capture)` and is reported at the second, as `ExerciseDuplicate` is.
   `OutputNeverRuns` fires on `verify` for a `compile_fail` step, or on any
   capture for a `none` step. That last one is the matrix, stated at plan
   time: a fence the verifier could never fill is a typo.
4. **Normalization is kernel code; its inputs come from the edge.** The kernel
   cannot know the scratch path. The edge passes a `Scrub` of
   `(prefix, replacement)` pairs, and `normalize(raw, &Scrub)` stays pure and
   dependency-free.
5. **By default, match the full rendering exactly.** This is the idea's Part F
   Q3 default (`bower-ideas.md:832-836`). The spans are the lesson: the caret
   under `c` is what E0004 teaches. Both hand-pastes quote only the headline,
   which is evidence for an escape hatch. That escape hatch is open question 1.
6. **Judged only when upheld.** A `Broken` step's outputs are not compared,
   because the broken claim is the headline. `--record` never writes them: a
   snapshot of the wrong failure is worse than none.
7. **An empty fence fails verify.** An author writes the directive, then runs
   `--record`. Until then, plain `verify` says what to run.
8. **Recording rewrites the lock.** `lock_text` prints the captured lines. A
   changed line count moves every later `anchor=` in that chapter
   (`bower-core/src/plan.rs:378-382`). So `--record` re-resolves and writes
   `bower.lock`, as `run_plan` does (`main.rs:237-241`).
9. **No SHA moves.** An output block is `Op::Prose` and joins no step, as a
   play cell does (`bower-core/src/block.rs:304-310`). Commits name the step
   by id, not line (`bower/src/trailers.rs:25-28`). The site fingerprint does
   move, because it hashes chapter text (`bower/src/publish.rs:854`). That is
   correct, because the page changed.
10. **The toolchain is pinned, or verify warns.** The scratch tree lives under
    the system temp directory (`verify.rs:32-40`), so the workspace pin
    (`rust-toolchain.toml:2`) does not reach it. A tree without its own
    `rust-toolchain.toml` compiles on whatever rustup resolves there.
    `hello-playbook` pins at step `toolchain`
    (`books/hello-playbook/src/ch03-lints-and-format.md:19`);
    `rust4failures` never does. A new per-repo `toolchain` key is exported as
    `RUSTUP_TOOLCHAIN` beside `CARGO_TARGET_DIR` (`verify.rs:298`). Verify
    prints one warning when a repo has output blocks, no `toolchain` key, and
    a tree that pins nothing.
11. **Error codes are plan data.** `capture::error_codes` has one parser. The
    render caption and EPIC-12 both call it.

---

## Domain map

| Domain concept | Code construct | Status |
|---|---|---|
| Whose output | `Capture { Check, Verify }` in `directive.rs` | 🔴 new |
| The key | `Directive::output`, beside `notebook` `directive.rs:151` | 🔴 new |
| An output block | `Block::output` `block.rs:25` | 🔴 new |
| Binding | `StepIndex::locate` `plan.rs:241` | ✅ reuse |
| A step's outputs | `PlannedStep::outputs: Vec<CapturedOutput>` `plan.rs:39` | 🔴 new |
| Five errors | `BowerError` `lib.rs:63` | 🔴 new |
| Normalization, drift, codes, rewrite | `capture.rs` | 🔴 new |
| Fence scanning | `fence_width` `block.rs:127`, `capture_block` `block.rs:159` | 🟡 exists, private |
| What a command printed | `Outcome` `verify.rs:44` | 🟡 gains `stdout` |
| Claim vs reality | `verdict_for` `verify.rs:232` | ✅ unchanged |
| Output vs reality | `OutputVerdict` on `StepVerdict` `verify.rs:68` | 🔴 new |
| The toolchain | `RepoConfig::toolchain` `config.rs:54` | 🔴 new |
| Safe chapter write | `materialize::check_path` `materialize.rs:87` | ✅ reuse |
| The caption | `render::chapter` `render.rs:23` | 🟡 one new branch |
| Error-code link | `LinkTemplates::error_code` `config.rs:85` | 🔴 new |

---

## Design

### Kernel — key, block, binding

`Capture` gets `Display` and `FromStr`, shaped like `Expect`
(`directive.rs:92-132`). `Directive::output` is parsed with a `BadValue` arm
and overlaid in `overlaid_on` (`directive.rs:266`). `block::resolve`
(`block.rs:206`) treats output blocks as a third non-tree kind beside `play`
and `exercise_block` (`block.rs:249-266`). The fence is required
(`DirectiveWithoutBlock`). Anything `carries_tree_keys` sees (`block.rs:195`)
is refused, and the op becomes `Op::Prose`. `include` is refused too: the
loader never loads a library (`bower/src/loader.rs:90-93`), and `--record`
would have to write into one.

`plan()` partitions output blocks out before `step::group`, as it does play
and exercise blocks (`plan.rs:129-131`). `bind_outputs` sits beside
`bind_play_cells` (`plan.rs:262`), then checks `OutputNeverRuns` against the
bound step's `expect`.

```rust
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Capture { Check, Verify }

/// One `output="…"` block, bound to a step. Never part of any tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapturedOutput {
    pub loc: Location,      // the directive: where errors and `--record` point
    pub capture: Capture,
    pub info: String,       // the author's fence info string, kept on rewrite
    pub lines: Vec<String>, // empty = not recorded yet
}
// PlannedStep gains `outputs: Vec<CapturedOutput>`, in document order.
```

`lock_text` (`plan.rs:369`) prints outputs under their step, the way it
prints exercise detail (`plan.rs:390-399`), so review sees a rewording:

```
002 from-char-broken expect=compile_fail anchor=src/ch03-rank.md:73 files=src/rank.rs
    output check at=src/ch03-rank.md:98
    > error[E0004]: non-exhaustive patterns: `'\0'..='/'`, `'1'`, `':'..='@'` and 9 more not covered
```

### Kernel — `capture.rs` (new)

```rust
pub struct Scrub(pub Vec<(String, String)>);   // longest prefix first
pub fn normalize(raw: &str, scrub: &Scrub) -> Vec<String>;   // pure, idempotent

pub struct Drift { pub line: usize, pub recorded: Option<String>, pub live: Option<String> }
pub fn drift(recorded: &[String], live: &[String]) -> Option<Drift>;

pub fn error_codes(lines: &[String]) -> Vec<String>;   // "E0004", first-seen order, deduped

/// Replace the fence body after the directive at `directive_line`; widen the
/// fence if a body line would close it; touch nothing else.
pub fn rewrite(chapter_text: &str, directive_line: usize, lines: &[String]) -> String;
```

The rules run in this order. Each has a texture, and the table is a first cut:

| # | Rule | Why |
|---|---|---|
| 1 | CRLF → LF | a Windows runner must record what a Mac records |
| 2 | Strip ANSI escapes (hand-written, no `regex`) | `CARGO_TERM_COLOR=always` in someone's shell |
| 3 | Replace each `Scrub` prefix | the tree path is absolute. rustc spans are already relative, as `--> src/lib.rs:23:9` beside `Compiling … (/Users/…)` shows (`books/rust4failures/src/REJECTED_ch01-hell_in_a_cell.md:127-129`) |
| 4 | Drop cargo status lines: a right-aligned verb (`Compiling`, `Checking`, `Finished`, `Running`, `Doc-tests`, `Blocking`, `Locking`, `Updating`, `Downloading`, `Downloaded`, `Adding`, `Fresh`) | build chatter that varies with the cache |
| 5 | Drop `error: could not compile …` and `error: test failed, to rerun pass …` | cargo's words about the failure, not the failure (open question 2) |
| 6 | Strip `; finished in N.NNs` from `test result:` | the clock |
| 7 | Sort each contiguous run of `test <name> ... <result>` lines | libtest reports tests as they finish, and with several threads the order is not fixed (open question 5) |
| 8 | Trim trailing whitespace and outer blank lines | a snapshot must survive an editor |

`rewrite` reuses `fence_width` and `capture_block`, made `pub(crate)`
(`block.rs:127`, `block.rs:159`), rather than writing a third fence scanner.
`render.rs:270` already holds a second one.

### Testkit — `bower-testkit`

`src/textures.rs` (new) holds raw outputs as constants:

- E0004 with its note and spans
- E0308 (the sample book's `wont-compile`)
- a test panic with `left`/`right` and the backtrace note
- a warning-only `check`
- E0004 in ANSI colour
- E0004 with CRLF line endings
- a cargo manifest error naming the scratch path (the shape of
  `ch01-local_development.md:95`, a `compile_fail` on a lone `Cargo.toml`)
- a timed `test result:`
- three tests in two orders

`fixtures::captured_outputs()` covers the five legal `capture × expect` pairs,
binding by position and by explicit `step=`, and joins `valid()`
(`fixtures.rs:374`). Five broken fixtures join `broken()`, which
`bower-testkit/tests/corpus.rs:23` holds to its named error. Coverage gains
`Mechanism::CapturedOutput` (`coverage.rs:41`) and a `capture × expect` axis
beside `exercise_expects` (`coverage.rs:83`, `coverage.rs:200`).

### Verify — `bower`

`Outcome` gains `stdout` (`verify.rs:311-315`), and the `Broken` report still
prints stderr only. `Verifier::run` keeps the outcomes it already computes
(`verify.rs:201-206`) and judges each output of an `Upheld` step:

```rust
pub enum OutputResult { Matches, NotRecorded, Drifted(Drift), Unjudged }
pub struct OutputVerdict { pub loc: Location, pub capture: Capture, pub live: Vec<String>, pub result: OutputResult }
// StepVerdict gains `outputs: Vec<OutputVerdict>`; VerifyReport gains `drifted()`.
```

The edge builds the `Scrub` from its own paths (`verify.rs:182-183`):

- `tree_dir` → `""`
- `target_dir` → `target`
- `$CARGO_HOME` → `$CARGO_HOME`

`run_verify` prints `drft` rows. For each drift it prints the block's
location, the first differing line, and both sides. It exits non-zero on any
drift. For `NotRecorded`, the hint is the literal
`bower verify --record --step <id>`.

`--record` joins `Verify` (`main.rs:106-128`) and honours the
`--repo`/`--step`/`--from` selection. For each chapter holding `Upheld`
outputs, it folds `capture::rewrite` over the chapter text. It writes only
changed chapters, through `check_path` (`materialize.rs:87`), because
`loc.chapter` is book-controlled (`loader.rs:87`). Then it re-resolves and
writes `bower.lock`. `RepoConfig::toolchain` joins the wire struct
(`config.rs:292`) and reaches `run` (Decision 10).

### Render — `bower`

`render::chapter` already passes an output fence through verbatim. It is
absent from `blocks_by_line` (`render.rs:563`), so it gets no header, and
`body_lines` returns an unmarked body untouched (`render.rs:183-185`). The
only addition is `outputs_by_line` and one caption above the fence, classed
the way `step-meta` is (`render.rs:324-327`):

```
<span class="step-output"><sub>$ cargo check · step 002 of rust4failures · [E0004](…/error_codes/E0004.html)</sub></span>
```

The command comes from `LinkTemplates::check` and `verify` with defaults
applied (`config.rs:104-109`). Codes link through a new
`links.error_code` template with a `{code}` placeholder; with no template,
the code prints plain (`render.rs:542-547`). A link has to go in the caption,
because markdown does not render inside a fence. Epub, PDF, and the
preprocessor get the caption through the one `chapter` function
(`publish.rs:520`, `mdbook_bower.rs:98`).

---

## Work Items

### Phase 0 — Edge prerequisites

- [ ] **0a.** `Outcome::stdout`, filled by `run`. `run__captures_stdout` uses
  `echo`, not cargo.
- [ ] **0b.** `RepoConfig::toolchain` (`config.rs:54`, `config.rs:292`) →
  `RUSTUP_TOOLCHAIN` (`verify.rs:298`). `config__toolchain_is_optional`,
  `run__exports_the_toolchain_when_set`.
- [ ] **0c.** `block.rs` fence helpers to `pub(crate)`; `make test` green at
  the same count.

### Phase 1 — Kernel: key, block, binding

- [ ] **1a.** `Capture`, `Directive::output`, `BadValue` arm, overlay.
- [ ] **1b.** `Block::output`; `resolve`; `OutputConflictingKeys`.
- [ ] **1c.** Partition, `bind_outputs`, `OutputUnbound`, `OutputUnknownStep`,
  `OutputDuplicate`, `OutputNeverRuns`, `PlannedStep::outputs`, prelude.
- [ ] **1d.** `lock_text` output lines.
- [ ] **1e.** `captured_outputs()`, five broken fixtures, the mechanism, the
  axis.

### Phase 2 — Kernel: `capture.rs`

- [ ] **2a.** Textures first, then `normalize`, one rule per failing test, in
  table order.
- [ ] **2b.** `drift`, `error_codes`.
- [ ] **2c.** `rewrite`, widened-fence case included. Settle open question 1
  first.
- [ ] **2d.** The four properties; `make purity` green.

### Phase 3 — Verify

- [ ] **3a.** `OutputVerdict`, the `Scrub`, judging only `Upheld` steps.
- [ ] **3b.** Report rows, drift message, exit code, `NotRecorded` hint.
- [ ] **3c.** `--record`: write through `check_path`, re-resolve, write the
  lock; a second run writes nothing.
- [ ] **3d.** The toolchain warning.

### Phase 4 — Render

- [ ] **4a.** `outputs_by_line`, the caption, `step-output`.
- [ ] **4b.** `links.error_code` in `LinkTemplates` and `WireLinks`
  (`config.rs:307`).
- [ ] **4c.** Render tests on all three targets; a case in
  `bower/tests/preprocessor.rs`.

### Phase 5 — Books and docs

- [ ] **5a.** `hello-playbook` ch04: `output="check"` under `wont-compile`
  (`ch04-tests-and-failing-on-purpose.md:124`, E0308) and `output="verify"`
  under `test-that-fails` (`…:70`), recorded. The fast golden
  (`bower/tests/verification.rs:62`) asserts `Matches`.
- [ ] **5b.** `rust4failures`: output blocks replace the fences at
  `ch01-local_development.md:262-268` and `ch03-rank.md:100-103`.
  `ch03-rank.md` is commented out of `SUMMARY.md`
  (`books/rust4failures/src/SUMMARY.md:11`), so its block is planned only once
  the chapter is listed.
- [ ] **5c.** Spec: `output` in § 3.2 (`bower-spec.md:122-136`). The output
  block replaces § 6's "stored pattern" and "named test"
  (`bower-spec.md:348-349`). § 12 Q3 amended to "failure-only by default;
  visible opt-in snapshots, EPIC-11".
- [ ] **5d.** `.okf/model/directive.md`, a new
  `.okf/model/captured-output.md`, and `.okf/commands/verify.md`. Close
  `TECHNICAL_DEBT.md:70-74` and `BACKLOG.md:85`, and move A1
  (`BACKLOG.md:54`) out of Ideas.
- [ ] **5e.** Flip Status rows; append the corrigendum.

---

## Test Plan

- **Kernel:**
  - `parse__output_accepts_check_and_verify`,
    `parse__output_rejects_any_other_value`
  - `extract__output_needs_its_fence`,
    `extract__output_with_tree_keys_is_refused`,
    `extract__output_with_include_is_refused`
  - `plan__output_binds_to_the_nearest_preceding_step`,
    `plan__output_binds_explicitly_by_step`
  - `plan__verify_output_on_compile_fail_never_runs`,
    `plan__output_on_a_none_step_never_runs`
  - `plan__duplicate_capture_is_reported_at_the_second`,
    `plan__output_never_touches_a_tree`
  - `lock_text__records_outputs_line_by_line`
- **Normalization,** one test per rule:
  - `normalize__folds_crlf`, `__strips_ansi`, `__scrubs_the_tree_prefix`
  - `__drops_cargo_status_lines`, `__drops_cargo_summary_lines`
  - `__strips_test_timings`, `__sorts_parallel_test_lines`
  - `__keeps_a_warning_only_step`, `__keeps_the_spans_and_notes`
- **Capture helpers:**
  - `drift__names_the_first_differing_line`,
    `drift__a_shorter_live_output_is_drift`
  - `error_codes__in_order_once_each`
  - `rewrite__replaces_only_the_fence_body`,
    `rewrite__widens_the_fence_when_output_holds_backticks`,
    `rewrite__fills_an_empty_fence`
- **Properties:** `normalize_is_idempotent`,
  `normalize_ignores_ansi_and_crlf`, `rewrite_round_trips_through_plan` (the
  bound `lines` equal `normalize(raw)`),
  `rewrite_touches_nothing_outside_the_fence`.
- **Verify:**
  - `output__a_match_upholds_the_step`
  - `output__drift_fails_verify_and_names_the_line`
  - `output__an_empty_fence_is_not_recorded_and_fails`
  - `output__a_broken_step_is_unjudged_and_never_recorded`
  - `record__rewrites_the_chapter_and_the_lock`,
    `record__is_a_no_op_when_nothing_changed`
  - `record__refuses_a_chapter_path_that_climbs_out`
- **Goldens:**
  - `upholds_the_two_deliberate_failures`, extended to assert both outputs
    match.
  - `reports_a_drifted_diagnostic_as_broken`: change one character of the
    recorded E0308 in a copied book (`copy_dir`, `verification.rs:43`). This
    is the test that proves the check can say no, as EPIC-02 Phase 4c did for
    `expect`.
  - `replay__recording_an_output_moves_no_sha`
- **Render:**
  - `render__output_block_keeps_its_fence_and_gains_a_caption`
  - `render__error_code_links_through_the_template`
  - `render__no_template_no_link`
  - `render__epub_caption_matches_html`

## Key Files

| File | Role |
|---|---|
| `bower-core/src/directive.rs`, `block.rs` | `Capture`, the key, output blocks, fence helpers |
| `bower-core/src/plan.rs` | partition, `bind_outputs`, `CapturedOutput`, lock text |
| `bower-core/src/capture.rs` | new: `normalize`, `drift`, `error_codes`, `rewrite` |
| `bower-core/src/lib.rs` | five variants, `location()` arms, prelude |
| `bower-testkit/src/{textures,fixtures,coverage}.rs`, `tests/properties.rs` | textures, fixtures, the axis, properties |
| `bower/src/verify.rs`, `main.rs` | `stdout`, `OutputVerdict`, `Scrub`, `--record`, lock rewrite |
| `bower/src/config.rs`, `render.rs` | `toolchain`, `links.error_code`, the caption |
| `books/*/src/*.md` | the recorded outputs |

## Reuse (do NOT recreate)

- `plan.rs:241` `StepIndex::locate` and `plan.rs:129-131`: one binding rule
  and one partition for every non-tree block.
- `block.rs:127`, `block.rs:159`: fence scanning. `rewrite` uses these.
- `verify.rs:232` `verdict_for`: unchanged. Outputs are judged beside it.
- `materialize.rs:87` `check_path`: every chapter write goes through it
  (`docs/DEFECT_Path_Traversal.md`).
- `main.rs:237-241`: the lock write `--record` repeats.
- `config.rs:104-109`: the commands the caption prints.
- `render.rs:548` `subst`: no template, no link.

## Compatibility

- **Preserves** every existing book. With no `output=`, the plan data, lock,
  render, and verify output are all unchanged. No SHA moves, even when
  recording (Decision 9).
- **Adds** one key, five `BowerError` variants (`#[non_exhaustive]`,
  `lib.rs:62`), one kernel module, one flag, two config keys, and one CSS
  class.
- **Breaks** nothing in purity. `make purity` stays green, and the ANSI
  scanner and code extractor are hand-written.

## Dependencies

- **Built on:** EPIC-02 (the verifier, its two commands, `verdict_for`), and
  EPIC-03 and EPIC-06 (the one `render::chapter`). Also the play-cell binding
  rule (spec § 15.1) and the exercises design (its error shapes).
- **Blocks:** EPIC-12 (back matter), whose failure index folds
  `PlannedStep::outputs` through `capture::error_codes`.
- **Related:**
  - EPIC-09: outputs bind on any line, so an abandoned branch shows its
    diagnostic, and both EPICs bind through `StepIndex::locate`.
  - EPIC-13: an exercise box on a `compile_fail` step sits under the recorded
    diagnostic.
  - EPIC-14: `bower follow check` can diff a reader's output through
    `normalize`.
  - EPIC-16: B4's toolchain board is this drift check run per toolchain.

## Verification

```bash
make fmt-check
make test
make lint                      # clippy pedantic + make purity
make slow                      # the #[ignore]d 20-step sweep
cargo run -p bower -- --book books/hello-playbook verify
cargo run -p bower -- --book books/hello-playbook verify --record --step wont-compile
git diff --stat books/         # only fence bodies and bower.lock
cargo run -p bower -- --book books/hello-playbook status -o target/hello-playbook
make book && make epub
make failures
```

Exit criteria:

1. `bower verify` on `hello-playbook` reports every step `ok`, with both
   outputs matched, and exits `0`.
2. After one character of a recorded diagnostic changes, `bower verify` exits
   non-zero and names the chapter, the block, and the first differing line.
3. After `--record`, a second `--record` writes nothing, and `status` reports
   the lock in sync.
4. Two machines with different scratch paths, terminals, and thread
   scheduling record identical text on one toolchain.
5. `bower build` gives identical SHAs for every tag before and after
   `--record`.
6. `make failures` fails when the pinned toolchain rewords E0004, which makes
   `slow.yml:130-133` true.
7. `cargo tree -p bower-core -e normal` still prints one line.

---

## Open questions — decisions, not code

| # | Question |
|---|---|
| 1 | **Match granularity (ideas Part F Q3).** Full rendering, exact, is the default (Decision 5), yet both hand-pastes quote only the headline (`ch03-rank.md:102`, `ch01-local_development.md:264-267`). The options: the idea's `match="code"` (a second key, against the one-key rule); `output="check:code"` (one key with two meanings); or a **prefix rule**, where the recorded lines must be a prefix of the live output and `--record` preserves an author's trim. The prefix rule adds no key and keeps a trim honest. Lean: prefix. Decide before 2c. |
| 2 | **Which cargo lines are chatter?** Rule 5 drops `error: could not compile`, which is what a reader sees. Drop it, keep it, or keep only the last one? |
| 3 | **Toolchain source of truth.** `RUSTUP_TOOLCHAIN` overrides a tree's own `rust-toolchain.toml`. That is surprising in `hello-playbook`, which teaches the pin (`ch03-lints-and-format.md:19`). Should the key apply only to steps before the tree pins itself, or should verify refuse when the two disagree? |
| 4 | **Default error-code link.** Derive `links.error_code` when `check` starts with `cargo`, as `checkout` and `clone` are derived from `github` (`config.rs:92-103`)? Or keep it declared-only? |
| 5 | **Test order.** Rule 7 sorts `test … ok` runs. Alternatively, require `-- --test-threads=1` on repos with `output="verify"`: honest, but slower. |
| 6 | **`--record` on a dirty chapter.** Verify never touches git (`verify.rs:7-9`). Should `--record` refuse on a dirty chapter (a read-only `git status`), or trust `git diff`? |
| 7 | **Non-Rust books (C1).** The chatter table is cargo-shaped. A per-repo `normalize` preset (`cargo` \| `none`) keeps the kernel language-agnostic. Not before a second language exists. |

---

*Drafted 10 September 2026 against `ImperialBower/bower` @ `4ad21bf` ("docs:
file the forge design and add it to the backlog"). No spike; the design
reuses the EPIC-02 verifier and the play-cell binding rule unchanged.*
