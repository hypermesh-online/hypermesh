#!/bin/bash

# Web3 Ecosystem Service Startup Script
# Starts all services in proper dependency order

set -e
set -o pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
COMPONENTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOGS_DIR="$COMPONENTS_DIR/logs"
PID_DIR="$COMPONENTS_DIR/pids"

log() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

error() {
    echo -e "${RED}[ERROR]${NC} $*"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $*"
}

# Create directories
setup_directories() {
    mkdir -p "$LOGS_DIR" "$PID_DIR"
    log "Created logs and PID directories"
}

# Function to start a service
start_service() {
    local service_name="$1"
    local binary_path="$2"
    local args="${3:-}"
    local port="${4:-}"
    
    log "Starting $service_name..."
    
    # Check if binary exists
    if [[ ! -f "$binary_path" ]]; then
        warn "$service_name binary not found at $binary_path - skipping"
        return 1
    fi
    
    # Check if port is already in use
    if [[ -n "$port" ]] && netstat -tuln 2>/dev/null | grep -q ":$port "; then
        warn "Port $port is already in use - $service_name may already be running"
        return 1
    fi
    
    # Start the service
    nohup "$binary_path" $args > "$LOGS_DIR/$service_name.log" 2>&1 & 
    local pid=$!
    echo $pid > "$PID_DIR/$service_name.pid"
    
    # Brief wait to check if service started successfully
    sleep 2
    if kill -0 $pid 2>/dev/null; then
        success "$service_name started (PID: $pid)"
        if [[ -n "$port" ]]; then
            log "$service_name listening on port $port"
        fi
        return 0
    else
        error "$service_name failed to start - check $LOGS_DIR/$service_name.log"
        return 1
    fi
}

# Function to check service status
check_service_status() {
    local service_name="$1"
    local pid_file="$PID_DIR/$service_name.pid"
    
    if [[ -f "$pid_file" ]]; then
        local pid=$(cat "$pid_file")
        if kill -0 $pid 2>/dev/null; then
            success "$service_name is running (PID: $pid)"
        else
            warn "$service_name PID file exists but process is not running"
            rm -f "$pid_file"
        fi
    else
        warn "$service_name is not running"
    fi
}

