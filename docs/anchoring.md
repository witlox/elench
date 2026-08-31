# Anchoring

**Status: unresolved. Everything else in this repository is contingent on it.**

## The problem

A claim points at code. Code moves. If the pointer rots, then:

- revocation propagates to the wrong lines, and the blast radius is fiction;
- `unevaluated` residue is miscounted in both directions;
- the whole log degrades into per-file gossip.

This is the same problem as making review comments survive a rebase,
which forges have not solved well in twenty years. Assume it is hard.
Do not assume a clever hash fixes it.

In elench, a "tree" is a content-addressed tree state in the store
(ADR-0001), not a git commit. Anchors point at elench tree OIDs. The
git projection (ADR-0002) is irrelevant to anchoring — anchors resolve
in elench-native space.

## Candidates

**Path + line range.** Trivial to compute, rots on the first reformat.
Baseline for E1, not a proposal.

**Symbol identity.** Language-server qualified name. Survives reformatting and
line motion; dies on rename, and rename is the single most common refactor an
agent performs. Requires a language server per language, which is a large
dependency surface for a tool that owns its own store.

**Normalised content digest.** Digest of the anchored span after
whitespace/comment normalisation. Survives motion and rename; dies on any
semantic edit, which is arguably correct — an edited span *should* invalidate
claims about it. The failure is the opposite direction: a claim about an
invariant survives an edit that preserves the invariant, and content digest
kills it anyway. High false-revocation rate.

**Multi-strategy with confidence.** Record all three; resolve by agreement;
report an anchor as `degraded` when strategies disagree rather than silently
picking one. Costs more storage and introduces a fourth status
(`anchor-degraded`) that policy has to handle.

## Rejected

- **Anchoring to the tree only, not to a span.** Removes the problem by
  removing the capability. A claim scoped to a whole tree cannot support
  blast-radius tracing, which is R1's reason to exist.
- **Requiring agents to re-anchor on every tree change.** Puts the audited
  party in charge of whether its own claims still apply. Violates the
  AGENTS.md asymmetry directly.

## Decision procedure

E1 measures survival rate for each strategy over real refactor sequences.
Pre-register the threshold before looking at results.

The honest possibility to hold open: if no strategy survives at an acceptable
rate, the correct response is to narrow the claim granularity — claims about
module-level or interface-level invariants rather than spans — and accept
coarser blast radius. That is a smaller product, and it should be preferred
over a finer product built on anchors that lie.
