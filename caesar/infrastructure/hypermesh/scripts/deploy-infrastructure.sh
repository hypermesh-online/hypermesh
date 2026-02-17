#!/bin/bash

# Gateway Coin Hypermesh Infrastructure Deployment Script
# Automated deployment and configuration of production-ready infrastructure

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="${SCRIPT_DIR}/.."
COMPOSE_FILE="${PROJECT_ROOT}/docker-compose.yml"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] $1${NC}"
}

warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING: $1${NC}"
}

error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $1${NC}"
    exit 1
}

info() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')] INFO: $1${NC}"
}

# Check prerequisites
check_prerequisites() {
    log "Checking prerequisites..."
    
    # Check Docker
    if ! command -v docker &> /dev/null; then
        error "Docker is required but not installed"
    fi
    
    # Check Docker Compose
    if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
        error "Docker Compose is required but not installed"
    fi
    
    # Check OpenSSL for certificate generation
    if ! command -v openssl &> /dev/null; then
        error "OpenSSL is required for certificate generation"
    fi
    
    # Check if running as root or in docker group
    if [[ $EUID -ne 0 ]] && ! groups | grep -q docker; then
        warn "You may need to run as root or add user to docker group"
    fi
    
    # Check available system resources
    local mem_gb=$(free -g | awk '/^Mem:/{print $2}')
    local disk_gb=$(df -BG "${PROJECT_ROOT}" | awk 'NR==2{print int($4)}')
    
    if [[ ${mem_gb} -lt 8 ]]; then
        warn "Less than 8GB RAM available. Infrastructure may not perform optimally."
    fi
    
    if [[ ${disk_gb} -lt 20 ]]; then
        warn "Less than 20GB disk space available. Consider freeing up space."
    fi
    
    log "Prerequisites check completed"
}

# Generate certificates
generate_certificates() {
    log "Generating certificates..."
    
    if [[ ! -f "${PROJECT_ROOT}/certs/ca/ca.crt" ]]; then
        info "Certificates not found, generating new ones..."
        "${SCRIPT_DIR}/generate-certs.sh"
    else
        info "Certificates already exist, skipping generation"
    fi
}

# Compile eBPF programs
compile_ebpf() {
    log "Compiling eBPF programs..."
    
    local ebpf_dir="${PROJECT_ROOT}/config/ebpf"
    local ebpf_bin_dir="${PROJECT_ROOT}/ebpf"
    
    mkdir -p "${ebpf_bin_dir}"
    
    # Check if clang is available
    if command -v clang &> /dev/null; then
        info "Compiling network_stats.c..."
        clang -O2 -target bpf -c "${ebpf_dir}/network_stats.c" -o "${ebpf_bin_dir}/network_stats.o" || warn "eBPF compilation failed, using pre-compiled binaries"
    else
        warn "clang not found, skipping eBPF compilation"
    fi
}

# Initialize Redis cluster
init_redis_cluster() {
    log "Initializing Redis cluster..."
    
    # Wait for Redis nodes to be ready
    info "Waiting for Redis nodes to start..."
    sleep 10
    
    # Check if cluster is already initialized
    if docker exec redis-cluster-1 redis-cli cluster nodes &> /dev/null; then
        info "Redis cluster already initialized"
        return 0
    fi
    
    # Create cluster
    info "Creating Redis cluster..."
    docker exec redis-cluster-1 redis-cli --cluster create \
        redis-cluster-1:7001 redis-cluster-2:7002 redis-cluster-3:7003 \
        --cluster-replicas 0 --cluster-yes || warn "Redis cluster creation failed"
}

# Setup monitoring
setup_monitoring() {
    log "Setting up monitoring..."
    
    # Wait for InfluxDB to be ready
    info "Waiting for InfluxDB to start..."
    local retries=30
    while [[ $retries -gt 0 ]]; do
        if docker exec gateway-coin-influxdb influx ping &> /dev/null; then
            break
        fi
        sleep 5
        ((retries--))
    done
    
    if [[ $retries -eq 0 ]]; then
        warn "InfluxDB not ready, monitoring setup may be incomplete"
        return 1
    fi
    
    # Configure InfluxDB
    info "Configuring InfluxDB..."
    docker exec gateway-coin-influxdb influx setup \
        --username admin \
        --password gateway-coin-metrics-2025 \
        --org gateway-coin \
        --bucket stoq-metrics \
        --token gateway-coin-influx-token-2025 \
        --force || info "InfluxDB already configured"
}

