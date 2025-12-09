#!/bin/bash
# Health check for both HTTP/3 servers

echo "HTTP/3 Server Health Check"
echo "==========================="
echo ""

# Check if servers are running
TRUSTCHAIN_PID=$(pgrep -f 'trustchain-http3-server' || echo "")
BLOCKMATRIX_PID=$(pgrep -f 'blockmatrix-http3-server' || echo "")

echo "Process Status:"
if [ -n "$TRUSTCHAIN_PID" ]; then
    echo "  ✅ TrustChain HTTP/3 server running (PID: $TRUSTCHAIN_PID)"
else
    echo "  ❌ TrustChain HTTP/3 server NOT running"
fi

if [ -n "$BLOCKMATRIX_PID" ]; then
    echo "  ✅ BlockMatrix HTTP/3 server running (PID: $BLOCKMATRIX_PID)"
else
    echo "  ❌ BlockMatrix HTTP/3 server NOT running"
fi

echo ""
echo "==========================="
echo "Endpoint Health Checks:"
echo "==========================="

# Check TrustChain health
echo ""
echo "TrustChain (https://[::1]:50053/health):"
if [ -n "$TRUSTCHAIN_PID" ]; then
    RESPONSE=$(curl -k -s -w "\n%{http_code}" https://[::1]:50053/health 2>/dev/null)
    HTTP_CODE=$(echo "$RESPONSE" | tail -1)
    BODY=$(echo "$RESPONSE" | sed '$d')

    if [ "$HTTP_CODE" = "200" ]; then
        echo "  Status: ✅ 200 OK"
        echo "$BODY" | jq '.' 2>/dev/null || echo "$BODY"
    else
        echo "  Status: ❌ HTTP $HTTP_CODE"
        echo "$BODY"
    fi
else
    echo "  ❌ Server not running"
fi

# Check BlockMatrix health
echo ""
echo "BlockMatrix (https://[::1]:8446/health):"
if [ -n "$BLOCKMATRIX_PID" ]; then
    RESPONSE=$(curl -k -s -w "\n%{http_code}" https://[::1]:8446/health 2>/dev/null)
    HTTP_CODE=$(echo "$RESPONSE" | tail -1)
    BODY=$(echo "$RESPONSE" | sed '$d')

    if [ "$HTTP_CODE" = "200" ]; then
        echo "  Status: ✅ 200 OK"
        echo "$BODY" | jq '.' 2>/dev/null || echo "$BODY"
    else
        echo "  Status: ❌ HTTP $HTTP_CODE"
        echo "$BODY"
    fi
else
    echo "  ❌ Server not running"
fi

echo ""
echo "==========================="
echo ""
