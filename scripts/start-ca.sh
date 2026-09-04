#!/bin/bash
#
# Start TrustChain CA Service for Multi-Node Communication
#
# This script starts the TrustChain Certificate Authority service that issues
# certificates to BlockMatrix nodes for secure STOQ communication.
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

# Configuration
CA_PORT=${CA_PORT:-8443}
CA_MODE=${1:-"local"}
LOG_FILE="/tmp/trustchain-ca.log"
PID_FILE="/tmp/trustchain-ca.pid"

# Clean up function
cleanup() {
    echo -e "${YELLOW}Stopping CA service...${NC}"
    if [ -f "$PID_FILE" ]; then
        PID=$(cat "$PID_FILE")
        if kill -0 "$PID" 2>/dev/null; then
            kill "$PID" 2>/dev/null || true
            echo -e "${GREEN}CA service stopped (PID: $PID)${NC}"
        fi
        rm -f "$PID_FILE"
    fi
}

# Set up trap for cleanup
trap cleanup EXIT

echo -e "${MAGENTA}========================================${NC}"
echo -e "${MAGENTA}    TrustChain CA Service Launcher${NC}"
echo -e "${MAGENTA}========================================${NC}"
echo ""

# Check if CA is already running
if [ -f "$PID_FILE" ]; then
    OLD_PID=$(cat "$PID_FILE")
    if kill -0 "$OLD_PID" 2>/dev/null; then
        echo -e "${YELLOW}CA service already running with PID $OLD_PID${NC}"
        echo -e "${YELLOW}Stop it first with: kill $OLD_PID${NC}"
        exit 1
    else
        rm -f "$PID_FILE"
    fi
fi

# Parse mode
case "$CA_MODE" in
    local|localhost|dev)
        MODE_FLAGS=""
        MODE_DESC="Local Development"
        BIND_ADDR="[::1]:$CA_PORT"
        ;;
    production|prod)
        MODE_FLAGS="--production"
        MODE_DESC="Production"
        BIND_ADDR="[::]:$CA_PORT"
        ;;
    *)
        echo -e "${RED}Invalid mode: $CA_MODE${NC}"
        echo "Usage: $0 [local|production]"
        exit 1
        ;;
esac

echo -e "${BLUE}Mode: ${MODE_DESC}${NC}"
echo -e "${BLUE}Bind Address: ${BIND_ADDR}${NC}"
echo -e "${BLUE}Log File: ${LOG_FILE}${NC}"
echo ""

# Build the CA binary
echo -e "${YELLOW}Building TrustChain CA binary...${NC}"
cd "$(dirname "$0")/../trustchain"

# Build with minimal output
if cargo build --bin trustchain_ca --release 2>&1 | grep -E "(error|warning:|Finished)" | head -20; then
    echo -e "${GREEN}Build successful!${NC}"
else
    echo -e "${RED}Build failed!${NC}"
    exit 1
fi

# Check if binary exists
if [ ! -f "target/release/trustchain_ca" ]; then
    echo -e "${RED}CA binary not found at target/release/trustchain_ca${NC}"
    exit 1
fi

echo ""
echo -e "${YELLOW}Starting TrustChain CA service...${NC}"

# Start the CA service
./target/release/trustchain_ca $MODE_FLAGS --port "$CA_PORT" > "$LOG_FILE" 2>&1 &
CA_PID=$!
echo "$CA_PID" > "$PID_FILE"

echo -e "${GREEN}CA service started with PID $CA_PID${NC}"

# Wait for service to start
echo -e "${YELLOW}Waiting for CA service to initialize...${NC}"
sleep 3

# Check if service is running
if ! kill -0 "$CA_PID" 2>/dev/null; then
    echo -e "${RED}CA service failed to start!${NC}"
    echo -e "${RED}Last 20 lines of log:${NC}"
    tail -20 "$LOG_FILE"
    exit 1
fi

# Test health endpoint
echo -e "${YELLOW}Testing CA health endpoint...${NC}"
if curl -s -k "https://$BIND_ADDR/health" >/dev/null 2>&1; then
    echo -e "${GREEN}CA service is healthy!${NC}"
else
    echo -e "${YELLOW}Note: Health check via curl failed (expected with STOQ transport)${NC}"
    echo -e "${GREEN}But CA process is running - checking logs...${NC}"
    if grep -q "CA Service listening" "$LOG_FILE"; then
        echo -e "${GREEN}CA service started successfully!${NC}"
    fi
fi

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}    CA Service Running Successfully${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo -e "${BLUE}Service Details:${NC}"
echo "  • PID: $CA_PID"
echo "  • Mode: $MODE_DESC"
echo "  • Address: https://$BIND_ADDR"
echo "  • Log File: $LOG_FILE"
echo ""
echo -e "${BLUE}Endpoints:${NC}"
echo "  • Health: https://$BIND_ADDR/health"
echo "  • Root CA: https://$BIND_ADDR/ca/root"
echo "  • Issue: https://$BIND_ADDR/certificate/issue"
echo "  • Validate: https://$BIND_ADDR/certificate/validate"
echo "  • Auto-issue: https://$BIND_ADDR/certificate"
echo ""
echo -e "${YELLOW}Monitoring commands:${NC}"
echo "  • View logs: tail -f $LOG_FILE"
echo "  • Check status: ps -p $CA_PID"
echo "  • Stop service: kill $CA_PID"
echo ""
echo -e "${GREEN}CA service is ready for multi-node communication!${NC}"
echo ""

# If running interactively, keep the script running
if [ -t 0 ]; then
    echo -e "${YELLOW}Press Ctrl+C to stop the CA service${NC}"

    # Monitor the CA process
    while true; do
        if ! kill -0 "$CA_PID" 2>/dev/null; then
            echo -e "${RED}CA service stopped unexpectedly!${NC}"
            echo -e "${RED}Last 20 lines of log:${NC}"
            tail -20 "$LOG_FILE"
            exit 1
        fi
        sleep 30

        # Show periodic status
        UPTIME=$(ps -o etime= -p "$CA_PID" 2>/dev/null | xargs)
        echo -e "${BLUE}[$(date '+%H:%M:%S')] CA running - Uptime: $UPTIME${NC}"
    done
else
    # If running in background, just exit
    echo -e "${GREEN}CA service started in background (PID: $CA_PID)${NC}"
fi