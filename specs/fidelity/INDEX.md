# Fidelity Index

Test depth per invariant. Phase 0 (elench-predicate, elench-claim) is
implemented — see depth column. All other invariants are NONE.

## Invariants

| INV | Description | Depth | Notes |
|-----|-------------|-------|-------|
| INV-01 | Status changed by appending, not modifying | NONE | elench-store (Phase 1) |
| INV-02 | Prior status remains visible | NONE | elench-store (Phase 1) |
| INV-03 | Claim identity stable across status changes | MOCK | elench-claim: ClaimId immutable |
| INV-04 | Status computed by folding, not stored | MOCK | elench-claim: compute_status pure fn |
| INV-05 | origin.kind required and structurally distinct | MOCK | elench-claim: type system enforces |
| INV-06 | Agents cannot emit harness-observed | MOCK | elench-claim: validate_claim cross-checks signer |
| INV-07 | Only harness emits verification | MOCK | elench-claim: validate_claim cross-checks signer |
| INV-08 | Predicate requires executable expression | MOCK | elench-claim: validate_claim calls elench-predicate::parse |
| INV-09 | Annotations never read by policy | NONE | elench-gate (Phase 3) |
| INV-10 | dependsOn populated with premises | REMOVED | Downgraded to guideline; empty dependsOn is a warning, not a rejection |
| INV-11 | Failure recorded only when status changed | MOCK | elench-claim: validate_claim checks target status in log |
| INV-12 | Agents cannot emit residue-acceptance | MOCK | elench-claim: validate_claim cross-checks signer |
| INV-13 | Gate evaluable without build capability | NONE | elench-gate (Phase 3) |
| INV-14 | Artifact acceptability is live evaluation | NONE | elench-gate (Phase 3) |
| INV-15 | Artifact carries (tree, policy), not verdict | NONE | elench (Phase 5) |
| INV-16 | unevaluated is first-class status | MOCK | elench-claim: ClaimStatus::Unevaluated |
| INV-17 | Policies permit bounded unevaluated residue | NONE | elench-gate (Phase 3) |
| INV-18 | elench owns content-addressed store | NONE | elench-store (Phase 1) |
| INV-19 | Git projection is read-only | NONE | elench (Phase 4) |
| INV-20 | Git synthesis is deterministic (BC4) | NONE | elench (Phase 4) |
| INV-21 | Git projection produces no side effects | NONE | elench (Phase 4) |
| INV-22 | Agent claims and provenance share DSSE/in-toto | NONE | elench-envelope (Phase 2) |
| INV-23 | Predicate expressions executable/deterministic/sandboxable | MOCK | elench-predicate: 4 primitives, not Turing-complete |
| INV-24 | AGENTS.md rules enforced by validator | MOCK | elench-claim: validate_claim (Phase 0) |
| INV-25 | Content addressing (SHA-256) | MOCK | elench-claim: ClaimId::from_content (claims); elench-store pending (blobs/trees) |
| INV-26 | Store is sole source of truth | NONE | elench-store (Phase 1) |
| INV-27 | Git projection is lossy, not authoritative | NONE | elench (Phase 4) |
| INV-28 | Claim OID is content hash | MOCK | elench-claim: ClaimId::from_content (SHA-256 of canonical JSON) |
| INV-29 | dependsOn acyclic | MOCK | elench-claim: validate_claim DFS cycle detection |

28 active invariants (24 original minus INV-10 removed, plus INV-25/26/27/28/29 added).

## Features

| Feature | Scenarios | Depth | Notes |
|---------|-----------|-------|-------|
| claim-emission | 7 | MOCK | elench-claim: validate_claim + types |
| claim-revocation | 7 | MOCK | elench-claim: compute_status + blast_radius |
| origin-typing | 6 | MOCK | elench-claim: validate_claim cross-checks |
| release-gate | 9 | NONE | elench-gate (Phase 3) |
| anchor-resolution | 5 | NONE | E1 PASSED; elench-anchor deferred |
| unevaluated-residue | 6 | MOCK | elench-claim: ClaimStatus::Unevaluated |
| git-projection | 4 | NONE | elench (Phase 4) |

## Experiments

| Experiment | Gates | Status | Result |
|------------|-------|--------|--------|
| E0 — Predicate ratio | BC1 | **PASSED** | 0.72 (threshold >= 0.30). PROCEED AS DESIGNED. |
| E1 — Anchor survival | BC2 | **PASSED** | All strategies USABLE (correct >= 85%, wrong <= 2%). Proceed with multi. |
| E2 — Build reproducibility | BC3 | **PASSED** | Same-triple divergences all cheap-to-fix. K-of-N available. |

## Phase 0 — Validator (ADR-0006)

The validator is **implemented**. `elench-predicate` (parser + evaluator,
22 tests) and `elench-claim` (types, validate_claim, compute_status,
blast_radius, 32 tests) are built and passing.

Phase 1 (elench-store) is the next implementation target.
