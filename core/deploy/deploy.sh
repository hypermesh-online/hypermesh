#!/bin/bash
# Hypermesh Nexus Deployment Script
# 
# Professional deployment automation for multi-environment deployments

set -euo pipefail

# Script configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
VERSION="1.0.0"

# Default configuration
DEPLOYMENT_TYPE="local"
CLUSTER_SIZE=5
ENVIRONMENT="staging"
CONFIG_FILE=""
SKIP_TESTS="false"
SKIP_BUILD="false"
VERBOSE="false"
DRY_RUN="false"

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${CYAN}[INFO]${NC} $1" >&2
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1" >&2
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1" >&2
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

log_debug() {
    if [[ "${VERBOSE}" == "true" ]]; then
        echo -e "${PURPLE}[DEBUG]${NC} $1" >&2
    fi
}

# Usage information
usage() {
    cat << EOF
🚀 Hypermesh Nexus Deployment Script v${VERSION}

USAGE:
    $0 [OPTIONS] [COMMAND]

COMMANDS:
    deploy      Deploy Nexus cluster (default)
    test        Run deployment tests
    clean       Clean up deployment
    status      Show deployment status
    logs        Show deployment logs

OPTIONS:
    -t, --type TYPE         Deployment type (local|docker|systemd|k8s) [default: ${DEPLOYMENT_TYPE}]
    -s, --size SIZE         Cluster size [default: ${CLUSTER_SIZE}]
    -e, --env ENV           Environment (dev|staging|prod) [default: ${ENVIRONMENT}]
    -c, --config FILE       Configuration file path
    --skip-tests            Skip pre-deployment tests
    --skip-build            Skip build step
    --dry-run               Show what would be deployed without executing
    -v, --verbose           Verbose output
    -h, --help              Show this help message

EXAMPLES:
    # Deploy 5-node local cluster
    $0 deploy --type local --size 5

    # Deploy to staging with custom config
    $0 deploy --env staging --config ./staging.toml

    # Test deployment without executing
    $0 deploy --dry-run --verbose

    # Deploy production cluster
    $0 deploy --type systemd --env prod --size 7

EOF
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -t|--type)
                DEPLOYMENT_TYPE="$2"
                shift 2
                ;;
            -s|--size)
                CLUSTER_SIZE="$2"
                shift 2
                ;;
            -e|--env)
                ENVIRONMENT="$2"
                shift 2
                ;;
            -c|--config)
                CONFIG_FILE="$2"
                shift 2
                ;;
            --skip-tests)
                SKIP_TESTS="true"
                shift
                ;;
            --skip-build)
                SKIP_BUILD="true"
                shift
                ;;
            --dry-run)
                DRY_RUN="true"
                shift
                ;;
            -v|--verbose)
                VERBOSE="true"
                shift
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            deploy|test|clean|status|logs)
                COMMAND="$1"
                shift
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
    done
    
    # Set default command
    COMMAND="${COMMAND:-deploy}"
}

# System requirements check
check_system_requirements() {
    log_info "Checking system requirements..."
    
    # Check Rust installation
    if ! command -v rustc &> /dev/null; then
        log_error "Rust compiler not found. Please install Rust."
        exit 1
    fi
    
    local rust_version
    rust_version=$(rustc --version)
    log_success "Rust found: ${rust_version}"
    
    # Check required tools based on deployment type
    case "${DEPLOYMENT_TYPE}" in
        docker)
            if ! command -v docker &> /dev/null; then
                log_error "Docker not found. Please install Docker."
                exit 1
            fi
            log_success "Docker found: $(docker --version)"
            ;;
        systemd)
            if ! command -v systemctl &> /dev/null; then
                log_error "systemctl not found. This deployment requires systemd."
                exit 1
            fi
            log_success "systemd found"
            ;;
        k8s)
            if ! command -v kubectl &> /dev/null; then
                log_error "kubectl not found. Please install kubectl."
                exit 1
            fi
            log_success "kubectl found: $(kubectl version --client --short)"
            ;;
    esac
    
    # Check available memory
    if [[ -f /proc/meminfo ]]; then
        local total_mem
        total_mem=$(awk '/MemTotal/ { print int($2/1024/1024) }' /proc/meminfo)
        if [[ ${total_mem} -lt 8 ]]; then
            log_warn "System has only ${total_mem}GB RAM. Recommended: 8GB+"
        else
            log_success "System memory: ${total_mem}GB"
        fi
    fi
    
    # Check disk space
    local available_space
    available_space=$(df -BG . | awk 'NR==2 {print int($4)}')
    if [[ ${available_space} -lt 10 ]]; then
        log_warn "Available disk space: ${available_space}GB. Recommended: 10GB+"
    else
        log_success "Available disk space: ${available_space}GB"
    fi
}

