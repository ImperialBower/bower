---
type: Configuration
title: bower.lock
description: The resolved plan as human-readable text, checked in so reordering shows up in review — written by the kernel, and never parsed.
tags: [config, lock, determinism]
timestamp: '2026-09-09T00:00:00Z'
---

# What it is

A generated, human-readable manifest of the resolved [plan](/model/plan.md): one
line per [step](/model/step.md), grouped by repo. It lives at the book root and
**is checked in**, so that reordering appears in diffs and code review rather
than as a silent history rewrite.

# The grammar

```
{seq:03} {id} expect={expect} anchor={chapter}:{line} files={comma,joined}[ play={n}]
```

with, for a step carrying one, an indented continuation:

```
    exercise form={form} answer={answer} task={task:?}
    | {detail}
```

# A real fragment

```
# bower.lock — generated; review, don't edit

[hello-playbook]
001 cargo-init expect=pass anchor=src/ch01-a-repo-that-builds.md:8 files=Cargo.toml,src/main.rs
002 hello-runs expect=pass anchor=src/ch01-a-repo-that-builds.md:42 files=
005 rustfmt expect=pass anchor=src/ch03-lints-and-format.md:9 files=rustfmt.toml
```

`files=` is empty for a step that writes no file — a step can exist to carry a
commit message alone.

# Written by the kernel; never parsed

`lock_text()` lives in `bower-core/src/plan.rs` and is **the only definition of
this format.** There is no parser anywhere in the codebase.

[`bower status`](/commands/status.md) compares *rendered text* against the file
on disk, as a line-set difference. That is deliberate: a parser would be a
second definition of the format, free to disagree with the writer. Comparing
rendered output means the two can never drift.

# The anchor is a line number — so prose edits invalidate it

`anchor=src/ch01-a-repo-that-builds.md:8` is a chapter line. Insert a paragraph
above a directive and every anchor below it moves, making the lock stale.

That is the report working correctly, not a defect. The remedy is
[`bower plan`](/commands/plan.md), which `make plan` runs for both books as part
of `make build` so a sweep never leaves a stale anchor behind.

# Citations

[1] `bower-core/src/plan.rs`, `bower/src/status.rs`
[2] [`bower-spec.md` §4](/references/bower-spec.md)
