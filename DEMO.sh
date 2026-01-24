#!/bin/bash
#
# HyperMesh Web3 Ecosystem Demo Script
# =====================================
# This script starts core components to demonstrate a working HyperMesh environment
#

set -e

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track PIDs for cleanup
PIDS=()

# Cleanup function
cleanup() {
    echo -e "\n${YELLOW}Shutting down demo...${NC}"
    for pid in "${PIDS[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid" 2>/dev/null || true
        fi
    done
    exit 0
}

trap cleanup SIGINT SIGTERM EXIT

echo -e "${BLUE}==========================================${NC}"
echo -e "${BLUE}    HyperMesh Web3 Ecosystem Demo${NC}"
echo -e "${BLUE}==========================================${NC}\n"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Must run from web3 project root${NC}"
    exit 1
fi

# Build components first
echo -e "${YELLOW}Building components...${NC}"
echo -e "  Building TrustChain..."
(cd trustchain && cargo build --bin trustchain-bootstrap --quiet 2>/dev/null) || {
    echo -e "${RED}Failed to build trustchain-bootstrap${NC}"
    exit 1
}

echo -e "  Building Gateway..."
(cd gateway && cargo build --bin gateway --quiet 2>/dev/null) || {
    echo -e "${RED}Failed to build gateway${NC}"
    exit 1
}

echo -e "${GREEN}✓ Build complete${NC}\n"

# Start TrustChain Bootstrap
echo -e "${BLUE}[1/3] Starting TrustChain Bootstrap${NC}"
echo -e "      CA Port: 8443"
echo -e "      DNS Port: 8853"
echo -e "      CT Port: 8863"
echo -e "      Bind: [::1]"

cd trustchain
cargo run --bin trustchain-bootstrap -- \
    --bind ::1 \
    --ca-port 8443 \
    --dns-port 8853 \
    --ct-port 8863 \
    2>&1 | sed 's/^/    [TrustChain] /' &
TRUST_PID=$!
PIDS+=($TRUST_PID)
cd ..
sleep 3

echo -e "${GREEN}✓ TrustChain started (PID: $TRUST_PID)${NC}\n"

# Start Gateway
echo -e "${BLUE}[2/3] Starting HTTP/3 Gateway${NC}"
echo -e "      Port: 8444"
echo -e "      Bind: [::]:8444"

# Create minimal config for gateway
cat > gateway_demo_config.json << EOF
{
    "bind_addr": "[::]:8444",
    "backends": {
        "trustchain": "[::1]:8443",
        "blockmatrix": "[::1]:8446"
    }
}
EOF

cd gateway
cargo run --bin gateway 2>&1 | sed 's/^/    [Gateway] /' &
GATEWAY_PID=$!
PIDS+=($GATEWAY_PID)
cd ..
sleep 3

echo -e "${GREEN}✓ Gateway started (PID: $GATEWAY_PID)${NC}\n"

# Run a test example
echo -e "${BLUE}[3/3] Running Example: Proof of State Validation${NC}"
cd trustchain
timeout 10 cargo run --example pos_validation_example 2>&1 | sed 's/^/    [Example] /' || true
cd ..

echo -e "\n${GREEN}==========================================${NC}"
echo -e "${GREEN}    Demo Environment Running!${NC}"
echo -e "${GREEN}==========================================${NC}\n"

echo -e "Services:"
echo -e "  ${BLUE}TrustChain CA${NC}:  https://[::1]:8443"
echo -e "  ${BLUE}TrustChain DNS${NC}: dns://[::1]:8853"
echo -e "  ${BLUE}TrustChain CT${NC}:  https://[::1]:8863"
echo -e "  ${BLUE}HTTP/3 Gateway${NC}: https://[::]:8444"

echo -e "\nTest with:"
echo -e "  ${YELLOW}curl -k https://[::1]:8443/health${NC}"
echo -e "  ${YELLOW}dig @::1 -p 8853 trust.hypermesh.online AAAA${NC}"

echo -e "\nView logs:"
echo -e "  ${YELLOW}ps aux | grep -E 'trustchain|gateway'${NC}"

echo -e "\n${YELLOW}Press Ctrl+C to stop all services${NC}\n"

# Keep running
while true; do
    sleep 1
    # Check if processes are still running
    for pid in "${PIDS[@]}"; do
        if ! kill -0 "$pid" 2>/dev/null; then
            echo -e "${RED}Process $pid died unexpectedly${NC}"
        fi
    done
done