#!/bin/bash
# HTTP/3 Server Stack Health Check Script
# Monitors status of Gateway, BlockMatrix, and TrustChain servers

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

LOG_DIR="/tmp/hypermesh-logs"

echo -e "${BLUE}╔════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║    HTTP/3 Server Stack Health Check            ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════╝${NC}"
echo ""

# Process Status Check
echo -e "${YELLOW}[Process Status]${NC}"
GATEWAY_PID=$(pgrep -f "target/release/gateway" 2>/dev/null)
BLOCKMATRIX_PID=$(pgrep -f "blockmatrix-http3-server" 2>/dev/null)
TRUSTCHAIN_PID=$(pgrep -f "trustchain-http3-server" 2>/dev/null)

if [ -n "$GATEWAY_PID" ]; then
    echo -e "  ${GREEN}✅${NC} Gateway         PID: $GATEWAY_PID"
else
    echo -e "  ${RED}❌${NC} Gateway         NOT RUNNING"
fi

if [ -n "$BLOCKMATRIX_PID" ]; then
    echo -e "  ${GREEN}✅${NC} BlockMatrix     PID: $BLOCKMATRIX_PID"
else
    echo -e "  ${RED}❌${NC} BlockMatrix     NOT RUNNING"
fi

if [ -n "$TRUSTCHAIN_PID" ]; then
    echo -e "  ${GREEN}✅${NC} TrustChain      PID: $TRUSTCHAIN_PID"
else
    echo -e "  ${RED}❌${NC} TrustChain      NOT RUNNING"
fi

echo ""

# Port Status Check
echo -e "${YELLOW}[Network Ports]${NC}"
if ss -uln | grep -q ":8443 "; then
    GATEWAY_ADDR=$(ss -uln | grep ":8443 " | awk '{print $5}')
    echo -e "  ${GREEN}✅${NC} Gateway         $GATEWAY_ADDR (UDP/QUIC)"
else
    echo -e "  ${RED}❌${NC} Gateway         Port 8443 not listening"
fi

if ss -uln | grep -q ":8446 "; then
    BLOCKMATRIX_ADDR=$(ss -uln | grep ":8446 " | awk '{print $5}')
    echo -e "  ${GREEN}✅${NC} BlockMatrix     $BLOCKMATRIX_ADDR (UDP/QUIC)"
else
    echo -e "  ${RED}❌${NC} BlockMatrix     Port 8446 not listening"
fi

if ss -uln | grep -q ":50053 "; then
    TRUSTCHAIN_ADDR=$(ss -uln | grep ":50053 " | awk '{print $5}')
    echo -e "  ${GREEN}✅${NC} TrustChain      $TRUSTCHAIN_ADDR (UDP/QUIC)"
else
    echo -e "  ${RED}❌${NC} TrustChain      Port 50053 not listening"
fi

echo ""

# Resource Usage
echo -e "${YELLOW}[Resource Usage]${NC}"
if [ -n "$GATEWAY_PID" ]; then
    GATEWAY_MEM=$(ps -o vsz= -p $GATEWAY_PID 2>/dev/null | awk '{print int($1/1024)"MB"}')
    GATEWAY_CPU=$(ps -o %cpu= -p $GATEWAY_PID 2>/dev/null)
    echo -e "  Gateway:     Memory: ${GATEWAY_MEM:-N/A}, CPU: ${GATEWAY_CPU:-N/A}%"
fi

if [ -n "$BLOCKMATRIX_PID" ]; then
    BLOCKMATRIX_MEM=$(ps -o vsz= -p $BLOCKMATRIX_PID 2>/dev/null | awk '{print int($1/1024)"MB"}')
    BLOCKMATRIX_CPU=$(ps -o %cpu= -p $BLOCKMATRIX_PID 2>/dev/null)
    echo -e "  BlockMatrix: Memory: ${BLOCKMATRIX_MEM:-N/A}, CPU: ${BLOCKMATRIX_CPU:-N/A}%"
fi

if [ -n "$TRUSTCHAIN_PID" ]; then
    TRUSTCHAIN_MEM=$(ps -o vsz= -p $TRUSTCHAIN_PID 2>/dev/null | awk '{print int($1/1024)"MB"}')
    TRUSTCHAIN_CPU=$(ps -o %cpu= -p $TRUSTCHAIN_PID 2>/dev/null)
    echo -e "  TrustChain:  Memory: ${TRUSTCHAIN_MEM:-N/A}, CPU: ${TRUSTCHAIN_CPU:-N/A}%"