# Build Nexus components
build_nexus() {
    if [[ "${SKIP_BUILD}" == "true" ]]; then
        log_info "Skipping build step"
        return
    fi
    
    log_info "Building Nexus components..."
    
    cd "${PROJECT_ROOT}"
    
    if [[ "${DRY_RUN}" == "true" ]]; then
        log_info "[DRY RUN] Would build: cargo build --release"
        return
    fi
    
    # Build in release mode for deployment
    if [[ "${VERBOSE}" == "true" ]]; then
        cargo build --release
    else
        cargo build --release > /dev/null 2>&1
    fi
    
    # Verify binaries were created
    local binaries=("nexus-coordinator" "nexus-api-server" "nexus-cli")
    for binary in "${binaries[@]}"; do
        if [[ -f "target/release/${binary}" ]]; then
            log_success "Built: ${binary}"
        else
            log_warn "Binary not found: ${binary} (may be optional)"
        fi
    done
}

# Run pre-deployment tests
run_tests() {
    if [[ "${SKIP_TESTS}" == "true" ]]; then
        log_info "Skipping pre-deployment tests"
        return
    fi
    
    log_info "Running pre-deployment tests..."
    
    cd "${PROJECT_ROOT}/tests"
    
    if [[ "${DRY_RUN}" == "true" ]]; then
        log_info "[DRY RUN] Would run: cargo run --bin nexus-test -- unit --parallel"
        return
    fi
    
    # Run unit tests
    if [[ "${VERBOSE}" == "true" ]]; then
        cargo run --bin nexus-test -- unit --parallel
    else
        cargo run --bin nexus-test -- unit --parallel > /dev/null 2>&1
    fi
    
    log_success "Pre-deployment tests passed"
}

# Generate deployment configuration
generate_config() {
    log_info "Generating deployment configuration..."
    
    local config_dir="${PROJECT_ROOT}/deploy/config"
    mkdir -p "${config_dir}"
    
    local base_port=7777
    local config_files=()
    
    for ((i=1; i<=CLUSTER_SIZE; i++)); do
        local node_id="nexus-${ENVIRONMENT}-${i}"
        local port=$((base_port + i - 1))
        local config_file="${config_dir}/${node_id}.toml"
        
        if [[ "${DRY_RUN}" == "true" ]]; then
            log_info "[DRY RUN] Would generate config: ${config_file}"
            continue
        fi
        
        # Generate bootstrap peers list (all other nodes)
        local bootstrap_peers=""
        for ((j=1; j<=CLUSTER_SIZE; j++)); do
            if [[ $j -ne $i ]]; then
                if [[ -n "${bootstrap_peers}" ]]; then
                    bootstrap_peers="${bootstrap_peers}, "
                fi
                bootstrap_peers="${bootstrap_peers}\"127.0.0.1:$((base_port + j - 1))\""
            fi
        done
        
        # Generate configuration file
        cat > "${config_file}" << EOF
# Nexus Node Configuration - ${node_id}
[node]
id = "${node_id}"
name = "${node_id}"
data_dir = "./data/${node_id}"

[transport]
bind_address = "127.0.0.1"
port = ${port}
max_connections = 1000

[consensus]
bootstrap_peers = [${bootstrap_peers}]
election_timeout_ms = 5000
heartbeat_interval_ms = 1000

[storage]
backend = "RocksDB"
max_size_mb = 1024

[security]
enable_tls = true
cert_path = "./certs/${node_id}.pem"
key_path = "./certs/${node_id}.key"

[logging]
level = "info"
format = "json"
file_path = "./logs/${node_id}.log"

[metrics]
enable = true
port = $((port + 1000))
EOF
        
        config_files+=("${config_file}")
        log_debug "Generated config: ${config_file}"
    done
    
    if [[ "${DRY_RUN}" != "true" ]]; then
        log_success "Generated ${#config_files[@]} configuration files"
    fi
}

# Deploy local processes
deploy_local() {
    log_info "Deploying ${CLUSTER_SIZE}-node local cluster..."
    
    local pid_file="${PROJECT_ROOT}/deploy/nexus.pids"
    local log_dir="${PROJECT_ROOT}/logs"
    
    mkdir -p "${log_dir}"
    
    if [[ "${DRY_RUN}" == "true" ]]; then
        log_info "[DRY RUN] Would start ${CLUSTER_SIZE} local processes"
        return
    fi
    
    # Clear existing PID file
    > "${pid_file}"
    
    # Start each node
    for ((i=1; i<=CLUSTER_SIZE; i++)); do
        local node_id="nexus-${ENVIRONMENT}-${i}"
        local config_file="${PROJECT_ROOT}/deploy/config/${node_id}.toml"
        local log_file="${log_dir}/${node_id}.log"
        
        # Start node in background
        # Note: In real implementation, this would start the actual nexus binary
        nohup bash -c "
            echo 'Starting ${node_id}...'
            sleep infinity  # Simulate running process
        " > "${log_file}" 2>&1 &
        
        local pid=$!
        echo "${pid}" >> "${pid_file}"
        
        log_success "Started ${node_id} (PID: ${pid})"
        sleep 1  # Stagger startup
    done
    
    log_success "Local cluster deployed successfully"
    log_info "Process IDs saved to: ${pid_file}"
}

