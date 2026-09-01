# Lints and format

Formatting and linting are not taste. They are the cheapest possible tests: they
run in a second and they catch a class of mistake before a human ever reads the
diff. So they belong in the gate, and the gate belongs in one command.

## Pin the format

<!-- bower repo="hello-playbook" step="rustfmt" file="rustfmt.toml" msg="style: pin the formatter" -->

```toml
# Only max_width is set. Everything else is rustfmt's default on purpose —
# a config file full of overrides is a config file nobody can read.
max_width = 100
```

## Pin the toolchain

<!-- bower repo="hello-playbook" step="toolchain" file="rust-toolchain.toml" msg="build: pin the toolchain" -->

```toml
[toolchain]
channel = "1.95.0"
components = ["clippy", "rustfmt"]
```

Anyone who clones this repository gets that exact compiler, with those exact
components, without being told to install anything. The version appears again in
`.tool-versions` in chapter 6, and the two must always agree.

## Fill the lints region

<!-- bower repo="hello-playbook" step="lints" file="Cargo.toml" op="region" region="lints" msg="style: forbid unsafe, warn on pedantic" -->

```toml
[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
pedantic = { level = "warn", priority = -1 }
unwrap_used = "warn"
expect_used = "warn"
```

This is the hole chapter 1 left open. The `priority = -1` is not decoration:
without it, `pedantic` as a lint *group* would override the individual lints
listed after it.

## Put them in the gate

<!-- bower repo="hello-playbook" step="gate-fmt-clippy" file="Makefile" op="region" region="gate" msg="build: fmt and clippy join the gate" -->

```makefile
GATE := build fmt lint

build:
	cargo build --workspace

fmt:
	cargo fmt --all --check

lint:
	cargo clippy --workspace --all-targets -- -D warnings

.PHONY: build fmt lint
```

`--check` rather than a plain `cargo fmt`, and `-D warnings` rather than a plain
`clippy`: a gate that quietly fixes things is not a gate. It must fail.
