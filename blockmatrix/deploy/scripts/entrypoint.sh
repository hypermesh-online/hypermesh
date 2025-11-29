#!/bin/bash
set -euo pipefail

# Nexus container entrypoint script
# Handles initialization, configuration, and startup

NEXUS_HOME=${NEXUS_HOME:-/var/lib/nexus}
NEXUS_CONFIG_DIR=${NEXUS_CONFIG_DIR:-/etc/nexus}
NEXUS_LOG_DIR=${NEXUS_LOG_DIR:-/var/log/nexus}

# Default values
NEXUS_NODE_ID=${NEXUS_NODE_ID:-$(hostname)}
NEXUS_NODE_NAME=${NEXUS_NODE_NAME:-$(hostname)}
NEXUS_LOG_LEVEL=${NEXUS_LOG_LEVEL:-info}
NEXUS_CLUSTER_BOOTSTRAP=${NEXUS_CLUSTER_BOOTSTRAP:-}

# Logging function
log() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] $*" >&2
}

# Initialize Nexus directories
init_directories() {
    log "Initializing Nexus directories"
    
    mkdir -p "${NEXUS_HOME}"/{data,certs,plugins}
    mkdir -p "${NEXUS_LOG_DIR}"
    
    # Set proper permissions
    chmod 755 "${NEXUS_HOME}"
    chmod 700 "${NEXUS_HOME}/certs"
    chmod 755 "${NEXUS_LOG_DIR}"
}

# Generate configuration file
generate_config() {
    log "Generating Nexus configuration"
    
    local config_file="${NEXUS_CONFIG_DIR}/nexus.toml"
    
    if [[ -f "$config_file" ]]; then
        log "Using existing configuration: $config_file"
        return
    fi
    
    cat > "$config_file" << EOF
[node]
id = "${NEXUS_NODE_ID}"
name = "${NEXUS_NODE_NAME}"
data_dir = "${NEXUS_HOME}/data"

[transport]
bind_address = "::"
port = 7777
max_connections = 10000
connection_timeout_ms = 30000
keep_alive_ms = 5000

[security]
cert_path = "${NEXUS_HOME}/certs/server.pem"
key_path = "${NEXUS_HOME}/certs/server.key"
require_client_cert = true
encrypt_at_rest = true

[storage]
backend = "RocksDB"
max_size_mb = 10240
enable_wal = true

[logging]
level = "${NEXUS_LOG_LEVEL}"
format = "json"
file_enabled = true
file_path = "${NEXUS_LOG_DIR}/nexus.log"
structured = true
metrics_enabled = true
EOF
    
    log "Generated configuration: $config_file"
}

# Generate TLS certificates if needed
generate_certs() {
    local cert_dir="${NEXUS_HOME}/certs"
    local cert_file="${cert_dir}/server.pem"
    local key_file="${cert_dir}/server.key"
    
    if [[ -f "$cert_file" && -f "$key_file" ]]; then
        log "TLS certificates already exist"
        return
    fi
    
    log "Generating self-signed TLS certificates"
    
    openssl req -x509 -newkey rsa:4096 -keyout "$key_file" -out "$cert_file" \
        -days 365 -nodes \
        -subj "/C=US/ST=CA/L=San Francisco/O=Nexus/CN=${NEXUS_NODE_NAME}"
    
    chmod 600 "$key_file"
    chmod 644 "$cert_file"
    
    log "Generated TLS certificates: $cert_file, $key_file"
}

# Wait for cluster bootstrap nodes
wait_for_bootstrap() {
    if [[ -z "$NEXUS_CLUSTER_BOOTSTRAP" ]]; then
        log "No bootstrap nodes specified, starting as single node"
        return
    fi
    
    log "Waiting for bootstrap nodes: $NEXUS_CLUSTER_BOOTSTRAP"
    
    IFS=',' read -ra BOOTSTRAP_NODES <<< "$NEXUS_CLUSTER_BOOTSTRAP"
    
    local max_attempts=30
    local attempt=1
    
    while [[ $attempt -le $max_attempts ]]; do
        local all_ready=true
        
        for node in "${BOOTSTRAP_NODES[@]}"; do
            local host_port=(${node//:/ })
            local host=${host_port[0]}
            local port=${host_port[1]:-7777}
            
            if ! timeout 2 bash -c "echo >/dev/tcp/$host/$port" 2>/dev/null; then
                log "Bootstrap node not ready: $node (attempt $attempt/$max_attempts)"
                all_ready=false
                break
            fi
        done
        
        if [[ "$all_ready" == "true" ]]; then
            log "All bootstrap nodes are ready"
            return
        fi
        
        ((attempt++))
        sleep 2
    done
    
    log "Warning: Not all bootstrap nodes are ready, proceeding anyway"
}

# Health check function
health_check() {
    local max_attempts=30
    local attempt=1
    
    while [[ $attempt -le $max_attempts ]]; do
        if curl -sf http://localhost:8080/health >/dev/null 2>&1; then
            log "Nexus health check passed"
            return 0
        fi
        
        log "Health check failed (attempt $attempt/$max_attempts)"
        ((attempt++))
        sleep 2
    done
    
    log "Health check failed after $max_attempts attempts"
    return 1
}

# Signal handlers
cleanup() {
    log "Received termination signal, shutting down gracefully"
    
    if [[ -n "${NEXUS_PID:-}" ]]; then
        log "Stopping Nexus process (PID: $NEXUS_PID)"
        kill -TERM "$NEXUS_PID" || true
        
        # Wait for graceful shutdown
        local timeout=30
        while [[ $timeout -gt 0 ]] && kill -0 "$NEXUS_PID" 2>/dev/null; do
            sleep 1
            ((timeout--))
        done
        
        if kill -0 "$NEXUS_PID" 2>/dev/null; then
            log "Force killing Nexus process"
            kill -KILL "$NEXUS_PID" || true
        fi
    fi
    
    log "Shutdown complete"
    exit 0
}

# Main execution
main() {
    log "Starting Nexus container (Node: $NEXUS_NODE_NAME)"
    
    # Set up signal handlers
    trap cleanup SIGTERM SIGINT
    
    # Initialize environment
    init_directories
    generate_config
    generate_certs
    
    # Wait for cluster if needed
    wait_for_bootstrap
    
    # Determine which binary to run
    local binary="$1"
    shift || true
    
    case "$binary" in
        "nexus-coordinator"|"coordinator")
            binary="/usr/local/bin/nexus-coordinator"
            ;;
        "nexus-api-server"|"api-server")
            binary="/usr/local/bin/nexus-api-server"
            ;;
        "nexus-cli"|"cli")
            binary="/usr/local/bin/nexus-cli"
            ;;
        *)
            if [[ -x "/usr/local/bin/$binary" ]]; then
                binary="/usr/local/bin/$binary"
            else
                log "Unknown binary: $binary"
                exit 1
            fi
            ;;
    esac
    
    log "Starting Nexus: $binary $*"
    
    # Start the main process
    exec "$binary" \
        --config "${NEXUS_CONFIG_DIR}/nexus.toml" \
        --log-level "$NEXUS_LOG_LEVEL" \
        "$@" &
    
    NEXUS_PID=$!
    log "Nexus started with PID: $NEXUS_PID"
    
    # Wait for the process to be ready
    sleep 5
    
    # Run health check in background
    if command -v curl >/dev/null 2>&1; then
        health_check &
    fi
    
    # Wait for the main process
    wait "$NEXUS_PID"
    local exit_code=$?
    
    log "Nexus process exited with code: $exit_code"
    exit $exit_code
}

# Execute main function
main "$@"