# Problem statement

Derived from the specific situation below. Nothing here is inherited
from prior designs. Where a familiar pattern appears (append-only logs,
signed envelopes, content addressing) it is because a requirement below
forces it, and the forcing requirement is named. If a requirement is
removed, re-derive rather than keep the pattern.

## Situation

1. Agents produce code faster than review capacity absorbs it.
2. VCS records tree state and authorship. The author field no longer
   identifies a responsible party — it names whichever key the agent
   held.
3. Verification depth is not recorded anywhere. A passing test suite
   and an absent test suite are indistinguishable in the repository.
4. Failures carry information that is discarded entirely. Nineteen
   rejected approaches vanish; the twentieth is merged with no record
   of what constrained it.
5. A conclusion reached early can be invalidated by a finding reached
   late. No layer in the current stack represents this.

## Requirements

Each is stated so it can be checked against an implementation.

**R1 — Revocability without tree mutation.**
A claim's status must be changeable by appending a new record. No code
changes, no history rewrite. The prior status must remain visible, not
be overwritten.
*Forces:* stable claim identity; append-only log; status as a computed
function of the log rather than a stored field.

**R2 — Origin is a type, not a label.**
Evidence the harness observed (a test exited 0; a gate transition
fired) and evidence an agent asserted (this input can be empty) must be
structurally distinguishable and must never be merged into one status.
*Forces:* `origin.kind` is required; policy predicates may discriminate
on it; agents cannot emit harness-observed records (see `AGENTS.md`).

**R3 — Gate evaluable without build capability.**
A party with the claim log and no compute must be able to evaluate the
release predicate and get the same answer as anyone else.
*Forces:* separation of gate (cheap, deterministic, over claims) from
build (expensive, over trees). Rules out embedding build execution in
the gate.

**R4 — Status is re-checkable after release.**
An artifact's acceptability must be a live evaluation against the
current claim log, not a signature frozen at release time.
*Forces:* artifacts carry a pointer to an evaluable policy + tree, not
a verdict. Consumers re-check.

**R5 — `unevaluated` is a first-class status.**
Distinct from `passed` and from `failed`. Brownfield code is
overwhelmingly unevaluated and a system that cannot say so is unusable
on real repositories.
*Forces:* three-valued minimum; policies must be able to permit bounded
unevaluated residue with a named signer accepting it.

**R6 — Git is a projection, not the substrate.**
The claim log is the primary history. The git CLI works because elench
synthesizes git-compatible objects from the claim log — deterministically,
so two parties with the same log produce byte-identical git objects.
Humans use git; elench is invisible.
*Forces:* deterministic synthesis (BC4); elench owns the
content-addressed store; git is a read-only view, never a write path.

**R7 — Composability with existing supply-chain attestation.**
Agent claims and build provenance must be the same kind of object so
they share a signing path, a store, and a verification library.
*Forces:* in-toto statements in DSSE envelopes. See ADR-0003.

## Anti-goals

- **Do not capture reasoning.** Prompts and rationale are
  justification. Capturing them is a different, already-occupied
  product. Mixing them in makes every claim look supported.
- **Do not log all attempts.** Most failures are typos. Only failures
  that changed a claim's status are recorded. This is the filter;
  without it the corpus is noise with a schema.
- **Do not build a merge algorithm.** Concurrent-edit resolution is
  being worked elsewhere (CoAgent, STORM, CodeCRDT). elench records
  what was verified; it does not arbitrate who writes. A merge result
  is a new tree + new claims; the resolution itself is external.
- **Do not require a daemon.** Everything is derivable from the
  content-addressed store by a client-side binary. The git projection
  is a helper or a filesystem, not a service.

## Binding constraints

These gate the entire approach. Each has a pre-registered experiment.
If any fails, the corresponding part of the design is dead and no
amount of engineering recovers it.

**BC1 — Predicate ratio.**
Fraction of extracted claims expressible as machine-checkable
predicates rather than prose annotations. Prose does not gate. If the
ratio is low enough, elench is an expensive search index over agent
regret.
*Experiment:* `experiments/E0-predicate-ratio.md`. **Run this first.**

**BC2 — Anchor survival.**
Fraction of claims whose anchor still identifies the intended code
after N subsequent tree states including at least one rebase and one
rename. If anchors rot, revocation targets the wrong code and the
blast radius is fiction.
*Experiment:* `experiments/E1-anchor-survival.md`.

**BC3 — Hermeticity.**
K-of-N independent builder agreement requires bit-reproducible builds.
Without it the release gate degrades to a single signature and R3's
independent evaluation buys nothing at the artifact layer.
*Experiment:* `experiments/E2-build-reproducibility.md`.

**BC4 — Deterministic synthesis.**
The git projection must produce byte-identical objects on different
machines from the same claim log. If synthesis is non-deterministic,
two parties' git views disagree and R6 fails. The forcing requirements
are: deterministic OID derivation, deterministic author/committer
strings, deterministic timestamp derivation (not wall clock), and
deterministic tree ordering.
*No experiment.* This is prior art — `git hash-object`, `git
commit-tree`, and `git mktree` are deterministic by construction. The
risk is in the derivation mapping (which claim becomes which commit),
not in git's object format. Addressed in ADR-0007.

## Open questions

All three original open questions are now **resolved**. Details below.

- ~~Does the claim log converge, or does it grow without bound on an
  active repository? No pruning story exists. Compaction may violate
  R1.~~ **Resolved (A-A10).** Compaction is a manual, destructive
  operator action. `elench log <claims.json>` provides statistics
  (total, kind distribution, status distribution, noise ratio,
  dependsOn density) and `elench compact <claims.json> --before <ts>`
  retires all claims before the cut-off, freezing their statuses as
  a snapshot. The compaction record carries frozen statuses forward.
  Active claims continue to be revocable (R1 preserved for active
  claims, deliberately violated for retired ones).

- ~~Who signs the residue acceptance under R5, and what stops it
  becoming a rubber stamp?~~ **Resolved (A-A11).** `elench review
  <tree> <claims.json>` shows all unevaluated claims with their
  content (form, language, source, producer). The human must name
  each gap before `elench accept` issues a residue-acceptance. This
  adds real friction without multi-party complexity.

- ~~If two agents assert contradictory predicates and neither is
  falsified, what is the tree's status?~~ **Resolved (A-A12).**
  Last-writer-wins (by timestamp). The conflict is detected and
  reported by `elench conflicts <tree> <claims.json>`. The gate
  evaluates against the winning predicate and includes the conflict
  as a warning (not a failure — the winning predicate still gates).
  The human is expected to resolve: falsify one or both predicates.
  Partial block: only the affected path (same anchor) is blocked by
  the conflict; other paths evaluate normally.

- What is the granularity of a git-projection commit? Answered by
  ADR-0007: one commit per tree-changing claim. The density risk (too
  verbose for human consumption) is accepted as A-A09; a session-level
  aggregation view may be added later.
