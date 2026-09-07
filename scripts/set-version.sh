#!/usr/bin/env bash
set -euo pipefail

# Bump the workspace version. Single source of truth — patches
# Cargo.toml and all internal crate references.
#
# Usage:
#   ./scripts/set-version.sh <version>      # set version
#   ./scripts/set-version.sh --dry-run      # print current

DRY_RUN="${1:-}"

if [ "$DRY_RUN" = "--dry-run" ]; then
    CURRENT=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
    echo "Current version: ${CURRENT}"
    exit 0
fi

if [ -z "$DRY_RUN" ]; then
    echo "Usage: ./scripts/set-version.sh <version>"
    echo "       ./scripts/set-version.sh --dry-run"
    exit 1
fi

VERSION="$DRY_RUN"
echo "Setting version to ${VERSION}"

# Patch workspace Cargo.toml
sed -i.bak "s/^version = \".*\"/version = \"${VERSION}\"/" Cargo.toml
rm -f Cargo.toml.bak

# Verify
NEW=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
if [ "$NEW" = "$VERSION" ]; then
    echo "  workspace: ${VERSION} ✓"
else
    echo "  workspace: FAILED (got ${NEW})"
    exit 1
fi

cargo check --quiet 2>&1 | tail -3 || true
echo "Done. Run 'make' to verify."
