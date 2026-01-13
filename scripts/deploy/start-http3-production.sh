#!/bin/bash
# Production HTTP/3 Server Deployment Script
# Starts TrustChain and BlockMatrix HTTP/3 servers with production settings

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Production configuration
LOG_DIR="/var/log/http3"
PID_DIR="/var/run/http3"
BINARY_DIR="/home/persist/repos/projects/web3/target/release"
PROJECT_DIR="/home/persist/repos/projects/web3"

# Create required directories - use user directories by default
LOG_DIR="$HOME/.http3/logs"
PID_DIR="$HOME/.http3/pids"
mkdir -p "$LOG_DIR" "$PID_DIR"

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}       HTTP/3 Production Server Deployment${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}\n"

# Function to check if port is available
check_port() {
    local port=$1
    # Check for both TCP and UDP since HTTP/3 uses UDP
    if lsof -Pi ":$port" -t >/dev/null 2>&1 || ss -ulnp 2>/dev/null | grep -q ":$port"; then
        echo -e "${RED}✗ Port $port is already in use${NC}"
        return 1
    fi
    return 0
}

# Function to build servers if needed
build_servers() {
    echo -e "${YELLOW}Building HTTP/3 servers in release mode...${NC}"

    cd "$PROJECT_DIR/trustchain"
    if cargo build --release --bin trustchain-http3-server 2>&1 | tail -5; then
        echo -e "${GREEN}✓ TrustChain server built${NC}"
    else
        echo -e "${RED}✗ Failed to build TrustChain server${NC}"
        exit 1
    fi

    cd "$PROJECT_DIR/blockmatrix"
    if cargo build --release --bin blockmatrix-http3-server 2>&1 | tail -5; then
        echo -e "${GREEN}✓ BlockMatrix server built${NC}"
    else
        echo -e "${RED}✗ Failed to build BlockMatrix server${NC}"
        exit 1
    fi
}

# Function to stop existing servers
stop_existing_servers() {
    echo -e "${YELLOW}Checking for existing servers...${NC}"

    # Stop existing processes
    if [ -f "$PID_DIR/trustchain.pid" ]; then
        local old_pid=$(cat "$PID_DIR/trustchain.pid")
        if kill -0 "$old_pid" 2>/dev/null; then
            echo -e "  Stopping existing TrustChain server (PID: $old_pid)"
            kill "$old_pid" 2>/dev/null || true
            sleep 1
        fi
    fi

    if [ -f "$PID_DIR/blockmatrix.pid" ]; then
        local old_pid=$(cat "$PID_DIR/blockmatrix.pid")
        if kill -0 "$old_pid" 2>/dev/null; then
            echo -e "  Stopping existing BlockMatrix server (PID: $old_pid)"
            kill "$old_pid" 2>/dev/null || true
            sleep 1
        fi
    fi

    # Fallback: kill any remaining processes
    pkill -f 'trustchain-http3-server' 2>/dev/null || true
    pkill -f 'blockmatrix-http3-server' 2>/dev/null || true
    sleep 1
}

# Function to start a server with monitoring
start_server() {
    local name=$1
    local binary=$2
    local port=$3
    local pid_file=$4
    local log_file=$5
    local work_dir=$6

    echo -e "${BLUE}Starting $name server...${NC}"

    # Check port availability
    if ! check_port "$port"; then
        echo -e "${RED}✗ Cannot start $name - port $port is in use${NC}"
        return 1
    fi

    # Start server with production settings
    cd "$work_dir"
    export RUST_LOG=info,quinn=warn,h3=warn
    export RUST_BACKTRACE=1

    nohup "$binary" > "$log_file" 2>&1 &
    local pid=$!

    # Save PID
    echo $pid > "$pid_file"

    # Wait for server to initialize
    local max_wait=10
    local wait_count=0
    while [ $wait_count -lt $max_wait ]; do
        if kill -0 $pid 2>/dev/null; then
            # Check if server is listening (HTTP/3 uses UDP not TCP)
            if lsof -Pi ":$port" -t >/dev/null 2>&1 || ss -ulnp 2>/dev/null | grep -q ":$port"; then
                echo -e "${GREEN}✓ $name server started (PID: $pid, Port: $port)${NC}"
                return 0
            fi
        else
            echo -e "${RED}✗ $name server failed to start (check logs at $log_file)${NC}"
            tail -n 20 "$log_file"
            return 1
        fi
        sleep 1
        wait_count=$((wait_count + 1))
    done

    echo -e "${RED}✗ $name server startup timeout${NC}"
    return 1
}

