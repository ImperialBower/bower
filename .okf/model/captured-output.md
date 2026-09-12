---
type: Domain Concept
title: Captured output
description: What a step's check or verify command printed, recorded as a fence in the chapter and checked by bower verify.
tags: [model, verification, rendering]
timestamp: '2026-09-12T00:00:00Z'
---

# What it is

An `output="check"` or `output="verify"` directive, followed by a fence, binds
to a [step](/model/step.md) the way a [play cell](/model/play-cell.md) does: an
explicit `step=`, else the nearest preceding code block of the repo. It never
touches a tree, so recording one moves no SHA.

The fence holds the **normalized** output of that command at that step. An
empty fence means "not recorded yet".

# Normalization

Eight rules, in order, all pure kernel code (`bower-core/src/capture.rs`):
CRLF to LF; ANSI escapes removed; the scratch tree, target directory, and
`$CARGO_HOME` replaced; cargo's right-aligned status lines dropped; the
panicking thread's ID removed; `; finished in N.NNs` removed; runs of
`test … ok` lines sorted; trailing whitespace and outer blank lines trimmed.
Cargo's summary lines (`error: could not compile …`) stay.

# Matching

A fence without a `[...]` line must equal the live output. A `[...]` line
stands for zero or more lines; the other lines must appear in order, and a
piece at either end is anchored there.

# The verifier

Only an **upheld** step's outputs are judged. A step with an output block runs
with one build job, one test thread, and `RUST_BACKTRACE=0`. Rows are
`ok` / `drft` / `todo` / `skip`; a drift or a missing recording fails
[`bower verify`](/commands/verify.md). `--record` rewrites only drifted and
missing fences, then the [lock](/config/bower-lock.md).

# Errors

| Error | Rule |
|---|---|
| `OutputUnbound` | No preceding step to attach to. |
| `OutputUnknownStep` | Names a step that does not exist. |
| `OutputConflictingKeys` | Carries a tree key, `notebook`, `exercise`, `expect`, or `include`. |
| `OutputDuplicate` | A second block for one step and capture. |
| `OutputNeverRuns` | `verify` on a `compile_fail` step, or anything on a `none` step. |

# Citations

[1] `bower-core/src/capture.rs`, `bower-core/src/plan.rs`, `bower/src/verify.rs`, `bower/src/record.rs`
[2] `docs/EPIC-11_Diagnostics.md`
