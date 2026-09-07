# Getting Started

## Prerequisites

- **Rust** 1.85+ (edition 2024). `rust-toolchain.toml` pins stable
  with rustfmt and clippy.
- **make** (for the Makefile targets)
- **git** (for `elench git init` materialization)
- **cargo-llvm-cov** (optional, for coverage)
- **cargo-deny** (optional, for license/advisory checks)

```sh
git clone https://github.com/witlox/elench.git
cd elench
make                 # fmt-check + lint + Tier 1 tests
```

If `make` passes, the workspace is ready for development. If it does
not, run `make fmt` first (auto-format), then `make` again.

## Building

```sh
cargo build                              # debug build (default)
cargo build --release                    # release build
cargo build --features elench/fjall-backend  # with persistent fjall store
```

## Running

elench is a CLI tool. All commands take an optional `--store
memory|fjall <path>` flag (before the command) to select the storage
backend.

```sh
# Emit a claim (sign + store)
elench [--store memory|fjall <path>] emit <claim.json>

# Verify a DSSE envelope
elench verify <envelope.json>

# Compute a claim's status by folding the log
elench status <claim_id> [<claims.json>]

# Evaluate the release gate for a tree
elench gate <tree_oid> [<claims.json>]

# Compute blast radius from a claim
elench blast <claim_id> [<claims.json>]

# Materialize git projection (oneline or full)
elench git <claims.json>
elench git oneline <claims.json>
elench git full <claims.json>

# Materialize a real .git/ directory
elench git init <output_path> <claims.json>

# Store a blob or tree
elench store blob <file>
elench store tree <file1> <file2> ...

# Log statistics
elench log <claims.json>

# Review unevaluated claims
elench review <tree_oid> <claims.json>

# Accept named unevaluated gaps (residue-acceptance)
elench accept <tree_oid> <claims.json> --claim <id> [<id>...]

# List active predicate conflicts
elench conflicts <tree_oid> <claims.json>

# Compact the claim log (manual, destructive)
elench compact <claims.json> [--before <timestamp>]

# Reconcile anchors against a tree
elench reconcile <tree_oid> <claims.json>

# Create or verify a release artifact (INV-15)
elench artifact create <tree_oid> <policy_name> <sha256_digest>
elench artifact verify <artifact.json> <claims.json>

# Run a build, capture exit code + digest, emit provenance
elench build <tree_oid> [--artifact <path>] -- <command...>

# Version
elench version
```

## Testing

Three tiers, cascading. Each higher tier includes the lower.

| Tier | Command | What | When |
|------|---------|------|------|
| 1 (fast) | `make test` | `cargo test --lib` | Between every edit; pre-commit |
| 2 (slow) | `make test-slow` | Tier 1 + `cargo test --all-targets` | Pre-PR |
| 3 (full) | `make test-full` | Tier 2 + dogfooding e2e | Pre-merge / nightly |

```sh
make              # fmt-check + lint + Tier 1 (before every commit)
make test         # Tier 1
make test-slow    # Tier 2
make test-full    # Tier 3 (runs dogfooding pipeline)
make coverage     # cargo-llvm-cov --workspace --fail-under-lines 50
make dogfood      # full dogfooding pipeline
```

## CI

CI (`.github/workflows/ci.yml`) runs automatically:
- **Push** → Tier 1 (fmt-check + clippy + `cargo test --lib`)
- **PR** → Tier 1 + Tier 2 (`cargo test --all-targets`)
- **Nightly** → Tier 1 + Tier 2 + Tier 3 (fjall backend + coverage + dogfooding)
