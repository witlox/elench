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
     └────┬─────┘ └──────────┘ └────────┘ └────┬───┘
          │                                      │
          │    ┌─────────────┐                   │
          └────┤ elench-     │                   │
               │ predicate   │                   │
               └─────────────┘                   │
                                                 │
     ┌─────────────┐                            │
     │ elench-     │                            │
     │ projection  ├────────────────────────────┘
     └──────┬──────┘
            │
            ▼
     ┌──────────┐
     │elench-   │
     │store     │
     └──────────┘
```

## Crates

| Crate | Path | Bounded context | Role |
|-------|------|-----------------|------|
| `elench` | `crates/elench` | CLI, Git Projection | Binary. Reads/writes store, verifies envelopes, evaluates predicates, computes closures, synthesizes git projection. |
| `elench-claim` | `crates/elench-claim` | Claim Emission, Claim Evaluation | Data model matching `schema/claim.schema.json`, log-folding status computation, emission-rule validation. |
| `elench-predicate` | `crates/elench-predicate` | Predicate Evaluation | Parser and evaluator for `elench-predicate-v1` DSL (ADR-0004). Four primitives: grep, test, run, exists. |
| `elench-envelope` | `crates/elench-envelope` | Envelope Verification | DSSE envelope signing/verification, in-toto statement handling, signer/producer distinction. |
| `elench-store` | `crates/elench-store` | elench Store | Content-addressed store: blobs, trees, claims. The substrate (ADR-0001). In-memory (default), fjall (optional, ADR-0008). |
| `elench-gate` | `crates/elench-gate` | Release Gating | Release policy evaluation: four conditions from `docs/release-policy.md`. |
| `elench-projection` | `crates/elench-projection` | Git Projection | Deterministic synthesis of git-compatible objects from the claim log (ADR-0002, ADR-0007). |

## Dependencies

| From | To | Why |
|------|----|-----|
| `elench` | `elench-claim` | Claim data model |
| `elench` | `elench-envelope` | Envelope signing/verification |
| `elench` | `elench-store` | Content-addressed storage |
| `elench` | `elench-gate` | Release gate evaluation |
| `elench` | `elench-predicate` | Predicate validation |
| `elench` | `elench-projection` | Git projection |
| `elench-envelope` | `elench-claim` | Claim type (sign/verify) |
| `elench-store` | `elench-claim` | Claim type (store/read) |
| `elench-gate` | `elench-claim` | Status computation (log fold) |
| `elench-projection` | `elench-claim` | Claim types |
| `elench-projection` | `elench-store` | Store (read trees) |

No cycles. `elench-claim` is the sole leaf — it depends on nothing in
the workspace. `elench-predicate` depends on nothing in the workspace
(regex is external). `elench-envelope`, `elench-store`, and
`elench-gate` all depend on `elench-claim`. `elench-projection`
depends on `elench-claim` and `elench-store`. The binary depends on
all six library crates.

## Deferred

| Crate | Why deferred |
|-------|-------------|
| `elench-anchor` | E1 PASSED (strategy=multi), but crate not yet created. Anchor object in schema/claim.schema.json uses strategy=multi by default. |
