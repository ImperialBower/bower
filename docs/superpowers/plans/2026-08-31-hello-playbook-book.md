# Hello Playbook Book — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `books/hello-playbook/`, a six-chapter mdBook whose annotated code blocks generate a complete dev-playbook-style Rust hello-world repository in twenty steps, and wire it into `bower-testkit` as a live, CI-checked fixture.

**Architecture:** The book is real markdown on disk. `bower-testkit` pulls each chapter in with `include_str!`, which resolves at compile time, so the kernel still receives nothing but text and the testkit performs no runtime I/O. A new fixture `fixtures::hello_playbook()` joins `fixtures::valid()`, so the existing corpus and state-coverage tests adopt the book for free. A dedicated test file asserts the plan shape, the tags, the final tree, and determinism.

**Tech Stack:** Rust 1.95.0, edition 2021, `bower-core` + `bower-testkit` (no new dependencies), mdBook for optional rendering.

**Spec:** `docs/superpowers/specs/2026-08-31-hello-playbook-book-design.md`

## Global Constraints

- Target repo name is `hello-playbook`, spelled exactly that way in every `repo="…"` attribute.
- Generated package: `edition = "2021"`, `rust-version = "1.95"`, `channel = "1.95.0"` — matching the `bower` workspace itself. Do not invent other versions.
- Licence string in the generated `Cargo.toml`: `MIT OR Apache-2.0 OR GPL-3.0-or-later`.
- **Add no dependencies** to `bower-core` or `bower-testkit`. The kernel is zero-dependency by contract.
- `cargo clippy --workspace --all-targets` must stay at zero warnings under `clippy::pedantic`.
- **Never run a state-changing git command.** Every "commit" step below means: print the command for the human to run, and stop until they confirm.
- Every chapter file is UTF-8 with LF line endings and no trailing whitespace.

## Three kernel facts you must not forget

These were verified against `bower-core/src/tree.rs` and `bower-core/src/block.rs` before this plan was written. Getting them wrong is the most likely way to waste an hour.

1. **Hidden-line stripping is Rust-only.** `assemble()` applies mdBook's `# `-hidden rule only when the fence info string starts with `rust`. A `#` line inside a `makefile`, `toml`, `bash`, `yaml`, or `text` fence survives byte-for-byte. This is why `# bower:begin gate` works in a Makefile.
2. **Marker comments are multi-language.** `comment_body()` accepts `//`, `#`, `--`, and `;`, and matches only if the remainder starts with `bower:` or `bf:`. Ordinary Makefile comments and the `#!/usr/bin/env bash` shebang are left alone.
3. **A block-less directive must be followed by prose, not a fence.** `capture_block()` skips blank lines and then grabs the next fence it sees. So after an `op="none"` or `op="delete"` directive you MUST write a paragraph of text before any code fence, or that unrelated fence is swallowed as the directive's block.

## File Structure

**Created:**

- `books/hello-playbook/book.toml` — mdBook config. Renders the prose; knows nothing about Bower.
- `books/hello-playbook/bower.toml` — the repo catalog and link templates. Documentation of Phase 2 intent; nothing parses it yet.
- `books/hello-playbook/src/SUMMARY.md` — chapter order. This order is the step order.
- `books/hello-playbook/src/ch01-a-repo-that-builds.md` … `ch06-ci.md` — the six chapters. Each owns one topic and the steps that build it.
- `books/hello-playbook/template/` — eight step-0 scaffolding files. Inert in Phase 1 (spec § 9.1).

**Modified:**

- `bower-testkit/src/fixtures.rs` — add `hello_playbook()` and register it in `valid()`.

**Created (test):**

- `bower-testkit/tests/sample_book.rs` — the assertions specific to this book.

---

### Task 1: Book skeleton, chapter 1, and the fixture that reads it

This task is deliberately the largest, because a vertical slice that plans end-to-end is worth more than six half-built pieces. It creates the directory, the inert template, chapter 1, the fixture, and the first passing assertion.

**Files:**
- Create: `books/hello-playbook/book.toml`
- Create: `books/hello-playbook/bower.toml`
- Create: `books/hello-playbook/src/SUMMARY.md`
- Create: `books/hello-playbook/src/ch01-a-repo-that-builds.md`
- Create: `books/hello-playbook/template/README.md`, `LICENSE-MIT`, `LICENSE-APACHE`, `LICENSE-GPLv3`, `CODE_OF_CONDUCT.md`, `CONTRIBUTING.md`, `SECURITY.md`, `.gitignore`
- Modify: `bower-testkit/src/fixtures.rs`
- Test: `bower-testkit/tests/sample_book.rs`

**Interfaces:**
- Consumes: `bower_core::prelude::{BookSource, Chapter, RepoCatalog, BookPlan, PlannedStep, plan}`; the existing private helpers `Fixture::new`, `chapter` in `fixtures.rs`.
- Produces: `bower_testkit::fixtures::hello_playbook() -> Fixture`; the test helpers `book_plan() -> BookPlan` and `tags(&BookPlan) -> Vec<String>` in `tests/sample_book.rs`.

- [ ] **Step 1: Create the book directory and its config files**

`books/hello-playbook/book.toml`:

