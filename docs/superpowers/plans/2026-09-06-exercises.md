# Exercises Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Mark "your turn" points in a Bower book — a task on any step, optionally with ideas — and render a box that tells the reader how to fork, check out, and try it, with the next step folded as the answer.

**Architecture:** The kernel (`bower-core`) learns one new directive key, `exercise`, in two forms — a key on a tree block, or its own directive with a fenced markdown body — and binds each to a step with the play-cell rules. The CLI (`bower`) derives reader commands from `bower.toml`, prints the box under the step in every target, and folds the answer step's code in HTML. Nothing enters any repo tree; verification does not change.

**Tech Stack:** Rust 2024 edition workspace; `bower-core` (zero deps), `bower` CLI, `bower-testkit` (rstest, proptest); mdBook + pandoc + typst renderers already wired.

**Spec:** `docs/superpowers/specs/2026-09-06-exercises-design.md` — read it first; every task below cites its sections.

## Global Constraints

- **Git is the user's.** Never run a state-changing git command (`~/.claude/CLAUDE.md`). Every "Commit" step means: stop, print the exact `git add … && git commit -m "…"` line for the user, and wait.
- **Kernel purity.** `bower-core` gains no dependency and no I/O. `cargo tree -p bower-core -e normal` must still print one line (`make lint` checks it).
- **Errors, not surprises.** Every new `BowerError` variant carries a `Location`, is collected (never short-circuits), and has a `Display` arm. Variant names begin with `Exercise`; the corpus test matches on the Debug prefix.
- **Lints.** `#![warn(clippy::pedantic, clippy::unwrap_used, clippy::expect_used)]` in every crate; tests `#[allow(non_snake_case, clippy::unwrap_used)]`. Test names use the `subject__does_thing` double-underscore style.
- **Copy, verbatim** (spec § 6.1 / § 6.2): task line `**Your turn: {task}**`; closing lines by `expect`: `compile_fail`/`test_fail` → `Green means you did it. The answer is the next step.`, `pass` → `Keep it green. The next step shows one way.`, `none` → `The next step shows one way.`; fold labels `Show the answer` / `Show one way`; clone comment `# or your fork`; HTML classes `step-exercise` and `step-answer`.
- **Defaults.** `DEFAULT_CHECK = "cargo check"`, `DEFAULT_VERIFY = "cargo test"`, one definition in `bower/src/config.rs`.
- **Run tests per crate while working** (`cargo test -p bower-core`), and `make ayce` once at the end.

---

## File Structure

| File | Responsibility in this feature |
|---|---|
| `bower-core/src/directive.rs` | Parse the `exercise` key. |
| `bower-core/src/lib.rs` | Five `Exercise*` error variants; prelude exports `Exercise`, `ExerciseForm`. |
| `bower-core/src/block.rs` | Classify a directive as key form or block form; `Block.exercise`, `Block.exercise_block`. |
| `bower-core/src/plan.rs` | `Exercise`, `ExerciseForm`, `PlannedStep.exercise`; a shared `StepIndex` for binding play cells and exercises; `lock_text` records exercises. |
| `bower-core/src/step.rs` | One test helper gains the two new `Block` fields. |
| `bower-testkit/src/fixtures.rs` | `exercise_forms()` valid fixture; `broken_exercises()`. |
| `bower-testkit/src/coverage.rs` | `Mechanism::Exercise`; the `exercise × expect` axis. |
| `bower-testkit/tests/corpus.rs` | Binding test over the fixture. |
| `bower/src/config.rs` | `DEFAULT_CHECK`/`DEFAULT_VERIFY`; `LinkTemplates.{fork, clone, check, verify}`. |
| `bower/src/verify.rs` | Uses the moved constants. |
| `bower/src/render.rs` | `exercise_box`, the block-form consumption, the HTML fold. |
| `bower/src/trailers.rs` | `STEPS.md` marks exercise steps. |
| `bower/tests/preprocessor.rs`, `bower/tests/publish.rs` | Sample-book assertions. |
| `books/hello-playbook/src/ch04-…md`, `books/rust4failures/src/ch03-rank.md` | One exercise each; both `theme/step-meta.css` files style the box and fold. |
| `README.md`, `bower-spec.md`, `BACKLOG.md` | The key, the feature, the two backlog items. |

---

### Task 1: The `exercise` key, its errors, and block classification

**Files:**
- Modify: `bower-core/src/directive.rs` (struct `Directive`, `parse`, `overlaid_on`, tests)
- Modify: `bower-core/src/lib.rs` (`BowerError`, `location`, `Display`)
- Modify: `bower-core/src/block.rs` (struct `Block`, `resolve`, tests)
- Modify: `bower-core/src/step.rs:300-315` (a test-only `Block` literal)

**Interfaces:**
- Produces: `Directive.exercise: Option<String>`; `Block.exercise: Option<String>`; `Block.exercise_block: bool` (true for the block form, whose `op` is `Op::Prose`); `BowerError::{ExerciseUnbound, ExerciseUnknownStep, ExerciseDuplicate, ExerciseWithoutAnswer, ExerciseConflictingKeys}`.

- [ ] **Step 1: Write the failing directive test**

In `bower-core/src/directive.rs`, inside `mod directive_tests`, add:

```rust
    #[test]
    fn parse__exercise_is_a_plain_string_key() {
        let d = Directive::parse(
            "<!-- bower repo=\"failers\" exercise=\"Make this compile\" -->",
            &loc(),
        )
        .unwrap();
        assert_eq!(d.exercise.as_deref(), Some("Make this compile"));
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test -p bower-core parse__exercise_is_a_plain_string_key`
Expected: compile error, `no field `exercise` on type `Directive``.

- [ ] **Step 3: Add the key to `Directive`**

In the `Directive` struct, after `notebook`:

```rust
    /// `exercise="…"` — a "your turn" task on this step (the key form), or,
    /// with no tree keys at all, a block-form exercise whose fence is the
    /// detail. See `docs/superpowers/specs/2026-09-06-exercises-design.md`.
    pub exercise: Option<String>,
```

In `parse`, after the `"after" => …` arm:

```rust
                "exercise" => d.exercise = Some(value),
```

In `overlaid_on`, after the `notebook:` line:

```rust
            exercise: self.exercise.clone().or_else(|| defaults.exercise.clone()),
```

- [ ] **Step 4: Run it to see it pass**

Run: `cargo test -p bower-core parse__exercise_is_a_plain_string_key`
Expected: PASS.

- [ ] **Step 5: Add the five error variants**

In `bower-core/src/lib.rs`, in `enum BowerError` after `PlayCellConflictingKeys`:

```rust
    /// A block-form exercise has no preceding step of its repo to attach
    /// to, and names none explicitly.
    ExerciseUnbound { loc: Location },
    /// A block-form exercise names a step that does not exist.
    ExerciseUnknownStep { loc: Location, step: String },
    /// A step already carries an exercise; reported at the second one.
    ExerciseDuplicate { loc: Location, step: String },
    /// The exercise's step is the last of its repo: nothing follows it to
    /// be the answer.
    ExerciseWithoutAnswer { loc: Location, step: String },
    /// A play cell also declares `exercise=`. A cell is one thing or the
    /// other.
    ExerciseConflictingKeys { loc: Location },
```

In `location()`, extend the big `|` arm — replace

```rust
            | Self::PlayCellConflictingKeys { loc } => Some(loc),
```

with

```rust
            | Self::PlayCellConflictingKeys { loc }
            | Self::ExerciseUnbound { loc }
            | Self::ExerciseUnknownStep { loc, .. }
            | Self::ExerciseDuplicate { loc, .. }
            | Self::ExerciseWithoutAnswer { loc, .. }
            | Self::ExerciseConflictingKeys { loc } => Some(loc),
```

In `impl Display`, after the `PlayCellConflictingKeys` arm:

```rust
            Self::ExerciseUnbound { loc } => {
                write!(f, "{loc}: exercise has no preceding step to attach to")
            }
            Self::ExerciseUnknownStep { loc, step } => {
                write!(f, "{loc}: exercise names step `{step}`, which does not exist")
            }
            Self::ExerciseDuplicate { loc, step } => {
                write!(f, "{loc}: step `{step}` already carries an exercise")
            }
            Self::ExerciseWithoutAnswer { loc, step } => write!(
                f,
                "{loc}: step `{step}` is the last of its repo; no next step can be the answer"
            ),
            Self::ExerciseConflictingKeys { loc } => {
                write!(f, "{loc}: a play cell cannot also be an exercise")
            }
```

- [ ] **Step 6: Write the failing block tests**

In `bower-core/src/block.rs`, inside `mod block_tests`, add:

