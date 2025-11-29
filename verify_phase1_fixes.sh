#!/bin/bash

# Phase 1 Verification Script
# Automated verification of critical fixes

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "========================================="
echo "Phase 1 Fix Verification Script"
echo "========================================="
echo ""

ERRORS=0
WARNINGS=0

# Function to print colored status
print_status() {
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✓${NC} $2"
    else
        echo -e "${RED}✗${NC} $2"
        ERRORS=$((ERRORS + 1))
    fi
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
    WARNINGS=$((WARNINGS + 1))
}

echo "1. VERIFYING WORKSPACE CONFIGURATION"
echo "-------------------------------------"

# Check for hypermesh references in Cargo files
echo -n "Checking for hypermesh references in Cargo.toml files... "
HYPERMESH_REFS=$(grep -r "hypermesh" --include="Cargo.toml" . 2>/dev/null | grep -v "keywords\|description\|comment" | wc -l)
print_status $([ $HYPERMESH_REFS -eq 0 ] && echo 0 || echo 1) "No hypermesh references in Cargo files ($HYPERMESH_REFS found)"

# Check for hypermesh imports in Rust files
echo -n "Checking for hypermesh imports in Rust files... "
HYPERMESH_IMPORTS=$(grep -r "use hypermesh::\|extern crate hypermesh" --include="*.rs" . 2>/dev/null | wc -l)
print_status $([ $HYPERMESH_IMPORTS -eq 0 ] && echo 0 || echo 1) "No hypermesh imports ($HYPERMESH_IMPORTS found)"

echo ""
echo "2. VERIFYING AWS CREDENTIALS REMOVAL"
echo "-------------------------------------"

# Check for AWS access keys
echo -n "Scanning for AWS access keys (AKIA pattern)... "
AWS_KEYS=$(grep -r "AKIA[0-9A-Z]\{16\}" . --exclude-dir=.git --exclude="*.log" --exclude="verify_phase1_fixes.sh" 2>/dev/null | wc -l)
print_status $([ $AWS_KEYS -eq 0 ] && echo 0 || echo 1) "No AWS access keys found ($AWS_KEYS found)"

# Check for AWS credential patterns
echo -n "Scanning for AWS credential patterns... "
AWS_PATTERNS=$(grep -rEi "aws[_-]?(access[_-]?key|secret)[_-]?(id|key)?.*=.*['\"].*['\"]" . \
    --exclude-dir=.git \
    --exclude-dir=node_modules \
    --exclude="*.log" \
    --exclude="verify_phase1_fixes.sh" \
    --exclude="PHASE1_VERIFICATION_PLAN.md" 2>/dev/null | wc -l)
print_status $([ $AWS_PATTERNS -eq 0 ] && echo 0 || echo 1) "No AWS credential patterns ($AWS_PATTERNS found)"

# Check for .env.example files
echo -n "Checking for .env.example files... "
ENV_EXAMPLES=$(find . -name ".env.example" 2>/dev/null | wc -l)
if [ $ENV_EXAMPLES -gt 0 ]; then
    echo -e "${GREEN}✓${NC} Found $ENV_EXAMPLES .env.example files"
else
    print_warning "No .env.example files found (may need documentation)"
fi

echo ""
echo "3. VERIFYING PROOF_OF_STATE → PROOF OF STATE RENAMING"
echo "----------------------------------------------"

# Check for Proof of State references
echo -n "Scanning for Proof of State references... "
PROOF_OF_STATE_REFS=$(grep -r "Proof of State\|proof_of_state" . \
    --exclude-dir=.git \
    --exclude-dir=node_modules \
    --exclude="*.log" \
    --exclude="verify_phase1_fixes.sh" \
    --exclude="PHASE1_VERIFICATION_PLAN.md" \
    --exclude="CLAUDE.md" 2>/dev/null | wc -l)
if [ $PROOF_OF_STATE_REFS -gt 50 ]; then
    print_status 1 "Too many Proof of State references remaining ($PROOF_OF_STATE_REFS found)"
else
    print_warning "$PROOF_OF_STATE_REFS Proof of State references found (review needed)"
fi

# Check for proof_of_state module paths
echo -n "Checking for proof_of_state module paths... "
PROOF_OF_STATE_MODS=$(find . -type f -name "*.rs" -exec grep -l "mod proof_of_state\|use.*proof_of_state" {} \; 2>/dev/null | wc -l)
print_status $([ $PROOF_OF_STATE_MODS -eq 0 ] && echo 0 || echo 1) "No proof_of_state module paths ($PROOF_OF_STATE_MODS found)"

# Check for proof_of_state module
echo -n "Verifying proof_of_state module exists... "
if [ -d "lib/src/proof_of_state" ]; then
    echo -e "${GREEN}✓${NC} proof_of_state module exists"
else
    echo -e "${RED}✗${NC} proof_of_state module not found"
    ERRORS=$((ERRORS + 1))
fi

echo ""
echo "4. BUILD VERIFICATION"
echo "---------------------"

# Try to build the workspace
echo "Attempting workspace build..."
if cargo build --workspace 2>&1 | grep -q "error\["; then
    echo -e "${RED}✗${NC} Workspace build failed"
    ERRORS=$((ERRORS + 1))
else
    echo -e "${GREEN}✓${NC} Workspace builds successfully"
fi

# Check individual crates
echo "Checking individual crate builds:"
for crate in stoq trustchain caesar catalog blockmatrix; do
    echo -n "  $crate: "
    if cargo check -p $crate 2>&1 | grep -q "error\["; then
        echo -e "${RED}FAIL${NC}"
        ERRORS=$((ERRORS + 1))
    else
        echo -e "${GREEN}OK${NC}"
    fi
done

echo ""
echo "5. SECURITY VERIFICATION"
echo "------------------------"

# Check for hardcoded secrets
echo -n "Scanning for potential hardcoded secrets... "
SECRETS=$(grep -r "password\|secret\|token\|key" . \
    --include="*.rs" \
    --include="*.toml" \
    --include="*.yml" 2>/dev/null | \
    grep -v "pub\|struct\|fn\|//\|#" | \
    grep "=" | wc -l)
if [ $SECRETS -gt 10 ]; then
    print_warning "$SECRETS potential secrets found (manual review needed)"
else
    echo -e "${GREEN}✓${NC} Minimal secret patterns ($SECRETS found)"
fi

# Check for localhost references
echo -n "Checking for hardcoded localhost/IPs... "
LOCALHOST=$(grep -r "127\.0\.0\.1\|localhost" . \
    --include="*.rs" \
    --exclude-dir=tests \
    --exclude-dir=examples 2>/dev/null | wc -l)
if [ $LOCALHOST -gt 5 ]; then
    print_warning "$LOCALHOST localhost references (may need config)"
else
    echo -e "${GREEN}✓${NC} Minimal localhost references ($LOCALHOST found)"
fi

echo ""
echo "========================================="
echo "VERIFICATION SUMMARY"
echo "========================================="
echo ""

if [ $ERRORS -eq 0 ]; then
    echo -e "${GREEN}✅ All critical checks passed!${NC}"
    echo -e "${YELLOW}⚠  $WARNINGS warnings (manual review recommended)${NC}"
    echo ""
    echo "Phase 1 fixes appear to be successfully applied."
    exit 0
else
    echo -e "${RED}❌ $ERRORS critical errors found${NC}"
    echo -e "${YELLOW}⚠  $WARNINGS warnings${NC}"
    echo ""
    echo "Phase 1 fixes need additional work."
    echo "Run './verify_phase1_fixes.sh -v' for verbose output"
    exit 1
fi