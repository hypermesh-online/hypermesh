#!/bin/bash
# HyperMesh Cluster Management Script
# Provides operations for managing a running HyperMesh cluster

set -euo pipefail

readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly DEPLOYMENT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Color output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly NC='\033[0m'

# Logging functions
log_info() { echo -e "${BLUE}[INFO]${NC} $1" >&2; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1" >&2; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1" >&2; }
log_error() { echo -e "${RED}[ERROR]${NC} $1" >&2; }

# Usage information
usage() {
    cat << EOF
Usage: $0 COMMAND [OPTIONS]

Manage a running HyperMesh cluster.

COMMANDS:
    status          Show cluster status and health
    scale           Scale cluster up or down
    restart         Restart cluster nodes
    stop            Stop the cluster
    start           Start the cluster
    logs            Show cluster logs
    health          Perform cluster health check
    test            Run cluster functionality tests
    backup          Backup cluster data
    restore         Restore cluster from backup
    upgrade         Upgrade cluster components
    byzantine       Byzantine fault tolerance operations

OPTIONS:
    -v, --verbose   Enable verbose logging
    -h, --help      Show this help message

EXAMPLES:
    # Show cluster status
    $0 status

    # Scale cluster to 7 nodes
    $0 scale --size 7

    # Restart all nodes
    $0 restart

    # Show logs for specific node
    $0 logs --node hypermesh-node-0

    # Test Byzantine fault tolerance
    $0 byzantine --test-fault-tolerance

EOF
}

# Get cluster nodes from docker-compose
get_cluster_nodes() {
    cd "${DEPLOYMENT_DIR}"
    docker-compose ps --services | grep -E "^hypermesh-node-" | sort
}

# Get cluster size
get_cluster_size() {
    get_cluster_nodes | wc -l
}

# Show cluster status
show_status() {
    log_info "HyperMesh Cluster Status"
    echo "=========================="
    echo

    cd "${DEPLOYMENT_DIR}"
    
    # Docker compose status
    log_info "Container Status:"
    docker-compose ps
    echo

    # Node health checks
    log_info "Node Health:"
    local healthy_nodes=0
    local total_nodes=0
    
    while read -r node; do
        ((total_nodes++))
        local api_port
        api_port=$(docker-compose port "${node}" 8000 2>/dev/null | cut -d: -f2 || echo "")
        
        if [[ -n "${api_port}" ]] && curl -sf "http://localhost:${api_port}/health" &>/dev/null; then
            echo "  ✓ ${node}: Healthy"
            ((healthy_nodes++))
        else
            echo "  ✗ ${node}: Unhealthy"
        fi
    done < <(get_cluster_nodes)
    
    echo
    log_info "Cluster Health: ${healthy_nodes}/${total_nodes} nodes healthy"
    
    # Byzantine fault tolerance status
    local byzantine_threshold=$(( (total_nodes - 1) / 3 ))
    local unhealthy_nodes=$((total_nodes - healthy_nodes))
    
    if [[ ${unhealthy_nodes} -le ${byzantine_threshold} ]]; then
        log_success "Byzantine fault tolerance: OK (can tolerate ${byzantine_threshold} faults, ${unhealthy_nodes} currently failed)"
    else
        log_error "Byzantine fault tolerance: COMPROMISED (can tolerate ${byzantine_threshold} faults, ${unhealthy_nodes} currently failed)"
    fi
    echo

    # Resource usage
    log_info "Resource Usage:"
    docker stats --no-stream --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.NetIO}}" $(get_cluster_nodes | tr '\n' ' ')
}