```rust
    #[test]
    fn extract__exercise_key_rides_on_a_tree_block() {
        let src = book(
            "<!-- bower repo=\"failers\" file=\"a.rs\" expect=\"compile_fail\" exercise=\"Make this compile\" -->\n```rust\nx\n```\n",
        );
        let (blocks, errors) = extract(&src, &catalog());
        assert!(errors.is_empty(), "{errors}");
        let b = &blocks[0];
        assert_eq!(b.exercise.as_deref(), Some("Make this compile"));
        assert!(!b.exercise_block);
        assert_eq!(b.op, Op::Create);
    }

    #[test]
    fn extract__exercise_without_tree_keys_is_the_block_form() {
        let src = book(
            "<!-- bower repo=\"failers\" exercise=\"Try a lookup table\" -->\n```markdown\n- one idea\n```\n",
        );
        let (blocks, errors) = extract(&src, &catalog());
        assert!(errors.is_empty(), "{errors}");
        let b = &blocks[0];
        assert!(b.exercise_block);
        assert_eq!(b.exercise.as_deref(), Some("Try a lookup table"));
        assert_eq!(b.op, Op::Prose);
        assert_eq!(b.content.lines, vec!["- one idea".to_string()]);
    }

    #[test]
    fn extract__block_form_needs_its_fence() {
        let src = book("<!-- bower repo=\"failers\" exercise=\"Try it\" -->\nprose instead\n");
        let (blocks, errors) = extract(&src, &catalog());
        assert!(blocks.is_empty());
        assert!(matches!(
            errors.0[0],
            BowerError::DirectiveWithoutBlock { .. }
        ));
    }

    #[test]
    fn extract__a_play_cell_cannot_also_be_an_exercise() {
        let src = book(
            "<!-- bower repo=\"failers\" notebook=\"play\" exercise=\"Try it\" -->\n```python\nx\n```\n",
        );
        let (blocks, errors) = extract(&src, &catalog());
        assert!(blocks.is_empty());
        assert!(matches!(
            errors.0[0],
            BowerError::ExerciseConflictingKeys { .. }
        ));
    }
```

- [ ] **Step 7: Run them to see them fail**

Run: `cargo test -p bower-core extract__exercise extract__block_form extract__a_play_cell`
Expected: compile errors on `exercise` / `exercise_block`.

- [ ] **Step 8: Classify in `resolve`**

In `bower-core/src/block.rs`, add to `struct Block` after `play`:

```rust
    /// `exercise="…"`: the task text, on either form.
    pub exercise: Option<String>,
    /// The block form: an `exercise` directive with no tree keys, whose fence
    /// is the exercise's detail. Like a play cell it never joins a step or
    /// touches a tree; [`crate::plan`] binds it.
    pub exercise_block: bool,
```

Add this free function above `resolve`:

```rust
/// Does the directive say anything about a repo tree? Play cells and
/// block-form exercises must not; every other directive must.
fn carries_tree_keys(d: &Directive) -> bool {
    d.op.is_some()
        || d.file.is_some()
        || d.region.is_some()
        || d.src.is_some()
        || !d.paths.is_empty()
}
```

In `resolve`, replace the section from `let play = directive.notebook.is_some();` through the end of the `if play { … } else { … }` with:

```rust
    let play = directive.notebook.is_some();
    let tree_keys = carries_tree_keys(&directive);
    let exercise_block = directive.exercise.is_some() && !tree_keys && !play;
    let op = directive.op.unwrap_or_default();

    if play {
        // A play cell is a notebook concern: it needs its code block and
        // must not carry anything that would touch a repo tree — nor be an
        // exercise, which is a different kind of aside.
        if tree_keys {
            errors.push(BowerError::PlayCellConflictingKeys { loc: loc.clone() });
        }
        if directive.exercise.is_some() {
            errors.push(BowerError::ExerciseConflictingKeys { loc: loc.clone() });
        }
        if content.is_none() {
            errors.push(BowerError::DirectiveWithoutBlock { loc: loc.clone() });
        }
    } else if exercise_block {
        // The fence *is* the exercise's detail. Without one there is nothing
        // the key form would not have said shorter.
        if content.is_none() {
            errors.push(BowerError::DirectiveWithoutBlock { loc: loc.clone() });
        }
    } else {
        if op.needs_file() && directive.file.is_none() && directive.paths.is_empty() {
            errors.push(BowerError::MissingKey {
                loc: loc.clone(),
                key: "file".to_string(),
            });
        }
        if op == Op::Region && directive.region.is_none() {
            errors.push(BowerError::MissingKey {
                loc: loc.clone(),
                key: "region".to_string(),
            });
        }
        if op == Op::Copy && directive.src.is_none() {
            errors.push(BowerError::MissingKey {
                loc: loc.clone(),
                key: "src".to_string(),
            });
        }
        if op.needs_block() && content.is_none() {
            errors.push(BowerError::DirectiveWithoutBlock { loc: loc.clone() });
        }
    }
```

In the `Some(Block { … })` literal, change the `op` line and add the two fields:

```rust
        // Neither a play cell nor a block-form exercise applies to a tree;
        // Prose is the inert op.
        op: if play || exercise_block { Op::Prose } else { op },
        …
        play,
        exercise: directive.exercise,
        exercise_block,
```

- [ ] **Step 9: Fix the test-only `Block` literal in `step.rs`**

Run: `grep -n "play: false" bower-core/src/step.rs` — one hit near line 309. Add directly after it:

```rust
            exercise: None,
            exercise_block: false,
```

- [ ] **Step 10: Run the kernel tests**

Run: `cargo test -p bower-core`
Expected: all PASS, including the four new block tests. (`plan.rs` compiles because it only reads `b.play`; the new fields are inert until Task 2.)

- [ ] **Step 11: Commit**

Tell the user to run, and wait:

```bash
git add bower-core/src/directive.rs bower-core/src/lib.rs bower-core/src/block.rs bower-core/src/step.rs && git commit -m "feat(core): the exercise key, its five errors, and block-form classification"
```

---

### Task 2: Bind exercises to steps in the plan

**Files:**
- Modify: `bower-core/src/plan.rs` (types, `plan`, `bind_play_cells` → shared `StepIndex`, new `bind_exercises`, `lock_text`, tests)
- Modify: `bower-core/src/lib.rs` (prelude)

**Interfaces:**
- Consumes: `Block.exercise`, `Block.exercise_block` (Task 1).
- Produces:
  ```rust
  pub struct Exercise { pub loc: Location, pub form: ExerciseForm, pub at: Location, pub task: String, pub detail: Vec<String>, pub answer: StepId }
  pub enum ExerciseForm { Key, Block }   // Display: "key" / "block"
  // PlannedStep gains: pub exercise: Option<Exercise>
  ```
  Lock text per exercise step, two indented line kinds under the step line:
  `    exercise form=<key|block> answer=<step id> task="<task, Debug-quoted>"` then `    | <detail line>` per detail line.

- [ ] **Step 1: Write the failing plan tests**

In `bower-core/src/plan.rs`, inside `mod plan_tests`, add:

