# Module Graph

Derived from the domain model (bounded contexts) and the ADR log. Each
crate maps to one or more bounded contexts. Dependencies are acyclic.

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
     └──────────┘ └──────────┘ └────────┘ └────┬───┘
                                                 │
                                          ┌──────┴───┐
                                          │elench-   │
                                          │claim     │
                                          └──────────┘
```

## Crates

| Crate | Path | Bounded context | Role |
|-------|------|-----------------|------|
| `elench` | `crates/elench` | CLI, Git Projection | Binary. Reads/writes store, verifies envelopes, evaluates predicates, computes closures, synthesizes git projection. |
| `elench-claim` | `crates/elench-claim` | Claim Emission, Claim Evaluation | Data model matching `schema/claim.schema.json`, log-folding status computation, emission-rule validation. |
| `elench-envelope` | `crates/elench-envelope` | Envelope Verification | DSSE envelope signing/verification, in-toto statement handling, signer/producer distinction. |
| `elench-store` | `crates/elench-store` | elench Store | Content-addressed store: blobs, trees, claims. The substrate (ADR-0001). No git, no daemon. |
| `elench-gate` | `crates/elench-gate` | Release Gating | Release policy evaluation: four conditions from `docs/release-policy.md`. |

## Dependencies

| From | To | Why |
|------|----|-----|
| `elench` | `elench-claim` | Claim data model |
| `elench` | `elench-envelope` | Envelope signing/verification |
| `elench` | `elench-store` | Content-addressed storage |
| `elench` | `elench-gate` | Release gate evaluation |
| `elench-gate` | `elench-claim` | Status computation (log fold) |

No cycles. `elench-claim` is the leaf — it depends on nothing in the
workspace. `elench-gate` depends on `elench-claim`. The binary depends
on all four library crates. The git projection lives in the binary
(`elench`) because it is a synthesis of objects from the store and the
claim log — it has no separate crate because it is not reusable
infrastructure, it is the compatibility layer.

## Not yet created

| Crate | Why deferred |
|-------|-------------|
| `elench-anchor` | Anchor strategy is UNRESOLVED (E1 gates). Created when E1 picks a strategy. |
| `elench-predicate` | Predicate language is UNDECIDED (E0 gates, ADR-0004). Created when ADR-0004 is decided. |