# Scale cluster
scale_cluster() {
    local target_size="${1:-}"
    
    if [[ -z "${target_size}" ]]; then
        log_error "Target cluster size required"
        echo "Usage: $0 scale --size <number>"
        return 1
    fi
    
    local current_size
    current_size=$(get_cluster_size)
    
    log_info "Current cluster size: ${current_size}"
    log_info "Target cluster size: ${target_size}"
    
    if [[ "${target_size}" -lt 4 ]]; then
        log_error "Minimum cluster size is 4 for Byzantine fault tolerance"
        return 1
    fi
    
    if [[ "${target_size}" -eq "${current_size}" ]]; then
        log_info "Cluster is already at target size"
        return 0
    fi
    
    if [[ "${target_size}" -gt "${current_size}" ]]; then
        log_info "Scaling up cluster..."
        scale_up "${current_size}" "${target_size}"
    else
        log_info "Scaling down cluster..."
        scale_down "${current_size}" "${target_size}"
    fi
}

# Scale up cluster
scale_up() {
    local current_size="$1"
    local target_size="$2"
    
    log_warn "Scaling up requires regenerating cluster configuration"
    log_info "This will temporarily disrupt the cluster"
    
    read -p "Continue? (y/N): " -n 1 -r
    echo
    
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        log_info "Scale up cancelled"
        return 0
    fi
    
    # Regenerate configuration with new size
    "${SCRIPT_DIR}/deploy-cluster.sh" --cluster-size "${target_size}" --dry-run
    
    log_info "Please review the new configuration and redeploy manually"
}

# Scale down cluster
scale_down() {
    local current_size="$1"
    local target_size="$2"
    local nodes_to_remove=$((current_size - target_size))
    
    log_warn "Scaling down will remove ${nodes_to_remove} nodes"
    log_warn "This may affect data consistency and Byzantine fault tolerance"
    
    read -p "Continue? (y/N): " -n 1 -r
    echo
    
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        log_info "Scale down cancelled"
        return 0
    fi
    
    # Stop nodes to be removed (highest numbered nodes)
    cd "${DEPLOYMENT_DIR}"
    for ((i = target_size; i < current_size; i++)); do
        local node_name="hypermesh-node-${i}"
        log_info "Stopping ${node_name}..."
        docker-compose stop "${node_name}"
        docker-compose rm -f "${node_name}"
    done
    
    log_success "Scaled down to ${target_size} nodes"
    
    # Verify remaining nodes are healthy
    sleep 10
    show_status
}

# Restart cluster nodes
restart_cluster() {
    local node="${1:-}"
    
    cd "${DEPLOYMENT_DIR}"
    
    if [[ -n "${node}" ]]; then
        log_info "Restarting node: ${node}"
        docker-compose restart "${node}"
    else
        log_info "Restarting all cluster nodes..."
        
        # Rolling restart to maintain availability
        while read -r node_name; do
            log_info "Restarting ${node_name}..."
            docker-compose restart "${node_name}"
            
            # Wait for node to be healthy before continuing
            local api_port
            api_port=$(docker-compose port "${node_name}" 8000 2>/dev/null | cut -d: -f2 || echo "")
            
            if [[ -n "${api_port}" ]]; then
                local retry_count=0
                while [[ ${retry_count} -lt 30 ]]; do
                    if curl -sf "http://localhost:${api_port}/health" &>/dev/null; then
                        log_success "${node_name} is healthy"
                        break
                    fi
                    sleep 2
                    ((retry_count++))
                done
                
                if [[ ${retry_count} -eq 30 ]]; then
                    log_error "${node_name} failed to become healthy"
                fi
            fi
            
            sleep 5  # Brief pause between restarts
        done < <(get_cluster_nodes)
    fi
    
    log_success "Restart completed"
}

# Show cluster logs
show_logs() {
    local node="${1:-}"
    local follow="${2:-false}"
    
    cd "${DEPLOYMENT_DIR}"
    
    if [[ -n "${node}" ]]; then
        if [[ "${follow}" == "true" ]]; then
            docker-compose logs -f "${node}"
        else
            docker-compose logs "${node}"
        fi
    else
        if [[ "${follow}" == "true" ]]; then
            docker-compose logs -f
        else
            docker-compose logs
        fi
    fi
}

