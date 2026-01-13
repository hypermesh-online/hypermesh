#!/bin/bash

echo "Testing TrustChain API endpoints..."

# Test health endpoint
echo "Testing /api/v1/trustchain/health..."
curl -s http://localhost:9100/api/v1/trustchain/health | jq .

# Test stats endpoint
echo "Testing /api/v1/trustchain/stats..."
curl -s http://localhost:9100/api/v1/trustchain/stats | jq .

# Test root certificate endpoint
echo "Testing /api/v1/trustchain/certificates/root..."
curl -s http://localhost:9100/api/v1/trustchain/certificates/root | jq .

# Test certificates list endpoint
echo "Testing /api/v1/trustchain/certificates..."
curl -s http://localhost:9100/api/v1/trustchain/certificates | jq .

# Test expiring certificates endpoint
echo "Testing /api/v1/trustchain/certificates/expiring..."
curl -s http://localhost:9100/api/v1/trustchain/certificates/expiring | jq .

# Test revoked certificates endpoint
echo "Testing /api/v1/trustchain/certificates/revoked..."
curl -s http://localhost:9100/api/v1/trustchain/certificates/revoked | jq .

# Test rotation policies endpoint
echo "Testing /api/v1/trustchain/policies/rotation..."
curl -s http://localhost:9100/api/v1/trustchain/policies/rotation | jq .

echo "TrustChain API test complete!"