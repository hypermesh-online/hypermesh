#!/bin/bash
# HyperMesh Cluster Deployment Script
# Deploys a Byzantine fault-tolerant container cluster with P2P networking

set -euo pipefail

# Script configuration
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
readonly DEPLOYMENT_DIR="${PROJECT_ROOT}/deployment"
readonly CONFIG_DIR="${DEPLOYMENT_DIR}/configs"
readonly TEMPLATE_DIR="${DEPLOYMENT_DIR}/templates"

# Default configuration
DEFAULT_CLUSTER_SIZE=4
DEFAULT_NODE_PREFIX="hypermesh-node"
DEFAULT_NETWORK_SUBNET="fd00::/64"
DEFAULT_QUIC_PORT_BASE=8080
DEFAULT_CONSENSUS_PORT_BASE=9090
DEFAULT_API_PORT_BASE=8000

# Color output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1" >&2
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

# Usage information
usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Deploy a Byzantine fault-tolerant HyperMesh container cluster.

OPTIONS:
    -s, --cluster-size NUM      Number of nodes in the cluster (default: ${DEFAULT_CLUSTER_SIZE})
    -p, --node-prefix PREFIX    Prefix for node names (default: ${DEFAULT_NODE_PREFIX})
    -n, --network SUBNET        IPv6 subnet for cluster networking (default: ${DEFAULT_NETWORK_SUBNET})
    -q, --quic-port PORT        Base port for QUIC transport (default: ${DEFAULT_QUIC_PORT_BASE})
    -c, --consensus-port PORT   Base port for consensus protocol (default: ${DEFAULT_CONSENSUS_PORT_BASE})
    -a, --api-port PORT         Base port for API endpoints (default: ${DEFAULT_API_PORT_BASE})
    -f, --config-file FILE      Custom cluster configuration file
    -d, --dry-run               Show what would be deployed without executing
    -v, --verbose               Enable verbose logging
    -h, --help                  Show this help message

EXAMPLES:
    # Deploy a 4-node cluster with default settings
    $0

    # Deploy a 7-node cluster for maximum Byzantine fault tolerance
    $0 --cluster-size 7

    # Deploy with custom configuration
    $0 --config-file custom-cluster.yaml

    # Dry run to see deployment plan
    $0 --dry-run --verbose

EOF
}

