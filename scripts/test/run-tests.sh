#!/bin/bash

# Comprehensive Test Execution Script for Web3 Ecosystem
# Runs all testing phases and generates reports

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PARALLEL=${PARALLEL:-true}
TIMEOUT=${TIMEOUT:-300}
RETRIES=${RETRIES:-3}
OUTPUT_DIR=${OUTPUT_DIR:-"test-results"}

# Create output directory
mkdir -p $OUTPUT_DIR

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Web3 Ecosystem Testing Framework${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Function to run a test phase
run_phase() {
    local phase_name=$1
    local command=$2

    echo -e "${YELLOW}Running: $phase_name${NC}"

    if eval $command; then
        echo -e "${GREEN}✓ $phase_name passed${NC}\n"
        return 0
    else
        echo -e "${RED}✗ $phase_name failed${NC}\n"
        return 1
    fi
}

# Track failures
FAILED_TESTS=()

# Phase 1: Code Quality
echo -e "${BLUE}Phase 1: Code Quality Checks${NC}"
echo "==============================="

if ! run_phase "Format Check" "cargo fmt --all -- --check"; then
    FAILED_TESTS+=("Format Check")
fi

if ! run_phase "Clippy Analysis" "cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tee $OUTPUT_DIR/clippy.log"; then
    FAILED_TESTS+=("Clippy Analysis")
fi

# Phase 2: Compilation
echo -e "${BLUE}Phase 2: Build Validation${NC}"
echo "=========================="

if ! run_phase "Debug Build" "cargo build --all-features"; then
    FAILED_TESTS+=("Debug Build")
    echo -e "${RED}Build failed - skipping remaining tests${NC}"
    exit 1
fi

if ! run_phase "Release Build" "cargo build --release --all-features"; then
    FAILED_TESTS+=("Release Build")
fi

# Phase 3: Unit Tests
echo -e "${BLUE}Phase 3: Unit Testing${NC}"
echo "======================"

for component in stoq trustchain hypermesh caesar catalog; do
    if ! run_phase "$component Unit Tests" "cargo test --package $component --lib -- --nocapture 2>&1 | tee $OUTPUT_DIR/unit-$component.log"; then
        FAILED_TESTS+=("$component Unit Tests")
    fi
done

# Phase 4: Integration Tests
echo -e "${BLUE}Phase 4: Integration Testing${NC}"
echo "============================="

if ! run_phase "Integration Tests" "cargo test --test '*' --all-features -- --nocapture 2>&1 | tee $OUTPUT_DIR/integration.log"; then
    FAILED_TESTS+=("Integration Tests")
fi

# Phase 5: Security Testing
echo -e "${BLUE}Phase 5: Security Validation${NC}"
echo "============================="

# Install security tools if not present
if ! command -v cargo-audit &> /dev/null; then
    echo "Installing cargo-audit..."
    cargo install cargo-audit
fi

if ! run_phase "Security Audit" "cargo audit 2>&1 | tee $OUTPUT_DIR/security-audit.log"; then
    FAILED_TESTS+=("Security Audit")
fi

# Run custom security tests
if ! run_phase "Cryptographic Validation" "cargo test test_cryptographic_implementations --package web3 -- --nocapture"; then
    FAILED_TESTS+=("Cryptographic Validation")
fi

# Phase 6: Performance Testing
echo -e "${BLUE}Phase 6: Performance Benchmarks${NC}"
echo "================================"

if ! run_phase "Performance Benchmarks" "cargo bench --all 2>&1 | tee $OUTPUT_DIR/benchmarks.log"; then
    FAILED_TESTS+=("Performance Benchmarks")
fi

# Phase 7: Test Coverage (optional)
if command -v cargo-tarpaulin &> /dev/null; then
    echo -e "${BLUE}Phase 7: Code Coverage${NC}"
    echo "======================"

    if ! run_phase "Coverage Analysis" "cargo tarpaulin --all-features --workspace --out Html --output-dir $OUTPUT_DIR"; then
        FAILED_TESTS+=("Coverage Analysis")
    fi
else
    echo -e "${YELLOW}Skipping coverage analysis (cargo-tarpaulin not installed)${NC}"
fi

# Phase 8: Production Readiness
echo -e "${BLUE}Phase 8: Production Readiness Check${NC}"
echo "===================================="

# Check if production validation tests exist
if [ -f "tests/validation.rs" ]; then
    if ! run_phase "Production Validation" "cargo test validate_production_readiness --package web3 -- --nocapture 2>&1 | tee $OUTPUT_DIR/validation.log"; then
        FAILED_TESTS+=("Production Validation")
    fi
fi

# Generate Summary Report
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}           TEST SUMMARY${NC}"
echo -e "${BLUE}========================================${NC}"

TOTAL_TESTS=$((8 - ${#FAILED_TESTS[@]}))

if [ ${#FAILED_TESTS[@]} -eq 0 ]; then
    echo -e "${GREEN}All tests passed! ✓${NC}"
    echo -e "Total: 8/8 tests passed"

    # Generate success report
    cat > $OUTPUT_DIR/summary.txt << EOF
Web3 Ecosystem Test Results
==========================
Status: SUCCESS
Date: $(date)
Total Tests: 8
Passed: 8
Failed: 0

All quality gates passed.
System is ready for deployment.
EOF

    exit 0
else
    echo -e "${RED}Some tests failed ✗${NC}"
    echo -e "Total: $TOTAL_TESTS/8 tests passed"
    echo -e "\n${RED}Failed tests:${NC}"
    for test in "${FAILED_TESTS[@]}"; do
        echo -e "  • $test"
    done

    # Generate failure report
    cat > $OUTPUT_DIR/summary.txt << EOF
Web3 Ecosystem Test Results
==========================
Status: FAILED
Date: $(date)
Total Tests: 8
Passed: $TOTAL_TESTS
Failed: ${#FAILED_TESTS[@]}

Failed Tests:
$(for test in "${FAILED_TESTS[@]}"; do echo "  - $test"; done)

Please review the logs in $OUTPUT_DIR for details.
EOF

    exit 1
fi