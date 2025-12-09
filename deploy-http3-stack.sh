#!/bin/bash
# HTTP/3 Server Stack Deployment Script
# Deploys Gateway, BlockMatrix, and TrustChain servers in production mode

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== HyperMesh HTTP/3 Stack Deployment ===${NC}"
echo ""

# Ensure we're in the right directory
cd /home/persist/repos/projects/web3

# Step 1: Build Release Binaries
echo -e "${YELLOW}[1/5] Building Release Binaries...${NC}"
echo "  Building Gateway..."
cd /home/persist/repos/projects/web3/gateway && cargo build --release --bin gateway --quiet

echo "  Building BlockMatrix HTTP/3 Server..."
cd /home/persist/repos/projects/web3/blockmatrix && cargo build --release --bin blockmatrix-http3-server --quiet

echo "  Building TrustChain HTTP/3 Server..."
cd /home/persist/repos/projects/web3/trustchain && cargo build --release --bin trustchain-http3-server --quiet

echo -e "${GREEN}✅ All binaries built successfully${NC}"
echo ""

# Step 2: Stop existing servers
echo -e "${YELLOW}[2/5] Stopping Existing Servers...${NC}"
pkill -f "target/release/gateway" || true
pkill -f "blockmatrix-http3-server" || true
pkill -f "trustchain-http3-server" || true

# Give processes time to clean shutdown
sleep 2

echo -e "${GREEN}✅ Previous instances stopped${NC}"
echo ""

# Step 3: Create log directory if needed
LOG_DIR="/tmp/hypermesh-logs"
mkdir -p "$LOG_DIR"

# Step 4: Start servers in background
echo -e "${YELLOW}[3/5] Starting HTTP/3 Servers...${NC}"

# Start TrustChain (backend, port 50053)
echo "  Starting TrustChain HTTP/3 Server on [::1]:50053..."
cd /home/persist/repos/projects/web3/trustchain
RUST_LOG=info nohup /home/persist/repos/projects/web3/target/release/trustchain-http3-server \
    > "$LOG_DIR/trustchain.log" 2>&1 &
echo "    PID: $!"

# Start BlockMatrix (backend, port 8446)
echo "  Starting BlockMatrix HTTP/3 Server on [::1]:8446..."
cd /home/persist/repos/projects/web3/blockmatrix
RUST_LOG=info nohup /home/persist/repos/projects/web3/target/release/blockmatrix-http3-server \
    > "$LOG_DIR/blockmatrix.log" 2>&1 &
echo "    PID: $!"

# Start Gateway (frontend, port 8443)
echo "  Starting Gateway on [::]:8443..."
cd /home/persist/repos/projects/web3/gateway
RUST_LOG=info nohup /home/persist/repos/projects/web3/target/release/gateway \
    > "$LOG_DIR/gateway.log" 2>&1 &
echo "    PID: $!"

echo -e "${GREEN}✅ All servers started${NC}"
echo ""

# Step 5: Wait for services to stabilize
echo -e "${YELLOW}[4/5] Waiting for Service Startup...${NC}"
sleep 5

# Step 6: Verify services are running
echo -e "${YELLOW}[5/5] Verifying Service Health...${NC}"
echo ""

# Check processes
echo "Process Status:"
if pgrep -f "target/release/gateway" > /dev/null; then
    echo -e "  ${GREEN}✅${NC} Gateway running (PID: $(pgrep -f 'target/release/gateway'))"
else
    echo -e "  ${RED}❌${NC} Gateway not running"
fi

if pgrep -f "blockmatrix-http3-server" > /dev/null; then
    echo -e "  ${GREEN}✅${NC} BlockMatrix running (PID: $(pgrep -f 'blockmatrix-http3-server'))"
else
    echo -e "  ${RED}❌${NC} BlockMatrix not running"
fi

if pgrep -f "trustchain-http3-server" > /dev/null; then
    echo -e "  ${GREEN}✅${NC} TrustChain running (PID: $(pgrep -f 'trustchain-http3-server'))"
else
    echo -e "  ${RED}❌${NC} TrustChain not running"
fi

echo ""
echo "Port Status:"
if ss -uln | grep -q ":8443 "; then
    echo -e "  ${GREEN}✅${NC} Gateway port 8443 listening"
else
    echo -e "  ${RED}❌${NC} Gateway port 8443 not listening"
fi

if ss -uln | grep -q ":8446 "; then
    echo -e "  ${GREEN}✅${NC} BlockMatrix port 8446 listening"
else
    echo -e "  ${RED}❌${NC} BlockMatrix port 8446 not listening"
fi

if ss -uln | grep -q ":50053 "; then
    echo -e "  ${GREEN}✅${NC} TrustChain port 50053 listening"
else
    echo -e "  ${RED}❌${NC} TrustChain port 50053 not listening"
fi

echo ""
echo -e "${GREEN}=== Deployment Complete ===${NC}"
echo ""
echo "Service URLs:"
echo "  Gateway:     https://[::]:8443 (HTTP/3)"
echo "  BlockMatrix: https://[::1]:8446 (HTTP/3)"
echo "  TrustChain:  https://[::1]:50053 (HTTP/3)"
echo ""
echo "Log Files:"
echo "  Gateway:     tail -f $LOG_DIR/gateway.log"
echo "  BlockMatrix: tail -f $LOG_DIR/blockmatrix.log"
echo "  TrustChain:  tail -f $LOG_DIR/trustchain.log"
echo ""
echo "Health Check: /home/persist/repos/projects/web3/health-check.sh"
echo ""
echo "To stop all services: pkill -f 'gateway|blockmatrix-http3-server|trustchain-http3-server'"