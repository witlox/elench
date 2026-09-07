# elench — Next Steps

**Last commit:** C4: Dogfooding — elench eats its own dog food
**State:** 411 tests (default), 237 tests (with fjall-backend). fmt clean, clippy clean. 7 crates.

## Completed

- A1: Real Ed25519 crypto via ed25519-dalek 3.0
- A2: --store CLI flag + FjallStore.read_tree
- B2: Build provenance digest — actual artifact (not stdout)
- B1: Anchor resolution — actually search trees
- C2: proptest — property-based tests
- C3: CI — .github/workflows/ci.yml
- C1: Git .git/ materialization — write real git objects
- C4: Dogfooding — elench eats its own dog food
  - `dogfooding/claims.json`: 10 claims about elench's own invariants
    (INV-01 store_blob idempotent, INV-04 compute_status pure fn,
    INV-06/07 validate_claim, INV-19 synthesize read-only, INV-25
    from_blob_data, INV-28 from_content, INV-29 check_acyclic,
    INV-13 gate evaluate, INV-22 envelope sign, INV-20 materialize).
  - `dogfooding/run.sh`: full pipeline — emit, gate, log, review,
    conflicts, git projection, materialize real .git/, verify with
    `git log` and `git fsck`.
  - `make dogfood` runs the full pipeline. Individual targets:
    `dogfood-emit`, `dogfood-gate`, `dogfood-reconcile`,
    `dogfood-project`.
  - `specs/features/dogfooding.feature` (5 scenarios).
  - The closed loop: elench records what was checked about itself,
    gates those records live, and materializes a git projection that
    `git log` and `git checkout` can read.

## Remaining

All implementation tasks (A1, A2, B1, B2, C1, C2, C3, C4) are
complete. elench is a working evidence layer with:

- 7 crates (elench, elench-claim, elench-envelope, elench-store,
  elench-gate, elench-predicate, elench-projection, elench-anchor)
- 411 tests default, 237 with fjall-backend
- 81% line coverage workspace-wide (~91% libs)
- 28 active invariants, 27 enforced (1 future: INV-15)
- 10 feature files, 9 ADRs (0000–0008)
- 3 experiments (E0/E1/E2, all PASSED)
- CI: three-tier cascading (push/PR/nightly)
- Dogfooding: elench records claims about its own code

## Remaining (in order)

### C2: proptest — property-based tests
- Add `proptest = "1"` dev-dependency
- Tests: INV-25 (content addressing), INV-20 (determinism), INV-13 (pure function), INV-29 (acyclic), INV-28 (idempotent)
- ~2 hours

### C3: CI — .github/workflows/ci.yml
- On push: Tier 1 (cargo test --lib + fmt-check + clippy)
- On PR: Tier 2 (cargo test --all-targets)
- Nightly: Tier 3 (cargo test --all-targets --features elench/fjall-backend + coverage)
- ~1 hour

### C1: Git .git/ materialization — write real git objects
- `elench git init <path>` — creates .git/ directory
- For each commit in projection: write blob, tree, commit objects to .git/objects/
- Write .git/refs/heads/main and .git/HEAD
- Result: `cd <path> && git log` works. `git blame` works. `git checkout` works.
- ~4-6 hours

### C4: Dogfooding (ongoing)
- Agents working on elench emit claims about elench's own code
- Depends on A1 (done) and A2 (done)
- Ongoing effort

## Key files

- `AGENTS.md` — project state, workflow router, harness contract
- `specs/fidelity/INDEX.md` — test depth per invariant
- `specs/architecture/enforcement-map.md` — enforcement status
- `specs/architecture/build-phases.md` — phase status (all COMPLETE)
- `specs/architecture/adr/` — ADRs 0001-0008
- `specs/features/store-backend.feature` — backend selection + read_tree round-trip
- `crates/elench-store/src/lib.rs` — StoreBackend trait, MemoryStore, canonical/deserialize tree bytes
- `crates/elench-store/src/fjall_backend.rs` — FjallStore (optional), read_tree now round-trips
- `crates/elench-projection/src/lib.rs` — synthesize(&[Claim], &dyn StoreBackend)
- `crates/elench/src/main.rs` — CLI (--store flag, emit, verify, status, gate, blast, git, store, log, review, accept, conflicts, compact, artifact, build)

## Build commands

```
make              # fmt-check + lint + Tier 1 (before every commit)
make test         # Tier 1: cargo test --lib
make test-slow    # Tier 2: cargo test --all-targets
make test-full    # Tier 3: Tier 2 + e2e (not yet configured)
make coverage     # cargo llvm-cov --workspace --fail-under-lines 50
```

### Feature flag

```
cargo test --workspace --all-targets --features elench/fjall-backend
```

Runs the persistent-store tests (`FjallStore::read_tree`, cross-reopen,
`--store fjall <path>` materialization, `interaction_7_..._fjall`).