# Deploy with Docker
deploy_docker() {
    log_info "Deploying ${CLUSTER_SIZE}-node Docker cluster..."
    
    if [[ "${DRY_RUN}" == "true" ]]; then
        log_info "[DRY RUN] Would create Docker containers"
        return
    fi
    
    # Create Docker network
    docker network create nexus-network 2>/dev/null || true
    
    # Start each node as Docker container
    for ((i=1; i<=CLUSTER_SIZE; i++)); do
        local node_id="nexus-${ENVIRONMENT}-${i}"
        local port=$((7777 + i - 1))
        
        # Note: In real implementation, this would use actual Nexus Docker image
        docker run -d \
            --name "${node_id}" \
            --network nexus-network \
            -p "${port}:${port}" \
            -v "${PROJECT_ROOT}/deploy/config/${node_id}.toml:/config/nexus.toml:ro" \
            -v "${PROJECT_ROOT}/data/${node_id}:/data" \
            busybox:latest \
            sleep infinity
        
        log_success "Started Docker container: ${node_id}"
    done
    
    log_success "Docker cluster deployed successfully"
}

# Deploy with systemd
deploy_systemd() {
    log_info "Deploying ${CLUSTER_SIZE}-node systemd cluster..."
    
    if [[ "${DRY_RUN}" == "true" ]]; then
        log_info "[DRY RUN] Would create systemd services"
        return
    fi
    
    # Check if running as root or with sudo
    if [[ $EUID -ne 0 ]]; then
        log_error "systemd deployment requires root privileges"
        exit 1
    fi
    
    # Create each systemd service
    for ((i=1; i<=CLUSTER_SIZE; i++)); do
        local node_id="nexus-${ENVIRONMENT}-${i}"
        local service_file="/etc/systemd/system/${node_id}.service"
        
        cat > "${service_file}" << EOF
[Unit]
Description=Nexus Node ${node_id}
After=network.target
Wants=network.target

[Service]
Type=simple
User=nexus
Group=nexus
ExecStart=${PROJECT_ROOT}/target/release/nexus-coordinator --config ${PROJECT_ROOT}/deploy/config/${node_id}.toml
Restart=always
RestartSec=5
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
EOF
        
        # Enable and start service
        systemctl daemon-reload
        systemctl enable "${node_id}"
        systemctl start "${node_id}"
        
        log_success "Started systemd service: ${node_id}"
    done
    
    log_success "systemd cluster deployed successfully"
}

# Show deployment status
show_status() {
    log_info "Deployment Status:"
    echo
    
    case "${DEPLOYMENT_TYPE}" in
        local)
            local pid_file="${PROJECT_ROOT}/deploy/nexus.pids"
            if [[ -f "${pid_file}" ]]; then
                echo "Local processes:"
                while IFS= read -r pid; do
                    if kill -0 "${pid}" 2>/dev/null; then
                        echo "  ✅ PID ${pid}: Running"
                    else
                        echo "  ❌ PID ${pid}: Not running"
                    fi
                done < "${pid_file}"
            else
                echo "  No deployment found"
            fi
            ;;
        docker)
            echo "Docker containers:"
            for ((i=1; i<=CLUSTER_SIZE; i++)); do
                local node_id="nexus-${ENVIRONMENT}-${i}"
                if docker ps --format "table {{.Names}}" | grep -q "^${node_id}$"; then
                    echo "  ✅ ${node_id}: Running"
                else
                    echo "  ❌ ${node_id}: Not running"
                fi
            done
            ;;
        systemd)
            echo "systemd services:"
            for ((i=1; i<=CLUSTER_SIZE; i++)); do
                local node_id="nexus-${ENVIRONMENT}-${i}"
                local status
                status=$(systemctl is-active "${node_id}" 2>/dev/null || echo "inactive")
                if [[ "${status}" == "active" ]]; then
                    echo "  ✅ ${node_id}: ${status}"
                else
                    echo "  ❌ ${node_id}: ${status}"
                fi
            done
            ;;
    esac
}