```toml
[book]
title = "Hello, Playbook"
description = "A Rust project foundation, taught as a book that builds its own repository."
authors = ["ImperialBower"]
language = "en"
src = "src"

[output.html]
default-theme = "ayu"
```

`books/hello-playbook/bower.toml` — Phase 2 input, unread today:

```toml
# Nothing parses this file yet. It records what the replay layer will need,
# so the book is complete as an example even while Phase 2 is unwritten.

[repos.hello-playbook]
template = "template"
keep_region_markers = false

[repos.hello-playbook.links]
blob = "https://github.com/ImperialBower/hello-playbook/blob/{tag}/{path}#L{start}-L{end}"
```

`books/hello-playbook/src/SUMMARY.md`:

```markdown
# Summary

- [A repo that builds](ch01-a-repo-that-builds.md)
- [The gate](ch02-the-gate.md)
- [Lints and format](ch03-lints-and-format.md)
- [Tests, and failing on purpose](ch04-tests-and-failing-on-purpose.md)
- [Supply chain](ch05-supply-chain.md)
- [CI](ch06-ci.md)
```

- [ ] **Step 2: Create the eight template files**

These are the scaffolding the generated repo gets at step 0 and the book never shows. They are inert in Phase 1 — no test reads them — but the book is not honest without them.

`books/hello-playbook/template/README.md`:

```markdown
# hello-playbook

**This repository is generated. Do not open pull requests here.**

Every commit in this repository was produced by replaying the book
[Hello, Playbook](https://github.com/ImperialBower/bower/tree/main/books/hello-playbook).
Each commit's trailer names the chapter and line that produced it, and each
tag (`step-001-cargo-init`, and so on) is the state of the code at one
teaching step.

To change the code, change the book.
```

`books/hello-playbook/template/.gitignore`:

```gitignore
/target
Cargo.lock
```

For the remaining six, copy them verbatim from the dev-playbook skill assets rather than retyping them:

```bash
SKILL=/Users/christoph/.claude/skills/dev-playbook/assets
DEST=books/hello-playbook/template
cp "$SKILL/licenses/LICENSE-MIT"    "$DEST/LICENSE-MIT"
cp "$SKILL/licenses/LICENSE-APACHE" "$DEST/LICENSE-APACHE"
cp "$SKILL/licenses/LICENSE-GPLv3"  "$DEST/LICENSE-GPLv3"
cp "$SKILL/CODE_OF_CONDUCT.md"      "$DEST/CODE_OF_CONDUCT.md"
cp "$SKILL/CONTRIBUTING.md.tmpl"    "$DEST/CONTRIBUTING.md"
cp "$SKILL/SECURITY.md.tmpl"        "$DEST/SECURITY.md"
```

The two `.tmpl` files each contain the single placeholder token `{{project}}`. Substitute it:

```bash
sed -i '' 's/{{project}}/hello-playbook/g' books/hello-playbook/template/CONTRIBUTING.md books/hello-playbook/template/SECURITY.md
grep -rn '{{' books/hello-playbook/template/   # must print nothing
```

- [ ] **Step 3: Write chapter 1**

`books/hello-playbook/src/ch01-a-repo-that-builds.md`. Note the `lints` region: it is created empty here and filled in chapter 3. Note also that the `hello-runs` directive is followed immediately by a paragraph — kernel fact 3.

`````markdown
# A repo that builds

Every project starts the same way: a manifest that names it, and one file that
runs. Nothing else has earned its place yet.

<!-- bower repo="hello-playbook" step="cargo-init" file="Cargo.toml" msg="feat: a package that builds" -->

```toml
[package]
name = "hello-playbook"
version = "0.1.0"
edition = "2021"
rust-version = "1.95"
license = "MIT OR Apache-2.0 OR GPL-3.0-or-later"

[dependencies]

# bower:begin lints
# bower:end lints
```

The two comment lines at the bottom are a *region marker*. They mark a hole in
the file that a later chapter will fill, and they are stripped out before the
file reaches the repository. Chapter 3 fills this one.

<!-- bower repo="hello-playbook" step="cargo-init" file="src/main.rs" -->

```rust
fn main() {
    println!("Hello, world!");
}
```

Both blocks above carry `step="cargo-init"`. Blocks that share a step id become
one commit, which is right: a manifest without a source file is not a state of
the project anyone would want to check out.

## Running it

<!-- bower repo="hello-playbook" step="hello-runs" op="none" msg="docs: cargo run prints the greeting" -->

This step writes no files. `op="none"` declares a commit that exists only to
mark a moment in the narrative — here, the moment you can first run the thing.
The command is:

```console
$ cargo run
Hello, world!
```

That fence has no directive above it, so Bower ignores it. Only fenced blocks
introduced by a `<!-- bower … -->` comment become code in the repository.
`````

- [ ] **Step 4: Write the failing test**

Create `bower-testkit/tests/sample_book.rs`:

