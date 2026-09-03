# Failure modes

Known failure scenarios and handling, ordered by severity. P0 kills the
project; P3 is a known limitation.

## P0 — Project-killing

### FM-P0-01: Anchor rot

Anchors no longer identify the intended code after realistic refactoring
(rename, move, reformat, semantic edit, delete). Wrong-resolution
poisons the blast radius silently — revocation targets the wrong code
and the central capability is fiction.

**Gated by:** BC2, E1
**Threshold:** Wrong-resolution > 2% disqualifies a strategy outright.
Correct-resolution ≥ 85% with wrong-resolution ≤ 2%: usable. Nothing
clears both: fall back to coarser granularity (module/interface claims,
not spans). This is an acceptable outcome, not a failure.
**Handling:** Pre-registered in experiments/E1-anchor-survival.md. Run
E1 after E0 passes. If no strategy survives, narrow claim granularity
and accept coarser blast radius. The smaller honest product beats a
larger one built on anchors that lie.
**Recovery:** Not applicable. If anchors lie, the design is dead at the
current granularity.

### FM-P0-02: Predicate ratio too low

Most claims are annotations, not predicates. The gate carries almost no
information and the human is back to reading everything. elench degrades
to an expensive search index over agent regret with a signing layer
attached.

**Gated by:** BC1, E0
**Threshold:** Ratio ≥ 0.30 proceed. 0.15–0.30 proceed with reduced
scope. < 0.15 stop — do not build the gate layer.
**Handling:** Pre-registered in experiments/E0-predicate-ratio.md. Run
E0 first, before any implementation. If < 0.15, build only the search
index if it is worth building, differently.
**Recovery:** Not applicable. If the ratio is too low, the gate layer
does not get built.

## P1 — Severe, blocks core capability

### FM-P1-01: Build non-reproducibility

The target build cannot be made bit-reproducible across independent
machines. K-of-N builder agreement is meaningless. The release gate
degrades to a single signature, and R3's independent evaluation buys
nothing at the artifact layer.

**Gated by:** BC3, E2
**Threshold:** All divergences cheap-to-fix: proceed with K-of-N. Any
structural divergence: K-of-N unavailable. Release policy must be
rewritten for a single trusted builder before implementation.
**Handling:** Pre-registered in experiments/E2-build-reproducibility.md.
Can run in parallel with E0/E1. Reuse prior art (guix challenge,
rebuilderd) rather than measuring from scratch.
**Recovery:** If structural divergence exists, rewrite
docs/release-policy.md for single-builder. This is a smaller product.

### FM-P1-02: Claim log grows without bound

~~No pruning story exists. On an active repository, the log accumulates
without limit. Compaction may violate R1 (append-only, no
overwriting).~~

**RESOLVED.** Compaction is a manual, destructive operator action
(A-A10). `elench log <claims.json>` provides statistics (total, kind
distribution, status distribution, noise ratio, dependsOn density).
`elench compact <claims.json> --before <ts>` retires all claims
before the cut-off, freezing their statuses as a snapshot. The
compaction record carries frozen statuses forward. Active claims
continue to be revocable (R1 preserved for active claims,
deliberately violated for retired ones).

### FM-P1-03: Non-deterministic git projection

Two parties with the same claim log produce different git objects.
`git log` disagrees. R6 fails.

**Gated by:** BC4, ADR-0007
**Threshold:** Git object synthesis is deterministic by construction
(`git hash-object`, `git commit-tree`, `git mktree` are deterministic).
The risk is in the derivation mapping (which claim becomes which
commit), not in git's object format.
**Handling:** ADR-0007 specifies deterministic OID derivation,
author/committer strings, timestamps, and tree ordering. Tested by
synthesizing from the same claim log on two machines and comparing
output.
**Recovery:** If the derivation mapping is non-deterministic, fix the
mapping. Git's object format is not the problem.

## P2 — Moderate, degrades quality

### FM-P2-01: Residue acceptance rubber stamp

~~The human key signing over unevaluated gaps becomes a formality rather
than a genuine acceptance. The R5 terminator fails — not because it is
wrong, but because the human rubber-stamps it.~~

**RESOLVED.** Review mode (A-A11) forces the human to look before
stamping. `elench review <tree> <claims.json>` shows all
unevaluated claims with their content (form, language, source,
producer). The human must name each gap before `elench accept`
issues a residue-acceptance. This adds real friction without
multi-party complexity.

### FM-P2-02: Contradictory predicates, neither falsified

~~Two agents assert contradictory predicates about the same span. Neither
is falsified. The tree's status is undefined.~~

**RESOLVED.** Last-writer-wins (by timestamp). The conflict is
detected and reported by `elench conflicts <tree> <claims.json>`
(A-A12). The gate evaluates against the winning predicate and
includes the conflict as a warning (not a failure — the winning
predicate still gates). Partial block: only the affected path (same
anchor) is blocked; other paths evaluate normally. The human is
expected to resolve: falsify one or both predicates.

### FM-P2-03: Reconciliation pass not triggered

~~A tree change lands that invalidates claims (moves code out from under
anchors). Nothing forces the claim log to notice. The log and the tree
drift apart silently.~~

**RESOLVED.** `elench-anchor::reconcile(tree, log)` detects claims
whose anchors no longer resolve after a tree change. Reports
affected claims as `DriftedClaim` (with path, symbol, resolution
result). Read-only: does NOT auto-fix. The human (or agent) must
re-anchor or falsify. CLI: `elench reconcile <tree> <claims.json>`
(future — currently library-only).

## P3 — Low, known limitation

### FM-P3-01: Consumers who never re-check are unprotected

An artifact's acceptability is a live evaluation (R4). Consumers who
never re-check are unprotected after a post-release falsification. There
is no push path.

**Gated by:** None (inherent to certificate-revocation shape)
**Handling:** Acknowledged in docs/release-policy.md. Do not pretend
otherwise in the docs. A push path may be added later but is not
part of the core design.
**Recovery:** Out of scope. Consumers must poll.

### FM-P3-02: Emergency override unspecified

Every real system grows one. Leaving it unspecified means it will be
added badly under pressure.

**Gated by:** None (deliberately deferred)
**Handling:** Flagged in docs/release-policy.md §Deliberately not
specified. Not a design gap — a conscious deferral.
**Recovery:** Specify when the need becomes concrete, not before.

### FM-P3-03: Git write-through unsupported

A user who runs `git commit` in a projected repository gets an error or
a no-op. This is the price of making the claim log primary (ADR-0002).

**Gated by:** R6 (write path goes through elench, never git)
**Handling:** Documented. The git projection is read-only. Users who
want to write must use `elench` (or a future write-back helper that
converts git commits into elench claims).
**Recovery:** A write-back helper is possible but not scoped. It would
convert a git commit into one or more elench claims, losing any
information git cannot represent.
