#!/bin/bash
# verify_todos.sh - Track TODO/FIXME cleanup progress

echo "=== TODO/FIXME Cleanup Progress ==="
echo "Date: $(date)"
echo ""

# Categorize TODOs
echo "Critical TODOs (requiring implementation):"
grep -rn "TODO" src/ --include="*.rs" | grep -v "test" | grep -E "(Implement|Parse|Extract|Replace)" | while IFS=: read file line content; do
    printf "%s:%d: %s\n" "$(basename $(dirname $file))/$(basename $file)" "$line" "$(echo $content | cut -c1-60)..."
done

echo ""
echo "Enhancement TODOs (can defer):"
grep -rn "TODO" src/ --include="*.rs" | grep -v "test" | grep -vE "(Implement|Parse|Extract|Replace)" | while IFS=: read file line content; do
    printf "%s:%d: %s\n" "$(basename $(dirname $file))/$(basename $file)" "$line" "$(echo $content | cut -c1-60)..."
done

echo ""
# Count by module
echo "TODO distribution by module:"
for module in ct dns crypto security api proof_of_state ca monitoring; do
    count=$(grep -r "TODO\|FIXME" src/$module --include="*.rs" 2>/dev/null | grep -v "test" | wc -l)
    if [ $count -gt 0 ]; then
        printf "%-15s: %2d TODOs\n" "$module" "$count"
    fi
done

echo ""
# Total counts
TODOS=$(grep -rn "TODO" src/ --include="*.rs" | grep -v "test" | wc -l)
FIXMES=$(grep -rn "FIXME" src/ --include="*.rs" | grep -v "test" | wc -l)
TOTAL=$((TODOS + FIXMES))

echo "Summary:"
echo "  TODOs: $TODOS"
echo "  FIXMEs: $FIXMES"
echo "  Total: $TOTAL"

echo ""
echo "Target: 0 critical TODOs, all others documented"
if [ $TOTAL -eq 0 ]; then
    echo "🎉 ALL TODOs RESOLVED!"
else
    echo "📊 Progress: $((26 - TOTAL))/26 resolved ($((100 * (26 - TOTAL) / 26))%)"
fi