---
type: Playbook
title: make ayce — the gate
description: All-you-can-eat, the default target and the whole pre-push sweep — plus the rule that keeps its prerequisite list from ever changing.
tags: [operations, make, ci, gate]
timestamp: '2026-09-09T00:00:00Z'
---

# The command

```
make ayce      # the whole gate; also the default target
make help      # every target, self-documented
make slow      # the #[ignore]d lanes
```

`ayce` is "all-you-can-eat: full pre-push sweep", and `default: ayce`.

# The contract

`ayce` runs a fixed list: `check-dependencies`, then `clean`, `fmt`, `build`,
`test`, `lint`, `security-scan`, `docs`.

**Those targets never change.** `check-dependencies` is the exception in kind —
it is a gate rather than a step, and runs first so a missing tool is named in a
second instead of surfacing as a test failure minutes in.

# The folding rule

This project's own invariants are extra targets, **folded into `build` and
`lint`** so a plain `make ayce` runs them without altering the contract:

| Folded into | Target | What it asserts |
|---|---|---|
| `build` | `minimal` | The CLI compiles with `--no-default-features`. |
| `build` | `plan` | Both books' [locks](/config/bower-lock.md) are regenerated, so an edited chapter never leaves a stale anchor. |
| `lint` | `purity` | `cargo tree -p bower-core -e normal` prints exactly one line. |

`fmt-check` exists separately, for CI: the same formatter as `fmt`, asked
instead of told.

If the purity line ever grows a second row, something crossed a boundary it
should not have. See [the domain kernel pattern](/architecture/domain-kernel.md).

# Why `plan` is folded in

Chapter anchors in `bower.lock` are line numbers. Adding a paragraph shifts
them, `bower status` correctly reports `lock STALE`, and `bower/tests/status.rs`
fails — which is the test suite doing its job, but it reads like a mysterious
breakage to whoever edited the prose.

Regenerating both locks during `build` closes the loop: the sweep either passes,
or leaves a reviewable lock diff to commit.

# The slow lane

`make slow` runs the five tool- and compiler-dependent lanes that are
`#[ignore]`d in the fast build:

```
cargo test -p bower --test verification -- --ignored     # a real compiler, per step
cargo build -p bower --all-features
… --test preprocessor -- --ignored                        # needs mdbook on PATH
… --test publish -- --ignored                             # needs pandoc, typst
cargo test -p bower --lib -- --ignored
```

# Citations

[1] `Makefile`, `bin/check-dependencies`
[2] [`README.md`](https://github.com/ImperialBower/bower/blob/main/README.md)
