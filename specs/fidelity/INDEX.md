# Fidelity Index

Test depth per invariant. All phases (0–5) are implemented, plus
post-implementation improvements (anchor resolution, build provenance,
`.git/` materialization, proptest, conflict detection, continuous
dogfooding, INV-15 artifact format).

**Test counts:** 344 (default), 351 (with `fjall-backend`). 92% line
coverage workspace-wide. fmt clean, clippy clean.

## Invariants

| INV | Description | Depth | Notes |
|-----|-------------|-------|-------|
| INV-01 | Status changed by appending, not modifying | MOCK | elench-store: store_blob/tree/claim idempotent (memory + fjall) |
| INV-02 | Prior status remains visible | MOCK | elench-claim: compute_status reads entire log |
| INV-03 | Claim identity stable across status changes | MOCK | elench-claim: ClaimId immutable |
| INV-04 | Status computed by folding, not stored | MOCK | elench-claim: compute_status pure fn |
| INV-05 | origin.kind required and structurally distinct | MOCK | elench-claim: type system enforces |
| INV-06 | Agents cannot emit harness-observed | MOCK | elench-claim: validate_claim cross-checks signer |
| INV-07 | Only harness emits verification | MOCK | elench-claim: validate_claim cross-checks signer |
| INV-08 | Predicate requires executable expression | MOCK | elench-claim: validate_claim calls elench-predicate::parse |
| INV-09 | Annotations never read by policy | MOCK | elench-gate: evaluate filters on form=predicate |
| INV-10 | dependsOn populated with premises | REMOVED | Downgraded to guideline; empty dependsOn is a warning, not a rejection |
| INV-11 | Failure recorded only when status changed | MOCK | elench-claim: validate_claim checks target status in log |
| INV-12 | Agents cannot emit residue-acceptance | MOCK | elench-claim: validate_claim cross-checks signer |
| INV-13 | Gate evaluable without build capability | MOCK | elench-gate: evaluate takes &[Claim], no build |
| INV-14 | Artifact acceptability is live evaluation | MOCK | elench-gate: evaluate called on demand, no cache |
| INV-15 | Artifact carries (tree, policy), not verdict | MOCK | elench-gate: Artifact struct with version field; schema/artifact.schema.json; 6 tests |
| INV-16 | unevaluated is first-class status | MOCK | elench-claim: ClaimStatus::Unevaluated |
| INV-17 | Policies permit bounded unevaluated residue | MOCK | elench-gate: condition 2, residue-acceptance |
| INV-18 | elench owns content-addressed store | MOCK | elench-store: no git dependency, owns storage; both backends pure-Rust |
| INV-19 | Git projection is read-only | MOCK | elench-projection: synthesize takes &dyn StoreBackend, no writes |
| INV-20 | Git synthesis is deterministic (BC4) | PROPERTY | elench-projection: proptest_inv_20_synthesis_deterministic + order_independent |
| INV-21 | Git projection produces no side effects | MOCK | elench-projection: scenario_inv19_projection_does_not_write_to_store |
| INV-22 | Agent claims and provenance share DSSE/in-toto | MOCK | elench-envelope: sign/verify, PREDICATE_TYPE_AGENT, Ed25519 (A1) |
| INV-23 | Predicate expressions executable/deterministic/sandboxable | MOCK | elench-predicate: 4 primitives, not Turing-complete |
| INV-24 | AGENTS.md rules enforced by validator | MOCK | elench-claim: validate_claim (Phase 0) |
| INV-25 | Content addressing (SHA-256) | PROPERTY | elench-claim: ClaimId::from_content; elench-store: Oid::from_blob_data/from_tree_entries + deserialize_tree_bytes round-trip + FjallStore::read_tree (feature tier) |
| INV-26 | Store is sole source of truth | MOCK | elench-store: all views derive from store; selectable at runtime via `--store memory|fjall <path>` |
| INV-27 | Git projection is lossy, not authoritative | MOCK | elench-projection: scenario_inv27_projection_is_lossy |
| INV-28 | Claim OID is content hash | PROPERTY | elench-claim: ClaimId::from_content (SHA-256 of canonical JSON); proptest_inv_28 |
| INV-29 | dependsOn acyclic | PROPERTY | elench-claim: validate_claim DFS cycle detection; proptest_inv_29 |

28 active invariants (24 original minus INV-10 removed, plus
INV-25/26/27/28/29 added). **28 ENFORCED** (was 27 + INV-15 future;
INV-15 now MOCK with 6 tests). 0 future.

## Features

