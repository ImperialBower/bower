# EPIC-13: Exercises — hand the reader the failing state (EXR)

## Summary

- **Builds:** `tests=`, `solution=`, and `reveal="never"` on an exercise, so it
  names the tests the reader turns green and can hide its answer.
- **Why:** any failing test passes as the exercise, a solution that deletes the
  test passes as a solution, and the answer is always printed in full.
- **Shape:** `Exercise` gains `tests`, `named`, `Reveal`; `PlannedStep` gains
  `solves`; `verify` gains `Outcome::stdout` and a pure `named_tests_verdict`.
- **Proves it:** the sample step is upheld only because its one named test
  failed, and a copy with a second failing test is `Broken`.
- **Status:** Planned; the base exercises shipped in PR #4 (`fdcd09b`,
  6 September 2026), this EPIC's work unstarted.

---

## Context

Exercises already ship. They landed in PR #4 (`fdcd09b`, 6 September 2026)
against `docs/superpowers/specs/2026-09-06-exercises-design.md` (the
**exercises spec**), without an EPIC number. What exists:

- `exercise="…"` is a directive key (`bower-core/src/directive.rs:158`,
  parsed at `:228`). It has two forms. The **key form** rides on a tree
  block. The **block form** is a directive with no tree keys and a markdown
  fence, classified at `bower-core/src/block.rs:251` by the play-cell test
  `carries_tree_keys` (`block.rs:195`).
- `Exercise { loc, form, at, task, detail, answer }` sits on
  `PlannedStep::exercise` (`bower-core/src/plan.rs:55`, `:71`).
  `bind_exercises` (`plan.rs:291`) binds both forms. The answer is **always**
  `planned[idx + 1]` (`plan.rs:336`), the next step of the same repo.
- Five plan-time errors, `ExerciseUnbound` through `ExerciseConflictingKeys`
  (`bower-core/src/lib.rs:165–175`). One broken fixture each
  (`bower-testkit/src/fixtures.rs:519`). Coverage has an exercise × expect axis
  (`bower-testkit/src/coverage.rs:83`).
- `lock_text` prints `exercise form=… answer=… task=…` under the step
  (`plan.rs:390`). `STEPS.md` appends `· exercise: <task>`
  (`bower/src/trailers.rs:93`).
- The box (`bower/src/render.rs:357`) prints the task, the detail, the fork,
  clone and checkout commands, and the reader's command
  (`reader_command`, `render.rs:396`). It ends with "The answer is the next
  step" (`closing_line`, `render.rs:407`). In HTML the answer step's blocks fold
  behind `<details class="step-answer">` (`folds_by_line`, `render.rs:615`).
  Print targets never fold (`fold_label`, `render.rs:497`).
- Both books use it. `books/hello-playbook/src/ch04-tests-and-failing-on-purpose.md:70`
  is already idea A4's shape: a `test_fail` step with
  `exercise="Make the test pass"`, answered by `test-that-passes` (`:108`).
  `books/rust4failures/src/ch03-rank.md:115` is a block form on a
  `compile_fail`.

So idea **A4** (`docs/bower-ideas.md:196–232`, Part E rank 3, Part F Q5) was
written as if none of this existed, and most of it now does. Both states are
tags. The answer folds under a toggle. The book prints the tests because they
are in the exercise step's blocks. Three parts of A4 are not built yet:

1. **The failure is not controlled.** `verdict_for` upholds a `test_fail`
   when the verify command exits non-zero (`bower/src/verify.rs:261–270`), and
   any failure counts. A typo that stops a helper compiling in a test module
   would uphold it, and so would an unrelated test that breaks. Bower cannot
   tell *which* tests fail: `Outcome` keeps only stderr
   (`verify.rs:44–49`), and libtest writes its `test … FAILED` lines to stdout,
   which `run` drops (`verify.rs:311–315`).
2. **The solution is not checked as a solution.** The answer step is verified
   against its own `expect` like any other step. An answer that *deletes* the
   failing test passes `cargo test`, and Bower upholds it.
3. **The answer cannot be named or hidden.** `solution=` does not exist, so
   the answer has to be the adjacent step. `BACKLOG.md:88–89` lists two gaps
   under exercises spec § 2:
   - **(a)** there is no place for a *reader's* answer;
   - **(b)** there is no hidden solution: the answer is always the next step,
     and the book always prints it.

