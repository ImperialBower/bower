# Bower

**Book-driven repository generation and publishing.**

In euchre, the right bower is the card that controls the game. Here, the book
is the right bower: annotated code blocks in the book's markdown are the single
source of truth, and Bower turns them into deterministic, replayable git
repositories — one commit per teaching step, with links in both directions
between the page and the code.

This workspace is **Phases 1 to 3** of the [design spec](https://github.com/ImperialBower)
(`bower-spec.md`, Draft 0.2): the pure kernel, its testkit, the replay layer that
turns a plan into real commits, and the verifier that checks every step's
declared `expect` against a real compiler. Publishing surfaces — the mdBook
preprocessor, `push`, and `status` — are Phase 4 and are not built yet.

## Crates

| Crate | What it is |
|---|---|
| `bower-core` | The domain kernel. Parses `<!-- bower … -->` directives out of chapter markdown, groups blocks into steps, orders them (document order, bent by `after=`), folds every step into a complete `TreeState`, computes display spans and line-anchored ranges, binds `notebook="play"` cells to their steps for the Jupyter target, and reports every error it can find in one located pass. Zero dependencies; nothing in the public API does I/O or serialization. |
| `bower` | The CLI. Reads `bower.toml` and the book's `SUMMARY.md`, resolves the plan through the kernel, replays it into a git repository with `gix` — one commit per step, annotated tags, commit trailers pointing back at the chapter, and a generated `STEPS.md` — and verifies every step's declared `expect` against a real compiler. Two runs of an unchanged book produce identical SHAs. |
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
cargo test --workspace
cargo clippy --workspace --all-targets   # pedantic, zero warnings
cargo tree -p bower-core -e normal       # must print the crate and nothing else
```

The last command is the kernel's purity gate. `bower-core` has no dependencies
and does no I/O; if that line ever grows a second row, something crossed a
boundary it should not have.

## Building the sample book's repo

```
cargo run -p bower -- --book books/hello-playbook plan
cargo run -p bower -- --book books/hello-playbook build -o /tmp/hello-playbook
cargo run -p bower -- --book books/hello-playbook verify
```

`verify` writes each step's tree to a scratch directory and runs the repo's
`check` and `verify` commands against it, comparing what happens to what the
book claimed. It never touches git, so it works before `build` has ever run.

## License

MIT OR Apache-2.0.
