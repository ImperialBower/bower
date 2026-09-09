---
type: Crate
title: bower (the CLI)
description: The I/O half — reads config, resolves the plan through the kernel, replays it into git with gix, verifies expectations against a real compiler, and reports drift.
resource: https://github.com/ImperialBower/bower/tree/main/bower
tags: [crate, cli, rust]
timestamp: '2026-09-09T00:00:00Z'
---

# What it is

Everything the [kernel](/architecture/crate-bower-core.md) refuses to do:
filesystem, git, subprocesses, network, and serde.

> Replay a book into deterministic git repositories.

# Two binaries, one crate

`autobins = false`, `default-run = "bower"`.

| Binary | Source | Notes |
|---|---|---|
| `bower` | `bower/src/main.rs` | The CLI. See [commands](/commands/index.md). |
| `mdbook-bower` | `bower/src/mdbook_bower.rs` | `required-features = ["preprocessor"]`. See [the preprocessor](/architecture/mdbook-bower.md). |

The `preprocessor` feature is on by default and gates `dep:serde_json`. `make
build` also compiles with `--no-default-features` (the `minimal` target) so the
CLI never quietly grows a hard dependency on it.

# The module map

| Module | Owns |
|---|---|
| `config.rs` | [`bower.toml`](/config/bower-toml.md) loading, and `catalog()` — the only crossing into the kernel. |
| `replay.rs` | `build`: gix, commits, tags, worktree. |
| `trailers.rs` | Commit trailers, and the `STEPS.md` marker writer/reader pair. |
| `verify.rs` | The [expectation](/model/expectation.md) matrix against real commands. |
| `status.rs` | Lock, repo, and site [drift](/commands/status.md). |
| `push.rs`, `forge.rs` | [Publishing](/commands/push.md), the marker guard, releases, Pages. |
| `publish.rs`, `render.rs` | [Render plan and renderers](/commands/publish.md). |
| `mdbook.rs`, `mdbook_bower.rs` | The preprocessor protocol. |

# Notable implementation decisions

- **The branch is pinned by hand.** `BRANCH = "refs/heads/main"`, written
  directly into `.git/HEAD`, because `gix::init` otherwise honours the machine's
  `init.defaultBranch` — a machine-dependent input into a deterministic process.
- **Path traversal is checked here, not delegated.** `write_tree` calls
  `check_all(blobs)` to reject `..` paths itself "rather than relying on `gix`
  happening to". `file=` is book-controlled data; see
  `docs/DEFECT_Path_Traversal.md` and `bower/tests/traversal.rs`.
- **An unmatched `--repo` is a failure, in every command.** It prints
  `bower: no repo matched` and exits non-zero, because empty output plus exit 0
  reads as success.
- **`deny_unknown_fields` on every config struct**, so a typo in `bower.toml` is
  a load-time error rather than a silently ignored setting.

# Test suites

| File | Invariant it guards |
|---|---|
| `determinism.rs` | Byte-identical SHAs; dirty-directory replay matches empty. |
| `multi_repo.rs` | Books feeding more than one repo; per-repo output dirs. |
| `preprocessor.rs` | The real mdBook stdin/stdout protocol, not crate internals. |
| `publish.rs` | The render plan as a pure fold, assertable with no renderer installed. |
| `push.rs` | The push command and the marker guard. Never touches the network. |
| `site.rs` | The site-branch flow end to end against a `FakeForge`. |
| `status.rs` | Drift reporting *and exit codes*. |
| `traversal.rs` | Path-traversal regressions. |
| `verification.rs` | `expect` claims against a real compiler. |

# Citations

[1] `bower/src/`, `bower/Cargo.toml`, `bower/tests/`
[2] [`bower-spec.md` §8](/references/bower-spec.md)
