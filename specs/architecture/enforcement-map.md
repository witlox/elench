# Enforcement Map

For each invariant, where it is enforced (or will be). Status:
UNIMPLEMENTED (no code), PLANNED (contract exists), or ENFORCED (code
exists and a test fails if violated).

All invariants are UNIMPLEMENTED. The "Enforcement point" column names
where enforcement WILL live once the validator is built (ADR-0006).

## Revocability and status (R1)

| INV | Enforcement point | Status |
|-----|-------------------|--------|
| INV-01: Append, not modify | `elench-store::store_claim` — only writes, never updates | UNIMPLEMENTED |
| INV-02: Prior status visible | `elench-claim::compute_status` — fold reads all records | UNIMPLEMENTED |
| INV-03: Claim identity stable | `elench-claim::ClaimId` — content address, immutable | UNIMPLEMENTED |
| INV-04: Status computed, not stored | `elench-claim::compute_status` — pure function | UNIMPLEMENTED |

## Origin typing (R2, AGENTS.md)

| INV | Enforcement point | Status |
|-----|-------------------|--------|
| INV-05: origin.kind required | `elench-claim::validate_claim` — schema validation | UNIMPLEMENTED |
| INV-06: No harness-observed from agents | `elench-claim::validate_claim` — reject if producer is agent and origin.kind = harness-observed | UNIMPLEMENTED |
| INV-07: Only harness emits verification | `elench-claim::validate_claim` — reject kind=verification from non-harness | UNIMPLEMENTED |
| INV-08: Predicate requires expression | `elench-claim::validate_claim` — reject form=predicate without expression | UNIMPLEMENTED |
| INV-09: Annotations never read by policy | `elench-gate::evaluate` — filter on form before evaluating | UNIMPLEMENTED |
| INV-11: Failure recorded only when status changed | `elench-claim` — emission rule, checked by validator | UNIMPLEMENTED |
| INV-12: No residue-acceptance from agents | `elench-claim::validate_claim` — reject kind=residue-acceptance from non-human | UNIMPLEMENTED |

## Gate and release (R3, R4)

| INV | Enforcement point | Status |
|-----|-------------------|--------|
| INV-13: Gate without build | `elench-gate::evaluate` — no build calls in evaluation path | UNIMPLEMENTED |
| INV-14: Live evaluation | `elench-gate::evaluate` — called on demand, no cached verdict | UNIMPLEMENTED |
| INV-15: Artifact carries (tree, policy) | `elench` CLI — artifact format includes elench tree OID, not verdict | UNIMPLEMENTED |

## Unevaluated (R5)

| INV | Enforcement point | Status |
|-----|-------------------|--------|
| INV-16: unevaluated first-class | `elench-claim::ClaimStatus::Unevaluated` | UNIMPLEMENTED |
| INV-17: Bounded residue with acceptance | `elench-gate::evaluate` — condition 2 of release policy | UNIMPLEMENTED |

## Substrate and projection (R6, ADR-0001, ADR-0002, ADR-0007)

| INV | Enforcement point | Status |
|-----|-------------------|--------|
| INV-18: elench owns content-addressed store | `elench-store` — no git dependency, owns storage | UNIMPLEMENTED |
| INV-19: Git projection is read-only | `elench` binary (git projection) — no write path to store | UNIMPLEMENTED |
| INV-20: Git synthesis deterministic | `elench` binary (git projection) — ADR-0007, tested by two-machine synthesis | UNIMPLEMENTED |
| INV-21: Git projection no side effects | `elench` binary (git projection) — reads only, generates objects | UNIMPLEMENTED |
| INV-25: Content addressing (SHA-256) | `elench-store` — OID computation, verified on read | UNIMPLEMENTED |
| INV-26: Store is sole source of truth | `elench-gate::evaluate`, `elench` binary (projection) — no external state reads | UNIMPLEMENTED |
| INV-27: Git projection is lossy, not authoritative | `elench` binary (projection) — no status/origin/blast-radius in git objects | UNIMPLEMENTED |

## Supply-chain composability (R7, ADR-0003)

| INV | Enforcement point | Status |
|-----|-------------------|--------|
| INV-22: DSSE/in-toto shared format | `elench-envelope::sign` / `verify` | UNIMPLEMENTED |

## Predicate language (ADR-0004)

| INV | Enforcement point | Status |
|-----|-------------------|--------|
| INV-23: Expressions executable/deterministic/sandboxable | `elench-predicate` (not yet created) — gated by E0 | UNIMPLEMENTED |

## Validator (ADR-0006)

| INV | Enforcement point | Status |
|-----|-------------------|--------|
| INV-24: AGENTS.md rules enforced | `elench-claim::validate_claim` — first milestone | UNIMPLEMENTED |
