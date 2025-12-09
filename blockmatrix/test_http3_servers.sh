#!/bin/bash

echo "=== HTTP/3 Server Testing Script ==="
echo "Testing BlockMatrix HTTP/3 servers with QUIC transport"
echo ""

# Kill any existing servers
echo "1. Cleaning up existing processes..."
pkill -f "blockmatrix-http3" 2>/dev/null
pkill -f "trustchain-http3" 2>/dev/null
sleep 2

# Start BlockMatrix minimal server
echo "2. Starting BlockMatrix HTTP/3 server (minimal)..."
cargo run --bin blockmatrix-http3-server-minimal > /tmp/blockmatrix-http3.log 2>&1 &
BM_PID=$!
echo "   Started with PID: $BM_PID"

# Wait for startup
echo "3. Waiting for server startup (5 seconds)..."
sleep 5

# Check if server is running
echo "4. Checking server status..."
if ps -p $BM_PID > /dev/null; then
    echo "   ✓ Server is running"
else
    echo "   ✗ Server failed to start"
    echo "   Last 20 lines of log:"
    tail -20 /tmp/blockmatrix-http3.log
    exit 1
fi

# Check UDP port (QUIC uses UDP)
echo "5. Checking UDP port 8446..."
if ss -uln | grep -q ":8446"; then
    echo "   ✓ Port 8446 is listening (UDP/QUIC)"
else
    echo "   ✗ Port 8446 is not listening"
    echo "   Active UDP ports:"
    ss -uln | grep -v "^State"
fi

# Try to test with curl (won't work for HTTP/3 but shows connectivity)
echo "6. Testing with curl (HTTP/1.1 fallback test)..."
curl -k --http1.1 https://localhost:8446/health 2>&1 | head -5 || echo "   Note: curl doesn't support HTTP/3, this is expected to fail"

echo ""
echo "7. Server log output (last 20 lines):"
tail -20 /tmp/blockmatrix-http3.log

echo ""
echo "=== Test Summary ==="
echo "BlockMatrix HTTP/3 server is running on PID $BM_PID"
echo "QUIC/UDP port 8446 should be active"
echo "Use a HTTP/3 compatible client to test (curl doesn't support HTTP/3)"
echo ""
echo "Expected endpoints:"
echo "  - https://[::1]:8446/api/v1/blockmatrix/health"
echo "  - https://[::1]:8446/api/v1/blockmatrix/status"
echo "  - https://[::1]:8446/api/v1/blockmatrix/matrix"
echo "  - https://[::1]:8446/api/v1/blockmatrix/assets"
echo ""

# Run simple Rust test if available
echo "8. Running Rust integration test..."
cargo test test_server_compilation --test http3_integration_test 2>&1 | tail -10

echo ""
echo "9. Cleanup (keeping server running for manual testing)..."
echo "   Server is still running on PID $BM_PID"
echo "   To stop: kill $BM_PID"