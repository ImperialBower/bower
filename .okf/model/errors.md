---
type: Domain Concept
title: Errors — located and collected
description: One non-exhaustive enum of 37 variants, each carrying the chapter and line that caused it, reported all at once rather than one rebuild at a time.
tags: [model, kernel, errors]
timestamp: '2026-09-12T00:00:00Z'
---

# The posture

Plan-time errors are first-class here, not an afterthought. The kernel's own doc
comment puts it plainly:

> **Exhaustive, located errors.** Every failure is a `BowerError` carrying the
> chapter and line that caused it. Errors are collected, not short-circuited —
> one pass reports everything it can.

The spec goes further: *"the kernel's error enum is a chapter draft for Rust for
Failures."* The tool for a book about failing well is itself built to fail well.

# The type

`BowerError` — `#[non_exhaustive]`, `Clone + Debug + Eq + Hash + PartialEq`,
implements `Display` and `std::error::Error`. **37 variants**, grouped by what
goes wrong:

| Group | Variants |
|---|---|
| Directive syntax | `DirectiveParse`, `UnknownKey`, `BadValue`, `MissingKey`, `UnknownRepo`, `DirectiveWithoutBlock`, `UnclosedFence` |
| Block library and assets | `IncludeMissing`, `IncludeMalformed`, `AssetMissing` |
| Step composition | `DuplicateFileInStep`, `ConflictingRepoInStep`, `ConflictingExpectInStep` |
| Tree folding | `FileAlreadyExists`, `FileNotCreated`, `RegionMissing`, `RegionUnbalanced` |
| [Ordering](/model/ordering.md) | `OrphanAfter`, `OrderingCycle` |
| [Display spans](/model/display-markers.md) | `UnclosedShowSpan`, `NestedShowSpan`, `OrphanShowEnd`, `UnknownShowSpan`, `SpanNotInTree` |
| [Play cells](/model/play-cell.md) | `PlayCellUnbound`, `PlayCellUnknownStep`, `PlayCellConflictingKeys` |
| [Exercises](/model/exercise.md) | `ExerciseUnbound`, `ExerciseUnknownStep`, `ExerciseDuplicate`, `ExerciseWithoutAnswer`, `ExerciseConflictingKeys` |
| [Captured output](/model/captured-output.md) | `OutputUnbound`, `OutputUnknownStep`, `OutputConflictingKeys`, `OutputDuplicate`, `OutputNeverRuns` |

# Location

Every variant carries `loc: Location` **except `OrderingCycle`**, which carries
`repo` and `steps: Vec<String>` — a cycle is a property of a set of steps, not
of a single line. `BowerError::location() -> Option<&Location>` reflects exactly
that asymmetry.

`Display` renders `{loc}: message`, and `Location`'s own `Display` is
grep-shaped, so kernel output drops straight into an editor's error list:

```
ch01.md:7: repo `nope` is not declared in the catalog
```

# The collector

```rust
pub struct Errors(pub Vec<BowerError>);
```

> Bower reports everything it can find in one run rather than stopping at the
> first problem — an author fixing a chapter should not need ten rebuilds to see
> ten mistakes.

`Errors`' own `Display` writes one error per line.

# Two variants worth knowing about

- **`SpanNotInTree`** is documented as *"a kernel invariant violation surfaced as
  an error rather than a panic — if you see it, the line-map property test has a
  gap."* It should be unreachable; it exists so that a bug is a message rather
  than a crash.
- **`PlayCellConflictingKeys`** refuses a category confusion rather than
  guessing: a notebook cell that also names `file`/`op`/`region`/`src`/`paths`
  is not a cell with extra keys, it is a mistake about what a cell *is*.

# Every variant has a fixture

[`bower-testkit`](/architecture/crate-bower-testkit.md) ships roughly 31 broken
fixtures, each named for the variant it must produce. "Errors, not surprises" is
an [invariant](/architecture/invariants.md) with a test behind it.

# Citations

[1] `bower-core/src/lib.rs`
[2] [`bower-spec.md` §6.1](/references/bower-spec.md)
