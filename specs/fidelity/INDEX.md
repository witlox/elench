# Fidelity Index

Test depth per invariant. All phases (0–5) are implemented. 161 tests,
90% line coverage. fmt clean, clippy clean.

## Invariants

| INV | Description | Depth | Notes |
|-----|-------------|-------|-------|
| INV-01 | Status changed by appending, not modifying | MOCK | elench-store: store_blob/tree/claim idempotent |
| INV-02 | Prior status remains visible | MOCK | elench-claim: compute_status reads entire log |
| INV-03 | Claim identity stable across status changes | MOCK | elench-claim: ClaimId immutable |
| INV-04 | Status computed by folding, not stored | MOCK | elench-claim: compute_status pure fn |
| INV-05 | origin.kind required and structurally distinct | MOCK | elench-claim: type system enforces |
| INV-06 | Agents cannot emit harness-observed | MOCK | elench-claim: validate_claim cross-checks signer |
| INV-07 | Only harness emits verification | MOCK | elench-claim: validate_claim cross-checks signer |
| INV-08 | Predicate requires executable expression | MOCK | elench-claim: validate_claim calls elench-predicate::parse |
| INV-09 | Annotations never read by policy | MOCK | elench-gate: evaluate filters on form=predicate |
| INV-10 | dependsOn populated with premises | REMOVED | Downgraded to guideline; empty dependsOn is a warning, not a rejection |
| INV-11 | Failure recorded only when status changed | MOCK | elench-claim: validate_claim checks target status in log |
| INV-12 | Agents cannot emit residue-acceptance | MOCK | elench-claim: validate_claim cross-checks signer |
| INV-13 | Gate evaluable without build capability | MOCK | elench-gate: evaluate takes &[Claim], no build |
| INV-14 | Artifact acceptability is live evaluation | MOCK | elench-gate: evaluate called on demand, no cache |
| INV-15 | Artifact carries (tree, policy), not verdict | NONE | Future — artifact format not yet defined |
| INV-16 | unevaluated is first-class status | MOCK | elench-claim: ClaimStatus::Unevaluated |
| INV-17 | Policies permit bounded unevaluated residue | MOCK | elench-gate: condition 2, residue-acceptance |
| INV-18 | elench owns content-addressed store | MOCK | elench-store: no git dependency, owns storage |
| INV-19 | Git projection is read-only | MOCK | elench-projection: synthesize takes &Store, no writes |
| INV-20 | Git synthesis is deterministic (BC4) | MOCK | elench-projection: scenario_deterministic_synthesis_identical_oids |
| INV-21 | Git projection produces no side effects | MOCK | elench-projection: scenario_inv19_projection_does_not_write_to_store |
| INV-22 | Agent claims and provenance share DSSE/in-toto | MOCK | elench-envelope: sign/verify, PREDICATE_TYPE_AGENT |
| INV-23 | Predicate expressions executable/deterministic/sandboxable | MOCK | elench-predicate: 4 primitives, not Turing-complete |
| INV-24 | AGENTS.md rules enforced by validator | MOCK | elench-claim: validate_claim (Phase 0) |
| INV-25 | Content addressing (SHA-256) | MOCK | elench-claim: ClaimId::from_content; elench-store: Oid::from_blob_data/from_tree_entries |
| INV-26 | Store is sole source of truth | MOCK | elench-store: all views derive from store alone |
| INV-27 | Git projection is lossy, not authoritative | MOCK | elench-projection: scenario_inv27_projection_is_lossy |
| INV-28 | Claim OID is content hash | MOCK | elench-claim: ClaimId::from_content (SHA-256 of canonical JSON) |
| INV-29 | dependsOn acyclic | MOCK | elench-claim: validate_claim DFS cycle detection |

28 active invariants (24 original minus INV-10 removed, plus INV-25/26/27/28/29 added).
27 ENFORCED (code + tests), 1 future (INV-15 artifact format).

## Features

| Feature | Scenarios | Depth | Notes |
|---------|-----------|-------|-------|
| claim-emission | 7 | MOCK | elench-claim: validate_claim + types |
| claim-revocation | 7 | MOCK | elench-claim: compute_status + blast_radius + dependsOn propagation |
| origin-typing | 6 | MOCK | elench-claim: validate_claim cross-checks |
| release-gate | 9 | MOCK | elench-gate: evaluate, 4 conditions, hermeticity floor |
| anchor-resolution | 5 | NONE | E1 PASSED; elench-anchor deferred (strategy=multi) |
| unevaluated-residue | 6 | MOCK | elench-claim: ClaimStatus::Unevaluated |
| git-projection | 4 | MOCK | elench-projection: synthesize, git_log_oneline/full |

## Experiments

| Experiment | Gates | Status | Result |
|------------|-------|--------|--------|
| E0 — Predicate ratio | BC1 | **PASSED** | 0.72 (threshold >= 0.30). PROCEED AS DESIGNED. |
| E1 — Anchor survival | BC2 | **PASSED** | All strategies USABLE (correct >= 85%, wrong <= 2%). Proceed with multi. |
| E2 — Build reproducibility | BC3 | **PASSED** | Same-triple divergences all cheap-to-fix. K-of-N available. |

## Implementation status

All phases (0–5) are **implemented**:
- Phase 0: elench-predicate (25 tests) + elench-claim (40 tests)
- Phase 1: elench-store (30 tests)
- Phase 2: elench-envelope (13 tests)
- Phase 3: elench-gate (19 tests)
- Phase 4: elench-projection (14 tests)
- Phase 5: elench CLI (13 cli + 7 integration tests)

Total: 161 tests, 90% line coverage. fmt clean, clippy clean.

Store backend: in-memory (default), fjall (optional feature, ADR-0008).