```rust
//! The sample book, `books/hello-playbook/`, planned and checked.
//!
//! The chapters are real files on disk, pulled in by `fixtures::hello_playbook()`
//! at compile time. If a chapter's directives drift, this test fails.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use bower_testkit::prelude::*;

const REPO: &str = "hello-playbook";

/// Every expected tag, in order. Grows one task at a time.
const EXPECTED_TAGS: &[&str] = &["step-001-cargo-init", "step-002-hello-runs"];

fn book_plan() -> BookPlan {
    let f = fixtures::hello_playbook();
    match plan(&f.book, &f.catalog) {
        Ok(p) => p,
        Err(errs) => panic!("the sample book must plan cleanly:\n{errs}"),
    }
}

fn tags(p: &BookPlan) -> Vec<String> {
    p.repo(REPO)
        .expect("the sample book targets `hello-playbook`")
        .steps
        .iter()
        .map(PlannedStep::tag)
        .collect()
}

#[test]
fn sample_book__plans_cleanly_with_the_expected_tags() {
    let p = book_plan();
    assert_eq!(tags(&p), EXPECTED_TAGS);
}
```

- [ ] **Step 5: Run the test to verify it fails**

Run: `cargo test -p bower-testkit --test sample_book`
Expected: FAIL to compile, with `cannot find function 'hello_playbook' in module 'fixtures'`.

- [ ] **Step 6: Add the fixture**

In `bower-testkit/src/fixtures.rs`, add next to the existing `failers()` helper:

```rust
fn hello_playbook_catalog() -> RepoCatalog {
    RepoCatalog::from_names(&["hello-playbook"])
}

/// The sample book: a dev-playbook-shaped Rust hello world, taught in six
/// chapters that build the repository they describe.
///
/// The chapters live on disk under `books/hello-playbook/src/` and are pulled
/// in with `include_str!`, which resolves at compile time — the kernel still
/// sees nothing but text, and the testkit does no runtime I/O.
#[must_use]
pub fn hello_playbook() -> Fixture {
    Fixture::new(
        "hello-playbook",
        BookSource::from_chapters(vec![chapter(
            "ch01-a-repo-that-builds.md",
            include_str!("../../books/hello-playbook/src/ch01-a-repo-that-builds.md"),
        )]),
        hello_playbook_catalog(),
    )
}
```

Then add the fixture to `valid()` in the same file (it is at roughly line 250 and returns `vec![minimal(), rank_saga(), …]`). Append `hello_playbook(),` as the last entry, so the existing corpus and coverage tests adopt the book.

- [ ] **Step 7: Run the tests to verify they pass**

Run: `cargo test -p bower-testkit`
Expected: PASS. `sample_book__plans_cleanly_with_the_expected_tags` passes, and the pre-existing `every_valid_fixture_plans_cleanly` and `corpus_state_coverage_is_complete` still pass.

If the tag assertion fails with only `step-001-cargo-init`, the `hello-runs` prose directive was not recognised — check that a paragraph, not a fence, follows it.

- [ ] **Step 8: Commit**

Print this for the human to run. Do not run it yourself.

```bash
git add books/hello-playbook bower-testkit/src/fixtures.rs bower-testkit/tests/sample_book.rs && git commit -m "feat(books): hello-playbook chapter 1, wired in as a live fixture"
```

---

### Task 2: Chapter 2 — the Makefile and the gate

**Files:**
- Create: `books/hello-playbook/src/ch02-the-gate.md`
- Modify: `bower-testkit/src/fixtures.rs` (one more chapter)
- Modify: `bower-testkit/tests/sample_book.rs` (two more tags)

**Interfaces:**
- Consumes: `fixtures::hello_playbook()`, `EXPECTED_TAGS` from Task 1.
- Produces: the `Makefile` regions `help` and `gate`, which Tasks 3 and 5 edit.

**TABS.** Makefile recipe lines must begin with one literal tab character (0x09). Spaces will produce a Makefile that Make rejects. The block content is copied into the repository byte for byte, so the tabs must be in the markdown.

- [ ] **Step 1: Extend the expected tags**

In `bower-testkit/tests/sample_book.rs`, change `EXPECTED_TAGS` to:

```rust
const EXPECTED_TAGS: &[&str] = &[
    "step-001-cargo-init",
    "step-002-hello-runs",
    "step-003-makefile",
    "step-004-make-help",
];
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p bower-testkit --test sample_book`
Expected: FAIL, `assertion failed: left has 2 elements, right has 4`.

- [ ] **Step 3: Write chapter 2**

`books/hello-playbook/src/ch02-the-gate.md`:

`````markdown
# The gate

One command has to mean "check everything". In these books that command is
`make ayce` — *all your code, evaluated* — and its meaning never varies from
project to project. What varies is what it depends on, and that list grows as
the book does.

<!-- bower repo="hello-playbook" step="makefile" file="Makefile" msg="build: one command runs everything" -->

```makefile
.DEFAULT_GOAL := ayce

# bower:begin help
# bower:end help

# bower:begin gate
GATE := build

build:
	cargo build --workspace

.PHONY: build
# bower:end gate

ayce: $(GATE)
	@echo "ayce — all your code, evaluated"

.PHONY: ayce
```

The `GATE` variable lives *inside* the region, which is the whole trick. Later
chapters replace the region wholesale and the `ayce` target picks up the new
prerequisites without ever being edited again.

<!-- bower repo="hello-playbook" step="make-help" file="Makefile" op="region" region="help" msg="build: make help lists the targets" -->

```makefile
help:
	@echo "ayce   run the whole gate"
	@echo "build  compile the workspace"
	@echo "help   this list"

