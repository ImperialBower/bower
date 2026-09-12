---
type: CLI Command
title: bower verify
description: Runs each step's declared expectation against a real compiler, in a scratch directory outside the book, and reports which of the book's claims are not true.
tags: [command, verification, thesis]
timestamp: '2026-09-12T00:00:00Z'
---

# Usage

```
bower --book <DIR> verify [--repo <NAME>] [--step <ID> | --from <ID>] [--work <DIR>] [--record]
```

`--step` and `--from` conflict. A step id no repo holds is an error
(`no step named \`{id}\` in any repo`) rather than a silent success.

# What it does

Writes each step's [tree state](/model/tree-state.md) to a scratch directory and
runs the repo's `check` and `verify` commands against it, comparing what happens
to what the book claimed. See [expectation](/model/expectation.md) for the
matrix.

**It never touches git**, so it works before `build` has ever run.

# The verdicts

`Upheld` | `Broken { happened, command, output }` | `Skipped`, where `output`
is both streams in the order they were written.

Output is `ok  ` / `skip` / `FAIL` per step. A failure prints the chapter
anchor, the command, and the last 8 lines of its output. The exit line is
`every claim holds`, or `N claim(s) in the book are not true`.

Each [output block](/model/captured-output.md) gets a row under its step:
`ok  ` / `drft` / `todo` / `skip`. A drift or a missing recording fails the run.

# Which toolchain

The inherited `RUSTUP_TOOLCHAIN` is always removed from a step's commands, so
whatever launched `bower` never decides. A tree's own `rust-toolchain.toml` (or
`rust-toolchain`) wins; else the repo's `toolchain` key in
[`bower.toml`](/config/bower-toml.md) applies. A step that records output with
neither gets a warning: what it records would depend on this machine's default
compiler.

Before a pinned step's commands, `verify` runs `rustc --version` and discards
its output. On a machine without the pinned toolchain, that is where rustup
installs it — so its install notes never reach a recording — and a failed
install stops the run as the verifier's error, not the book's.

# --record

Rewrites only the output fences that are drifted or not recorded yet, bottom-up
within a chapter, after checking every chapter path with `check_path`. Then it
re-plans and rewrites `bower.lock`. A fence that still matches is left alone, so
a `[...]` trim survives. A second run prints `nothing to record`.

# Where the scratch directory goes, and why

```
std::env::temp_dir()/bower-verify/<book-dir-name>
```

**Not** the book's `target/`. Most Rust books are themselves cargo workspaces,
and cargo refuses to build a package written inside one.

This is recorded in the source as a real defect: the old `target/bower-verify`
default reported all 20 of the sample book's true claims as false — *and no test
caught it, because every test passes `--work` explicitly*. When the condition
does occur, `verify` now names it rather than reporting every true claim as
false.

That note is worth keeping in mind generally: a test suite that always supplies
an override cannot test the default.

# Known gap

`bower verify` does not run the book's own gate. A real defect was caught by eye
rather than by tool as a result — it is the highest-value open item in
`docs/TECHNICAL_DEBT.md`. Verification is also sequential, though the spec
describes it as embarrassingly parallel.

# Citations

[1] `bower/src/verify.rs`, `bower/src/record.rs`, `bower/tests/verification.rs`
[2] [`bower-spec.md` §6](/references/bower-spec.md), `docs/EPIC-02_Verification.md`, `docs/EPIC-11_Diagnostics.md`
