# Update Log

## 2026-09-14

* **Update**: [pull request](/model/pull-request.md) — EPIC-09 slice 2 built: the push schedule and stepping stones (`bower/src/schedule.rs`), the `Forge` methods that replace `Forge::push` (`remote_heads`, `pull_requests`, `push_ref`, `push_tags`, `open_pull_request`, `edit_pull_request`, `set_default_branch`), and the PR body's footer and `bower-pr:` digest (Decision 23). The live run (exit criterion 9) is still open.
* **Update**: [pull request](/model/pull-request.md) — the final review's fixes: PRs from forks are never Bower's (filtered out by `isCrossRepository`), and the branches and main's first move go out in one atomic push (`Forge::push_refs`), because an open PR survives a rebuild only when head and base move together (the `pr-remote` spike's Q2b).

## 2026-09-13

* **Creation**: [Branch](/model/branch.md) and [pull request](/model/pull-request.md) — EPIC-09 slice 1's `branch=`, `from=`, `merge=`, and `pr=` keys, the per-line fold, the merge and conflict rules, branch-name collisions, and the `PULLS.md`/lock homes for a declared PR.
* **Update**: [the bower directive](/model/directive.md) gains the four keys; [errors](/model/errors.md) counts 49 variants and gains the twelve EPIC-09 variants' row; [the domain model index](/model/index.md) lists both new pages and the new error count.

## 2026-09-12

* **Creation**: [Captured output](/model/captured-output.md) — EPIC-11's `output="check"|"verify"` blocks, their normalization, `[...]` matching, and five errors.
* **Update**: [bower verify](/commands/verify.md) gains `--record`, output rows, and the toolchain rule; [bower.toml](/config/bower-toml.md) gains `toolchain` and `links.error_code`; [errors](/model/errors.md) counts 37 variants.

## 2026-09-09

* **Update**: `bower publish --target pdf` now defines the Typst helpers pandoc's writer assumes its own template supplies (`PANDOC_TYPST_HELPERS`). Older pandoc emits `#blockquote[…]`, so every book with an exercise failed to build a PDF on a distro pandoc. `slow.yml` pins pandoc 3.11 alongside. Recorded in [bower publish](/commands/publish.md) and [CI lanes](/operations/ci-lanes.md).
* **Update**: `slow.yml` now installs typst from its own GitHub release rather than through `taiki-e/install-action`, which has no typst entry and whose cargo-binstall fallback fails because the crates.io `typst` crate is the compiler library and ships no binary. Recorded in [CI lanes](/operations/ci-lanes.md).

* **Creation**: Scaffolded the bundle with `okf_init.py`.
* **Creation**: Wrote the [domain model](/model/index.md) — directives, steps, ordering, plans, tree states, assembly, display markers, expectations, exercises, play cells, and errors — distilled from `bower-spec.md` §2–§6 and read against `bower-core/src/`.
* **Creation**: Wrote the [architecture](/architecture/index.md) set — the pipeline, the domain-kernel pattern, the four crates, and the [invariants](/architecture/invariants.md) table naming the suite that proves each property.
* **Creation**: Wrote the [command surface](/commands/index.md), [configuration](/config/index.md), [books](/books/index.md), and [operations](/operations/index.md).
* **Creation**: Recorded six settled [decisions](/decisions/index.md) with their dates, and the [roadmap](/roadmap/index.md) for the four spec sections that are design rather than code.
* **Note**: Recorded two documentation drifts found while writing. `README.md` documents the elision leader as `⋯`; the code emits ASCII `...` and pins it with a test. `BACKLOG.md` and `README.md` describe *Rust for Failures* as further along than its `bower.lock` (2 steps) shows.