.PHONY: help
```

`op="region"` replaces the text between the two `help` markers and leaves the
rest of the file alone. The page shows you five lines; the repository gets the
whole Makefile. Note that the block does *not* repeat the marker lines — they
stay in the file, waiting for the next edit.
`````

- [ ] **Step 4: Verify the tabs survived**

Run: `grep -cP '^\t' books/hello-playbook/src/ch02-the-gate.md`
Expected: `5` — the five recipe lines. If it prints `0`, your editor converted tabs to spaces; fix it before continuing.

- [ ] **Step 5: Register the chapter**

In `bower-testkit/src/fixtures.rs`, add a second `chapter(…)` entry to the vector in `hello_playbook()`:

```rust
            chapter(
                "ch02-the-gate.md",
                include_str!("../../books/hello-playbook/src/ch02-the-gate.md"),
            ),
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p bower-testkit`
Expected: PASS.

- [ ] **Step 7: Commit**

Print this for the human to run:

```bash
git add books/hello-playbook/src/ch02-the-gate.md bower-testkit && git commit -m "feat(books): hello-playbook chapter 2, the make gate"
```

---

### Task 3: Chapter 3 — lints and format

**Files:**
- Create: `books/hello-playbook/src/ch03-lints-and-format.md`
- Modify: `bower-testkit/src/fixtures.rs`
- Modify: `bower-testkit/tests/sample_book.rs`

**Interfaces:**
- Consumes: the `Cargo.toml` `lints` region from Task 1, the `Makefile` `gate` region from Task 2.
- Produces: `rustfmt.toml`, `rust-toolchain.toml`; the `gate` region now names `build fmt lint`.

- [ ] **Step 1: Extend the expected tags**

Append these four entries to `EXPECTED_TAGS`:

```rust
    "step-005-rustfmt",
    "step-006-toolchain",
    "step-007-lints",
    "step-008-gate-fmt-clippy",
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p bower-testkit --test sample_book`
Expected: FAIL, left has 4 elements, right has 8.

- [ ] **Step 3: Write chapter 3**

`books/hello-playbook/src/ch03-lints-and-format.md`. The `gate` block has tabs on its recipe lines, same rule as chapter 2.

`````markdown
# Lints and format

Formatting and linting are not taste. They are the cheapest possible tests: they
run in a second and they catch a class of mistake before a human ever reads the
diff. So they belong in the gate, and the gate belongs in one command.

## Pin the format

<!-- bower repo="hello-playbook" step="rustfmt" file="rustfmt.toml" msg="style: pin the formatter" -->

```toml
# Only max_width is set. Everything else is rustfmt's default on purpose —
# a config file full of overrides is a config file nobody can read.
max_width = 100
```

## Pin the toolchain

<!-- bower repo="hello-playbook" step="toolchain" file="rust-toolchain.toml" msg="build: pin the toolchain" -->

```toml
[toolchain]
channel = "1.95.0"
components = ["clippy", "rustfmt"]
```

Anyone who clones this repository gets that exact compiler, with those exact
components, without being told to install anything. The version appears again in
`.tool-versions` in chapter 6, and the two must always agree.

## Fill the lints region

<!-- bower repo="hello-playbook" step="lints" file="Cargo.toml" op="region" region="lints" msg="style: forbid unsafe, warn on pedantic" -->

```toml
[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
pedantic = { level = "warn", priority = -1 }
unwrap_used = "warn"
expect_used = "warn"
```

This is the hole chapter 1 left open. The `priority = -1` is not decoration:
without it, `pedantic` as a lint *group* would override the individual lints
listed after it.

## Put them in the gate

<!-- bower repo="hello-playbook" step="gate-fmt-clippy" file="Makefile" op="region" region="gate" msg="build: fmt and clippy join the gate" -->

```makefile
GATE := build fmt lint

build:
	cargo build --workspace

fmt:
	cargo fmt --all --check

lint:
	cargo clippy --workspace --all-targets -- -D warnings

.PHONY: build fmt lint
```

`--check` rather than a plain `cargo fmt`, and `-D warnings` rather than a
plain `clippy`: a gate that quietly fixes things is not a gate. It must fail.
`````

- [ ] **Step 4: Verify the tabs survived**

Run: `grep -cP '^\t' books/hello-playbook/src/ch03-lints-and-format.md`
Expected: `3`.

- [ ] **Step 5: Register the chapter**

Add to the vector in `hello_playbook()`:

```rust
            chapter(
                "ch03-lints-and-format.md",
                include_str!("../../books/hello-playbook/src/ch03-lints-and-format.md"),
            ),
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p bower-testkit`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add books/hello-playbook/src/ch03-lints-and-format.md bower-testkit && git commit -m "feat(books): hello-playbook chapter 3, lints and format"
```

---

### Task 4: Chapter 4 — tests, and failing on purpose

This is the chapter the whole book exists for. It commits a failing test, tags it, and fixes it in the next commit. It also commits code that does not compile, on purpose, and repairs that too.

**Files:**
- Create: `books/hello-playbook/src/ch04-tests-and-failing-on-purpose.md`
- Modify: `bower-testkit/src/fixtures.rs`
- Modify: `bower-testkit/tests/sample_book.rs`

**Interfaces:**
- Consumes: `src/main.rs` from Task 1.
- Produces: `src/lib.rs` with regions `mods`, `greet`, `tests`; `src/scratch.rs` with region `scratch`. Task 6 empties `mods` and deletes `scratch.rs`.

**Blank-line discipline.** The `mods` region is created empty and sits directly between the crate doc comment and the `greet` region, with **no blank line** between `// bower:end mods` and `// bower:begin greet`. The blank line that separates them in the finished file is carried *inside* the region's content instead. This matters: markers are stripped at materialization, so a blank line on each side of an empty region would collapse into two consecutive blank lines, and `cargo fmt --check` — the gate this very book installs — rejects that. Do not "tidy" the spacing.

- [ ] **Step 1: Extend the expected tags**

Append to `EXPECTED_TAGS`:

```rust
    "step-009-greet-lib",
    "step-010-greet-test",
    "step-011-test-that-fails",
    "step-012-test-that-passes",
    "step-013-wont-compile",
    "step-014-scratch-fixed",
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p bower-testkit --test sample_book`
Expected: FAIL, left has 8 elements, right has 14.

- [ ] **Step 3: Write chapter 4**

`books/hello-playbook/src/ch04-tests-and-failing-on-purpose.md`:

`````markdown
# Tests, and failing on purpose

A test that has never failed has never been tested. This chapter writes one that
fails, commits it in that state, and fixes it in the next commit — so the
failure is a real, tagged, checkoutable state of the repository rather than
something that happened off-camera.

## A library to test

<!-- bower repo="hello-playbook" step="greet-lib" file="src/lib.rs" msg="feat: greet(), extracted into a library" -->

```rust
//! A greeting, and nothing else.

// bower:begin mods
// bower:end mods
// bower:begin greet
/// Build a greeting for `name`.
#[must_use]
pub fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}
// bower:end greet
```

The `mods` region is empty and has no blank line around it. Its blank line will
arrive as part of its contents later, so that emptying the region again does not
leave a double blank line behind — which the formatter, installed in chapter 3,
would reject.

<!-- bower repo="hello-playbook" step="greet-lib" file="src/main.rs" op="replace" -->

```rust
use hello_playbook::greet;

