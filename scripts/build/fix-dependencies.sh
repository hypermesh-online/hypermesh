#!/bin/bash

# Fix dependency issues in Web3 workspace

echo "=== Web3 Workspace Dependency Fix Script ==="
echo ""

# Step 1: Clean build artifacts
echo "Step 1: Cleaning build artifacts..."
cargo clean

# Step 2: Update dependencies
echo "Step 2: Updating dependencies..."
cargo update

# Step 3: Check each component individually
echo "Step 3: Checking individual components..."
echo ""

components=("stoq" "trustchain" "caesar" "catalog" "hypermesh")

for component in "${components[@]}"; do
    echo "Checking $component..."
    cargo check -p $component 2>&1 | grep -E "error\[E" | head -5
    echo ""
done

# Step 4: Full workspace check
echo "Step 4: Full workspace check..."
cargo check --workspace 2>&1 | tail -20

echo ""
echo "=== Dependency Resolution Complete ==="