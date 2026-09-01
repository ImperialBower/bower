# Hello Playbook — a sample Bower book

**Status:** approved design, not yet implemented
**Date:** 2026-08-31
**Repo:** `ImperialBower/bower`
**Relates to:** `bower-spec.md` Draft 0.2 (§ 3 annotations, § 4 ordering, § 3.6 scaffolding)

---

## 1. Purpose

Build the first real Bower book: a short mdBook whose annotated code blocks
generate a complete `dev-playbook`-style Rust hello-world repository, one
commit per teaching step.

It serves three jobs at once:

1. **A live test fixture.** `bower-testkit` plans the book on every `cargo test`
   and asserts the resulting plan, tags, and file trees. The book cannot rot,
   because CI reads it.
2. **The reference example.** It is the thing an author copies when starting a
   Bower book. Every directive key that matters appears in it, in context.
3. **A proof of the pipeline.** It shows end to end that a book can be the
   single source of truth for a repository, using only the Phase 1 kernel.

## 2. Non-goals

- No git replay. Phase 1 produces values; turning them into commits is Phase 2.
- No mdBook preprocessor. The book renders as plain mdBook; the directives are
  HTML comments and stay invisible.
- No `bower.toml` parsing. The file is written as documentation of Phase 2
  intent; nothing reads it yet.
- Not a complete Rust tutorial. The subject is the *foundation* — build, gate,
  lint, test, scan, CI — not the language.

## 3. Layout

```
books/hello-playbook/
  book.toml                          # mdBook config
  bower.toml                         # repo catalog + link templates (Phase 2)
  src/
    SUMMARY.md
    ch01-a-repo-that-builds.md
    ch02-the-gate.md
    ch03-lints-and-format.md
    ch04-tests-and-failing-on-purpose.md
    ch05-supply-chain.md
    ch06-ci.md
  template/                          # step-0 scaffolding, never shown on the page
    README.md                        # "generated from the book; do not open PRs here"
    LICENSE-MIT
    LICENSE-APACHE
    LICENSE-GPLv3
    CODE_OF_CONDUCT.md
    CONTRIBUTING.md
    SECURITY.md
    .gitignore
```

The target repo name is `hello-playbook`. Every directive carries
`repo="hello-playbook"`.

## 4. The generated repository

Final tree after the last step:

```
.github/workflows/ci.yml
.tool-versions
Cargo.toml
Makefile
bin/security-scan
deny.toml
rust-toolchain.toml
rustfmt.toml
src/lib.rs
src/main.rs
```

plus the eight files the `template/` directory contributes at step 0.

## 5. Steps

Every step declares an explicit `step="…"` id, so tags are stable and readable
(`PlannedStep::tag()` renders `step-{seq:03}-{id}`). Steps are in document
order; no `after=` is used, because the teaching order *is* the build order.

| # | Chapter | Step id | Files | Op(s) | `expect` |
|---|---|---|---|---|---|
| 1 | ch01 | `cargo-init` | `Cargo.toml`, `src/main.rs` | create, create | pass |
| 2 | ch01 | `hello-runs` | — | none (prose) | pass |
| 3 | ch02 | `makefile` | `Makefile` | create | pass |
| 4 | ch02 | `make-help` | `Makefile` | region `help` | pass |
| 5 | ch03 | `rustfmt` | `rustfmt.toml` | create | pass |
| 6 | ch03 | `toolchain` | `rust-toolchain.toml` | create | pass |
| 7 | ch03 | `lints` | `Cargo.toml` | region `lints` | pass |
| 8 | ch03 | `gate-fmt-clippy` | `Makefile` | region `gate` | pass |
| 9 | ch04 | `greet-lib` | `src/lib.rs`, `src/main.rs` | create, replace | pass |
| 10 | ch04 | `greet-test` | `src/lib.rs` | append | pass |
| 11 | ch04 | `test-that-fails` | `src/lib.rs` | region `tests` | **test_fail** |
| 12 | ch04 | `test-that-passes` | `src/lib.rs` | region `tests` | pass |
| 13 | ch04 | `wont-compile` | `src/lib.rs` | region `scratch` | **compile_fail** |
| 14 | ch04 | `scratch-gone` | `src/lib.rs` | region `scratch` | pass |
| 15 | ch05 | `deny-toml` | `deny.toml` | create | pass |
| 16 | ch05 | `security-scan` | `bin/security-scan` | create | pass |
| 17 | ch05 | `gate-audit` | `Makefile` | region `gate` | pass |
| 18 | ch06 | `tool-versions` | `.tool-versions` | create | pass |
| 19 | ch06 | `ci-workflow` | `.github/workflows/ci.yml` | create | pass |
| 20 | ch06 | `drop-scratch` | `src/scratch.rs` | delete | pass |

