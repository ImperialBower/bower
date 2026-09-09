---
type: Domain Concept
title: Book source
description: The pure input to the kernel — chapters in reading order, an optional block library, and binary assets — with no filesystem behind it.
tags: [model, kernel, input]
timestamp: '2026-09-09T00:00:00Z'
---

# What it is

`BookSource` is everything the [kernel](/architecture/domain-kernel.md) is
allowed to see. It is handed in by the caller as values; the kernel never opens
a file to get it.

| Field | Type | Meaning |
|---|---|---|
| `chapters` | `Vec<Chapter>` | Reading order — which is `SUMMARY.md` order, and therefore the default [step order](/model/ordering.md). |
| `library` | `BTreeMap<String, String>` | The block library, addressed by the `include=` key on a [directive](/model/directive.md). |
| `assets` | `BTreeMap<String, Vec<u8>>` | Raw bytes for `op="copy"`. |

Constructed in tests and doc examples with `BookSource::from_chapters`.

# Chapter

`Chapter { path, text }`. Its `stem()` strips directories and the `.md` suffix
— `src/ch01-ranks.md` becomes `ch01-ranks` — and that stem is what derives
commit subjects and auto step ids. See [steps](/model/step.md).

# Location

`Location { chapter: String, line: usize }`, 1-based. Its `Display` is
deliberately grep-shaped:

```
src/ch03.md:42
```

Every [error](/model/errors.md) and every step anchor carries one. This is the
single mechanism that lets Bower say *which line of which chapter* is wrong,
rather than merely that something is.

# Repo catalog

The book declares which repositories exist; a [directive](/model/directive.md)
naming any other is an `UnknownRepo` error.

- `RepoName(pub String)`
- `RepoSpec { keep_region_markers: bool }`
- `RepoCatalog(pub BTreeMap<RepoName, RepoSpec>)`

`RepoSpec` holds **only settings that affect pure computation**. Remotes,
identities, and verify commands are the replay layer's business and never cross
into the kernel — [`bower.toml`](/config/bower-toml.md) projects the catalog and
drops the rest. A test (`config__catalog_carries_only_kernel_settings`) pins that
boundary by equality on a one-field struct.

# Why this shape matters

Because the input is a value, the [plan](/model/plan.md) is a pure function of
it. That is what makes [determinism](/architecture/invariants.md) provable rather
than hoped for, and it is why the mdBook preprocessor, the CLI, and the
publishing pipeline can all be consumers of one kernel without any of them
owning a second copy of the grammar.

# Citations

[1] `bower-core/src/source.rs`
[2] [`bower-spec.md` §2, §8](/references/bower-spec.md)
