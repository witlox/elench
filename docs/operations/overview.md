# Operations

Operational reference for elench.

## Durability

elench's claim log is append-only (INV-01). The in-memory store
(default) does not persist across process restarts. The fjall backend
(`--store fjall <path>`) persists claims, blobs, and trees to disk.

### Default (in-memory)

| Operation | Durability |
|-----------|------------|
| `elench emit` | Claim stored in memory; lost on process exit |
| `elench gate` | Live evaluation; no persistence needed |
| `elench git init` | Writes real git objects to disk (zlib-compressed) |

### Fjall backend

| Operation | Durability |
|-----------|------------|
| `elench --store fjall <path> emit` | Claim persisted to disk (SyncAll) |
| `elench --store fjall <path> store blob` | Blob persisted to disk (Buffer) |
| `elench --store fjall <path> store tree` | Tree persisted to disk (Buffer) |

Claims use `PersistMode::SyncAll` (must survive crashes). Blobs and
trees use `PersistMode::Buffer` (content-addressed, can be recomputed
from source).

## Compaction

The claim log grows without bound on an active repository. Compaction
is manual and destructive:

```sh
elench compact <claims.json> --before <timestamp>
```

Retires all claims before the cut-off, freezing their statuses. The
compaction record carries the frozen status snapshot forward. Active
claims continue to be revocable (R1 preserved for active claims,
deliberately violated for retired ones).

## Reconciliation

After a tree change (rename, reformat, semantic edit), claims may no
longer resolve to the correct code. The reconciliation pass detects
this:

```sh
elench reconcile <tree_oid> <claims.json>
```

For each claim anchored to the tree, resolves the anchor using the
multi strategy (path-range, symbol, content-digest). Claims that fail
to resolve or are degraded are reported as drifted. The
reconciliation pass is read-only — it does not auto-fix. The human
(or agent) must re-anchor or falsify drifted claims.

## Conflict detection

Two predicates are a conflict if and only if they anchor to the
**same code location** (same path+range, or same symbol) but assert
**different expressions**. Last-writer-wins by timestamp; the older
claim is flagged for resolution.

```sh
elench conflicts <tree_oid> <claims.json>
```

## Build provenance

```sh
elench build <tree_oid> [--artifact <path>] -- <command...>
```

Runs the build command, captures the exit code, and computes a
SHA-256 digest. With `--artifact <path>`, the digest is SHA-256 of
that file (the real build output). Without it, the digest falls back
to SHA-256 of stdout. Emits a build provenance claim with
`origin.kind = harness-observed` (INV-22).
