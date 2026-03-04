#!/bin/bash
# verify_unwraps.sh - Track unwrap() elimination progress

echo "=== Unwrap Elimination Progress ==="
echo "Date: $(date)"
echo ""

# Per-module breakdown
echo "Module Breakdown:"
for module in ct dns crypto security api proof_of_state ca monitoring http3; do
    count=$(grep -r "unwrap()" src/$module --include="*.rs" 2>/dev/null | grep -v "test" | wc -l)
    printf "%-15s: %3d unwraps\n" "$module" "$count"
done

echo ""
echo "High-density files (>5 unwraps):"
grep -r "unwrap()" src/ --include="*.rs" | grep -v "test" | cut -d: -f1 | sort | uniq -c | sort -rn | while read count file; do
    if [ $count -gt 5 ]; then
        printf "%3d: %s\n" "$count" "$file"
    fi
done

echo ""
# Total count
TOTAL=$(grep -r "unwrap()" src/ --include="*.rs" | grep -v "test" | wc -l)
echo "Total unwraps in production code: $TOTAL"

# Check if build still works
echo ""
echo "Build check:"
if cargo build --lib 2>&1 | grep -q "error"; then
    echo "❌ Build has errors"
    cargo build --lib 2>&1 | grep "error" | head -5
else
    echo "✅ Build successful"
fi

# Target
echo ""
echo "Target: 0 unwraps in production code"
if [ $TOTAL -eq 0 ]; then
    echo "🎉 TARGET ACHIEVED!"
else
    echo "📊 Progress: $((371 - TOTAL))/371 eliminated ($((100 * (371 - TOTAL) / 371))%)"
fi