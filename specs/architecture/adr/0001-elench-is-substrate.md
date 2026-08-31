# ADR-0001 — elench is the substrate; the claim log is the primary history

**Status:** proposed
**Serves:** R1, R6
**Supersedes:** the original ADR-0001 ("claims as parallel refs")

## Context

elench must record what was checked, to what depth, and what remains
unevaluated — as durable state, replicated alongside the code. The
claim log must support append-only status change without tree mutation
(R1). Git is a projection, not the substrate (R6); claims cannot live
"in refs/claims/ alongside the code" because there is no git repo
underneath — the claim log IS the history.

## Decision

elenench owns its own content-addressed store. The claim log is the
primary history: it records tree state (as content-addressed blobs and
trees), signed assertions about that state, and status changes — all
in one append-only log. There is no separate git repository; there is
no parallel ref namespace. The store is the substrate.

Git objects are synthesized from the claim log on demand by a
projection helper (ADR-0002). Humans interact through git; elench is
invisible to them. The claim log is the single source of truth.

## Rejected alternatives

- **Claims in a parallel git ref namespace (`refs/claims/`).** Requires
  a git repo underneath, making git the substrate and elench a
  sidecar. Reverses R6. The original ADR-0001 chose this; it was
  wrong.
- **External database.** Breaks clone-and-go, needs a service, and
  separates evidence from the thing it is evidence about.
- **Git notes.** Poor merge behaviour, dropped by common workflows.
  Wrong substrate, same problem as parallel refs.

## Consequences

elench must implement its own content-addressed storage (blobs, trees,
commit-like objects). This is more work than reusing git's object
database, but it is the only way to make the claim log primary.

Code and claims cannot drift — there is no separate tree to drift out
from under anchors. The anchor problem (BC2) becomes: do anchors
survive across tree states in the claim log, not across git commits.

Revocation is invisible in the git projection's `git log` — but
`git log` is itself a projection. The authoritative view is
elench-native: blame-to-claim, status-over-time, blast-radius. The git
projection is a compatibility affordance, not the interface being
designed for.