```rust
    fn exercise_chapter() -> Chapter {
        Chapter::new(
            "ch02-exercise.md",
            concat!(
                "# Broken\n\n",
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" step=\"broken\" expect=\"compile_fail\" exercise=\"Make this compile\" -->\n",
                "```rust\nfn x() -> u32 { \"42\" }\n```\n\n",
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" op=\"replace\" step=\"fixed\" -->\n",
                "```rust\nfn x() -> u32 { 42 }\n```\n\n",
                "<!-- bower repo=\"failers\" exercise=\"Do it without a literal\" -->\n",
                "```markdown\nParse it instead.\n\n- `str::parse` is one way.\n```\n\n",
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" op=\"append\" step=\"last\" -->\n",
                "```rust\n// the end\n```\n",
            ),
        )
    }

    #[test]
    fn exercise__key_form_binds_to_its_own_step_and_the_next_is_the_answer() {
        let book = BookSource::from_chapters(vec![exercise_chapter()]);
        let p = plan(&book, &catalog()).unwrap();
        let steps = &p.repo("failers").unwrap().steps;
        let x = steps[0].exercise.as_ref().unwrap();
        assert_eq!(x.form, ExerciseForm::Key);
        assert_eq!(x.task, "Make this compile");
        assert!(x.detail.is_empty());
        assert_eq!(x.answer, StepId("fixed".to_string()));
        assert_eq!(x.loc, Location::new("ch02-exercise.md", 3));
        assert_eq!(x.at, Location::new("ch02-exercise.md", 3));
    }

    #[test]
    fn exercise__block_form_binds_to_the_nearest_preceding_step_with_its_detail() {
        let book = BookSource::from_chapters(vec![exercise_chapter()]);
        let p = plan(&book, &catalog()).unwrap();
        let steps = &p.repo("failers").unwrap().steps;
        let x = steps[1].exercise.as_ref().unwrap();
        assert_eq!(x.form, ExerciseForm::Block);
        assert_eq!(x.task, "Do it without a literal");
        assert_eq!(x.at, Location::new("ch02-exercise.md", 13));
        assert_eq!(
            x.detail,
            vec![
                "Parse it instead.".to_string(),
                String::new(),
                "- `str::parse` is one way.".to_string(),
            ]
        );
        assert_eq!(x.answer, StepId("last".to_string()));
        assert!(steps[2].exercise.is_none());
        // The block form's fence never reaches a tree.
        assert_eq!(
            steps[2].tree.text("src/lib.rs").unwrap(),
            "fn x() -> u32 { 42 }\n// the end\n"
        );
    }

    #[test]
    fn exercise__key_form_on_a_multi_block_step_renders_after_the_last_block() {
        let book = BookSource::from_chapters(vec![Chapter::new(
            "ch.md",
            concat!(
                "<!-- bower repo=\"failers\" file=\"a.rs\" step=\"two\" exercise=\"Try\" -->\n```rust\nx\n```\n",
                "<!-- bower repo=\"failers\" file=\"b.rs\" step=\"two\" -->\n```rust\ny\n```\n",
                "<!-- bower repo=\"failers\" file=\"c.rs\" step=\"next\" -->\n```rust\nz\n```\n",
            ),
        )]);
        let p = plan(&book, &catalog()).unwrap();
        let x = p.repo("failers").unwrap().steps[0].exercise.as_ref().unwrap();
        assert_eq!(x.loc.line, 1);
        assert_eq!(x.at.line, 5);
    }

    #[test]
    fn exercise__duplicate_is_reported_at_the_second_one() {
        let book = BookSource::from_chapters(vec![Chapter::new(
            "ch.md",
            concat!(
                "<!-- bower repo=\"failers\" file=\"a.rs\" step=\"one\" exercise=\"First\" -->\n```rust\nx\n```\n",
                "<!-- bower repo=\"failers\" exercise=\"Second\" -->\n```markdown\nmore\n```\n",
                "<!-- bower repo=\"failers\" file=\"b.rs\" step=\"two\" -->\n```rust\ny\n```\n",
            ),
        )]);
        let errs = plan(&book, &catalog()).unwrap_err();
        assert_eq!(errs.len(), 1, "{errs}");
        assert!(
            matches!(&errs.0[0], BowerError::ExerciseDuplicate { loc, step } if loc.line == 5 && step == "one"),
            "{errs}"
        );
    }

    #[test]
    fn exercise__on_the_last_step_has_no_answer() {
        let book = BookSource::from_chapters(vec![Chapter::new(
            "ch.md",
            "<!-- bower repo=\"failers\" file=\"a.rs\" step=\"only\" exercise=\"Try\" -->\n```rust\nx\n```\n",
        )]);
        let errs = plan(&book, &catalog()).unwrap_err();
        assert!(
            matches!(&errs.0[0], BowerError::ExerciseWithoutAnswer { step, .. } if step == "only"),
            "{errs}"
        );
    }

    #[test]
    fn exercise__block_form_errors_mirror_play_cells() {
        let unbound = BookSource::from_chapters(vec![Chapter::new(
            "ch.md",
            "<!-- bower repo=\"failers\" exercise=\"Try\" -->\n```markdown\nx\n```\n<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n",
        )]);
        let errs = plan(&unbound, &catalog()).unwrap_err();
        assert!(matches!(errs.0[0], BowerError::ExerciseUnbound { .. }), "{errs}");

        let ghost = BookSource::from_chapters(vec![Chapter::new(
            "ch.md",
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n<!-- bower repo=\"failers\" exercise=\"Try\" step=\"ghost\" -->\n```markdown\nx\n```\n",
        )]);
        let errs = plan(&ghost, &catalog()).unwrap_err();
        assert!(
            matches!(&errs.0[0], BowerError::ExerciseUnknownStep { step, .. } if step == "ghost"),
            "{errs}"
        );
    }

    #[test]
    fn lock_text__records_the_exercise_and_its_detail() {
        let book = BookSource::from_chapters(vec![exercise_chapter()]);
        let lock = lock_text(&plan(&book, &catalog()).unwrap());
        assert!(
            lock.contains("    exercise form=key answer=fixed task=\"Make this compile\"\n"),
            "{lock}"
        );
        assert!(
            lock.contains("    exercise form=block answer=last task=\"Do it without a literal\"\n"),
            "{lock}"
        );
        assert!(lock.contains("    | Parse it instead.\n    | \n    | - `str::parse` is one way.\n"), "{lock}");
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p bower-core exercise__ lock_text__records`
Expected: compile errors (`ExerciseForm`, `exercise` field).

- [ ] **Step 3: Add the types**

In `bower-core/src/plan.rs`, change `use crate::{BowerError, Errors, block};` to `use crate::block::Block;` plus `use crate::{BowerError, Errors, block};` (keep both — `block::extract` is still called by path). Add `use std::collections::BTreeMap;` at the top and delete the `use std::collections::BTreeMap;` inside `bind_play_cells`.

After `struct PlayCell`, add:

```rust
/// One "your turn" point, bound to a step. Never part of any tree. See
/// `docs/superpowers/specs/2026-09-06-exercises-design.md`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Exercise {
    /// The directive that declared it — where errors point.
    pub loc: Location,
    pub form: ExerciseForm,
    /// Where the box renders: the block form's own directive, or the key
    /// form's step's last block in book order.
    pub at: Location,
    /// The `exercise="…"` value: one imperative line.
    pub task: String,
    /// The block form's fenced body, verbatim markdown. Empty for the key
    /// form.
    pub detail: Vec<String>,
    /// The step that carries the answer: the next step of the same repo.
    pub answer: StepId,
}

/// How an exercise was written: a key on a tree block's directive, or its
/// own directive with a fenced body.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExerciseForm {
    Key,
    Block,
}

impl std::fmt::Display for ExerciseForm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Key => "key",
            Self::Block => "block",
        })
    }
}
```

In `struct PlannedStep`, after `play_cells`:

```rust
    /// The "your turn" point on this step, if the book declares one.
    pub exercise: Option<Exercise>,
```

- [ ] **Step 4: Partition and bind in `plan`**

Replace

```rust
    let (play_blocks, code_blocks): (Vec<_>, Vec<_>) = blocks.into_iter().partition(|b| b.play);
```

with

```rust
    let (play_blocks, rest): (Vec<_>, Vec<_>) = blocks.into_iter().partition(|b| b.play);
    let (exercise_blocks, code_blocks): (Vec<_>, Vec<_>) =
        rest.into_iter().partition(|b| b.exercise_block);
```

In the `PlannedStep { … }` literal add `exercise: None,` after `play_cells: Vec::new(),`.

Replace the line `bind_play_cells(&play_blocks, name, &ordered, &mut planned, &mut errors);` with:

```rust
        let index = StepIndex::new(&ordered, &planned);
        bind_play_cells(&play_blocks, name, &index, &mut planned, &mut errors);
        bind_exercises(
            &exercise_blocks,
            name,
            &ordered,
            &index,
            &mut planned,
            &mut errors,
        );
```

- [ ] **Step 5: Replace `bind_play_cells` with the shared index and the two binders**

Delete the whole existing `bind_play_cells` function and put this in its place:

```rust
/// Where a non-tree block (a play cell, a block-form exercise) attaches: step
/// ids, and the document position of every code block, for the two binding
/// rules of spec § 15.1 — an explicit `step=`, else the nearest preceding
/// code block of the repo.
struct StepIndex {
    by_id: BTreeMap<String, usize>,
    /// (document position of a code block) → index of its planned step,
    /// sorted by position.
    by_doc_pos: Vec<(usize, usize)>,
}

/// Why a non-tree block could not be bound; the caller names the error.
enum Unbindable {
    NoPrecedingStep,
    UnknownStep(String),
}

impl StepIndex {
    fn new(ordered: &[Step], planned: &[PlannedStep]) -> Self {
        let by_id: BTreeMap<String, usize> = planned
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id.0.clone(), i))
            .collect();
        let mut by_doc_pos: Vec<(usize, usize)> = Vec::new();
        for s in ordered {
            if let Some(&idx) = by_id.get(&s.id.0) {
                for b in &s.blocks {
                    by_doc_pos.push((b.seq_in_book, idx));
                }
            }
        }
        by_doc_pos.sort_unstable();
        Self { by_id, by_doc_pos }
    }

    fn locate(&self, explicit: Option<&str>, seq_in_book: usize) -> Result<usize, Unbindable> {
        match explicit {
            Some(id) => self
                .by_id
                .get(id)
                .copied()
                .ok_or_else(|| Unbindable::UnknownStep(id.to_string())),
            None => self
                .by_doc_pos
                .iter()
                .take_while(|(pos, _)| *pos < seq_in_book)
                .last()
                .map(|&(_, idx)| idx)
                .ok_or(Unbindable::NoPrecedingStep),
        }
    }
}

/// Attach each play block of `repo` to its step: the explicit `step=` when
/// given, else the step containing the nearest preceding code block of the
/// same repo in document order — you play with what you just built.
fn bind_play_cells(
    play_blocks: &[Block],
    repo: &RepoName,
    index: &StepIndex,
    planned: &mut [PlannedStep],
    errors: &mut Errors,
) {
    for p in play_blocks.iter().filter(|p| &p.repo == repo) {
        match index.locate(p.step.as_deref(), p.seq_in_book) {
            Ok(idx) => planned[idx].play_cells.push(PlayCell {
                loc: p.loc.clone(),
                info: p.content.info.clone(),
                lines: p.content.lines.clone(),
            }),
            Err(Unbindable::UnknownStep(step)) => errors.push(BowerError::PlayCellUnknownStep {
                loc: p.loc.clone(),
                step,
            }),
            Err(Unbindable::NoPrecedingStep) => {
                errors.push(BowerError::PlayCellUnbound { loc: p.loc.clone() });
            }
        }
    }
}

