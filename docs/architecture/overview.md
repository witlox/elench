# System Overview

elench is an evidence layer for repositories — and the substrate that
replaces git. The claim log is the primary history: a signed,
append-only, revocable record of what was asserted, what was verified,
and what remains unevaluated. The git CLI works because elench
synthesizes git-compatible objects from the claim log on demand.
Humans use git; elench is invisible.

## The problem

Version control records what changed. Forges record who approved it.
Neither records **what was checked, to what depth, and what remains
unevaluated**. When agents write code at volume, there is no human
head to sample and review bandwidth does not scale. The record has to
become durable state or it does not exist.

See [`problem.md`](problem.md) for the full requirements (R1–R7),
binding constraints (BC1–BC4), and anti-goals.

## The one claim that makes this worth building

Retroactive invalidation with traceable blast radius. A finding late
in a session can falsify a claim that a shipped artifact depended on,
with no byte of code changing. Current supply-chain tooling signs a
verdict once and freezes it. If elench is only SLSA with extra steps,
abandon it.

## Workspace structure

The codebase is a single Rust workspace with 8 crates:

| Crate | Role |
|-------|------|
| [`elench`](../crates/elench) | Binary. CLI + git projection. Synthesizes git objects from the claim log (ADR-0002, ADR-0007). Materializes real `.git/` directories. |
| [`elench-claim`](../crates/elench-claim) | Claim data model, log-folding status computation, emission-rule validation, conflict detection. |
| [`elench-predicate`](../crates/elench-predicate) | Parser and evaluator for `elench-predicate-v1` DSL (ADR-0004). Four primitives: grep, test, run, exists. |
| [`elench-envelope`](../crates/elench-envelope) | DSSE envelopes carrying in-toto statements. Ed25519 signing/verification (A1). |
| [`elench-store`](../crates/elench-store) | Content-addressed store: blobs, trees, claims. The substrate (ADR-0001). In-memory (default), fjall (optional, ADR-0008). |
| [`elench-gate`](../crates/elench-gate) | Release gate evaluation — a predicate over claims, not a build. Artifact format (INV-15). |
| [`elench-projection`](../crates/elench-projection) | Deterministic git synthesis + `.git/` materialization from the claim log (ADR-0002, ADR-0007). |
| [`elench-anchor`](../crates/elench-anchor) | Multi-strategy anchor resolution: path-range, symbol, content-digest (E1: strategy=multi). Reconciliation. |

See [`specs/architecture/module-graph.md`](../specs/architecture/module-graph.md)
for the dependency graph and [`specs/architecture/build-phases.md`](../specs/architecture/build-phases.md)
for the implementation order (all phases COMPLETE).

## Architecture diagram

```
                    ┌─────────────┐
                    │   elench    │  (binary: CLI + git projection)
                    │  (crates/   │
                    │   elench)   │
                    └──────┬──────┘
            ┌────────┬─────┴──────┬─────────┐
            ▼        ▼            ▼         ▼
     ┌──────────┐ ┌──────────┐ ┌────────┐ ┌────────┐
     │elench-   │ │elench-   │ │elench- │ │elench- │
     │claim     │ │envelope  │ │store   │ │gate    │
     └────┬─────┘ └──────────┘ └────────┘ └────┬───┘
          │                                      │
          │    ┌─────────────┐                   │
          └────┤ elench-     │                   │
                │ predicate   │                   │
                └─────────────┘                   │
                                                  │
     ┌─────────────┐    ┌─────────────┐          │
     │ elench-     │    │ elench-     │          │
     │ anchor      │    │ projection  ├──────────┘
     └──────┬──────┘    └──────┬──────┘
            │                  │
            ▼                  ▼
     ┌──────────┐       ┌──────────┐
     │elench-   │       │elench-   │
     │store     │       │store     │
     └──────────┘       └──────────┘
```

## Key design decisions

- **ADR-0001**: elench is the substrate; the claim log is the primary
  history. Git is a read-only projection, not the source of truth.
- **ADR-0002**: Git objects are synthesized deterministically from the
  claim log. Two parties with the same log produce byte-identical
  objects.
- **ADR-0003**: DSSE envelopes carrying in-toto statements. Agent
  claims and build provenance share the same signing path.
- **ADR-0004**: A small DSL of check primitives (grep, test, run,
  exists). Not Turing-complete. Gated by E0 (ratio 0.72).
- **ADR-0007**: One commit per tree-changing claim. Deterministic
  author/committer/timestamp derivation.
- **ADR-0008**: Fjall as the persistent content-addressed backend.
  In-memory by default; `--store fjall <path>` enables persistence.

See [`specs/architecture/adr/`](../specs/architecture/adr/) for all
ADRs (0001–0008).
