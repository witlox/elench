# Ubiquitous Language

Terms whose loose use will destroy the design. Every term here has a
precise meaning; using one loosely in a spec or in code is a defect.

## Core concepts

**claim** — a signed assertion about a tree. Not a commit, not a comment,
not a log line. Has a stable identity (`id`), a `kind`, an `assertion`,
an `origin`, an `anchor`, optional `evidence`, and optional `dependsOn`.

**tree** — a content-addressed tree state in the elench store (ADR-0001).
NOT a git commit. The git projection (ADR-0002) synthesizes git commits
from tree states, but the tree is the elench-native concept. Claims are
anchored to spans within a tree.

**claim log** — the append-only set of claims that IS the primary
history (ADR-0001). No record is ever overwritten; status is computed
by folding the log, never stored. The git projection is derived from
the claim log, not the other way around.

**status** — a claim's current standing, computed from the log: `passed`,
`falsified`, `unevaluated`. NOT a stored field. A claim with no
falsification or verification record against it is `unevaluated` by
default, not `passed`.

## Assertion types

**predicate** — a claim whose `assertion.form = predicate`. Carries an
executable `expression` that can be evaluated by a machine. Can gate.
A predicate without an executable expression is an annotation wearing a
costume; validators MUST reject it.

**annotation** — a claim whose `assertion.form = annotation`. Carries
only prose. Searchable. Cannot gate. Never read by policy. This is not
a lesser predicate; it is a different thing.

**falsification** — a claim with `kind = falsification` targeting an
earlier claim. Appended, never overwriting. The prior status stays
visible. This is the mechanism that makes retroactive invalidation work.

**verification** — a claim with `kind = verification`. Only the harness
emits these. An agent believing something passed is an assertion, not a
verification.

**supersession** — a claim with `kind = supersession` targeting an
earlier claim. The earlier claim is not falsified but is replaced. A
supersession claim changes the target's computed status to `falsified`
— same status effect as falsification, different intent. Supersession is
a claim kind, not a status value.

**residue-acceptance** — a claim with `kind = residue-acceptance`. A
human key signing over named unevaluated gaps. The terminator, made
explicit and made small: not "I reviewed this" but "I accept these
specific gaps." Agents MUST NOT emit this.

## Status values

**unevaluated** — no verification was attempted. Distinct from `falsified`
(verification ran and did not hold) and from `passed`. Most of a
brownfield repository is `unevaluated`, and a system that cannot say so
is lying.

**passed** — a verification record exists and no falsification has
invalidated it. Computed, not stored.

## Origin

**origin** — a typed value, not a label. `origin.kind` is required and
structurally distinguishes evidence sources. Policies may and generally
should discriminate on it.

**harness-observed** — `origin.kind = harness-observed`. Emitted by the
harness from direct observation (a test exited 0; a gate transition
fired). The agent cannot produce or suppress these.

**agent-asserted** — `origin.kind = agent-asserted`. Emitted by the
audited party. Legitimate, useful, and structurally weaker. Never
silently merged with `harness-observed`.

**human-asserted** — `origin.kind = human-asserted`. Emitted by a human
key. Distinct from `agent-asserted` because a human can be held
responsible in ways an agent cannot.

**producer** — the entity that produced a claim (harness build id, model
id + version, or human key id). Distinct from the **signer** (the key
that signed the DSSE envelope). The envelope supplies signer identity;
the claim payload supplies producer identity. These are different things
(R2).

## Blast radius

**blast radius** — the transitive `dependsOn` closure from a falsified
claim. Only as trustworthy as the anchors (see `docs/anchoring.md`) and
as populated as the `dependsOn` field (E0 secondary measurement). If
`dependsOn` is empty in practice, revocation cannot propagate and the
central capability is unreachable.

## Evidence and verification

**evidence** — a record of what was observed: process-exit, test-report,
artifact-digest, or external-attestation. Empty for agent-asserted
claims by default; that emptiness is the signal, not a gap to fill.

**verification** (as a concept) — evidence was observed. Distinct from
**justification** (reasoning about why a choice was made). Existing
tools capture justification. Confusing the two makes every claim look
supported; if this distinction stops being honoured, the project has
quietly become something else.

## Release

**release policy** — a predicate over claims that determines whether an
artifact is releasable. Cheap and deterministic; evaluable without build
capability (R3). See `docs/release-policy.md`.

**artifact** — something that can be released. Carries a pointer to
`(tree, policy)`, not a verdict. Consumers re-evaluate. If a
load-bearing claim is falsified after release, the artifact's status
changes with no byte moving and no re-signing.

**residue** — the set of `unevaluated` claims remaining at evaluation
time. Bounded by policy; excess must be covered by a
`residue-acceptance` record signed by a key the policy names.

## Anchoring

**anchor** — how a claim points at code within a tree. Strategy:
`path-range` (trivial, rots on reformat), `symbol` (language-server
qualified name, dies on rename), `content-digest` (normalised digest,
dies on any semantic edit), or `multi` (all three, resolve by
agreement). UNRESOLVED — E1 determines which survives.

## Substrate and projection

**elench store** — the content-addressed store that IS the substrate
(ADR-0001). Holds blobs, trees, and claims. No daemon. Everything is
derivable from the store by a client-side binary. Blobs and trees are
addressed by SHA-256 content hash (64 hex chars, no prefix), identical
to git SHA-256 blob/tree OIDs. Claims are addressed with a `cl_`
prefix. The git projection is a passthrough for blobs and trees; only
commits are synthesized.

**git projection** — a deterministic, read-only synthesis of git objects
from the claim log (ADR-0002). `git log`, `git blame`, `git checkout`
work because elench produces git-compatible objects. Writes go through
elench, never through git. Two parties with the same claim log produce
byte-identical git objects (BC4, ADR-0007).
