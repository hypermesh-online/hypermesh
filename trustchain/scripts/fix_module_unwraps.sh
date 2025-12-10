#!/bin/bash
# fix_module_unwraps.sh - Helper to fix unwraps in a specific module
# Usage: ./fix_module_unwraps.sh <module_name>

MODULE=$1
if [ -z "$MODULE" ]; then
    echo "Usage: $0 <module_name>"
    echo "Example: $0 ct/storage"
    exit 1
fi

echo "=== Fixing unwraps in $MODULE ==="

# Get file path
if [[ "$MODULE" == *"/"* ]]; then
    # Full path like ct/storage
    FILE="src/${MODULE}.rs"
else
    # Just module name
    FILE="src/${MODULE}/mod.rs"
fi

if [ ! -f "$FILE" ]; then
    echo "Error: File $FILE not found"
    exit 1
fi

# Count before
BEFORE=$(grep -c "\.unwrap()" "$FILE" | grep -v "test")
echo "Before: $BEFORE unwraps in $FILE"

# Show unwrap locations with context
echo ""
echo "Unwrap locations:"
grep -n "\.unwrap()" "$FILE" | while IFS=: read line_num content; do
    # Skip test code
    if echo "$content" | grep -q "#\[test\]\|#\[cfg(test)\]"; then
        continue
    fi
    echo "Line $line_num: $(echo $content | sed 's/^[ \t]*//' | cut -c1-80)"
done

echo ""
echo "Common patterns to apply:"
echo "1. path.to_str().unwrap() → path.to_str().ok_or_else(|| anyhow!(\"Invalid UTF-8\"))?"
echo "2. lock.lock().unwrap() → lock.lock().map_err(|e| anyhow!(\"Lock poisoned: {}\", e))?"
echo "3. value.parse().unwrap() → value.parse().context(\"Failed to parse value\")?"
echo "4. option.unwrap() → option.ok_or_else(|| anyhow!(\"Missing required value\"))?"

echo ""
echo "After fixing, verify with:"
echo "  cargo build --lib"
echo "  grep -c '\.unwrap()' $FILE"

# Create a backup
cp "$FILE" "${FILE}.backup"
echo ""
echo "Backup created: ${FILE}.backup"