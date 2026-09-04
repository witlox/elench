# elench — Next Steps

**Last commit:** c9ca3e1 (A1: Real Ed25519 crypto via ed25519-dalek)
**State:** 176 tests, all passing. fmt clean, clippy clean. 8 crates.

## Completed

- A1: Real Ed25519 crypto via ed25519-dalek 3.0
  - SigningKey::generate(SignerEntity) — real Ed25519 keypair
  - sign(claim, &SigningKey) — Ed25519 over DSSE PAE
  - verify(envelope, &[VerifyingKey]) — public key only, no secrets
  - Key ID = SHA-256 of public key, first 16 hex chars
  - Tests: 12 envelope + 13 cli + 7 integration, all updated

## Remaining (in order)

### A2: --store CLI flag + FjallStore.read_tree
- Parse `--store memory` (default) or `--store fjall <path>` from CLI args
- Wire chosen backend into all commands that use a store (emit, gate, git, etc.)
- Implement `FjallStore::read_tree`: deserialize canonical bytes (mode space name null oid) back to `Vec<TreeEntry>`
- elench-store/Cargo.toml already has `fjall = { version = "3", optional = true }` with `fjall-backend` feature
- `FjallStore` already implements `StoreBackend` trait (crates/elench-store/src/fjall_backend.rs)
- `elench-projection::synthesize` already takes `&impl StoreBackend`
- CLI currently hardcodes `elench_store::MemoryStore::new()` — needs `--store` flag
- ~2 hours

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
- Nightly: Tier 3 (cargo test --all-targets --features elench-store/fjall-backend + coverage)
- ~1 hour

### C1: Git .git/ materialization — write real git objects
- `elench git init <path>` — creates .git/ directory
- For each commit in projection: write blob, tree, commit objects to .git/objects/
- Write .git/refs/heads/main and .git/HEAD
- Result: `cd <path> && git log` works. `git blame` works. `git checkout` works.
- ~4-6 hours

### C4: Dogfooding (ongoing)
- Agents working on elench emit claims about elench's own code
- Depends on A1 (done) and A2 (pending)
- Ongoing effort

## Key files

- `AGENTS.md` — project state, workflow router, harness contract
- `specs/fidelity/INDEX.md` — test depth per invariant
- `specs/architecture/enforcement-map.md` — enforcement status
- `specs/architecture/build-phases.md` — phase status (all COMPLETE)
- `specs/architecture/adr/` — ADRs 0001-0008
- `crates/elench-envelope/src/lib.rs` — Ed25519 signing/verification
- `crates/elench-store/src/lib.rs` — StoreBackend trait + MemoryStore
- `crates/elench-store/src/fjall_backend.rs` — FjallStore (optional)
- `crates/elench/src/main.rs` — CLI (emit, verify, status, gate, blast, git, store, log, review, accept, conflicts, compact, artifact, build)

## Build commands

```
make              # fmt-check + lint + Tier 1 (before every commit)
make test         # Tier 1: cargo test --lib
make test-slow    # Tier 2: cargo test --all-targets
make coverage     # cargo llvm-cov --workspace --fail-under-lines 50
```