/// Attach every exercise of `repo` to its step, in document order so a
/// duplicate is reported at the second one. Key forms ride on their own
/// step's blocks; block forms bind exactly as play cells do. The answer is
/// the next step of the repo, so the last step cannot carry one.
fn bind_exercises(
    exercise_blocks: &[Block],
    repo: &RepoName,
    ordered: &[Step],
    index: &StepIndex,
    planned: &mut [PlannedStep],
    errors: &mut Errors,
) {
    // (document position, declaring block, form, bound step index)
    let mut found: Vec<(usize, &Block, ExerciseForm, Option<usize>)> = Vec::new();

    for (idx, s) in ordered.iter().enumerate() {
        for b in s.blocks.iter().filter(|b| b.exercise.is_some()) {
            found.push((b.seq_in_book, b, ExerciseForm::Key, Some(idx)));
        }
    }
    for b in exercise_blocks.iter().filter(|b| &b.repo == repo) {
        let target = match index.locate(b.step.as_deref(), b.seq_in_book) {
            Ok(idx) => Some(idx),
            Err(Unbindable::UnknownStep(step)) => {
                errors.push(BowerError::ExerciseUnknownStep {
                    loc: b.loc.clone(),
                    step,
                });
                None
            }
            Err(Unbindable::NoPrecedingStep) => {
                errors.push(BowerError::ExerciseUnbound { loc: b.loc.clone() });
                None
            }
        };
        found.push((b.seq_in_book, b, ExerciseForm::Block, target));
    }
    found.sort_by_key(|(pos, ..)| *pos);

    for (_, b, form, target) in found {
        let Some(idx) = target else { continue };
        let step_id = planned[idx].id.0.clone();
        if planned[idx].exercise.is_some() {
            errors.push(BowerError::ExerciseDuplicate {
                loc: b.loc.clone(),
                step: step_id,
            });
            continue;
        }
        let Some(answer) = planned.get(idx + 1).map(|s| s.id.clone()) else {
            errors.push(BowerError::ExerciseWithoutAnswer {
                loc: b.loc.clone(),
                step: step_id,
            });
            continue;
        };
        let at = match form {
            ExerciseForm::Block => b.loc.clone(),
            ExerciseForm::Key => ordered[idx]
                .blocks
                .iter()
                .max_by_key(|x| x.seq_in_book)
                .map_or_else(|| b.loc.clone(), |x| x.loc.clone()),
        };
        planned[idx].exercise = Some(Exercise {
            loc: b.loc.clone(),
            form,
            at,
            task: b.exercise.clone().unwrap_or_default(),
            detail: match form {
                ExerciseForm::Block => b.content.lines.clone(),
                ExerciseForm::Key => Vec::new(),
            },
            answer,
        });
    }
}
```

- [ ] **Step 6: Record exercises in `lock_text`**

In `lock_text`, inside the `for s in &repo.steps` loop, after the existing `writeln!` of the step line, add:

```rust
            if let Some(x) = &s.exercise {
                let _ = writeln!(
                    out,
                    "    exercise form={} answer={} task={:?}",
                    x.form, x.answer, x.task
                );
                for line in &x.detail {
                    let _ = writeln!(out, "    | {line}");
                }
            }
```

- [ ] **Step 7: Export from the prelude**

In `bower-core/src/lib.rs`, change the plan line of `pub mod prelude` to:

```rust
    pub use crate::plan::{
        BookPlan, Exercise, ExerciseForm, PlannedStep, PlayCell, RepoPlan, lock_text, plan,
    };
```

- [ ] **Step 8: Confirm no other `PlannedStep` literal exists**

Run: `grep -rn "play_cells: Vec::new()" bower bower-core bower-testkit`
Expected: exactly one hit, `bower-core/src/plan.rs`, which Step 4 already changed. If a second appears, add `exercise: None,` beside it.

- [ ] **Step 9: Run the kernel tests**

Run: `cargo test -p bower-core`
Expected: all PASS — the seven new tests and every existing one (`notebook_play` behaviour is unchanged by the refactor).

- [ ] **Step 10: Build the whole workspace**

Run: `cargo build --workspace --all-features && cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: clean. If clippy flags `needless_pass_by_value` or similar on `bind_exercises`, fix in place; do not `allow`.

- [ ] **Step 11: Commit**

Tell the user to run, and wait:

```bash
git add bower-core/src/plan.rs bower-core/src/lib.rs && git commit -m "feat(core): bind exercises to steps; the next step is the answer"
```

---

### Task 3: Testkit fixtures and the `exercise × expect` coverage axis

**Files:**
- Modify: `bower-testkit/src/fixtures.rs` (`exercise_forms`, `valid`, `broken_exercises`, `broken`)
- Modify: `bower-testkit/src/coverage.rs` (`Mechanism::Exercise`, `exercise_expects`)
- Modify: `bower-testkit/tests/corpus.rs`

**Interfaces:**
- Consumes: `Exercise`, `ExerciseForm`, `PlannedStep.exercise`, `lock_text` (Task 2).
- Produces: `fixtures::exercise_forms() -> Fixture`; `CoverageReport.exercise_expects: BTreeSet<String>`; `CoverageReport::missing_exercise_expects()`; `Mechanism::Exercise` (Display `exercise`).

- [ ] **Step 1: Write the failing corpus test**

In `bower-testkit/tests/corpus.rs`, after `notebook_play_binds_cells_to_steps`, add:

```rust
#[test]
fn exercise_forms_bind_both_forms_on_every_expect() {
    let f = fixtures::exercise_forms();
    let p = plan(&f.book, &f.catalog).unwrap();
    let steps = &p.repo("failers").unwrap().steps;
    let by_id = |id: &str| steps.iter().find(|s| s.id.0 == id).unwrap();

    let broken = by_id("broken").exercise.as_ref().unwrap();
    assert_eq!(broken.form, ExerciseForm::Key);
    assert_eq!(broken.answer.0, "fixed");

    // The positional block form binds to the step just built.
    let fixed = by_id("fixed").exercise.as_ref().unwrap();
    assert_eq!(fixed.form, ExerciseForm::Block);
    assert_eq!(fixed.detail[0], "Parse text instead.");
    assert_eq!(fixed.answer.0, "tested");

    // The explicit `step=` block form reaches back past `green`.
    let tested = by_id("tested").exercise.as_ref().unwrap();
    assert_eq!(tested.task, "Make the test pass");
    assert_eq!(tested.answer.0, "green");
    assert!(by_id("green").exercise.is_none());

    // A prose step can carry one too; its answer is whatever follows.
    assert_eq!(by_id("narrated").exercise.as_ref().unwrap().answer.0, "end");
    assert!(by_id("end").exercise.is_none());

    let lock = lock_text(&p);
    assert!(lock.contains("exercise form=key answer=fixed"), "{lock}");
    assert!(lock.contains("exercise form=block answer=tested"), "{lock}");
}
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test -p bower-testkit --test corpus exercise_forms`
Expected: compile error, `no function exercise_forms`.

- [ ] **Step 3: Add the valid fixture**

In `bower-testkit/src/fixtures.rs`, after `notebook_play()`:

```rust
/// Every exercise form on every `expect`: a key form on a `compile_fail`
/// step, a positional block form on a `pass` step, an explicitly bound block
/// form on a `test_fail` step, and a key form on a prose (`none`) step. The
/// last step carries none, because nothing follows it to be the answer.
#[must_use]
pub fn exercise_forms() -> Fixture {
    Fixture::new(
        "exercise_forms",
        BookSource::from_chapters(vec![chapter(
            "ch07-exercises.md",
            concat!(
                "# Your turn\n\n",
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" step=\"broken\" expect=\"compile_fail\" exercise=\"Make this compile\" -->\n",
                "```rust\npub fn answer() -> u32 { \"42\" }\n```\n\n",
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" op=\"replace\" step=\"fixed\" -->\n",
                "```rust\npub fn answer() -> u32 { 42 }\n```\n\n",
                "<!-- bower repo=\"failers\" exercise=\"Do it without a literal\" -->\n",
                "```markdown\nParse text instead.\n\n- `str::parse` is one way.\n```\n\n",
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" op=\"append\" step=\"tested\" expect=\"test_fail\" -->\n",
                "```rust\n#[test]\nfn answers() { assert_eq!(answer(), 41); }\n```\n\n",
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" op=\"replace\" step=\"green\" -->\n",
                "```rust\npub fn answer() -> u32 { 42 }\n#[test]\nfn answers() { assert_eq!(answer(), 42); }\n```\n\n",
                "<!-- bower repo=\"failers\" exercise=\"Make the test pass\" step=\"tested\" -->\n",
                "```markdown\nThe test is right. The function is not.\n```\n\n",
                "<!-- bower repo=\"failers\" op=\"none\" step=\"narrated\" expect=\"none\" msg=\"docs: a refactor the book only describes\" exercise=\"Try the refactor yourself\" -->\n\n",
                "<!-- bower repo=\"failers\" file=\"src/lib.rs\" op=\"append\" step=\"end\" -->\n",
                "```rust\n// fin\n```\n",
            ),
        )]),
        failers(),
    )
}
```

In `valid()`, insert `exercise_forms(),` after `notebook_play(),`.

- [ ] **Step 4: Add the broken fixtures**

After `broken_display()`:

```rust
/// Errors raised by exercises: binding, duplication, the missing answer, and
/// the play-cell collision.
fn broken_exercises() -> Vec<(&'static str, Fixture)> {
    vec![
        single(
            "ExerciseUnbound",
            "<!-- bower repo=\"failers\" exercise=\"Try\" -->\n```markdown\nx\n```\n<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n",
        ),
        single(
            "ExerciseUnknownStep",
            "<!-- bower repo=\"failers\" file=\"a.rs\" -->\n```rust\nx\n```\n<!-- bower repo=\"failers\" exercise=\"Try\" step=\"ghost\" -->\n```markdown\nx\n```\n",
        ),
        single(
            "ExerciseDuplicate",
            "<!-- bower repo=\"failers\" file=\"a.rs\" step=\"one\" exercise=\"First\" -->\n```rust\nx\n```\n<!-- bower repo=\"failers\" exercise=\"Second\" -->\n```markdown\nx\n```\n<!-- bower repo=\"failers\" file=\"b.rs\" -->\n```rust\ny\n```\n",
        ),
        single(
            "ExerciseWithoutAnswer",
            "<!-- bower repo=\"failers\" file=\"a.rs\" exercise=\"Try\" -->\n```rust\nx\n```\n",
        ),
        single(
            "ExerciseConflictingKeys",
            "<!-- bower repo=\"failers\" notebook=\"play\" exercise=\"Try\" -->\n```python\nx\n```\n",
        ),
    ]
}
```

In `broken()`, after `cases.extend(broken_display());` add `cases.extend(broken_exercises());`.

- [ ] **Step 5: Run the corpus tests**

Run: `cargo test -p bower-testkit --test corpus`
Expected: `exercise_forms_bind_both_forms_on_every_expect`, `every_valid_fixture_plans_cleanly`, `every_broken_fixture_produces_its_named_error` PASS. `corpus_state_coverage_is_complete` still passes (the axis does not exist yet).

- [ ] **Step 6: Write the failing coverage assertion**

In `bower-testkit/tests/corpus.rs`, replace `corpus_state_coverage_is_complete` with:

```rust
#[test]
fn corpus_state_coverage_is_complete() {
    let mut corpus = fixtures::valid();
    corpus.extend(fixtures::broken().into_iter().map(|(_, f)| f));
    let report = CoverageReport::measure(&corpus);
    assert!(report.is_complete(), "coverage gaps:\n{report}");
    // The exercise axis is a set of its own: an exercise on every `expect`.
    assert!(
        report.missing_exercise_expects().is_empty(),
        "coverage gaps:\n{report}"
    );
}
```

Run: `cargo test -p bower-testkit --test corpus corpus_state`
Expected: compile error, `no method missing_exercise_expects`.

- [ ] **Step 7: Add the axis to the report**

In `bower-testkit/src/coverage.rs`:

Add a variant to `enum Mechanism` after `PlayCell`:

```rust
    /// An `exercise="…"` in either form.
    Exercise,
