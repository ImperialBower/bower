---
type: CLI Command
title: bower plan
description: Resolves the book into a plan, prints it, and writes bower.lock — the command that must be re-run whenever chapter edits move a step's anchor.
tags: [command, plan, lock]
timestamp: '2026-09-09T00:00:00Z'
---

# Usage

```
bower --book <DIR> plan [--repo <NAME>]
```

# What it does

Resolves the book through the [kernel](/architecture/crate-bower-core.md),
prints one line per [step](/model/step.md), and writes
[`bower.lock`](/config/bower-lock.md) to the book root.

```
hello-playbook — 20 steps
  step-001-cargo-init          pass feat: a package that builds
  step-011-test-that-fails     test_fail test: greet should ignore stray whitespace (failing)
  step-013-wont-compile        compile_fail feat: scratch module (does not compile)

wrote books/hello-playbook/bower.lock
```

# Why it must be re-run after editing prose

The lock records each step's **anchor as a chapter line number**. Adding a
paragraph to a chapter shifts every anchor below it, and
[`bower status`](/commands/status.md) correctly reports the lock as `STALE`:

```
lock      STALE — 4 line(s) differ; run `bower plan`
            - 001 cargo-init … anchor=src/ch01-a-repo-that-builds.md:6 …
            + 001 cargo-init … anchor=src/ch01-a-repo-that-builds.md:8 …
```

This is not a bug — it is the drift report doing its job. The fix is to re-plan
and commit the new lock.

Because that is easy to forget, `make plan` regenerates both books' locks and is
folded into `make build`, so a plain [`make ayce`](/operations/make-ayce.md)
never leaves a stale anchor behind.

# Why the lock is checked in

The resolved order lives in version control so that reordering shows up in diffs
and code review, rather than as a silent history rewrite on the next
[`bower build`](/commands/build.md).

# Citations

[1] `bower/src/main.rs`, `bower-core/src/plan.rs`
[2] [`bower-spec.md` §4, §7](/references/bower-spec.md)
