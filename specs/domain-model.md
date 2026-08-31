# Domain Model

Derived from `docs/problem.md`, `schema/claim.schema.json`, and the ADR
log. Every entity here traces to a requirement; every relationship
traces to a constraint.

## Bounded contexts

| Context | Responsibility | Key entity |
|---------|---------------|------------|
| Claim Emission | Create, sign, and store claims in refs/ | Claim |
| Claim Evaluation | Compute status by folding the log | ClaimStatus |
| Anchor Resolution | Resolve anchors to code spans within a tree | Anchor |
| Release Gating | Evaluate release policy against claims | Verdict |
| Predicate Evaluation | Execute predicate expressions (GATED by E0) | Expression |

## Aggregates

### Claim (root)

A signed assertion about a tree. The aggregate root of the claim log.
Identity is the content address at creation; stable across status
changes.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `id` | string `^cl_[0-9a-f]{64}$` | yes | Content address, never reassigned |
| `kind` | enum: assertion, falsification, verification, supersession, residue-acceptance | yes | falsification/verification/supersession require `target` |
| `target` | array of claim ids | no | Claims this record acts upon |
| `assertion` | object | yes | Contains `form` (predicate/annotation), optional `expression`, optional `text` |
| `origin` | object | yes | Contains `kind` and `producer` |
| `anchor` | object | yes | Contains `tree` and `strategy` |
| `evidence` | array | no | Empty for agent-asserted claims by default |
| `dependsOn` | array of claim ids | no | Premises. Transitive closure IS the blast radius. |

### Claim Log (root)

The append-only set of claims stored in `refs/claims/<type>/<id>`. No
aggregate owns individual claims; the log is the aggregate that
collects them. Status is computed by folding, never stored.

- Append-only. No record is ever modified or deleted.
- Replicated by git transport alongside the code.
- Invisible to tooling that does not know about it (R6).

## Value objects

### Assertion

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `form` | enum: predicate, annotation | yes | predicate can gate; annotation cannot |
| `expression` | object: { language, source, digest? } | required when form=predicate | executable, deterministic, sandboxable |
| `text` | string, max 2000 chars | no | Human-readable. Never load-bearing. |

### Origin

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `kind` | enum: harness-observed, agent-asserted, human-asserted | yes | R2. Agents cannot emit harness-observed. |
| `producer` | object: { id, sessionId?, hermeticity? } | yes | Distinct from signer (envelope). |

### Anchor

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `tree` | string (commit OID) | yes | Commit the anchor was resolved against. |
| `strategy` | enum: path-range, symbol, content-digest, multi | yes | UNRESOLVED — E1 decides. |
| `path` | string | no | For path-range strategy. |
| `range` | [int, int] | no | For path-range strategy. |
| `symbol` | string | no | For symbol strategy. |
| `contentDigest` | string | no | For content-digest strategy. |

### Evidence

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `kind` | enum: process-exit, test-report, artifact-digest, external-attestation | yes | |
| `digest` | string | no | |
| `exitCode` | integer | no | For process-exit. |
| `uri` | string | no | |

### ClaimStatus (computed, not stored)

| Value | Meaning |
|-------|---------|
| `unevaluated` | No verification or falsification record targets this claim. Default. |
| `passed` | A verification record exists and no falsification has invalidated it. |
| `falsified` | A falsification record has changed this claim's status. |

### Verdict

The result of evaluating a release policy against a claim log for a
given tree. Not stored — computed at evaluation time.

| Field | Type | Notes |
|-------|------|-------|
| `result` | enum: pass, fail | |
| `reasons` | array of strings | Which conditions failed and why. |
| `tree` | string | The tree evaluated. |
| `policy` | string | The policy evaluated against. |

## Entity relationships

```
Claim
 ├── 1..* ──→ Claim (via dependsOn: premises)
 ├── 1..* ──→ Claim (via target: falsification/verification/supersession)
 ├── 1..* ──→ Evidence
 └── 1    ──→ Anchor

Claim Log
 └── 0..* ──→ Claim

Verdict
 └── computed from ──→ Claim Log + Release Policy
```

## Status machine

```
                    ┌─────────────────────────────────────┐
                    │                                     │
                    ▼                                     │
              ┌───────────┐    falsification      ┌──────────┐
   (default)  │unevaluated│ ──────────────────▶   │falsified │
              └───────────┘                       └──────────┘
                    │                                   ▲
          verification                                │
                    ▼                                   │
              ┌───────────┐    falsification           │
              │  passed   │ ──────────────────────▶   │
              └───────────┘                             │
                    │                                   │
                  supersession                          │
                    ▼                                   │
              ┌───────────┐    falsification           │
              │superseded │ ──────────────────────▶   │
              └───────────┘                             │
```

Status is computed by folding the log, not stored. A claim's status at
any point in time is a pure function of the claims that target it.
