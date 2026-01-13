#!/bin/bash
# Stop production HTTP/3 servers gracefully

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PID_DIR="/var/run/http3"
LOG_DIR="/var/log/http3"

# Check for user directories
if [ ! -d "$PID_DIR" ]; then
    PID_DIR="$HOME/.http3/pids"
fi

if [ ! -d "$LOG_DIR" ]; then
    LOG_DIR="$HOME/.http3/logs"
fi

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}       Stopping HTTP/3 Production Servers${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}\n"

# Function to stop a server
stop_server() {
    local name=$1
    local pid_file=$2
    local monitor_pid_file=$3

    echo -e "${YELLOW}Stopping $name server...${NC}"

    # Stop monitor first
    if [ -f "$monitor_pid_file" ]; then
        local monitor_pid=$(cat "$monitor_pid_file")
        if kill -0 "$monitor_pid" 2>/dev/null; then
            echo -e "  Stopping auto-restart monitor (PID: $monitor_pid)"
            kill "$monitor_pid" 2>/dev/null || true
        fi
        rm -f "$monitor_pid_file"
    fi

    # Stop server
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 "$pid" 2>/dev/null; then
            echo -e "  Sending SIGTERM to $name (PID: $pid)"
            kill -TERM "$pid" 2>/dev/null || true

            # Wait for graceful shutdown (max 10 seconds)
            local wait_count=0
            while [ $wait_count -lt 10 ] && kill -0 "$pid" 2>/dev/null; do
                sleep 1
                wait_count=$((wait_count + 1))
            done

            # Force kill if still running
            if kill -0 "$pid" 2>/dev/null; then
                echo -e "  ${YELLOW}Forcing shutdown with SIGKILL${NC}"
                kill -KILL "$pid" 2>/dev/null || true
            fi

            echo -e "${GREEN}✓ $name server stopped${NC}"
        else
            echo -e "  $name server not running (stale PID file)"
        fi
        rm -f "$pid_file"
    else
        echo -e "  No PID file found for $name"
    fi

    # Clean up restart script
    rm -f "$PID_DIR/${name,,}-restart.sh"
}

# Stop monitors and servers
stop_server "TrustChain" "$PID_DIR/trustchain.pid" "$PID_DIR/trustchain-monitor.pid"
stop_server "BlockMatrix" "$PID_DIR/blockmatrix.pid" "$PID_DIR/blockmatrix-monitor.pid"

# Final cleanup - ensure no orphaned processes
echo -e "\n${YELLOW}Cleaning up any orphaned processes...${NC}"
pkill -f 'trustchain-http3-server' 2>/dev/null || true
pkill -f 'blockmatrix-http3-server' 2>/dev/null || true

# Log the shutdown
echo "$(date): HTTP/3 servers stopped" >> "$LOG_DIR/shutdown.log"

echo -e "\n${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}       All HTTP/3 Servers Stopped${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}\n"

echo -e "${GREEN}Shutdown complete.${NC}"
echo -e "Logs preserved at: $LOG_DIR/"
echo ""