#!/usr/bin/env bash
# elench continuous dogfooding — the real pipeline
#
# Runs elench against its own codebase: builds, tests, lints, formats.
# Emits harness-observed verification/falsification claims AND
# agent-asserted claims (premises, assumptions). Accumulates into a
# persistent fjall store so the gate evaluates against ALL history,
# not just the current run.
#
# The gate is real: if a load-bearing claim is falsified, this script
# exits non-zero. CI should fail on non-zero exit.
#
# Usage: ./dogfooding/emit-continuous.sh [store_dir]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ELENCH="$ROOT/target/debug/elench"
STORE="${1:-$ROOT/dogfooding/store}"

# Build elench WITH fjall backend (required for --store fjall).
# Do NOT run `cargo build` without features later — it overwrites
# the binary and strips the fjall feature.
if [ ! -f "$ELENCH" ] || ! "$ELENCH" --store fjall /tmp/elench-feature-check 2>&1 | grep -qv "requires the fjall-backend"; then
    echo "Building elench (--features elench/fjall-backend)..."
    (cd "$ROOT" && cargo build --features elench/fjall-backend)
fi
rm -rf /tmp/elench-feature-check

echo "=== elench continuous dogfooding ==="
echo "store:  $STORE"
echo

# ---------------------------------------------------------------------------
# 0. Store source tree so anchors resolve
# ---------------------------------------------------------------------------
echo "--- 0. Store source tree ---"
TREE_OUTPUT=$("$ELENCH" --store fjall "$STORE" store tree \
    "$ROOT/crates/elench-claim/src/lib.rs" \
    "$ROOT/crates/elench-store/src/lib.rs" \
    "$ROOT/crates/elench-envelope/src/lib.rs" \
    "$ROOT/crates/elench-gate/src/lib.rs" \
    "$ROOT/crates/elench-predicate/src/lib.rs" \
    "$ROOT/crates/elench-projection/src/lib.rs" \
    "$ROOT/crates/elench-anchor/src/lib.rs" \
    "$ROOT/crates/elench/src/main.rs" \
    2>&1)
TREE=$(echo "$TREE_OUTPUT" | grep '^tree:' | awk '{print $2}')
echo "  tree: $TREE"

if [ -z "$TREE" ]; then
    echo "FATAL: could not store source tree"
    exit 1
fi

NOW=$(date +%s)
CLAIMS_DIR="$STORE/claims"
mkdir -p "$CLAIMS_DIR"

emit() {
    # emit <kind> <text> <origin_kind> <producer_id> <timestamp> <path> [symbol]
    local kind="$1" text="$2" origin="$3" producer="$4" ts="$5" path="$6" symbol="${7:-null}"
    local claim_file="$CLAIMS_DIR/claim-$ts-$kind.json"
    cat > "$claim_file" <<ENDCLAIM
{"id":"cl_0000000000000000000000000000000000000000000000000000000000000000","kind":"$kind","target":[],"assertion":{"form":"annotation","text":"$text"},"origin":{"kind":"$origin","producer":{"id":"$producer","session_id":"ci-$NOW"}},"anchor":{"tree":"$TREE","strategy":"multi","path":"$path","range":null,"symbol":$symbol,"content_digest":null},"timestamp":$ts,"evidence":[],"depends_on":[]}
ENDCLAIM
    # Keep the claim file (with computed ID) for later gate/log/reconcile.
    # Harness-observed claims require --harness; agent-asserted don't.
    local harness_flag=""
    if [ "$origin" = "harness-observed" ]; then
        harness_flag="--harness"
    fi
    # elench emit prints the computed claim ID. We capture it, then
    # rewrite the file with the computed ID so gate/log/reconcile
    # see the actual claim (not the placeholder ID).
    local emit_output
    emit_output=$("$ELENCH" $harness_flag --store fjall "$STORE" emit "$claim_file" 2>&1)
    echo "$emit_output" | head -1
    local computed_id
    computed_id=$(echo "$emit_output" | grep '^  id:' | awk '{print $2}')
    if [ -n "$computed_id" ]; then
        sed -i "s/cl_0000000000000000000000000000000000000000000000000000000000000000/$computed_id/" "$claim_file"
    fi
}

# ---------------------------------------------------------------------------
# 1. Build provenance (harness-observed)
# ---------------------------------------------------------------------------
echo "--- 1. Build provenance ---"
if (cd "$ROOT" && cargo test --lib --no-run --quiet 2>&1); then
    echo "  build: PASS"
    emit verification "build: cargo test --lib --no-run exit=0" "harness-observed" "elench-ci" "$NOW" "Cargo.toml"
else
    echo "  build: FAIL"
    emit falsification "build: cargo test --lib --no-run FAILED" "harness-observed" "elench-ci" "$NOW" "Cargo.toml"
fi

