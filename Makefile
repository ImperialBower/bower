.PHONY: default help clean fmt build test lint security-scan docs ayce purity minimal slow book epub pdf ship-hello ship-hello-execute
default: ayce

help:  ## self-documenting: every target has a '## comment' printed here
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  make %-16s %s\n", $$1, $$2}'

clean: ## remove build artifacts
	cargo clean
	rm -rf books/*/book
	rm -rf books/*/published

fmt: ## format all sources
	cargo fmt --all

build: ## compile/build
	cargo build --workspace --all-features
	@$(MAKE) --no-print-directory minimal

test: ## run all tests
	cargo test --workspace --all-features

lint: ## static analysis at the pedantic end
	cargo clippy --workspace --all-targets --all-features -- -D warnings
	@$(MAKE) --no-print-directory purity

security-scan: ## dependency vulnerability scan
	./bin/security-scan

docs: ## build docs, fail on warnings
	RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps

ayce: clean fmt build test lint security-scan docs ## all-you-can-eat: full pre-push sweep

# ---------------------------------------------------------------------------
# Extra targets. The seven above are the contract and never change; these are
# this project's own invariants, folded into `build` and `lint` so that a plain
# `make ayce` runs them without altering `ayce`'s prerequisite list.
# ---------------------------------------------------------------------------

purity: ## assert bower-core still has zero dependencies
	@n=$$(cargo tree -p bower-core -e normal | wc -l | tr -d ' '); \
	if [ "$$n" != "1" ]; then \
		echo "kernel purity broken: bower-core has dependencies"; \
		cargo tree -p bower-core -e normal; \
		exit 1; \
	fi; \
	echo "purity: bower-core has no dependencies"

minimal: ## assert the CLI builds without the preprocessor feature
	cargo build -p bower --no-default-features

slow: ## the #[ignore]d lanes: the 20-step verify sweep and a real mdbook build
	cargo test -p bower --test verification -- --ignored
	cargo build -p bower --all-features
	PATH="$(CURDIR)/target/debug:$$PATH" cargo test -p bower --test preprocessor -- --ignored
	cargo test -p bower --test publish -- --ignored

HELLO     := books/hello-playbook
HELLO_OUT := target/hello-playbook

book: ## render the sample book as HTML
	cargo build -p bower --all-features
	PATH="$(CURDIR)/target/debug:$$PATH" cargo run -q -p bower -- \
		--book books/hello-playbook publish --target html -o books/hello-playbook/book

epub: ## render the sample book as an epub (needs pandoc)
	cargo run -q -p bower -- --book $(HELLO) publish --target epub -o $(HELLO)/published

pdf: ## render the sample book as a PDF (needs pandoc and typst)
	cargo run -q -p bower -- --book $(HELLO) publish --target pdf -o $(HELLO)/published

# ---------------------------------------------------------------------------
# Shipping the sample book.
#
# Split in two on purpose. `ship-hello` does every step that can be undone and
# stops at a dry run; `ship-hello-execute` is the only place `--execute` is
# written down. A force-push should have to be asked for by name.
# ---------------------------------------------------------------------------

# `epub` and `pdf` are prerequisites of the *dry run*, not just the real push:
# a preview that cannot see the release it would publish is not a preview. They
# need pandoc and typst, which is the price of shipping downloads.
ship-hello: book epub pdf ## build, verify and render the sample book, then report what a push would do
	cargo run -q -p bower -- --book $(HELLO) build -o $(HELLO_OUT)
	cargo run -q -p bower -- --book $(HELLO) verify
	cargo run -q -p bower -- --book $(HELLO) status -o $(HELLO_OUT)
	cargo run -q -p bower -- --book $(HELLO) push   -o $(HELLO_OUT)
	@echo
	@echo "dry run only. to publish: make ship-hello-execute"

ship-hello-execute: ship-hello ## the same, then push the repo, its site, and a release
	cargo run -q -p bower -- --book $(HELLO) push -o $(HELLO_OUT) --execute