# Parse command line arguments
parse_args() {
    CLUSTER_SIZE="${DEFAULT_CLUSTER_SIZE}"
    NODE_PREFIX="${DEFAULT_NODE_PREFIX}"
    NETWORK_SUBNET="${DEFAULT_NETWORK_SUBNET}"
    QUIC_PORT_BASE="${DEFAULT_QUIC_PORT_BASE}"
    CONSENSUS_PORT_BASE="${DEFAULT_CONSENSUS_PORT_BASE}"
    API_PORT_BASE="${DEFAULT_API_PORT_BASE}"
    CONFIG_FILE=""
    DRY_RUN=false
    VERBOSE=false

    while [[ $# -gt 0 ]]; do
        case $1 in
            -s|--cluster-size)
                CLUSTER_SIZE="$2"
                shift 2
                ;;
            -p|--node-prefix)
                NODE_PREFIX="$2"
                shift 2
                ;;
            -n|--network)
                NETWORK_SUBNET="$2"
                shift 2
                ;;
            -q|--quic-port)
                QUIC_PORT_BASE="$2"
                shift 2
                ;;
            -c|--consensus-port)
                CONSENSUS_PORT_BASE="$2"
                shift 2
                ;;
            -a|--api-port)
                API_PORT_BASE="$2"
                shift 2
                ;;
            -f|--config-file)
                CONFIG_FILE="$2"
                shift 2
                ;;
            -d|--dry-run)
                DRY_RUN=true
                shift
                ;;
            -v|--verbose)
                VERBOSE=true
                shift
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
    done

    # Validate cluster size for Byzantine fault tolerance
    if [[ "${CLUSTER_SIZE}" -lt 4 ]]; then
        log_error "Cluster size must be at least 4 for Byzantine fault tolerance (3f+1 where f=1)"
        exit 1
    fi

    # Calculate Byzantine fault tolerance
    BYZANTINE_FAULTS=$(( (CLUSTER_SIZE - 1) / 3 ))
    
    log_info "Cluster configuration:"
    log_info "  Size: ${CLUSTER_SIZE} nodes"
    log_info "  Byzantine fault tolerance: ${BYZANTINE_FAULTS} faults"
    log_info "  Network: ${NETWORK_SUBNET}"
    log_info "  Node prefix: ${NODE_PREFIX}"
    
    if [[ "${VERBOSE}" == "true" ]]; then
        log_info "  QUIC port base: ${QUIC_PORT_BASE}"
        log_info "  Consensus port base: ${CONSENSUS_PORT_BASE}"
        log_info "  API port base: ${API_PORT_BASE}"
    fi
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking deployment prerequisites..."

    # Check for required tools
    local required_tools=("docker" "docker-compose" "openssl" "jq" "yq")
    local missing_tools=()

    for tool in "${required_tools[@]}"; do
        if ! command -v "${tool}" &> /dev/null; then
            missing_tools+=("${tool}")
        fi
    done

    if [[ ${#missing_tools[@]} -gt 0 ]]; then
        log_error "Missing required tools: ${missing_tools[*]}"
        log_error "Please install the missing tools and try again"
        exit 1
    fi

    # Check Docker daemon
    if ! docker info &> /dev/null; then
        log_error "Docker daemon is not running"
        exit 1
    fi

    # Check for sufficient resources
    local available_memory
    available_memory=$(free -m | awk 'NR==2{printf "%.0f", $7}')
    local required_memory=$((CLUSTER_SIZE * 512)) # 512MB per node minimum

    if [[ "${available_memory}" -lt "${required_memory}" ]]; then
        log_warn "Available memory (${available_memory}MB) may be insufficient for cluster (requires ~${required_memory}MB)"
    fi

    log_success "Prerequisites check passed"
}

# Generate node certificates
generate_certificates() {
    log_info "Generating node certificates for Byzantine fault tolerance..."

    local cert_dir="${DEPLOYMENT_DIR}/certs"
    mkdir -p "${cert_dir}"

    # Generate CA certificate if it doesn't exist
    if [[ ! -f "${cert_dir}/ca.pem" ]]; then
        log_info "Generating cluster CA certificate..."
        
        # Generate CA private key
        openssl genrsa -out "${cert_dir}/ca-key.pem" 4096
        
        # Generate CA certificate
        openssl req -new -x509 -days 365 -key "${cert_dir}/ca-key.pem" \
            -sha256 -out "${cert_dir}/ca.pem" -subj "/C=US/ST=State/L=City/O=HyperMesh/OU=Cluster/CN=HyperMesh-CA"
    fi

    # Generate node certificates
    for ((i = 0; i < CLUSTER_SIZE; i++)); do
        local node_name="${NODE_PREFIX}-${i}"
        local node_cert_dir="${cert_dir}/${node_name}"
        
        if [[ ! -f "${node_cert_dir}/cert.pem" ]]; then
            log_info "Generating certificate for ${node_name}..."
            
            mkdir -p "${node_cert_dir}"
            
            # Generate node private key
            openssl genrsa -out "${node_cert_dir}/key.pem" 2048
            
            # Generate certificate signing request
            openssl req -subj "/C=US/ST=State/L=City/O=HyperMesh/OU=Node/CN=${node_name}" \
                -sha256 -new -key "${node_cert_dir}/key.pem" -out "${node_cert_dir}/csr.pem"
            
            # Generate node certificate signed by CA
            openssl x509 -req -days 365 -sha256 -in "${node_cert_dir}/csr.pem" \
                -CA "${cert_dir}/ca.pem" -CAkey "${cert_dir}/ca-key.pem" -CAcreateserial \
                -out "${node_cert_dir}/cert.pem"
            
            # Clean up CSR
            rm "${node_cert_dir}/csr.pem"
            
            # Copy CA certificate for verification
            cp "${cert_dir}/ca.pem" "${node_cert_dir}/ca.pem"
        fi
    done

    log_success "Certificate generation completed"
}

# Generate cluster configuration
generate_cluster_config() {
    log_info "Generating cluster configuration..."

    local config_file="${CONFIG_DIR}/cluster-config.yaml"
    local node_configs=()

    # Generate node configurations
    for ((i = 0; i < CLUSTER_SIZE; i++)); do
        local node_name="${NODE_PREFIX}-${i}"
        local node_id="node-${i}"
        local quic_port=$((QUIC_PORT_BASE + i))
        local consensus_port=$((CONSENSUS_PORT_BASE + i))
        local api_port=$((API_PORT_BASE + i))
        
        # Calculate IPv6 address within subnet
        local ipv6_address
        ipv6_address=$(python3 -c "
import ipaddress
network = ipaddress.IPv6Network('${NETWORK_SUBNET}')
addresses = list(network.hosts())
if ${i} < len(addresses):
    print(str(addresses[${i}]))
else:
    print(str(network.network_address + ${i} + 1))
")

        node_configs+=("
  ${node_name}:
    node_id: \"${node_id}\"
    ipv6_address: \"${ipv6_address}\"
    ports:
      quic: ${quic_port}
      consensus: ${consensus_port}
      api: ${api_port}
    certificates:
      cert_file: \"/certs/${node_name}/cert.pem\"
      key_file: \"/certs/${node_name}/key.pem\"
      ca_file: \"/certs/${node_name}/ca.pem\"
")
    done

    # Create main cluster configuration
    cat > "${config_file}" << EOF
# HyperMesh Cluster Configuration
# Generated on $(date)

cluster:
  name: "hypermesh-cluster"
  size: ${CLUSTER_SIZE}
  byzantine_fault_tolerance: ${BYZANTINE_FAULTS}
  
network:
  subnet: "${NETWORK_SUBNET}"
  transport: "quic"
  
consensus:
  protocol: "pbft"
  timeout_ms: 5000
  view_change_timeout_ms: 10000
  
nodes:$(printf "%s" "${node_configs[@]}")

security:
  enable_tls: true
  certificate_rotation_hours: 24
  byzantine_detection: true
  
monitoring:
  enable_metrics: true
  prometheus_port: 9090
  health_check_interval_seconds: 30
  
runtime:
  container_startup_timeout_ms: 100
  network_setup_timeout_ms: 10
  p2p_connection_timeout_ms: 5
EOF

    log_success "Cluster configuration generated: ${config_file}"
}

# Generate Docker Compose configuration
generate_docker_compose() {
    log_info "Generating Docker Compose configuration..."

    local compose_file="${DEPLOYMENT_DIR}/docker-compose.yml"
    local services=""

    # Generate service configurations for each node
    for ((i = 0; i < CLUSTER_SIZE; i++)); do
        local node_name="${NODE_PREFIX}-${i}"
        local node_id="node-${i}"
        local quic_port=$((QUIC_PORT_BASE + i))
        local consensus_port=$((CONSENSUS_PORT_BASE + i))
        local api_port=$((API_PORT_BASE + i))
        local prometheus_port=$((9090 + i))

        services+="
  ${node_name}:
    image: hypermesh/nexus:latest
    container_name: ${node_name}
    hostname: ${node_name}
    networks:
      hypermesh-network:
        ipv6_address: fd00::$(printf "%x" $((i + 1)))
    ports:
      - \"${quic_port}:8080\"
      - \"${consensus_port}:9090\"
      - \"${api_port}:8000\"
      - \"${prometheus_port}:9091\"
    volumes:
      - \"./certs/${node_name}:/certs:ro\"
      - \"./configs/cluster-config.yaml:/config/cluster.yaml:ro\"
      - \"./data/${node_name}:/data\"
      - \"/var/run/docker.sock:/var/run/docker.sock\"
    environment:
      - NODE_ID=${node_id}
      - NODE_NAME=${node_name}
      - CLUSTER_CONFIG_FILE=/config/cluster.yaml
      - RUST_LOG=info
      - RUST_BACKTRACE=1
    command:
      - \"--config\"
      - \"/config/cluster.yaml\"
      - \"--node-id\"
      - \"${node_id}\"
      - \"--data-dir\"
      - \"/data\"
    healthcheck:
      test: [\"CMD\", \"curl\", \"-f\", \"http://localhost:8000/health\"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 60s
    restart: unless-stopped
    security_opt:
      - seccomp:unconfined
    cap_add:
      - NET_ADMIN
      - SYS_ADMIN
    depends_on:
      - hypermesh-setup"

    done

    # Create Docker Compose file
    cat > "${compose_file}" << EOF
# HyperMesh Cluster Docker Compose Configuration
# Generated on $(date)

version: '3.8'

services:
  hypermesh-setup:
    image: hypermesh/setup:latest
    container_name: hypermesh-setup
    volumes:
      - \"./data:/shared-data\"
      - \"./configs:/shared-config:ro\"
    command: [\"sh\", \"-c\", \"echo 'Setup completed' && sleep 5\"]
    networks:
      - hypermesh-network

${services}

  prometheus:
    image: prom/prometheus:latest
    container_name: hypermesh-prometheus
    ports:
      - \"9090:9090\"
    volumes:
      - \"./monitoring/prometheus.yml:/etc/prometheus/prometheus.yml:ro\"
      - \"prometheus_data:/prometheus\"
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--web.console.libraries=/etc/prometheus/console_libraries'
      - '--web.console.templates=/etc/prometheus/consoles'
      - '--storage.tsdb.retention.time=200h'
      - '--web.enable-lifecycle'
    networks:
      - hypermesh-network

  grafana:
    image: grafana/grafana:latest
    container_name: hypermesh-grafana
    ports:
      - \"3000:3000\"
    volumes:
      - \"grafana_data:/var/lib/grafana\"
      - \"./monitoring/grafana/dashboards:/etc/grafana/provisioning/dashboards:ro\"
      - \"./monitoring/grafana/datasources:/etc/grafana/provisioning/datasources:ro\"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    networks:
      - hypermesh-network

networks:
  hypermesh-network:
    enable_ipv6: true
    driver: bridge
    ipam:
      config:
        - subnet: ${NETWORK_SUBNET}

volumes:
  prometheus_data:
  grafana_data:
EOF

    # Create data directories
    for ((i = 0; i < CLUSTER_SIZE; i++)); do
        local node_name="${NODE_PREFIX}-${i}"
        mkdir -p "${DEPLOYMENT_DIR}/data/${node_name}"
    done

    log_success "Docker Compose configuration generated: ${compose_file}"
}

# Generate monitoring configuration
generate_monitoring_config() {
    log_info "Generating monitoring configuration..."

    local monitoring_dir="${DEPLOYMENT_DIR}/monitoring"
    mkdir -p "${monitoring_dir}/grafana/dashboards" "${monitoring_dir}/grafana/datasources"

    # Generate Prometheus configuration
    local prometheus_targets=""
    for ((i = 0; i < CLUSTER_SIZE; i++)); do
        local node_name="${NODE_PREFIX}-${i}"
        prometheus_targets+="      - '${node_name}:9091'\n"
    done

    cat > "${monitoring_dir}/prometheus.yml" << EOF
# Prometheus configuration for HyperMesh cluster monitoring

global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  # - "first_rules.yml"
  # - "second_rules.yml"

scrape_configs:
  - job_name: 'hypermesh-nodes'
    static_configs:
      - targets:
$(echo -e "${prometheus_targets}")

  - job_name: 'prometheus'
    static_configs:
      - targets: ['localhost:9090']
EOF

    # Generate Grafana datasource configuration
    cat > "${monitoring_dir}/grafana/datasources/prometheus.yml" << EOF
apiVersion: 1

datasources:
  - name: Prometheus
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    isDefault: true
EOF

    # Generate basic HyperMesh dashboard
    cat > "${monitoring_dir}/grafana/dashboards/hypermesh-overview.json" << 'EOF'
{
  "dashboard": {
    "id": null,
    "title": "HyperMesh Cluster Overview",
    "tags": ["hypermesh"],
    "timezone": "browser",
    "panels": [
      {
        "title": "Cluster Health",
        "type": "stat",
        "targets": [
          {
            "expr": "up{job=\"hypermesh-nodes\"}",
            "legendFormat": "{{instance}}"
          }
        ]
      },
      {
        "title": "Consensus Latency",
        "type": "graph",
        "targets": [
          {
            "expr": "hypermesh_consensus_latency_ms",
            "legendFormat": "{{instance}}"
          }
        ]
      },
      {
        "title": "Container Operations",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(hypermesh_container_operations_total[5m])",
            "legendFormat": "{{instance}}"
          }
        ]
      },
      {
        "title": "Network Throughput",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(hypermesh_network_bytes_total[5m])",
            "legendFormat": "{{instance}}"
          }
        ]
      }
    ],
    "time": {
      "from": "now-1h",
      "to": "now"
    },
    "refresh": "5s"
  }
}
EOF

    log_success "Monitoring configuration generated"
}

# Build container images
build_images() {
    log_info "Building HyperMesh container images..."

    # Build main HyperMesh image
    if [[ "${DRY_RUN}" == "false" ]]; then
        docker build -t hypermesh/nexus:latest "${PROJECT_ROOT}" \
            --file "${DEPLOYMENT_DIR}/docker/Dockerfile" \
            --build-arg BUILD_MODE=release
        
        # Build setup image
        docker build -t hypermesh/setup:latest "${DEPLOYMENT_DIR}/docker" \
            --file "${DEPLOYMENT_DIR}/docker/Dockerfile.setup"
    else
        log_info "[DRY RUN] Would build Docker images"
    fi

    log_success "Container images built"
}

# Deploy cluster
deploy_cluster() {
    log_info "Deploying HyperMesh cluster..."

    cd "${DEPLOYMENT_DIR}"

    if [[ "${DRY_RUN}" == "false" ]]; then
        # Start the cluster
        docker-compose up -d

        # Wait for cluster to be ready
        log_info "Waiting for cluster to be ready..."
        local retry_count=0
        local max_retries=30

        while [[ ${retry_count} -lt ${max_retries} ]]; do
            local healthy_nodes=0
            
            for ((i = 0; i < CLUSTER_SIZE; i++)); do
                local node_name="${NODE_PREFIX}-${i}"
                if docker-compose exec -T "${node_name}" curl -f http://localhost:8000/health &> /dev/null; then
                    ((healthy_nodes++))
                fi
            done

            if [[ ${healthy_nodes} -eq ${CLUSTER_SIZE} ]]; then
                log_success "All ${CLUSTER_SIZE} nodes are healthy"
                break
            fi

            log_info "Waiting for cluster readiness... (${healthy_nodes}/${CLUSTER_SIZE} nodes healthy)"
            sleep 10
            ((retry_count++))
        done

        if [[ ${retry_count} -eq ${max_retries} ]]; then
            log_error "Cluster failed to become ready within timeout"
            return 1
        fi

        # Verify Byzantine fault tolerance
        log_info "Verifying Byzantine fault tolerance..."
        verify_byzantine_tolerance

    else
        log_info "[DRY RUN] Would deploy cluster with Docker Compose"
    fi

    log_success "Cluster deployment completed"
}

# Verify Byzantine fault tolerance
verify_byzantine_tolerance() {
    log_info "Testing Byzantine fault tolerance..."

    # Test container operation with one node failure
    local test_node="${NODE_PREFIX}-0"
    
    log_info "Simulating node failure for ${test_node}..."
    docker-compose stop "${test_node}"

    # Wait a moment for cluster to adjust
    sleep 10

    # Test container creation through another node
    local test_api_port=$((API_PORT_BASE + 1))
    local test_result
    
    if test_result=$(curl -s -X POST "http://localhost:${test_api_port}/containers" \
        -H "Content-Type: application/json" \
        -d '{"image": "alpine:latest", "command": ["echo", "Byzantine test"]}'); then
        log_success "Container operation succeeded with one node down"
    else
        log_warn "Container operation failed with one node down: ${test_result}"
    fi

    # Restart the stopped node
    log_info "Restarting ${test_node}..."
    docker-compose start "${test_node}"

    # Wait for node to rejoin
    sleep 15

    log_success "Byzantine fault tolerance verification completed"
}

# Show cluster status
show_cluster_status() {
    log_info "Cluster Status:"
    echo
    
    cd "${DEPLOYMENT_DIR}"
    docker-compose ps

    echo
    log_info "Access Points:"
    log_info "  Grafana Dashboard: http://localhost:3000 (admin/admin)"
    log_info "  Prometheus: http://localhost:9090"
    
    for ((i = 0; i < CLUSTER_SIZE; i++)); do
        local node_name="${NODE_PREFIX}-${i}"
        local api_port=$((API_PORT_BASE + i))
        log_info "  ${node_name} API: http://localhost:${api_port}"
    done
}

# Cleanup function
cleanup() {
    if [[ "${DRY_RUN}" == "false" ]] && [[ -f "${DEPLOYMENT_DIR}/docker-compose.yml" ]]; then
        log_info "Cleaning up on exit..."
        cd "${DEPLOYMENT_DIR}"
        docker-compose down --remove-orphans || true
    fi
}

# Create Docker files
create_docker_files() {
    log_info "Creating Docker configuration files..."

    local docker_dir="${DEPLOYMENT_DIR}/docker"
    mkdir -p "${docker_dir}"

    # Main Dockerfile
    cat > "${docker_dir}/Dockerfile" << 'EOF'
# Multi-stage build for HyperMesh Nexus
FROM rust:1.70 as builder

WORKDIR /build

# Copy source code
COPY . .

# Build the application
ARG BUILD_MODE=release
RUN if [ "$BUILD_MODE" = "release" ]; then \
        cargo build --release && \
        cp target/release/nexus /nexus; \
    else \
        cargo build && \
        cp target/debug/nexus /nexus; \
    fi

# Runtime image
FROM ubuntu:22.04

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    libc6 \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /nexus /usr/local/bin/nexus

# Create non-root user
RUN useradd -r -s /bin/false nexus

# Create directories
RUN mkdir -p /data /config /certs && \
    chown -R nexus:nexus /data /config /certs

# Expose ports
EXPOSE 8080 9090 8000 9091

USER nexus

ENTRYPOINT ["/usr/local/bin/nexus"]
EOF

    # Setup Dockerfile
    cat > "${docker_dir}/Dockerfile.setup" << 'EOF'
FROM ubuntu:22.04

RUN apt-get update && apt-get install -y \
    curl \
    jq \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /setup

COPY setup-scripts/ .

RUN chmod +x *.sh

ENTRYPOINT ["/setup/init-cluster.sh"]
EOF

    log_success "Docker configuration files created"
}

# Main deployment function
main() {
    log_info "Starting HyperMesh cluster deployment"
    
    # Set up cleanup trap
    trap cleanup EXIT

    # Parse arguments
    parse_args "$@"

    # Check prerequisites
    check_prerequisites

    # Create necessary directories
    mkdir -p "${CONFIG_DIR}" "${TEMPLATE_DIR}"

    # Generate configurations
    generate_certificates
    generate_cluster_config
    generate_monitoring_config
    create_docker_files
    generate_docker_compose

    # Build and deploy
    build_images
    deploy_cluster

    # Show status
    show_cluster_status

    log_success "HyperMesh cluster deployment completed successfully!"
    echo
    log_info "Next steps:"
    log_info "1. Monitor cluster health at http://localhost:3000"
    log_info "2. Test container operations using the API endpoints"
    log_info "3. Review logs: docker-compose -f ${DEPLOYMENT_DIR}/docker-compose.yml logs"
    echo
    log_info "To stop the cluster: docker-compose -f ${DEPLOYMENT_DIR}/docker-compose.yml down"
}

# Execute main function if script is run directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi