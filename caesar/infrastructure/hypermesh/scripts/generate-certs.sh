#!/bin/bash

# Gateway Coin Certificate Generation Script
# Generates production-ready certificates for hypermesh infrastructure

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CERT_DIR="${SCRIPT_DIR}/../certs"
CONFIG_DIR="${SCRIPT_DIR}/../config/certs"

# Certificate configuration
CA_NAME="Gateway-Coin-CA"
ORG_NAME="Gateway Coin"
COUNTRY="US"
STATE="CA"
CITY="San Francisco"
EMAIL="admin@gateway-coin.com"

# Key configurations
RSA_KEY_SIZE=4096
ECDSA_CURVE="secp384r1"
CERT_VALIDITY_DAYS=365
CA_VALIDITY_DAYS=3650

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
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

# Create directories
create_directories() {
    log "Creating certificate directories..."
    mkdir -p "${CERT_DIR}"
    mkdir -p "${CONFIG_DIR}"
    mkdir -p "${CERT_DIR}/ca"
    mkdir -p "${CERT_DIR}/certs"
    mkdir -p "${CERT_DIR}/private"
    mkdir -p "${CERT_DIR}/csr"
    
    # Set secure permissions
    chmod 700 "${CERT_DIR}/private"
}

# Generate CA private key
generate_ca_key() {
    log "Generating CA private key..."
    if [[ ! -f "${CERT_DIR}/ca/ca-key.pem" ]]; then
        openssl genpkey -algorithm RSA \
            -pkeyopt rsa_keygen_bits:${RSA_KEY_SIZE} \
            -out "${CERT_DIR}/ca/ca-key.pem"
        chmod 600 "${CERT_DIR}/ca/ca-key.pem"
    else
        warn "CA key already exists, skipping generation"
    fi
}

# Generate CA certificate
generate_ca_cert() {
    log "Generating CA certificate..."
    if [[ ! -f "${CERT_DIR}/ca/ca.crt" ]]; then
        openssl req -new -x509 \
            -key "${CERT_DIR}/ca/ca-key.pem" \
            -out "${CERT_DIR}/ca/ca.crt" \
            -days ${CA_VALIDITY_DAYS} \
            -subj "/C=${COUNTRY}/ST=${STATE}/L=${CITY}/O=${ORG_NAME}/CN=${CA_NAME}" \
            -extensions ca_ext \
            -config <(cat << EOF
[req]
distinguished_name = req_distinguished_name
[ca_ext]
basicConstraints = critical,CA:TRUE
keyUsage = critical,keyCertSign,cRLSign
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid:always,issuer:always
EOF
        )
        chmod 644 "${CERT_DIR}/ca/ca.crt"
    else
        warn "CA certificate already exists, skipping generation"
    fi
}

# Generate service certificate
generate_service_cert() {
    local service_name="$1"
    local dns_names="$2"
    local ip_addresses="$3"
    
    log "Generating certificate for ${service_name}..."
    
    local key_file="${CERT_DIR}/private/${service_name}-key.pem"
    local csr_file="${CERT_DIR}/csr/${service_name}.csr"
    local cert_file="${CERT_DIR}/certs/${service_name}.crt"
    
    # Generate private key
    if [[ ! -f "${key_file}" ]]; then
        openssl genpkey -algorithm RSA \
            -pkeyopt rsa_keygen_bits:${RSA_KEY_SIZE} \
            -out "${key_file}"
        chmod 600 "${key_file}"
    fi
    
    # Create certificate signing request
    openssl req -new \
        -key "${key_file}" \
        -out "${csr_file}" \
        -subj "/C=${COUNTRY}/ST=${STATE}/L=${CITY}/O=${ORG_NAME}/CN=${service_name}" \
        -config <(cat << EOF
[req]
distinguished_name = req_distinguished_name
req_extensions = v3_req

[v3_req]
basicConstraints = CA:FALSE
keyUsage = nonRepudiation,digitalSignature,keyEncipherment
subjectAltName = @alt_names

[alt_names]
EOF
        echo "$dns_names" | tr ',' '\n' | awk '{print "DNS." NR " = " $1}'
        echo "$ip_addresses" | tr ',' '\n' | awk '{print "IP." NR " = " $1}'
    )
    
    # Sign certificate with CA
    openssl x509 -req \
        -in "${csr_file}" \
        -CA "${CERT_DIR}/ca/ca.crt" \
        -CAkey "${CERT_DIR}/ca/ca-key.pem" \
        -CAcreateserial \
        -out "${cert_file}" \
        -days ${CERT_VALIDITY_DAYS} \
        -extensions v3_req \
        -extfile <(cat << EOF
[v3_req]
basicConstraints = CA:FALSE
keyUsage = nonRepudiation,digitalSignature,keyEncipherment
extendedKeyUsage = serverAuth,clientAuth
subjectAltName = @alt_names

[alt_names]
EOF
        echo "$dns_names" | tr ',' '\n' | awk '{print "DNS." NR " = " $1}'
        echo "$ip_addresses" | tr ',' '\n' | awk '{print "IP." NR " = " $1}'
    )
    
    chmod 644 "${cert_file}"
    
    # Create combined cert file for nginx
    cat "${cert_file}" "${CERT_DIR}/ca/ca.crt" > "${CERT_DIR}/certs/${service_name}-bundle.crt"
}