This EPIC closes 1, 2, and gap **(b)**. It defers gap **(a)** (Decision 10).

**This EPIC does not** grade a reader's attempt, host submissions, or link
to a Discussion. It does not check a failure's *reason* beyond which tests
fail: matching a panic message or a rustc error is EPIC-11's captured
diagnostics. It does not add a tag namespace, a new `op`, or a new `expect`.
It does not build `bower follow` (EPIC-14). It leaves every existing exercise
alone: a book without the three new keys renders, locks, and replays
byte-identically.

---

## Status

| Component | Status |
|---|---|
| Exercise key, two forms, binding, five errors, box, HTML fold, lock, `STEPS.md` marker | **Complete** (PR #4, `fdcd09b`; pre-existing, not this EPIC's work) |
| Kernel: `tests=`, `solution=`, `reveal=` keys; `Exercise` fields; `PlannedStep::solves` | **Planned** |
| Kernel: eight plan-time errors, textual test-presence check | **Planned** |
| Kernel: lock text for the new keys (only when present) | **Planned** |
| Testkit: fixtures, three mechanisms, three properties | **Planned** |
| Verify: `Outcome::stdout`, `libtest_results`, the named-tests verdict | **Planned** |
| Replay: `Bower-Exercise-Tests`, `Bower-Solution`, `Bower-Solves` trailers; `STEPS.md` | **Planned** |
| Render: tests line, honest closing line, `reveal="never"` on every target | **Planned** |
| Books: `tests=` on hello-playbook ch04; one hidden solution in a sample chapter | **Planned** |

---

## Goals

- Make an exercise's starting state a **controlled failure**: the named
  tests fail, no other test fails, and `bower verify` checks both.
- Make the **solution** a checked claim: every named test runs and passes
  there, and a deleted test is an error.
- Let the author **name** the solution step (`solution=`) and **hide** it
  (`reveal="never"`). The solution stays in the repo, tagged and verified, but
  appears on no page.
- Put the exercise's contract **in the generated repo** (trailers), so that
  EPIC-14's `bower follow check` can read it without the book.
- Keep the kernel **pure**. The kernel checks that the names exist in the
  tree. Only the verifier checks that the tests fail and pass.

## Scope

### The kata

- **Things.** The *exercise* (shipped), the *named test*, the *solution*
  step, the *reveal* mode, and the *named-tests verdict*.
- **Business requirements.** The rules below.
- **Business logic.** § Design and § Work Items.

### Authoring — three keys, all on the exercise's own directive

| Key | Meaning |
|---|---|
| `tests="a,b"` | The tests the reader turns green. The exercise step must be `expect="test_fail"`. Exactly these tests fail there. |
| `solution="step-id"` | The step that answers the exercise, when it is not the next one. It must come later and be `pass`. Defaults to the next step, as today. |
| `reveal="never"` | The solution is tagged and verified but appears on no page. Default `fold`, which is today's behaviour. |

```markdown
<!-- bower repo="hello-playbook" step="test-that-fails" file="src/lib.rs" op="region" region="tests" expect="test_fail" exercise="Make the test pass" tests="greet_ignores_stray_whitespace" -->
```

A hidden solution, somewhere after the exercise:

```markdown
<!-- bower repo="failers" step="exercise-from-char" file="tests/rank.rs" expect="test_fail" exercise="Make Rank::from total" tests="from_char_is_total" solution="solution-from-char" reveal="never" -->
…
<!-- bower repo="failers" step="solution-from-char" file="src/rank.rs" op="region" region="from_char" -->
```rust
// in the repo at step-NNN-solution-from-char; on no page
```
```

### Rules

1. `tests=`, `solution=`, and `reveal=` appear only on a directive that
   carries `exercise=`.
2. An exercise with `tests=` is on a `test_fail` step.
3. At the exercise step every named test is defined in the tree. At the
   solution step every named test is still defined.
4. The solution comes after the exercise, in the same repo.
5. An exercise with `tests=` or `solution=` has a `pass` solution.
6. A step answers at most one exercise.
7. At verify time the exercise step's failed tests are **exactly** the named
   ones, and each of them ran. At the solution step each named test ran and
   passed.
8. The box never calls a step "the next step" unless it is the next step.

### What the reader gets

- A box that names the tests to turn green.
- A solution that the verifier has proven solves *these* tests. For a
  hidden one, the box gives the solution tag, a browse link, and
  `git diff <exercise-tag> <solution-tag>`.
- In the generated repo: trailers on both commits, and a `STEPS.md` row that
  names the solution.

### Not in scope

Reader answers (gap a), grading, per-reason matching (EPIC-11), a
solutions appendix in print (EPIC-12, open question 3), solutions on a
branch (EPIC-09, open question 4), and test runners other than libtest's
human output (open question 2).

---

## Decisions

1. **Extend, don't replace. This answers Part F Q5.** The idea offered
   `exercise = true` plus `solution =`, or an `op = "exercise"`. Neither
   ships. `exercise="task"` already means "this is an exercise", and it
   carries the words the box prints. `solution=` is the idea's key, kept. No
   new `op`, and no new `expect`: a controlled failure is a `test_fail` with
   names attached, so it uses the same verification path unchanged, which is
   Q5's own argument for the key form.
2. **`tests=` turns "some test failed" into "these tests failed, and only
   these".** A test the author did not name must not fail: the reader would
   be handed a failure the book never mentions. A named test must not pass,
   or there is nothing for the reader to do. Names match libtest's full path
   or any `::`-suffix of it, so `greet_ignores_stray_whitespace` matches
   `tests::greet_ignores_stray_whitespace`.
3. **A solution must still run the named tests.** Upholding the solution
   step as `pass` is not enough. Each named test must appear in the solution
   step's output as `ok`. Deleting the test is not a solution. This is caught
   at plan time too, by `SolutionDropsTest` (Decision 4).
4. **The plan-time presence check is textual, and it is only a floor.** The
   kernel has trees and no processes. `TreeState::defines_fn` looks for
   `fn <last segment>` in some `.rs` file of the tree, using
   `paths`/`text` (`bower-core/src/tree.rs:47`, `:39`). It catches typos and
   deleted tests before a compiler runs. It cannot see macro-generated tests
   (open question 1). The verifier has the final say.
5. **`solution=` defaults to the next step. When named, it must come later
   and be green.** Today's rule (`plan.rs:336`) stays the default, so no
   existing book changes. A named solution may have steps between it and
   the exercise. The rule is order in the repo (`seq`), not document order.
   EPIC-09 turns this into an ancestry check: the solution must descend from
   the exercise on the parents DAG. EPIC-09 Decision 11 already makes the
   *default* answer follow the line.
6. **A hidden solution is still a step. That is how this EPIC closes gap
   (b).** The backlog asks for "an answer the book does not print". An
   answer that is not a tree is not verified, and Bower does not publish
   unverified claims. So `reveal="never"` hides the step's *text* on every
   target and changes nothing in the repo. Its directives and fences are
   consumed the way a block-form exercise's fence already is
   (`skip_directive_fence`, `render.rs:527`). Its anchor stays, so its
   `Book-Url` trailer still resolves. What the book continues with is the
   author's business: a later `op="region"` may build on code the reader
   never saw.
7. **Tags are unchanged.** The idea's `step-031-exercise-from-char` is an
   ordinary step tag (`PlannedStep::tag`, `plan.rs:108`) whose author chose
   the id. An `exercise/<id>` alias namespace would grow `expected_tags`
   (`bower/src/replay.rs:351`), `status` drift, and `push`, for a name the
   author can already choose.
8. **Trailers only for strict exercises.** Exercises spec § 5 says commit
   messages do not change for exercises. That stays true for every exercise
   without `tests=` or `solution=`, so existing books keep their SHAs. A
   strict exercise's commit gains `Bower-Exercise-Tests:` and
   `Bower-Solution:`, and its solution's commit gains `Bower-Solves:`. This
   is the repo-side contract EPIC-14 reads: its idea says `follow` is "driven
   by the generated repo's own metadata" (`bower-ideas.md:240`).
9. **The verifier learns stdout, and reads only libtest.** `Outcome` gains
   `stdout`. `libtest_results` parses `test <name> ... ok|FAILED|ignored`.
   When a verify command prints no line Bower can read (nextest, `make
   test`), the verdict is `Broken` with "its verify command printed no test
   results Bower can read". Bower does not guess. `cargo test` stops at the
   first failing test binary, so a named test in a later binary never runs.
   That is also `Broken`, and the hint names `--no-fail-fast`. EPIC-11 wants
   the same `stdout`, and whichever EPIC ships first adds it.
10. **Gap (a), the reader's answer, is deferred, and it splits in two.**
    *Checking* a reader's answer is EPIC-14's `bower follow check`. It runs
    this EPIC's named-tests verdict against the reader's tree, read from the
    trailers of Decision 8, and it is the only place a reader's answer is
    ever judged. *Discussing* a reader's answer, the per-exercise Discussion
    link, waits for `bower-ideas.md` Part F Q8 (`:850`, "any reader signal at
    all?", default no). `BACKLOG.md:88` stays open, and its note should point
    to both.
11. **The closing line tells the truth about distance.** `closing_line`
    (`render.rs:407`) says "the next step" whatever the answer is. Once
    `solution=` exists, that could be false. A non-adjacent solution is named
    by its step number and linked to its `#step-<id>` anchor. A hidden one
    is named by its tag.

---

## Domain map

| Domain concept | Code construct | Status |
|---|---|---|
| An exercise, two forms | `Exercise`, `ExerciseForm` `plan.rs:71`, `:90` | ✅ shipped |
| Its answer | `Exercise::answer` `plan.rs:84`, bound at `plan.rs:336` | 🟡 becomes `solution=` or next |
| The box | `exercise_box` `render.rs:357` | 🟡 tests line, closing line, hidden form |
| The fold | `folds_by_line` `render.rs:615` | 🟡 skips `reveal="never"` |
| Named tests | `Exercise::tests` | 🔴 new |
| Which step solves what | `PlannedStep::solves` | 🔴 new |
| Reveal mode | `Reveal { Fold, Never }` | 🔴 new |
| "Is this test in the tree?" | `TreeState::defines_fn` | 🔴 new, pure |
| What a test run said | `Outcome::stdout`, `libtest_results` | 🔴 new |
| Which tests failed, and why that matters | `named_tests_verdict` beside `verdict_for` `verify.rs:232` | 🔴 new |
| Repo-side contract | `Bower-Exercise-Tests`, `Bower-Solution`, `Bower-Solves` in `commit_message` `trailers.rs:19` | 🔴 new |
| Coverage | `Mechanism` `coverage.rs:16` | 🟡 three mechanisms |

---

## Design

### Kernel — `bower-core`

`Directive` (`directive.rs:138`) gains `tests: Vec<String>` (comma-split, like
`paths`), `solution: Option<String>`, and `reveal: Option<Reveal>` (`BadValue`
for anything but `fold`/`never`). `overlaid_on` (`directive.rs:266`) carries
all three. `Block` (`block.rs:25`) carries them through. `carries_tree_keys`
(`block.rs:195`) does **not** learn them, so a block-form exercise with
`tests=` is still a block form.

```rust
pub struct Exercise {
    // loc, form, at, task, detail, answer — unchanged
    /// `tests="…"`: libtest names the reader turns green. Empty = none named.
    pub tests: Vec<String>,
    /// `answer` came from `solution=` rather than the next-step default.
    pub named: bool,
    pub reveal: Reveal,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Reveal { #[default] Fold, Never }

impl Exercise {
    /// Carries trailers and a `pass` requirement: `tests=` or `solution=`.
    #[must_use]
    pub fn strict(&self) -> bool { !self.tests.is_empty() || self.named }
}

// PlannedStep gains:
/// The exercise this step answers, if any — the inverse of `answer`.
pub solves: Option<StepId>,
```

`bind_exercises` (`plan.rs:291`) resolves `solution=` through
`StepIndex::by_id` (`plan.rs:210`) and falls back to `idx + 1`. It runs the
rule checks after all steps are planned and sets `solves` on the answer.
`TreeState::defines_fn(&self, name: &str) -> bool` sits in `tree.rs` and is
pure string work over materialized text.

New `BowerError` variants (`lib.rs:63`). Each carries the exercise's
`Location` and follows the `Exercise*` naming:

| Variant | Rule |
|---|---|
| `ExerciseKeyWithoutExercise { loc, key }` | 1 |
| `ExerciseTestsNeedTestFail { loc, step, expect }` | 2 |
| `ExerciseTestAbsent { loc, step, test }` | 3 |
| `SolutionDropsTest { loc, solution, test }` | 3 |
| `SolutionUnknownStep { loc, step }` | 4 |
| `SolutionNotAfterExercise { loc, exercise, solution }` | 4 |
| `SolutionNotGreen { loc, solution, expect }` | 5 |
| `SolutionShared { loc, solution, first }` | 6, reported at the second |

`lock_text` (`plan.rs:369`) extends the exercise line **only** when a new key
is present, so every existing lock is byte-identical:

```
    exercise form=key answer=solution-from-char task="Make Rank::from total" tests=from_char_is_total solution=named reveal=never
```

### Testkit — `bower-testkit`

`exercise_solutions()` joins `valid()`. It has one strict exercise with
`tests=` and an adjacent solution, and one with a distant named solution and
`reveal="never"`. `broken_exercises()` (`fixtures.rs:519`) gains one fixture
per new variant, and `every_broken_fixture_produces_its_named_error`
(`bower-testkit/tests/corpus.rs:23`) holds them to it. `Mechanism`
(`coverage.rs:16`) gains `NamedTests`, `NamedSolution`, and `HiddenSolution`,
so `corpus_state_coverage_is_complete` (`corpus.rs:37`) fails until the corpus
visits each one.

Properties over `arb_book`-style generated books with exercises:

- every `answer`'s `seq` is larger than its exercise step's;
- `solves` is exactly the inverse of `answer`;
- a strict exercise's answer is `pass`.

### Verify — `bower`

```rust
pub struct Outcome { pub command: String, pub success: bool,
                     pub stdout: String, pub stderr: String }

pub enum TestResult { Ok, Failed, Ignored }

/// libtest's human output, one `test <name> ... <result>` line per test.
/// Doc tests (`src/lib.rs - f (line 3)`) are names like any other.
#[must_use]
pub fn libtest_results(stdout: &str) -> BTreeMap<String, TestResult>;

pub enum Role<'a> { Exercise(&'a [String]), Solution(&'a [String]) }

/// Rule 7, as one pure function. Called only after `verdict_for` upheld.
#[must_use]
pub fn named_tests_verdict(role: Role<'_>, verify: &Outcome) -> Verdict;
```

`Verifier::run` (`verify.rs:145`) works out each step's role from the plan.
The role is `Exercise` when the step's exercise names tests, and `Solution`
when its `solves` names one that does. A step never has both roles, because
Rule 2 and Rule 5 require opposite `expect`s. `--step solution-from-char` looks
up the exercise's names in the same plan. It needs no extra run. The `happened`
strings are "test `x` never ran", "test `x` passed, so there is nothing for the
reader to do", "test `y` failed too, and the exercise does not name it", and
the two from Decision 9. `main.rs` prints them like any other `Broken`
(`bower/src/main.rs:359–375`), and when a named test is the reason it prints
the stdout tail, not the stderr tail.

### Replay — `bower`

`commit_message` (`trailers.rs:19`) receives the plan's view of the step.
When the step's exercise is `strict()`, it adds `Bower-Exercise-Tests: a, b`
(omitted when `tests` is empty) and `Bower-Solution: <solution tag>`. On the
answer to a strict exercise it adds `Bower-Solves: <exercise tag>`. The
call site is `replay.rs:136`. `steps_md` (`trailers.rs:93`) adds
`· solves: <exercise tag>` to a solution's row. A hidden solution's row
reads the same as any other: the repo hides nothing.

### Render — `bower`

`exercises_by_line` (`render.rs:596`) also carries the answer's
`&PlannedStep`, so the box can see its `seq` and tag. `exercise_box`
(`render.rs:357`):

- adds `The tests to turn green: \`a\`, \`b\`.` after the detail when
  `tests` is non-empty;
- `closing_line` (`render.rs:407`) keeps today's wording when the answer's
  `seq` is the exercise's plus one. Otherwise it says "The answer is
  [step 034](#step-solution-from-char)" (bare text in print);
- for `reveal="never"`: when the repo has remote templates, the console
  block gains `git diff <exercise-tag> <solution-tag>`, and the closing line
  becomes "One solution is tagged `step-034-solution-from-char` · [browse]",
  using the `tree` template (`config.rs:89`). Without templates it says the
  tag and gives no link.

`folds_by_line` (`render.rs:615`) skips `Reveal::Never`. A new
`hidden_by_line` lists every directive line of a never-revealed solution.
`chapter` (`render.rs:23`) emits the step's anchor and then consumes the
directive and its fence with `skip_directive_fence` (`render.rs:527`) on
**every** target: HTML, epub, and PDF. There is no footer and no checkout
line. `publish` goes through the same `render::chapter`, so the epub and PDF
paths need no code of their own.

---

## Work Items

### Phase 0 — Kernel

- [ ] **0a.** Directive keys `tests`, `solution`, `reveal`. Parse tests, the
  overlay, and `Block` fields. `ExerciseKeyWithoutExercise`.
- [ ] **0b.** `Exercise::{tests, named, reveal}`, `Reveal`, `strict()`,
  `PlannedStep::solves`. Resolve `solution=` in `bind_exercises` with
  `SolutionUnknownStep`, `SolutionNotAfterExercise`, and `SolutionShared`.
- [ ] **0c.** `ExerciseTestsNeedTestFail` and `SolutionNotGreen`.
- [ ] **0d.** `TreeState::defines_fn`, `ExerciseTestAbsent`, and
  `SolutionDropsTest`, with unit tests for a `#[test] fn` in a `mod tests`, a
  suffix path, and a non-`.rs` file that must not match.
- [ ] **0e.** Lock text for the new keys. Assert that the `exercise_forms`
  lock is unchanged byte for byte.
- [ ] **0f.** `make purity` still prints one line.

### Phase 1 — Testkit

- [ ] **1a.** `exercise_solutions()` fixture, and eight broken fixtures.
- [ ] **1b.** The three `Mechanism`s. Corpus coverage stays complete.
- [ ] **1c.** The three properties.

### Phase 2 — Verification

- [ ] **2a.** `Outcome::stdout`, captured in `run` (`verify.rs:289`). If
  EPIC-11 has already added it, reuse it.
- [ ] **2b.** `libtest_results` over recorded libtest output: pass, fail,
  ignored, doc tests, and two binaries with a name in both.
- [ ] **2c.** `named_tests_verdict`, one unit test per `happened` string.
  `Verifier::run` assigns roles.
- [ ] **2d.** `main.rs` prints the stdout tail for a named-test `Broken`.
- [ ] **2e.** Goldens in `bower/tests/verification.rs`, copying
  `reports_a_wrong_expectation_as_broken` (`:79`): a copy of the sample book
  where the named test is misspelled (plan-time error), one where a second
  test also fails (`Broken`), and one where the fix deletes the test
  (`SolutionDropsTest`).

### Phase 3 — Replay

- [ ] **3a.** Trailers on strict exercises and their solutions. Pass the step's
  solution and exercise tags to `commit_message`.
- [ ] **3b.** `STEPS.md` `· solves:` marker.
- [ ] **3c.** Determinism golden (`bower/tests/determinism.rs`): a book with
  no strict exercise has the same SHAs as before this EPIC, and two builds of
  `exercise_solutions` are identical.

### Phase 4 — Render

- [ ] **4a.** Tests line and the distance-aware closing line.
- [ ] **4b.** `reveal="never"`: `hidden_by_line`, consume on every target,
  and keep the anchor. The fold skips it.
- [ ] **4c.** Hidden-solution box: tag, browse, and `git diff`, with and without
  templates.
- [ ] **4d.** Preprocessor and publish tests (Test Plan).

### Phase 5 — Books and docs

- [ ] **5a.** `tests="greet_ignores_stray_whitespace"` on
  `books/hello-playbook/src/ch04-tests-and-failing-on-purpose.md:70`, and a
  sentence in the chapter's `exercise` paragraph (`:101`) saying what it
  buys.
- [ ] **5b.** One hidden solution in the sample book. The *Failures* chapter
  map decides whether `rust4failures` gets one now.
- [ ] **5c.** `bower-spec.md` § 3.2 key table (three rows) and § 6
  (the named-tests row). Add a corrigendum to the exercises spec, since
  § 2's "hidden solutions" and § 5's "commit messages do not change" are now
  qualified. Update `.okf/model/exercise.md`, `.okf/model/errors.md`, and the
  README § Exercises.
- [ ] **5d.** `BACKLOG.md`: close line 89 (gap b). Point line 88 (gap a) at
  EPIC-14 and Part F Q8. Mark rank 3's A4 half as this EPIC.
- [ ] **5e.** Flip Status rows. Append the corrigendum.

---

## Test Plan

Kernel: `exercise__solution_key_names_a_distant_answer`,
`exercise__default_answer_is_still_the_next_step`,
`exercise__solution_before_the_exercise_is_refused`,
`exercise__solution_in_another_repo_is_unknown`,
`exercise__tests_on_a_compile_fail_step_is_refused`,
`exercise__strict_solution_must_be_pass`,
`exercise__two_exercises_one_solution_is_refused`,
`exercise__named_test_absent_from_the_tree_is_refused`,
`exercise__solution_that_drops_the_test_is_refused`,
`exercise__tests_key_without_exercise_is_refused`,
`exercise__solves_is_set_on_the_answer`,
`tree__defines_fn_matches_the_last_path_segment_only_in_rust_files`,
`lock_text__new_keys_appear_only_when_present`.

Verify: `libtest__parses_ok_failed_ignored_and_doc_tests`,
`named__exercise_upheld_when_exactly_the_named_tests_fail`,
`named__an_unnamed_failure_breaks_the_exercise`,
`named__a_passing_named_test_breaks_the_exercise`,
`named__a_test_that_never_ran_is_broken_with_the_no_fail_fast_hint`,
`named__unreadable_output_is_broken_not_guessed`,
`named__solution_must_report_every_named_test_ok`,
`verification::upholds_the_named_test_exercise` (fast lane; it extends
`upholds_the_two_deliberate_failures`, `verification.rs:62`),
`verification::an_extra_failing_test_breaks_the_exercise`.

Replay and render: `trailers__strict_exercise_carries_tests_and_solution`,
`trailers__plain_exercise_message_is_unchanged`,
`steps_md__marks_a_solution_row`,
`render__a_distant_solution_is_named_not_called_next`,
`render__hidden_solution_leaves_no_code_on_any_target`,
`render__hidden_solution_keeps_its_anchor`,
`render__hidden_solution_box_prints_tag_browse_and_diff`,
`preprocessor::sample_book_names_the_tests_to_turn_green` (beside
`preprocessor.rs:281`),
`publish::epub_and_pdf_omit_a_hidden_solution` (beside `publish.rs:105`).

## Key Files

| File | Role |
|---|---|
| `bower-core/src/directive.rs`, `block.rs` | three keys, `Reveal` parse, carried on `Block` |
| `bower-core/src/plan.rs` | `Exercise` fields, `solves`, `bind_exercises`, lock text |
| `bower-core/src/tree.rs` | `defines_fn` |
| `bower-core/src/lib.rs` | eight error variants |
| `bower-testkit/src/{fixtures,coverage}.rs` | fixtures, mechanisms, properties |
| `bower/src/verify.rs` | `Outcome::stdout`, `libtest_results`, `named_tests_verdict`, roles |
| `bower/src/main.rs` | stdout tail for a named-test failure |
| `bower/src/trailers.rs`, `replay.rs` | three trailers, `STEPS.md` marker |
| `bower/src/render.rs` | tests line, closing line, `hidden_by_line` |
| `books/hello-playbook/src/ch04-tests-and-failing-on-purpose.md` | the first `tests=` |

## Reuse (do NOT recreate)

- `plan.rs:291` `bind_exercises` and `plan.rs:241` `StepIndex::locate`:
  `solution=` resolves through `by_id`. Do not add a second binding pass.
- `verify.rs:232` `verdict_for`: the matrix stays as it is. The named-tests
  verdict runs after it and never replaces it.
- `render.rs:527` `skip_directive_fence`: hiding a solution consumes
  directives exactly as a block-form exercise's fence is consumed.
- `render.rs:615` `folds_by_line`: already finds the answer by id, not by
  position, so a distant solution folds with no change.
- `config.rs:22–23` `DEFAULT_CHECK`/`DEFAULT_VERIFY`: the box's command and
  the verifier's stay one definition.
- `fixtures.rs:519` `broken_exercises`: the new variants join it. Do not add
  a second list.

## Compatibility

- **Preserves** every existing book byte for byte: plan, lock, commit
  messages, SHAs, and rendered pages. None of them uses the new keys, and
  every new output is gated on them.
- **Adds** three keys, eight errors, one `Exercise` enum, one `PlannedStep`
  field, three trailers, one `Outcome` field, and two pure verify functions.
- **Changes** `hello-playbook` once, in Phase 5a. Its step-011 commit gains
  trailers, so its SHAs change from step 011 on. It is live at
  `abstecker/hello-playbook`, so the next `make ship-hello-execute`
  force-pushes behind the marker gate, as every edit to a published book does.
- **Breaks** nothing in `bower-core`'s purity.

## Dependencies

- **Built on:** the shipped exercises (PR #4, exercises spec), EPIC-01
  (tags, trailers, `STEPS.md`), EPIC-02 (the verifier and its matrix), EPIC-03
  and EPIC-06 (one render path for every target).
- **Blocks:** EPIC-14 `bower follow`. `follow check` runs
  `named_tests_verdict` against a reader's tree, reading Decision 8's
  trailers. `--hands-on` (`bower-ideas.md:254`) generalises an exercise to
  any step. Idea C3 (classroom) sits on both.
- **Coordinates with:** EPIC-09. The ordering check becomes an ancestry
  check, and the default answer follows the line (EPIC-09 Decision 11).
  Whichever lands second rebases `bind_exercises`. EPIC-11 also needs
  `Outcome::stdout`, and whichever lands first adds it. EPIC-11 is also where
  "fails for the stated reason" gains a message, not just a name: an
  exercise's failing test output could be shown via its `output` directive.
- **Related:** EPIC-12, where a solutions appendix is back matter (open
  question 3). EPIC-15, where the scrubber shows every step's tree, so it
  must respect `reveal="never"` or it undoes it. EPIC-16, where a published
  edition pins hidden solutions with everything else, and no new work is
  expected.

## Verification

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
make purity
make plan                      # both locks regenerate; rust4failures' is unchanged
cargo run -p bower -- --book books/hello-playbook verify --step test-that-fails
cargo run -p bower -- --book books/hello-playbook build -o /tmp/hp
git -C /tmp/hp log -1 --format=%B step-011-test-that-fails
make book                      # the box names the test; the fold still works
make slow                      # the 20-step sweep, mdBook, epub, and pdf
```

Exit criteria:

1. `verify --step test-that-fails` upholds the step, and only because
   `greet_ignores_stray_whitespace` is the one test that failed.
2. A copy of the sample book with a second failing test in step 011 is
   `Broken` and names that test. A copy whose fix deletes the test is a
   plan-time `SolutionDropsTest` naming the test and the solution step.
3. A hidden solution's code appears in no HTML, epub, or PDF render. Its tag
   exists, it is verified `pass`, and the box names it.
4. No box calls a non-adjacent solution "the next step".
5. `bower.lock`, the commit messages, and the SHAs of `rust4failures` are
   unchanged by this EPIC.
6. `make purity` prints one line.

---

## Open questions — decisions, not code

| # | Question |
|---|---|
| 1 | **Macro-generated tests.** `rstest`, `test_case`, and `proptest!` can produce names that no `fn` defines, so `defines_fn` would reject them. Options: skip the plan-time check when a test name contains `::` plus a case suffix, add an escape key, or say that `tests=` names only what the text defines. Lean: the last, until a chapter needs it. |
| 2 | **Runners other than libtest.** `cargo nextest` and `make test` get the "cannot read" `Broken` of Decision 9. Should `[repos.<name>] test_format` be a declared parser choice, or does libtest stay the only one until a book asks? |
| 3 | **Solutions in print.** A folded solution in HTML prints in full in epub and PDF (`fold_label`, `render.rs:497`), right under its exercise. A `reveal="end"` that moves solutions to a chapter-end or appendix block is EPIC-12's back matter. Is it wanted? |
| 4 | **Solutions on a branch.** Once EPIC-09 exists, a hidden solution could live on a branch that never merges, so the book continues on main with its own fix. Should `solution=` accept a branch step, and does `Bower-Solves` then point across lines? |
| 5 | **Shared solutions.** Rule 6 refuses one step answering two exercises. A chapter with "try it two ways, here is one fix" would want it. Lift the rule when a chapter asks, and make `Bower-Solves` a list. |
| 6 | **Reader answers (gap a).** Decision 10 splits it: EPIC-14 checks, and Part F Q8 decides whether anything is discussed. If Q8 says yes, the link is a `discuss` template in `[repos.<name>.links]`. It is not decided here. |

---

*Drafted 10 September 2026 against `ImperialBower/bower` @ `b6bfd19`
("docs: file EPIC-09 branches, the ideas doc, and the spikes; fold them into
the backlog"). Exercises shipped in PR #4 (`fdcd09b`). No spike: the
kernel half is a binding-rule extension of shipped code, and the one unknown
(libtest's stdout format) is Phase 2b's first test.*
