# Enforcement Map

For each invariant, where it is enforced. All 28 invariants are
ENFORCED (code exists and tests fail if violated). INV-15 was
upgraded from FUTURE to MOCK — the artifact now carries a `version`
field and `schema/artifact.schema.json` defines the format.

## Revocability and status (R1)

| INV | Enforcement point | Status |
|-----|-------------------|--------|
| INV-01: Append, not modify | `elench-store::store_blob/tree/claim` — idempotent, no update (MemoryStore + FjallStore) | ENFORCED |
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
| INV-14: Artifact acceptability is live evaluation | `elench-gate::evaluate` — called on demand, no cached verdict | ENFORCED |
| INV-15: Artifact carries (tree, policy), not verdict | `elench-gate::Artifact` — `version`, `tree`, `policy`, `digest`, `released_at`; `schema/artifact.schema.json`; no verdict field | ENFORCED (MOCK) |

## Unevaluated (R5)

| INV | Enforcement point | Status |
|-----|-------------------|--------|
| INV-16: unevaluated first-class | `elench-claim::ClaimStatus::Unevaluated` | ENFORCED |
| INV-17: Bounded residue with acceptance | `elench-gate::evaluate` — condition 2, residue-acceptance records | ENFORCED |

## Substrate and projection (R6, ADR-0001, ADR-0002, ADR-0007)

| INV | Enforcement point | Status |
|-----|-------------------|--------|
| INV-18: elench owns content-addressed store | `elench-store` — no git dependency, owns storage; MemoryStore (default) + FjallStore (optional, ADR-0008) | ENFORCED |
| INV-19: Git projection is read-only | `elench-projection::synthesize` — takes `&dyn StoreBackend`, no writes; `elench-projection::materialize` — writes to filesystem, not to store | ENFORCED |
| INV-20: Git synthesis deterministic | `elench-projection` — `proptest_inv_20_synthesis_deterministic` + `proptest_inv_20_synthesis_order_independent` | ENFORCED |
| INV-21: Git projection no side effects | `elench-projection` — `scenario_inv19_projection_does_not_write_to_store` | ENFORCED |
| INV-25: Content addressing (SHA-256) | `elench-store::Oid::from_blob_data/from_tree_entries` + `deserialize_tree_bytes` round-trip; `elench-claim::ClaimId::from_content`; `FjallStore::read_tree` round-trip (feature tier) | ENFORCED |
| INV-26: Store is sole source of truth | `elench-store` — all views derive from store; `--store memory|fjall <path>` selects backend at runtime | ENFORCED |
| INV-27: Git projection is lossy, not authoritative | `elench-projection` — `scenario_inv27_projection_is_lossy` | ENFORCED |

## Supply-chain composability (R7, ADR-0003)

| INV | Enforcement point | Status |
|-----|-------------------|--------|
| INV-22: DSSE/in-toto shared format | `elench-envelope::sign` / `verify`; PREDICATE_TYPE_AGENT; Ed25519 signatures (A1) | ENFORCED |

## Predicate language (ADR-0004)

| INV | Enforcement point | Status |
|-----|-------------------|--------|
| INV-23: Expressions executable/deterministic/sandboxable | `elench-predicate` — 4 primitives (grep, test, run, exists), not Turing-complete | ENFORCED |

## Validator (ADR-0006)

| INV | Enforcement point | Status |
|-----|-------------------|--------|
| INV-24: AGENTS.md rules enforced | `elench-claim::validate_claim` — implemented, all rules enforced | ENFORCED |

## Conflict detection

| Function | Enforcement point | Status |
|----------|-------------------|--------|
| `elench_claim::detect_conflicts` | Same-anchor, different expression — `crates/elench-claim/src/lib.rs`; 8 unit tests + 4 CLI tests | ENFORCED |
