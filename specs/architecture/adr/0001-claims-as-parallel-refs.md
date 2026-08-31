# ADR-0001 — Claims live in a parallel ref namespace

**Status:** proposed
**Serves:** R1, R6

## Context
The claim log must be replicated with the code, survive clone, and be
invisible to tooling that does not know about it (R6). It must also support
append-only status change without tree mutation (R1).

## Decision
Claims are git objects under `refs/claims/<type>/<id>`, in the same object
database as the code. Fetched by whoever wants them, ignored by everyone
else. Precedent: Radicle stores COBs under `refs/cobs/<typename>/<id>` and
gets replication, integrity, and offline availability from git for free.

## Rejected alternatives
- **Synthesise git commits from the claim log (projection).** Requires
  deterministic commit synthesis or two people's views disagree. Worse, git
  is a *write* interface — a read-only projection makes the human a
  spectator, inverting the property the system exists to protect.
- **Git notes.** Poor merge behaviour, dropped by common workflows.
- **Sidecar files in the tree.** Mutating the tree to record a status change
  violates R1 directly and pollutes diffs.
- **External database.** Breaks clone-and-go, needs a service, and separates
  evidence from the thing it is evidence about.

## Consequences
Code and claims can drift — a rebase moves code out from under anchors. This
is `docs/anchoring.md` arriving with a bill attached; the parallel-namespace
choice does not cause it but does not help either.

Revocation changes no tree, so it is **invisible in `git log`**. Any git-side
view is lossy in exactly the dimension carrying the signal. Accept this: the
git view is a migration affordance, not the interface being designed for.
