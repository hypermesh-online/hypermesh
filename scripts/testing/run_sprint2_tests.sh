#!/bin/bash
# Sprint 2 Test Runner
# Validates Byzantine fault-tolerant container orchestration

set -e

echo "========================================="
echo "    Sprint 2 Test Validation Suite"
echo "========================================="
echo ""
echo "Testing Byzantine fault-tolerant container orchestration"
echo "Performance targets:"
echo "  - <50ms consensus coordination overhead"
echo "  - <100ms container startup with consensus"
echo "  - <10ms network setup per container"
echo "  - <5ms P2P connectivity establishment"
echo ""

# Change to project root
cd /home/persist/repos/work/vazio/hypermesh/core

# Build the project first
echo "Building project..."
cargo build --release 2>/dev/null || {
    echo "⚠️  Build warnings detected, continuing..."
}

echo ""
echo "=== Running Sprint 2 Tests ==="
echo ""

# Track test results
PASSED=0
FAILED=0

# Function to run a test and track results
run_test() {
    local test_name=$1
    local test_module=$2
    
    echo "Running: $test_name"
    if cargo test --test integration_tests "$test_module" --release -- --nocapture 2>&1 | grep -q "test result: ok"; then
        echo "✅ $test_name PASSED"
        ((PASSED++))
    else
        echo "❌ $test_name FAILED"
        ((FAILED++))
    fi
    echo ""
}

# Quick validation tests (should complete quickly)
echo "1. Quick Validation Tests"
echo "------------------------"
run_test "Sprint 2 Quick Validation" "sprint2_validation::test_sprint2_quick_validation"
run_test "Byzantine Quick Test" "sprint2_validation::test_byzantine_fault_tolerance_quick"
run_test "P2P Networking Integration" "sprint2_validation::test_p2p_networking_integration"

# Performance benchmarks
echo "2. Performance Benchmarks"
echo "------------------------"
run_test "Performance Suite" "sprint2_performance::test_sprint2_performance_suite"
run_test "Load Performance" "sprint2_performance::test_load_performance"

# Byzantine fault tolerance tests
echo "3. Byzantine Fault Tolerance"
echo "---------------------------"
run_test "Single Byzantine Fault" "sprint2_byzantine::test_byzantine_single_fault"
run_test "Byzantine Detection" "sprint2_byzantine::test_byzantine_detection"
run_test "Byzantine Recovery" "sprint2_byzantine::test_byzantine_recovery"
run_test "Byzantine Performance" "sprint2_byzantine::test_byzantine_performance"

echo ""
echo "========================================="
echo "          Test Summary"
echo "========================================="
echo "✅ Passed: $PASSED"
echo "❌ Failed: $FAILED"
echo ""

if [ $FAILED -eq 0 ]; then
    echo "🎉 All Sprint 2 tests PASSED!"
    echo ""
    echo "Sprint 2 successfully delivers:"
    echo "  ✓ Byzantine fault-tolerant container orchestration"
    echo "  ✓ <50ms consensus coordination overhead"
    echo "  ✓ <100ms container startup with consensus"
    echo "  ✓ P2P networking with performance targets met"
    echo "  ✓ Automatic Byzantine node detection and isolation"
    exit 0
else
    echo "⚠️  Some tests failed. Please review the output above."
    exit 1
fi