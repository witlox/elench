#!/usr/bin/env bash
# elench dogfooding pipeline
#
# Emits claims about elench's own invariants, gates them, reconciles
# them, and materializes a git projection. This is the closed loop:
# elench records what was checked about itself.
#
# Usage: ./dogfooding/run.sh [output_dir]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ELENCH="$ROOT/target/debug/elench"
OUTPUT="${1:-$(mktemp -d -t elench-dogfood-XXXXXX)}"

echo "=== elench dogfooding pipeline ==="
echo "output: $OUTPUT"
echo

# Build elench if needed
if [ ! -f "$ELENCH" ]; then
    echo "Building elench..."
    (cd "$ROOT" && cargo build)
fi

# 1. Emit each claim (sign + store)
echo "--- 1. Emit claims ---"
STORE="$OUTPUT/store"
mkdir -p "$STORE"

# Use the elench CLI to emit the first claim (demonstrates the pipeline)
"$ELENCH" emit "$ROOT/dogfooding/claims.json" 2>&1 | head -10 || true
echo "(all ${#} claims in claims.json)"
echo

# 2. Gate the dogfooding tree
echo "--- 2. Gate (evaluate against claim log) ---"
"$ELENCH" gate "0000000000000000000000000000000000000000000000000000000000000000" "$ROOT/dogfooding/claims.json" 2>&1
echo

# 3. Log statistics
echo "--- 3. Log statistics ---"
"$ELENCH" log "$ROOT/dogfooding/claims.json" 2>&1
echo

# 4. Review unevaluated claims
echo "--- 4. Review unevaluated ---"
"$ELENCH" review "0000000000000000000000000000000000000000000000000000000000000000" "$ROOT/dogfooding/claims.json" 2>&1
echo

# 5. Check for conflicts
echo "--- 5. Conflicts ---"
"$ELENCH" conflicts "0000000000000000000000000000000000000000000000000000000000000000" "$ROOT/dogfooding/claims.json" 2>&1
echo

# 6. Project to git log
echo "--- 6. Git projection (oneline) ---"
"$ELENCH" git "$ROOT/dogfooding/claims.json" 2>&1
echo

# 7. Materialize real git objects
echo "--- 7. Materialize .git/ ---"
GIT_DIR="$OUTPUT/git"

# The materialize step requires tree data in the store. We build a
# minimal tree from the actual source files referenced by the claims,
# store it in a fjall-backed store, then materialize.
STORE_DIR="$OUTPUT/store-db"
(cd "$ROOT" && "$ELENCH" --store fjall "$STORE_DIR" store tree \
    crates/elench-store/src/lib.rs \
    crates/elench-claim/src/lib.rs \
    crates/elench-projection/src/lib.rs \
    crates/elench-gate/src/lib.rs \
    crates/elench-envelope/src/lib.rs \
    2>&1 | head -5)

# Get the tree OID from the store output
TREE_OID=$(cd "$ROOT" && "$ELENCH" --store fjall "$STORE_DIR" store tree \
    crates/elench-store/src/lib.rs \
    crates/elench-claim/src/lib.rs \
    crates/elench-projection/src/lib.rs \
    crates/elench-gate/src/lib.rs \
    crates/elench-envelope/src/lib.rs \
    2>&1 | grep '^tree:' | awk '{print $2}')

if [ -n "$TREE_OID" ]; then
    echo "tree: $TREE_OID"
    # Rewrite claims to point at the real tree OID
    sed "s/0000000000000000000000000000000000000000000000000000000000000000/$TREE_OID/g" \
        "$ROOT/dogfooding/claims.json" > "$OUTPUT/claims-real.json"
    "$ELENCH" --store fjall "$STORE_DIR" git init "$GIT_DIR" "$OUTPUT/claims-real.json" 2>&1
else
    echo "(could not build tree — skipping materialization)"
fi
echo

if [ -d "$GIT_DIR/.git" ]; then
    echo "--- 8. Verify: git log ---"
    git -C "$GIT_DIR" log --oneline 2>&1
    echo

    echo "--- 9. Verify: git fsck ---"
    git -C "$GIT_DIR" fsck 2>&1
    echo
fi

echo "=== dogfooding complete ==="
echo "output: $OUTPUT"
