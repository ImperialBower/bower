---
type: CLI Command
title: bower verify
description: Runs each step's declared expectation against a real compiler, in a scratch directory outside the book, and reports which of the book's claims are not true.
tags: [command, verification, thesis]
timestamp: '2026-09-09T00:00:00Z'
---

# Usage

```
bower --book <DIR> verify [--repo <NAME>] [--step <ID> | --from <ID>] [--work <DIR>]
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

`Upheld` | `Broken { happened, command, stderr }` | `Skipped`.

Output is `ok  ` / `skip` / `FAIL` per step. A failure prints the chapter
anchor, the command, and the last 8 lines of stderr. The exit line is
`every claim holds`, or `N claim(s) in the book are not true`.

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

[1] `bower/src/verify.rs`, `bower/tests/verification.rs`
[2] [`bower-spec.md` §6](/references/bower-spec.md), `docs/EPIC-02_Verification.md`
