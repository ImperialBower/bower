# Bower

**Book-driven repository generation and publishing.**

In euchre, the right bower is the card that controls the game. Here, the book
is the right bower: annotated code blocks in the book's markdown are the single
source of truth, and Bower turns them into deterministic, replayable git
repositories — one commit per teaching step, with links in both directions
between the page and the code.

This workspace is **Phase 1** of the [design spec](https://github.com/ImperialBower)
(`bower-spec.md`, Draft 0.2): the pure kernel and its testkit. No I/O, no git
yet — that is Phase 2's replay layer, which consumes what this produces.

## Crates

| Crate | What it is |
|---|---|
| `bower-core` | The domain kernel. Parses `<!-- bower … -->` directives out of chapter markdown, groups blocks into steps, orders them (document order, bent by `after=`), folds every step into a complete `TreeState`, computes display spans and line-anchored ranges, binds `notebook="play"` cells to their steps for the Jupyter target, and reports every error it can find in one located pass. Zero dependencies; nothing in the public API does I/O or serialization. |
| `bower-testkit` | The controllability half, per the kernel-testkit pattern: a fixture corpus covering every op, mechanism, and error variant (the *data textures* of book sources); proptest generators over arbitrary valid books; and a *state coverage* report over (op × expect × mechanism). |

## The invariants (proven in `bower-testkit/tests/`)

- **Determinism** — equal book in, byte-identical plan out.
- **Line-map exactness** — every displayed span's `LineRange` covers exactly
  that span's text in the materialized tree; source links cannot lie.
- **Errors, not surprises** — every broken fixture produces its named error,
  with chapter and line attached.
- **Full state coverage** — the corpus exercises every op, every expectation,
  and every mechanism, and the report proves it.

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
```

## License

MIT OR Apache-2.0.
