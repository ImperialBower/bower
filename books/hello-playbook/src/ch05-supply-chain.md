# Supply chain

Your code is a small fraction of what you ship. The rest arrives from a
registry, written by people you will never meet, and it changes underneath you.
Two tools watch it, and one script gives them a single name.

## Rules for dependencies

<!-- bower repo="hello-playbook" step="deny-toml" file="deny.toml" msg="chore: cargo-deny configuration" -->

```toml
[advisories]
version = 2

[licenses]
version = 2
# Exactly the licences this project actually needs. A longer list is not
# safer — it is a longer list of things you have stopped checking.
allow = ["Apache-2.0", "GPL-3.0-or-later", "MIT"]

[bans]
multiple-versions = "warn"
```

## One name for "scan"

<!-- bower repo="hello-playbook" step="security-scan" file="bin/security-scan" msg="chore: one definition of the security scan" -->

```bash
#!/usr/bin/env bash
# One definition of "scan". The Makefile calls this; so does CI. When the
# scan changes, it changes in one place.
set -euo pipefail

cargo audit
cargo deny check advisories bans licenses sources
```

`set -euo pipefail` is not boilerplate. Without it a failing `cargo audit` in
the middle of the script would be shrugged off and the gate would pass while the
advisory stood.

## Into the gate

<!-- bower repo="hello-playbook" step="gate-test-audit" file="Makefile" op="region" region="gate" msg="build: tests and the security scan join the gate" -->

```makefile
GATE := build fmt lint test audit

build:
	cargo build --workspace

fmt:
	cargo fmt --all --check

lint:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

audit:
	./bin/security-scan

.PHONY: build fmt lint test audit
```

The tests written in chapter 4 join the gate here, alongside the scan. `ayce`
itself has not been edited since chapter 2 — it never needs to be, because its
prerequisite list lives inside the region.
