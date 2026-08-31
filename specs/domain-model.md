# Domain Model

Derived from `docs/problem.md`, `schema/claim.schema.json`, and the ADR
log. Every entity here traces to a requirement; every relationship
traces to a constraint.

## Bounded contexts

| Context | Responsibility | Key entity |
|---------|---------------|------------|
| Claim Emission | Create, sign, and store claims in the store | Claim |
| Claim Evaluation | Compute status by folding the log | ClaimStatus |
| Anchor Resolution | Resolve anchors to code spans within a tree | Anchor |
| Release Gating | Evaluate release policy against claims | Verdict |
| Predicate Evaluation | Execute predicate expressions (GATED by E0) | Expression |
| Git Projection | Synthesize git objects from the claim log | GitCommit |
| elench Store | Content-addressed storage: blobs, trees, claims | Store |
| Envelope Verification | DSSE envelope signing and verification | Envelope |

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
| `anchor` | object | yes | Contains `tree` (elench OID) and `strategy` |
| `evidence` | array | no | Empty for agent-asserted claims by default |
| `dependsOn` | array of claim ids | no | Premises. Transitive closure IS the blast radius. |

### Claim Log (root)

The append-only set of claims that IS the primary history (ADR-0001).
No aggregate owns individual claims; the log is the aggregate that
collects them. Status is computed by folding, never stored.

- Append-only. No record is ever modified or deleted.
- The git projection is derived from the claim log, not the other way
  around (ADR-0002).
- No daemon. Everything is derivable from the store by a client-side
  binary.

### elench Store (root)

The content-addressed substrate (ADR-0001). Holds blobs, trees, and
claims. There is no separate git repository.

- Content-addressed: blobs by content hash, trees by content hash,
  claims by content hash.
- The store IS the history. There is no parallel ref namespace, no
  sidecar database.
- The git projection reads from the store and synthesizes git objects
  on demand (ADR-0002).

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
| `tree` | string (elench tree OID) | yes | Content address of the tree state in the store. NOT a git commit OID. |
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
| `tree` | string | The elench tree OID evaluated. |
| `policy` | string | The policy evaluated against. |

### GitCommit (projection, not stored)

A git commit object synthesized from the claim log by the git
projection (ADR-0002, ADR-0007). Not stored in the elench store —
generated on demand.

| Field | Type | Notes |
|-------|------|-------|
| `oid` | string | Deterministic function of (tree, parents, author, committer, message, timestamps). |
| `tree` | string | elench tree OID, mapped to a git tree OID. |
| `parents` | array of git commit OIDs | Derived from the claim log. |
| `author` | string | Derived from the claim's `producer.id`. |
| `committer` | string | Same as author (no separate committer). |
| `message` | string | Derived from an annotation claim, if present. |

### Store entities

### Blob

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| oid | string `[0-9a-f]{64}` | yes | SHA-256 content hash. Identical to git SHA-256 blob OID. |
| data | bytes | yes | Content. |

### TreeEntry

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| path | string | yes | Path relative to tree root. |
| mode | integer | yes | File mode (e.g., 0o100644 for regular file). |
| blob | string `[0-9a-f]{64}` | yes | SHA-256 content hash of the blob. |

### Tree

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| oid | string `[0-9a-f]{64}` | yes | SHA-256 content hash of the tree (sorted entries, canonical serialization). Identical to git SHA-256 tree OID. |
| entries | array of TreeEntry | yes | Sorted by path. |

## Entity relationships

```
Claim
 ├── 1..* ──→ Claim (via dependsOn: premises)
 ├── 1..* ──→ Claim (via target: falsification/verification/supersession)
 ├── 1..* ──→ Evidence
 └── 1    ──→ Anchor

elench Store
 ├── 0..* ──→ Claim
 └── 0..* ──→ Tree (content-addressed)

GitCommit
 └── projected from ──→ Claim Log + elench Store

Verdict
 └── computed from ──→ Claim Log + Release Policy
```

## Status machine

```
              ┌───────────┐    falsification      ┌──────────┐
   (default)  │unevaluated│ ──────────────────▶   │falsified │
              └───────────┘    supersession       └──────────┘
                    │                                   ▲
          verification                                │
                    ▼                                  │
              ┌───────────┐    falsification          │
              │  passed   │ ──────────────────────▶  │
              └───────────┐    supersession          │
                                                  │
```

Status is computed by folding the log, not stored. A claim's status at
any point in time is a pure function of the claims that target it.
Falsification and supersession both change the target's status to
`falsified`; they differ in intent, not in status effect.
