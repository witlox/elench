# ADR-0002 — No synthesised git history

**Status:** proposed
**Serves:** R6

## Context
Human tooling must work unmodified. One route is to generate a git view from
the claim log; the other is to leave git alone and sit beside it.

## Decision
Git remains the primary write interface for code. Compatibility is achieved
by *not interfering*, plus optionally a `git-remote-` helper for transport if
a non-git backing store is ever introduced. No commit synthesis.

## Rejected alternatives
- **Claims primary, git generated.** Determinism burden (pinned timestamps,
  author strings, tree ordering) plus the spectator problem above.
- **Custom review UI as the only interface.** Guarantees no adoption.

## Consequences
Two writers to one repository with no shared transaction. A commit can land
that invalidates claims, with nothing forcing the claim log to notice. A
reconciliation pass is required and does not exist yet.

The genuinely useful human view is not `git log` but blame-to-claim: this
line exists because of claim C, status unevaluated, falsified once and
re-asserted. Reachable *from* git coordinates, not expressible *in* git.
That view is a separate tool and is not scoped here.
