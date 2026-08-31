# ADR-0003 — DSSE envelopes carrying in-toto statements

**Status:** proposed
**Serves:** R7, R2

## Context
Claims need signing, a subject, and a distinction between who signed and who
produced. Build provenance needs the same. Two formats means two signing
paths, two verification libraries, and two stores.

## Decision
Reuse DSSE + in-toto. SLSA's provenance predicate already separates builder
and signer identity and does not assume a human signer, so agent-produced and
build-produced records become the same kind of object with different
`predicateType` and different `origin.kind`.

## Rejected alternatives
- **A bespoke signed-JSON format.** No compatibility gain, all the crypto
  review cost, and agent provenance ends up living beside build provenance
  instead of composing with it.
- **Raw git-signed commits as the claim record.** No subject field, no
  producer/signer split, and forces claims into tree mutations.
- **C2PA.** Has source-code manifest embedding and an AI disclosure
  assertion, but is oriented at media provenance and output attribution
  rather than verification depth. Wrong shape; revisit if regulatory
  pressure makes it mandatory anyway.

## Consequences
Inherits in-toto's verbosity and its key-distribution problem. Gains a
verification path that existing supply-chain tooling already understands.