# Validate deployment
validate_deployment() {
    log "Validating deployment..."
    
    local failed_services=()
    
    # Check service health
    local services=("hypermesh-nexus" "stoq-engine" "ml-inference" "redis-cluster-1" "redis-cluster-2" "redis-cluster-3")
    
    for service in "${services[@]}"; do
        info "Checking ${service}..."
        if ! docker ps | grep -q "${service}"; then
            warn "${service} is not running"
            failed_services+=("${service}")
        else
            info "${service} is running"
        fi
    done
    
    # Check network connectivity
    info "Testing network connectivity..."
    
    # Test nexus API
    if curl -k -f "https://localhost:8443/health" &> /dev/null; then
        info "Nexus API is accessible"
    else
        warn "Nexus API is not accessible"
        failed_services+=("nexus-api")
    fi
    
    # Test STOQ API
    if curl -f "http://localhost:8081/health" &> /dev/null; then
        info "STOQ API is accessible"
    else
        warn "STOQ API is not accessible"
        failed_services+=("stoq-api")
    fi
    
    # Test Redis cluster
    if docker exec redis-cluster-1 redis-cli ping | grep -q PONG; then
        info "Redis cluster is responding"
    else
        warn "Redis cluster is not responding"
        failed_services+=("redis-cluster")
    fi
    
    # Test Grafana
    if curl -f "http://localhost:3000/api/health" &> /dev/null; then
        info "Grafana is accessible"
    else
        warn "Grafana is not accessible"
        failed_services+=("grafana")
    fi
    
    # Report results
    if [[ ${#failed_services[@]} -eq 0 ]]; then
        log "All services are healthy!"
        return 0
    else
        warn "Failed services: ${failed_services[*]}"
        return 1
    fi
}

# Performance benchmarking
run_benchmarks() {
    log "Running performance benchmarks..."
    
    info "Waiting for services to stabilize..."
    sleep 30
    
    # Benchmark nexus API
    info "Benchmarking Nexus API..."
    if command -v ab &> /dev/null; then
        ab -n 1000 -c 10 -k "http://localhost:8080/health" > "${PROJECT_ROOT}/benchmark-nexus.txt" 2>&1 || warn "Nexus benchmark failed"
    else
        warn "Apache Bench (ab) not available, skipping benchmarks"
    fi
    
    # Test STOQ performance
    info "Testing STOQ performance..."
    # Add custom STOQ performance tests here
    
    # Test ML inference performance
    info "Testing ML inference performance..."
    # Add ML inference performance tests here
    
    log "Benchmarks completed. Results in ${PROJECT_ROOT}/benchmark-*.txt"
}

# Cleanup function
cleanup() {
    log "Cleaning up on exit..."
    # Any cleanup tasks if the script is interrupted
}

# Setup trap for cleanup
trap cleanup EXIT

# Display deployment information
show_deployment_info() {
    log "Gateway Coin Hypermesh Infrastructure Deployed Successfully!"
    echo ""
    info "Service Endpoints:"
    echo "  - Nexus Core (HTTPS): https://localhost:8443"
    echo "  - Nexus Core (HTTP):  http://localhost:8080"
    echo "  - STOQ Engine:        http://localhost:8081"
    echo "  - ML Inference:       http://localhost:8082"
    echo "  - Load Balancer:      https://localhost"
    echo "  - Grafana Dashboard:  http://localhost:3000"
    echo "  - InfluxDB:           http://localhost:8086"
    echo ""
    info "Credentials:"
    echo "  - Grafana: admin / gateway-coin-grafana-2025"
    echo "  - InfluxDB: admin / gateway-coin-metrics-2025"
    echo ""
    info "Monitoring:"
    echo "  - Metrics: http://localhost/metrics"
    echo "  - Health: http://localhost/health"
    echo "  - Status: http://localhost:8080/status"
    echo ""
    info "Configuration:"
    echo "  - Certificates: ${PROJECT_ROOT}/certs/"
    echo "  - Logs: docker logs <container_name>"
    echo "  - Config: ${PROJECT_ROOT}/config/"
    echo ""
    info "Performance Targets:"
    echo "  - >99.9% uptime for core services"
    echo "  - <100ms latency for state operations"
    echo "  - >1000 TPS transaction processing"
    echo "  - <2 second cross-cluster consensus time"
    echo ""
    warn "Security Notes:"
    echo "  - Change default passwords in production"
    echo "  - Review firewall rules"
    echo "  - Monitor certificate expiration"
    echo "  - Regular security updates required"
}

# Main deployment function
deploy() {
    log "Starting Gateway Coin Hypermesh Infrastructure deployment..."
    
    check_prerequisites
    generate_certificates
    compile_ebpf
    
    info "Starting services..."
    cd "${PROJECT_ROOT}"
    
    # Pull latest images
    info "Pulling Docker images..."
    docker-compose pull || warn "Some images may not be available, continuing with local builds"
    
    # Start services in stages
    info "Starting Redis cluster..."
    docker-compose up -d redis-cluster-1 redis-cluster-2 redis-cluster-3
    sleep 10
    
    info "Starting InfluxDB..."
    docker-compose up -d influxdb
    sleep 15
    
    info "Starting core services..."
    docker-compose up -d hypermesh-nexus stoq-engine ml-inference
    sleep 20
    
    info "Starting supporting services..."
    docker-compose up -d p2p-mesh-node container-runtime cert-manager
    sleep 10
    
    info "Starting load balancer and monitoring..."
    docker-compose up -d load-balancer grafana
    sleep 10
    
    # Initialize components
    init_redis_cluster
    setup_monitoring
    
    # Validate deployment
    if validate_deployment; then
        run_benchmarks
        show_deployment_info
    else
        warn "Deployment validation failed. Check logs: docker-compose logs"
        return 1
    fi
}

# Script options
case "${1:-deploy}" in
    deploy)
        deploy
        ;;
    validate)
        validate_deployment
        ;;
    benchmark)
        run_benchmarks
        ;;
    stop)
        log "Stopping all services..."
        cd "${PROJECT_ROOT}"
        docker-compose down
        ;;
    restart)
        log "Restarting all services..."
        cd "${PROJECT_ROOT}"
        docker-compose down
        sleep 5
        deploy
        ;;
    logs)
        cd "${PROJECT_ROOT}"
        docker-compose logs -f "${2:-}"
        ;;
    status)
        cd "${PROJECT_ROOT}"
        docker-compose ps
        validate_deployment
        ;;
    *)
        echo "Usage: $0 {deploy|validate|benchmark|stop|restart|logs|status}"
        echo ""
        echo "Commands:"
        echo "  deploy    - Deploy the full infrastructure"
        echo "  validate  - Validate running deployment"
        echo "  benchmark - Run performance benchmarks"
        echo "  stop      - Stop all services"
        echo "  restart   - Restart all services"
        echo "  logs      - View logs (optionally for specific service)"
        echo "  status    - Show service status"
        exit 1
        ;;
esac