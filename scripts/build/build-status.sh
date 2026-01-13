#!/bin/bash

echo "=== BUILD STATUS REPORT ==="
echo "Testing individual components..."
echo ""

components=("stoq" "trustchain" "catalog" "caesar" "hypermesh")
success_count=0
total_count=${#components[@]}

for pkg in "${components[@]}"; do
    echo -n "Building $pkg... "
    if cargo build --release -p "$pkg" 2>&1 | grep -q "Finished"; then
        echo "✓ SUCCESS"
        ((success_count++))
    else
        echo "✗ FAILED"
    fi
done

echo ""
echo "=== SUMMARY ==="
echo "Success: $success_count/$total_count components"
echo "Build Success Rate: $(( success_count * 100 / total_count ))%"