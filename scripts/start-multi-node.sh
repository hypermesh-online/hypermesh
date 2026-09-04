#!/bin/bash
#
# Start Multi-Node Block-MATRIX Network
#
# This script starts multiple BlockMatrix nodes that discover and communicate with each other
# using the STOQ protocol and matrix topology-based neighbor discovery.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BASE_PORT=9292
LOG_DIR="/tmp/blockmatrix-nodes"
PID_FILE="/tmp/blockmatrix-nodes.pid"

# Clean up function
cleanup() {
    echo -e "${YELLOW}Cleaning up nodes...${NC}"
    if [ -f "$PID_FILE" ]; then
        while IFS= read -r pid; do
            if kill -0 "$pid" 2>/dev/null; then
                echo "Stopping node with PID $pid"
                kill "$pid" 2>/dev/null || true
            fi
        done < "$PID_FILE"
        rm -f "$PID_FILE"
    fi
    echo -e "${GREEN}Cleanup complete${NC}"
}

# Set up trap for cleanup
trap cleanup EXIT

# Parse arguments
NUM_NODES=${1:-3}
PRIVACY_MODE=${2:-"public"}

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}   Block-MATRIX Multi-Node Network${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo -e "${GREEN}Starting $NUM_NODES nodes in $PRIVACY_MODE mode${NC}"
echo ""

# Create log directory
mkdir -p "$LOG_DIR"
rm -f "$PID_FILE"

# Build the node binary
echo -e "${YELLOW}Building node binary...${NC}"
cd "$(dirname "$0")/../blockmatrix"
cargo build --bin node --release 2>&1 | tail -5

# Check if build succeeded (check both debug and release)
if [ -f "../target/release/node" ]; then
    NODE_BIN="../target/release/node"
elif [ -f "../target/debug/node" ]; then
    NODE_BIN="../target/debug/node"
elif [ -f "target/release/node" ]; then
    NODE_BIN="target/release/node"
elif [ -f "target/debug/node" ]; then
    NODE_BIN="target/debug/node"
else
    echo -e "${RED}Failed to find node binary${NC}"
    exit 1
fi
echo -e "${GREEN}Using node binary: $NODE_BIN${NC}"

echo -e "${GREEN}Build successful!${NC}"
echo ""

# Start the first node (bootstrap node)
BOOTSTRAP_PORT=$BASE_PORT
BOOTSTRAP_ADDR="[::1]:$BOOTSTRAP_PORT"

echo -e "${YELLOW}Starting bootstrap node (0,0,0) on port $BOOTSTRAP_PORT${NC}"
"$NODE_BIN" \
    -x 0 -y 0 -z 0 \
    -p "$PRIVACY_MODE" \
    -s "$BOOTSTRAP_PORT" \
    start \
    > "$LOG_DIR/node-0.log" 2>&1 &

BOOTSTRAP_PID=$!
echo "$BOOTSTRAP_PID" >> "$PID_FILE"
echo -e "${GREEN}Bootstrap node started with PID $BOOTSTRAP_PID${NC}"

# Wait for bootstrap node to start
echo -e "${YELLOW}Waiting for bootstrap node to initialize...${NC}"
sleep 3

# Check if bootstrap node is running
if ! kill -0 "$BOOTSTRAP_PID" 2>/dev/null; then
    echo -e "${RED}Bootstrap node failed to start. Check logs at $LOG_DIR/node-0.log${NC}"
    tail -20 "$LOG_DIR/node-0.log"
    exit 1
fi

echo -e "${GREEN}Bootstrap node is running${NC}"
echo ""

# Start additional nodes
for i in $(seq 1 $((NUM_NODES - 1))); do
    NODE_PORT=$((BASE_PORT + i))

    # Generate matrix coordinates (simple pattern for demo)
    X=$i
    Y=$((i * 2))
    Z=$((i % 3))

    echo -e "${YELLOW}Starting node $i at ($X,$Y,$Z) on port $NODE_PORT${NC}"

    "$NODE_BIN" \
        -x "$X" -y "$Y" -z "$Z" \
        -p "$PRIVACY_MODE" \
        -s "$NODE_PORT" \
        -b "$BOOTSTRAP_ADDR" \
        start \
        > "$LOG_DIR/node-$i.log" 2>&1 &

    NODE_PID=$!
    echo "$NODE_PID" >> "$PID_FILE"
    echo -e "${GREEN}Node $i started with PID $NODE_PID${NC}"

    # Small delay between node starts
    sleep 2
done

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}   All nodes started successfully!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo -e "${BLUE}Network Status:${NC}"
echo "  • Nodes running: $NUM_NODES"
echo "  • Privacy mode: $PRIVACY_MODE"
echo "  • Bootstrap node: [::1]:$BOOTSTRAP_PORT"
echo "  • Log directory: $LOG_DIR"
echo ""
echo -e "${YELLOW}Monitoring commands:${NC}"
echo "  • View logs: tail -f $LOG_DIR/node-*.log"
echo "  • Check connections: grep 'Connected' $LOG_DIR/*.log"
echo "  • See neighbors: grep 'neighbor' $LOG_DIR/*.log"
echo ""

# Monitor for initial connections
echo -e "${YELLOW}Waiting for nodes to discover each other...${NC}"
sleep 10

# Check for connections
echo -e "${BLUE}Checking node connections:${NC}"
for i in $(seq 0 $((NUM_NODES - 1))); do
    if [ -f "$LOG_DIR/node-$i.log" ]; then
        echo -e "${GREEN}Node $i:${NC}"
        grep -E "(Connected|neighbor|discovered)" "$LOG_DIR/node-$i.log" | tail -3 || echo "  No connections yet"
    fi
done

echo ""
echo -e "${GREEN}Multi-node network is running!${NC}"
echo -e "${YELLOW}Press Ctrl+C to stop all nodes${NC}"
echo ""

# Keep script running to maintain trap
while true; do
    # Check if nodes are still running
    RUNNING=0
    if [ -f "$PID_FILE" ]; then
        while IFS= read -r pid; do
            if kill -0 "$pid" 2>/dev/null; then
                RUNNING=$((RUNNING + 1))
            fi
        done < "$PID_FILE"
    fi

    if [ "$RUNNING" -eq 0 ]; then
        echo -e "${RED}All nodes have stopped${NC}"
        break
    fi

    sleep 30

    # Periodic status update
    echo -e "${BLUE}[$(date '+%H:%M:%S')] Nodes running: $RUNNING/$NUM_NODES${NC}"
done