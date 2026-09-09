---
type: Roadmap
title: The notebook target — playable chapters
description: One Jupyter notebook per chapter, where play cells become live Python against a PyO3 wrapper pinned to that chapter's tag. Kernel half built; renderer not.
tags: [roadmap, notebooks, jupyter, pyo3]
timestamp: '2026-09-09T00:00:00Z'
---

# The idea

`bower publish --target ipynb` emits **one notebook per chapter**: prose becomes
markdown cells; each displayed span becomes a read-only view of the Rust with
its line-anchored links; and [play cells](/model/play-cell.md) become live
Python cells a reader can run and mutate.

# Status

**The kernel half is built.** Play cells are parsed, bound to their step, and
error-checked today — `PlayCellUnbound`, `PlayCellUnknownStep`, and
`PlayCellConflictingKeys` all exist, and `Mechanism::PlayCell` is one of the ten
states the [coverage report](/architecture/crate-bower-testkit.md) must cover.

**The renderer is not.** `Target::from_str` names `ipynb` in its error text as
spec §15's target. Nothing emits `.ipynb`.

# The wrapper is book content, not magic

Bower deliberately does **not** generate the PyO3 binding code:

> auto-generated bindings would be exactly the kind of unauditable machinery
> both books argue against.

Instead, the repo template ships a `bindings/` skeleton — pyo3 dependency,
maturin metadata, an empty `#[pymodule]` — and chapters grow it with ordinary
annotated blocks. The bindings evolve step by step and are visible at every tag.

This is framed as teaching material rather than overhead: wrapping a kernel for
a scripting surface *is* the Controllability thesis made concrete, and the
wrapper's `Result`-to-exception mapping is a *Rust for Failures* chapter waiting
to happen.

# Pinning

Each chapter's notebook must import exactly the code state that chapter ends at.
The generated first cell installs the wheel for the chapter-end tag — preferably
a CI-built wheel published against `ch03-end`, falling back to
`pip install "git+https://…@ch03-end#subdirectory=bindings"` with maturin
building locally. Regenerating the book regenerates the pin, so the notebook can
never drift ahead of or behind its chapter.

`bower verify --target ipynb` would execute every notebook against its pinned
wheel in CI, headless, so a play cell that errors fails the build.

# The open questions

Three of the spec's four remaining open questions live here:

- **Wheel distribution** — CI-built wheels attached to chapter-end tags, a
  per-book PyPI package, or build-on-install via maturin?
- **`expect` for play cells** — a Python cell that *should* raise, extending the
  [expectation](/model/expectation.md) matrix across the language boundary. It
  probably wants the same vocabulary.
- **JupyterLite / pyodide** — native PyO3 wheels do not load in the browser
  today, but maturin's experimental pyodide support is worth tracking. A
  zero-install playable book would be a real distribution edge.

# The road not taken

The evcxr Rust kernel — notebooks in raw Rust, no wrapper — was considered and
set aside as the default: it puts a full Rust toolchain between the reader and
their first keystroke. It remains viable as a per-book execution mode later.

# Citations

[1] [`bower-spec.md` §15](/references/bower-spec.md), `bower-core/src/plan.rs`
