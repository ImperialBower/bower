---
type: Crate
title: bower-testkit
description: The controllability half of the kernel — a fixture corpus covering every error variant, proptest generators over arbitrary valid books, and a state-coverage report.
resource: https://github.com/ImperialBower/bower/tree/main/bower-testkit
tags: [crate, testing, controllability]
timestamp: '2026-09-09T00:00:00Z'
---

# The idea

A pure kernel is *observable*. The testkit makes it **controllable** — fake data
generation, data textures, and state coverage shipped as part of the kernel's
contract rather than hidden in a `tests/` directory.

The spec puts the ambition plainly: *"The tool for the Controllability book gets
built the way the Controllability book says tools should be built."*

# Three modules

## `fixtures.rs` — the corpus

`Fixture { name, book: BookSource, catalog: RepoCatalog }`.

- **`valid()`** returns 10 named books: `minimal`, `rank_saga`, `multi_repo`,
  `display_textures`, `op_sampler`, `include_library`, `self_hosting_chapter`,
  `notebook_play`, `exercise_forms`, `hello_playbook`.
- **`broken()`** returns roughly 31 cases, **each named for the
  [`BowerError`](/model/errors.md) variant it must produce** — assembled from
  `broken_source()`, `broken_tree()`, `broken_display()` and
  `broken_exercises()`.

It also exports `HELLO_PLAYBOOK_TAGS` (all 20) and
`HELLO_PLAYBOOK_FINAL_PATHS`, so the kernel tests and the CLI tests assert
against one list rather than two that can drift.

## `generators.rs` — proptest strategies

`arb_book() -> impl Strategy<Value = (BookSource, RepoCatalog)>`, built from
`GenFile { create, mods }` and `GenMod { replace, lines, marked }`.

Generated lines are drawn from `[a-z]{1,8}` so they can never accidentally
collide with a directive, a fence, a marker, or a heading. **Every generated
book must plan cleanly** — one that errors is itself a bug in the generator or
the kernel.

## `coverage.rs` — state coverage

`CoverageReport { ops, expects, mechanisms, exercise_expects }` over the corpus:
the state-space analogue of code coverage. `Mechanism` has 10 variants —
`HiddenLines`, `Regions`, `ShowMarkers`, `ShowFilter`, `Include`,
`MultiBlockStep`, `MultiRepo`, `AfterConstraint`, `PlayCell`, `Exercise`.

`corpus_state_coverage_is_complete` is the test that turns the report into an
[invariant](/architecture/invariants.md): the corpus must exercise every op,
every expectation, and every mechanism.

# Test suites

| File | What it proves |
|---|---|
| `tests/corpus.rs` | Every valid fixture plans; every broken fixture produces its named error. |
| `tests/properties.rs` | Determinism, generated-book validity, line-map exactness, lock stability. |
| `tests/sample_book.rs` | The same assertions against the real on-disk [hello-playbook](/books/hello-playbook.md) chapters. |

That last suite matters: the corpus proves the kernel handles *constructed*
books, and `sample_book.rs` proves it handles the one a reader will actually
read.

# Citations

[1] `bower-testkit/src/`, `bower-testkit/tests/`
[2] [`bower-spec.md` §8](/references/bower-spec.md)
