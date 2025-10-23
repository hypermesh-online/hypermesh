#!/bin/bash
# Hello World Nexus Deployment Script
# Deploy a simple 3-node cluster locally

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NEXUS_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"
SIZE=3
VERBOSE=false
STEP_BY_STEP=false
BASE_PORT=8080

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'  
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

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
Hypermesh Nexus Hello World Deployment

USAGE:
    $0 [OPTIONS]

OPTIONS:
    --size SIZE         Cluster size (default: 3)
    --base-port PORT    Starting port number (default: 8080)
    --verbose          Enable verbose output
    --step-by-step     Interactive step-by-step deployment
    --help             Show this help message

EXAMPLES:
    $0                              # Deploy 3-node cluster  
    $0 --size 5 --verbose          # Deploy 5-node cluster with verbose output
    $0 --step-by-step               # Interactive deployment
    $0 --base-port 9000             # Use ports 9000, 9001, 9002

EOF
}

wait_for_input() {
    if [ "$STEP_BY_STEP" = true ]; then
        echo -e "${YELLOW}Press Enter to continue...${NC}"
        read -r
    fi
}

check_prerequisites() {
    title "Checking Prerequisites"
    
    # Check Rust
    if ! command -v cargo &> /dev/null; then
        error "Rust/Cargo not found. Please install Rust: https://rustup.rs/"
        exit 1
    fi
    
    local rust_version=$(rustc --version | grep -o '[0-9]\+\.[0-9]\+' | head -1)
    log "✅ Rust found: $(rustc --version)"
    
    # Check system resources
    local memory_gb=$(free -g | awk '/^Mem:/{print $2}')
    local available_space_gb=$(df . | awk 'NR==2{print int($4/1024/1024)}')
    
    if [ "$memory_gb" -lt 4 ]; then
        warn "⚠️  Low memory detected: ${memory_gb}GB (recommended: 4GB+)"
    else
        log "✅ Memory: ${memory_gb}GB available"
    fi
    
    if [ "$available_space_gb" -lt 10 ]; then
        warn "⚠️  Low disk space: ${available_space_gb}GB (recommended: 10GB+)"
    else
        log "✅ Disk space: ${available_space_gb}GB available"
    fi
    
    # Check port availability
    for ((i=0; i<SIZE; i++)); do
        local port=$((BASE_PORT + i))
        if ss -tln | grep -q ":$port "; then
            error "Port $port is already in use"
            log "Try: $0 --base-port $((BASE_PORT + 1000))"
            exit 1
        fi
    done
    log "✅ Ports $BASE_PORT-$((BASE_PORT + SIZE - 1)) available"
    
    wait_for_input
}

build_nexus() {
    title "Building Nexus Components"
    
    cd "$NEXUS_ROOT/core"
    
    if [ "$VERBOSE" = true ]; then
        log "Building with: cargo build --release"
        cargo build --release
    else
        log "Building Nexus components (this may take a few minutes)..."
        cargo build --release > build.log 2>&1
        
        if [ $? -ne 0 ]; then
            error "Build failed. Check build.log for details"
            tail -20 build.log
            exit 1
        fi
    fi
    
    log "✅ Build completed successfully"
    wait_for_input
}

deploy_cluster() {
    title "Deploying ${SIZE}-Node Local Cluster"
    
    cd "$NEXUS_ROOT/core/deploy"
    
    local deploy_cmd="./deploy.sh deploy --type local --size $SIZE --env dev"
    
    if [ "$VERBOSE" = true ]; then
        deploy_cmd="$deploy_cmd --verbose"
    fi
    
    log "Running: $deploy_cmd"
    
    if [ "$STEP_BY_STEP" = true ]; then
        $deploy_cmd
    else
        $deploy_cmd > deploy.log 2>&1
        
        if [ $? -ne 0 ]; then
            error "Deployment failed. Check deploy.log for details"
            tail -20 deploy.log
            exit 1
        fi
    fi
    
    log "✅ Cluster deployed successfully"
    wait_for_input
}

