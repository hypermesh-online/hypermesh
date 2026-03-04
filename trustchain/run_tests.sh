#!/bin/bash
# TrustChain Test Execution Script
# Run different categories of tests with proper isolation

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "========================================="
echo "TrustChain Test Suite Runner"
echo "========================================="

# Parse arguments
TEST_TYPE="${1:-all}"
VERBOSE="${2:-}"

# Function to run tests with retry logic for port conflicts
run_test_category() {
    local category=$1
    local pattern=$2
    local retries=3
    local delay=2

    echo -e "\n${YELLOW}Running $category tests...${NC}"

    for i in $(seq 1 $retries); do
        if [ "$VERBOSE" = "-v" ]; then
            cargo test $pattern -- --nocapture 2>&1 | tee "${category}_test.log"
        else
            cargo test $pattern 2>&1 | tee "${category}_test.log"
        fi

        if [ ${PIPESTATUS[0]} -eq 0 ]; then
            echo -e "${GREEN}✓ $category tests passed${NC}"
            return 0
        elif grep -q "Address already in use" "${category}_test.log"; then
            echo -e "${YELLOW}⚠ Port conflict detected, retrying in ${delay}s... (attempt $i/$retries)${NC}"
            sleep $delay
            delay=$((delay * 2))
        else
            echo -e "${RED}✗ $category tests failed${NC}"
            return 1
        fi
    done

    echo -e "${RED}✗ $category tests failed after $retries attempts${NC}"
    return 1
}

# Function to collect metrics
collect_metrics() {
    echo -e "\n${YELLOW}Collecting test metrics...${NC}"

    local total_tests=$(cargo test --lib 2>&1 | grep "running" | awk '{print $2}')
    local passed_tests=$(cargo test --lib 2>&1 | grep "test result" | sed -n 's/.*\([0-9]*\) passed.*/\1/p')
    local failed_tests=$(cargo test --lib 2>&1 | grep "test result" | sed -n 's/.*\([0-9]*\) failed.*/\1/p')

    echo "========================================="
    echo "Test Metrics Summary"
    echo "========================================="
    echo "Total tests: $total_tests"
    echo "Passed: $passed_tests"
    echo "Failed: $failed_tests"

    if [ "$failed_tests" = "0" ]; then
        echo -e "${GREEN}✓ All tests passing!${NC}"
        return 0
    else
        echo -e "${YELLOW}⚠ Some tests failing${NC}"
        return 1
    fi
}

# Main execution
case "$TEST_TYPE" in
    unit)
        echo "Running unit tests only..."
        run_test_category "unit" "--lib"
        ;;
    integration)
        echo "Running integration tests..."
        run_test_category "integration" "--test '*'"
        ;;
    perf|performance)
        echo "Running performance benchmarks..."
        if cargo bench --no-run 2>/dev/null; then
            cargo bench 2>&1 | tee performance_test.log
        else
            echo -e "${YELLOW}⚠ No benchmarks configured${NC}"
        fi
        ;;
    security)
        echo "Running security-related tests..."
        run_test_category "crypto" "crypto::"
        run_test_category "security" "security::"
        run_test_category "ca" "ca::"
        ;;
    proof_of_state)
        echo "Running Proof of State tests..."
        run_test_category "proof_of_state" "proof_of_state::"
        ;;
    dns)
        echo "Running DNS tests..."
        run_test_category "dns" "dns::"
        ;;
    metrics)
        collect_metrics
        ;;
    all)
        echo "Running all tests..."

        # Run tests sequentially to avoid port conflicts
        run_test_category "unit" "--lib" || true
        sleep 2

        # Run integration tests if they exist
        if ls tests/*.rs 2>/dev/null | grep -q .; then
            run_test_category "integration" "--test '*'" || true
        fi

        # Collect final metrics
        collect_metrics
        ;;
    *)
        echo "Usage: $0 [unit|integration|performance|security|proof_of_state|dns|metrics|all] [-v]"
        echo "  unit           - Run unit tests only"
        echo "  integration    - Run integration tests"
        echo "  performance    - Run performance benchmarks"
        echo "  security       - Run security-related tests"
        echo "  proof_of_state - Run Proof of State tests"
        echo "  dns            - Run DNS tests"
        echo "  metrics     - Show test metrics"
        echo "  all         - Run all tests (default)"
        echo "  -v          - Verbose output (show test output)"
        exit 1
        ;;
esac

echo -e "\n========================================="
echo "Test run complete!"
echo "========================================="