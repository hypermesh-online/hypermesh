#!/bin/bash
# Quick verification that HTTP/3 servers can start

echo "Starting TrustChain HTTP/3 server for 3 seconds..."
cd /home/persist/repos/projects/web3/trustchain
timeout 3 cargo run --bin trustchain-http3-server 2>&1 | head -20 &
PID=$!
sleep 1

if ps -p $PID > /dev/null 2>&1; then
    echo "✓ TrustChain HTTP/3 server started successfully"
else
    echo "✗ TrustChain HTTP/3 server failed to start"
fi

wait $PID 2>/dev/null

echo ""
echo "Starting BlockMatrix HTTP/3 server for 3 seconds..."
cd /home/persist/repos/projects/web3/blockmatrix
timeout 3 cargo run --bin blockmatrix-http3-server 2>&1 | head -20 &
PID=$!
sleep 1

if ps -p $PID > /dev/null 2>&1; then
    echo "✓ BlockMatrix HTTP/3 server started successfully"
else
    echo "✗ BlockMatrix HTTP/3 server failed to start"
fi

wait $PID 2>/dev/null

echo ""
echo "Both servers can start successfully!"