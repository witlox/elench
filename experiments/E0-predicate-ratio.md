# E0 — Predicate ratio

**Gates: BC1. Run before writing implementation code.**

## Question

Of the claims an agent actually generates during real work, what fraction can
be expressed as machine-checkable predicates rather than prose annotations?

Annotations cannot gate. If the ratio is low, `elench` is a search index over
agent regret with a signing layer attached — which may still have some value,
but it is a different and much smaller project, and it should be built
differently.

## Method

1. Take 20–30 completed agent sessions on a real repository. Brownfield, not
   greenfield: the unevaluated fraction is the point.
2. For each session, extract candidate claims by hand or with a second model.
   Extraction method must be recorded; if a model does it, spot-check 20% by
   hand and report disagreement rate. A model extracting claims will invent
   structure that was never in the session, and that inflation is the main
   threat to this measurement.
3. Classify each: `predicate` (an executable expression can be written now,
   by the person doing the classification, in under five minutes) or
   `annotation`.
4. Report: ratio, distribution by session, and the predicate expressions
   themselves — the expressions are as informative as the ratio.

## Pre-registered thresholds

State these before running. Do not adjust after seeing results.

| Ratio | Reading |
|---|---|
| ≥ 0.30 | Proceed as designed. |
| 0.15 – 0.30 | Proceed with reduced scope: gating over a small predicate core, annotations demoted to a search index only. Re-derive requirements before continuing. |
| < 0.15 | Stop. Build the search index if it is worth building; do not build the gate layer. |

These numbers are a judgement call, not a derivation. The reasoning: below
roughly one in six, the predicate set is too sparse to cover a change's
surface, so a passing gate carries almost no information and the human is back
to reading everything. If someone has a better-grounded threshold, replace
these — but replace them *before* running.

## Secondary measurements

Cheap to collect, and each answers something the design currently guesses at.

- **`dependsOn` density.** Mean premises per claim. If near zero, revocation
  cannot propagate and the central capability is unreachable regardless of the
  predicate ratio. Arguably a second stop condition.
- **Falsification rate.** Fraction of claims falsified later in the same
  session. If near zero, revocability is solving a problem that does not
  occur at session scale, and the case rests entirely on cross-session and
  post-release revocation — a weaker and slower-to-validate claim.
- **Harness-derivable fraction.** Of the predicate claims, how many could have
  been emitted by the harness without asking the agent? High is good: it means
  most of the load-bearing record does not depend on the audited party.

## Threat to validity

The sessions were produced by a harness that was not asked to emit claims.
Claims extracted retrospectively may not resemble claims emitted natively —
plausibly they are cleaner, since hindsight removes the dead ends. This biases
the ratio **upward**. Treat E0's result as an optimistic bound and say so in
the writeup.
