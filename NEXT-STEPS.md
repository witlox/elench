# elench — Next Steps

**Last commit:** C2: proptest — property-based tests
**State:** 406 tests (default), 432 tests (with fjall-backend). fmt clean, clippy clean. 7 crates.

## Completed

- A1: Real Ed25519 crypto via ed25519-dalek 3.0
- A2: --store CLI flag + FjallStore.read_tree
- B2: Build provenance digest — actual artifact (not stdout)
- B1: Anchor resolution — actually search trees
- C2: proptest — property-based tests
  - Added `proptest = "1"` as a workspace dev-dependency, forwarded
    to `elench-claim`, `elench-store`, `elench-projection`.
  - `elench-claim`: `proptest_inv_25` (SHA-256 deterministic),
    `proptest_inv_28` (ClaimId is content hash), `proptest_inv_13`
    (compute_status is pure), `proptest_inv_29` (dependsOn chain
    terminates).
  - `elench-store`: `proptest_inv_25` (Oid::from_blob_data
    deterministic + round-trip + tree OID deterministic),
    `proptest_inv_28` (store_blob idempotent).
  - `elench-projection`: `proptest_inv_20` (synthesis deterministic
    + order-independent).
  - `specs/features/proptest-property.feature` (5 scenarios).
  - Tests are fast (proptest default: 256 cases each) and run in
    Tier 1 (`cargo test --lib`).

## Remaining (in order)

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
- Depends on A1 (done), A2 (done), B1 (done), B2 (done), C2 (done)
- Ongoing effort

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