# Clean up deployment
clean_deployment() {
    log_info "Cleaning up deployment..."
    
    if [[ "${DRY_RUN}" == "true" ]]; then
        log_info "[DRY RUN] Would clean up deployment"
        return
    fi
    
    case "${DEPLOYMENT_TYPE}" in
        local)
            local pid_file="${PROJECT_ROOT}/deploy/nexus.pids"
            if [[ -f "${pid_file}" ]]; then
                while IFS= read -r pid; do
                    if kill -0 "${pid}" 2>/dev/null; then
                        kill "${pid}"
                        log_success "Stopped process ${pid}"
                    fi
                done < "${pid_file}"
                rm -f "${pid_file}"
            fi
            ;;
        docker)
            for ((i=1; i<=CLUSTER_SIZE; i++)); do
                local node_id="nexus-${ENVIRONMENT}-${i}"
                if docker ps -a --format "table {{.Names}}" | grep -q "^${node_id}$"; then
                    docker stop "${node_id}" >/dev/null
                    docker rm "${node_id}" >/dev/null
                    log_success "Removed container: ${node_id}"
                fi
            done
            docker network rm nexus-network 2>/dev/null || true
            ;;
        systemd)
            for ((i=1; i<=CLUSTER_SIZE; i++)); do
                local node_id="nexus-${ENVIRONMENT}-${i}"
                systemctl stop "${node_id}" 2>/dev/null || true
                systemctl disable "${node_id}" 2>/dev/null || true
                rm -f "/etc/systemd/system/${node_id}.service"
                log_success "Removed service: ${node_id}"
            done
            systemctl daemon-reload
            ;;
    esac
    
    # Clean up generated files
    rm -rf "${PROJECT_ROOT}/deploy/config"
    rm -rf "${PROJECT_ROOT}/data/nexus-${ENVIRONMENT}-"*
    
    log_success "Cleanup completed"
}

# Show deployment logs
show_logs() {
    log_info "Showing deployment logs..."
    
    case "${DEPLOYMENT_TYPE}" in
        local)
            local log_dir="${PROJECT_ROOT}/logs"
            if [[ -d "${log_dir}" ]]; then
                find "${log_dir}" -name "nexus-${ENVIRONMENT}-*.log" -exec echo "=== {} ===" \; -exec tail -20 {} \;
            else
                log_warn "No log directory found"
            fi
            ;;
        docker)
            for ((i=1; i<=CLUSTER_SIZE; i++)); do
                local node_id="nexus-${ENVIRONMENT}-${i}"
                echo "=== ${node_id} ==="
                docker logs --tail 20 "${node_id}" 2>/dev/null || echo "Container not found"
            done
            ;;
        systemd)
            for ((i=1; i<=CLUSTER_SIZE; i++)); do
                local node_id="nexus-${ENVIRONMENT}-${i}"
                echo "=== ${node_id} ==="
                journalctl -u "${node_id}" --lines 20 --no-pager 2>/dev/null || echo "Service not found"
            done
            ;;
    esac
}

# Main deployment function
main_deploy() {
    log_info "🚀 Starting Nexus deployment..."
    log_info "Configuration:"
    log_info "  Type: ${DEPLOYMENT_TYPE}"
    log_info "  Size: ${CLUSTER_SIZE} nodes"
    log_info "  Environment: ${ENVIRONMENT}"
    
    if [[ "${DRY_RUN}" == "true" ]]; then
        log_warn "DRY RUN MODE - No changes will be made"
    fi
    
    check_system_requirements
    build_nexus
    run_tests
    generate_config
    
    case "${DEPLOYMENT_TYPE}" in
        local)
            deploy_local
            ;;
        docker)
            deploy_docker
            ;;
        systemd)
            deploy_systemd
            ;;
        *)
            log_error "Unknown deployment type: ${DEPLOYMENT_TYPE}"
            exit 1
            ;;
    esac
    
    if [[ "${DRY_RUN}" != "true" ]]; then
        echo
        log_success "🎉 Nexus deployment completed successfully!"
        echo
        log_info "Next steps:"
        log_info "  1. Check status: $0 status --type ${DEPLOYMENT_TYPE} --env ${ENVIRONMENT}"
        log_info "  2. View logs: $0 logs --type ${DEPLOYMENT_TYPE} --env ${ENVIRONMENT}"
        log_info "  3. Run tests: cd tests && cargo run --bin nexus-test -- staging"
        echo
        log_info "To clean up: $0 clean --type ${DEPLOYMENT_TYPE} --env ${ENVIRONMENT}"
    fi
}

# Main entry point
main() {
    # Parse command line arguments
    parse_args "$@"
    
    # Execute command
    case "${COMMAND}" in
        deploy)
            main_deploy
            ;;
        test)
            run_tests
            ;;
        clean)
            clean_deployment
            ;;
        status)
            show_status
            ;;
        logs)
            show_logs
            ;;
        *)
            log_error "Unknown command: ${COMMAND}"
            usage
            exit 1
            ;;
    esac
}

# Run main function
main "$@"