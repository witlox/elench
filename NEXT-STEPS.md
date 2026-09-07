# elench — Next Steps

**Last commit:** All tasks complete
**State:** 344 tests (default), 351 tests (with fjall-backend). 83% line coverage. fmt clean, clippy clean. 7 crates.

All implementation tasks (A1, A2, B1, B2, C1, C2, C3, C4) plus
post-implementation improvements (INV-15, conflict detection, coverage,
continuous dogfooding) are complete. No remaining work.

## Completed

- A1: Real Ed25519 crypto via ed25519-dalek
- A2: --store CLI flag + FjallStore.read_tree
- B2: Build provenance digest — actual artifact (not stdout)
- B1: Anchor resolution — actually search trees
- C2: proptest — property-based tests
- C3: CI — .github/workflows/ci.yml
- C1: Git .git/ materialization — write real git objects
- C4: Dogfooding — elench eats its own dog food
- INV-15: Artifact format — schema/artifact.schema.json + version field (NONE → MOCK)
- Conflict detection: extracted to elench-claim, fixed (same-anchor, different expression)
- Coverage: elench-predicate 80→90%, elench-envelope 83→90%, elench binary 40→70%
- Continuous dogfooding: emit-continuous.sh runs build/test/lint/fmt,
  emits harness-observed claims, gates them live, CI nightly

## Key files

- `AGENTS.md` — project state, workflow router, harness contract
- `specs/fidelity/INDEX.md` — test depth per invariant
- `specs/architecture/enforcement-map.md` — enforcement status
- `specs/architecture/adr/` — ADRs 0001-0008
- `specs/features/` — 11 feature files, 71 scenarios
- `schema/` — claim.schema.json, artifact.schema.json
- `dogfooding/` — claims, run.sh, emit-continuous.sh, README
- `.github/workflows/ci.yml` — three-tier cascading CI

## Build commands

```
make              # fmt-check + lint + Tier 1 (before every commit)
make test         # Tier 1: cargo test --lib
make test-slow    # Tier 2: cargo test --all-targets
make test-full    # Tier 3: Tier 2 + dogfooding e2e
make coverage     # cargo llvm-cov --workspace --fail-under-lines 50
make dogfood      # static dogfooding pipeline
make dogfood-continuous  # continuous: build/test/lint/fmt → claims → gate
```

### Feature flag

```
cargo test --workspace --all-targets --features elench/fjall-backend
```
