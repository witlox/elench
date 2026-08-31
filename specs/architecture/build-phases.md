# Build Phases

Implementation order derived from the dependency graph. Each phase
depends only on earlier phases. No phase begins until E0 has run (per
README and ADR-0006), except Phase 0 (the validator), which is the
first milestone and is gated by ADR-0004 (predicate language), which is
gated by E0.

## Phase 0 — Validator (gated by E0 → ADR-0004)

**Crate:** `elench-claim` (validation portion only)

The validator enforces the AGENTS.md emission rules (INV-05 through
INV-12, INV-23). A claim log with unenforced rules is testimony from
the audited party in a structured format — not evidence.

- Claim schema validation (against `schema/claim.schema.json`)
- Origin.kind enforcement (INV-05, INV-06, INV-07, INV-12)
- Predicate/annotation enforcement (INV-08, INV-09)
- dependsOn population check (INV-10)
- Failure recording rule (INV-11)

**Gating:** E0 must run first. The predicate language (ADR-0004) must
be decided before `expression.language` validation can be implemented.
If E0's ratio is < 0.15, Phase 0 is the last phase — build only the
search index.

## Phase 1 — Claim data model and store

**Crates:** `elench-claim` (full), `elench-store`

- Claim types matching `schema/claim.schema.json`
- Status computation by folding (INV-01 through INV-04)
- Blast radius computation (transitive dependsOn closure)
- Git ref namespace operations: store, read all, read for tree
- Append-only enforcement (INV-01, INV-19)

**Depends on:** Phase 0 (validator exists and passes).

## Phase 2 — Envelope handling

**Crate:** `elench-envelope`

- DSSE envelope signing and verification
- in-toto statement handling
- Signer/producer distinction (INV-21)
- Integration with claim storage (store only verified envelopes)

**Depends on:** Phase 1 (claim types exist).

## Phase 3 — Release gate

**Crate:** `elench-gate`

- Four conditions from `docs/release-policy.md`:
  1. No falsified premise (uses blast radius from Phase 1)
  2. Bounded residue with acceptance (INV-17)
  3. Origin floor (uses origin from Phase 1)
  4. Builder agreement (UNAVAILABLE unless E2 passes)
- Gate evaluation without build capability (INV-13)
- Live evaluation, no cached verdict (INV-14, INV-15)

**Depends on:** Phase 1 (claim status, blast radius), Phase 2 (envelope
verification for residue-acceptance records).

## Phase 4 — CLI

**Crate:** `elench` (binary)

- `elench emit` — create, sign, store a claim
- `elench verify` — verify envelope and validate claim
- `elench status` — compute a claim's status
- `elench gate` — evaluate the release gate for a tree
- `elench blast` — compute the blast radius from a claim

**Depends on:** All prior phases.

## Deferred phases (gated by experiments)

| Phase | Crate | Gated by |
|-------|-------|----------|
| Anchor resolution | `elench-anchor` | E1 (anchor survival) |
| Predicate evaluation | `elench-predicate` | E0 (ratio) + ADR-0004 (language) |
| Builder agreement | (in `elench-gate`) | E2 (build reproducibility) |
| Reconciliation pass | (new) | A-A02 (open question) |