fn main() {
    println!("{}", greet("world"));
}
```

`op="replace"` writes a whole new file over the old one. Both blocks share
`step="greet-lib"`, so extracting the function and rewiring the binary are one
commit — the repository never passes through a state where `main.rs` refers to
something that is not there yet.

## A test that passes

<!-- bower repo="hello-playbook" step="greet-test" file="src/lib.rs" op="append" msg="test: greet uses the name it is given" -->

```rust

// bower:begin tests
#[cfg(test)]
mod tests {
    use super::greet;

    #[test]
    fn greet__uses_the_name() {
        assert_eq!(greet("world"), "Hello, world!");
    }
}
// bower:end tests
```

`op="append"` adds to the bottom of a file without repeating its top. The block
opens with a blank line, which is how the separating blank line gets there.

## A test that fails

<!-- bower repo="hello-playbook" step="test-that-fails" file="src/lib.rs" op="region" region="tests" expect="test_fail" msg="test: greet should ignore stray whitespace (failing)" -->

```rust
#[cfg(test)]
mod tests {
    use super::greet;

    #[test]
    fn greet__uses_the_name() {
        assert_eq!(greet("world"), "Hello, world!");
    }

    // bower:show
    #[test]
    fn greet__ignores_stray_whitespace() {
        assert_eq!(greet("  world  "), "Hello, world!");
    }
    // bower:show end
}
```

Two things are happening here.

`expect="test_fail"` declares that this commit's tests are *supposed* to fail.
That is a promise the toolchain checks: when the replay layer builds this
repository it runs the tests at this step and treats a pass as the error.

The `bower:show` markers decide what you just read. The block above contains the
whole test module, because the repository needs the whole file — but only the
new test was printed on the page. The markers never reach the repository.

## The fix

<!-- bower repo="hello-playbook" step="test-that-passes" file="src/lib.rs" op="region" region="greet" msg="fix: greet ignores stray whitespace" -->

```rust
/// Build a greeting for `name`, ignoring stray whitespace.
#[must_use]
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name.trim())
}
```

The fix is in a different region of the same file. That is the point of regions:
the test module and the function it tests grow independently, and each edit
shows only itself.

## Code that does not compile

<!-- bower repo="hello-playbook" step="wont-compile" file="src/scratch.rs" expect="compile_fail" msg="feat: scratch module (does not compile)" -->

```rust
// bower:begin scratch
/// Deliberately wrong: a string literal is not a `u32`.
#[must_use]
pub fn answer() -> u32 {
    let n: u32 = "42";
    n
}
// bower:end scratch
```

<!-- bower repo="hello-playbook" step="wont-compile" file="src/lib.rs" op="region" region="mods" -->

```rust
pub mod scratch;