```

Change `all()` to return `[Mechanism; 10]` and append `Self::Exercise,`. In `Display` add `Self::Exercise => "exercise",`.

In `struct CoverageReport` add:

```rust
    /// The `expect` values that carry an exercise somewhere in the corpus —
    /// the exercise × expect axis of the spec.
    pub exercise_expects: BTreeSet<String>,
```

In `absorb`, change the play-cell branch at the top of the block loop to:

```rust
            if b.exercise.is_some() {
                self.mechanisms.insert(Mechanism::Exercise.to_string());
            }
            if b.play || b.exercise_block {
                if b.play {
                    self.mechanisms.insert(Mechanism::PlayCell.to_string());
                }
                repos.insert(&b.repo.0);
                continue;
            }
```

At the end of `absorb` (after the `library` check) add:

```rust
        // Which `expect` an exercise sits on is a plan-time fact: the block
        // form learns its step only when bound.
        if let Ok(p) = plan(&fixture.book, &fixture.catalog) {
            for s in p.repos.iter().flat_map(|r| r.steps.iter()) {
                if s.exercise.is_some() {
                    self.exercise_expects.insert(s.expect.to_string());
                }
            }
        }
```

Add after `missing_mechanisms`:

```rust
    /// Expectations no exercise in the corpus sits on.
    #[must_use]
    pub fn missing_exercise_expects(&self) -> Vec<String> {
        Expect::all()
            .iter()
            .map(ToString::to_string)
            .filter(|e| !self.exercise_expects.contains(e))
            .collect()
    }
```

In `is_complete` add `&& self.missing_exercise_expects().is_empty()`.

In `Display`, add a fourth line after `mechanisms`:

```rust
        writeln!(
            f,
            "{}",
            line(
                "exercise × expect",
                &self.exercise_expects,
                &self.missing_exercise_expects()
            )
        )
