#!/bin/bash

# Script to run both HTTP/3 servers

echo "Starting HTTP/3 REST API Servers..."
echo "=================================="
echo ""
echo "NOTE: These are minimal placeholder servers (TCP/HTTP) for now."
echo "Full HTTP/3 implementation with QUIC will be added in next iteration."
echo ""

# Kill any existing servers on our ports
echo "Stopping any existing servers..."
lsof -ti:9293 | xargs -r kill -9 2>/dev/null
lsof -ti:8446 | xargs -r kill -9 2>/dev/null
sleep 1

# Start TrustChain server
echo "Starting TrustChain server on port 9293..."
./trustchain/target/debug/trustchain-http3-server-minimal &
TRUSTCHAIN_PID=$!
sleep 1

# Start BlockMatrix server
echo "Starting BlockMatrix server on port 8446..."
./blockmatrix/target/debug/blockmatrix-http3-server-minimal &
BLOCKMATRIX_PID=$!
sleep 1

echo ""
echo "Servers started!"
echo "================"
echo "TrustChain:  http://[::1]:9293 (PID: $TRUSTCHAIN_PID)"
echo "BlockMatrix: http://[::1]:8446 (PID: $BLOCKMATRIX_PID)"
echo ""
echo "Health check endpoints:"
echo "  curl http://[::1]:9293/api/v1/trustchain/health"
echo "  curl http://[::1]:8446/api/v1/blockmatrix/health"
echo ""

# Test health endpoints
echo "Testing health endpoints..."
echo "---------------------------"
echo ""
echo "TrustChain health check:"
curl -s http://[::1]:9293/api/v1/trustchain/health | python3 -m json.tool 2>/dev/null || echo "Failed to connect"
echo ""
echo "BlockMatrix health check:"
curl -s http://[::1]:8446/api/v1/blockmatrix/health | python3 -m json.tool 2>/dev/null || echo "Failed to connect"
echo ""

echo "Press Ctrl+C to stop servers..."

# Wait for Ctrl+C
trap "echo 'Stopping servers...'; kill $TRUSTCHAIN_PID $BLOCKMATRIX_PID 2>/dev/null; exit" INT
wait