# ---------------------------------------------------------------------------
# 2. Test provenance (harness-observed)
# ---------------------------------------------------------------------------
echo "--- 2. Test provenance ---"
TEST_OUTPUT=$(cd "$ROOT" && cargo test --lib 2>&1) || true
TEST_TS=$((NOW + 1))
if echo "$TEST_OUTPUT" | grep -q "test result: ok"; then
    TEST_COUNT=$(echo "$TEST_OUTPUT" | grep "test result" | awk '{s+=$4} END {print s}')
    echo "  tests: PASS ($TEST_COUNT passed)"
    emit verification "tests: cargo test --lib passed ($TEST_COUNT)" "harness-observed" "elench-ci" "$TEST_TS" "crates/elench-claim/src/lib.rs" '"compute_status"'
else
    echo "  tests: FAIL"
    emit falsification "tests: cargo test --lib FAILED" "harness-observed" "elench-ci" "$TEST_TS" "crates/elench-claim/src/lib.rs" '"compute_status"'
fi

# ---------------------------------------------------------------------------
# 3. Lint provenance (harness-observed)
# ---------------------------------------------------------------------------
echo "--- 3. Lint provenance ---"
LINT_TS=$((NOW + 2))
if (cd "$ROOT" && cargo clippy --all-targets -- -D warnings >/dev/null 2>&1); then
    echo "  lint: PASS"
    emit verification "lint: cargo clippy --all-targets -- -D warnings exit=0" "harness-observed" "elench-ci" "$LINT_TS" "crates/elench-claim/src/lib.rs" '"validate_claim"'
else
    echo "  lint: FAIL"
    emit falsification "lint: cargo clippy FAILED" "harness-observed" "elench-ci" "$LINT_TS" "crates/elench-claim/src/lib.rs" '"validate_claim"'
fi

# ---------------------------------------------------------------------------
# 4. Format check provenance (harness-observed)
# ---------------------------------------------------------------------------
echo "--- 4. Format check provenance ---"
FMT_TS=$((NOW + 3))
if (cd "$ROOT" && cargo fmt --all -- --check >/dev/null 2>&1); then
    echo "  fmt: PASS"
    emit verification "fmt: cargo fmt --all -- --check exit=0" "harness-observed" "elench-ci" "$FMT_TS" "Cargo.toml"
else
    echo "  fmt: FAIL"
    emit falsification "fmt: cargo fmt --all -- --check FAILED" "harness-observed" "elench-ci" "$FMT_TS" "Cargo.toml"
fi

# ---------------------------------------------------------------------------
# 5. Agent-asserted claim: INV-01 is enforced by store_blob
# ---------------------------------------------------------------------------
echo "--- 5. Agent-asserted claim ---"
AGENT_TS=$((NOW + 4))
emit assertion "INV-01: store_blob is idempotent (append-only)" "agent-asserted" "elench-dogfooding-agent" "$AGENT_TS" "crates/elench-store/src/lib.rs" '"store_blob"'

echo
echo "=== Claims emitted: 5 (4 harness-observed + 1 agent-asserted) ==="
echo "  store: $STORE"
echo "  tree:  $TREE"
echo

# ---------------------------------------------------------------------------
# 6. Gate — real, exits non-zero on falsified premise
# ---------------------------------------------------------------------------
# elench gate takes a claims file. We export all claims from the store
# by collecting the individual claim files we just emitted.
echo "--- 6. Gate ---"
ALL_CLAIMS="$STORE/all-claims.json"
echo '[' > "$ALL_CLAIMS"
FIRST=true
for f in "$CLAIMS_DIR"/claim-*.json; do
    [ -f "$f" ] || continue
    if [ "$FIRST" = true ]; then
        FIRST=false
    else
        echo ',' >> "$ALL_CLAIMS"
    fi
    cat "$f" >> "$ALL_CLAIMS"
done
echo ']' >> "$ALL_CLAIMS"

GATE_OUTPUT=$("$ELENCH" --store fjall "$STORE" gate "$TREE" "$ALL_CLAIMS" 2>&1)
GATE_EXIT=$?
echo "$GATE_OUTPUT"
echo

# ---------------------------------------------------------------------------
# 7. Reconcile — real, against the actual source tree
# ---------------------------------------------------------------------------
echo "--- 7. Reconcile ---"
"$ELENCH" --store fjall "$STORE" reconcile "$TREE" "$ALL_CLAIMS" 2>&1 || true
echo

# ---------------------------------------------------------------------------
# 8. Log statistics
# ---------------------------------------------------------------------------
echo "--- 8. Log statistics ---"
"$ELENCH" --store fjall "$STORE" log "$ALL_CLAIMS" 2>&1 || true
echo

# ---------------------------------------------------------------------------
# 9. Conflicts
# ---------------------------------------------------------------------------
echo "--- 9. Conflicts ---"
"$ELENCH" --store fjall "$STORE" conflicts "$TREE" "$ALL_CLAIMS" 2>&1 || true
echo

# ---------------------------------------------------------------------------
# Final: check gate verdict
# ---------------------------------------------------------------------------
if echo "$GATE_OUTPUT" | grep -q "result: Pass"; then
    echo "=== continuous dogfooding: PASS ==="
    exit 0
elif echo "$GATE_OUTPUT" | grep -q "result: Fail"; then
    echo "=== continuous dogfooding: FAIL (gate rejected) ==="
    exit 1
else
    echo "=== continuous dogfooding: UNKNOWN gate verdict ==="
    exit 1
fi
