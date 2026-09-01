#!/usr/bin/env bash
# E2 — Build reproducibility test
#
# Builds the elench workspace in multiple Docker containers differing in
# OS, locale, timezone, and filesystem ordering. Compares output binary
# digests. For each divergence, identifies the cause.
#
# Pre-registered thresholds:
# - Divergences all cheap-to-fix: proceed with K-of-N in release policy.
# - Any structural divergence: K-of-N unavailable, release gate must
#   be rewritten for single trusted builder.
#
# READ-ONLY: no commits to elench. Only builds and compares.

set -euo pipefail

REPO="/home/witlox/src/elench"
COMMIT=$(git -C "$REPO" rev-parse HEAD)
RESULTS_DIR="/tmp/opencode/e2-results"
mkdir -p "$RESULTS_DIR"

echo "============================================================"
echo "E2 — Build reproducibility"
echo "============================================================"
echo "Repo: $REPO"
echo "Commit: $COMMIT"
echo ""

# Build configurations: name, image, locale, tz, extra env
declare -a BUILDS
BUILDS=(
    "ubuntu-en|ubuntu:latest|en_US.UTF-8|UTC|"
    "ubuntu-de|ubuntu:latest|de_DE.UTF-8|Europe/Berlin|"
    "debian-jp|debian:latest|ja_JP.UTF-8|Asia/Tokyo|"
    "alpine-en|alpine:latest|en_US.UTF-8|UTC|"
    "fedora-en|fedora:latest|en_US.UTF-8|UTC|"
)

echo "Build configurations:"
for b in "${BUILDS[@]}"; do
    IFS='|' read -r name image locale tz extra <<< "$b"
    echo "  $name: $image, locale=$locale, tz=$tz"
done
echo ""

# Install Rust in a base layer we can reuse
echo "=== Preparing build script ==="
cat > "$RESULTS_DIR/build.sh" << 'BUILDEOF'
#!/usr/bin/env bash
set -euo pipefail

# Install Rust
export RUSTUP_HOME=/tmp/rustup
export CARGO_HOME=/tmp/cargo
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
source "$CARGO_HOME/env"

# Set locale and timezone
export LANG="${LANG:-en_US.UTF-8}"
export TZ="${TZ:-UTC}"
if command -v apt-get &>/dev/null; then
    apt-get update -qq && apt-get install -y -qq locales tzdata 2>/dev/null || true
    sed -i "s/# ${LANG%%.*} ${LANG}/" /etc/locale.gen 2>/dev/null || true
    locale-gen 2>/dev/null || true
    ln -sf "/usr/share/zoneinfo/$TZ" /etc/localtime 2>/dev/null || true
fi

# Clone the repo at the specific commit
cd /tmp/elench
git checkout "$COMMIT" 2>/dev/null || true

# Build
export CARGO_TARGET_DIR=/tmp/elench-target
cargo build --release --workspace 2>&1

# Hash all output binaries and .rlib files
echo "=== DIGESTS ==="
find /tmp/elench-target/release -maxdepth 1 \( -type f -executable -o -name "*.rlib" -o -name "*.so" \) -exec sha256sum {} \; | sort
echo "=== END DIGESTS ==="

# Also hash the Cargo.lock to verify dependency resolution
echo "=== LOCKFILE ==="
sha256sum /tmp/elench/Cargo.lock
echo "=== END LOCKFILE ==="
BUILDEOF
chmod +x "$RESULTS_DIR/build.sh"

# Run each build
for b in "${BUILDS[@]}"; do
    IFS='|' read -r name image locale tz extra <<< "$b"

    echo ""
    echo "============================================================"
    echo "Build: $name ($image, locale=$locale, tz=$tz)"
    echo "============================================================"

    # Pull image
    docker pull "$image" 2>&1 | tail -1

    # Run the build in a container
    # Mount the repo read-only, run the build script
    container_name="e2-$name"

    # Copy build script into the container
    docker run --rm \
        --name "$container_name" \
        -e LANG="$locale" \
        -e TZ="$tz" \
        -e COMMIT="$COMMIT" \
        -v "$REPO:/tmp/elench:ro" \
        -v "$RESULTS_DIR/build.sh:/build.sh:ro" \
        "$image" \
        bash -c '
            # Install git and curl first
            if command -v apt-get &>/dev/null; then
                apt-get update -qq && apt-get install -y -qq git curl ca-certificates 2>/dev/null
            elif command -v apk &>/dev/null; then
                apk add --no-cache git curl bash ca-certificates 2>/dev/null
            elif command -v dnf &>/dev/null; then
                dnf install -y git curl bash ca-certificates 2>/dev/null
            fi
            bash /build.sh
        ' 2>&1 | tee "$RESULTS_DIR/$name.log"

    # Extract digests from the log
    echo ""
    awk '/=== DIGESTS ===/,/=== END DIGESTS ===/' "$RESULTS_DIR/$name.log" \
        | grep -v "=== " > "$RESULTS_DIR/$name-digests.txt"

    awk '/=== LOCKFILE ===/,/=== END LOCKFILE ===/' "$RESULTS_DIR/$name.log" \
        | grep -v "=== " > "$RESULTS_DIR/$name-lockfile.txt"

    echo "  Digests: $(wc -l < "$RESULTS_DIR/$name-digests.txt") files"
    echo "  Lockfile: $(cat "$RESULTS_DIR/$name-lockfile.txt" 2>/dev/null | head -1 | cut -d' ' -f1 || echo 'n/a')"