run_health_checks() {
    title "Running Health Checks"
    
    cd "$NEXUS_ROOT/core/tests"
    
    log "Running comprehensive test suite..."
    
    if [ "$VERBOSE" = true ]; then
        cargo run --bin nexus-test -- all --detailed-report
    else
        cargo run --bin nexus-test -- all --detailed-report > test.log 2>&1
        
        if [ $? -ne 0 ]; then
            error "Health checks failed. Check test.log for details"  
            tail -20 test.log
            exit 1
        fi
    fi
    
    log "✅ All health checks passed"
    
    if [ -f "nexus-test-report.md" ]; then
        log "📄 Detailed report available: nexus-test-report.md"
    fi
    
    wait_for_input
}

show_cluster_status() {
    title "Cluster Status"
    
    cd "$NEXUS_ROOT/core/deploy"
    
    log "Checking cluster status..."
    ./deploy.sh status
    
    echo ""
    log "🎉 Hello World cluster is ready!"
    log ""
    log "Cluster Information:"
    log "  Type: Local development cluster"  
    log "  Size: $SIZE nodes"
    log "  Environment: Development"
    log "  Base Port: $BASE_PORT"
    log ""
    log "Available Commands:"
    log "  $SCRIPT_DIR/status.sh          - Check cluster status"
    log "  $SCRIPT_DIR/demo.sh            - Run interactive demo"  
    log "  $SCRIPT_DIR/cleanup.sh         - Stop and cleanup cluster"
    log ""
    log "Node Endpoints:"
    for ((i=0; i<SIZE; i++)); do
        local port=$((BASE_PORT + i))
        log "  Node $((i+1)): http://localhost:$port"
    done
    
    wait_for_input
}

show_metrics_demo() {
    title "Real-Time Metrics Demo"
    
    cd "$NEXUS_ROOT/core/tests"
    
    log "Starting 30-second metrics collection..."
    log "Watch for real-time cluster performance data:"
    echo ""
    
    cargo run --bin nexus-test -- metrics --duration 30 --real-time
    
    log "✅ Metrics demo completed"
    wait_for_input
}

main() {
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --size)
                SIZE="$2"
                shift 2
                ;;
            --base-port)
                BASE_PORT="$2"
                shift 2
                ;;
            --verbose)
                VERBOSE=true
                shift
                ;;
            --step-by-step)
                STEP_BY_STEP=true
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
    
    # Validate size
    if ! [[ "$SIZE" =~ ^[0-9]+$ ]] || [ "$SIZE" -lt 3 ] || [ "$SIZE" -gt 10 ]; then
        error "Invalid cluster size: $SIZE (must be 3-10)"
        exit 1
    fi
    
    # Welcome message
    title "🚀 Hypermesh Nexus - Hello World Deployment"
    log "Deploying your first Nexus cluster..."
    log "Configuration: $SIZE nodes, starting at port $BASE_PORT"
    
    if [ "$STEP_BY_STEP" = true ]; then
        log "Running in step-by-step mode"
    fi
    
    echo ""
    wait_for_input
    
    # Deployment steps
    check_prerequisites
    build_nexus  
    deploy_cluster
    run_health_checks
    show_cluster_status
    show_metrics_demo
    
    # Success message
    title "✅ Hello World Deployment Complete!"
    
    log "🎉 Your Nexus cluster is ready to use!"
    log ""
    log "Next Steps:"
    log "  1. Run '$SCRIPT_DIR/demo.sh' for an interactive tour"
    log "  2. Try '$SCRIPT_DIR/status.sh --watch' for real-time monitoring"
    log "  3. Explore '../multi-node-cluster/' for a production example"
    log "  4. When done, run '$SCRIPT_DIR/cleanup.sh' to clean up"
    log ""
    log "Welcome to the future of cloud infrastructure! 🚀"
}

# Handle script interruption
trap 'error "Deployment interrupted"; exit 1' INT TERM

# Run main function
main "$@"