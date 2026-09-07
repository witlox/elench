# elench — Next Steps

**Last commit:** B1: Anchor resolution — actually search trees
**State:** 193 tests (default), 222 tests (with fjall-backend). fmt clean, clippy clean. 7 crates.

## Completed

- A1: Real Ed25519 crypto via ed25519-dalek 3.0
- A2: --store CLI flag + FjallStore.read_tree
- B2: Build provenance digest — actual artifact (not stdout)
- B1: Anchor resolution — actually search trees
  - `resolve_path_range(anchor, store)`: navigate tree by path, read
    blob, verify line range [start, end] is within the blob. Fails on
    rename, missing path, or range exceeding blob size.
  - `resolve_symbol(anchor, store)`: traverse all blobs recursively,
    search for definition patterns (fn name, def name, struct name,
    etc.). Returns the path of the first blob containing the definition.
  - `resolve_content_digest(anchor, store)`: check has_blob(digest)
    directly, then traverse all blobs comparing SHA-256. Returns the
    path of the first matching blob.
  - `reconcile(tree, log, store)`: passes the store through to
    resolve, so reconciliation actually checks tree data. CLI:
    `elench reconcile <tree> <claims.json>`.
  - `resolve`/`reconcile` now take `&dyn StoreBackend`.
    `elench-anchor` gained `elench-store` as a dependency.
  - 23 anchor tests (real tree search against MemoryStore), 4 CLI
    reconcile tests.

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
- Depends on A1 (done), A2 (done), B1 (done), B2 (done)
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