# Generate client certificates for mutual TLS
generate_client_cert() {
    local client_name="$1"
    
    log "Generating client certificate for ${client_name}..."
    
    local key_file="${CERT_DIR}/private/${client_name}-client-key.pem"
    local csr_file="${CERT_DIR}/csr/${client_name}-client.csr"
    local cert_file="${CERT_DIR}/certs/${client_name}-client.crt"
    
    # Generate private key
    if [[ ! -f "${key_file}" ]]; then
        openssl genpkey -algorithm RSA \
            -pkeyopt rsa_keygen_bits:${RSA_KEY_SIZE} \
            -out "${key_file}"
        chmod 600 "${key_file}"
    fi
    
    # Create certificate signing request
    openssl req -new \
        -key "${key_file}" \
        -out "${csr_file}" \
        -subj "/C=${COUNTRY}/ST=${STATE}/L=${CITY}/O=${ORG_NAME}/CN=${client_name}-client"
    
    # Sign certificate with CA
    openssl x509 -req \
        -in "${csr_file}" \
        -CA "${CERT_DIR}/ca/ca.crt" \
        -CAkey "${CERT_DIR}/ca/ca-key.pem" \
        -CAcreateserial \
        -out "${cert_file}" \
        -days ${CERT_VALIDITY_DAYS} \
        -extensions client_ext \
        -extfile <(cat << EOF
[client_ext]
basicConstraints = CA:FALSE
keyUsage = nonRepudiation,digitalSignature,keyEncipherment
extendedKeyUsage = clientAuth
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid:always,issuer:always
EOF
    )
    
    chmod 644 "${cert_file}"
}

# Create certificate configuration files
create_cert_configs() {
    log "Creating certificate configuration files..."
    
    # CA config
    cat > "${CONFIG_DIR}/ca-config.json" << EOF
{
    "signing": {
        "default": {
            "expiry": "8760h",
            "usages": [
                "signing",
                "key encipherment",
                "server auth",
                "client auth"
            ]
        }
    }
}
EOF

    # Service configs
    for service in nexus stoq p2p ml-inference; do
        cat > "${CONFIG_DIR}/${service}-config.json" << EOF
{
    "CN": "${service}.gateway-coin.local",
    "hosts": [
        "${service}",
        "${service}.gateway-coin.local",
        "localhost",
        "127.0.0.1",
        "::1"
    ],
    "key": {
        "algo": "rsa",
        "size": ${RSA_KEY_SIZE}
    }
}
EOF
    done
}

