#!/usr/bin/env bash
# Build HyperMesh release binaries for deployment.
# Default: static musl binaries that run on ANY x86-64 Linux (no glibc dependency).
#
# Usage:
#   ./scripts/deploy/build-release.sh             # static musl (default, most portable)
#   ./scripts/deploy/build-release.sh --native     # dev machine, uses .cargo/config.toml as-is
#   ./scripts/deploy/build-release.sh --target <triple>  # cross-compile
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO_ROOT"

CRATES="-p gateway -p trustchain -p blockmatrix -p catalog -p engauge"
MODE="${1:---portable}"

case "$MODE" in
    --native)
        echo "Building release binaries (native — using .cargo/config.toml)..."
        cargo build --release $CRATES
        # engauge-server binary requires 'server' feature
        cargo build --release -p engauge --features engauge/server
        BINDIR="target/release"
        ;;
    --portable)
        echo "Building portable static release binaries (musl, x86-64)..."
        # Static musl build: zero glibc dependency, runs on any x86-64 Linux.
        # C_INCLUDE_PATH needed for libbpf-sys kernel headers.
        export RUSTFLAGS="-C target-cpu=x86-64 -C force-frame-pointers=yes"
        export C_INCLUDE_PATH=/usr/include
        cargo build --release --target x86_64-unknown-linux-musl $CRATES
        # engauge-server binary requires 'server' feature
        cargo build --release --target x86_64-unknown-linux-musl -p engauge --features engauge/server
        BINDIR="target/x86_64-unknown-linux-musl/release"
        ;;
    --target)
        TARGET="${2:?Usage: build-release.sh --target <triple>}"
        echo "Building release binaries (target: $TARGET)..."
        export RUSTFLAGS="-C target-cpu=x86-64 -C force-frame-pointers=yes"
        export C_INCLUDE_PATH=/usr/include
        cargo build --release --target "$TARGET" $CRATES
        # engauge-server binary requires 'server' feature
        cargo build --release --target "$TARGET" -p engauge --features engauge/server
        BINDIR="target/$TARGET/release"
        ;;
    *)
        echo "Usage: build-release.sh [--portable|--native|--target <triple>]"
        exit 1
        ;;
esac

# List built binaries
echo ""
echo "=== Built binaries ==="
for bin in gateway trustchain_ca hypermesh catalog-server engauge-server; do
    path="$BINDIR/$bin"
    if [ -f "$path" ]; then
        size=$(du -h "$path" | cut -f1)
        echo "  $bin ($size) → $path"
    else
        echo "  $bin — NOT FOUND at $path"
    fi
done

# Portability check for portable/target builds
if [ "$MODE" != "--native" ]; then
    echo ""
    echo "=== Portability check ==="
    for bin in gateway trustchain_ca hypermesh catalog-server engauge-server; do
        path="$BINDIR/$bin"
        if [ -f "$path" ]; then
            if file "$path" | grep -q "static"; then
                echo "  $bin: OK (static-pie, runs on any Linux)"
            elif file "$path" | grep -q "x86-64"; then
                echo "  $bin: OK (x86-64 ELF, dynamically linked)"
            else
                echo "  $bin: WARNING — unexpected format"
            fi
        fi
    done
fi

echo ""
echo "Build complete."
