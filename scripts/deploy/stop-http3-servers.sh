#!/bin/bash
# Stop both HTTP/3 servers

echo "Stopping HTTP/3 servers..."

TRUSTCHAIN_PID=$(pgrep -f 'trustchain-http3-server' || echo "")
BLOCKMATRIX_PID=$(pgrep -f 'blockmatrix-http3-server' || echo "")

if [ -n "$TRUSTCHAIN_PID" ]; then
    echo "Stopping TrustChain HTTP/3 server (PID: $TRUSTCHAIN_PID)..."
    pkill -f 'trustchain-http3-server'
else
    echo "TrustChain HTTP/3 server not running"
fi

if [ -n "$BLOCKMATRIX_PID" ]; then
    echo "Stopping BlockMatrix HTTP/3 server (PID: $BLOCKMATRIX_PID)..."
    pkill -f 'blockmatrix-http3-server'
else
    echo "BlockMatrix HTTP/3 server not running"
fi

sleep 1

# Verify stopped
if pgrep -f 'http3-server' > /dev/null; then
    echo "⚠️  Warning: Some HTTP/3 servers still running"
    pgrep -af 'http3-server'
else
    echo "✅ All HTTP/3 servers stopped"
fi
