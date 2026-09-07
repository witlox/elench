# elench

An evidence layer for repositories — and the substrate that replaces git.

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE-MIT)
[![Rust: stable](https://img.shields.io/badge/rust-stable-orange.svg)](rust-toolchain.toml)
[![CI](https://github.com/witlox/elench/actions/workflows/ci.yml/badge.svg)](https://github.com/witlox/elench/actions/workflows/ci.yml)

**344 tests** (default), **351** (with `fjall-backend`), **92% line
coverage**. All three binding experiments PASSED. fmt clean, clippy
clean.

## What this is

Version control records what changed. Forges record who approved it.
Neither records **what was checked, to what depth, and what remains
unevaluated**. elench is that durable state — a signed, append-only,
revocable claim log that IS the primary history. The git CLI works
because elench synthesizes git-compatible objects from the claim log
on demand. Humans use git; elench is invisible.

## Quick start

```sh
git clone https://github.com/witlox/elench.git
cd elench
make              # fmt-check + lint + Tier 1
make test         # cargo test --lib
make test-slow    # cargo test --all-targets
make dogfood      # full dogfooding pipeline
```

Requires Rust 1.85+ (edition 2024). See
[CONTRIBUTING.md](CONTRIBUTING.md) and [docs/guide/getting-started.md](docs/guide/getting-started.md).

## Documentation

Full documentation is published as an [mdBook](https://witlox.github.io/elench/)
and available in `docs/`:

- [Getting Started](docs/guide/getting-started.md)
- [System Overview](docs/architecture/overview.md)
- [CLI Reference](docs/reference/cli.md)
- [Operations](docs/operations/overview.md)
- [Problem Statement](docs/problem.md)
- [Anchoring](docs/anchoring.md)
- [Release Policy](docs/release-policy.md)

Specifications in `specs/` (ubiquitous language, domain model,
invariants, features, ADRs 0001-0008, fidelity index, cross-context
interactions, failure modes, assumptions).

## License

[MIT](LICENSE-MIT)
