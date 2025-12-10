#!/bin/bash
# run_test_category.sh - Run tests by category for focused debugging
# Usage: ./run_test_category.sh <category>

CATEGORY=$1

if [ -z "$CATEGORY" ]; then
    echo "Usage: $0 <category>"
    echo ""
    echo "Categories:"
    echo "  crypto    - Crypto module tests (kyber, falcon, hybrid)"
    echo "  ct        - Certificate transparency tests"
    echo "  dns       - DNS resolution tests"
    echo "  security  - Security and monitoring tests"
    echo "  consensus - Consensus validation tests"
    echo "  integration - Integration tests"
    echo "  all       - All tests"
    exit 1
fi

echo "=== Running $CATEGORY tests ==="
echo ""

case $CATEGORY in
    crypto)
        echo "Crypto tests:"
        cargo test --lib crypto:: -- --show-output 2>&1 | grep -E "test.*\.\.\.|PASSED|FAILED|error"
        ;;
    ct)
        echo "Certificate Transparency tests:"
        cargo test --lib ct:: -- --show-output 2>&1 | grep -E "test.*\.\.\.|PASSED|FAILED|error"
        ;;
    dns)
        echo "DNS tests:"
        cargo test --lib dns:: -- --show-output 2>&1 | grep -E "test.*\.\.\.|PASSED|FAILED|error"
        ;;
    security)
        echo "Security tests:"
        cargo test --lib security:: -- --show-output 2>&1 | grep -E "test.*\.\.\.|PASSED|FAILED|error"
        ;;
    consensus)
        echo "Consensus tests:"
        cargo test --lib consensus:: -- --show-output 2>&1 | grep -E "test.*\.\.\.|PASSED|FAILED|error"
        ;;
    integration)
        echo "Integration tests:"
        cargo test --lib tests:: -- --show-output 2>&1 | grep -E "test.*\.\.\.|PASSED|FAILED|error"
        ;;
    all)
        echo "All tests:"
        cargo test --lib -- --show-output
        ;;
    *)
        echo "Unknown category: $CATEGORY"
        exit 1
        ;;
esac

echo ""
echo "=== Test Summary ==="
cargo test --lib $CATEGORY 2>&1 | grep "test result:"