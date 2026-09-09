---
type: Domain Concept
title: Expectation — controlled failure as an executable claim
description: A step declares whether it should pass, fail to compile, or fail its tests — and `bower verify` asserts the declared outcome really happens.
tags: [model, verification, thesis]
timestamp: '2026-09-09T00:00:00Z'
---

# Why this exists

*Rust for Failures* is a book about controlled failing. A chapter that says
"this won't compile, and here's what the compiler tells you" is making a claim.
`expect=` turns that claim into something a machine checks.

This is the load-bearing feature of the whole system: **the book's thesis,
enforced by the book's own toolchain.** The day a new rustc changes a
diagnostic, the build says so before a reader does.

# The matrix

| `expect` | Kernel value | What [`bower verify`](/commands/verify.md) asserts |
|---|---|---|
| `pass` (default) | `Expect::Pass` | The check command succeeds **and** the verify command succeeds. |
| `compile_fail` | `Expect::CompileFail` | The check command **fails**. The verify command never runs. |
| `test_fail` | `Expect::TestFail` | The check command succeeds **and** the verify command fails. |
| `none` | `Expect::Skip` | Nothing. The step is skipped — prose-only steps, mid-refactor states. |

The commands come from [`bower.toml`](/config/bower-toml.md): `check` defaults
to `cargo check`, `verify` to `cargo test`.

# The verdict, and its wording

`Verdict` is `Upheld` | `Broken { happened, command, stderr }` | `Skipped`. When
a claim is broken, `happened` reports the *opposite* of what was promised, in
the book's own voice:

- a `compile_fail` step that compiled → `"it compiled"`
- a `test_fail` step whose tests passed → `"its tests passed"`

`bower verify` exits 0 with `every claim holds`, and otherwise
`N claim(s) in the book are not true`.

# Diagnostics are not snapshotted

An early open question asked whether `compile_fail` should match stderr against
a stored pattern, trybuild-style. **Decided failure-only** — see
[the decision](/decisions/failure-only-diagnostics.md). `verify` asserts the
command fails and captures stderr for the failure report, but matches nothing.

# Consistency within a step

Blocks that merge into one [step](/model/step.md) must agree. Conflicting
`expect` values across blocks sharing a step id is
`ConflictingExpectInStep`.

# In the sample book

[`hello-playbook`](/books/hello-playbook.md) exercises all of it: of its 20
steps, `011 test-that-fails` is `test_fail` and `013 wont-compile` is
`compile_fail`. Its `book.toml` sets `[output.html.playground] runnable = false`
deliberately — a `compile_fail` step is *supposed* to break, and the playground
would report a different error than the book's.

# Citations

[1] `bower/src/verify.rs`, `bower-core/src/directive.rs`
[2] [`bower-spec.md` §6](/references/bower-spec.md)