```

(and turn the previous `mechanisms` `writeln!` into a `writeln!(…)?;`).

- [ ] **Step 8: Run the whole testkit**

Run: `cargo test -p bower-testkit`
Expected: all PASS, including `properties` (generators never emit exercises, so every generated book still plans).

- [ ] **Step 9: Commit**

Tell the user to run, and wait:

```bash
git add bower-testkit && git commit -m "test(testkit): exercise fixtures, five broken books, and the exercise x expect axis"
```

---

### Task 4: Reader commands in `bower.toml`

**Files:**
- Modify: `bower/src/config.rs` (`LinkTemplates`, `WireLinks`, `parse`, tests)
- Modify: `bower/src/verify.rs:21-22` (constants move)
- Modify: every full `LinkTemplates { … }` literal outside `config.rs`

**Interfaces:**
- Produces: `pub const DEFAULT_CHECK: &str`, `pub const DEFAULT_VERIFY: &str` in `bower::config`; `LinkTemplates.{fork, clone, check, verify}: Option<String>`. Derivations, given `github = "o/r"`: `fork = "https://github.com/o/r/fork"` (declarable in `[repos.<n>.links]`), `clone = "git clone https://github.com/o/r.git"`; `check`/`verify` always `Some`, defaults applied.

- [ ] **Step 1: Write the failing config tests**

In `bower/src/config.rs`, inside `mod config_tests`, add:

```rust
    #[test]
    fn config__a_repo_with_a_remote_gets_fork_and_clone() {
        let text = format!("{MINIMAL}[repos.r]\ngithub = \"o/r\"\n");
        let cfg = BookConfig::parse(&text).unwrap();
        let links = &cfg.repos.get("r").unwrap().links;
        assert_eq!(links.fork.as_deref(), Some("https://github.com/o/r/fork"));
        assert_eq!(
            links.clone.as_deref(),
            Some("git clone https://github.com/o/r.git")
        );
    }

    #[test]
    fn config__a_declared_fork_link_wins() {
        let text = format!(
            "{MINIMAL}[repos.r]\ngithub = \"o/r\"\n[repos.r.links]\nfork = \"https://forge.invalid/o/r/fork\"\n"
        );
        let cfg = BookConfig::parse(&text).unwrap();
        assert_eq!(
            cfg.repos.get("r").unwrap().links.fork.as_deref(),
            Some("https://forge.invalid/o/r/fork")
        );
    }

    #[test]
    fn config__a_repo_without_a_remote_gets_no_fork_or_clone() {
        let text = format!("{MINIMAL}[repos.r]\ncheck = \"cargo check\"\n");
        let cfg = BookConfig::parse(&text).unwrap();
        let links = &cfg.repos.get("r").unwrap().links;
        assert_eq!(links.fork, None);
        assert_eq!(links.clone, None);
    }

    #[test]
    fn config__reader_commands_carry_the_verifier_defaults() {
        // What the exercise box prints must be what `bower verify` runs.
        let text = format!("{MINIMAL}[repos.r]\ngithub = \"o/r\"\n");
        let cfg = BookConfig::parse(&text).unwrap();
        let links = &cfg.repos.get("r").unwrap().links;
        assert_eq!(links.check.as_deref(), Some(DEFAULT_CHECK));
        assert_eq!(links.verify.as_deref(), Some(DEFAULT_VERIFY));

        let text = format!("{MINIMAL}[repos.r]\ncheck = \"make check\"\nverify = \"make test\"\n");
        let cfg = BookConfig::parse(&text).unwrap();
        let links = &cfg.repos.get("r").unwrap().links;
        assert_eq!(links.check.as_deref(), Some("make check"));
        assert_eq!(links.verify.as_deref(), Some("make test"));
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p bower config__`
Expected: compile errors on `fork`, `clone`, `DEFAULT_CHECK`.

- [ ] **Step 3: Implement**

In `bower/src/config.rs`, after the `use` lines:

```rust
/// The verifier's default commands, and what the exercise box prints when a
/// repo declares neither. One definition, so the box never names a command
/// `bower verify` would not run.
pub const DEFAULT_CHECK: &str = "cargo check";
pub const DEFAULT_VERIFY: &str = "cargo test";
```

Change the doc comment on `LinkTemplates` to `/// Forge URL templates, and the reader commands derived from them. Keeping the URLs as templates is what lets a book point at Codeberg or a self-hosted forge without a code change.` and add fields after `checkout`:

```rust
    /// Where a reader forks the repository. Declarable in `[links]`; derived
    /// from `github` when absent.
    pub fork: Option<String>,
    /// The command that clones the upstream repository. Derived from
    /// `github`; the reader's own fork is theirs to substitute.
    pub clone: Option<String>,
    /// The repo's check command with the verifier's default applied: what the
    /// exercise box tells a reader to run after a `compile_fail` step.
    pub check: Option<String>,
    /// The repo's verify command, default applied: the command after every
    /// other kind of step.
    pub verify: Option<String>,
```

In `struct WireLinks` add `fork: Option<String>,`.

In `parse`, replace the `let checkout = …;` line and the `links: LinkTemplates { … }` literal with:

```rust
                    // Derived, not declared: the commands are the same
                    // everywhere git is, and only offerable once a remote
                    // exists. `fork` alone may be declared, for forges whose
                    // fork URL is not GitHub's.
                    let checkout = r.github.as_ref().map(|_| "git checkout {tag}".to_string());
                    let fork = r.links.fork.clone().or_else(|| {
                        r.github
                            .as_ref()
                            .map(|g| format!("https://github.com/{g}/fork"))
                    });
                    let clone = r
                        .github
                        .as_ref()
                        .map(|g| format!("git clone https://github.com/{g}.git"));
                    let check = Some(r.check.clone().unwrap_or_else(|| DEFAULT_CHECK.to_string()));
                    let verify =
                        Some(r.verify.clone().unwrap_or_else(|| DEFAULT_VERIFY.to_string()));
                    (
                        name,
                        RepoConfig {
                            github: r.github,
                            site_branch: r.site_branch,
                            assets: r.assets,
                            template: r.template,
                            check: r.check,
                            verify: r.verify,
                            keep_region_markers: r.keep_region_markers,
                            links: LinkTemplates {
                                blob: r.links.blob,
                                tree: r.links.tree,
                                commit: r.links.commit,
                                checkout,
                                fork,
                                clone,
                                check,
                                verify,
                            },
                        },
                    )
```

In `bower/src/verify.rs`, delete the two `const DEFAULT_CHECK` / `DEFAULT_VERIFY` lines and add `use crate::config::{DEFAULT_CHECK, DEFAULT_VERIFY};` with the other `use crate::…` imports.

- [ ] **Step 4: Fix the two other literals**

Exactly two full `LinkTemplates { … }` literals exist outside `config.rs`: `bower/src/render.rs:397` (`github_links()` in the tests) and `bower/src/publish.rs:1150` (the tests' `links()`). In each, add `..LinkTemplates::default()` as the last entry so the four new fields default to `None`. Confirm with `grep -rn "LinkTemplates {" bower/src bower/tests | grep -v config.rs`.

- [ ] **Step 5: Run the CLI tests**

Run: `cargo test -p bower --all-features`
Expected: all PASS.

- [ ] **Step 6: Commit**

Tell the user to run, and wait:

```bash
git add bower/src/config.rs bower/src/verify.rs bower/src/render.rs bower/src/publish.rs && git commit -m "feat(bower): fork, clone, check and verify as reader commands beside checkout"
```

---

### Task 5: Render the box and fold the answer

**Files:**
- Modify: `bower/src/render.rs` (`chapter`, new `exercise_box` and helpers, new maps, tests)

**Interfaces:**
- Consumes: `Exercise`, `ExerciseForm`, `Expect`, `StepId` (prelude); `LinkTemplates.{fork, clone, check, verify, checkout}` (Task 4); `Target::has_hidden_lines()`.
- Produces: `pub fn exercise_box(step: &PlannedStep, exercise: &Exercise, links: Option<&LinkTemplates>, target: Target) -> Vec<String>`.

- [ ] **Step 1: Write the failing render tests**

In `bower/src/render.rs`, inside `mod render_tests`, add:

```rust
    const EXERCISE_CH: &str = concat!(
        "# One\n\n",
        "<!-- bower repo=\"r\" step=\"broken\" file=\"src/lib.rs\" expect=\"compile_fail\" exercise=\"Make this compile\" -->\n\n",
        "```rust\n",
        "fn x() -> u32 { \"42\" }\n",
        "```\n\n",
        "Between.\n\n",
        "<!-- bower repo=\"r\" step=\"fixed\" file=\"src/lib.rs\" op=\"replace\" -->\n\n",
        "```rust\n",
        "fn x() -> u32 { 42 }\n",
        "```\n\n",
        "<!-- bower repo=\"r\" exercise=\"Do it without a literal\" -->\n",
        "```markdown\n",
        "Parse it instead.\n",
        "```\n\n",
        "<!-- bower repo=\"r\" step=\"last\" file=\"src/lib.rs\" op=\"append\" -->\n\n",
        "```rust\n",
        "// fin\n",
        "```\n",
    );

    fn exercise_links() -> BTreeMap<String, LinkTemplates> {
        let mut m = github_links();
        let l = m.get_mut("r").unwrap();
        l.fork = Some("https://x.invalid/fork".to_string());
        l.clone = Some("git clone https://x.invalid/r.git".to_string());
        l.check = Some("cargo check".to_string());
        l.verify = Some("cargo test".to_string());
        m
    }

    fn render_exercise(target: Target, links: &BTreeMap<String, LinkTemplates>) -> String {
        chapter(EXERCISE_CH, "src/ch01.md", &tiny_plan(EXERCISE_CH), links, target)
    }

    #[test]
    fn exercise__key_form_box_follows_the_checkout_line() {
        let out = render_exercise(Target::Html, &exercise_links());
        let checkout = out.find("$> `git checkout step-001-broken`").expect("checkout");
        let bx = out.find("**Your turn: Make this compile**").expect("box");
        let between = out.find("Between.").expect("prose");
        assert!(checkout < bx && bx < between, "{out}");
    }

    #[test]
    fn exercise__box_names_fork_clone_checkout_and_the_check_command() {
        let out = render_exercise(Target::Html, &exercise_links());
        assert!(
            out.contains("Fork [the repository](https://x.invalid/fork), then:"),
            "{out}"
        );
        assert!(
            out.contains(
                "```console\ngit clone https://x.invalid/r.git   # or your fork\ngit checkout step-001-broken\ncargo check\n```"
            ),
            "{out}"
        );
        assert!(
            out.contains("Green means you did it. The answer is [the next step](#step-fixed)."),
            "{out}"
        );
    }

    #[test]
    fn exercise__block_form_replaces_its_fence_and_says_one_way() {
        let out = render_exercise(Target::Html, &exercise_links());
        assert!(
            !out.contains("```markdown"),
            "the detail fence must not render as code: {out}"
        );
        assert!(
            out.contains("**Your turn: Do it without a literal**\n\nParse it instead.\n"),
            "{out}"
        );
        // A green step: `verify`, and one way rather than the answer.
        assert!(out.contains("git checkout step-002-fixed\ncargo test\n```"), "{out}");
        assert!(
            out.contains("Keep it green. [The next step](#step-last) shows one way."),
            "{out}"
        );
    }

    #[test]
    fn exercise__html_folds_the_answer_step_and_not_the_question() {
        let out = render_exercise(Target::Html, &exercise_links());
        assert_eq!(
            out.matches("<details class=\"step-answer\">").count(),
            2,
            "fixed and last are both answers: {out}"
        );
        assert_eq!(out.matches("</details>").count(), 2, "{out}");
        assert!(out.contains("<summary>Show the answer</summary>"), "{out}");
        assert!(out.contains("<summary>Show one way</summary>"), "{out}");
        let fold = out.find("<details").unwrap();
        let broken = out.find("fn x() -> u32 { \"42\" }").unwrap();
        let fixed = out.find("fn x() -> u32 { 42 }").unwrap();
        assert!(broken < fold && fold < fixed, "{out}");
        // The fold closes after the block's checkout line, before the box.
        let close = out.find("</details>").unwrap();
        let fixed_checkout = out.find("$> `git checkout step-002-fixed`").unwrap();
        let second_box = out.find("**Your turn: Do it without a literal**").unwrap();
        assert!(fixed_checkout < close && close < second_box, "{out}");
    }

    #[test]
    fn exercise__epub_is_a_blockquote_with_no_toggle_or_anchor_link() {
        let out = render_exercise(Target::Epub, &exercise_links());
        assert!(!out.contains("<details"), "{out}");
        assert!(!out.contains("<div class=\"step-exercise\">"), "{out}");
        assert!(out.contains("> **Your turn: Make this compile**"), "{out}");
        assert!(
            out.contains("> Green means you did it. The answer is the next step."),
            "{out}"
        );
        assert!(!out.contains("#step-fixed"), "{out}");
    }

    #[test]
    fn exercise__without_a_remote_keeps_the_task_and_drops_the_commands() {
        let out = render_exercise(Target::Html, &no_links());
        assert!(out.contains("**Your turn: Make this compile**"), "{out}");
        assert!(!out.contains("Fork"), "{out}");
        assert!(!out.contains("git clone"), "{out}");
        assert!(!out.contains("cargo check"), "{out}");
        assert!(out.contains("Green means you did it."), "{out}");
    }

    #[test]
    fn exercise__html_box_is_a_div_the_theme_styles() {
        let out = render_exercise(Target::Html, &exercise_links());
        assert!(
            out.contains("<div class=\"step-exercise\">\n\n**Your turn"),
            "{out}"
        );
        assert!(out.contains("\n\n</div>"), "{out}");
    }

    const PROSE_EXERCISE_CH: &str = concat!(
        "# One\n\n",
        "<!-- bower repo=\"r\" step=\"note\" op=\"none\" expect=\"none\" exercise=\"Try the refactor\" -->\n\n",
        "Just narrative.\n\n",
        "<!-- bower repo=\"r\" step=\"code\" file=\"src/lib.rs\" -->\n\n",
        "```rust\nfn y() {}\n```\n",
    );

    #[test]
    fn exercise__on_a_prose_step_renders_with_no_command_line() {
        let out = chapter(
            PROSE_EXERCISE_CH,
            "src/ch01.md",
            &tiny_plan(PROSE_EXERCISE_CH),
            &exercise_links(),
            Target::Html,
        );
        let bx = out.find("**Your turn: Try the refactor**").expect("box");
        let prose = out.find("Just narrative.").unwrap();
        assert!(bx < prose, "{out}");
        assert!(
            out.contains("git checkout step-001-note\n```"),
            "no command after the checkout on a skipped step: {out}"
        );
        assert!(
            out.contains("[The next step](#step-code) shows one way."),
            "{out}"
        );
    }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p bower exercise__`
Expected: the eight tests FAIL (no box, no fold; ````markdown` survives).

- [ ] **Step 3: Add the box and its helpers**

In `bower/src/render.rs`, extend the prelude import to:

```rust
use bower_core::prelude::{
    BlockDisplay, BookPlan, Directive, Exercise, ExerciseForm, Expect, LineRange, PlannedStep,
    ShowMark, StepId, show_marker,
};
```

After `checkout_line`, add:

```rust
/// The "your turn" box for a step's exercise (spec § 6.1), as markdown lines.
///
/// One text for every target; only the wrapping differs. HTML gets a `<div>`
/// with a class the book's theme styles — the tool's whole opinion about
/// looks, as with `step-meta`. Epub and PDF have no stylesheet of ours, so
/// they get a blockquote, which every renderer draws as an aside.
///
/// The commands need a remote: a repo nobody can clone gets the task, the
/// detail, and the closing line, and no command that would fail.
#[must_use]
pub fn exercise_box(
    step: &PlannedStep,
    exercise: &Exercise,
    links: Option<&LinkTemplates>,
    target: Target,
) -> Vec<String> {
    let mut body: Vec<String> = vec![format!("**Your turn: {}**", exercise.task)];
    if !exercise.detail.is_empty() {
        body.push(String::new());
        body.extend(exercise.detail.iter().cloned());
    }

    if let Some(l) = links
        && let Some(clone) = l.clone.as_deref()
        && let Some(checkout) = subst(l.checkout.as_deref(), &step.tag(), None)
    {
        body.push(String::new());
        body.push(match l.fork.as_deref() {
            Some(url) => format!("Fork [the repository]({url}), then:"),
            None => "Clone the repository, then:".to_string(),
        });
        body.push(String::new());
        body.push("```console".to_string());
        body.push(format!("{clone}   # or your fork"));
        body.push(checkout);
        if let Some(cmd) = reader_command(step.expect, l) {
            body.push(cmd.to_string());
        }
        body.push("```".to_string());
    }

    body.push(String::new());
    body.push(closing_line(step.expect, &exercise.answer, target));
    wrap_box(body, target)
}

