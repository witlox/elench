#!/usr/bin/env bash
# elench continuous dogfooding
#
# Emits claims about elench's own build, tests, and lint. Each step
# produces a harness-observed claim. The claims are accumulated and
# gated at the end.
#
# Usage: ./dogfooding/emit-continuous.sh [output_dir] [tree_oid]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ELENCH="$ROOT/target/debug/elench"
OUTPUT="${1:-$(mktemp -d -t elench-cont-XXXXXX)}"
TREE="${2:-$(sha256sum "$ROOT/Cargo.toml" | awk '{print $1}')}"

echo "=== elench continuous dogfooding ==="
mkdir -p "$OUTPUT"
echo "output:  $OUTPUT"
echo "tree:    $TREE"
echo

# Ensure elench is built
if [ ! -f "$ELENCH" ]; then
    echo "Building elench..."
    (cd "$ROOT" && cargo build)
fi

CLAIMS="$OUTPUT/claims-continuous.json"
FIRST=true

emit_claim() {
    if [ "$FIRST" = true ]; then
        FIRST=false
        printf '[' > "$CLAIMS"
    else
        printf ',' >> "$CLAIMS"
    fi

    local kind="$1" text="$2" origin="$3" producer="$4" ts="$5" path="$6" rs="$7" re="$8"

    local range="null"
    if [ -n "$rs" ] && [ -n "$re" ]; then
        range="[$rs,$re]"
    fi

    printf '{"id":"cl_0000000000000000000000000000000000000000000000000000000000000000","kind":"%s","target":[],"assertion":{"form":"annotation","text":"%s"},"origin":{"kind":"%s","producer":{"id":"%s"}},"anchor":{"tree":"%s","strategy":"multi","path":"%s","range":%s,"symbol":null,"content_digest":null},"timestamp":%s,"evidence":[],"depends_on":[]}' \
        "$kind" "$text" "$origin" "$producer" "$TREE" "$path" "$range" "$ts" >> "$CLAIMS"
}

NOW=$(date +%s)

# 1. Build provenance
echo "--- 1. Build provenance ---"
if (cd "$ROOT" && cargo build >/dev/null 2>&1); then
    echo "  build: PASS"
    emit_claim "verification" "build: cargo build exit=0" "harness-observed" "elench-ci-build" "$NOW" "Cargo.toml" "" ""
else
    echo "  build: FAIL"
    emit_claim "falsification" "build: cargo build FAILED" "harness-observed" "elench-ci-build" "$NOW" "Cargo.toml" "" ""
fi

# 2. Test provenance
echo "--- 2. Test provenance ---"
TEST_OUTPUT=$(cd "$ROOT" && cargo test --lib 2>&1) || true
if echo "$TEST_OUTPUT" | grep -q "test result: ok"; then
    TEST_COUNT=$(echo "$TEST_OUTPUT" | grep "test result" | awk '{s+=$4} END {print s}')
    echo "  tests: PASS ($TEST_COUNT passed)"
    emit_claim "verification" "tests: cargo test --lib passed ($TEST_COUNT)" "harness-observed" "elench-ci-test" "$((NOW + 1))" "crates/elench-claim/src/lib.rs" "" ""
else
    echo "  tests: FAIL"
    emit_claim "falsification" "tests: cargo test --lib FAILED" "harness-observed" "elench-ci-test" "$((NOW + 1))" "crates/elench-claim/src/lib.rs" "" ""
fi

# 3. Lint provenance
echo "--- 3. Lint provenance ---"
if (cd "$ROOT" && cargo clippy --all-targets -- -D warnings >/dev/null 2>&1); then
    echo "  lint: PASS"
    emit_claim "verification" "lint: cargo clippy --all-targets -- -D warnings exit=0" "harness-observed" "elench-ci-lint" "$((NOW + 2))" "Cargo.toml" "" ""
else
    echo "  lint: FAIL"
    emit_claim "falsification" "lint: cargo clippy FAILED" "harness-observed" "elench-ci-lint" "$((NOW + 2))" "Cargo.toml" "" ""
fi

# 4. Format check provenance
echo "--- 4. Format check provenance ---"
if (cd "$ROOT" && cargo fmt --all -- --check >/dev/null 2>&1); then
    echo "  fmt: PASS"
    emit_claim "verification" "fmt: cargo fmt --all -- --check exit=0" "harness-observed" "elench-ci-fmt" "$((NOW + 3))" "Cargo.toml" "" ""
else
    echo "  fmt: FAIL"
    emit_claim "falsification" "fmt: cargo fmt --all -- --check FAILED" "harness-observed" "elench-ci-fmt" "$((NOW + 3))" "Cargo.toml" "" ""
fi

printf ']' >> "$CLAIMS"

CLAIM_COUNT=$(grep -c '"kind":' "$CLAIMS" || true)
echo
echo "=== Claims emitted: $CLAIM_COUNT ==="
echo "  output: $CLAIMS"
echo

# 5. Gate
echo "--- 5. Gate ---"
"$ELENCH" gate "$TREE" "$CLAIMS" 2>&1 || true
echo

# 6. Log statistics
echo "--- 6. Log statistics ---"
"$ELENCH" log "$CLAIMS" 2>&1 || true
echo

# 7. Reconcile
echo "--- 7. Reconcile ---"
"$ELENCH" reconcile "$TREE" "$CLAIMS" 2>&1 || true
echo

# 8. Conflicts
echo "--- 8. Conflicts ---"
"$ELENCH" conflicts "$TREE" "$CLAIMS" 2>&1 || true
echo

echo "=== continuous dogfooding complete ==="
echo "  claims: $CLAIMS"
echo "  tree:   $TREE"