fi

echo ""

# Recent Log Activity
echo -e "${YELLOW}[Recent Log Activity]${NC}"

echo "Gateway (last 3 lines):"
if [ -f "$LOG_DIR/gateway.log" ]; then
    tail -3 "$LOG_DIR/gateway.log" | sed 's/^/  /'
else
    echo "  No log file found"
fi

echo ""
echo "BlockMatrix (last 3 lines):"
if [ -f "$LOG_DIR/blockmatrix.log" ]; then
    tail -3 "$LOG_DIR/blockmatrix.log" | sed 's/^/  /'
else
    echo "  No log file found"
fi

echo ""
echo "TrustChain (last 3 lines):"
if [ -f "$LOG_DIR/trustchain.log" ]; then
    tail -3 "$LOG_DIR/trustchain.log" | sed 's/^/  /'
else
    echo "  No log file found"
fi

echo ""

# Error Check
echo -e "${YELLOW}[Recent Errors]${NC}"
ERROR_COUNT=0

if [ -f "$LOG_DIR/gateway.log" ]; then
    GATEWAY_ERRORS=$(tail -100 "$LOG_DIR/gateway.log" | grep -i error | tail -1)
    if [ -n "$GATEWAY_ERRORS" ]; then
        echo -e "  ${RED}Gateway:${NC}"
        echo "    $GATEWAY_ERRORS"
        ((ERROR_COUNT++))
    fi
fi

if [ -f "$LOG_DIR/blockmatrix.log" ]; then
    BLOCKMATRIX_ERRORS=$(tail -100 "$LOG_DIR/blockmatrix.log" | grep -i error | tail -1)
    if [ -n "$BLOCKMATRIX_ERRORS" ]; then
        echo -e "  ${RED}BlockMatrix:${NC}"
        echo "    $BLOCKMATRIX_ERRORS"
        ((ERROR_COUNT++))
    fi
fi

if [ -f "$LOG_DIR/trustchain.log" ]; then
    TRUSTCHAIN_ERRORS=$(tail -100 "$LOG_DIR/trustchain.log" | grep -i error | tail -1)
    if [ -n "$TRUSTCHAIN_ERRORS" ]; then
        echo -e "  ${RED}TrustChain:${NC}"
        echo "    $TRUSTCHAIN_ERRORS"
        ((ERROR_COUNT++))
    fi
fi

if [ $ERROR_COUNT -eq 0 ]; then
    echo -e "  ${GREEN}✅ No recent errors detected${NC}"
fi

echo ""

# Overall Health Status
echo -e "${YELLOW}[Overall Status]${NC}"
ALL_RUNNING=true
if [ -z "$GATEWAY_PID" ] || [ -z "$BLOCKMATRIX_PID" ] || [ -z "$TRUSTCHAIN_PID" ]; then
    ALL_RUNNING=false
fi

ALL_PORTS=true
if ! ss -uln | grep -q ":8443 " || ! ss -uln | grep -q ":8446 " || ! ss -uln | grep -q ":50053 "; then
    ALL_PORTS=false
fi

if [ "$ALL_RUNNING" = true ] && [ "$ALL_PORTS" = true ] && [ $ERROR_COUNT -eq 0 ]; then
    echo -e "  ${GREEN}✅ HEALTHY - All services operational${NC}"
elif [ "$ALL_RUNNING" = true ] && [ "$ALL_PORTS" = true ]; then
    echo -e "  ${YELLOW}⚠️  WARNING - Services running with errors${NC}"
else
    echo -e "  ${RED}❌ UNHEALTHY - Services need attention${NC}"
fi

echo ""
echo -e "${BLUE}────────────────────────────────────────────────${NC}"
echo "Quick Actions:"
echo "  Deploy:  /home/persist/repos/projects/web3/deploy-http3-stack.sh"
echo "  Logs:    tail -f $LOG_DIR/{gateway,blockmatrix,trustchain}.log"
echo "  Stop:    pkill -f 'gateway|blockmatrix-http3-server|trustchain-http3-server'"