/// Which of the repo's two commands a reader runs after this step: `check`
/// after a `compile_fail`, `verify` after anything with tests to run, nothing
/// after a step the verifier skips.
fn reader_command(expect: Expect, links: &LinkTemplates) -> Option<&str> {
    match expect {
        Expect::CompileFail => links.check.as_deref(),
        Expect::TestFail | Expect::Pass => links.verify.as_deref(),
        Expect::Skip => None,
    }
}

/// The last line: what green means, and where the next step is. A broken
/// step has an answer; a green one has one way among many. Only HTML has an
/// anchor to link.
fn closing_line(expect: Expect, answer: &StepId, target: Target) -> String {
    let (lower, upper) = match target {
        Target::Html => (
            format!("[the next step](#step-{answer})"),
            format!("[The next step](#step-{answer})"),
        ),
        Target::Epub | Target::Pdf => ("the next step".to_string(), "The next step".to_string()),
    };
    match expect {
        Expect::CompileFail | Expect::TestFail => {
            format!("Green means you did it. The answer is {lower}.")
        }
        Expect::Pass => format!("Keep it green. {upper} shows one way."),
        Expect::Skip => format!("{upper} shows one way."),
    }
}

/// HTML: a `<div>` the theme styles, blank-line separated so the markdown
/// inside still renders (CommonMark ends an HTML block at a blank line).
/// Everything else: a blockquote.
fn wrap_box(body: Vec<String>, target: Target) -> Vec<String> {
    match target {
        Target::Html => {
            let mut out = vec!["<div class=\"step-exercise\">".to_string(), String::new()];
            out.extend(body);
            out.push(String::new());
            out.push("</div>".to_string());
            out
        }
        Target::Epub | Target::Pdf => body
            .into_iter()
            .map(|l| {
                if l.is_empty() {
                    ">".to_string()
                } else {
                    format!("> {l}")
                }
            })
            .collect(),
    }
}

/// The toggle's label over an answer step's blocks: a broken step has an
/// answer, a green one has one way.
fn answer_summary(expect: Expect) -> &'static str {
    match expect {
        Expect::CompileFail | Expect::TestFail => "Show the answer",
        Expect::Pass | Expect::Skip => "Show one way",
    }
}

/// Print a key-form box below whatever the step's last block left on the
/// page — a fence and its checkout line, or nothing at all for a prose step.
fn push_key_box(
    out: &mut Vec<String>,
    entry: Option<&(String, &PlannedStep, &Exercise)>,
    forge: &BTreeMap<String, LinkTemplates>,
    target: Target,
) {
    if let Some((repo, step, x)) = entry
        && x.form == ExerciseForm::Key
    {
        out.push(String::new());
        out.extend(exercise_box(step, x, forge.get(repo), target));
        out.push(String::new());
    }
}

/// From just past a directive, the index just past the fence that follows it
/// (blank lines between allowed). A directive with no fence yields `from`.
fn skip_directive_fence(lines: &[&str], from: usize) -> usize {
    let mut j = from;
    while j < lines.len() && lines[j].trim().is_empty() {
        j += 1;
    }
    let Some(width) = lines.get(j).and_then(|l| fence_width(l)) else {
        return from;
    };
    let mut k = j + 1;
    while k < lines.len() && fence_width(lines[k]).is_none_or(|w| w < width) {
        k += 1;
    }
    (k + 1).min(lines.len())
}
```

After `anchors_by_line`, add the two maps:

```rust
/// Line number → the exercise that renders there, with its step and repo.
/// Keyed by [`Exercise::at`]: the block form's own directive, or the key
/// form's last block.
fn exercises_by_line<'a>(
    plan: &'a BookPlan,
    chapter_path: &str,
) -> BTreeMap<usize, (String, &'a PlannedStep, &'a Exercise)> {
    let mut out = BTreeMap::new();
    for repo in &plan.repos {
        for step in &repo.steps {
            if let Some(x) = &step.exercise
                && x.at.chapter == chapter_path
            {
                out.insert(x.at.line, (repo.repo.0.clone(), step, x));
            }
        }
    }
    out
}

/// Line number → the fold's label, for every block of a step that is some
/// exercise's answer.
fn folds_by_line(plan: &BookPlan, chapter_path: &str) -> BTreeMap<usize, &'static str> {
    let mut out = BTreeMap::new();
    for repo in &plan.repos {
        for step in &repo.steps {
            let Some(x) = &step.exercise else { continue };
            let Some(answer) = repo.steps.iter().find(|s| s.id == x.answer) else {
                continue;
            };
            for display in &answer.displays {
                if display.loc.chapter == chapter_path {
                    out.insert(display.loc.line, answer_summary(step.expect));
                }
            }
        }
    }
    out
}
```

- [ ] **Step 4: Wire `chapter`**

In `chapter`, after `let blocks = blocks_by_line(plan, chapter_path);` add:

```rust
    let exercises = exercises_by_line(plan, chapter_path);
    let folds = folds_by_line(plan, chapter_path);
```

Directly after the line `i += 1; // the directive itself never reaches the page`, add:

```rust
        // A block-form exercise: its fence is the box's detail, not code. Eat
        // the fence and print the box where the author put the directive.
        if let Some((repo, step, x)) = exercises.get(&directive_line)
            && x.form == ExerciseForm::Block
        {
            i = skip_directive_fence(&lines, i);
            out.extend(exercise_box(step, x, forge.get(repo), target));
            out.push(String::new());
            continue;
        }
```

Change the no-fence early exit

```rust
        let Some(width) = fence_width(lines.get(j).copied().unwrap_or_default()) else {
            continue;
        };
```

to

```rust
        let Some(width) = fence_width(lines.get(j).copied().unwrap_or_default()) else {
            push_key_box(&mut out, exercises.get(&directive_line), forge, target);
            continue;
        };

        // An answer step's code folds shut in HTML; the reader opens it after
        // trying. Print has no toggle, so it prints.
        let fold = if target.has_hidden_lines() {
            folds.get(&directive_line).copied()
        } else {
            None
        };
        if let Some(label) = fold {
            out.push("<details class=\"step-answer\">".to_string());
            out.push(format!("<summary>{label}</summary>"));
            out.push(String::new());
        }
```

At the end of the fence path, replace

```rust
        if let Some((repo, step, _)) = block
            && let Some(line) = checkout_line(step, forge.get(repo))
        {
            out.push(String::new());
            out.push(line);
        }
        i = k + 1;
```

with

```rust
        if let Some((repo, step, _)) = block
            && let Some(line) = checkout_line(step, forge.get(repo))
        {
            out.push(String::new());
            out.push(line);
        }
        if fold.is_some() {
            out.push(String::new());
            out.push("</details>".to_string());
        }
        push_key_box(&mut out, exercises.get(&directive_line), forge, target);
        i = k + 1;
```

- [ ] **Step 5: Run the render tests**

Run: `cargo test -p bower render_tests`
Expected: all PASS, the eight new ones included, and every earlier `render__`, `footer__`, `checkout__` test unchanged.

- [ ] **Step 6: Lint**

Run: `cargo clippy -p bower --all-targets --all-features -- -D warnings`
Expected: clean. If `chapter` trips `too_many_lines`, move the fence path's fold open/close into two tiny helpers (`open_fold(out, label)` / `close_fold(out)`) rather than allowing the lint.

- [ ] **Step 7: Commit**

Tell the user to run, and wait:

```bash
git add bower/src/render.rs && git commit -m "feat(bower): render the exercise box and fold the answer step in HTML"
```

---

### Task 6: `STEPS.md` marks exercise steps

**Files:**
- Modify: `bower/src/trailers.rs` (`steps_md`, tests)

**Interfaces:**
- Consumes: `PlannedStep.exercise` (Task 2).
- Produces: the Subject cell of an exercise step reads `<msg> · exercise: <task>`.

- [ ] **Step 1: Write the failing test**

In `bower/src/trailers.rs`, inside `mod trailer_tests` (which already imports `RepoCatalog` and `plan` from the prelude), add:

```rust
    #[test]
    fn steps_md__marks_an_exercise_step() {
        use bower_core::prelude::{BookSource, Chapter};
        let book = BookSource::from_chapters(vec![Chapter::new(
            "src/ch01.md",
            concat!(
                "<!-- bower repo=\"r\" step=\"broken\" file=\"a.rs\" expect=\"compile_fail\" msg=\"feat: broken\" exercise=\"Make this compile\" -->\n```rust\nx\n```\n",
                "<!-- bower repo=\"r\" step=\"fixed\" file=\"a.rs\" op=\"replace\" msg=\"fix: fixed\" -->\n```rust\ny\n```\n",
            ),
        )]);
        let p = plan(&book, &RepoCatalog::from_names(&["r"])).unwrap();
        let md = steps_md(p.repo("r").unwrap(), "book", None);
        assert!(
            md.contains("| 001 | `step-001-broken` | compile_fail | feat: broken · exercise: Make this compile |"),
            "{md}"
        );
        assert!(md.contains("| 002 | `step-002-fixed` | pass | fix: fixed |"), "{md}");
    }
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test -p bower steps_md__marks`
Expected: FAIL on the first assertion.

