.PHONY: default fmt fmt-check lint test test-slow test-full coverage clean

# Default: fmt-check + lint + Tier 1. Run before every commit.
default: fmt-check lint test

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

lint:
	cargo clippy --all-targets -- -D warnings

# Tier 1: fast unit tests + BDD @smoke. Between every edit, pre-commit.
test:
	cargo test --lib

# Tier 2: Tier 1 + slow-marked tests + full BDD + race/coverage checks. Pre-PR.
test-slow: test
	cargo test --all-targets

# Tier 3: Tier 2 + e2e against real git repositories. Pre-merge / nightly.
test-full: test-slow
	@echo "e2e: not yet configured (requires real git repos for anchor survival)"

coverage:
	cargo llvm-cov --workspace --fail-under-lines 50

clean:
	cargo clean