Twenty steps. Op coverage: `create`, `replace`, `region`, `append`, `delete`,
`none`. `copy` is already covered by the existing fixture corpus and is not
forced into this book.

Note on step 20: `src/scratch.rs` is created in ch04 as the home for the
`compile_fail` demonstration and deleted in ch06, so `delete` has an honest
reason to exist rather than a manufactured one.

## 6. Region markers

Regions are the mechanism that lets a file grow across chapters while each page
shows only its new lines. This book establishes five:

| File | Region | Established | Edited by |
|---|---|---|---|
| `Makefile` | `help` | step 3 | step 4 |
| `Makefile` | `gate` | step 3 | steps 8, 17 |
| `Cargo.toml` | `lints` | step 1 | step 7 |
| `src/lib.rs` | `tests` | step 10 | steps 11, 12 |
| `src/scratch.rs` | `scratch` | step 13 | step 14 |

Markers are ordinary comments in the host language (`# bower:begin gate` in
Make and TOML, `// bower:begin tests` in Rust) and are stripped from the
materialized tree, because the book's `RepoSpec` leaves `keep_region_markers`
at its default of `false`.

## 7. Display markers

Chapters 4 and 6 use `// bower:show` spans, so that a full compiling file can be
committed while the page prints only the lines that carry the lesson. Chapters 1
to 3 use no display markers, so the whole block renders — the small-example path
that pays no tax.

## 8. The test contract

Two additions to `bower-testkit`:

**`fixtures::hello_playbook()`** — a new `Fixture` whose chapters are pulled in
with `include_str!` from `books/hello-playbook/src/`. Because `include_str!` is
resolved at compile time, the kernel still receives nothing but text and the
testkit performs no runtime I/O. The fixture joins `fixtures::valid()`, so the
existing corpus tests and the state-coverage report pick it up for free.

**`bower-testkit/tests/sample_book.rs`** — asserts, in this order:

1. `plan(&book, &catalog)` returns `Ok`.
2. The repo plan has exactly 20 steps.
3. Every step's `tag()` matches the expected list in § 5, in order.
4. The final `TreeState` contains exactly the paths listed in § 4 (template
   files excluded — the kernel never sees them).
5. `Cargo.toml`, `Makefile`, and `src/lib.rs` in the final tree match golden
   text held in the test.
6. No region marker survives into any materialized tree.
7. Steps `test-that-fails` and `wont-compile` carry `Expect::TestFail` and
   `Expect::CompileFail` respectively.
8. Planning the same book twice yields byte-identical `BookPlan` values.

## 9. Known gaps

These are stated so no reader mistakes intent for capability.

1. **`template/` is inert in Phase 1.** Step-0 scaffolding is applied by the
   replay layer, which does not exist. The files are written, correct, and
   unused. The test asserts nothing about them.
2. **`expect` is recorded, not verified.** The kernel writes down that a step
   should fail to compile or fail its tests. Actually running `cargo` and
   confirming the failure is Phase 2's `bower verify`. Chapter 4's argument is
   therefore true in the plan and unproven in fact, for now.
3. **`bower.toml` is documentation.** No configuration parser exists yet.
## 10. Verification

```
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
mdbook build books/hello-playbook     # optional; proves the prose renders
```

The book is done when all three are clean and the state-coverage report still
reports complete coverage.
