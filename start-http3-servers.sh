#!/bin/bash
# Start both HTTP/3 servers for development

set -e

echo "Starting HTTP/3 Servers..."
echo "=========================="
echo ""

# Create log directory if it doesn't exist
mkdir -p /tmp/http3-logs

# Kill any existing servers
pkill -f 'trustchain-http3-server' 2>/dev/null || true
pkill -f 'blockmatrix-http3-server' 2>/dev/null || true
sleep 1

# Start TrustChain HTTP/3 server
echo "Starting TrustChain HTTP/3 server on port 50053..."
cd /home/persist/repos/projects/web3/trustchain
nohup /home/persist/repos/projects/web3/target/debug/trustchain-http3-server > /tmp/http3-logs/trustchain-http3.log 2>&1 &
TRUSTCHAIN_PID=$!
echo "  PID: $TRUSTCHAIN_PID"

# Start BlockMatrix HTTP/3 server
echo "Starting BlockMatrix HTTP/3 server on port 8446..."
cd /home/persist/repos/projects/web3/blockmatrix
nohup /home/persist/repos/projects/web3/target/debug/blockmatrix-http3-server > /tmp/http3-logs/blockmatrix-http3.log 2>&1 &
BLOCKMATRIX_PID=$!
echo "  PID: $BLOCKMATRIX_PID"

# Wait for servers to start
echo ""
echo "Waiting for servers to initialize..."
sleep 2

# Check if servers are running
TRUSTCHAIN_RUNNING=$(ps -p $TRUSTCHAIN_PID > /dev/null 2>&1 && echo "yes" || echo "no")
BLOCKMATRIX_RUNNING=$(ps -p $BLOCKMATRIX_PID > /dev/null 2>&1 && echo "yes" || echo "no")

echo ""
echo "=========================="
echo "HTTP/3 Servers Status"
echo "=========================="
echo ""

if [ "$TRUSTCHAIN_RUNNING" = "yes" ]; then
    echo "✅ TrustChain:  https://[::1]:50053  (PID: $TRUSTCHAIN_PID)"
else
    echo "❌ TrustChain:  Failed to start (check logs)"
fi

if [ "$BLOCKMATRIX_RUNNING" = "yes" ]; then
    echo "✅ BlockMatrix: https://[::1]:8446   (PID: $BLOCKMATRIX_PID)"
else
    echo "❌ BlockMatrix: Failed to start (check logs)"
fi

echo ""
echo "Logs:"
echo "  tail -f /tmp/http3-logs/trustchain-http3.log"
echo "  tail -f /tmp/http3-logs/blockmatrix-http3.log"
echo ""
echo "Health Check:"
echo "  ./check-http3-health.sh"
echo ""
echo "To stop:"
echo "  ./stop-http3-servers.sh"
echo ""
