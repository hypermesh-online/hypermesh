#!/bin/bash

echo "=== HTTP/3 QUIC Verification Script ==="
echo ""

# Check if servers are built
echo "1. Checking for compiled binaries..."
if [ -f "/home/persist/repos/projects/web3/target/debug/blockmatrix-http3-server-minimal" ]; then
    echo "   ✅ BlockMatrix HTTP/3 server binary exists"
else
    echo "   ❌ BlockMatrix HTTP/3 server binary NOT FOUND"
fi

if [ -f "/home/persist/repos/projects/web3/target/debug/trustchain-http3-server-minimal" ]; then
    echo "   ✅ TrustChain HTTP/3 server binary exists"
else
    echo "   ❌ TrustChain HTTP/3 server binary NOT FOUND"
fi

echo ""
echo "2. Checking source code for TCP violations..."

# Check for TcpListener usage
if grep -q "TcpListener" /home/persist/repos/projects/web3/blockmatrix/src/bin/blockmatrix-http3-server-minimal.rs; then
    echo "   ❌ BlockMatrix uses TcpListener (VIOLATION)"
else
    echo "   ✅ BlockMatrix does NOT use TcpListener"
fi

if grep -q "TcpListener" /home/persist/repos/projects/web3/trustchain/src/bin/trustchain-http3-server-minimal.rs; then
    echo "   ❌ TrustChain uses TcpListener (VIOLATION)"
else
    echo "   ✅ TrustChain does NOT use TcpListener"
fi

echo ""
echo "3. Checking for QUIC/HTTP3 implementation..."

# Check for quinn and h3 usage
if grep -q "quinn::Endpoint" /home/persist/repos/projects/web3/blockmatrix/src/bin/blockmatrix-http3-server-minimal.rs; then
    echo "   ✅ BlockMatrix uses quinn::Endpoint (QUIC)"
else
    echo "   ❌ BlockMatrix missing quinn::Endpoint"
fi

if grep -q "h3::server::Connection" /home/persist/repos/projects/web3/blockmatrix/src/bin/blockmatrix-http3-server-minimal.rs; then
    echo "   ✅ BlockMatrix uses h3::server::Connection (HTTP/3)"
else
    echo "   ❌ BlockMatrix missing h3::server::Connection"
fi

if grep -q "quinn::Endpoint" /home/persist/repos/projects/web3/trustchain/src/bin/trustchain-http3-server-minimal.rs; then
    echo "   ✅ TrustChain uses quinn::Endpoint (QUIC)"
else
    echo "   ❌ TrustChain missing quinn::Endpoint"
fi

if grep -q "h3::server::Connection" /home/persist/repos/projects/web3/trustchain/src/bin/trustchain-http3-server-minimal.rs; then
    echo "   ✅ TrustChain uses h3::server::Connection (HTTP/3)"
else
    echo "   ❌ TrustChain missing h3::server::Connection"
fi

echo ""
echo "4. Checking for ALPN protocol 'h3'..."

if grep -q 'b"h3"' /home/persist/repos/projects/web3/blockmatrix/src/bin/blockmatrix-http3-server-minimal.rs; then
    echo "   ✅ BlockMatrix sets ALPN protocol to 'h3'"
else
    echo "   ❌ BlockMatrix missing ALPN 'h3' protocol"
fi

if grep -q 'b"h3"' /home/persist/repos/projects/web3/trustchain/src/bin/trustchain-http3-server-minimal.rs; then
    echo "   ✅ TrustChain sets ALPN protocol to 'h3'"
else
    echo "   ❌ TrustChain missing ALPN 'h3' protocol"
fi

echo ""
echo "5. Checking active network ports (if servers running)..."
echo "   UDP ports for QUIC (expected):"
ss -ulnp 2>/dev/null | grep -E "(8446|9293)" | while read line; do
    echo "     ✅ $line"
done

echo ""
echo "   TCP ports (should be NONE for these servers):"
ss -tlnp 2>/dev/null | grep -E "(8446|9293)" | while read line; do
    echo "     ❌ VIOLATION: $line"
done

if ! ss -tlnp 2>/dev/null | grep -qE "(8446|9293)"; then
    echo "     ✅ No TCP listeners on ports 8446 or 9293"
fi

echo ""
echo "=== Summary ==="
echo "HTTP/3 over QUIC Implementation Status:"
echo "- Transport: QUIC (UDP-based, NOT TCP)"
echo "- Protocol: HTTP/3 (via h3 crate)"
echo "- TLS: Self-signed certificates"
echo "- ALPN: 'h3' protocol negotiation"
echo ""
echo "Ports:"
echo "- BlockMatrix: https://[::1]:8446 (QUIC/UDP)"
echo "- TrustChain: https://[::1]:9293 (QUIC/UDP)"
echo ""
echo "✅ = Compliant with 'HTTP/3 QUIC or BUST' requirement"
echo "❌ = Architecture violation that must be fixed"