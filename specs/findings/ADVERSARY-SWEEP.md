# Adversary Sweep — Pre-Implementation (Gate 1)

**Date:** 2026-08-31
**Mode:** Architecture (no implementation)
**Finding count:** 22 (3 Critical, 6 High, 8 Medium, 5 Low)

## Critical

1. **No timestamp field in claim schema** — ADR-0007 requires timestamps for deterministic git commit synthesis, but `schema/claim.schema.json` has no timestamp field. Without it, the git projection either cannot synthesize commits, falls back to the wall clock (rejected by ADR-0007), or pulls from an unspecified field.

2. **`validate_claim(&Claim)` cannot enforce origin.kind rules** — The function signature lacks the signer identity needed to enforce INV-06, INV-07, INV-12. The claim's `origin.kind` is self-reported; without the DSSE envelope's signer, the validator cannot distinguish an agent forging `harness-observed` from a genuine harness claim. R2 is unenforceable as designed.

3. **Tree structure ambiguous (flat vs hierarchical)** — "Identical to git SHA-256 tree OID" requires git's exact serialization (hierarchical trees, mode-space-path-null-OID, sort by name with trailing '/' for directories). The spec doesn't specify this. If elench trees are flat or use different serialization, OIDs differ and the passthrough claim is false.

## High

4. **Supersession status semantics self-contradictory** — UL says "not falsified but replaced" and also "status becomes falsified." Contradictory.
5. **No invariant for claim ID content addressing** — INV-25 covers blobs and trees but not claims. What goes into the hash is unspecified.
6. **dependsOn cycles unaddressed** — `blast_radius` and `compute_status` can loop infinitely. No cycle detection, no invariant, no error type.
7. **INV-11 cannot be enforced by `validate_claim(&Claim)`** — Needs the full log to check whether a falsification "changed some claim's status."
8. **Schema does not enforce conditional requirements** — `target` is "required unless kind=assertion" but no JSON Schema if/then. `residue-acceptance` needs non-empty target but no `minItems`.
9. **Verification record falsification — no "passed → unevaluated" transition** — If a verification claim is itself falsified, the target should revert to `unevaluated`, but the status machine has no such transition.

## Medium

10. Two contradictory predicates invisible to the gate (worse than "undefined status")
11. `origin.producer.id` is a free string — impersonation possible
12. Corrupt log cascading — claims depending on corrupt claims have undefined status
13. `origin.producer.hermeticity` is self-reported — release gate condition 4 checks self-attested value
14. E0 session selection criteria not pre-registered — gate is gameable
15. `predicateType` for agent claims vs build provenance not specified
16. Empty store / zero claims / minimal case behavior undefined
17. No "residue-acceptance from human" or "harness emits verification" feature scenarios
18. Schema `$id` is `example.invalid` placeholder
19. `additionalProperties: false` inconsistency (anchor, evidence, producer)

## Low

20. AGENTS.md says "no source code" but scaffold stubs exist
21. "tree-changing claim" used but not in UL
22. `TreeEntry.blob` field name misleading for directory entries
23. No `maxItems` on `dependsOn`, `target`, `evidence` arrays
24. `compute_status` and `blast_radius` are O(N²) — no performance invariant

## Recommendation

Implementation should NOT proceed until Critical (3) and High (6) findings are resolved. The highest-risk area is the boundary between claim identity and enforcement: the validation function's signature, the claim schema's missing timestamp, and the tree serialization ambiguity all undermine the foundations the git projection rests on.
