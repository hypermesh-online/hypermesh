#!/bin/bash

# Test Hardware Detection API Endpoints

API_BASE="http://[::1]:8443"

echo "Testing Hardware Detection API Endpoints"
echo "========================================"

# Test 1: Get Hardware Capabilities
echo -e "\n1. Testing GET /api/v1/system/hardware"
curl -s "${API_BASE}/api/v1/system/hardware" | jq '.' || echo "Failed to get hardware capabilities"

# Test 2: Get Network Capabilities
echo -e "\n2. Testing GET /api/v1/system/network"
curl -s "${API_BASE}/api/v1/system/network" | jq '.' || echo "Failed to get network capabilities"

# Test 3: Get Resource Allocation
echo -e "\n3. Testing GET /api/v1/system/allocation"
curl -s "${API_BASE}/api/v1/system/allocation" | jq '.' || echo "Failed to get resource allocation"

# Test 4: Get Sharing Capabilities
echo -e "\n4. Testing GET /api/v1/system/capabilities"
curl -s "${API_BASE}/api/v1/system/capabilities" | jq '.' || echo "Failed to get sharing capabilities"

# Test 5: Refresh Hardware Detection
echo -e "\n5. Testing POST /api/v1/system/refresh"
curl -s -X POST "${API_BASE}/api/v1/system/refresh" | jq '.' || echo "Failed to refresh hardware detection"

echo -e "\nAll tests completed!"