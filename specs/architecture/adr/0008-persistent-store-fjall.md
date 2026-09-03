# ADR-0008 — Persistent store: fjall as the content-addressed backend

**Status:** proposed
**Serves:** R1 (revocability), INV-01 (append-only), INV-18 (elench owns store)

## Context

The current `Store` in `elench-store` is in-memory (`HashMap`). Claims,
blobs, and trees do not persist across sessions. ADR-0001 requires
elench to own its own content-addressed store; ADR-0005 requires pure
Rust with no C/C++ dependency.

## Decision

Use **fjall** as the persistent backend for `elench-store`, behind an
optional `fjall-backend` feature flag.

### Keyspaces

One fjall database, three keyspaces (matching the current `Store` API):

| Keyspace | Key | Value | Durability |
|----------|-----|-------|------------|
| `blobs` | SHA-256 OID (64 hex) | blob data | `Buffer` (content-addressed, can be recomputed) |
| `trees` | SHA-256 OID (64 hex) | serialized tree entries | `Buffer` |
| `claims` | `cl_` + SHA-256 OID (67 chars) | serialized claim JSON | `SyncAll` (evidence must survive crashes) |

### StoreBackend trait

Extract a `StoreBackend` trait from the current `Store`:

```rust
pub trait StoreBackend {
    fn store_blob(&mut self, data: &[u8]) -> Result<Oid, StoreError>;
    fn store_tree(&mut self, entries: Vec<TreeEntry>) -> Result<Oid, StoreError>;
    fn store_claim(&mut self, claim: &Claim) -> Result<String, StoreError>;
    fn read_blob(&self, oid: &Oid) -> Result<Vec<u8>, StoreError>;
    fn read_tree(&self, oid: &Oid) -> Result<Tree, StoreError>;
    fn read_all_claims(&self) -> Result<Vec<Claim>, StoreError>;
    fn read_claims_for_tree(&self, tree: &Oid) -> Result<Vec<Claim>, StoreError>;
    fn has_blob(&self, oid: &Oid) -> bool;
    fn has_tree(&self, oid: &Oid) -> bool;
    fn has_claim(&self, claim_oid: &str) -> bool;
    fn blob_count(&self) -> usize;
    fn tree_count(&self) -> usize;
    fn claim_count(&self) -> usize;
}
```

- `MemoryStore`: current `HashMap` implementation (default, no deps)
- `FjallStore`: fjall-backed implementation (optional feature)

`elench-projection::synthesize` takes `&impl StoreBackend` instead of
`&Store`. The CLI accepts `--store memory` (default) or `--store fjall
<path>`.

### Why fjall

| Criterion | fjall | redb | sled | rocksdb | custom |
|-----------|-------|------|------|---------|--------|
| Pure Rust | Yes | Yes | Yes | No (C++) | Yes |
| Build deps | None | None | None | cmake, clang | None |
| Binary size | ~100KB | ~50KB | ~200KB | ~5MB | 0 |
| LSM-tree | Yes | No (B-tree) | Yes | Yes | Manual |
| Append-only | Natural | No | Yes | Yes | Manual |
| Crash recovery | Automatic | Automatic | Yes | Automatic | Manual |
| Compaction | Yes | None | Yes | Yes | None |
| Compression | LZ4 | No | Yes | Yes | No |
| Cross-keyspace atomic | Yes | No | No | No | Manual |
| Key-value separation | Yes | No | No | No | Manual |
| Stable disk format | Yes | Yes | No | Yes | N/A |
| Maintained | Yes (2.3k stars) | Yes | No (since 2022) | Yes | N/A |
| Proven in production | kiseki 3.1 | kiseki (rev-1) | No | Yes | No |

kiseki migrated from redb to fjall across 4 revisions (ADR-022).
Benchmark: 36k PUT/s vs redb's 18k (2x), 194k GET/s vs 150k (1.3x).

## Rejected alternatives

- **redb.** B-tree, no compaction, no key-value separation. Was
  kiseki's first choice; 2x slower on PUT. No cross-keyspace
  atomicity (blobs + trees + claims need separate databases).
- **sled.** Unmaintained since 2022. Unstable disk format. NOT
  recommended.
- **RocksDB.** C++ dependency (cmake, clang, librocksdb). Violates
  ADR-0005 (no C surface). ~5MB binary size.
- **Custom files.** Zero deps but no crash recovery, no compaction,
  no compression. Manual fsync. Not worth the engineering cost.

## Consequences

Adds `fjall` as an optional dependency (~100KB binary size when
enabled). Gains crash recovery, compaction, compression, and
cross-keyspace atomicity. The `StoreBackend` trait keeps the API
stable — existing 30 store tests and 14 projection tests continue to
pass against `MemoryStore`. New tests exercise `FjallStore` with
`#[ignore = "slow: requires fjall"]` for Tier 2.

Feature flag: `fjall-backend` (off by default). Users who want
persistence add `--features fjall-backend` to their build.