| Feature | Scenarios | Depth | Notes |
|---------|-----------|-------|-------|
| claim-emission | 7 | MOCK | elench-claim: validate_claim + types. 1 scenario untested (empty dependsOn warning — dead code) |
| claim-revocation | 7 | MOCK | elench-claim: compute_status + blast_radius + dependsOn propagation. All 7 covered. |
| origin-typing | 6 | MOCK | elench-claim: validate_claim cross-checks. 2 scenarios untested (query-by-origin.kind not implemented) |
| release-gate | 9 | MOCK | elench-gate: evaluate, 4 conditions. 1 scenario untested (contradictory predicates — FM-P2-02 known limitation) |
| anchor-resolution | 11 | MOCK | elench-anchor: resolve_path_range/symbol/content_digest, reconcile CLI. 1 scenario untested (wrong-resolution — dead code) |
| unevaluated-residue | 6 | MOCK | elench-claim: ClaimStatus::Unevaluated. 1 scenario untested (corrupt cascade — no corrupt status) |
| git-projection | 4 | MOCK | elench-projection: synthesize, git_log_oneline/full, materialize. 2 scenarios untested (git blame, write-through-git rejected) |
| store-backend | 5 | MOCK | elench-store: --store flag, FjallStore::read_tree round-trip. All 5 covered. |
| build-provenance | 4 | SHALLOW | elench: --artifact flag, SHA-256 of build output. CLI output checks, not in-process claim verification. |
| proptest-property | 5 | PROPERTY | proptest: INV-25/20/13/29/28 (256 cases each). All 5 covered. |
| dogfooding | 7 | NONE | Shell scripts (run.sh, emit-continuous.sh) with no assertions. 6 of 7 scenarios have no Rust test. |

**71 scenarios total.** 57 tested, 14 untested (see per-feature notes).
**344 tests** (default), **351** (with `fjall-backend`). 92% line
coverage.

## Untested scenarios (14)

| Feature | Scenario | Reason |
|---------|----------|--------|
| anchor-resolution | Wrong-resolution is reported distinctly from failure | `StrategyOutcome::Wrong` is dead code (`#[allow(dead_code)]`) |
| claim-emission | A claim with empty dependsOn is accepted with a warning | `ValidationError::EmptyDependsOn` is defined but never returned |
| dogfooding (×6) | Dogfooding claims emitted/stored, gate, log stats, git projection, continuous emission, CI nightly | Shell scripts with `\|\| true`, no assertions |
| git-projection | git blame maps to claims | No test exercises `git blame` after materialization |
| git-projection | Write through git is rejected | No test verifies git writes don't propagate back |
| origin-typing (×2) | Harness-observed and agent-asserted claims are distinct; human-asserted distinct from agent | No query-by-origin.kind function exists |
| release-gate | Two contradictory predicates: gate passes (known limitation) | FM-P2-02 not tested |
| unevaluated-residue | A corrupt claim cascades to unevaluated dependents | No "corrupt" status distinction; `compute_status` returns `Err(ClaimNotFound)` |

## Experiments

| Experiment | Gates | Status | Result |
|------------|-------|--------|--------|
| E0 — Predicate ratio | BC1 | **PASSED** | 0.72 (threshold >= 0.30). PROCEED AS DESIGNED. |
| E1 — Anchor survival | BC2 | **PASSED** | All strategies USABLE (correct >= 85%, wrong <= 2%). Proceed with multi. |
| E2 — Build reproducibility | BC3 | **PASSED** | Same-triple divergences all cheap-to-fix. K-of-N available. |

## Implementation status

All phases (0–5) are **implemented**, plus elench-anchor, store-backend
selection, build provenance, `.git/` materialization, proptest,
conflict detection, continuous dogfooding, and INV-15 artifact format.

- Phase 0: elench-predicate (64 tests) + elench-claim (40 tests)
- Phase 1: elench-store (34 tests default; 40 with `fjall-backend`)
- Phase 2: elench-envelope (12 tests)
- Phase 3: elench-gate (23 tests)
- Phase 4: elench-projection (14 tests)
- Phase 5: elench binary (9 unit + 65 cli + 9 integration tests; 10 integration with `fjall-backend`)
- elench-anchor (23 tests)

Store backend: in-memory (default), fjall (optional feature, ADR-0008).
`--store memory|fjall <path>` selects the backend at the CLI; `synthesize`
takes `&dyn StoreBackend` so any command can target either. `materialize`
writes real zlib-compressed git objects to `.git/objects/`.

Dogfooding: `dogfooding/claims.json` (10 claims about elench's own
invariants), `make dogfood` runs the full pipeline (emit, gate, log,
review, conflicts, git projection, materialize, verify). Continuous
dogfooding (`dogfooding/emit-continuous.sh`) emits 4 harness-observed
claims per CI run.
