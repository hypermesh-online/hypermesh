#!/bin/bash
# Test just the blockchain module in isolation

cd /home/persist/repos/projects/web3/blockmatrix

echo "Testing blockchain module..."
echo "=========================="

# Test individual blockchain module files
echo "Testing block.rs..."
rustc --edition 2021 --test src/blockchain/block.rs -L target/debug/deps 2>&1 | grep -E "error|test result" | head -10

echo ""
echo "Compiling blockchain module..."
rustc --edition 2021 --crate-type lib src/blockchain/mod.rs -L target/debug/deps 2>&1 | grep -E "error|warning" | head -10

echo ""
echo "Running cargo check on blockchain module..."
cargo check --lib 2>&1 | grep blockchain | head -20