# Perform health check
health_check() {
    log_info "Performing comprehensive health check..."
    
    local health_results=()
    local overall_health="healthy"
    
    # Check container health
    log_info "Checking container health..."
    cd "${DEPLOYMENT_DIR}"
    
    while read -r node; do
        local health_status
        health_status=$(docker-compose exec -T "${node}" curl -s http://localhost:8000/health | jq -r '.status' 2>/dev/null || echo "unknown")
        
        if [[ "${health_status}" == "healthy" ]]; then
            health_results+=("✓ ${node}: ${health_status}")
        else
            health_results+=("✗ ${node}: ${health_status}")
            overall_health="unhealthy"
        fi
    done < <(get_cluster_nodes)
    
    # Check consensus health
    log_info "Checking consensus health..."
    local consensus_healthy=true
    
    while read -r node; do
        local consensus_status
        consensus_status=$(docker-compose exec -T "${node}" curl -s http://localhost:8000/consensus/status | jq -r '.state' 2>/dev/null || echo "unknown")
        
        if [[ "${consensus_status}" != "normal" ]]; then
            health_results+=("✗ ${node} consensus: ${consensus_status}")
            consensus_healthy=false
            overall_health="unhealthy"
        fi
    done < <(get_cluster_nodes)
    
    if [[ "${consensus_healthy}" == "true" ]]; then
        health_results+=("✓ Consensus: All nodes in normal state")
    fi
    
    # Check network connectivity
    log_info "Checking network connectivity..."
    local network_healthy=true
    
    while read -r node; do
        local peer_connections
        peer_connections=$(docker-compose exec -T "${node}" curl -s http://localhost:8000/network/peers | jq -r '.connected_peers' 2>/dev/null || echo "0")
        
        local expected_peers=$(($(get_cluster_size) - 1))
        
        if [[ "${peer_connections}" -lt "${expected_peers}" ]]; then
            health_results+=("✗ ${node} network: ${peer_connections}/${expected_peers} peers connected")
            network_healthy=false
            overall_health="degraded"
        fi
    done < <(get_cluster_nodes)
    
    if [[ "${network_healthy}" == "true" ]]; then
        health_results+=("✓ Network: All nodes fully connected")
    fi
    
    # Display results
    echo
    log_info "Health Check Results:"
    printf '%s\n' "${health_results[@]}"
    echo
    
    case "${overall_health}" in
        "healthy")
            log_success "Overall cluster health: HEALTHY"
            return 0
            ;;
        "degraded")
            log_warn "Overall cluster health: DEGRADED"
            return 1
            ;;
        "unhealthy")
            log_error "Overall cluster health: UNHEALTHY"
            return 2
            ;;
    esac
}

# Run functionality tests
run_tests() {
    log_info "Running cluster functionality tests..."
    
    # Test container operations
    log_info "Testing container operations..."
    test_container_operations
    
    # Test Byzantine fault tolerance
    log_info "Testing Byzantine fault tolerance..."
    test_byzantine_tolerance
    
    # Test network performance
    log_info "Testing network performance..."
    test_network_performance
    
    log_success "All tests completed"
}

# Test container operations
test_container_operations() {
    local node="hypermesh-node-0"
    local api_port
    api_port=$(cd "${DEPLOYMENT_DIR}" && docker-compose port "${node}" 8000 | cut -d: -f2)
    
    # Test container creation
    log_info "Testing container creation..."
    local container_id
    container_id=$(curl -s -X POST "http://localhost:${api_port}/containers" \
        -H "Content-Type: application/json" \
        -d '{"image": "alpine:latest", "command": ["echo", "test"]}' | jq -r '.id')
    
    if [[ -n "${container_id}" && "${container_id}" != "null" ]]; then
        log_success "Container creation: PASS (${container_id})"
        
        # Test container start
        if curl -s -X POST "http://localhost:${api_port}/containers/${container_id}/start" | grep -q "success"; then
            log_success "Container start: PASS"
        else
            log_error "Container start: FAIL"
        fi
        
        # Clean up
        curl -s -X DELETE "http://localhost:${api_port}/containers/${container_id}" >/dev/null
    else
        log_error "Container creation: FAIL"
    fi
}

