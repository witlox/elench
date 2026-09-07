# elench — Next Steps

**Last commit:** C4: Dogfooding — elench eats its own dog food
**State:** 230 tests (default), 237 tests (with fjall-backend). 83% line coverage. fmt clean, clippy clean. 7 crates.

All implementation tasks (A1, A2, B1, B2, C1, C2, C3, C4) are
complete. elench is a working evidence layer.

## Completed

- A1: Real Ed25519 crypto via ed25519-dalek 3.0
- A2: --store CLI flag + FjallStore.read_tree
- B2: Build provenance digest — actual artifact (not stdout)
- B1: Anchor resolution — actually search trees
- C2: proptest — property-based tests
- C3: CI — .github/workflows/ci.yml
- C1: Git .git/ materialization — write real git objects
- C4: Dogfooding — elench eats its own dog food
  - `dogfooding/claims.json`: 10 claims about elench's own invariants.
  - `dogfooding/run.sh`: full pipeline — emit, gate, log, review,
    conflicts, git projection, materialize real `.git/`, verify with
    `git log` and `git fsck`.
  - `make dogfood` runs the full pipeline. Individual targets:
    `dogfood-emit`, `dogfood-gate`, `dogfood-reconcile`,
    `dogfood-project`.
  - `specs/features/dogfooding.feature` (5 scenarios).
  - The closed loop: elench records what was checked about itself,
    gates those records live, and materializes a git projection that
    `git log` and `git checkout` can read.

## Remaining (ongoing / future)

- **INV-15** (artifact format): still NONE depth. The artifact carries
  `(tree, policy)` but a formal serialization format is not yet defined.
- **Coverage**: elench-predicate (80%) and elench-envelope (82%) are
  the lowest. The binary (`elench/src/main.rs`, 42%) has many error
  paths that are under-exercised.
- **Dogfooding**: ongoing. The initial 10 claims are a proof of
  concept; a full dogfooding regime would record claims continuously as
  agents work on elench.

## Key files

- `AGENTS.md` — project state, workflow router, harness contract
- `specs/fidelity/INDEX.md` — test depth per invariant
- `specs/architecture/enforcement-map.md` — enforcement status
- `specs/architecture/build-phases.md` — phase status (all COMPLETE)
- `specs/architecture/adr/` — ADRs 0001-0008
- `specs/features/` — 11 feature files, 69 scenarios
- `dogfooding/` — claims, run script, README
- `crates/elench-store/src/lib.rs` — StoreBackend trait, MemoryStore
- `crates/elench-store/src/fjall_backend.rs` — FjallStore (optional)
- `crates/elench-projection/src/lib.rs` — synthesize + materialize
- `crates/elench/src/main.rs` — CLI (15 commands)

## Build commands

```
make              # fmt-check + lint + Tier 1 (before every commit)
make test         # Tier 1: cargo test --lib
make test-slow    # Tier 2: Tier 1 + cargo test --all-targets
make test-full    # Tier 3: Tier 2 + dogfooding e2e
make coverage     # cargo llvm-cov --workspace --fail-under-lines 50
make dogfood      # full dogfooding pipeline
```

### Feature flag

```
cargo test --workspace --all-targets --features elench/fjall-backend
```

Runs the persistent-store tests (`FjallStore::read_tree`, cross-reopen,
`--store fjall <path>` materialization, `interaction_7_..._fjall`).
