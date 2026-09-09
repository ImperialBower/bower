# Decisions

Questions the project has settled, with the date and the reasoning. These are
the things a reader cannot recover from the code — the code shows what was
chosen, not what was rejected or why.

# Settled

* [gix, not git2, for replay](gix-over-git2.md) - 2026-09-01. Pure Rust, and direct control of commit time.
* [One workspace for every book — a reversal](one-workspace-for-books.md) - 2026-09-05. Decided the other way first, then reversed after building something.
* [compile_fail asserts failure, not a diagnostic snapshot](failure-only-diagnostics.md) - 2026-09-01. Explicitly reversible; no book has needed more.
* [The block library is core, not a Scrivener fallback](block-library-is-core.md) - 2026-09-04. `include=` is a recommended way to write a block.
* [Generated repositories carry their own CI](generated-repos-carry-ci.md) - 2026-09-04. A public badge that every step passes.
* [The crates.io names are free — and unreserved](crate-names.md) - 2026-09-02. Checked, not claimed.

# Still open

Four of the spec's ten open questions remain unanswered, all in territory that
is not yet built: wheel distribution for notebooks, `expect` for play cells,
JupyterLite/pyodide, and whether `devenv.nix` is hand-authored or derived. See
[the notebook target](/roadmap/notebook-target.md) and
[reader environments](/roadmap/reader-environments.md).
