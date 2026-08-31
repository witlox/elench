# Assumptions

Three categories: **validated** (backed by evidence or prior art),
**accepted** (acknowledged risk, no mitigation yet), and **unknown**
(needs investigation, may invalidate the design).

## Validated

**A-V01.** Reproducible builds are achievable with existing tooling
(guix challenge, rebuilderd). E2 confirms this is not novel — the
contribution here is the claim log, not the rebuild comparison.
*Source:* experiments/E2-build-reproducibility.md

**A-V02.** Git ref namespaces support parallel storage without
affecting human tooling. Radicle's COBs under `refs/cobs/` demonstrate
replication, integrity, and offline availability from git for free.
*Source:* ADR-0001

**A-V03.** DSSE + in-toto provide the signer/producer distinction
needed for R2. SLSA's provenance predicate already separates builder
and signer identity and does not assume a human signer.
*Source:* ADR-0003

## Accepted

**A-A01.** Revocation is invisible in `git log`. Any git-side view is
lossy in exactly the dimension carrying the signal. This is a migration
affordance, not the interface being designed for.
*Source:* ADR-0001 consequences

**A-A02.** Code and claims can drift — a rebase moves code out from
under anchors. A reconciliation pass is required and does not exist yet.
*Source:* ADR-0002 consequences

**A-A03.** The predicate expression language is undecided. Deciding
before E0 is designing against a guess. E0's collected predicate
expressions are the requirements input for this decision.
*Source:* ADR-0004

**A-A04.** The implementation substrate is undecided. The forcing
requirements are: runs inside an existing agent sandbox, no daemon,
deterministic evaluation, small enough to audit. The decision goes
here before any code is written.
*Source:* ADR-0005

**A-A05.** AGENTS.md rules are unimplemented gates. Until a validator
exists, the rules are prose, and prose does not gate. The first
implementation milestone is the validator, not the CLI.
*Source:* ADR-0006

**A-A06.** E0's result is an optimistic bound. Sessions produced by a
harness not asked to emit claims, extracted retrospectively, are
plausibly cleaner than natively emitted ones — hindsight removes dead
ends, biasing the ratio upward.
*Source:* experiments/E0-predicate-ratio.md §Threat to validity

**A-A07.** Emergency override is unspecified. Every real system grows
one; specifying it now would be guessing, and leaving it unspecified
means it will be added badly under pressure. Flagged, not resolved.
*Source:* docs/release-policy.md §Deliberately not specified

## Unknown

**A-U01.** Does the claim log converge, or grow without bound on an
active repository? No pruning story exists. Compaction may violate R1.
*Source:* docs/problem.md §Open questions

**A-U02.** Who signs the residue acceptance under R5, and what stops
it becoming a rubber stamp? This is the human-in-the-loop reappearing
at the release boundary. It is deliberate, but it is also the obvious
failure point.
*Source:* docs/problem.md §Open questions

**A-U03.** If two agents assert contradictory predicates and neither
is falsified, what is the tree's status? Currently undefined.
*Source:* docs/problem.md §Open questions

**A-U04.** What fraction of claims are predicates vs annotations?
(BC1/E0). If the ratio is < 0.15, elench is a search index over agent
regret, not a gate. The threshold is pre-registered; the measurement
has not been run.
*Source:* experiments/E0-predicate-ratio.md

**A-U05.** What fraction of anchors survive realistic refactor
sequences? (BC2/E1). If no strategy clears wrong-resolution ≤ 2% and
correct-resolution ≥ 85%, the claim granularity must narrow and the
product is smaller than designed.
*Source:* experiments/E1-anchor-survival.md

**A-U06.** Can the target build be made bit-reproducible at acceptable
cost? (BC3/E2). If structural divergence exists, K-of-N is unavailable
and the release gate degrades to a single trusted builder.
*Source:* experiments/E2-build-reproducibility.md

**A-U07.** What is the `dependsOn` density in practice? If near zero,
revocation cannot propagate and the central capability is unreachable
regardless of the predicate ratio. Arguably a second stop condition.
*Source:* experiments/E0-predicate-ratio.md §Secondary measurements
