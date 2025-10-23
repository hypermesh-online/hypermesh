#!/bin/bash
# Hello World Cluster Status Script
# Check the status of your Nexus cluster

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NEXUS_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"

DETAILED=false
WATCH_MODE=false
SHOW_METRICS=false

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'  
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

title() {
    echo -e "${BLUE}===== $1 =====${NC}"
}

usage() {
    cat << EOF
Hypermesh Nexus Hello World - Cluster Status

USAGE:
    $0 [OPTIONS]

OPTIONS:
    --detailed         Show detailed cluster information
    --watch           Watch real-time status updates
    --show-metrics    Include performance metrics
    --help            Show this help message

EXAMPLES:
    $0                           # Quick status check
    $0 --detailed                # Detailed cluster info
    $0 --watch                   # Real-time status updates
    $0 --watch --show-metrics    # Watch with performance data

EOF
}

check_cluster_status() {
    title "Cluster Status Check"
    
    cd "$NEXUS_ROOT/core/deploy"
    
    log "Checking deployment status..."
    
    if [ -f "deploy.sh" ] && ./deploy.sh status > /dev/null 2>&1; then
        log "✅ Cluster is running"
        
        if [ "$DETAILED" = true ] || [ "$WATCH_MODE" = true ]; then
            echo ""
            ./deploy.sh status
        fi
        
        return 0
    else
        warn "⚠️  Cluster may not be running or not found"
        log "Try running: $SCRIPT_DIR/deploy.sh"
        return 1
    fi
}

run_health_checks() {
    if [ "$DETAILED" = true ] || [ "$WATCH_MODE" = true ]; then
        title "Health Checks"
        
        cd "$NEXUS_ROOT/core/tests"
        
        log "Running quick health validation..."
        
        # Run basic health check
        if cargo run --bin nexus-test -- unit --parallel > /dev/null 2>&1; then
            log "✅ All core components healthy"
        else
            warn "⚠️  Some components may have issues"
            log "Run detailed check: cargo run --bin nexus-test -- all"
        fi
    fi
}

show_performance_metrics() {
    if [ "$SHOW_METRICS" = true ]; then
        title "Performance Metrics"
        
        cd "$NEXUS_ROOT/core/tests"
        
        log "Collecting current performance data..."
        
        # Run 10-second metrics collection
        cargo run --bin nexus-test -- metrics --duration 10 2>/dev/null || {
            warn "⚠️  Could not collect metrics - cluster may be offline"
        }
    fi
}

show_quick_status() {
    title "Quick Status"
    
    # Check if processes are running
    local running_processes=0
    
    # Look for nexus processes
    if pgrep -f "nexus" > /dev/null 2>&1; then
        running_processes=$(pgrep -f "nexus" | wc -l)
        log "✅ Found $running_processes Nexus processes running"
    else
        warn "⚠️  No Nexus processes found"
        log "Cluster may be stopped. Try: $SCRIPT_DIR/deploy.sh"
        return
    fi
    
    # Check ports
    local ports_open=0
    for port in 8080 8081 8082; do
        if ss -tln | grep -q ":$port "; then
            ports_open=$((ports_open + 1))
        fi
    done
    
    if [ "$ports_open" -gt 0 ]; then
        log "✅ $ports_open service ports are open"
    else
        warn "⚠️  No service ports detected"
    fi
    
    # Show endpoints
    echo ""
    log "Cluster Endpoints:"
    for i in 1 2 3; do
        local port=$((8079 + i))
        if ss -tln | grep -q ":$port "; then
            log "  Node $i: http://localhost:$port ✅"
        else
            warn "  Node $i: http://localhost:$port ❌"
        fi
    done
}

watch_status() {
    title "Real-Time Status Monitor"
    log "Watching cluster status... (Press Ctrl+C to stop)"
    echo ""
    
    while true; do
        clear
        echo -e "${BLUE}Hypermesh Nexus - Live Status Monitor${NC}"
        echo -e "${BLUE}$(date)${NC}"
        echo ""
        
        show_quick_status
        echo ""
        
        if [ "$SHOW_METRICS" = true ]; then
            show_performance_metrics
            echo ""
        fi
        
        log "Refreshing in 5 seconds..."
        sleep 5
    done
}

show_connection_info() {
    if [ "$DETAILED" = true ]; then
        title "Connection Information"
        
        log "How to connect to your cluster:"
        echo ""
        echo "# Using curl to check node health:"
        echo "curl -k https://localhost:8080/health"
        echo "curl -k https://localhost:8081/health"
        echo "curl -k https://localhost:8082/health"
        echo ""
        echo "# View logs (if using systemd deployment):"
        echo "journalctl -u nexus-node-1 -f"
        echo ""
        echo "# Connect with nexus CLI (when available):"
        echo "nexus cluster status"
        echo "nexus node list"
        echo ""
    fi
}

show_troubleshooting_tips() {
    if [ "$DETAILED" = true ]; then
        title "Troubleshooting Tips"
        
        echo ""
        log "If cluster seems unhealthy:"
        echo "  1. Check logs: $SCRIPT_DIR/demo.sh --logs"
        echo "  2. Restart cluster: $SCRIPT_DIR/cleanup.sh && $SCRIPT_DIR/deploy.sh"  
        echo "  3. Check system resources: free -h && df -h"
        echo "  4. Verify ports: netstat -tulpn | grep 808"
        echo ""
        log "For detailed diagnostics:"
        echo "  cd $NEXUS_ROOT/core/tests"
        echo "  cargo run --bin nexus-test -- all --detailed-report"
        echo ""
        log "Need help? Check the documentation:"
        echo "  $NEXUS_ROOT/docs/quick-start.md"
        echo "  $NEXUS_ROOT/examples/hello-world/README.md"
        echo ""
    fi
}

main() {
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --detailed)
                DETAILED=true
                shift
                ;;
            --watch)
                WATCH_MODE=true
                shift
                ;;
            --show-metrics)
                SHOW_METRICS=true
                shift
                ;;
            --help|-h)
                usage
                exit 0
                ;;
            *)
                error "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
    done
    
    if [ "$WATCH_MODE" = true ]; then
        watch_status
    else
        show_quick_status
        echo ""
        
        check_cluster_status
        echo ""
        
        run_health_checks
        echo ""
        
        show_performance_metrics
        echo ""
        
        show_connection_info
        show_troubleshooting_tips
        
        # Final status summary
        title "Status Summary"
        
        log "Cluster appears to be running normally"
        log "Next steps:"
        log "  • Run '$SCRIPT_DIR/demo.sh' for an interactive tour"
        log "  • Try '$SCRIPT_DIR/status.sh --watch' for live monitoring"
        log "  • Use '$SCRIPT_DIR/cleanup.sh' when done"
        echo ""
    fi
}

# Handle interruption in watch mode
trap 'echo ""; log "Status monitoring stopped"; exit 0' INT TERM

# Run main function
main "$@"