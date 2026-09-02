# Technical Debt

> Maintained by the `/backlog` skill. Items tagged 🤖 were proposed by automated
> review — review and edit them; they are suggestions, not facts.
>
> Created 1 September 2026 at `b3def07`. Last refreshed 2 September 2026, when
> the Makefile landed.
>
> There are **no `TODO`, `FIXME`, `HACK`, or `XXX` markers anywhere in this
> codebase**, so nothing here came from a code comment. Most items were written
> down deliberately in an EPIC corrigendum at the moment the shortcut was taken;
> the rest were found by `make ayce`.

## Tracked debt

- [ ] **`bower verify` does not run the book's own gate.**
  `books/hello-playbook/bower.toml` declares `check = "cargo check"` and
  `verify = "cargo test"`. Neither runs `cargo fmt --check` or clippy, so the
  generated repo can fail `make ayce` while every step verifies clean. This is
  not hypothetical: it happened, and was caught by eye rather than by the tool
  (EPIC-02 corrigendum items 4 and 5). The obvious fix — point `verify` at
  `make ayce` — does not work, because the `Makefile` does not exist until step
  3 and `make audit` needs `cargo-audit` and `cargo-deny` installed. Likely
  shape: a list of verify commands per step range, or a `verify_from = "<step>"`
  key.

- [ ] **Multi-repo books are untested.**
  `bower/src/main.rs` loops over `plan.repos` and gives each its own output
  directory, and `bower/src/replay.rs` takes a `&RepoPlan` precisely so repos
  replay independently. No book targets two repos, so this is untested rather
  than proven (EPIC-01, still-open list). A second tiny repo in
  `books/hello-playbook` would close it cheaply.

- [ ] **Verification is sequential.**
  Spec § 6 calls verification embarrassingly parallel. It is not parallel
  (`bower/src/verify.rs`). Twenty steps take 16 seconds thanks to one shared
  `CARGO_TARGET_DIR`, so parallelism would have been premature — but a real
  book with hundreds of steps will need it, and `std::thread::scope` would add
  no dependency.

- [ ] **No stderr snapshots for `compile_fail`.**
  `bower verify` asserts the check command failed and captures stderr for the
  report, but matches nothing against a stored pattern. Spec § 12 Q3 was closed
  as failure-only (EPIC-02, Phase 5c). A book that says "here is what the
  compiler tells you" cannot yet prove the message is still that.

- [ ] **File modes are inferred from a shebang.**
  The kernel does not model file modes — `FileBody` is text or bytes
  (`bower-core/src/tree.rs:15`) — so `bower/src/materialize.rs` marks a file
  executable when it starts `#!`. That covers `bin/security-scan` and nothing
  else. A book needing an executable without a shebang has no way to say so;
  the honest fix is a `mode=` directive key in the kernel, not a longer
  heuristic (EPIC-01 corrigendum item 2).

- [ ] **The `template/` directory has no test.**
  `books/hello-playbook/template/` is applied as step 0 by
  `bower/src/replay.rs`, and `bower verify` overlays it before checking a step
  — but nothing asserts its contents. A licence file deleted by accident would
  be noticed by nobody.

- [ ] **The epub elision rendering is designed but unbuilt.**
  Spec § 3.4 describes elided spans collapsing to a linked comment for
  pandoc/epub. `bower/src/render.rs` implements the mdBook HTML form and a
  comment form for non-Rust fences, but there is no epub build to render into.

- [ ] **One copyleft crate in the dependency tree.**
  `uluru` (MPL-2.0) arrives through `gix-pack` → `gix`, and is the only
  non-permissive licence in a workspace that is otherwise MIT OR Apache-2.0.
  MPL-2.0 is weak, file-level copyleft: using `uluru` unmodified as a dependency
  does not affect this workspace's own terms; modifying its files would. It is
  allowed explicitly in `deny.toml` with that reasoning written beside it, so it
  is a decision rather than an accident. Revisit if `gix` is ever swapped for
  `git2` (EPIC-01 corrigendum) or if a downstream consumer forbids copyleft
  outright.

- [ ] **`GitHubForge` is untested.**
  `bower/src/forge.rs`'s real implementation — every `gh api` call and both
  `git push` invocations — is executed by no test. This is the acknowledged last
  inch of EPIC-05: the *decisions* are tested against `FakeForge`, and the pure
  parts that could be extracted (`remote_url`, `state_from`, `is_not_found`)
  were. What remains untested is the shelling-out itself, and it cannot be
  covered without a network and a repository we are willing to destroy. Related:
  **no `bower push --execute` has ever been run**, here or anywhere, because no
  book in this repository declares a `github` remote.

- [ ] **Symlinks in a `template/` directory are followed.**
  A book cannot create a symlink — the kernel has no op for it — but
  `materialize::read_dir_recursive` follows one it finds while reading a
  configured `template/`, so a symlinked file would be copied by content into
  every generated repo. Related to `docs/DEFECT_Path_Traversal.md`, and out of
  scope for that fix.

- [ ] **Path case is not normalized.**
  On a case-insensitive filesystem, `SRC/lib.rs` and `src/lib.rs` are one file
  but two `TreeState` keys. Not an escape; a way for two steps to collide
  without anyone noticing.

## 🤖 Automated review findings

<!-- Promote good ones up to "Tracked debt", delete the rest. -->

_None yet._ The deep automated review pass that normally seeds this section on
first creation was **not run** — it needs a code-review subagent, and this
session is configured not to spawn one unasked. Say the word and I will run it.
