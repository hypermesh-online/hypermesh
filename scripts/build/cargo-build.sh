#!/bin/bash

# Web3 Workspace Build Script
# Provides quick build status and error analysis

set -e

echo "======================================"
echo "   Web3 Workspace Build Status"
echo "======================================"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to check component
check_component() {
    local component=$1
    echo -n "Checking $component... "

    error_count=$(cargo check -p $component 2>&1 | grep -c "error\[" || true)
    warning_count=$(cargo check -p $component 2>&1 | grep -c "warning:" || true)

    if [ "$error_count" -eq 0 ]; then
        echo -e "${GREEN}✅ PASS${NC} (warnings: $warning_count)"
    else
        echo -e "${RED}❌ FAIL${NC} (errors: $error_count, warnings: $warning_count)"
    fi
}

# Clean build
echo "Cleaning previous build artifacts..."
cargo clean

echo ""
echo "Component Status:"
echo "-----------------"

# Check each component
check_component "stoq"
check_component "trustchain"
check_component "caesar"
check_component "catalog"
check_component "hypermesh"

echo ""
echo "Workspace Compilation:"
echo "----------------------"

# Full workspace check
echo -n "Full workspace... "
if cargo check --workspace 2>&1 | grep -q "error: could not compile"; then
    total_errors=$(cargo check --workspace 2>&1 | grep -c "error\[" || true)
    echo -e "${RED}❌ FAIL${NC} (total errors: $total_errors)"
else
    echo -e "${GREEN}✅ PASS${NC}"
fi

echo ""
echo "Dependency Analysis:"
echo "--------------------"

# Check for duplicate dependencies
echo -n "Checking for duplicate dependencies... "
dup_count=$(cargo tree -d 2>/dev/null | grep -c "^[a-z]" || true)
if [ "$dup_count" -gt 0 ]; then
    echo -e "${YELLOW}⚠️  Found $dup_count duplicates${NC}"
else
    echo -e "${GREEN}✅ No duplicates${NC}"
fi

# Check for outdated dependencies
echo -n "Checking for outdated dependencies... "
if command -v cargo-outdated &> /dev/null; then
    outdated=$(cargo outdated --workspace 2>/dev/null | grep -c "^[a-z]" || true)
    if [ "$outdated" -gt 0 ]; then
        echo -e "${YELLOW}⚠️  Found $outdated outdated${NC}"
    else
        echo -e "${GREEN}✅ All up to date${NC}"
    fi
else
    echo "cargo-outdated not installed"
fi

echo ""
echo "======================================"
echo "Build script complete!"
echo ""

# Summary
if cargo check --workspace 2>&1 | grep -q "error: could not compile"; then
    echo -e "${RED}Status: Build Failed${NC}"
    echo "Run 'cargo check --workspace' for detailed errors"
else
    echo -e "${GREEN}Status: Build Successful${NC}"
    echo "All components compiled successfully!"
fi