# Function to setup automatic restart
setup_auto_restart() {
    local name=$1
    local binary=$2
    local port=$3
    local pid_file=$4
    local log_file=$5
    local work_dir=$6

    # Create restart script
    cat > "$PID_DIR/${name,,}-restart.sh" << EOF
#!/bin/bash
# Auto-restart script for $name HTTP/3 server
while true; do
    if [ -f "$pid_file" ]; then
        PID=\$(cat "$pid_file")
        if ! kill -0 \$PID 2>/dev/null; then
            echo "\$(date): $name server crashed, restarting..." >> "$LOG_DIR/restarts.log"
            cd "$work_dir"
            export RUST_LOG=info,quinn=warn,h3=warn
            export RUST_BACKTRACE=1
            "$binary" >> "$log_file" 2>&1 &
            echo \$! > "$pid_file"
        fi
    fi
    sleep 5
done
EOF
    chmod +x "$PID_DIR/${name,,}-restart.sh"

    # Start monitor in background
    nohup "$PID_DIR/${name,,}-restart.sh" > /dev/null 2>&1 &
    echo $! > "$PID_DIR/${name,,}-monitor.pid"
}

# Main execution
main() {
    # 1. Build servers if needed
    if [ ! -f "$BINARY_DIR/trustchain-http3-server" ] || [ ! -f "$BINARY_DIR/blockmatrix-http3-server" ]; then
        build_servers
    fi

    # 2. Stop existing servers
    stop_existing_servers

    # 3. Start TrustChain server
    if ! start_server "TrustChain" \
         "$BINARY_DIR/trustchain-http3-server" \
         50053 \
         "$PID_DIR/trustchain.pid" \
         "$LOG_DIR/trustchain.log" \
         "$PROJECT_DIR/trustchain"; then
        echo -e "${RED}Failed to start TrustChain server${NC}"
        exit 1
    fi

    # 4. Start BlockMatrix server
    if ! start_server "BlockMatrix" \
         "$BINARY_DIR/blockmatrix-http3-server" \
         8446 \
         "$PID_DIR/blockmatrix.pid" \
         "$LOG_DIR/blockmatrix.log" \
         "$PROJECT_DIR/blockmatrix"; then
        echo -e "${RED}Failed to start BlockMatrix server${NC}"
        exit 1
    fi

    # 5. Setup automatic restart monitors
    setup_auto_restart "TrustChain" \
         "$BINARY_DIR/trustchain-http3-server" \
         50053 \
         "$PID_DIR/trustchain.pid" \
         "$LOG_DIR/trustchain.log" \
         "$PROJECT_DIR/trustchain"

    setup_auto_restart "BlockMatrix" \
         "$BINARY_DIR/blockmatrix-http3-server" \
         8446 \
         "$PID_DIR/blockmatrix.pid" \
         "$LOG_DIR/blockmatrix.log" \
         "$PROJECT_DIR/blockmatrix"

    # 6. Run health check
    echo -e "\n${YELLOW}Running health checks...${NC}"
    sleep 2

    if "$PROJECT_DIR/validate-http3-health.sh"; then
        echo -e "${GREEN}✓ Health checks passed${NC}"
    else
        echo -e "${YELLOW}⚠ Health checks failed (servers may still be initializing)${NC}"
    fi

    # 7. Display status
    echo -e "\n${BLUE}═══════════════════════════════════════════════════════${NC}"
    echo -e "${GREEN}       HTTP/3 Servers Running in Production${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}\n"

    echo -e "${GREEN}Endpoints:${NC}"
    echo -e "  TrustChain:  https://[::1]:50053/health"
    echo -e "  BlockMatrix: https://[::1]:8446/api/v1/blockmatrix/health"
    echo ""
    echo -e "${GREEN}Management:${NC}"
    echo -e "  Logs:     tail -f $LOG_DIR/*.log"
    echo -e "  PIDs:     $PID_DIR/"
    echo -e "  Stop:     $PROJECT_DIR/stop-http3-production.sh"
    echo -e "  Health:   $PROJECT_DIR/validate-http3-health.sh"
    echo ""
    echo -e "${GREEN}Monitoring:${NC}"
    echo -e "  Auto-restart enabled with 5-second checks"
    echo -e "  Restart logs: $LOG_DIR/restarts.log"
    echo ""
}

# Run main function
main "$@"