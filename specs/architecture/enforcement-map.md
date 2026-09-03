# Enforcement Map

For each invariant, where it is enforced. All invariants are ENFORCED
(code exists and tests fail if violated), except INV-15 (artifact
format, future).

## Revocability and status (R1)

| INV | Enforcement point | Status |
|-----|-------------------|--------|
| INV-01: Append, not modify | `elench-store::store_blob/tree/claim` — idempotent, no update | ENFORCED |
| INV-02: Prior status visible | `elench-claim::compute_status` — fold reads all records | ENFORCED |
| INV-03: Claim identity stable | `elench-claim::ClaimId` — content address, immutable | ENFORCED |
| INV-04: Status computed, not stored | `elench-claim::compute_status` — pure function | ENFORCED |
| INV-28: Claim OID is content hash | `elench-claim::ClaimId::from_content` — SHA-256 of canonical JSON | ENFORCED |
| INV-29: dependsOn acyclic | `elench-claim::validate_claim` — DFS cycle detection over log | ENFORCED |

## Origin typing (R2, AGENTS.md)

| INV | Enforcement point | Status |
|-----|-------------------|--------|
| INV-05: origin.kind required | `elench-claim` — type system, `OriginKind` non-optional | ENFORCED |
| INV-06: No harness-observed from agents | `elench-claim::validate_claim` — cross-checks signer.entity against origin.kind | ENFORCED |
| INV-07: Only harness emits verification | `elench-claim::validate_claim` — cross-checks signer.entity against kind | ENFORCED |
| INV-08: Predicate requires expression | `elench-claim::validate_claim` — calls elench-predicate::parse | ENFORCED |
| INV-09: Annotations never read by policy | `elench-gate::evaluate` — filters on form=predicate, skips annotations | ENFORCED |
| INV-11: Failure recorded only when status changed | `elench-claim::validate_claim` — checks target status in log | ENFORCED |
| INV-12: No residue-acceptance from agents | `elench-claim::validate_claim` — cross-checks signer.entity against kind | ENFORCED |

## Gate and release (R3, R4)

| INV | Enforcement point | Status |
|-----|-------------------|--------|
| INV-13: Gate without build | `elench-gate::evaluate` — takes &[Claim], no build calls | ENFORCED |
| INV-14: Live evaluation | `elench-gate::evaluate` — called on demand, no cached verdict | ENFORCED |
| INV-15: Artifact carries (tree, policy) | `elench` CLI — artifact format not yet defined | FUTURE |

## Unevaluated (R5)

| INV | Enforcement point | Status |
|-----|-------------------|--------|
| INV-16: unevaluated first-class | `elench-claim::ClaimStatus::Unevaluated` | ENFORCED |
| INV-17: Bounded residue with acceptance | `elench-gate::evaluate` — condition 2, residue-acceptance records | ENFORCED |

## Substrate and projection (R6, ADR-0001, ADR-0002, ADR-0007)

| INV | Enforcement point | Status |
|-----|-------------------|--------|
| INV-18: elench owns content-addressed store | `elench-store` — no git dependency, owns storage | ENFORCED |
| INV-19: Git projection is read-only | `elench-projection::synthesize` — takes &Store, no writes | ENFORCED |
| INV-20: Git synthesis deterministic | `elench-projection` — scenario_deterministic_synthesis_identical_oids | ENFORCED |
| INV-21: Git projection no side effects | `elench-projection` — scenario_inv19_projection_does_not_write_to_store | ENFORCED |
| INV-25: Content addressing (SHA-256) | `elench-store::Oid::from_blob_data/from_tree_entries`; `elench-claim::ClaimId::from_content` | ENFORCED |
| INV-26: Store is sole source of truth | `elench-store` — all views derive from store; `elench-gate` takes &[Claim] | ENFORCED |
| INV-27: Git projection is lossy, not authoritative | `elench-projection` — scenario_inv27_projection_is_lossy | ENFORCED |

## Supply-chain composability (R7, ADR-0003)

| INV | Enforcement point | Status |
|-----|-------------------|--------|
| INV-22: DSSE/in-toto shared format | `elench-envelope::sign` / `verify`; PREDICATE_TYPE_AGENT | ENFORCED |

## Predicate language (ADR-0004)

| INV | Enforcement point | Status |
|-----|-------------------|--------|
| INV-23: Expressions executable/deterministic/sandboxable | `elench-predicate` — 4 primitives, not Turing-complete | ENFORCED |

## Validator (ADR-0006)

| INV | Enforcement point | Status |
|-----|-------------------|--------|
| INV-24: AGENTS.md rules enforced | `elench-claim::validate_claim` — implemented | ENFORCED |
