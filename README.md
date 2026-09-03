# Bower

**Book-driven repository generation and publishing.**

In euchre, the right bower is the card that controls the game. Here, the book
is the right bower: annotated code blocks in the book's markdown are the single
source of truth, and Bower turns them into deterministic, replayable git
repositories — one commit per teaching step, with links in both directions
between the page and the code.

This workspace is **Phases 1 to 4** of the [design spec](https://github.com/ImperialBower)
(`bower-spec.md`, Draft 0.2): the pure kernel, its testkit, the replay layer that
turns a plan into real commits, the verifier that checks every step's declared
`expect` against a real compiler, the `mdbook-bower` preprocessor, the `status`
drift report, and `push`. Migration — standing up the real *Rust for Failers*
book — is Phase 5, and is where all of this first meets a book that was not
written to flatter it.

## Crates

| Crate | What it is |
|---|---|
| `bower-core` | The domain kernel. Parses `<!-- bower … -->` directives out of chapter markdown, groups blocks into steps, orders them (document order, bent by `after=`), folds every step into a complete `TreeState`, computes display spans and line-anchored ranges, binds `notebook="play"` cells to their steps for the Jupyter target, and reports every error it can find in one located pass. Zero dependencies; nothing in the public API does I/O or serialization. |
| `bower` | The CLI. Reads `bower.toml` and the book's `SUMMARY.md`, resolves the plan through the kernel, replays it into a git repository with `gix` — one commit per step, annotated tags, commit trailers pointing back at the chapter, and a generated `STEPS.md` — verifies every step's declared `expect` against a real compiler, and reports drift between the book, the lock, and a built repo. Two runs of an unchanged book produce identical SHAs. |
| `mdbook-bower` | The mdBook preprocessor, a second binary in the same crate behind the `preprocessor` feature. Strips directives, applies display markers so a block prints only what it marks, injects `#step-<id>` anchors, and adds a footer linking each block to its exact lines in the generated repository. A directive naming an unknown repo fails `mdbook build`. |
| `bower-testkit` | The controllability half, per the kernel-testkit pattern: a fixture corpus covering every op, mechanism, and error variant (the *data textures* of book sources); proptest generators over arbitrary valid books; and a *state coverage* report over (op × expect × mechanism). |

## The invariants (proven in the test suites)

- **Determinism** — equal book in, byte-identical plan out.
- **Line-map exactness** — every displayed span's `LineRange` covers exactly
  that span's text in the materialized tree; source links cannot lie.
- **Errors, not surprises** — every broken fixture produces its named error,
  with chapter and line attached.
- **Full state coverage** — the corpus exercises every op, every expectation,
  and every mechanism, and the report proves it.
- **Replay determinism** — two `bower build` runs of an unchanged book produce
  identical commit SHAs, and a replay over a dirty directory produces the same
  SHAs as one over an empty directory (`bower/tests/determinism.rs`).
- **Declared failures really fail** — a step marked `expect="compile_fail"` does
  not compile, and one marked `expect="test_fail"` compiles but fails its tests.
  Change an expectation to something untrue and the build says so, naming the
  chapter and line (`bower/tests/verification.rs`).
- **Drift is visible** — a freshly built book reports in sync; edit a chapter,
  delete a file, or drop a tag and `bower status` names exactly what changed
  (`bower/tests/status.rs`).
- **The page links to the code, by line** — every rendered block carries a
  footer naming its file and line range at a tag, and those lines are the lines
  the reader just saw (`bower/tests/preprocessor.rs`).
- **Every target shows the same thing** — html, epub, and PDF render identically
  except for how an elided span is shown, which is the one thing that must
  differ (`bower/tests/publish.rs`).
- **The PDF is reproducible** — two runs of an unchanged book produce identical
  bytes (`pdf_is_byte_identical_across_runs`).

## Quick tour

```rust
use bower_core::prelude::*;

let book = BookSource::from_chapters(vec![Chapter::new(
    "ch01.md",
    r#"# Hello

<!-- bower repo="failers" file="src/lib.rs" -->
```rust
pub fn hello() {}
```
"#,
)]);
let catalog = RepoCatalog::from_names(&["failers"]);
let plan = plan(&book, &catalog)?;

let step = &plan.repo("failers").unwrap().steps[0];
assert_eq!(step.tag(), "step-001-ch01-hello");
assert_eq!(step.tree.text("src/lib.rs").unwrap(), "pub fn hello() {}\n");
```

## Developing

```
make ayce      # clean, fmt, build, test, lint, security-scan, docs
make help      # every target, self-documented
make slow      # the two #[ignore]d lanes: the verify sweep and a real mdbook build
make book      # render the sample book with the preprocessor
```

`make ayce` is the whole gate and the default target. `lint` also asserts the
kernel's purity (`cargo tree -p bower-core -e normal` must print one line), and
`build` also compiles the CLI with `--no-default-features`, so both invariants
run on every sweep without changing `ayce`'s prerequisite list.

The last command is the kernel's purity gate. `bower-core` has no dependencies
and does no I/O; if that line ever grows a second row, something crossed a
boundary it should not have.

## Building the sample book's repo

```
cargo run -p bower -- --book books/hello-playbook plan
cargo run -p bower -- --book books/hello-playbook build -o /tmp/hello-playbook
cargo run -p bower -- --book books/hello-playbook verify
cargo run -p bower -- --book books/hello-playbook status -o /tmp/hello-playbook
cargo run -p bower -- --book books/hello-playbook push   -o /tmp/hello-playbook
cargo run -p bower -- --book books/hello-playbook publish --target epub -o published
```

`verify` writes each step's tree to a scratch directory and runs the repo's
`check` and `verify` commands against it, comparing what happens to what the
book claimed. It never touches git, so it works before `build` has ever run.

`push` publishes a built repository to the `github` remote its `bower.toml`
declares. It **reports and changes nothing** unless given `--execute`, and it
refuses to force-push over any repository that does not carry Bower's own
`STEPS.md` marker naming this book. **There is no override** — no flag, no
environment variable, no config key. To adopt an existing repository, push a
`STEPS.md` to it by hand first. Force-pushing is a generated repo's normal life,
and the guard is what keeps that from being anyone else's problem.

`status` answers the question between the other commands: what here is out of
date? It compares the book against `bower.lock` and against a previously built
repository, names the steps, tags, and files that drifted, and exits non-zero
if any did. A book nobody has planned and a repo nobody has built are reported
as such and are **not** drift, so it is safe in CI on a fresh checkout.

## Rendering the book

```
make book      # HTML, via mdBook
make epub      # epub, via pandoc
make pdf       # PDF, via pandoc + typst
```

Both go through `bower publish --target …`, which folds the book into one
*render plan* and hands it to a renderer. The two targets share a single
display-marker engine and differ in exactly one rule: mdBook HTML keeps Rust's
expandable hidden lines, and an epub — which has no toggle anywhere — collapses
each elided span to `// ⋯ 9 lines elided — full file: <url>`.

Three targets, one engine. `html` needs `mdbook-bower` on `PATH` — the book's
`book.toml` declares `[preprocessor.bower]`, so without it the build **fails**
rather than quietly rendering a book whose directives were never applied.
`epub` needs `pandoc`; `pdf` needs `pandoc` and `typst`. Every tool is checked,
by name and with its install command, before anything is written.

The PDF is **byte-identical across runs**, because its timestamp is pinned from
the same `bower.toml` epoch that already makes commit SHAs reproducible.
`books/hello-playbook/template.typ` sets its typography, and any font it names
is checked against `typst fonts` first — a silently substituted font is how a
missing glyph reaches a reader.

## License

MIT OR Apache-2.0.
