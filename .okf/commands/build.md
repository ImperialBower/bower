---
type: CLI Command
title: bower build
description: Replays the plan into a real git repository with gix — one commit per step, annotated tags, trailers linking back to the book, and byte-identical SHAs across runs.
tags: [command, git, replay, determinism]
timestamp: '2026-09-09T00:00:00Z'
---

# Usage

```
bower --book <DIR> build [--repo <NAME>] [-o|--out <DIR>]
```

`--out` defaults to `out`. With one repo selected it is used as-is; with several,
each repo gets `out/<repo>`.

# What it does

Replays the [plan](/model/plan.md) into a git repository, from scratch. The
output directory is **removed and recreated** — never merged — because
[determinism](/architecture/invariants.md) makes full regeneration cheap and
removes a class of drift bugs.

# The sequence

1. **Init**, with `refs/heads/main` written into `.git/HEAD` by hand. `gix::init`
   would otherwise honour the machine's `init.defaultBranch`, which is a
   machine-dependent input into a process that must not have any.
2. **Step 0 — scaffolding**, if the repo declares a `template`. Message:
   `chore: initial commit — scaffolding`. It deliberately carries **no**
   `Book-Source` trailer, since a trailer naming a chapter that did not produce
   the commit would be a lie.
3. **One commit per [step](/model/step.md)**, whose blobs are the scaffolding
   plus that step's [tree state](/model/tree-state.md). The final step swaps in
   a tree that also contains the generated `STEPS.md`.
4. **An annotated tag per step** (`step-012-rank-enum`), plus a
   `<chapter-stem>-end` tag at each chapter boundary.
5. **A materialized worktree and index**, so the result is `cd`-able and
   `git status`-clean rather than a bare repository.

# How the SHAs stay identical

```
stamp(seq) = epoch + seq minutes   →  "{unix_timestamp} +0000"
```

The `epoch` comes from [`bower.toml`](/config/bower-toml.md); the offset is
pinned to `+0000` by hand; author and committer are the same fixed signature
from `[identity]`. No clock, no environment variable, no machine identity enters
the commit.

# Trailers

Every generated commit links back to the book:

```
Book-Source: {book}/{chapter}#step-{id}
Book-Url:    {site}/…              (only when [book] site is set)
Bower-Step:  {repo}/{seq:03}
Generated-By: bower v{version}
```

# Safety

`write_tree` calls `check_all(blobs)` and rejects `..` paths itself rather than
relying on `gix` happening to. `file=` is book-controlled data, and this was a
real, fixed defect — see `docs/DEFECT_Path_Traversal.md`.

# Citations

[1] `bower/src/replay.rs`, `bower/src/trailers.rs`
[2] [`bower-spec.md` §5.1–§5.3](/references/bower-spec.md), `docs/EPIC-01_Replay.md`
