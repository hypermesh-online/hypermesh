#!/bin/bash

# Verify No Credentials Script
# Checks codebase for hardcoded credentials and sensitive data

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "🔍 Verifying no credentials in codebase..."
echo "==========================================="

FOUND_ISSUES=0

# Check for AWS ARNs
echo -n "Checking for hardcoded AWS ARNs... "
if grep -r "arn:aws:kms.*123456789012" . --exclude-dir={.git,node_modules,dist,build} --exclude="*.log" 2>/dev/null | grep -v "verify-no-credentials.sh"; then
    echo -e "${RED}FOUND${NC}"
    FOUND_ISSUES=1
else
    echo -e "${GREEN}CLEAN${NC}"
fi

# Check for AWS access keys
echo -n "Checking for AWS access keys (AKIA pattern)... "
if grep -r "AKIA[0-9A-Z]\{16\}" . --exclude-dir={.git,node_modules,dist,build} --exclude="*.log" 2>/dev/null | grep -v "example" | grep -v "verify-no-credentials.sh"; then
    echo -e "${RED}FOUND${NC}"
    FOUND_ISSUES=1
else
    echo -e "${GREEN}CLEAN${NC}"
fi

# Check for hardcoded S3 buckets (production patterns)
echo -n "Checking for hardcoded S3 bucket names... "
if grep -r "trustchain-ct-logs-prod\|trustchain-backups-prod\|trustchain-terraform-state" . --exclude-dir={.git,node_modules,dist,build} --exclude="*.log" 2>/dev/null | grep -v "verify-no-credentials.sh" | grep -v "REQUIRED_ENV_VARS.md"; then
    echo -e "${RED}FOUND${NC}"
    FOUND_ISSUES=1
else
    echo -e "${GREEN}CLEAN${NC}"
fi

# Check for private keys (64 hex chars)
echo -n "Checking for potential private keys... "
if grep -r "0x[a-fA-F0-9]\{64\}" . --exclude-dir={.git,node_modules,dist,build,artifacts} --exclude="*.log" --exclude="*.json" 2>/dev/null | grep -i "private" | grep -v "example" | grep -v "verify-no-credentials.sh"; then
    echo -e "${RED}FOUND${NC}"
    FOUND_ISSUES=1
else
    echo -e "${GREEN}CLEAN${NC}"
fi

# Check for Stripe keys
echo -n "Checking for Stripe keys... "
if grep -r "sk_live_\|pk_live_" . --exclude-dir={.git,node_modules,dist,build} --exclude="*.log" 2>/dev/null | grep -v "example" | grep -v "verify-no-credentials.sh"; then
    echo -e "${RED}FOUND${NC}"
    FOUND_ISSUES=1
else
    echo -e "${GREEN}CLEAN${NC}"
fi

# Check for API keys with actual values
echo -n "Checking for hardcoded API keys... "
if grep -r "API_KEY.*=.*['\"][a-zA-Z0-9]\{20,\}" . --exclude-dir={.git,node_modules,dist,build} --exclude="*.log" --exclude="*.example" 2>/dev/null | grep -v "process.env" | grep -v "verify-no-credentials.sh" | grep -v "\${"; then
    echo -e "${RED}FOUND${NC}"
    FOUND_ISSUES=1
else
    echo -e "${GREEN}CLEAN${NC}"
fi

# Check for mnemonics with actual words
echo -n "Checking for mnemonic phrases... "
if grep -r "MNEMONIC.*=.*['\"][a-z ]\{20,\}" . --exclude-dir={.git,node_modules,dist,build} --exclude="*.log" --exclude="*.example" 2>/dev/null | grep -v "verify-no-credentials.sh" | grep -v '""'; then
    echo -e "${RED}FOUND${NC}"
    FOUND_ISSUES=1
else
    echo -e "${GREEN}CLEAN${NC}"
fi

# Check .env files
echo -n "Checking for .env files with actual values... "
ENV_FILES=$(find . -name ".env" -o -name ".env.*" | grep -v ".env.example" | grep -v node_modules || true)
if [ ! -z "$ENV_FILES" ]; then
    for file in $ENV_FILES; do
        if grep -E "=.+[a-zA-Z0-9]{10,}" "$file" 2>/dev/null | grep -v '=""' | grep -v "verify-no-credentials.sh"; then
            echo -e "${RED}FOUND in $file${NC}"
            FOUND_ISSUES=1
        fi
    done
fi
if [ $FOUND_ISSUES -eq 0 ]; then
    echo -e "${GREEN}CLEAN${NC}"
fi

echo "==========================================="

if [ $FOUND_ISSUES -eq 1 ]; then
    echo -e "${RED}❌ SECURITY ISSUES FOUND!${NC}"
    echo "Please remove all hardcoded credentials and use environment variables instead."
    echo "See REQUIRED_ENV_VARS.md for the list of required environment variables."
    exit 1
else
    echo -e "${GREEN}✅ No credentials found in codebase!${NC}"
    echo "All sensitive data should be provided via environment variables."
    echo "See REQUIRED_ENV_VARS.md for configuration details."
    exit 0
fi