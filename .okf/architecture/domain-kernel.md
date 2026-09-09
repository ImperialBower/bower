---
type: Architecture Pattern
title: The domain kernel pattern
description: Everything upstream of git replay is a pure core — no I/O, no git, no serialization in the public API — and CI asserts the purity rather than trusting it.
tags: [architecture, purity, kernel]
timestamp: '2026-09-09T00:00:00Z'
---

# The rule

Everything to the left of "git replay" in [the pipeline](/architecture/pipeline.md)
is a **domain kernel**. Input is chapter text; output is a
[`BookPlan`](/model/plan.md) and a sequence of [`TreeState`](/model/tree-state.md)s.

The kernel's contract, quoted from `bower-core/src/lib.rs`:

> * **No I/O.** Input is text handed in by the caller; output is values. No
>   filesystem, no git, no network.
> * **No serialization in the public API.** Lock-file *text* is produced as a
>   `String`; no format crate appears in any signature.
> * **Deterministic.** Given the same `BookSource` and `RepoCatalog`, `plan`
>   returns an identical `BookPlan`, down to every tree byte.
> * **Exhaustive, located errors.** Every failure is a `BowerError` carrying the
>   chapter and line that caused it. Errors are collected, not short-circuited.

And the direction of dependency:

> The replay layer (`bower` CLI), the mdBook preprocessor, and the publishing
> pipeline are all consumers of this crate, never the other way around.

# Purity is asserted, not assumed

`bower-core`'s `[dependencies]` section is empty, carrying the comment: *"None.
The kernel is pure by construction, and CI asserts `cargo tree` stays empty."*

The assertion is the `purity` target, folded into `make lint`:

```make
n=$$(cargo tree -p bower-core -e normal | wc -l | tr -d ' '); \
if [ "$$n" != "1" ]; then echo "kernel purity broken"; exit 1; fi
```

One line of output means one crate and no dependencies. If that line ever grows
a second row, something crossed a boundary it should not have.

# Where the boundary is actually drawn

The interesting question is not "what is pure" but "what gets to cross". The
answer is visible in `RepoSpec`, which carries exactly one field —
`keep_region_markers` — because that is the only *repository* setting that
changes a pure computation. Remotes, identities, verify commands, and link
templates all stay in the [CLI's config](/config/bower-toml.md).

`BookConfig::catalog()` is the single crossing point, and a test named
`config__catalog_carries_only_kernel_settings` pins it by equality on a
one-field struct. The boundary is enforced by a test, not by a convention.

# The one place the kernel touches a format

[`bower.lock`](/config/bower-lock.md) is rendered by `lock_text()` **inside** the
kernel, as a `String`. No serde. That keeps determinism and the lock format in
the same place, and it is why `bower status` compares rendered text rather than
parsing — a parser would be a second definition free to disagree.

# The controllability corollary

A kernel that is pure is *observable*; making it **controllable** is the
[testkit](/architecture/crate-bower-testkit.md)'s job — fixtures, generators, and
a state-coverage report shipped as part of the kernel's contract.

# Citations

[1] `bower-core/src/lib.rs`, `bower-core/Cargo.toml`, `Makefile`
[2] [`bower-spec.md` §2.2, §8](/references/bower-spec.md)
