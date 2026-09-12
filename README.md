# Bower

[![CI](https://github.com/ImperialBower/bower/actions/workflows/ci.yml/badge.svg)](https://github.com/ImperialBower/bower/actions/workflows/ci.yml)
[![slow](https://github.com/ImperialBower/bower/actions/workflows/slow.yml/badge.svg)](https://github.com/ImperialBower/bower/actions/workflows/slow.yml)

**Book-driven repository generation and publishing.**

In euchre, the right bower is the card that controls the game. Here the book is
the right bower: annotated code blocks in the book's markdown are the single
source of truth. Bower turns them into deterministic, replayable git
repositories — one commit per teaching step, linked in both directions between
page and code.

This workspace is **Phases 1 to 4** of the [design spec](https://github.com/ImperialBower)
(`bower-spec.md`, Draft 0.2): the kernel, its testkit, replay, verification, the
`mdbook-bower` preprocessor, `status`, and `push`. Phase 5 — migration — is
under way: *Rust for Failures* lives in `books/rust4failures/`, one chapter
so far, and publishes to `abstecker/rust4failures`.

## Crates

| Crate | What it is |
|---|---|
| `bower-core` | The domain kernel. Parses `<!-- bower … -->` directives out of chapter markdown, groups blocks into steps, orders them (document order, bent by `after=`), folds each step into a complete `TreeState`, computes display spans and line-anchored ranges, binds `notebook="play"` cells to their steps, and reports every error in one located pass. Zero dependencies; no I/O or serialization in the public API. |
| `bower` | The CLI. Reads `bower.toml` and the book's `SUMMARY.md`, resolves the plan through the kernel, and replays it into a git repository with `gix` — one commit per step, annotated tags, commit trailers pointing back at the chapter, and a generated `STEPS.md`. Verifies each step's declared `expect` against a real compiler and reports drift between book, lock, and built repo. Two runs of an unchanged book produce identical SHAs. |
| `mdbook-bower` | The mdBook preprocessor, a second binary in the same crate behind the `preprocessor` feature. Strips directives, applies display markers, injects `#step-<id>` anchors, and adds a footer linking each block to its exact lines in the generated repository. A directive naming an unknown repo fails `mdbook build`. |
| `bower-testkit` | The controllability half, per the kernel-testkit pattern: a fixture corpus covering every op, mechanism, and error variant; proptest generators over arbitrary valid books; and a state coverage report over (op × expect × mechanism). |

## The invariants (proven in the test suites)

- **Determinism** — equal book in, byte-identical plan out.
- **Line-map exactness** — every displayed span's `LineRange` covers exactly
  that span's text in the materialized tree.
- **Errors, not surprises** — every broken fixture produces its named error,
  with chapter and line attached.
- **Full state coverage** — the corpus exercises every op, expectation, and
  mechanism, and the report proves it.
- **Replay determinism** — two `bower build` runs of an unchanged book produce
  identical SHAs, and a replay over a dirty directory matches one over an empty
  directory (`bower/tests/determinism.rs`).
- **Declared failures really fail** — `expect="compile_fail"` does not compile;
  `expect="test_fail"` compiles but fails its tests. An untrue expectation fails
  the build, naming chapter and line (`bower/tests/verification.rs`).
- **Drift is visible** — edit a chapter, delete a file, or drop a tag, and
  `bower status` names exactly what changed (`bower/tests/status.rs`).
- **The page links to the code, by line** — every rendered block's footer names
  its file and line range at a tag, and those are the lines the reader just saw
  (`bower/tests/preprocessor.rs`).
- **Every target shows the same thing** — html, epub, and PDF render identically
  except for how an elided span is shown (`bower/tests/publish.rs`).
- **The PDF is reproducible** — two runs of an unchanged book produce identical
  bytes (`pdf_is_byte_identical_across_runs`).
- **A stale site is visible** — edit one paragraph and `bower status` reports
  the rendered book out of date, even when plan, lock, and repo are unchanged
  (`bower/tests/site.rs`).

## Quick tour

````rust
use bower_core::prelude::*;

let book = BookSource::from_chapters(vec![Chapter::new(
    "ch01.md",
    r#"# Hello

<!-- bower repo="failures" file="src/lib.rs" -->
```rust
pub fn hello() {}
```
"#,
)]);
let catalog = RepoCatalog::from_names(&["failures"]);
let plan = plan(&book, &catalog)?;

let step = &plan.repo("failures").unwrap().steps[0];
assert_eq!(step.tag(), "step-001-ch01-hello");
assert_eq!(step.tree.text("src/lib.rs").unwrap(), "pub fn hello() {}\n");
````

## Developing

```
make ayce      # clean, fmt, build, test, lint, security-scan, docs
make help      # every target, self-documented
make slow      # the two #[ignore]d lanes: the verify sweep and a real mdbook build
make book      # render the sample book with the preprocessor
```

`make ayce` is the whole gate and the default target. `lint` also asserts the
kernel's purity (`cargo tree -p bower-core -e normal` must print one line), and
`build` also compiles the CLI with `--no-default-features` — both invariants run
on every sweep without changing `ayce`'s prerequisites. If that purity line ever
grows a second row, something crossed a boundary it should not have.

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
book claimed. It never touches git, so it works before `build` has ever run. The
scratch directory lives outside the book — under the system temp directory by
default, never the book's `target/` — because most Rust books are themselves
cargo workspaces, and cargo refuses to build a package written inside one. When
that happens `verify` says so by name instead of reporting every true claim as
false.

`status` answers the question between the other commands: what here is out of
date? It compares the book against `bower.lock` and against a previously built
repository, names the steps, tags, and files that drifted, and exits non-zero if
any did. An unplanned book and an unbuilt repo are reported as such and are
**not** drift, so it is safe in CI on a fresh checkout.

`push` publishes a built repository — **and its rendered site** — to the `github`
remote its `bower.toml` declares. A book that names a `site_branch` ships its
HTML there in the same command, so code and pages cannot drift apart. It
**reports and changes nothing** unless given `--execute`, and it refuses to
force-push over any repository that does not carry Bower's own `STEPS.md` marker
naming this book. **There is no override** — no flag, no environment variable,
no config key. To adopt an existing repository, push a `STEPS.md` to it by hand
first.

The whole sequence has a name:

```
make ship-hello           # build, verify, render, and report what a push would do
make ship-hello-execute   # the same, then actually push
```

Two targets rather than one flag, so `--execute` is written down in exactly one
place.

### Releases

A book that declares an edition ships its downloads with it:

```toml
[book]
version = "0.1.0"          # the edition; absent means no releases

[repos.hello-playbook]
assets = "published"       # book-relative; where the epub and PDF are rendered
```

`push` then hangs a GitHub release off `v0.1.0` and attaches every `.pdf` and
`.epub` directly inside that directory — not recursively, since the render's own
scratch folders live under the same roof. Re-shipping the same edition replaces
the files rather than failing, which is safe because both artifacts are
byte-reproducible.

Half-configured is an error, not a silence. `assets` without a `version`, or an
`assets` directory holding neither format, refuses the push and says which.

The release is decided locally and published last, after the repository and the
site are really there — a release pointing at an unfetchable tag is worse than
no release.

## Rendering the book

```
make book      # HTML, via mdBook
make epub      # epub, via pandoc
make pdf       # PDF, via pandoc + typst
```

All three go through `bower publish --target …`, which folds the book into one
*render plan* and hands it to a renderer. They share a single display-marker
engine and differ in exactly one rule: mdBook HTML keeps Rust's expandable
hidden lines, and epub — which has no toggle — collapses each elided span to
`// ⋯ 9 lines elided — full file: <url>`.

`html` needs `mdbook-bower` on `PATH`; the book's `book.toml` declares
`[preprocessor.bower]`, so without it the build **fails** rather than rendering a
book whose directives were never applied. `epub` needs `pandoc`; `pdf` needs
`pandoc` and `typst`. Every tool is checked, by name and with its install
command, before anything is written.

The PDF is **byte-identical across runs**: its timestamp is pinned from the same
`bower.toml` epoch that makes commit SHAs reproducible.
`books/hello-playbook/template.typ` sets its typography, and any font it names is
checked against `typst fonts` first, so no font is silently substituted.

### Covers

A book's cover is two files beside its `book.toml`, found by convention exactly
as `template.typ` is:

| File | What it is |
|---|---|
| `cover.svg` | The title band. Required — no `cover.svg`, no cover. |
| `cover.png` / `.jpg` | Artwork, stacked below the band. Optional; without it the band fills the page. |

`publish::compose_cover` stacks them into **one** 1600×2400 SVG, with the artwork
embedded as a `data:` URI so the result is self-contained. Both renderers get
those same bytes — pandoc as `--epub-cover-image`, Typst as a full-bleed first
page — so the epub and the PDF cannot show different covers. The composition is
a pure function, so it is tested with no renderer installed, and deterministic,
so the PDF stays byte-identical.

### Exercises

A step can be a place to stop reading and start typing. Put `exercise="Make
this compile"` on a step's directive, or give the exercise its own directive
with a fenced markdown body of ideas right after the step:

`````markdown
<!-- bower repo="rust4failures" exercise="Make this compile" -->
````markdown
Every `char` needs an answer, not just the card ones.

- Try a lookup table instead of a `match`.
````
`````

The rendered box names the fork link, the clone and checkout commands, and the
command to run — `check` after a `compile_fail`, `verify` otherwise, the same
ones `bower verify` runs. The next step is the answer: in HTML its code folds
behind "Show the answer" (or "Show one way" after a green step); epub and PDF
print it with a note. An exercise can sit on any step but the last, and a step
carries at most one. Design:
`docs/superpowers/specs/2026-09-06-exercises-design.md`.

### Recorded output

A step that fails on purpose can show what the compiler said, and prove it.
Put an `output` directive after the step, with an empty fence:

````markdown
<!-- bower repo="hello-playbook" output="check" -->
```text
```
````

`bower verify --record` fills the fence with what the repo's `check` (or
`verify`) command printed at that step, with paths, colour, timings, build
chatter and thread IDs taken out. From then on `bower verify` fails the day the
compiler says something else. A line that is exactly `[...]` stands for lines
left out, so a fence can quote only what matters; `--record` keeps that trim
while it still matches. The rendered page gets a caption with the command and a
link for each error code.

Which compiler says it matters, so a step runs on its tree's own
`rust-toolchain.toml`, or else on the repo's `toolchain` key in `bower.toml`.
Design: `docs/EPIC-11_Diagnostics.md`.

## License

MIT OR Apache-2.0.
