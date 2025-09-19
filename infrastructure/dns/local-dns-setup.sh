#!/bin/bash
#
# Local DNS Setup for Web3 Ecosystem Testing
#
# This script establishes local DNS routing for hypermesh.online domains
# to enable full testing without requiring public domain deployment.
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
LOCAL_PORT=8443
DOMAINS=(
    "trust.hypermesh.online"
    "caesar.hypermesh.online"
    "catalog.hypermesh.online"
    "stoq.hypermesh.online"
    "ngauge.hypermesh.online"
    "hypermesh.online"
)

# Platform detection
PLATFORM=""
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    PLATFORM="linux"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    PLATFORM="macos"
elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" ]]; then
    PLATFORM="windows"
else
    PLATFORM="unknown"
fi

print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_header() {
    echo -e "${PURPLE}[DNS-SETUP]${NC} $1"
}

show_banner() {
    echo
    echo -e "${CYAN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║                  Local DNS Configuration                     ║${NC}"
    echo -e "${CYAN}║                Web3 Ecosystem Testing                       ║${NC}"
    echo -e "${CYAN}║                                                              ║${NC}"
    echo -e "${CYAN}║  🌐 Domain Mapping: *.hypermesh.online → localhost:8443    ║${NC}"
    echo -e "${CYAN}║  🔐 SSL Certificates: Self-signed for local testing        ║${NC}"
    echo -e "${CYAN}║  📡 DNS Resolution: /etc/hosts + IPv6 support              ║${NC}"
    echo -e "${CYAN}║  🚀 Cross-Platform: Linux, macOS, Windows                  ║${NC}"
    echo -e "${CYAN}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo
}

# Backup hosts file
backup_hosts() {
    print_header "Backing up hosts file"
    
    local hosts_file=""
    case $PLATFORM in
        "linux"|"macos")
            hosts_file="/etc/hosts"
            ;;
        "windows")
            hosts_file="C:\\Windows\\System32\\drivers\\etc\\hosts"
            ;;
        *)
            print_error "Unsupported platform: $OSTYPE"
            exit 1
            ;;
    esac
    
    local backup_file="${hosts_file}.hypermesh.backup.$(date +%Y%m%d_%H%M%S)"
    
    if [[ -f "$hosts_file" ]]; then
        if [[ "$PLATFORM" == "windows" ]]; then
            powershell -Command "Copy-Item '$hosts_file' '$backup_file'"
        else
            sudo cp "$hosts_file" "$backup_file" 2>/dev/null || {
                print_warning "Could not backup hosts file - may need sudo privileges"
                return 1
            }
        fi
        print_success "Hosts file backed up to: $backup_file"
    else
        print_warning "Hosts file not found: $hosts_file"
    fi
}

# Configure /etc/hosts entries
configure_hosts() {
    print_header "Configuring /etc/hosts entries"
    
    local hosts_file=""
    case $PLATFORM in
        "linux"|"macos")
            hosts_file="/etc/hosts"
            ;;
        "windows")
            hosts_file="C:\\Windows\\System32\\drivers\\etc\\hosts"
            ;;
    esac
    
    # Create temporary hosts entries
    local temp_entries=$(mktemp)
    cat > "$temp_entries" << EOF

# HyperMesh Web3 Ecosystem - Local Testing
# Added by local-dns-setup.sh on $(date)
# IPv4 entries
127.0.0.1 trust.hypermesh.online
127.0.0.1 caesar.hypermesh.online
127.0.0.1 catalog.hypermesh.online
127.0.0.1 stoq.hypermesh.online
127.0.0.1 ngauge.hypermesh.online
127.0.0.1 hypermesh.online

# IPv6 entries (preferred for Internet 2.0)
::1 trust.hypermesh.online
::1 caesar.hypermesh.online
::1 catalog.hypermesh.online
::1 stoq.hypermesh.online
::1 ngauge.hypermesh.online
::1 hypermesh.online
# End HyperMesh Web3 Ecosystem

