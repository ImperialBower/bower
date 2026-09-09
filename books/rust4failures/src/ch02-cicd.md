# Continuous Integration

## GitHub Actions

This is our traffic cop. There are many different ways to setup continuous integration, but
[GitHub Actions](https://docs.github.com/en/actions) is one of my favorites. I'd recommend
that you check out the entire file for our CI guardrails. For now, I've highlighted two sections:
`on` and the `test` job. On tells us that we are checking everything every time there's a push
or a pull request. After a while, that may be too much, and we can dial it down, but for now
it's good.

<!-- bower repo="rust4failures" step="kata" file=".github/workflows/CI.yaml" msg="init" -->

```yaml
name: CI

# bower:show
on:
  push:
  pull_request:
# bower:show end

permissions:
  contents: read

env:
  RUSTFLAGS: -Dwarnings

# bower:show
jobs:
  test:
    name: Rust ${{matrix.rust}}
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        rust: [stable, 1.98.1]
    timeout-minutes: 45
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{matrix.rust}}
      - run: cargo test --all
        # bower:show end

  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    if: github.event_name != 'pull_request'
    timeout-minutes: 45
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: stable
          components: clippy
      - run: cargo clippy -- -Dclippy::all -Dclippy::pedantic

  fmt:
    name: Fmt
    runs-on: ubuntu-latest
    if: github.event_name != 'pull_request'
    timeout-minutes: 45
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: stable
          components: rustfmt
      - run: cargo fmt --all -- --check

  doc:
    name: Doc
    runs-on: ubuntu-latest
    timeout-minutes: 45
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: stable
          components: rust-docs
      - run: cargo doc --no-deps --document-private-items
        env:
          RUSTDOCFLAGS: "-D warnings"
```