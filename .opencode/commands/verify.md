---
description: "Pre-commit verification: fmt + lint + Tier 1 tests. Run before every commit."
---

Pre-commit verification. Run this before every commit.

1. Format: `cargo fmt --all -- --check` — must pass
2. Lint: `cargo clippy --all-targets -- -D warnings` — must be 0 warnings
3. Build: `cargo build` — must succeed
4. Unit tests: `cargo test --lib` — all must pass

If any step fails, do not commit. Run `make fmt` to auto-format,
then re-run.

For the elench binary (CLI integration tests):
5. CLI tests: `cargo test -p elench --test cli` — all must pass

For the fjall backend (if changes touch elench-store):
6. Feature tests: `cargo test --workspace --all-targets --features elench/fjall-backend`

The Makefile wraps these:
```
make              # fmt-check + lint + Tier 1 (equivalent to steps 1-4)
make test-slow    # Tier 2: all targets (equivalent to steps 1-5)
```
