# ADR-0004 — Predicate expression language (UNDECIDED)

**Status:** proposed — no decision made
**Gated by:** E0. The predicate expressions E0 collects are the requirements
input for this decision. Deciding before E0 is designing against a guess.

## Context
`assertion.expression` must be executable, deterministic, sandboxable, and
writable by an agent under time pressure. These pull in different directions.

## Candidates, none chosen
- **Rego (OPA).** Mature, purpose-built for policy, sandboxed. Awkward for
  assertions about program behaviour rather than about data.
- **CUE.** Strong at constraint expression and schema. Small ecosystem.
- **Starlark.** Deterministic Python subset, easy for a model to emit,
  well-sandboxed. Not designed for policy.
- **Existing test frameworks as the predicate.** The predicate *is* a named
  test; the expression is a test identifier. Cheapest by far and reuses all
  existing infrastructure. Weakness: shallow tests asserting almost nothing
  are structurally equivalent to no tests, and this route makes that failure
  invisible rather than visible.
- **Two languages** — one for policy over claims, one for assertions about
  code. Honest about the split; doubles the surface.

## Consequences of deferring
`schema/claim.schema.json` leaves `expression.language` as a free string.
That is a hole, and it should be closed before any validator is written or
the validator will bake in the answer by accident.
