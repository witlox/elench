# elench — Next Steps

**Last commit:** A2: --store CLI flag + FjallStore.read_tree
**State:** 196 tests (default), 203 tests (with fjall-backend). fmt clean, clippy clean. 7 crates.

## Completed

- A1: Real Ed25519 crypto via ed25519-dalek 3.0
  - SigningKey::generate(SignerEntity) — real Ed25519 keypair
  - sign(claim, &SigningKey) — Ed25519 over DSSE PAE
  - verify(envelope, &[VerifyingKey]) — public key only, no secrets
  - Key ID = SHA-256 of public key, first 16 hex chars
  - Tests: 12 envelope + 13 cli + 7 integration, all updated

- A2: --store CLI flag + FjallStore.read_tree
  - `--store memory` (default) | `--store fjall <path>` parsed as a global
    flag (before the command or before `--`). Unknown backends and a missing
    value are rejected with a clear message. `--store fjall` without the
    `fjall-backend` feature reports how to enable it.
  - `elench-store` now has `deserialize_tree_bytes` (inverse of
    `canonical_tree_bytes`); `FjallStore::read_tree` round-trips the
    canonical form back to `Vec<TreeEntry>` (was a deferred empty-tree stub).
  - `elench-projection::synthesize` now takes `&dyn StoreBackend` so a
    runtime-selected backend can be used. Wired into `emit`, `store
    blob`/`store tree`, and `git` via `open_store(&StoreConfig)`.
  - `elench` binary gained a `fjall-backend` feature forwarding to
    `elench-store/fjall-backend`.
  - `specs/features/store-backend.feature` (5 scenarios). Tests:
    `deserialize_tree_bytes` round-trip (default tier), `FjallStore::read_tree`
    round-trip + cross-reopen (feature tier), `--store` flag parsing
    (unit + CLI), and `interaction_7_projection_uses_stored_tree_{memory,fjall}`.

## Remaining (in order)

### B2: Build provenance digest — actual artifact (not stdout)
- `elench build <tree> -- <command> --artifact <path>` runs the build, then SHA-256 the file at `--artifact` path
- If no `--artifact`, fall back to stdout digest (current behavior)
- ~30 min

### B1: Anchor resolution — actually search trees
- `resolve_path_range(anchor, store)` — read tree, find entry at path, check lines
- `resolve_symbol(anchor, store)` — traverse blobs, search for symbol definition
- `resolve_content_digest(anchor, store)` — traverse blobs, normalize content, compare digest
- Multi-strategy: try all three, resolve by agreement (existing logic in elench-anchor)
- `elench reconcile <tree> <claims.json>` CLI command (currently library-only)
- ~4 hours

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
