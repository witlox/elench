# elench

An evidence layer for repositories worked on by agents.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

## Status

Nothing is built. Nothing is decided. This repository contains a problem
statement, a draft data model, a spec scaffold, and three pre-registered
experiments whose results determine whether the project should exist at
all.

Do not begin implementation until E0 has run. See `experiments/`.

## What this is

Version control records what changed. Forges record who approved it. Neither
records **what was checked, to what depth, and what remains unevaluated**.

When a human writes the code, that record lives in the human's head and the
review process samples it. When agents write the code at volume, there is no
head to sample and review bandwidth does not scale. The record has to become
durable state or it does not exist.

elench proposes that durable state as a claim log: signed, append-only,
revocable assertions about a tree, stored in a git ref namespace alongside the
code and replicated by the same transport.

## What this is not

- **Not a git replacement.** Git is unmodified. Claims live in
  `refs/claims/`, adjacent to code, the way Radicle's COBs live in
  `refs/cobs/`. Human tooling is unaffected by design.
- **Not a CI system.** It defines what a build must emit, not how to build.
- **Not a consensus mechanism.** Independent parties evaluate the same claim
  log against their own policy and may legitimately reach different verdicts.
  Reconciling them is out of scope.
- **Not a reasoning capture system.** Prompts, chains of thought, and
  rationale are *justification*, not *verification*. Several existing tools
  capture the former. This captures the latter. If the distinction collapses
  in practice, the project has failed. See `docs/problem.md` §Anti-goals.

## The one claim that makes this worth building

Retroactive invalidation with traceable blast radius. A finding late in a
session can falsify a claim that a shipped artifact depended on, with no byte
of code changing. Current supply-chain tooling signs a verdict once and
freezes it. If elench is only SLSA with extra steps, abandon it.

## Architecture

elench is a Rust workspace of five crates, organized by bounded context:

| Crate | Role |
|-------|------|
| [`elench`](crates/elench) | Binary. The CLI: emit, verify, status, gate, blast. |
| [`elench-claim`](crates/elench-claim) | Claim data model, log-folding status computation, emission-rule validation. |
| [`elench-envelope`](crates/elench-envelope) | DSSE envelopes carrying in-toto statements. |
| [`elench-store`](crates/elench-store) | Git ref namespace operations (`refs/claims/<type>/<id>`). |
| [`elench-gate`](crates/elench-gate) | Release gate evaluation — a predicate over claims, not a build. |

See `specs/architecture/module-graph.md` for the dependency graph and
`specs/architecture/build-phases.md` for the implementation order (all
phases are gated by E0; the validator is the first milestone per
ADR-0006).

## Quick start

```sh
make              # fmt-check + lint + Tier 1 (before every commit)
make test         # Tier 1: cargo test --lib
make test-slow    # Tier 2: all tests including slow-marked
make test-full    # Tier 3: + e2e (not yet configured)
```

Requires Rust 1.85+ (edition 2024). See `CONTRIBUTING.md` for details.

## Reading order

1. `docs/problem.md` — requirements (R1–R7), binding constraints (BC1–BC3)
2. `experiments/E0-predicate-ratio.md` — the go/no-go measurement
3. `schema/claim.schema.json` — draft data model
4. `docs/anchoring.md` — the unsolved problem everything rests on
5. `docs/release-policy.md` — the gate shape
6. `AGENTS.md` — workflow router + harness contract
7. `specs/` — ubiquitous language, domain model, invariants, features,
   failure modes, assumptions, fidelity, cross-context, architecture
8. `specs/architecture/adr/` — ADR log (0000–0006, all proposed except 0006)

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE),
at your option. Contributions intentionally submitted for inclusion
must be under the same terms.