# Verify certificates
verify_certificates() {
    log "Verifying certificates..."
    
    # Verify CA certificate
    if openssl x509 -in "${CERT_DIR}/ca/ca.crt" -text -noout > /dev/null; then
        log "CA certificate is valid"
    else
        error "CA certificate is invalid"
    fi
    
    # Verify service certificates
    for cert_file in "${CERT_DIR}/certs"/*.crt; do
        if [[ -f "$cert_file" && ! "$cert_file" =~ -bundle\.crt$ ]]; then
            local service_name=$(basename "$cert_file" .crt)
            if openssl verify -CAfile "${CERT_DIR}/ca/ca.crt" "$cert_file" > /dev/null; then
                log "Certificate for ${service_name} is valid"
            else
                error "Certificate for ${service_name} is invalid"
            fi
        fi
    done
}

# Create certificate renewal script
create_renewal_script() {
    log "Creating certificate renewal script..."
    
    cat > "${SCRIPT_DIR}/renew-certs.sh" << 'EOF'
#!/bin/bash

# Certificate renewal script for Gateway Coin infrastructure
# Run this script monthly to check and renew certificates

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CERT_DIR="${SCRIPT_DIR}/../certs"

# Check certificate expiration (30 days warning)
check_expiration() {
    local cert_file="$1"
    local service_name=$(basename "$cert_file" .crt)
    
    local expiry_date=$(openssl x509 -in "$cert_file" -noout -enddate | cut -d= -f2)
    local expiry_epoch=$(date -d "$expiry_date" +%s)
    local current_epoch=$(date +%s)
    local days_until_expiry=$(( (expiry_epoch - current_epoch) / 86400 ))
    
    if [[ $days_until_expiry -lt 30 ]]; then
        echo "WARNING: Certificate for ${service_name} expires in ${days_until_expiry} days"
        return 1
    else
        echo "Certificate for ${service_name} is valid for ${days_until_expiry} more days"
        return 0
    fi
}

# Check all certificates
renewal_needed=false
for cert_file in "${CERT_DIR}/certs"/*.crt; do
    if [[ -f "$cert_file" && ! "$cert_file" =~ -bundle\.crt$ ]]; then
        if ! check_expiration "$cert_file"; then
            renewal_needed=true
        fi
    fi
done

if [[ "$renewal_needed" == true ]]; then
    echo "Some certificates need renewal. Run generate-certs.sh to renew."
    exit 1
else
    echo "All certificates are valid."
fi
EOF

    chmod +x "${SCRIPT_DIR}/renew-certs.sh"
}

# Main execution
main() {
    log "Starting Gateway Coin certificate generation..."
    
    create_directories
    generate_ca_key
    generate_ca_cert
    create_cert_configs
    
    # Generate service certificates
    generate_service_cert "nexus" "nexus,hypermesh-nexus,nexus.gateway-coin.local,localhost" "127.0.0.1,::1,172.20.0.10"
    generate_service_cert "stoq" "stoq,stoq-engine,stoq.gateway-coin.local,localhost" "127.0.0.1,::1,172.20.0.11"
    generate_service_cert "p2p" "p2p,p2p-mesh-node,p2p.gateway-coin.local,localhost" "127.0.0.1,::1,172.20.0.12"
    generate_service_cert "ml-inference" "ml,ml-inference,ml.gateway-coin.local,localhost" "127.0.0.1,::1,172.20.0.13"
    generate_service_cert "gateway-coin" "gateway-coin.local,*.gateway-coin.local,localhost" "127.0.0.1,::1"
    
    # Generate client certificates
    generate_client_cert "gateway-coin-client"
    generate_client_cert "stoq-client" 
    generate_client_cert "ml-client"
    
    verify_certificates
    create_renewal_script
    
    log "Certificate generation completed successfully!"
    log "CA certificate: ${CERT_DIR}/ca/ca.crt"
    log "Service certificates: ${CERT_DIR}/certs/"
    log "Private keys: ${CERT_DIR}/private/"
    log ""
    log "To use these certificates:"
    log "1. Copy ${CERT_DIR} to your container volumes"
    log "2. Update docker-compose.yml volume mounts"
    log "3. Restart the services"
    log ""
    log "Certificate renewal script: ${SCRIPT_DIR}/renew-certs.sh"
}

# Check dependencies
if ! command -v openssl &> /dev/null; then
    error "OpenSSL is required but not installed"
fi

# Run main function
main "$@"