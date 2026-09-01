# CI

CI does not need its own idea of what "good" means. It has one job: check out
the code, install the toolchain, and run the same gate you run locally.

## One version, declared once more

<!-- bower repo="hello-playbook" step="tool-versions" file=".tool-versions" msg="chore: declare the toolchain for version managers" -->

```text
rust 1.95.0
```

This has to agree with `rust-toolchain.toml` from chapter 3. Two places that
name a version are two places that can disagree, so the rule in these books is
that every version-declaring file is updated together, always.

## The workflow

<!-- bower repo="hello-playbook" step="ci-workflow" file=".github/workflows/ci.yml" msg="ci: run the gate on every push and pull request" -->

```yaml
name: ci

on:
  push:
    branches: [main]
  pull_request:

jobs:
  gate:
    runs-on: ubuntu-latest
    steps:
      # bower:show
      - uses: actions/checkout@v7
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - uses: taiki-e/install-action@cargo-audit
      - uses: taiki-e/install-action@cargo-deny
      - run: make ayce
      # bower:show end
```

Two of those references name a version and one names a thing. `actions/checkout`
is pinned to a major, resolved when this chapter was written. But
`dtolnay/rust-toolchain@stable` and `taiki-e/install-action@cargo-audit` are not
versions at all — `stable` and `cargo-audit` name what you want, and the action
resolves it. Those are copied exactly and never "updated".

The last line is the whole argument. CI runs `make ayce` and nothing else. There
is no second list of checks to drift out of step with the first.

## Clearing the scratch

<!-- bower repo="hello-playbook" step="drop-scratch" file="src/scratch.rs" op="delete" msg="chore: remove the scratch module" -->

The broken module did its job in chapter 4 and has no reason to stay. This
directive has no code block after it — `op="delete"` names a file and removes
it, and the paragraph you are reading is what stops the next fence from being
mistaken for its contents.

Deleting the file is not enough on its own. `src/lib.rs` still declares the
module, and a declaration pointing at nothing does not compile, so the same
commit empties the `mods` region:

<!-- bower repo="hello-playbook" step="drop-scratch" file="src/lib.rs" op="region" region="mods" -->

```rust
```

That block really is empty. The region returns to the state chapter 4 created it
in, and the file goes back to having exactly one blank line where the module
declaration used to be.
