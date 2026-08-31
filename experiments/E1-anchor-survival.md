# E1 — Anchor survival

**Gates: BC2. Run after E0 passes; results decide `schema/claim.schema.json`'s
`anchor` object, which is currently a placeholder.**

## Question

For each anchoring strategy in `docs/anchoring.md`, what fraction of anchors
still identify the intended code after a realistic refactor sequence?

## Method

1. Pick 3–5 repositories with real history. Include at least one with a
   large mechanical reformat in its past (a formatter adoption commit) — that
   is the adversarial case and it is common.
2. At tree T0, place 100+ synthetic anchors across representative code:
   function bodies, interface definitions, config constants, test assertions.
3. Replay history forward N tree states. For each anchor and each strategy,
   record: resolved correctly / resolved to wrong code / failed to resolve.
4. **Wrong-resolution is the outcome that matters.** Failure to resolve is
   loud and recoverable. Silent misresolution poisons the blast radius and is
   the one that must be near zero.

## Pre-registered thresholds

Per strategy, over the full replay:

- Wrong-resolution rate **> 2%** disqualifies the strategy outright,
  regardless of how good its correct-resolution rate is.
- Correct-resolution **≥ 85%** with wrong-resolution **≤ 2%**: usable.
- Nothing clears both: fall back to coarser granularity per
  `docs/anchoring.md` §Decision procedure. This is an acceptable outcome, not
  a failure — a smaller honest product beats a larger one built on anchors
  that lie.

## Report

Breakdown by refactor class (rename, move, reformat, semantic edit, delete),
not just an aggregate. The aggregate will hide the case that kills you.
