# Fidelity Index

Test depth per invariant. All depths are NONE — no code exists yet. E0,
E1, E2 are research experiments, not test depth; they gate whether code
should be written at all.

## Invariants

| INV | Description | Depth | Notes |
|-----|-------------|-------|-------|
| INV-01 | Status changed by appending, not modifying | NONE | No code |
| INV-02 | Prior status remains visible | NONE | No code |
| INV-03 | Claim identity stable across status changes | NONE | No code |
| INV-04 | Status computed by folding, not stored | NONE | No code |
| INV-05 | origin.kind required and structurally distinct | NONE | No code |
| INV-06 | Agents cannot emit harness-observed | NONE | No validator yet (A-A05) |
| INV-07 | Only harness emits verification | NONE | No validator yet (A-A05) |
| INV-08 | Predicate requires executable expression | NONE | No validator yet (A-A05) |
| INV-09 | Annotations never read by policy | NONE | No code |
| INV-10 | dependsOn populated with premises | REMOVED | Downgraded to guideline; empty dependsOn is a warning, not a rejection |
| INV-11 | Failure recorded only when status changed | NONE | No code |
| INV-12 | Agents cannot emit residue-acceptance | NONE | No validator yet (A-A05) |
| INV-13 | Gate evaluable without build capability | NONE | No code |
| INV-14 | Artifact acceptability is live evaluation | NONE | No code |
| INV-15 | Artifact carries (tree, policy), not verdict | NONE | No code; tree is elench OID |
| INV-16 | unevaluated is first-class status | NONE | No code |
| INV-17 | Policies permit bounded unevaluated residue | NONE | No code |
| INV-18 | elench owns content-addressed store | NONE | No code; ADR-0001 |
| INV-19 | Git projection is read-only | NONE | No code; ADR-0002 |
| INV-20 | Git synthesis is deterministic (BC4) | NONE | No code; ADR-0007 |
| INV-21 | Git projection produces no side effects | NONE | No code |
| INV-22 | Agent claims and provenance share DSSE/in-toto | NONE | No code |
| INV-23 | Predicate expressions executable/deterministic/sandboxable | NONE | Gated by E0/ADR-0004 |
| INV-24 | AGENTS.md rules enforced by validator | NONE | First milestone (ADR-0006) |
| INV-25 | Content addressing (SHA-256) | NONE | No code |
| INV-26 | Store is sole source of truth | NONE | No code |
| INV-27 | Git projection is lossy, not authoritative | NONE | No code |
| INV-28 | Claim OID is content hash | NONE | No code |
| INV-29 | dependsOn acyclic | NONE | No code |

28 active invariants (24 original minus INV-10 removed, plus INV-25/26/27/28/29 added).

## Features

| Feature | Scenarios | Depth | Notes |
|---------|-----------|-------|-------|
| claim-emission | 5 | NONE | No code |
| claim-revocation | 4 | NONE | No code |
| origin-typing | 4 | NONE | No code |
| release-gate | 6 | NONE | No code |
| anchor-resolution | 5 | NONE | No code |
| unevaluated-residue | 5 | NONE | No code |
| git-projection | 4 | NONE | No code; ADR-0002/0007 |

## Experiments (research, not test depth)

| Experiment | Gates | Status | Notes |
|------------|-------|--------|-------|
| E0 — Predicate ratio | BC1 | NOT RUN | 20-30 brownfield sessions needed |
| E1 — Anchor survival | BC2 | NOT RUN | Depends on E0 passing |
| E2 — Build reproducibility | BC3 | NOT RUN | Independent, can run in parallel |

## First milestone

The validator (INV-24, ADR-0006) is the first implementation target. A
claim log with unenforced emission rules is not evidence; it is
testimony from the audited party in a structured format. All other
implementation is gated by E0's result.
