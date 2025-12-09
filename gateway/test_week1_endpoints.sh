#!/bin/bash

# Test script for Week 1 Critical Endpoints
# Tests all 10 endpoints implemented in the backend servers

set -e

echo "======================================================"
echo "Testing Week 1 Critical Endpoints (10 endpoints)"
echo "======================================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Base URLs
BLOCKMATRIX_URL="https://[::1]:8446"
TRUSTCHAIN_URL="https://[::1]:50053"
GATEWAY_URL="https://[::1]:8443"

# Counter for successful tests
SUCCESS_COUNT=0
TOTAL_COUNT=10

echo ""
echo "Starting servers..."
echo "Note: Please ensure blockmatrix-http3-server is running on port 8446"
echo "      and trustchain-http3-server is running on port 50053"
echo ""

# Function to test an endpoint
test_endpoint() {
    local endpoint=$1
    local url=$2
    local method=${3:-GET}
    local data=${4:-}

    echo -n "Testing $method $endpoint... "

    if [ "$method" = "POST" ]; then
        if curl -k -s -X POST "$url$endpoint" \
             -H "Content-Type: application/json" \
             -d "$data" \
             --connect-timeout 5 > /dev/null 2>&1; then
            echo -e "${GREEN}✓ SUCCESS${NC}"
            ((SUCCESS_COUNT++))
        else
            echo -e "${RED}✗ FAILED${NC}"
        fi
    else
        if curl -k -s "$url$endpoint" --connect-timeout 5 > /dev/null 2>&1; then
            echo -e "${GREEN}✓ SUCCESS${NC}"
            ((SUCCESS_COUNT++))
        else
            echo -e "${RED}✗ FAILED${NC}"
        fi
    fi
}

echo "======================================================"
echo "1. Testing Gateway Health (Gateway)"
echo "======================================================"
test_endpoint "/health" "$GATEWAY_URL"

echo ""
echo "======================================================"
echo "2-9. Testing BlockMatrix Endpoints (Port 8446)"
echo "======================================================"

# 2. HyperMesh System Status
test_endpoint "/api/v1/hypermesh/system/status" "$BLOCKMATRIX_URL"

# 3. HyperMesh Assets
test_endpoint "/api/v1/hypermesh/assets" "$BLOCKMATRIX_URL"

# 4. HyperMesh Allocations
test_endpoint "/api/v1/hypermesh/allocations" "$BLOCKMATRIX_URL"

# 5. STOQ System Health
test_endpoint "/api/v1/stoq/system/health" "$BLOCKMATRIX_URL"

# 6. STOQ Connections
test_endpoint "/api/v1/stoq/connections" "$BLOCKMATRIX_URL"

# 7. HyperMesh Nodes Health
test_endpoint "/api/v1/hypermesh/nodes/health" "$BLOCKMATRIX_URL"

# 8. STOQ Performance Metrics
test_endpoint "/api/v1/stoq/metrics/performance" "$BLOCKMATRIX_URL"

# 9. Byzantine Detections
test_endpoint "/api/v1/hypermesh/byzantine/detections" "$BLOCKMATRIX_URL"

echo ""
echo "======================================================"
echo "10. Testing TrustChain Auth Endpoint (Port 50053)"
echo "======================================================"

# 10. TrustChain Auth Certificate
test_endpoint "/api/v1/trustchain/auth/certificate" "$TRUSTCHAIN_URL" "POST" '{"certificate_pem":"-----BEGIN CERTIFICATE-----\ntest\n-----END CERTIFICATE-----"}'

echo ""
echo "======================================================"
echo "TEST SUMMARY"
echo "======================================================"
echo "Successful: $SUCCESS_COUNT / $TOTAL_COUNT"

if [ $SUCCESS_COUNT -eq $TOTAL_COUNT ]; then
    echo -e "${GREEN}All endpoints are working!${NC}"
    exit 0
else
    echo -e "${YELLOW}Some endpoints failed. Please check the servers.${NC}"
    exit 1
fi