#!/bin/bash
# HTTP/3 Stack Continuous Monitoring Script
# Monitors health and restarts failed services

LOG_DIR="/tmp/hypermesh-logs"
CHECK_INTERVAL=30  # Check every 30 seconds
RESTART_THRESHOLD=3  # Restart after 3 consecutive failures

# Counters for consecutive failures
GATEWAY_FAILURES=0
BLOCKMATRIX_FAILURES=0
TRUSTCHAIN_FAILURES=0

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

restart_service() {
    local service=$1
    local binary=$2
    local port=$3
    local workdir=$4

    echo -e "${YELLOW}Restarting $service...${NC}"
    pkill -f "$binary" || true
    sleep 2

    cd "$workdir"
    RUST_LOG=info nohup "/home/persist/repos/projects/web3/target/release/$binary" \
        > "$LOG_DIR/${service,,}.log" 2>&1 &

    sleep 3
    if pgrep -f "$binary" > /dev/null; then
        echo -e "${GREEN}✅ $service restarted successfully${NC}"
        return 0
    else
        echo -e "${RED}❌ Failed to restart $service${NC}"
        return 1
    fi
}

check_service() {
    local service=$1
    local process=$2
    local port=$3
    local failure_var=$4
    local workdir=$5
    local binary=$6

    local is_healthy=true

    # Check process
    if ! pgrep -f "$process" > /dev/null; then
        is_healthy=false
    fi

    # Check port
    if ! ss -uln | grep -q ":$port "; then
        is_healthy=false
    fi

    if [ "$is_healthy" = false ]; then
        eval "$failure_var=\$((\$$failure_var + 1))"
        local failures=$(eval echo \$$failure_var)

        echo -e "${RED}⚠️  $service unhealthy (failures: $failures/$RESTART_THRESHOLD)${NC}"

        if [ $failures -ge $RESTART_THRESHOLD ]; then
            restart_service "$service" "$binary" "$port" "$workdir"
            if [ $? -eq 0 ]; then
                eval "$failure_var=0"
            fi
        fi
    else
        eval "$failure_var=0"
        echo -e "${GREEN}✅ $service healthy${NC}"
    fi
}

echo -e "${GREEN}Starting HTTP/3 Stack Monitoring${NC}"
echo "Check interval: ${CHECK_INTERVAL}s"
echo "Restart threshold: $RESTART_THRESHOLD failures"
echo ""

while true; do
    echo -e "\n${YELLOW}[$(date '+%Y-%m-%d %H:%M:%S')]${NC} Health Check"

    check_service "Gateway" \
        "target/release/gateway" \
        "8443" \
        "GATEWAY_FAILURES" \
        "/home/persist/repos/projects/web3/gateway" \
        "gateway"

    check_service "BlockMatrix" \
        "blockmatrix-http3-server" \
        "8446" \
        "BLOCKMATRIX_FAILURES" \
        "/home/persist/repos/projects/web3/blockmatrix" \
        "blockmatrix-http3-server"

    check_service "TrustChain" \
        "trustchain-http3-server" \
        "50053" \
        "TRUSTCHAIN_FAILURES" \
        "/home/persist/repos/projects/web3/trustchain" \
        "trustchain-http3-server"

    # Log error summary
    ERRORS=$(grep -c ERROR "$LOG_DIR"/*.log 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    echo "Total errors in logs: $ERRORS"

    sleep $CHECK_INTERVAL
done