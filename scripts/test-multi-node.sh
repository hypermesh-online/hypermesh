#!/bin/bash
#
# Test Multi-Node Communication using existing binaries
#

set -e

echo "Testing multi-node communication..."

# Build the blockmatrix main binary which has networking capabilities
cd /home/persist/repos/projects/web3/blockmatrix
echo "Building blockmatrix binary..."
cargo build --bin blockmatrix 2>&1 | tail -2

# Check if binary exists
if [ ! -f "target/debug/blockmatrix" ]; then
    echo "Failed to build blockmatrix binary"
    exit 1
fi

echo "Binary built successfully at target/debug/blockmatrix"
ls -la target/debug/blockmatrix

# The main blockmatrix binary can be used for testing
echo ""
echo "To test multi-node:"
echo "1. Run STOQ transport tests to verify connectivity"
echo "2. Use blockmatrix library tests for multi-node scenarios"
echo ""

# Run multi-node integration tests instead
echo "Running multi-node integration tests..."
cargo test --test multi_node_integration --lib 2>&1 | tail -20