# Test Byzantine fault tolerance
test_byzantine_tolerance() {
    cd "${DEPLOYMENT_DIR}"
    
    # Stop one node
    local test_node="hypermesh-node-0"
    log_info "Stopping ${test_node} to test fault tolerance..."
    docker-compose stop "${test_node}"
    
    # Wait for cluster to adjust
    sleep 10
    
    # Test operations on remaining nodes
    local remaining_node="hypermesh-node-1"
    local api_port
    api_port=$(docker-compose port "${remaining_node}" 8000 | cut -d: -f2)
    
    # Test that operations still work
    local test_result
    if test_result=$(curl -s -X POST "http://localhost:${api_port}/containers" \
        -H "Content-Type: application/json" \
        -d '{"image": "alpine:latest", "command": ["echo", "byzantine-test"]}'); then
        
        if echo "${test_result}" | jq -e '.id' >/dev/null 2>&1; then
            log_success "Byzantine fault tolerance: PASS (operations succeed with 1 node down)"
        else
            log_error "Byzantine fault tolerance: FAIL (${test_result})"
        fi
    else
        log_error "Byzantine fault tolerance: FAIL (no response)"
    fi
    
    # Restart the stopped node
    log_info "Restarting ${test_node}..."
    docker-compose start "${test_node}"
    
    # Wait for node to rejoin
    sleep 15
}

# Test network performance
test_network_performance() {
    log_info "Testing P2P network performance..."
    
    cd "${DEPLOYMENT_DIR}"
    local node="hypermesh-node-0"
    
    # Get network statistics
    local network_stats
    network_stats=$(docker-compose exec -T "${node}" curl -s http://localhost:8000/network/stats)
    
    if [[ -n "${network_stats}" ]]; then
        local avg_latency
        avg_latency=$(echo "${network_stats}" | jq -r '.avg_connection_time_ms // "unknown"')
        
        if [[ "${avg_latency}" != "unknown" ]] && (( $(echo "${avg_latency} <= 5.0" | bc -l) )); then
            log_success "P2P connection latency: PASS (${avg_latency}ms <= 5ms target)"
        else
            log_warn "P2P connection latency: DEGRADED (${avg_latency}ms > 5ms target)"
        fi
    else
        log_error "Network performance: FAIL (could not get stats)"
    fi
}

# Byzantine operations
byzantine_operations() {
    local operation="${1:-}"
    
    case "${operation}" in
        "test-fault-tolerance")
            test_byzantine_tolerance
            ;;
        "show-quarantine")
            show_quarantined_nodes
            ;;
        "release-quarantine")
            release_quarantined_nodes "${2:-}"
            ;;
        *)
            log_error "Unknown Byzantine operation: ${operation}"
            echo "Available operations: test-fault-tolerance, show-quarantine, release-quarantine"
            return 1
            ;;
    esac
}

