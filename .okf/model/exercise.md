---
type: Domain Concept
title: Exercise
description: A step can be a place to stop reading and start typing — the next step is the answer, folded away in HTML.
tags: [model, rendering, pedagogy]
timestamp: '2026-09-09T00:00:00Z'
---

# What it is

An exercise turns a [step](/model/step.md) into a "your turn". The **next step
of the same repo is the answer** — which is why an exercise may sit on any step
but the last, and why a step carries at most one.

# Two forms

`ExerciseForm::Key` — the task rides on a step's own directive:

```markdown
<!-- bower repo="rust4failures" file="src/rank.rs" exercise="Make this compile" -->
```

The box prints after that step's last block and its checkout line.

`ExerciseForm::Block` — the exercise gets its own directive, with a fenced
markdown body that becomes the box's *detail* and never renders as code:

`````markdown
<!-- bower repo="rust4failures" exercise="Make this compile" -->
````markdown
Every `char` needs an answer, not just the card ones.

- Try a lookup table instead of a `match`.
````
`````

# What the box contains

1. `**Your turn: {task}**`, then the detail lines.
2. Only when the repo declares both `clone` and `checkout` templates: a fork or
   clone line, then a `console` block with the clone command, the checkout
   command, and the reader's command to run.
3. A closing line naming the answer.

The reader's command is chosen to match what [`bower verify`](/commands/verify.md)
would run for that step's [expectation](/model/expectation.md) — `links.check`
after a `compile_fail`, `links.verify` otherwise, and none for `none`. That
correspondence is enforced in [config](/config/bower-toml.md), where `check` and
`verify` fall back to the same defaults the verifier uses, **so the box can
never print a command the tool would not run**.

Without a remote configured, the box keeps the task, the detail, and the closing
line, and simply drops every command. It never invents a link.

# The closing line, by expectation

| Expectation | Line |
|---|---|
| `compile_fail`, `test_fail` | `Green means you did it. The answer is the next step.` |
| `pass` | `Keep it green. The next step shows one way.` |
| `none` | `The next step shows one way.` |

HTML links "the next step" to `#step-{answer}`; epub and PDF print it bare,
having no anchor to land on.

# Folding the answer

In HTML only, the **answer step's** blocks are wrapped in
`<details class="step-answer">`, with a summary chosen from the *question's*
expectation:

- `"Show the answer"` after a `compile_fail` or `test_fail`
- `"Show one way"` after a `pass` or `none`

The distinction is pedagogical: a red step has *an* answer; a green step has one
way among several.

Wrapping differs by target: HTML uses `<div class="step-exercise">`; epub and
PDF use a blockquote.

# Placement rules are kernel errors

| Error | Rule |
|---|---|
| `ExerciseDuplicate` | At most one per step; reported at the second. |
| `ExerciseWithoutAnswer` | Not the repo's last step — nothing follows to be the answer. |
| `ExerciseUnbound` | A block-form exercise with no preceding step to attach to. |
| `ExerciseUnknownStep` | Names a step that does not exist. |
| `ExerciseConflictingKeys` | A [play cell](/model/play-cell.md) may not also be an exercise. |

# Citations

[1] `bower/src/render.rs`, `bower-core/src/plan.rs`
[2] `docs/superpowers/specs/2026-09-06-exercises-design.md`