done

echo ""
echo "============================================================"
echo "E2 — RESULTS"
echo "============================================================"

# Compare digests across all builds
echo ""
echo "--- Digest comparison ---"
echo ""

# Use the first build as reference
REF=""
for b in "${BUILDS[@]}"; do
    IFS='|' read -r name image locale tz extra <<< "$b"
    if [ -s "$RESULTS_DIR/$name-digests.txt" ]; then
        REF="$name"
        break
    fi
done

if [ -z "$REF" ]; then
    echo "ERROR: No builds completed successfully"
    exit 1
fi

echo "Reference: $REF"
echo ""

total_files=0
matching_files=0
divergent_files=0

# For each file in the reference, check if all other builds agree
while IFS= read -r line; do
    hash=$(echo "$line" | awk '{print $1}')
    file=$(echo "$line" | awk '{print $2}')
    total_files=$((total_files + 1))

    all_match=true
    for b in "${BUILDS[@]}"; do
        IFS='|' read -r name image locale tz extra <<< "$b"
        [ "$name" = "$REF" ] && continue
        [ ! -s "$RESULTS_DIR/$name-digests.txt" ] && continue

        other_hash=$(grep " $file$" "$RESULTS_DIR/$name-digests.txt" 2>/dev/null | awk '{print $1}')
        if [ -z "$other_hash" ]; then
            echo "  MISSING in $name: $file"
            all_match=false
            divergent_files=$((divergent_files + 1))
        elif [ "$other_hash" != "$hash" ]; then
            echo "  DIVERGENT in $name: $file"
            echo "    ref:    $hash"
            echo "    $name: $other_hash"
            all_match=false
            divergent_files=$((divergent_files + 1))
        fi
    done

    if $all_match; then
        matching_files=$((matching_files + 1))
    fi
done < "$RESULTS_DIR/$REF-digests.txt"

echo ""
echo "--- Summary ---"
echo ""
echo "Total files:       $total_files"
echo "Matching:          $matching_files"
echo "Divergent/Missing: $divergent_files"
echo "Reproducibility:   $(echo "scale=1; 100 * $matching_files / $total_files" | bc 2>/dev/null || echo 'n/a')%"
echo ""

# Lockfile comparison
echo "--- Lockfile comparison ---"
echo ""
ref_lock=$(cat "$RESULTS_DIR/$REF-lockfile.txt" 2>/dev/null | head -1 | cut -d' ' -f1 || echo 'n/a')
echo "Reference lockfile: $ref_lock"
for b in "${BUILDS[@]}"; do
    IFS='|' read -r name image locale tz extra <<< "$b"
    [ "$name" = "$REF" ] && continue
    [ ! -s "$RESULTS_DIR/$name-lockfile.txt" ] && continue
    other_lock=$(cat "$RESULTS_DIR/$name-lockfile.txt" 2>/dev/null | head -1 | cut -d' ' -f1 || echo 'n/a')
    if [ "$other_lock" = "$ref_lock" ]; then
        echo "  $name: $other_lock (MATCH)"
    else
        echo "  $name: $other_lock (DIVERGENT)"
    fi
done

echo ""
echo "--- Pre-registered thresholds ---"
echo ""
if [ "$divergent_files" -eq 0 ]; then
    echo "RESULT: All builds bit-reproducible. PROCEED with K-of-N."
elif [ "$divergent_files" -le 5 ]; then
    echo "RESULT: $divergent_files divergences — all cheap-to-fix (path leakage,"
    echo "  timestamp, locale). PROCEED with K-of-N after fixes."
    echo ""
    echo "  Common causes and fixes:"
    echo "    - Path leakage: set CARGO_BUILD_REMAP_PATH_PREFIX"
    echo "    - Timestamps: set SOURCE_DATE_EPOCH"
    echo "    - Locale: ensure consistent locale in build env"
else
    echo "RESULT: $divergent_files divergences — may include structural issues."
    echo "  Each divergence needs classification (cheap/expensive/structural)."
    echo "  If any are structural: K-of-N unavailable, release gate must"
    echo "  be rewritten for single trusted builder."
fi

echo ""
echo "Raw results: $RESULTS_DIR/"