- [ ] **Step 3: Implement**

In `steps_md`, before the `writeln!` of the row, add:

```rust
        // The code side's pointer to the "your turn" points: a reader
        // browsing the repo finds them without opening the book.
        let subject = match &step.exercise {
            Some(x) => format!("{} · exercise: {}", step.msg, x.task),
            None => step.msg.clone(),
        };
```

and in the row's `writeln!` replace `step.msg` with `subject`.

- [ ] **Step 4: Run the CLI tests**

Run: `cargo test -p bower --all-features`
Expected: all PASS (`status` reads `STEPS.md` through the same function, so nothing else moves).

- [ ] **Step 5: Commit**

Tell the user to run, and wait:

```bash
git add bower/src/trailers.rs && git commit -m "feat(bower): STEPS.md names each step's exercise"
```

---

### Task 7: An exercise in each book, styled, locked, and verified end to end

**Files:**
- Modify: `books/hello-playbook/src/ch04-tests-and-failing-on-purpose.md:70`
- Modify: `books/hello-playbook/theme/step-meta.css`, `books/rust4failures/theme/step-meta.css`
- Modify: `books/rust4failures/src/ch03-rank.md` (before `## Answering the question`)
- Regenerate: `books/hello-playbook/bower.lock`, `books/rust4failures/bower.lock`
- Modify: `bower/tests/preprocessor.rs`, `bower/tests/publish.rs`

**Interfaces:**
- Consumes: everything above.

- [ ] **Step 1: Write the failing sample-book tests**

In `bower/tests/preprocessor.rs`, after `sample_book_footers_link_to_the_right_lines`:

```rust
#[test]
fn sample_book_renders_the_exercise_and_folds_its_answer() {
    let ch04 = &rendered_chapters()[3];
    assert!(ch04.contains("<div class=\"step-exercise\">"), "{ch04}");
    assert!(ch04.contains("**Your turn: Make the test pass**"), "{ch04}");
    assert!(
        ch04.contains("Fork [the repository](https://github.com/abstecker/hello-playbook/fork), then:"),
        "{ch04}"
    );
    assert!(
        ch04.contains("git checkout step-011-test-that-fails\ncargo test\n```"),
        "{ch04}"
    );
    assert!(ch04.contains("<summary>Show the answer</summary>"), "{ch04}");
    let fold = ch04.find("<details class=\"step-answer\">").unwrap();
    let fix = ch04.find("name.trim()").unwrap();
    assert!(fold < fix, "the fold must precede the fix: {ch04}");
}
```

In `bower/tests/publish.rs`, after `html_plan_keeps_the_toggle`:

```rust
#[test]
fn epub_plan_carries_the_exercise_without_a_toggle() {
    let ch4 = &plan_for(Target::Epub).chapters[3].markdown;
    assert!(ch4.contains("> **Your turn: Make the test pass**"), "{ch4}");
    assert!(!ch4.contains("<details"), "{ch4}");
    assert!(!ch4.contains("step-exercise"), "{ch4}");

    let html = &plan_for(Target::Html).chapters[3].markdown;
    assert!(html.contains("<details class=\"step-answer\">"), "{html}");
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p bower --all-features --test preprocessor sample_book_renders_the_exercise && cargo test -p bower --all-features --test publish epub_plan_carries`
Expected: both FAIL — the sample book has no exercise yet.

- [ ] **Step 3: Add the key form to the sample book**

In `books/hello-playbook/src/ch04-tests-and-failing-on-purpose.md`, change the `test-that-fails` directive (line 70) to:

```markdown
<!-- bower repo="hello-playbook" step="test-that-fails" file="src/lib.rs" op="region" region="tests" expect="test_fail" msg="test: greet should ignore stray whitespace (failing)" exercise="Make the test pass" -->
```

After the paragraph that ends `The markers never reach the repository.` and before `## The fix`, add:

```markdown
The `exercise` key is the third thing. It marks this step as a place to stop
reading and start typing: the box below the code tells you how to fork the
repository, check out exactly this commit, and run the tests yourself. The
next section is one answer; it stays folded until you open it.
```

- [ ] **Step 4: Add the block form to *Rust for Failures***

In `books/rust4failures/src/ch03-rank.md`, immediately before the `## Answering the question` heading, add:

```markdown
<!-- bower repo="rust4failures" exercise="Make this compile" -->
```markdown
The compiler has named every character it is not told about. Each one needs an
answer — not just the thirteen on a card.

- The smallest fix is one more arm. Which pattern, and what does it return?
- Could `'z'` be an *error* instead of a rank? What would `from` have to
  become for that to be true?
```
```

- [ ] **Step 5: Style the box and the fold in both themes**

Append to **both** `books/hello-playbook/theme/step-meta.css` and `books/rust4failures/theme/step-meta.css`:

```css
/* The "your turn" box bower prints after a step that carries an exercise.
   Same left edge as .step-meta and .step-checkout, so the three read as one
   column of things-about-this-step. */
.step-exercise {
    margin: 0.5em 0 1em 1.5em;
    padding: 0.4em 1em;
    border-left: 4px solid var(--links, #4183c4);
    background: var(--table-alternate-bg, rgba(127, 127, 127, 0.08));
}

/* The fold over an answer step's code. The summary is all a reader sees
   until they choose to see more. */
.step-answer > summary {
    cursor: pointer;
    font-weight: bold;
    margin: 0.5em 0;
}
```

- [ ] **Step 6: Regenerate both locks**

Run:

```bash
cargo run -q -p bower -- --book books/hello-playbook plan
cargo run -q -p bower -- --book books/rust4failures plan
git diff --stat books/hello-playbook/bower.lock books/rust4failures/bower.lock
```

Expected: `plan` writes `bower.lock` (`bower/src/main.rs:237`). Each lock gains one `    exercise form=… answer=… task="…"` line, and the *Rust for Failures* lock also gains `    | …` detail lines under `from-char-broken`.

- [ ] **Step 7: Run the sample-book tests**

Run: `cargo test -p bower --all-features && cargo test -p bower-testkit`
Expected: all PASS, the two new ones included. `sample_book.rs` in the testkit still passes (tags and final paths are unchanged).

- [ ] **Step 8: Render both books and verify the real one**

Run: `make book && make failures`
Expected: both render; `bower verify` on *Rust for Failures* reports every step as claimed; `bower status` reports no drift. Then open `books/rust4failures/book/ch03-rank.html` and `books/hello-playbook/book/ch04-tests-and-failing-on-purpose.html` in a browser (or `grep -c "step-exercise\|step-answer"` each) and confirm the box and the fold are there.

- [ ] **Step 9: Commit**

Tell the user to run, and wait:

```bash
git add books/hello-playbook/src/ch04-tests-and-failing-on-purpose.md books/hello-playbook/theme/step-meta.css books/hello-playbook/bower.lock books/rust4failures/src/ch03-rank.md books/rust4failures/theme/step-meta.css books/rust4failures/bower.lock bower/tests/preprocessor.rs bower/tests/publish.rs && git commit -m "feat(books): an exercise in each book, styled and locked"
```

(The rendered `book/` directories are build output; whether to commit them follows whatever the user already does for them.)

---

### Task 8: Docs, backlog, and the full gate

**Files:**
- Modify: `README.md` (after `### Covers`)
- Modify: `bower-spec.md:135` (the key table)
- Modify: `BACKLOG.md` (Known gaps)

- [ ] **Step 1: README**

After the `### Covers` section (before `## License`), add:

```markdown
### Exercises

A step can be a place to stop reading and start typing. Put `exercise="Make
this compile"` on a step's directive, or give the exercise its own directive
with a fenced markdown body of ideas right after the step:

```markdown
<!-- bower repo="rust4failures" exercise="Make this compile" -->
```markdown
Every `char` needs an answer, not just the card ones.

- Try a lookup table instead of a `match`.
```
```

The rendered box names the fork link, the clone and checkout commands, and the
command to run — `check` after a `compile_fail`, `verify` otherwise, the same
ones `bower verify` runs. The next step is the answer: in HTML its code folds
behind "Show the answer" (or "Show one way" after a green step); epub and PDF
print it with a note. An exercise can sit on any step but the last, and a step
carries at most one. Design: `docs/superpowers/specs/2026-09-06-exercises-design.md`.
```

- [ ] **Step 2: Spec key table**

In `bower-spec.md`, after the `notebook` row of the § 3.2 key table, add:

```markdown
| `exercise` | no | — | A "your turn" task on this step. Alone, with no tree keys, it is a block-form exercise whose fence is the detail. See `docs/superpowers/specs/2026-09-06-exercises-design.md` |
```

- [ ] **Step 3: Backlog**

In `BACKLOG.md`, under `## Known gaps`, add:

```markdown
- [ ] Exercises have no place for a *reader's* answer — a per-exercise link to a Discussion or similar (exercises spec § 2)
- [ ] Exercises cannot carry a hidden solution the book does not print; the answer is always the next step (exercises spec § 2)
```

- [ ] **Step 4: The full gate**

Run: `make ayce`
Expected: clean, fmt, build, test, lint (purity line still one row), security-scan, docs — all green. Fix anything it finds before reporting done.

- [ ] **Step 5: Commit**

Tell the user to run, and wait:

```bash
git add README.md bower-spec.md BACKLOG.md && git commit -m "docs: exercises in the README, the spec's key table, and the backlog"
```
