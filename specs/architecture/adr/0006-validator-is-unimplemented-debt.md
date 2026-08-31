# ADR-0006 — AGENTS.md rules are unimplemented gates

**Status:** accepted (as a statement of debt, not a design)

## Context
`AGENTS.md` states MUST rules governing what agents may emit. Nothing
enforces them. Prose gates do not gate.

## Decision
Record every rule in `AGENTS.md` as a named, unimplemented validator check.
Until implemented, they are `unevaluated` — not `passed`. The project's own
distinction applies to the project.

## Consequences
The first implementation milestone is the validator, not the CLI. A claim
log with unenforced emission rules is not evidence; it is testimony from the
audited party in a structured format.
