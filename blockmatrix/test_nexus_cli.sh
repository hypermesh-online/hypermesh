#!/bin/bash
# Test script for nexus CLI functionality

set -e

echo "🧪 Testing Nexus CLI Functionality"
echo "================================="

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo_test() {
    echo -e "${BLUE}Testing: $1${NC}"
}

echo_pass() {
    echo -e "${GREEN}✅ PASS: $1${NC}"
}

echo_info() {
    echo -e "${YELLOW}ℹ️  $1${NC}"
}

# Test 1: Basic help command
echo_test "nexus help command"
if nexus help > /dev/null 2>&1; then
    echo_pass "Help command works"
else
    echo -e "${RED}❌ FAIL: Help command failed${NC}"
    exit 1
fi

# Test 2: Version command
echo_test "nexus version command"
VERSION_OUTPUT=$(nexus version)
if [[ $VERSION_OUTPUT == *"nexus 0.1.0"* ]]; then
    echo_pass "Version command works"
else
    echo -e "${RED}❌ FAIL: Version command failed${NC}"
    exit 1
fi

# Test 3: Status command
echo_test "nexus status command"
if nexus status | grep -q "Cluster: Healthy"; then
    echo_pass "Status command shows healthy cluster"
else
    echo -e "${RED}❌ FAIL: Status command failed${NC}"
    exit 1
fi

# Test 4: Cluster creation
echo_test "nexus cluster create command"
if nexus cluster create test-cluster | grep -q "created successfully"; then
    echo_pass "Cluster creation command works"
else
    echo -e "${RED}❌ FAIL: Cluster creation failed${NC}"
    exit 1
fi

# Test 5: Service deployment
echo_test "nexus service deploy command"
if nexus service deploy redis:alpine | grep -q "deployed successfully"; then
    echo_pass "Service deployment command works"
else
    echo -e "${RED}❌ FAIL: Service deployment failed${NC}"
    exit 1
fi

# Test 6: Service listing
echo_test "nexus service list command"
if nexus service list | grep -q "nginx-service"; then
    echo_pass "Service list command works"
else
    echo -e "${RED}❌ FAIL: Service list failed${NC}"
    exit 1
fi

# Test 7: Error handling
echo_test "Error handling for unknown commands"
if nexus unknown-command 2>&1 | grep -q "Unknown command"; then
    echo_pass "Error handling works for unknown commands"
else
    echo -e "${RED}❌ FAIL: Error handling failed${NC}"
    exit 1
fi

echo ""
echo_info "All tests passed! 🎉"
echo_info "The nexus CLI is working correctly."

echo ""
echo "📋 Summary of working commands:"
echo "  nexus help"
echo "  nexus version"
echo "  nexus status"
echo "  nexus cluster create <name>"
echo "  nexus service deploy <image>"
echo "  nexus service list"

echo ""
echo "🚀 Next Steps:"
echo "1. Add Docker integration for real container management"
echo "2. Implement persistent state tracking"
echo "3. Add more detailed error messages"
echo "4. Create configuration file support"
echo "5. Build web dashboard integration"