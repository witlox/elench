# ADR-0002 — Git is a deterministic projection of the claim log

**Status:** proposed
**Serves:** R6
**Supersedes:** the original ADR-0002 ("no synthesised git history")

## Context

R6 requires that the git CLI works with no changes and no awareness of
elench. Since elench is the substrate (ADR-0001), git objects must be
synthesized from the claim log. The original ADR-0002 rejected this
approach citing deterministic synthesis as too hard; that rejection
was based on the wrong substrate assumption. Synthesis is now a
requirement, and its determinism is a binding constraint (BC4).

## Decision

elench ships a projection helper (`git-remote-elench` or a FUSE
filesystem) that synthesizes git-compatible objects from the claim log.
The projection is read-only — humans use git to read, but writes go
through elench. Synthesis is deterministic: given the same claim log,
any party produces byte-identical git objects (BC4).

Commit OIDs are derived from the claims that produced the tree change,
not from wall-clock timestamps. Author and committer strings are
derived from claim producer identity. Tree ordering is deterministic.

## Rejected alternatives

- **No projection — elench-native CLI only.** Guarantees no adoption.
  The git CLI is the universal interface to version control; not
  providing it makes elench a curiosity.
- **Bidirectional git — write through git, read through elench.**
  Requires bidirectional translation, which means the git view can
  diverge from the claim log. The projection must be read-only.
- **Non-deterministic synthesis.** Two parties with the same claim log
  get different git objects, different OIDs, different `git log`.
  R6 fails. Rejected by BC4.

## Consequences

The projection helper is a new component with its own test surface.
Deterministic synthesis must be verified (BC4 is prior art, but the
derivation mapping — which claim becomes which commit — is novel and
needs its own test).

The projection is lossy: git commits have no concept of claim status,
origin, or blast radius. `git log` shows a synthesized history; the
authoritative view is elench-native. This is the same lossiness as
before, but in the other direction — git is the view, not the
substrate.

Write-through-git is not supported. A user who runs `git commit` gets
an error or a no-op, depending on the projection implementation. This
is a UX cost; it is the price of making the claim log primary.
