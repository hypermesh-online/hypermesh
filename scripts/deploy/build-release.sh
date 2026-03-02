#!/usr/bin/env bash
# Build HyperMesh release binaries for deployment.
# Usage: ./scripts/deploy/build-release.sh [--target <triple>]
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO_ROOT"

TARGET="${1:---native}"
if [ "$TARGET" = "--native" ]; then
    echo "Building release binaries (native)..."
    cargo build --release -p gateway -p trustchain -p blockmatrix -p catalog
else
    shift
    echo "Building release binaries (target: $TARGET)..."
    cargo build --release --target "$TARGET" -p gateway -p trustchain -p blockmatrix -p catalog
fi

# List built binaries
echo ""
echo "=== Built binaries ==="
for bin in gateway trustchain_ca node catalog-server; do
    if [ "$TARGET" = "--native" ] || [ -z "${1:-}" ]; then
        path="target/release/$bin"
    else
        path="target/$TARGET/release/$bin"
    fi
    if [ -f "$path" ]; then
        size=$(du -h "$path" | cut -f1)
        echo "  $bin ($size) → $path"
    else
        echo "  $bin — NOT FOUND at $path"
    fi
done
echo ""
echo "Build complete."
