# Cross-Context Interactions

Integration points between bounded contexts. Each interaction has a
direction, a contract, and a failure mode.

## Contexts

| Context | Owns | Consumes |
|---------|------|----------|
| Claim Emission | Claim creation, signing, storage | Envelope Verification (before accepting) |
| Claim Evaluation | Status computation (log folding) | Claim Store (reads log) |
| Anchor Resolution | Anchor → code span resolution | Claim Evaluation (for blast radius) |
| Release Gating | Verdict computation | Claim Evaluation (uses status), Predicate Evaluation |
| Predicate Evaluation | Expression execution | — (gated by E0/ADR-0004) |

## Interactions

### 1. Claim Emission → Claim Store

**Direction:** Write
**Contract:** A signed DSSE envelope containing an in-toto statement
whose predicate matches `schema/claim.schema.json` is written to
`refs/claims/<type>/<id>`.
**Failure:** Invalid envelope → rejected before write. Valid envelope,
invalid claim → rejected by validator (once implemented, ADR-0006).
**Status:** No code. Contract is `schema/claim.schema.json` (DRAFT).

### 2. Claim Evaluation ← Claim Store

**Direction:** Read
**Contract:** The evaluator reads all claims in `refs/claims/` and
folds them to compute each claim's status (unevaluated / passed /
falsified). The fold is a pure function of the log.
**Failure:** Corrupt or missing ref → status is "unevaluated" for all
claims that cannot be read. Silent corruption → wrong status. (No
reconciliation pass exists yet, A-A02.)
**Status:** No code. Fold semantics are in `specs/domain-model.md`.

### 3. Release Gating ← Claim Evaluation

**Direction:** Read
**Contract:** The gate receives a tree T and a policy P, queries
Claim Evaluation for the status of all claims anchored to T, and
evaluates the four conditions from `docs/release-policy.md`:
  1. No falsified premise in transitive dependsOn closure
  2. Bounded residue (unevaluated within P's allowance, excess covered
     by residue-acceptance)
  3. Origin floor (load-bearing claims are harness-observed)
  4. Builder agreement (K independent producers — unavailable unless E2)
**Failure:** Any condition fails → gate fails with named reason. The
verdict is not stored; it is recomputed on demand.
**Status:** No code. Policy shape is in `docs/release-policy.md`.

### 4. Anchor Resolution ← Claim Evaluation

**Direction:** Read (for blast radius), Write (anchor resolution result)
**Contract:** When a claim is falsified, the blast radius is the
transitive dependsOn closure. Each claim in the closure has an anchor
that must be resolved to verify it still points at the intended code.
If an anchor is degraded (multi-strategy disagreement), the blast
radius report marks it.
**Failure:** Wrong-resolution (anchor points at wrong code) → blast
radius is fiction, falsification targets the wrong lines. This is
FM-P0-01.
**Status:** No code. Anchor strategies are in `docs/anchoring.md`
(UNRESOLVED — E1 gates).

### 5. Envelope Verification → Claim Emission

**Direction:** Read (before accepting a claim)
**Contract:** Before a claim is stored, its DSSE envelope is verified:
signature is valid, signer is known, and the in-toto statement's
predicateType is recognised. The signer (envelope) and producer
(claim payload) are distinct; both are recorded.
**Failure:** Invalid signature → rejected. Unknown signer → rejected
or flagged per policy. Valid signature, invalid claim → rejected by
validator.
**Status:** No code. Envelope format is in ADR-0003.

### 6. Predicate Evaluation ← Release Gating

**Direction:** Read
**Contract:** The release gate is itself a predicate. If the predicate
language is the same as claim predicates (ADR-0004 undecided), the
gate reuses the same evaluation engine. If not, a separate policy
language exists.
**Failure:** Predicate evaluation fails → gate fails (fail-closed).
Non-deterministic predicate → two evaluators reach different verdicts,
violating R3.
**Status:** No code. Gated by E0 (predicate ratio) and ADR-0004
(language undecided).

## Open cross-context issues

1. **Reconciliation pass** (Claim Store ↔ Claim Evaluation): A commit
   lands that moves code out from under anchors. Nothing forces the
   claim log to notice. A reconciliation pass is required and does not
   exist yet (A-A02, FM-P2-03).

2. **Claim log convergence** (Claim Store): Does the log grow without
   bound on an active repository? No pruning story exists. Compaction
   may violate R1 (A-U01, FM-P1-02).

3. **Contradictory predicates** (Predicate Evaluation ↔ Release Gating):
   Two agents assert contradictory predicates and neither is
   falsified. The tree's status is undefined (A-U03, FM-P2-02).