```

The second block is not decoration. A file that no module declaration points at
is never compiled, so a `compile_fail` in an undeclared file would be a lie
about the toolchain. Declaring the module and writing the broken code are one
commit, and that commit genuinely does not build.

Notice the blank line at the end of that block. That is the separator promised
earlier — it belongs to the region, not to the file around it.

## The repair

<!-- bower repo="hello-playbook" step="scratch-fixed" file="src/scratch.rs" op="region" region="scratch" msg="fix: parse the text instead of pretending it is a number" -->

```rust
/// Parse the text, and fall back to zero when it is not a number.
#[must_use]
pub fn answer() -> u32 {
    "42".parse().unwrap_or_default()
}
```

`unwrap_or_default` rather than `unwrap`: chapter 3 set `unwrap_used = "warn"`,
and the gate turns warnings into failures.
`````

- [ ] **Step 4: Register the chapter**

Add to the vector in `hello_playbook()`:

```rust
            chapter(
                "ch04-tests-and-failing-on-purpose.md",
                include_str!("../../books/hello-playbook/src/ch04-tests-and-failing-on-purpose.md"),
            ),
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p bower-testkit`
Expected: PASS.

Two failures are worth naming in advance. `ConflictingExpectInStep` on
`wont-compile` means you put `expect="compile_fail"` on both of its blocks —
it belongs on one. `RegionMissing` for `greet` means step 9's markers were
edited or reflowed; regions are matched by exact marker text.

- [ ] **Step 6: Add the expectation assertions**

Append this test to `bower-testkit/tests/sample_book.rs`:

```rust
#[test]
fn sample_book__records_the_two_deliberate_failures() {
    let p = book_plan();
    let repo = p.repo(REPO).expect("repo present");
    let expect_of = |id: &str| {
        repo.steps
            .iter()
            .find(|s| s.id.0 == id)
            .unwrap_or_else(|| panic!("no step `{id}`"))
            .expect
    };

    assert_eq!(expect_of("test-that-fails"), Expect::TestFail);
    assert_eq!(expect_of("wont-compile"), Expect::CompileFail);
    assert_eq!(expect_of("test-that-passes"), Expect::Pass);
    assert_eq!(expect_of("scratch-fixed"), Expect::Pass);
}
```

- [ ] **Step 7: Run the new test to verify it passes**

Run: `cargo test -p bower-testkit --test sample_book`
Expected: PASS, both tests.

- [ ] **Step 8: Commit**

```bash
git add books/hello-playbook/src/ch04-tests-and-failing-on-purpose.md bower-testkit && git commit -m "feat(books): hello-playbook chapter 4, controlled failure"
```

---

### Task 5: Chapter 5 — supply chain

**Files:**
- Create: `books/hello-playbook/src/ch05-supply-chain.md`
- Modify: `bower-testkit/src/fixtures.rs`
- Modify: `bower-testkit/tests/sample_book.rs`

**Interfaces:**
- Consumes: the `Makefile` `gate` region as Task 3 left it (`build fmt lint`).
- Produces: `deny.toml`, `bin/security-scan`; the `gate` region now names `build fmt lint test audit`.

- [ ] **Step 1: Extend the expected tags**

Append to `EXPECTED_TAGS`:

```rust
    "step-015-deny-toml",
    "step-016-security-scan",
    "step-017-gate-test-audit",
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p bower-testkit --test sample_book`
Expected: FAIL, left has 14 elements, right has 17.

- [ ] **Step 3: Write chapter 5**

`books/hello-playbook/src/ch05-supply-chain.md`. Tabs on the recipe lines again.

`````markdown
# Supply chain

Your code is a small fraction of what you ship. The rest arrives from a
registry, written by people you will never meet, and it changes underneath you.
Two tools watch it, and one script gives them a single name.

## Rules for dependencies

<!-- bower repo="hello-playbook" step="deny-toml" file="deny.toml" msg="chore: cargo-deny configuration" -->

```toml
[advisories]
version = 2

[licenses]
version = 2
# Exactly the licences this project actually needs. A longer list is not
# safer — it is a longer list of things you have stopped checking.
allow = ["Apache-2.0", "GPL-3.0-or-later", "MIT"]

[bans]
multiple-versions = "warn"
```

## One name for "scan"

<!-- bower repo="hello-playbook" step="security-scan" file="bin/security-scan" msg="chore: one definition of the security scan" -->

```bash
#!/usr/bin/env bash
# One definition of "scan". The Makefile calls this; so does CI. When the
# scan changes, it changes in one place.
set -euo pipefail

cargo audit
cargo deny check advisories bans licenses sources
```

`set -euo pipefail` is not boilerplate. Without it a failing `cargo audit` in
the middle of the script would be shrugged off and the gate would pass while
the advisory stood.

## Into the gate

<!-- bower repo="hello-playbook" step="gate-test-audit" file="Makefile" op="region" region="gate" msg="build: tests and the security scan join the gate" -->

```makefile
GATE := build fmt lint test audit

build:
	cargo build --workspace

fmt:
	cargo fmt --all --check

lint:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

audit:
	./bin/security-scan

