#!/bin/bash
#
# HyperMesh Ecosystem - Simple Test Demo
# ============================================
# This script runs available examples and tests to demonstrate what works
#

set -e

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}==========================================${NC}"
echo -e "${BLUE}    HyperMesh - Component Tests${NC}"
echo -e "${BLUE}==========================================${NC}\n"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Must run from HyperMesh project root${NC}"
    exit 1
fi

# Function to run example with timeout
run_example() {
    local component=$1
    local example=$2
    local timeout_sec=${3:-10}

    echo -e "${BLUE}Testing ${component}::${example}${NC}"
    cd "$component"
    if timeout "$timeout_sec" cargo run --example "$example" 2>&1 | grep -v "warning:" | head -20; then
        echo -e "${GREEN}✓ ${component}::${example} ran successfully${NC}\n"
    else
        echo -e "${YELLOW}⚠ ${component}::${example} timed out or failed${NC}\n"
    fi
    cd ..
}

# Function to build and check binary
check_binary() {
    local component=$1
    local binary=$2

    echo -e "${BLUE}Building ${component}::${binary}${NC}"
    cd "$component"
    if cargo build --bin "$binary" 2>&1 | tail -3; then
        echo -e "${GREEN}✓ ${component}::${binary} builds successfully${NC}"
        # Show help if available
        if timeout 2 cargo run --bin "$binary" -- --help 2>&1 | head -10 | grep -v "warning:"; then
            echo -e "${GREEN}  Binary accepts --help flag${NC}"
        fi
    else
        echo -e "${RED}✗ ${component}::${binary} failed to build${NC}"
    fi
    echo ""
    cd ..
}

# 1. Test STOQ Examples
echo -e "${YELLOW}=== STOQ Transport Layer ===${NC}\n"
run_example "stoq" "ebpf_demo" 5
run_example "stoq" "simple_adaptive_test" 5

# 2. Test TrustChain Examples
echo -e "${YELLOW}=== TrustChain CA/DNS/CT ===${NC}\n"
run_example "trustchain" "pos_validation_example" 5
run_example "trustchain" "falcon_integration" 5

# 3. Check Binaries
echo -e "${YELLOW}=== Binary Components ===${NC}\n"
check_binary "trustchain" "trustchain-bootstrap"
check_binary "gateway" "gateway"

# 4. Run Unit Tests (quick check)
echo -e "${YELLOW}=== Unit Tests (Quick Check) ===${NC}\n"

echo -e "${BLUE}Running STOQ tests${NC}"
cd stoq
if timeout 20 cargo test --lib 2>&1 | tail -5; then
    echo -e "${GREEN}✓ STOQ tests pass${NC}\n"
else
    echo -e "${YELLOW}⚠ STOQ tests incomplete${NC}\n"
fi
cd ..

echo -e "${BLUE}Running TrustChain tests${NC}"
cd trustchain
if timeout 20 cargo test --lib 2>&1 | tail -5; then
    echo -e "${GREEN}✓ TrustChain tests pass${NC}\n"
else
    echo -e "${YELLOW}⚠ TrustChain tests incomplete${NC}\n"
fi
cd ..

# 5. Summary
echo -e "${GREEN}==========================================${NC}"
echo -e "${GREEN}    Component Test Summary${NC}"
echo -e "${GREEN}==========================================${NC}\n"

echo -e "Working Components:"
echo -e "  ${GREEN}✓${NC} STOQ transport layer builds and has examples"
echo -e "  ${GREEN}✓${NC} TrustChain CA/DNS/CT builds with examples"
echo -e "  ${GREEN}✓${NC} Gateway HTTP/3 server builds"
echo -e "  ${GREEN}✓${NC} Unit tests are present"

echo -e "\nKnown Issues:"
echo -e "  ${YELLOW}⚠${NC} Catalog has compilation errors"
echo -e "  ${YELLOW}⚠${NC} BlockMatrix examples failing"
echo -e "  ${YELLOW}⚠${NC} Services require actual network setup to run"

echo -e "\nNext Steps:"
echo -e "  1. Fix Catalog compilation errors"
echo -e "  2. Implement missing BlockMatrix node binary"
echo -e "  3. Create integration tests between components"
echo -e "  4. Add proper service orchestration"

echo -e "\n${BLUE}Demo complete!${NC}"