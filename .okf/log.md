# Update Log

## 2026-09-09

* **Update**: `bower publish --target pdf` now defines the Typst helpers pandoc's writer assumes its own template supplies (`PANDOC_TYPST_HELPERS`). Older pandoc emits `#blockquote[…]`, so every book with an exercise failed to build a PDF on a distro pandoc. `slow.yml` pins pandoc 3.11 alongside. Recorded in [bower publish](/commands/publish.md) and [CI lanes](/operations/ci-lanes.md).
* **Update**: `slow.yml` now installs typst from its own GitHub release rather than through `taiki-e/install-action`, which has no typst entry and whose cargo-binstall fallback fails because the crates.io `typst` crate is the compiler library and ships no binary. Recorded in [CI lanes](/operations/ci-lanes.md).

* **Creation**: Scaffolded the bundle with `okf_init.py`.
* **Creation**: Wrote the [domain model](/model/index.md) — directives, steps, ordering, plans, tree states, assembly, display markers, expectations, exercises, play cells, and errors — distilled from `bower-spec.md` §2–§6 and read against `bower-core/src/`.
* **Creation**: Wrote the [architecture](/architecture/index.md) set — the pipeline, the domain-kernel pattern, the four crates, and the [invariants](/architecture/invariants.md) table naming the suite that proves each property.
* **Creation**: Wrote the [command surface](/commands/index.md), [configuration](/config/index.md), [books](/books/index.md), and [operations](/operations/index.md).
* **Creation**: Recorded six settled [decisions](/decisions/index.md) with their dates, and the [roadmap](/roadmap/index.md) for the four spec sections that are design rather than code.
* **Note**: Recorded two documentation drifts found while writing. `README.md` documents the elision leader as `⋯`; the code emits ASCII `...` and pins it with a test. `BACKLOG.md` and `README.md` describe *Rust for Failures* as further along than its `bower.lock` (2 steps) shows.