.PHONY: build fmt lint test audit
```

The tests written in chapter 4 join the gate here, alongside the scan. `ayce`
itself has not been edited since chapter 2 — it never needs to be, because its
prerequisite list lives inside the region.
`````

- [ ] **Step 4: Verify the tabs survived**

Run: `grep -cP '^\t' books/hello-playbook/src/ch05-supply-chain.md`
Expected: `5`.

- [ ] **Step 5: Register the chapter**

```rust
            chapter(
                "ch05-supply-chain.md",
                include_str!("../../books/hello-playbook/src/ch05-supply-chain.md"),
            ),
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p bower-testkit`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add books/hello-playbook/src/ch05-supply-chain.md bower-testkit && git commit -m "feat(books): hello-playbook chapter 5, supply chain"
```

---

### Task 6: Chapter 6 — CI, and cleaning up

**Files:**
- Create: `books/hello-playbook/src/ch06-ci.md`
- Modify: `bower-testkit/src/fixtures.rs`
- Modify: `bower-testkit/tests/sample_book.rs`

**Interfaces:**
- Consumes: `src/scratch.rs` and the `mods` region from Task 4.
- Produces: the final tree — `src/scratch.rs` deleted, `mods` empty again.

- [ ] **Step 1: Resolve the checkout action's major version**

Do not copy a version from this plan. Ask GitHub:

```bash
gh api repos/actions/checkout/releases/latest --jq .tag_name
```

Use the major it prints (for example `v5`) in the workflow below. The two other
actions are referenced by name rather than version — `dtolnay/rust-toolchain@stable`
and `taiki-e/install-action@cargo-audit` — and are copied verbatim, never resolved.

- [ ] **Step 2: Extend the expected tags**

Append to `EXPECTED_TAGS`:

```rust
    "step-018-tool-versions",
    "step-019-ci-workflow",
    "step-020-drop-scratch",
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test -p bower-testkit --test sample_book`
Expected: FAIL, left has 17 elements, right has 20.

- [ ] **Step 4: Write chapter 6**

`books/hello-playbook/src/ch06-ci.md`. Substitute the checkout major you resolved in Step 1 where this plan writes `v5`.

`````markdown
# CI

CI does not need its own idea of what "good" means. It has one job: check out
the code, install the toolchain, and run the same gate you run locally.

## One version, declared once more

<!-- bower repo="hello-playbook" step="tool-versions" file=".tool-versions" msg="chore: declare the toolchain for version managers" -->

```text
rust 1.95.0
```

This has to agree with `rust-toolchain.toml` from chapter 3. Two places that
name a version are two places that can disagree, so the rule in these books is
that every version-declaring file is updated together, always.

## The workflow

<!-- bower repo="hello-playbook" step="ci-workflow" file=".github/workflows/ci.yml" msg="ci: run the gate on every push and pull request" -->

```yaml
name: ci

on:
  push:
    branches: [main]
  pull_request:

jobs:
  gate:
    runs-on: ubuntu-latest
    steps:
      # bower:show
      - uses: actions/checkout@v5
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - uses: taiki-e/install-action@cargo-audit
      - uses: taiki-e/install-action@cargo-deny
      - run: make ayce
      # bower:show end
```

The last line is the whole argument. CI runs `make ayce` and nothing else. There
is no second list of checks to drift out of step with the first.

## Clearing the scratch

<!-- bower repo="hello-playbook" step="drop-scratch" file="src/scratch.rs" op="delete" msg="chore: remove the scratch module" -->

The broken module did its job in chapter 4 and has no reason to stay. This
directive has no code block after it — `op="delete"` names a file and removes
it, and the paragraph you are reading is what stops the next fence from being
mistaken for its contents.

Deleting the file is not enough on its own. `src/lib.rs` still declares the
module, and a declaration pointing at nothing does not compile, so the same
commit empties the `mods` region:

<!-- bower repo="hello-playbook" step="drop-scratch" file="src/lib.rs" op="region" region="mods" -->

```rust
```

That block really is empty. The region returns to the state chapter 4 created it
in, and the file goes back to having exactly one blank line where the module
declaration used to be.
`````

- [ ] **Step 5: Register the chapter**

```rust
            chapter(
                "ch06-ci.md",
                include_str!("../../books/hello-playbook/src/ch06-ci.md"),
            ),
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p bower-testkit`
Expected: PASS, all twenty tags.

- [ ] **Step 7: Commit**

```bash
git add books/hello-playbook/src/ch06-ci.md bower-testkit && git commit -m "feat(books): hello-playbook chapter 6, CI and cleanup"
```

---

### Task 7: Close the contract — trees, goldens, determinism

The tags prove the shape. This task proves the *content*.

**Files:**
- Modify: `bower-testkit/tests/sample_book.rs`

**Interfaces:**
- Consumes: `book_plan()`, `tags()`, `REPO` from Task 1.
- Produces: nothing later tasks depend on. This is the last task.

- [ ] **Step 1: Write the failing tests**

Append all four to `bower-testkit/tests/sample_book.rs`:

```rust
/// Every path the book's twenty steps leave behind. Template scaffolding is
/// not here: the kernel never sees it (spec § 9.1).
const FINAL_PATHS: &[&str] = &[
    ".github/workflows/ci.yml",
    ".tool-versions",
    "Cargo.toml",
    "Makefile",
    "bin/security-scan",
    "deny.toml",
    "rust-toolchain.toml",
    "rustfmt.toml",
    "src/lib.rs",
    "src/main.rs",
];

