---
type: Decision
title: gix, not git2, for replay
description: Decided 2026-09-01 (EPIC-01) — pure Rust keeps no C library in the supply chain, and deterministic commits need direct control of author and committer time.
tags: [decision, git, determinism, supply-chain]
timestamp: '2026-09-01T00:00:00Z'
---

# The question

Which git library should [`bower build`](/commands/build.md) replay through?

# The decision

**`gix`.** Recorded in EPIC-01, 1 September 2026.

# Why

Two reasons, and the second is the one that actually mattered:

1. **Supply-chain posture.** Pure Rust, so the dependency tree keeps no C
   library.
2. **Deterministic commits need direct control of author and committer time.**
   `gix` gives that plainly — `SignatureRef::time` is git's own time string, so
   the `+0000` offset is pinned by hand rather than inherited from whatever the
   build machine thinks its timezone is.

The spec had predicted the kernel would be unaffected by this choice, and it
was. That is the domain-kernel pattern paying out: a library swap at the I/O
edge never reached the pure core.

# The consequence nobody predicted

`gix::init` honours the machine's `init.defaultBranch`. That is a
machine-dependent input to a process whose whole point is having none, so
`replay.rs` pins `refs/heads/main` by writing `.git/HEAD` itself.

A related instinct shows up in [`bower status`](/commands/status.md), which reads
tags as loose refs rather than through `gix` at all: *"a drift report that shares
a library with the thing it inspects can share a bug with it."*

# Citations

[1] [`bower-spec.md` §12 Q1](/references/bower-spec.md), `docs/EPIC-01_Replay.md`
[2] `bower/src/replay.rs`