EOF
    
    # Check if entries already exist
    if grep -q "trust.hypermesh.online" "$hosts_file" 2>/dev/null; then
        print_warning "HyperMesh entries already exist in hosts file"
        print_status "Use --force to overwrite existing entries"
        rm -f "$temp_entries"
        return 0
    fi
    
    # Add entries to hosts file
    print_status "Adding domain entries to hosts file..."
    
    if [[ "$PLATFORM" == "windows" ]]; then
        powershell -Command "Get-Content '$temp_entries' | Add-Content '$hosts_file'"
    else
        if ! sudo bash -c "cat '$temp_entries' >> '$hosts_file'" 2>/dev/null; then
            print_error "Failed to modify hosts file - need sudo privileges"
            print_status "Please run: sudo $0 $*"
            rm -f "$temp_entries"
            exit 1
        fi
    fi
    
    rm -f "$temp_entries"
    print_success "Domain entries added to hosts file"
}

# Generate SSL certificates
generate_certificates() {
    print_header "Generating SSL certificates"
    
    local cert_dir="certificates"
    mkdir -p "$cert_dir"
    
    # Check if openssl is available
    if ! command -v openssl &> /dev/null; then
        print_error "OpenSSL not found - required for certificate generation"
        print_status "Please install OpenSSL and retry"
        exit 1
    fi
    
    # Generate CA key and certificate
    local ca_key="$cert_dir/hypermesh-ca.key"
    local ca_cert="$cert_dir/hypermesh-ca.crt"
    
    if [[ ! -f "$ca_key" || ! -f "$ca_cert" ]]; then
        print_status "Generating Certificate Authority..."
        
        openssl genrsa -out "$ca_key" 4096
        openssl req -new -x509 -days 365 -key "$ca_key" -out "$ca_cert" -subj "/C=US/ST=CA/L=San Francisco/O=HyperMesh/OU=Local Testing/CN=HyperMesh Local CA"
        
        print_success "CA certificate generated: $ca_cert"
    else
        print_status "CA certificate already exists: $ca_cert"
    fi
    
    # Generate server certificate with SAN for all domains
    local server_key="$cert_dir/hypermesh-server.key"
    local server_csr="$cert_dir/hypermesh-server.csr"
    local server_cert="$cert_dir/hypermesh-server.crt"
    local server_config="$cert_dir/hypermesh-server.conf"
    
    if [[ ! -f "$server_cert" ]]; then
        print_status "Generating server certificate with Subject Alternative Names..."
        
        # Create OpenSSL config with SAN
        cat > "$server_config" << EOF
[req]
distinguished_name = req_distinguished_name
req_extensions = v3_req
prompt = no

[req_distinguished_name]
C = US
ST = CA
L = San Francisco
O = HyperMesh
OU = Local Testing
CN = hypermesh.online

[v3_req]
keyUsage = keyEncipherment, dataEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = hypermesh.online
DNS.2 = trust.hypermesh.online
DNS.3 = caesar.hypermesh.online
DNS.4 = catalog.hypermesh.online
DNS.5 = stoq.hypermesh.online
DNS.6 = ngauge.hypermesh.online
DNS.7 = localhost
IP.1 = 127.0.0.1
IP.2 = ::1
EOF
        
        # Generate private key
        openssl genrsa -out "$server_key" 4096
        
        # Generate certificate signing request
        openssl req -new -key "$server_key" -out "$server_csr" -config "$server_config"
        
        # Generate signed certificate
        openssl x509 -req -in "$server_csr" -CA "$ca_cert" -CAkey "$ca_key" -CAcreateserial -out "$server_cert" -days 365 -extensions v3_req -extfile "$server_config"
        
        # Clean up
        rm -f "$server_csr"
        
        print_success "Server certificate generated: $server_cert"
    else
        print_status "Server certificate already exists: $server_cert"
    fi
    
    # Set proper permissions
    chmod 600 "$cert_dir"/*.key 2>/dev/null || true
    chmod 644 "$cert_dir"/*.crt 2>/dev/null || true
    
    print_success "SSL certificates ready in: $cert_dir/"
}

# Test DNS resolution
test_dns_resolution() {
    print_header "Testing DNS resolution"
    
    local failed_domains=()
    
    for domain in "${DOMAINS[@]}"; do
        print_status "Testing resolution: $domain"
        
        # Test IPv4 resolution
        if ! ping -c 1 -W 2 "$domain" &>/dev/null; then
            if ! nslookup "$domain" &>/dev/null; then
                failed_domains+=("$domain")
                print_error "  Failed to resolve: $domain"
                continue
            fi
        fi
        
        # Test IPv6 resolution (if available)
        if command -v ping6 &>/dev/null; then
            if ping6 -c 1 -W 2 "$domain" &>/dev/null; then
                print_success "  ✓ IPv6 resolution: $domain"
            else
                print_warning "  IPv6 resolution failed: $domain"
            fi
        fi
        
        print_success "  ✓ Resolved: $domain"
    done
    
    if [[ ${#failed_domains[@]} -eq 0 ]]; then
        print_success "All domains resolved successfully"
    else
        print_error "Failed to resolve ${#failed_domains[@]} domains: ${failed_domains[*]}"
        return 1
    fi
}

# Test server connectivity
test_server_connectivity() {
    print_header "Testing server connectivity"
    
    print_status "Checking if server is running on port $LOCAL_PORT..."
    
    # Check if port is listening
    if command -v netstat &>/dev/null; then
        if netstat -tuln 2>/dev/null | grep -q ":$LOCAL_PORT "; then
            print_success "Server listening on port $LOCAL_PORT"
        else
            print_warning "No server detected on port $LOCAL_PORT"
            print_status "Start the server with: ./deploy-internet2.sh start"
            return 1
        fi
    elif command -v ss &>/dev/null; then
        if ss -tuln 2>/dev/null | grep -q ":$LOCAL_PORT "; then
            print_success "Server listening on port $LOCAL_PORT"
        else
            print_warning "No server detected on port $LOCAL_PORT"
            print_status "Start the server with: ./deploy-internet2.sh start"
            return 1
        fi
    else
        print_warning "Cannot check port status (netstat/ss not available)"
    fi
    
    # Test HTTPS connectivity to each domain
    local failed_connections=()
    
    for domain in "${DOMAINS[@]}"; do
        print_status "Testing HTTPS connection: https://$domain:$LOCAL_PORT"
        
        # Use curl with insecure flag for self-signed certificates
        if command -v curl &>/dev/null; then
            if curl -k -s --connect-timeout 5 "https://$domain:$LOCAL_PORT/health" 2>/dev/null | grep -q "healthy"; then
                print_success "  ✓ HTTPS connection: $domain"
            else
                failed_connections+=("$domain")
                print_warning "  HTTPS connection failed: $domain"
            fi
        else
            print_warning "  Cannot test HTTPS (curl not available)"
        fi
    done
    
    if [[ ${#failed_connections[@]} -eq 0 ]]; then
        print_success "All HTTPS connections successful"
    else
        print_warning "HTTPS connection issues with: ${failed_connections[*]}"
        print_status "This is normal if the server is not running"
    fi
}

# Remove DNS entries
remove_dns_entries() {
    print_header "Removing DNS entries"
    
    local hosts_file=""
    case $PLATFORM in
        "linux"|"macos")
            hosts_file="/etc/hosts"
            ;;
        "windows")
            hosts_file="C:\\Windows\\System32\\drivers\\etc\\hosts"
            ;;
    esac
    
    if [[ -f "$hosts_file" ]]; then
        print_status "Removing HyperMesh entries from hosts file..."
        
        if [[ "$PLATFORM" == "windows" ]]; then
            powershell -Command "(Get-Content '$hosts_file') | Where-Object { \$_ -notmatch 'hypermesh.online' -and \$_ -notmatch 'HyperMesh Web3 Ecosystem' } | Set-Content '$hosts_file'"
        else
            if ! sudo sed -i.bak '/# HyperMesh Web3 Ecosystem/,/# End HyperMesh Web3 Ecosystem/d' "$hosts_file" 2>/dev/null; then
                print_error "Failed to modify hosts file - need sudo privileges"
                exit 1
            fi
        fi
        
        print_success "DNS entries removed from hosts file"
    else
        print_warning "Hosts file not found: $hosts_file"
    fi
}

# Generate development configuration
generate_dev_config() {
    print_header "Generating development configuration"
    
    local config_file="config/development-local.toml"
    
    print_status "Creating local development configuration..."
    
    cat > "$config_file" << EOF
# Local Development Configuration for Web3 Ecosystem
# Generated by local-dns-setup.sh on $(date)

[global]
bind_address = "::"
port = $LOCAL_PORT
server_id = "internet2-local-dev"
max_connections = 1000
ipv6_only = false  # Allow IPv4 for local testing
log_level = "debug"
metrics_interval = { secs = 30, nanos = 0 }

[stoq.performance]
target_throughput_gbps = 1.0  # Reduced for local testing
enable_zero_copy = false
enable_hardware_acceleration = false
connection_pool_size = 100
memory_pool_size = 256
frame_batch_size = 8
enable_cpu_affinity = false
enable_large_send_offload = false

[stoq.quic]
max_concurrent_streams = 100
send_buffer_size = 1048576  # 1MB
receive_buffer_size = 1048576  # 1MB
connection_timeout = { secs = 10, nanos = 0 }
idle_timeout = { secs = 60, nanos = 0 }
enable_0rtt = true
enable_migration = false
congestion_control = "cubic"
max_datagram_size = 1500

[stoq.certificates]
validate_at_connection = true
validation_timeout = { secs = 30, nanos = 0 }
enable_caching = true
cache_size = 1000
cache_ttl = { secs = 3600, nanos = 0 }
# Local certificate paths
cert_file = "certificates/hypermesh-server.crt"
key_file = "certificates/hypermesh-server.key"
ca_file = "certificates/hypermesh-ca.crt"

[stoq.dns]
use_embedded_resolver = false  # Use system DNS for local testing
query_timeout = { secs = 10, nanos = 0 }
enable_caching = true
cache_size = 1000
cache_ttl = { secs = 300, nanos = 0 }

[hypermesh.consensus]
mandatory_four_proof = false  # Relaxed for local testing
validation_timeout = { secs = 1, nanos = 0 }
min_stake_requirement = 1
pow_difficulty = 1
enable_byzantine_detection = false
max_consensus_participants = 10

[hypermesh.assets]
max_assets_per_node = 1000
allocation_timeout = { secs = 30, nanos = 0 }
enable_pooling = true
pool_size = 100
cleanup_interval = { secs = 60, nanos = 0 }
default_resource_capacity = 100.0
require_consensus_for_allocation = false

[hypermesh.vm]
enable_vm_execution = true
max_vms_per_node = 10
execution_timeout = { secs = 300, nanos = 0 }
memory_limit_mb = 1024
cpu_limit_cores = 2
enable_snapshots = false

[hypermesh.proxy]
enable_nat_addressing = true
connection_timeout = { secs = 30, nanos = 0 }
max_proxy_connections = 1000
enable_trust_validation = false
enable_performance_monitoring = true

[trustchain.ca]
ca_mode = "local"  # Local mode for testing
certificate_validity_days = 30
rotation_interval = { secs = 86400, nanos = 0 }
enable_auto_rotation = false
max_chain_depth = 3

[trustchain.dns]
dns_mode = "hybrid"  # Use system DNS + local resolution
dns_port = 5353  # Non-privileged port
query_timeout = { secs = 10, nanos = 0 }
enable_caching = true
cache_ttl = { secs = 300, nanos = 0 }
max_cache_size = 1000

[trustchain.ct]
enable_ct_logging = false  # Disabled for local testing
submission_timeout = { secs = 30, nanos = 0 }
enable_verification = false
max_log_entries = 10000

[trustchain.pqc]
enable_pqc = false  # Simplified for local testing
enable_falcon = false
enable_kyber = false
enable_hybrid = false
security_level = 128

[integration]
enable_cross_layer_optimization = false
validation_timeout = { secs = 30, nanos = 0 }
coordination_interval = { secs = 60, nanos = 0 }
enable_layer_monitoring = true

[deployment]
mode = "development"
production_security = false
consensus_mandatory = false
legacy_compatibility = true
federated_bootstrap = false

# Local domain mappings
[domains]
trust = "trust.hypermesh.online:$LOCAL_PORT"
caesar = "caesar.hypermesh.online:$LOCAL_PORT"
catalog = "catalog.hypermesh.online:$LOCAL_PORT"
stoq = "stoq.hypermesh.online:$LOCAL_PORT"
ngauge = "ngauge.hypermesh.online:$LOCAL_PORT"
main = "hypermesh.online:$LOCAL_PORT"
EOF
    
    print_success "Development configuration created: $config_file"
}

# Show setup summary
show_summary() {
    print_header "Setup Summary"
    
    echo
    print_success "🌐 Local DNS routing configured for Web3 ecosystem"
    echo
    print_status "Domain mappings:"
    for domain in "${DOMAINS[@]}"; do
        print_status "  • https://$domain:$LOCAL_PORT → localhost:$LOCAL_PORT"
    done
    echo
    print_status "Certificates:"
    print_status "  • CA Certificate: certificates/hypermesh-ca.crt"
    print_status "  • Server Certificate: certificates/hypermesh-server.crt"
    print_status "  • Server Key: certificates/hypermesh-server.key"
    echo
    print_status "Configuration:"
    print_status "  • Development config: config/development-local.toml"
    print_status "  • Production config: config/production.toml"
    echo
    print_status "Next steps:"
    print_status "  1. Start the server: ./deploy-internet2.sh start"
    print_status "  2. Test connections: $0 test"
    print_status "  3. Open frontend: https://hypermesh.online:$LOCAL_PORT"
    echo
    print_status "Certificate trust (for browsers):"
    print_status "  • Import certificates/hypermesh-ca.crt as trusted CA"
    print_status "  • Or use --insecure/ignore certificate warnings"
    echo
}

# Main setup function
setup() {
    show_banner
    
    print_status "Platform detected: $PLATFORM"
    print_status "Configuring local DNS for Web3 ecosystem testing..."
    
    backup_hosts
    configure_hosts
    generate_certificates
    generate_dev_config
    test_dns_resolution
    
    show_summary
}

# Test existing setup
test_setup() {
    show_banner
    print_status "Testing existing DNS setup..."
    
    test_dns_resolution
    test_server_connectivity
    
    print_success "Testing complete"
}

# Remove setup
remove_setup() {
    print_header "Removing local DNS setup"
    
    remove_dns_entries
    
    print_status "Removing certificates..."
    rm -rf certificates/ 2>/dev/null || true
    
    print_status "Removing development config..."
    rm -f config/development-local.toml 2>/dev/null || true
    
    print_success "Local DNS setup removed"
}

# Command handling
case "${1:-setup}" in
    "setup")
        setup
        ;;
    "test")
        test_setup
        ;;
    "remove"|"cleanup")
        remove_setup
        ;;
    "cert"|"certificates")
        generate_certificates
        ;;
    "config")
        generate_dev_config
        ;;
    "hosts")
        backup_hosts
        configure_hosts
        ;;
    *)
        echo "Usage: $0 {setup|test|remove|cert|config|hosts}"
        echo
        echo "Commands:"
        echo "  setup      - Complete DNS setup (hosts, certificates, config)"
        echo "  test       - Test existing DNS resolution and connectivity"
        echo "  remove     - Remove DNS entries and certificates"
        echo "  cert       - Generate SSL certificates only"
        echo "  config     - Generate development configuration only"
        echo "  hosts      - Configure hosts file entries only"
        echo
        echo "Platform support: Linux, macOS, Windows"
        exit 1
        ;;
esac