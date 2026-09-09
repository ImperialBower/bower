---
type: Roadmap
title: Reader environments — devenv
description: The book promises "check out any step and watch it fail on cue"; that promise dies at toolchain friction. devenv pins what the code runs on, as the blessed path but not the only one.
tags: [roadmap, devenv, nix, reproducibility]
timestamp: '2026-09-09T00:00:00Z'
---

# The problem

The books promise: *check out any step, run it, and watch it fail on cue.*

That promise dies at toolchain friction. A reader who clones
`rust4failures@step-012-rank-enum` needs the **exact** rustc the book was
verified with — a [`compile_fail`](/model/expectation.md) step is a claim about
a specific compiler — plus, for notebook chapters, Python, maturin and Jupyter
in compatible versions. "Install these nine things first" is where follow-along
readers quit.

# The fit

devenv extends Bower's pinning discipline one layer down:

- The book pins the **code** — steps, tags, editions.
- devenv pins what the code **runs on** — `devenv.nix` plus `devenv.lock`,
  checked in and versioned.

An edition then freezes the toolchain too. Readers of the 1.0 epub get the 1.0
rustc forever, which is what keeps a `compile_fail` step failing *for the stated
reason* years later — the reader-side answer to the diagnostic-drift problem
that [failure-only verification](/decisions/failure-only-diagnostics.md) leaves
open.

# Mechanics

- **Template, step 0.** `devenv.nix`, `devenv.lock` and `.envrc` ship in the
  repo template, so they exist at every tag from the first commit. The Rust
  version derives from `rust-toolchain.toml`; repos with a `bindings/` crate add
  Python, maturin and Jupyter.
- **Environment changes are steps.** Bumping `devenv.lock` or adding a tool is
  an ordinary annotated block — narratable in the book ("we now need maturin;
  here's why"), verified like any other step, visible in the diff.
- **One environment, three audiences.** CI's `bower verify` runs inside the same
  devenv the reader gets, collapsing "works in CI" and "works for the reader"
  into one fact. The workspace gets its own devenv too, so contributor
  onboarding is one command.
- **`devenv up` as the playground** — launching Jupyter with the chapter wheel
  on the path, or a `cargo watch` loop for the follow-along reader.

# Posture: blessed, not required

Nix is a real ask — smooth on macOS and Linux, WSL2 territory on Windows, and a
conceptual speed bump for a reader who just wants to see code fail. So devenv is
the *blessed* path, not a gate:

- the generated README keeps a plain `rustup` fallback, honest about version
  requirements;
- a devcontainer/Codespaces route, exported from the same environment
  definition, covers the browser-only reader.

The rule mirrors [the editor invariant](/roadmap/authoring-bridges.md): the
repo's contract is with its committed files; devenv is the most faithful way in,
never the only one.

# Status

**Not built.** The remaining open question is whether `devenv.nix` is
hand-authored per repo template or derived by Bower from
[`bower.toml`](/config/bower-toml.md) plus `rust-toolchain.toml` — and whether
editions pin nixpkgs per-edition or share one channel per book.

# Citations

[1] [`bower-spec.md` §16, §12 Q10](/references/bower-spec.md)
