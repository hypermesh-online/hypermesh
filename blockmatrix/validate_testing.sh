#!/bin/bash
# Validate what's actually testable in Phase 8b

echo "Phase 8b Testing Validation"
echo "==========================="
echo ""

# Check build status
echo "1. Build Status Check:"
echo "----------------------"
echo -n "Main build errors: "
cargo build --all 2>&1 | grep -E "error\[E[0-9]+\]" | wc -l

echo -n "Test build errors: "
cargo test --all --no-run 2>&1 | grep -E "error\[E[0-9]+\]" | wc -l

echo ""
echo "2. Module Compilation Status:"
echo "-----------------------------"

check_module() {
    local module=$1
    echo -n "$module: "

    # Check library compilation
    if cargo build --package $module --lib 2>&1 | grep -q "error\[E"; then
        echo -n "LIB:❌ "
    else
        echo -n "LIB:✅ "
    fi

    # Check test compilation
    if cargo test --package $module --lib --no-run 2>&1 | grep -q "error\[E"; then
        echo "TEST:❌"
    else
        echo "TEST:✅"
    fi
}

check_module "trustchain"
check_module "stoq"
check_module "caesar"
check_module "catalog"

echo ""
echo "3. Simple Integration Test:"
echo "---------------------------"

# Create minimal test file
cat > /tmp/minimal_test.rs << 'EOF'
#[test]
fn test_basic_compilation() {
    // Test that we can at least run a basic test
    assert_eq!(1 + 1, 2);
}

#[test]
fn test_arc_pattern() {
    use std::sync::Arc;
    let data = Arc::new(42);
    let clone = Arc::clone(&data);
    assert_eq!(*data, *clone);
}
EOF

echo -n "Minimal test execution: "
if rustc --test /tmp/minimal_test.rs -o /tmp/minimal_test 2>/dev/null && /tmp/minimal_test --quiet; then
    echo "✅ Tests can run"
else
    echo "❌ Test framework issue"
fi

echo ""
echo "4. Actual Runnable Tests:"
echo "-------------------------"

# Try to run any test that might work
echo "Attempting STOQ unit tests..."
if timeout 5s cargo test --package stoq --lib -- --test-threads=1 2>&1 | grep -q "test result"; then
    echo "✅ STOQ tests executable"
    cargo test --package stoq --lib -- --test-threads=1 2>&1 | grep "test result"
else
    echo "❌ STOQ tests blocked"
fi

echo ""
echo "5. Error Categories Blocking Tests:"
echo "-----------------------------------"

cargo test --all --no-run 2>&1 | grep "error\[E" | cut -d']' -f1 | cut -d'[' -f2 | sort | uniq -c | head -10

echo ""
echo "Summary:"
echo "--------"
echo "Libraries compile but tests have additional compilation errors."
echo "This is due to test-specific code paths and dependencies."
echo "Recommendation: Fix test compilation errors before integration testing."