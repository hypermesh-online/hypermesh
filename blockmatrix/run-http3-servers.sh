#!/bin/bash

# HTTP/3 QUIC Server Launcher
# This launches proper HTTP/3 servers over QUIC transport (NOT TCP)

echo "=== HTTP/3 QUIC Server Launcher ==="
echo "Architecture: QUIC transport with HTTP/3 protocol"
echo "NO TCP connections - pure QUIC only"
echo ""

# Kill any existing servers
echo "Stopping any existing servers..."
pkill -f "trustchain-http3-server-minimal" 2>/dev/null
pkill -f "blockmatrix-http3-server-minimal" 2>/dev/null
sleep 1

# Start TrustChain HTTP/3 server
echo "Starting TrustChain HTTP/3 server on https://[::1]:9293 (QUIC)..."
/home/persist/repos/projects/web3/target/debug/trustchain-http3-server-minimal > /tmp/trustchain-http3.log 2>&1 &
TRUST_PID=$!
echo "TrustChain PID: $TRUST_PID"

# Start BlockMatrix HTTP/3 server
echo "Starting BlockMatrix HTTP/3 server on https://[::1]:8446 (QUIC)..."
/home/persist/repos/projects/web3/target/debug/blockmatrix-http3-server-minimal > /tmp/blockmatrix-http3.log 2>&1 &
BLOCK_PID=$!
echo "BlockMatrix PID: $BLOCK_PID"

echo ""
echo "=== Servers Started ==="
echo "TrustChain: https://[::1]:9293/api/v1/trustchain/health"
echo "BlockMatrix: https://[::1]:8446/api/v1/blockmatrix/health"
echo ""
echo "Logs:"
echo "  tail -f /tmp/trustchain-http3.log"
echo "  tail -f /tmp/blockmatrix-http3.log"
echo ""
echo "Test with curl (requires HTTP/3 support):"
echo "  curl --http3 https://[::1]:9293/api/v1/trustchain/health"
echo "  curl --http3 https://[::1]:8446/api/v1/blockmatrix/health"
echo ""
echo "Note: Certificate warnings are expected (self-signed)"
echo "Press Ctrl+C to stop servers..."

# Wait for interrupt
trap "echo 'Stopping servers...'; kill $TRUST_PID $BLOCK_PID 2>/dev/null; exit" INT
wait