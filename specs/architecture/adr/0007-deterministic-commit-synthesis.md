# ADR-0007 — Deterministic commit synthesis for the git projection

**Status:** proposed
**Serves:** R6, BC4

## Context

R6 requires the git CLI to work as a projection of the claim log
(ADR-0002). BC4 requires the projection to be deterministic: two
parties with the same claim log produce byte-identical git objects.
The original ADR-0002 identified this as the blocking problem; this
ADR addresses it.

## Decision

Commit OIDs are derived from the claim log, not from wall-clock time
or machine state. Specifically:

1. **Tree OID.** The content address of the tree state, computed
   as a SHA-256 hash of the canonical serialization (sorted entries,
   mode, path, blob OID). This is identical to a git SHA-256 tree OID —
   the projection is a passthrough for trees and blobs. Only commits
   are synthesized.
2. **Commit OID.** Computed from (tree OID, parent commit OIDs,
   author, committer, message, timestamps) — all derived from the
   claim log, not the machine.
3. **Author/committer.** Derived from the claim's `producer.id`. The
   signer (DSSE envelope) is not the author; the producer is. No
   user-configurable `user.name` / `user.email`.
4. **Timestamps.** Derived from the claim's timestamp, not the wall
   clock. Two parties synthesizing the same claim at different times
   get the same commit.
5. **Parent commits.** The commit(s) synthesized from the claim(s)
   that produced the previous tree state(s). Linear history by
   default; merge commits when the claim log records a merge.
6. **Commit granularity.** One commit per tree-changing claim. A
   session that produces N tree changes gets N commits. This preserves
   the blast-radius connection: `git blame` maps to the specific claim
   that introduced the line.

elench uses SHA-256 for all content addressing. Blobs, trees, and
claims are all SHA-256. The git projection produces SHA-256 git objects;
consumers use SHA-256 git repos. No SHA-1 is used anywhere.

## Rejected alternatives

- **One commit per session.** Loses the blast-radius connection.
  `git blame` maps to the session, not the specific claim.
- **Non-deterministic timestamps.** Two parties get different OIDs for
  the same tree state. R6 fails.
- **User-configurable author.** Reintroduces the identity problem the
  system exists to solve. The author field is derived, not chosen.

## Consequences

The git projection is fully deterministic and reproducible. Any party
with the claim log can verify the projection by re-synthesizing.

The commit history is denser than a human would write — one commit
per claim, not one per "logical change." This is correct: the claim
is the unit of change, and denser history means finer-grained
`git blame`.

`git rebase` and `git merge` are read-only operations in the
projection — they produce new tree states that must be written back
to elench as new claims. The projection does not support write-through
(ADR-0002).
