#!/usr/bin/env bash
set -euo pipefail

# Compute release version: YYYY.count(ADR).commitnr
#
#   YYYY        current year (e.g., 2026)
#   count(ADR)  number of non-template ADRs in specs/architecture/adr/
#   commitnr    git commit count (rev-list --count HEAD)
#
# The base version (e.g., 2026.8) is manually bumped for new version
# series — typically when the ADR count changes. The patch version is
# the commit count since the base was set, providing a unique,
# monotonically increasing version for every commit.
#
# Usage:
#   ./scripts/set-version.sh              # compute and set
#   ./scripts/set-version.sh --dry-run    # print current + computed, no write

DRY_RUN="${1:-}"

YEAR=$(date +%Y)
ADR_COUNT=$(ls specs/architecture/adr/*.md 2>/dev/null | grep -v template | wc -l)
COMMIT_COUNT=$(git rev-list --count HEAD)
BASE_VERSION="${YEAR}.${ADR_COUNT}"
FULL_VERSION="${BASE_VERSION}.${COMMIT_COUNT}"

echo "Year:        ${YEAR}"
echo "ADR count:   ${ADR_COUNT}"
echo "Commit count: ${COMMIT_COUNT}"
echo "Base version: ${BASE_VERSION}"
echo "Full version: ${FULL_VERSION}"

if [ "$DRY_RUN" = "--dry-run" ]; then
    CURRENT=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
    echo "Current in Cargo.toml: ${CURRENT}"
    echo "(dry run — no files modified)"
    exit 0
fi

# Patch workspace Cargo.toml — replace the version line
sed -i.bak "s/^version = \".*\"/version = \"${FULL_VERSION}\"/" Cargo.toml
rm -f Cargo.toml.bak

# Verify
NEW=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
if [ "$NEW" = "$FULL_VERSION" ]; then
    echo "  workspace: ${FULL_VERSION} ✓"
else
    echo "  workspace: FAILED (got ${NEW})"
    exit 1
fi

# Sync the lockfile's workspace-member versions ONLY. Never use
# `cargo generate-lockfile` — it re-resolves every third-party
# dependency to the newest compatible version, silently discarding
# committed pins.
cargo update --workspace --quiet

echo "Version set to ${FULL_VERSION}"
