.PHONY: default help clean fmt build test lint security-scan docs ayce purity minimal slow book epub
default: ayce

help:  ## self-documenting: every target has a '## comment' printed here
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  make %-16s %s\n", $$1, $$2}'

clean: ## remove build artifacts
	cargo clean
	rm -rf books/*/book

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

book: ## render the sample book as HTML
	cargo build -p bower --all-features
	PATH="$(CURDIR)/target/debug:$$PATH" cargo run -q -p bower -- \
		--book books/hello-playbook publish --target html -o books/hello-playbook/book

epub: ## render the sample book as an epub (needs pandoc)
	cargo run -q -p bower -- --book books/hello-playbook publish --target epub -o published