# Show quarantined nodes
show_quarantined_nodes() {
    log_info "Checking for quarantined nodes..."
    
    cd "${DEPLOYMENT_DIR}"
    local found_quarantine=false
    
    while read -r node; do
        local quarantine_info
        quarantine_info=$(docker-compose exec -T "${node}" curl -s http://localhost:8000/byzantine/quarantine)
        
        local quarantined_count
        quarantined_count=$(echo "${quarantine_info}" | jq -r '.quarantined_nodes | length' 2>/dev/null || echo "0")
        
        if [[ "${quarantined_count}" -gt 0 ]]; then
            echo "Node ${node} has ${quarantined_count} quarantined peers:"
            echo "${quarantine_info}" | jq -r '.quarantined_nodes[]'
            found_quarantine=true
        fi
    done < <(get_cluster_nodes)
    
    if [[ "${found_quarantine}" == "false" ]]; then
        log_success "No quarantined nodes found"
    fi
}

# Backup cluster data
backup_cluster() {
    local backup_dir="${1:-./backups/$(date +%Y%m%d_%H%M%S)}"
    
    log_info "Backing up cluster data to ${backup_dir}..."
    
    mkdir -p "${backup_dir}"
    
    cd "${DEPLOYMENT_DIR}"
    
    # Backup configuration files
    cp -r configs/ "${backup_dir}/"
    cp -r certs/ "${backup_dir}/"
    cp docker-compose.yml "${backup_dir}/"
    
    # Backup node data
    mkdir -p "${backup_dir}/data"
    cp -r data/ "${backup_dir}/"
    
    # Create backup manifest
    cat > "${backup_dir}/manifest.json" << EOF
{
    "timestamp": "$(date -Iseconds)",
    "cluster_size": $(get_cluster_size),
    "backup_type": "full",
    "version": "1.0"
}
EOF
    
    log_success "Backup completed: ${backup_dir}"
}

# Main function
main() {
    local command="${1:-}"
    local verbose=false
    
    # Parse global options
    while [[ $# -gt 0 ]]; do
        case $1 in
            -v|--verbose)
                verbose=true
                shift
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            *)
                if [[ -z "${command}" ]]; then
                    command="$1"
                fi
                shift
                ;;
        esac
    done
    
    if [[ -z "${command}" ]]; then
        usage
        exit 1
    fi
    
    case "${command}" in
        status)
            show_status
            ;;
        scale)
            local size=""
            while [[ $# -gt 0 ]]; do
                case $1 in
                    --size)
                        size="$2"
                        shift 2
                        ;;
                    *)
                        shift
                        ;;
                esac
            done
            scale_cluster "${size}"
            ;;
        restart)
            local node=""
            while [[ $# -gt 0 ]]; do
                case $1 in
                    --node)
                        node="$2"
                        shift 2
                        ;;
                    *)
                        shift
                        ;;
                esac
            done
            restart_cluster "${node}"
            ;;
        stop)
            cd "${DEPLOYMENT_DIR}"
            docker-compose down
            ;;
        start)
            cd "${DEPLOYMENT_DIR}"
            docker-compose up -d
            ;;
        logs)
            local node=""
            local follow=false
            while [[ $# -gt 0 ]]; do
                case $1 in
                    --node)
                        node="$2"
                        shift 2
                        ;;
                    --follow|-f)
                        follow=true
                        shift
                        ;;
                    *)
                        shift
                        ;;
                esac
            done
            show_logs "${node}" "${follow}"
            ;;
        health)
            health_check
            ;;
        test)
            run_tests
            ;;
        backup)
            local backup_dir=""
            while [[ $# -gt 0 ]]; do
                case $1 in
                    --dir)
                        backup_dir="$2"
                        shift 2
                        ;;
                    *)
                        shift
                        ;;
                esac
            done
            backup_cluster "${backup_dir}"
            ;;
        byzantine)
            local operation=""
            local target=""
            while [[ $# -gt 0 ]]; do
                case $1 in
                    --test-fault-tolerance)
                        operation="test-fault-tolerance"
                        shift
                        ;;
                    --show-quarantine)
                        operation="show-quarantine"
                        shift
                        ;;
                    --release-quarantine)
                        operation="release-quarantine"
                        target="$2"
                        shift 2
                        ;;
                    *)
                        shift
                        ;;
                esac
            done
            byzantine_operations "${operation}" "${target}"
            ;;
        *)
            log_error "Unknown command: ${command}"
            usage
            exit 1
            ;;
    esac
}

# Execute main function
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi