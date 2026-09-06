# Exercises — "your turn" points in a Bower book

**Status:** approved design, not yet implemented
**Date:** 2026-09-06
**Repo:** `ImperialBower/bower`
**Relates to:** `bower-spec.md` Draft 0.2 (§ 3.2 the directive, § 5.3 tags,
§ 5.4 the preprocessor, § 6 verification, § 15.1 play cells)

---

## 1. Purpose

Mark points in a book where the reader stops reading and starts typing: fork
the generated repository, check out the step, make it green, then compare with
what the book does next. The book already has everything this needs:

- every step is an annotated tag the reader can `git checkout`;
- a step with `expect="compile_fail"` or `expect="test_fail"` is a verified
  broken state — a question with a known-good next move;
- the step after it is, by the book's own structure, the answer.

An **exercise** is the missing piece: the author says "stop here and try
something", optionally adds ideas, and Bower renders the box that tells the
reader how to take the question home. A broken step is the obvious place —
"make it compile" — but an exercise can sit on **any** step: "do it with a
lookup table", "make this faster", "add the missing test". Nothing new enters
any repo tree.

## 2. Non-goals

Deliberately out, recorded in `BACKLOG.md` when this ships:

- **Reader's answers.** No link per exercise to a Discussion, form, or
  submission flow. Generated repos already say "no PRs".
- **Hidden solutions.** The answer is always the next step of the same repo, in
  book order. There is no separate solution block the book does not show.
- **Verification changes.** Every step is already verified against its
  `expect`. The exercise only tells the reader the same command `verify` ran.
- **Grading.** Bower does not judge the reader's attempt. "Green" is the only
  signal, and on a green step the task itself says what counts.

## 3. Authoring

Two forms, one meaning. The key form fits in the directive that already
declares the failure:

```markdown
<!-- bower repo="rust4failures" step="from-char-broken" file="src/rank.rs" op="region" region="from_char" expect="compile_fail" exercise="Make this compile" -->
```

The block form carries the same key plus a fenced markdown body with more to
say — the task in a sentence or two, then ideas to try:

```markdown
<!-- bower repo="rust4failures" exercise="Make this compile" -->
```markdown
Every `char` needs an answer, not just the card ones.

- Try a lookup table instead of a `match`.
- Can `'z'` be an error instead of `BLANK`?
```
```

Binding follows the play-cell rule (`bower-spec.md` § 15.1): the block form
attaches to the **nearest preceding step of its repo in document order**, or to
an explicit `step="…"`. The key form is already on the step's own directive.

The `exercise` value is the **task**: one short imperative line. The body, when
present, is the **detail**: markdown, rendered as-is. A block form with an
empty body is allowed but pointless; the key form is shorter.

## 4. Kernel: `bower-core`

### 4.1 Types

```rust
/// One "your turn" point, bound to a step. Never part of any tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Exercise {
    /// The directive that declared it — where errors point.
    pub loc: Location,
    /// Key form or block form. Decides where the box renders (§ 6).
    pub form: ExerciseForm,
    /// Where the box renders: the block form's own directive, or the
    /// key form's step's last block in book order.
    pub at: Location,
    /// The `exercise="…"` value: one imperative line.
    pub task: String,
    /// The block form's fenced body, verbatim markdown lines. Empty for
    /// the key form.
    pub detail: Vec<String>,
    /// The step that carries the answer: the next step of the same repo.
    pub answer: StepId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExerciseForm {
    Key,
    Block,
}
```

`PlannedStep` grows one field, `pub exercise: Option<Exercise>`. `Directive`
grows `exercise: Option<String>`. `Block` grows `exercise: Option<String>`
and the existing `play: bool` is joined by an `exercise_block: bool` that means
"this block is the block form" — it carries no tree op, exactly as a play cell
carries none (`Op::Prose`).

`RepoPlan` learns nothing; the exercise is reachable through its step.

### 4.2 Parsing

`directive.rs` accepts `exercise` as a string key. `block.rs` classifies a
directive as the block form when it has `exercise` and **no** `file`, `op`,
`region`, `src`, or `paths` — the play-cell test, reused. A block-form
directive **must** be followed by a fenced block (`DirectiveWithoutBlock`
otherwise, as today); the fence's info string is not checked, `markdown` is
the convention.

A directive that has `exercise` **and** tree keys is the key form: an ordinary
tree block that also declares an exercise on its step.

### 4.3 Binding and errors

`plan.rs` binds exercises after steps are ordered, next to `bind_play_cells`.
The answer step is the step whose `seq` is the exercise step's `seq + 1` in the
same repo.

New `BowerError` variants, each carrying the exercise's `Location` and
following the `PlayCell*` naming:

| Variant | When |
|---|---|
| `ExerciseUnbound { loc }` | A block form with no preceding step of its repo and no `step=`. |
| `ExerciseUnknownStep { loc, step }` | A block form naming a step that does not exist. |
| `ExerciseDuplicate { loc, step }` | A step already carries an exercise. Reported at the second one. |
| `ExerciseWithoutAnswer { loc, step }` | The bound step is the last step of its repo. |
| `ExerciseConflictingKeys { loc }` | A block form that also carries `notebook`. A cell cannot be both. |

