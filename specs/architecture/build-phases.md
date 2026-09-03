# Build Phases

Implementation order derived from the dependency graph. Each phase
depends only on earlier phases. All phases (0–5) are **COMPLETE**.
E0, E1, E2 all PASSED.

## Phase 0 — Validator **[COMPLETE]**

**Crate:** `elench-claim` (validation portion only)

The validator enforces the AGENTS.md emission rules (INV-05 through
INV-09, INV-11, INV-12, INV-24; INV-10 is a guideline warning, not a
rejection).

- Claim schema validation (against `schema/claim.schema.json`)
- Origin.kind enforcement (INV-05, INV-06, INV-07, INV-12)
- Predicate enforcement (INV-08)
- dependsOn population warning (guideline, was INV-10 — warning, not rejection)
- Failure recording rule (INV-11)

**Gating:** E0 PASSED (0.72). ADR-0004 ACCEPTED (elench-predicate-v1).

## Phase 1 — Content-addressed store and claim data model **[COMPLETE]**

**Crates:** `elench-store` (full), `elench-claim` (full)

- Content-addressed storage: blobs, trees, claims (ADR-0001)
- Claim types matching `schema/claim.schema.json`
- Status computation by folding (INV-01 through INV-04)
- Blast radius computation (transitive dependsOn closure)
- Append-only enforcement (INV-01)
- No git dependency, no daemon (INV-18)
- Persistent backend: fjall (optional feature, ADR-0008)

**Depends on:** Phase 0 (validator exists and passes).

## Phase 2 — Envelope handling **[COMPLETE]**

**Crate:** `elench-envelope`

- DSSE envelope signing and verification
- in-toto statement handling
- Signer/producer distinction (INV-22)
- Integration with content-addressed storage (store only verified envelopes)

**Depends on:** Phase 1 (claim types and store exist).

## Phase 3 — Release gate **[COMPLETE]**

**Crate:** `elench-gate`

- Four conditions from `docs/release-policy.md`:
  1. No falsified premise (uses blast radius from Phase 1)
  2. Bounded residue with acceptance (INV-17)
  3. Origin floor (uses origin from Phase 1)
  4. Builder agreement (available — E2 PASSED; hermeticity floor enforced)
- Gate evaluation without build capability (INV-13)
- Annotation filtering — annotations never contribute to gate verdict (INV-09)
- Live evaluation, no cached verdict (INV-14, INV-15)

**Depends on:** Phase 1 (claim status, blast radius), Phase 2 (envelope
verification for residue-acceptance records).

## Phase 4 — Git projection **[COMPLETE]**

**Crate:** `elench-projection`

- Deterministic commit synthesis from the claim log (ADR-0002,
  ADR-0007, BC4)
- `git log` shows commits derived from tree-changing claims
- `git blame` maps lines to the claims that introduced them
- Read-only: no write-through-git (INV-19, INV-21)
- Two-party determinism test: same claim log → byte-identical git
  objects (INV-20)
- Projection is lossy (INV-27): claim status, origin, blast radius
  NOT recoverable from git objects

**Depends on:** Phase 1 (store, claim log), Phase 2 (envelope
verification for signed claims).

## Phase 5 — CLI **[COMPLETE]**

**Crate:** `elench` (binary)

- `elench emit` — create, sign, store a claim
- `elench verify` — verify envelope and validate claim
- `elench status` — compute a claim's status
- `elench gate` — evaluate the release gate for a tree
- `elench blast` — compute the blast radius from a claim
- `elench git` — materialize the git projection
- `elench store` — store a blob or tree

**Depends on:** All prior phases.

## Deferred phases

| Phase | Crate | Gated by | Status |
|-------|-------|----------|--------|
| Anchor resolution | `elench-anchor` | E1 (PASSED, strategy=multi) | Deferred — crate not yet created |
| Reconciliation pass | (new) | A-A02 (open question) | Deferred — open question |
