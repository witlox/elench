# Invariants

Every invariant is testable (expressible as an assertion). Every
invariant has an enforcement point (to be mapped in
`specs/architecture/enforcement-map.md`). Invariants are numbered for
stable reference; do not renumber.

## Revocability and status (R1)

**INV-01.** A claim's status is changed by appending a new record to the
claim log, never by modifying or deleting an existing record.

**INV-02.** The prior status remains visible after a status change. The
original claim record is unchanged; only the computed status changes.

**INV-03.** Claim identity (`id`) is stable across status changes. The
`id` is the content address at creation and is never reassigned.

**INV-04.** Status is a pure function of the claim log, computed by
folding. No claim stores its own status as a field.

## Origin typing (R2, AGENTS.md)

**INV-05.** `origin.kind` is required on every claim. The three values
(`harness-observed`, `agent-asserted`, `human-asserted`) are
structurally distinguishable and must never be merged into one status.

**INV-06.** An agent MUST NOT emit a record with
`origin.kind = "harness-observed"`. This is the single rule that keeps
origin meaningful.

**INV-07.** Only the harness emits `kind = "verification"`. An agent
believing something passed is an assertion, not a verification.

**INV-08.** A claim with `assertion.form = "predicate"` MUST supply an
executable `expression`. Prose in a predicate slot is rejected at
validation. Prose gates do not gate.

**INV-09.** Annotations are never read by policy. An
`assertion.form = "annotation"` claim cannot contribute to a gate
verdict regardless of its content.

**INV-10.** REMOVED. Empty `dependsOn` is a warning, not a rejection.
Downgraded to a guideline per domain expert decision.

**INV-11.** A failure is recorded only when it changed some claim's
status. A failed attempt that falsified nothing is noise and must not
be recorded.

**INV-12.** An agent MUST NOT emit `kind = "residue-acceptance"`. That
record requires a human key and represents a person accepting named
unevaluated gaps (R5).

## Gate and release (R3, R4)

**INV-13.** The release gate can be evaluated without build capability.
A party with only the claim log and no compute must reach the same
verdict as a party with a build farm.

**INV-14.** An artifact's acceptability is a live evaluation against the
current claim log, not a signature frozen at release time.

**INV-15.** An artifact carries a pointer to `(tree, policy)`, not a
verdict. Consumers re-evaluate. The tree is an elench tree OID
(ADR-0001), not a git commit.

## Unevaluated (R5)

**INV-16.** `unevaluated` is a first-class status, distinct from
`passed` and from `failed`. A system that cannot represent it is lying
about brownfield code.

**INV-17.** Policies must be able to permit bounded `unevaluated`
residue, with each excess covered by a `residue-acceptance` record
signed by a key the policy names.

## Substrate and projection (R6, ADR-0001, ADR-0002, ADR-0007)

**INV-18.** elench owns its own content-addressed store. The claim log
IS the primary history. There is no separate git repository underneath.

**INV-19.** The git projection is a read-only synthesis of git objects
from the claim log. Writes go through elench, never through git.

**INV-20.** Git object synthesis is deterministic. Two parties with the
same claim log produce byte-identical git objects (BC4). Commit OIDs
are derived from the claim log, not from wall-clock time or machine
state.

**INV-21.** The git projection produces no side effects. It reads from
the store and generates objects on demand. It does not modify the
store, the claim log, or the working tree.

## Supply-chain composability (R7, ADR-0003)

**INV-22.** Agent claims and build provenance share the same DSSE/in-toto
envelope format. Same signing path, same store, same verification
library.

## Predicate language (ADR-0004)

**INV-23.** Predicate expressions must be executable, deterministic, and
sandboxable. The language is undecided (gated by E0); until decided,
`expression.language` is a free string and no validator bakes in an
answer.

## Validator (ADR-0006)

**INV-24.** The AGENTS.md emission rules MUST be enforced by a
validator. Until implemented, they are `unevaluated`, not `passed`.
The first implementation milestone is the validator, not the CLI.

## Substrate and projection (addendum)

**INV-25.** A blob's OID MUST equal a deterministic SHA-256 hash of its
content. A tree's OID MUST equal a deterministic SHA-256 hash of its
canonical serialization (sorted entries, mode, path, blob OID). Two
objects with the same content have the same OID. This is the
content-addressing property; without it the store is not the substrate.

**INV-26.** All derived views (claim status, blast radius, git
projection, release verdict) are computable from the elench store
alone. No external state (configuration, identity, network) is required
for evaluation.

**INV-27.** The git projection is lossy by design. Claim status,
origin.kind, and blast radius are NOT recoverable from git objects. The
git projection MUST NOT be the authoritative source for any property
the claim log computes.

**INV-28.** A claim's OID MUST equal the SHA-256 hash of its canonical JSON serialization (all fields except `id` itself). Two claims with identical content (assertion, origin, anchor, evidence, dependsOn, timestamp) get the same OID — this is deduplication, and it is correct: two agents independently asserting the same predicate about the same span produce one claim, not two. Two claims with different content get different OIDs.

**INV-29.** `dependsOn` MUST be acyclic. A claim MUST NOT list itself, directly or transitively, in `dependsOn`. The validator rejects cyclic `dependsOn`; `blast_radius` and `compute_status` detect cycles and return an error rather than looping.