Every error is collected in the same pass as the others; none aborts planning.
A key form on a multi-block step (blocks sharing `step=`) counts once per
directive that carries the key — two directives of one step both saying
`exercise=` is `ExerciseDuplicate`.

### 4.4 Determinism and the lock

The exercise is part of the plan and therefore of `bower.lock`'s text: task,
detail, and answer step id, under the step. Editing an exercise is drift
`status` reports, like editing a block.

## 5. Replay: `bower`

`STEPS.md` marks exercise steps. The step's line gains a trailing
`· exercise: <task>` so a reader browsing the generated repo finds the "your
turn" points from the code side. Commit messages and tags do not change —
tag names are stable as long as step ids are, and an exercise is not a step.

## 6. Rendering

One render plan, every target, as today (`bower publish`, `render.rs`). The
exercise box renders at `Exercise::at`: for the **key form**, after the step's
last block in book order (below its checkout line); for the **block form**,
where the author wrote it — its directive and fence are consumed and the box
takes their place. In the common case (block form right after the step) the
two land in the same spot.

### 6.1 The box

```
Your turn: Make this compile

<detail, rendered as markdown>

Fork <owner>/<repo>, then:

    git clone <your fork>
    git checkout step-003-from-char-broken
    cargo check

Green means you did it. The answer is the next step.
```

The command and the two closing sentences follow the step's `expect`:

| `expect` | Command | Closing lines |
|---|---|---|
| `compile_fail` | the repo's `check` | Green means you did it. The answer is the next step. |
| `test_fail` | the repo's `verify` | Green means you did it. The answer is the next step. |
| `pass` | the repo's `verify` | Keep it green. The next step shows one way. |
| `none` | *(no command line)* | The next step shows one way. |

- The fork line uses a new link template, `fork` (§ 7). A repo with no
  remote has nothing a reader can fork, clone, or check out — today it gets
  no checkout line under its blocks either. Its box keeps the task, the
  detail, and the closing lines, and omits the whole "Fork … then:" part.
- `check` and `verify` are the same commands `bower verify` runs, defaults
  included: a repo that declares neither still says `cargo check` and
  `cargo test`.
- The closing lines are the same on every target. In HTML "the next step"
  links to the answer step's `#step-<id>` anchor.

The box is a `<div class="step-exercise">` in HTML and a blockquote-shaped
markdown block in epub and PDF, following the footer's precedent: the tool's
whole opinion about looks is one class name, and `theme/step-meta.css` styles
it.

### 6.2 The fold

In HTML, every code block of the **answer step** is wrapped in
`<details class="step-answer"><summary>Show the answer</summary> … </details>`.
Prose between the answer's blocks stays open; only fences fold. Footers fold
with their block.

The summary text follows the step: "Show the answer" after a broken step,
"Show one way" after a green one.

Epub and PDF have no toggle. They do not fold; the box's last line already
says what is next. This mirrors the elided-span rule: HTML gets the
toggle, print gets a note.

## 7. Configuration

`[repos.<name>.links]` gains `fork`, declarable or derived from `github`
exactly as `checkout` is (`config.rs`):

```toml
[repos.rust4failures.links]
fork = "https://github.com/abstecker/rust4failures/fork"   # derived when absent
```

`LinkTemplates` — already home to the derived `checkout` command — also
carries three more reader commands, never declared in the file: `clone`
(`git clone https://github.com/<github>.git`, derived), and `check` and
`verify` (the repo's own, with the verifier's defaults applied). The
verifier's `DEFAULT_CHECK` / `DEFAULT_VERIFY` move to `config.rs` so there is
one definition.

The clone line prints the upstream URL with an `# or your fork` comment —
Bower cannot know the reader's fork, and an honest copyable line beats a
placeholder.

## 8. Tests

Same shape as everything else in the workspace.

- **Testkit fixtures** (`bower-testkit`): one fixture per error variant in
  § 4.3, one good key form, one good block form with `step=`, one good block
  form bound by position. The corpus's "every error variant has a fixture"
  assertion covers the new variants automatically.
- **State coverage**: the report gains an `exercise × expect` axis and must
  show all four `expect` values carrying an exercise.
- **Kernel unit tests**: answer resolution, duplicate detection across a
  multi-block step, the lock text including the exercise.
- **Preprocessor tests** (`bower/tests/preprocessor.rs`): the box appears
  under the right step; the fold wraps the answer step's blocks and nothing
  else; the block form's fence is consumed.
- **Publish tests** (`bower/tests/publish.rs`): epub plan carries the box and
  no `<details>`; html plan carries both.
- **Sample book**: `books/hello-playbook` gets one exercise so `make book`
  renders it and the testkit plans it.
- **The real book**: `books/rust4failures/src/ch03-rank.md` gets an exercise on
  `from-char-broken`; `make failures` stays green.

## 9. Backlog entries this creates

Added to `BACKLOG.md` when this ships:

- Reader's answers — a per-exercise link to a Discussion or similar.
- Hidden solutions — an answer the book does not print.
