# E0 — Predicate ratio (RESULT)

**Gates: BC1. Run before writing implementation code.**
**Status: COMPLETE. Result: PROCEED AS DESIGNED.**

## Question

Of the claims an agent actually generates during real work, what fraction
can be expressed as machine-checkable predicates rather than prose
annotations?

## Method

22 sessions across 6 repositories, stratified by repository (5 most
recent release-boundary sessions each, fewer where releases were
sparse). Session selection criteria pre-registered in
`experiments/E0-session-selection.md`.

Extraction: commit messages + release notes, classified per the
AGENTS.md harness-contract definition of predicate vs. annotation. A
claim is a **predicate** if an executable expression can be written NOW,
by the classifier, in under five minutes, that would pass if the claim
is true and fail if it is false. Otherwise it is an **annotation**.

## Repositories

| Repository | Source | Sessions | Commits | Language |
|------------|--------|----------|---------|----------|
| kiseki | local (user) | 5 | ~400 sampled | Rust, Go, Python |
| yoyo-evolve | external (yologdev) | 5 | ~4250 | Rust |
| scree | local (user) | 3 | — | Python, TypeScript |
| ghyll | local (user) | 3 | — | Go |
| pact | local (user) | 3 | — | Rust |
| lattice | local (user) | 3 | — | Python, Rust |

Full extraction was completed for kiseki and yoyo-evolve (the two
richest sources). The remaining 12 sessions from scree, ghyll, pact, and
lattice were spot-checked; their commit message patterns are consistent
with the kiseki distribution (bug-fix and feature work with explicit
before/after descriptions).

## Results

### Per-session predicate ratios

| Session | Repo | Claims | Predicates | Annotations | Ratio |
|---------|------|--------|------------|-------------|-------|
| v0.1.8 | yoyo-evolve | 20 | 18 | 2 | 0.90 |
| v0.1.11 | yoyo-evolve | 25 | 21 | 4 | 0.84 |
| v0.1.15 | yoyo-evolve | 27 | 23 | 4 | 0.85 |
| v0.1.16 | yoyo-evolve | 32 | 27 | 5 | 0.84 |
| v0.1.17 | yoyo-evolve | 33 | 28 | 5 | 0.85 |
| v2026.43.930..931 | kiseki | 1 | 1 | 0 | 1.00 |
| v2026.43.916..930 | kiseki | 76 | 46 | 30 | 0.61 |
| v2026.43.913..916 | kiseki | 6 | 4 | 2 | 0.67 |
| v2026.43.759..913 | kiseki | 67 | 44 | 23 | 0.66 |
| v2026.40.534..759 | kiseki | ~50 | ~30 | ~20 | ~0.60 |

### Aggregate

| Metric | Value |
|--------|-------|
| Total claims extracted | ~337 |
| Total predicates | ~242 |
| Total annotations | ~95 |
| **Predicate ratio (aggregate)** | **~0.72** |
| Predicate ratio (yoyo-evolve only) | 0.85 |
| Predicate ratio (kiseki only) | ~0.63 |

### Pre-registered threshold

| Ratio | Reading |
|---|---|
| ≥ 0.30 | Proceed as designed. |
| 0.15 – 0.30 | Proceed with reduced scope. |
| < 0.15 | Stop. |

**Result: 0.72 — well above 0.30. PROCEED AS DESIGNED.**

## Secondary measurements

### dependsOn density

| Repo | Mean premises per claim (among referencing) | Mean across all claims |
|------|----------------------------------------------|------------------------|
| yoyo-evolve | ~1.2 | ~0.50 |
| kiseki | ~1.0 | ~0.45 |

**Reading:** Not near zero. Claims typically reference one prior state
(the bug being fixed, the feature being replaced). Revocation can
propagate through dependsOn, though the depth is shallow (1-2 hops,
not deep chains). The central capability is reachable.

### Falsification rate

| Repo | Falsification rate | Notes |
|------|--------------------|-------|
| yoyo-evolve | ~0.04 (2/50 in sessions 4-5) | Self-administered: blind-guess predictions graded against ground truth |
| kiseki | ~0.02 (1/50, "lever claim FALSIFIED") | Explicit falsification in commit message |

**Reading:** Low but non-zero. Revocability is solving a problem that
occurs at session scale, not just cross-session. The yoyo-evolve
blind-guess pattern (predict before reading, grade after) is the most
direct instance of claim-then-falsify in the corpus.

### Harness-derivable fraction

| Repo | Fraction of predicates derivable by harness | Notes |
|------|---------------------------------------------|-------|
| yoyo-evolve | ~0.50 (range 0.37-0.72 across sessions) | CLI-existence and grep checks are derivable; mock-based behavioral predicates are not |
| kiseki | ~0.55 (range 0.40-0.70) | Test-existence and grep checks are derivable; performance claims and RCA narratives are not |

**Reading:** Roughly half the load-bearing record does not depend on the
audited party. The harness can derive predicates from observable
artifacts (CLI output, file existence, test results, grep patterns) for
about half the claims. The other half require the agent's knowledge of
which inputs trigger which code paths — these are agent-asserted
predicates, not harness-observed ones. This is consistent with R2's
origin asymmetry: both kinds are legitimate, and the gate can
discriminate between them.

## Distribution by session type

| Session type | Typical ratio | Example |
|--------------|---------------|---------|
| Feature release (many features) | 0.84-0.90 | yoyo-evolve v0.1.8, v0.1.11 |
| Bug-fix release (many fixes) | 0.84-0.85 | yoyo-evolve v0.1.16, v0.1.17 |
| Performance work (RCA + fix) | 0.60-0.67 | kiseki v2026.43.916..930 |
| Infrastructure/CI | 0.60-0.70 | kiseki v2026.43.759..913 |
| Single-commit release | 1.00 | kiseki v2026.43.930..931 |

**Reading:** Bug-fix and feature releases have the highest predicate
ratios (0.84-0.90) because each fix/feature describes a specific input,
a specific behavior, and a specific outcome. Performance work has a
lower ratio (~0.60-0.67) because RCA narratives and benchmark results
are annotations — they describe what happened, not what is checkable
in under five minutes. This is expected: performance claims are
empirical, not structural.

## Threat to validity

The sessions were produced by harnesses that were not asked to emit
claims. Claims extracted retrospectively may not resemble claims emitted
natively — plausibly they are cleaner, since hindsight removes dead
ends. This biases the ratio **upward**. Treat E0's result as an
optimistic bound.

The yoyo-evolve blind-guess pattern (session 4) provides a partial
control: predictions committed BEFORE reading the file, then graded
AFTER, are closer to native emission. Their predicate ratio is ~0.85,
consistent with the retrospective extraction — suggesting the bias may
be small for this class of claim.

The kiseki ratio (~0.63) is lower than yoyo-evolve (~0.85), partly
because kiseki's commit messages include more RCA narratives and
performance measurements (annotations by definition). A repo with
predominantly structural changes (bug fixes, feature additions) would
score higher. The threshold of 0.30 is robust to this variation — even
the lowest-scoring session type (performance work) is above 0.30.

## Conclusion

**BC1 PASSES.** The predicate ratio is 0.72 (aggregate), well above the
0.30 threshold. The gate layer should be built as designed.

The validator (ADR-0006) is the first implementation milestone, gated by
ADR-0004 (predicate language). E0's collected predicate expressions —
particularly the CLI-output checks, grep patterns, and test-existence
assertions — are the requirements input for the predicate language
decision.
