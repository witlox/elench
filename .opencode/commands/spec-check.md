---
description: "Spec/code alignment: ubiquitous language drift, invariant enforcement, scenario coverage."
---

Spec consistency check. Validates that specs, architecture, and code stay aligned.

1. **Ubiquitous language drift**: grep all Rust source files for type names.
   Compare against `specs/ubiquitous-language.md`. Flag types that don't match
   a defined term, and terms that have no corresponding type.

2. **Invariant enforcement coverage**: read `specs/architecture/enforcement-map.md`.
   For each invariant, check if the enforcement point exists in code (file/function).
   Report: ENFORCED (code exists) / UNIMPLEMENTED (file missing) / UNKNOWN.

3. **Scenario coverage**: for each `specs/features/*.feature`, check if a
   corresponding test exists:
   - Rust unit tests: `grep -r "scenario_" crates/*/src/ crates/*/tests/`
   - CLI integration: `grep -r "scenario_cli_" crates/elench/tests/`
   Report: COVERED / PARTIAL / NONE per feature file.

4. **ADR compliance**: for each ADR in `specs/architecture/adr/`, check if
   the decision is reflected in code. Flag ADRs marked "proposed" that
   have been implemented (should be "accepted") and vice versa.

5. **Schema drift**: compare `schema/claim.schema.json` and
   `schema/artifact.schema.json` against the Rust types in
   `elench-claim/src/lib.rs` and `elench-gate/src/lib.rs`.