# Function to stop all services
stop_all_services() {
    log "Stopping all services..."
    
    for pid_file in "$PID_DIR"/*.pid; do
        if [[ -f "$pid_file" ]]; then
            local service_name=$(basename "$pid_file" .pid)
            local pid=$(cat "$pid_file")
            
            if kill -0 $pid 2>/dev/null; then
                log "Stopping $service_name (PID: $pid)..."
                kill -TERM $pid
                sleep 2
                
                # Force kill if still running
                if kill -0 $pid 2>/dev/null; then
                    warn "Force killing $service_name..."
                    kill -KILL $pid
                fi
            fi
            
            rm -f "$pid_file"
        fi
    done
    
    success "All services stopped"
}

# Main startup process
start_ecosystem() {
    echo "=========================================="
    echo "   Web3 Ecosystem Service Startup"
    echo "=========================================="
    
    setup_directories
    
    log "Starting services in dependency order..."
    
    # Phase 1: Foundation Layer - STOQ Transport
    log "=== Phase 1: Foundation Layer ==="
    # Note: STOQ doesn't have a standalone server binary yet
    log "STOQ Transport Protocol - embedded in other services"
    
    # Phase 2: Certificate Layer - TrustChain
    log "=== Phase 2: Certificate Layer ==="
    start_service "trustchain-server" \
        "$COMPONENTS_DIR/trustchain/target/release/trustchain-server" \
        "--mode development --bind [::1]:8443" \
        "8443"
    
    start_service "trustchain-simple" \
        "$COMPONENTS_DIR/trustchain/target/release/trustchain-simple" \
        "--port 8444" \
        "8444"
    
    # Phase 3: Orchestration Layer - HyperMesh  
    log "=== Phase 3: Orchestration Layer ==="
    warn "HyperMesh compilation failed due to ML dependencies - will run when fixed"
    
    # Phase 4: Application Layer - Catalog
    log "=== Phase 4: Application Layer ==="
    if [[ -f "$COMPONENTS_DIR/catalog/target/release/catalog" ]]; then
        start_service "catalog" \
            "$COMPONENTS_DIR/catalog/target/release/catalog" \
            "--port 8445" \
            "8445"
    else
        warn "Catalog binary not found - may need to build without HyperMesh dependency"
    fi
    
    # Phase 5: Economics Layer - Caesar
    log "=== Phase 5: Economics Layer ==="
    if command -v npm &> /dev/null && [[ -d "$COMPONENTS_DIR/caesar" ]]; then
        cd "$COMPONENTS_DIR/caesar"
        if [[ -f "package.json" ]]; then
            log "Starting Caesar economics platform..."
            nohup npm start > "$LOGS_DIR/caesar.log" 2>&1 &
            local caesar_pid=$!
            echo $caesar_pid > "$PID_DIR/caesar.pid"
            success "Caesar started (PID: $caesar_pid)"
        fi
    else
        warn "Caesar not available (npm not found or directory missing)"
    fi
    
    echo "=========================================="
    log "Service startup complete!"
    echo "=========================================="
    
    # Show status of all services
    log "Service Status:"
    check_service_status "trustchain-server"
    check_service_status "trustchain-simple"
    check_service_status "catalog"
    check_service_status "caesar"
    
    log ""
    log "Logs are available in: $LOGS_DIR/"
    log "PIDs are tracked in: $PID_DIR/"
    log ""
    log "To stop all services: $0 stop"
    log "To check status: $0 status"
    log ""
    log "TrustChain CA Server: https://[::1]:8443"
    log "TrustChain Simple Server: https://[::1]:8444"
    log "Catalog Service: http://[::1]:8445"
}

# Function to show service status
show_status() {
    echo "=========================================="
    echo "   Web3 Ecosystem Service Status"
    echo "=========================================="
    
    check_service_status "trustchain-server"
    check_service_status "trustchain-simple"
    check_service_status "catalog"
    check_service_status "caesar"
    
    echo ""
    log "Recent log entries:"
    for log_file in "$LOGS_DIR"/*.log; do
        if [[ -f "$log_file" ]]; then
            local service_name=$(basename "$log_file" .log)
            log "=== $service_name ==="
            tail -n 3 "$log_file" 2>/dev/null || echo "No recent logs"
            echo ""
        fi
    done
}

# Handle script arguments
case "${1:-start}" in
    "start")
        start_ecosystem
        ;;
    "stop")
        stop_all_services
        ;;
    "restart")
        stop_all_services
        sleep 3
        start_ecosystem
        ;;
    "status")
        show_status
        ;;
    "logs")
        if [[ -n "$2" ]]; then
            tail -f "$LOGS_DIR/$2.log"
        else
            log "Available logs:"
            ls -la "$LOGS_DIR"/*.log 2>/dev/null || log "No log files found"
        fi
        ;;
    "help"|"-h"|"--help")
        echo "Usage: $0 [start|stop|restart|status|logs [service]|help]"
        echo ""
        echo "Commands:"
        echo "  start     Start all services (default)"
        echo "  stop      Stop all running services"
        echo "  restart   Stop and start all services"
        echo "  status    Show status of all services"
        echo "  logs      Show available logs or tail specific service log"
        echo "  help      Show this help message"
        echo ""
        echo "Examples:"
        echo "  $0                    # Start all services"
        echo "  $0 logs trustchain   # Follow TrustChain logs"
        echo "  $0 status            # Check service status"
        exit 0
        ;;
    *)
        error "Unknown command: $1"
        echo "Use '$0 help' for usage information"
        exit 1
        ;;
esac