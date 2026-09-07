# elench

An evidence layer for repositories — and the substrate that replaces git.

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE-MIT)
[![Rust: stable](https://img.shields.io/badge/rust-stable-orange.svg)](rust-toolchain.toml)
[![CI](https://github.com/witlox/elench/actions/workflows/ci.yml/badge.svg)](https://github.com/witlox/elench/actions/workflows/ci.yml)

## Status

Implemented: Phases 0-5 + post-implementation (anchor resolution,
build provenance, `.git/` materialization, proptest, conflict
detection, continuous dogfooding, INV-15 artifact format).

**344 tests** (default), **351 tests** (with `fjall-backend`), **92%
line coverage**. All three binding experiments PASSED (E0: 0.72, E1:
99.4%, E2: cheap-to-fix). fmt clean, clippy clean.

## What this is

Version control records what changed. Forges record who approved it. Neither
records **what was checked, to what depth, and what remains unevaluated**.

When a human writes the code, that record lives in the human's head and the
review process samples it. When agents write the code at volume, there is no
head to sample and review bandwidth does not scale. The record has to become
durable state or it does not exist.

elench is that durable state — and the substrate itself. The claim log is
the primary history: a signed, append-only, revocable record of what was
asserted, what was verified, and what remains unevaluated. The git CLI
works because elench synthesizes git-compatible objects from the claim log
on demand. Humans use git; elench is invisible.

## What this is not

- **Not a git sidecar.** elench is the substrate (ADR-0001). Git is a
  read-only projection (ADR-0002), not the source of truth. There is no
  separate git repository underneath.
- **Not a CI system.** It defines what a build must emit, not how to build.
- **Not a consensus mechanism.** Independent parties evaluate the same claim
  log against their own policy and may legitimately reach different verdicts.
  Reconciling them is out of scope.
- **Not a reasoning capture system.** Prompts, chains of thought, and
  rationale are *justification*, not *verification*. Several existing tools
  capture the former. This captures the latter. If the distinction collapses
  in practice, the project has failed. See `docs/problem.md` section
  Anti-goals.

## The one claim that makes this worth building

Retroactive invalidation with traceable blast radius. A finding late in a
session can falsify a claim that a shipped artifact depended on, with no byte
of code changing. Current supply-chain tooling signs a verdict once and
freezes it. If elench is only SLSA with extra steps, abandon it.

## Architecture

elench is a Rust workspace of eight crates, organized by bounded context:

| Crate | Role |
|-------|------|
| [`elench`](crates/elench) | Binary. CLI + git projection + `.git/` materialization. |
| [`elench-claim`](crates/elench-claim) | Claim data model, log-folding status, emission validation, conflict detection. |
| [`elench-predicate`](crates/elench-predicate) | Parser and evaluator for `elench-predicate-v1` DSL (ADR-0004). |
| [`elench-envelope`](crates/elench-envelope) | DSSE envelopes carrying in-toto statements. Ed25519 (A1). |
| [`elench-store`](crates/elench-store) | Content-addressed store: blobs, trees, claims. The substrate (ADR-0001). |
| [`elench-gate`](crates/elench-gate) | Release gate evaluation + artifact format (INV-15). |
| [`elench-projection`](crates/elench-projection) | Deterministic git synthesis + `.git/` materialization (ADR-0002, ADR-0007). |
| [`elench-anchor`](crates/elench-anchor) | Multi-strategy anchor resolution + reconciliation (E1: strategy=multi). |

See [docs/architecture/overview.md](docs/architecture/overview.md) for
the full system overview, `specs/architecture/module-graph.md` for the
dependency graph, and `specs/architecture/build-phases.md` for the
implementation order (all phases COMPLETE).

## Quick start

```sh
git clone https://github.com/witlox/elench.git
cd elench
make              # fmt-check + lint + Tier 1 (before every commit)
make test         # Tier 1: cargo test --lib
make test-slow    # Tier 2: cargo test --all-targets
make test-full    # Tier 3: Tier 2 + dogfooding e2e
make dogfood      # full dogfooding pipeline (emit, gate, project, materialize)
```

Requires Rust 1.85+ (edition 2024). `rust-toolchain.toml` pins stable.
See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

### Feature flag

```sh
cargo test --workspace --all-targets --features elench/fjall-backend
```

Runs the persistent-store tests (`FjallStore::read_tree` round-trip,
cross-reopen, `--store fjall <path>` materialization).

## CLI

```sh
elench [--store memory|fjall <path>] <COMMAND> [OPTIONS]

# Core commands
elench emit <claim.json>           # Create, sign, and store a claim
elench verify <envelope.json>      # Verify a DSSE envelope
elench status <claim_id> [<log>]   # Compute claim status by folding
elench gate <tree_oid> [<log>]     # Evaluate the release gate
elench blast <claim_id> [<log>]    # Compute blast radius
elench conflicts <tree> <log>      # List same-anchor predicate conflicts
elench reconcile <tree> <log>      # Detect anchors that no longer resolve

# Git projection
elench git <claims.json>           # Synthesize git log (oneline)
elench git init <path> <claims.json>  # Materialize real .git/

# Store
elench store blob <file>           # Store a blob (SHA-256)
elench store tree <file1> <file2>  # Store a tree

# Artifact
elench artifact create <tree> <policy> <digest>  # Create release artifact
elench artifact verify <artifact.json> <log>     # Re-evaluate gate

# Build provenance
elench build <tree> [--artifact <path>] -- <cmd...>  # Run build, emit claim

# Log management
elench log <claims.json>           # Statistics (count, status, noise)
elench review <tree> <claims.json> # Review unevaluated claims
elench accept <tree> <log> --claim <id>  # Accept named gaps
elench compact <claims.json> [--before <ts>]  # Compact (destructive)

elench version                     # Print version
elench help                        # Print usage
```

See [docs/reference/cli.md](docs/reference/cli.md) for the full CLI
reference.

## Documentation

- [Getting Started](docs/guide/getting-started.md)
- [System Overview](docs/architecture/overview.md)
- [Problem Statement](docs/problem.md)
- [Anchoring](docs/anchoring.md)
- [Release Policy](docs/release-policy.md)
- [Operations](docs/operations/overview.md)
- [CLI Reference](docs/reference/cli.md)
- [Claim Schema](schema/claim.schema.json)
- [Artifact Schema](schema/artifact.schema.json)

## Project structure

```
elench/
  Cargo.toml              workspace root (8 crates)
  Cargo.lock              pinned dependencies
  rust-toolchain.toml     stable + rustfmt + clippy
  rustfmt.toml            max_width = 100
  deny.toml               license + advisory scanning
  Makefile                three-tier cascading test + lint + dogfooding
  book.toml               mdBook configuration
  AGENTS.md               workflow router + harness contract
  CONTRIBUTING.md         dev setup, coding standards, PR process
  NEXT-STEPS.md           what's done, what's next
  README.md               this file
  LICENSE-MIT             MIT license
  crates/                 8 Rust crates (elench + 7 libraries)
  schema/                 JSON schemas (claim, artifact)
  specs/                  specifications
    architecture/         module graph, API contracts, enforcement map,
                          error taxonomy, build phases, ADRs (0001-0008)
    features/             11 Gherkin feature files, 71 scenarios
    fidelity/             test depth per invariant
    findings/             adversarial sweep results
    cross-context/        integration points between bounded contexts
    escalations/          role-to-role escalations
    invariants.md         29 numbered invariants
    assumptions.md        23 tracked assumptions
    failure-modes.md      11 failure modes (P0-P3)
    domain-model.md       entities, value objects, aggregates
    ubiquitous-language.md  domain glossary
  experiments/            3 binding experiments (E0, E1, E2 — all PASSED)
  dogfooding/             claims, run.sh, emit-continuous.sh, README
  docs/                   user guide, architecture, operations, reference
  .github/workflows/      CI (3-tier) + feature matrix
  .opencode/commands/     status, spec-check, verify
  scripts/                set-version.sh
```

## Reading order

1. `docs/problem.md` — requirements (R1-R7), binding constraints (BC1-BC4)
2. `experiments/E0-predicate-ratio.md` — the go/no-go measurement
3. `schema/claim.schema.json` — draft data model
4. `docs/anchoring.md` — the unsolved problem everything rests on
5. `docs/release-policy.md` — the gate shape
6. `docs/architecture/overview.md` — system overview + crate map
7. `AGENTS.md` — workflow router + harness contract
8. `specs/` — ubiquitous language, domain model, invariants, features,
   failure modes, assumptions, fidelity, cross-context, architecture
9. `specs/architecture/adr/` — ADR log (0001-0008)

## License

Licensed under the [MIT License](LICENSE-MIT). Contributions
intentionally submitted for inclusion must be under the same terms.