fn final_tree() -> TreeState {
    let p = book_plan();
    let repo = p.repo(REPO).expect("repo present");
    repo.steps
        .last()
        .expect("the book has steps")
        .tree
        .clone()
}

#[test]
fn sample_book__final_tree_holds_exactly_the_expected_files() {
    let tree = final_tree();
    let paths: Vec<&str> = tree.paths().collect();
    assert_eq!(paths, FINAL_PATHS);
}

#[test]
fn sample_book__no_region_marker_survives_into_any_tree() {
    let p = book_plan();
    for step in &p.repo(REPO).expect("repo present").steps {
        for path in step.tree.paths() {
            let Some(text) = step.tree.text(path) else {
                continue;
            };
            assert!(
                !text.contains("bower:"),
                "{} still carries a marker at {}",
                path,
                step.tag()
            );
        }
    }
}

#[test]
fn sample_book__cargo_toml_and_lib_rs_match_their_goldens() {
    let tree = final_tree();

    assert_eq!(
        tree.text("Cargo.toml").expect("Cargo.toml exists"),
        concat!(
            "[package]\n",
            "name = \"hello-playbook\"\n",
            "version = \"0.1.0\"\n",
            "edition = \"2021\"\n",
            "rust-version = \"1.95\"\n",
            "license = \"MIT OR Apache-2.0 OR GPL-3.0-or-later\"\n",
            "\n",
            "[dependencies]\n",
            "\n",
            "[lints.rust]\n",
            "unsafe_code = \"forbid\"\n",
            "\n",
            "[lints.clippy]\n",
            "pedantic = { level = \"warn\", priority = -1 }\n",
            "unwrap_used = \"warn\"\n",
            "expect_used = \"warn\"\n",
        )
    );

    let lib = tree.text("src/lib.rs").expect("src/lib.rs exists");
    assert!(lib.starts_with("//! A greeting, and nothing else.\n\n/// Build a greeting"));
    assert!(lib.contains("format!(\"Hello, {}!\", name.trim())"));
    assert!(lib.contains("fn greet__ignores_stray_whitespace()"));
    assert!(!lib.contains("pub mod scratch;"));
    assert!(
        !lib.contains("\n\n\n"),
        "two blank lines in a row — `cargo fmt --check` would reject this:\n{lib}"
    );
}

#[test]
fn sample_book__makefile_ends_with_the_full_gate() {
    let tree = final_tree();
    let mk = tree.text("Makefile").expect("Makefile exists");
    assert!(mk.contains("GATE := build fmt lint test audit"));
    assert!(mk.contains("audit:\n\t./bin/security-scan"));
    assert!(mk.contains(".DEFAULT_GOAL := ayce"));
}

#[test]
fn sample_book__planning_twice_is_byte_identical() {
    assert_eq!(book_plan(), book_plan());
}
```

- [ ] **Step 2: Run the tests to verify they fail or pass honestly**

Run: `cargo test -p bower-testkit --test sample_book`

The golden assertions above were computed by hand from the chapter blocks, so
one of them may differ from the real output by a blank line. If a golden fails:
read the actual value in the diff, satisfy yourself it is *correct*, and only
then paste it into the test. Never adjust a golden you have not read.

Two failures are not adjustable and mean the book is wrong, not the test:

- `sample_book__no_region_marker_survives_into_any_tree` failing means a marker
  is malformed and was never recognised as a marker.
- The `\n\n\n` assertion failing means the `mods` region has a blank line on
  both sides. Re-read the blank-line discipline note in Task 4.

- [ ] **Step 3: Run the whole workspace gate**

Run: `cargo test --workspace`
Expected: PASS, including `every_valid_fixture_plans_cleanly` and
`corpus_state_coverage_is_complete`, which now include the sample book.

Run: `cargo clippy --workspace --all-targets`
Expected: zero warnings.

- [ ] **Step 4: Render the book, if mdBook is installed**

Run: `mdbook build books/hello-playbook`
Expected: builds without error. If `mdbook` is not installed, say so and skip —
this is a nice-to-have, not part of the gate.

Open `books/hello-playbook/book/index.html` and confirm the obvious: no
`<!-- bower … -->` comment is visible, and chapter 4's failing-test block shows
only the new test rather than the whole module.

- [ ] **Step 5: Commit**

```bash
git add bower-testkit/tests/sample_book.rs && git commit -m "test(books): assert the hello-playbook final tree, goldens, and determinism"
```

---

## Definition of done

- `cargo test --workspace` passes.
- `cargo clippy --workspace --all-targets` is silent.
- `books/hello-playbook/` holds six chapters, a `SUMMARY.md`, a `book.toml`, a `bower.toml`, and eight template files.
- The book plans to exactly twenty steps with the tags listed in spec § 5.
- No `bower:` marker appears in any materialized tree.
- The two known gaps in spec § 9 are still true and still documented: the template directory is inert, and `expect` is recorded rather than verified.
