# Local DNS Setup Guide for Web3 Ecosystem Testing

## Overview

This guide establishes local DNS routing infrastructure to enable full testing of the Web3 ecosystem without requiring public domain deployment. The system provides local resolution for `*.hypermesh.online` domains pointing to the unified Internet 2.0 server.

## Quick Start

### Automated Setup (Recommended)

```bash
# Navigate to project directory
cd /home/persist/repos/projects/web3

# Run automated DNS setup
sudo ./infrastructure/dns/local-dns-setup.sh setup

# Start the Internet 2.0 server
./deploy-internet2.sh start

# Test the setup
./infrastructure/dns/local-dns-setup.sh test
```

### Manual Setup

If you prefer manual configuration or the automated setup fails:

#### 1. Configure /etc/hosts

Add the following entries to `/etc/hosts`:

```bash
# HyperMesh Web3 Ecosystem - Local Testing
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
```

#### 2. Generate SSL Certificates

```bash
# Create certificates directory
mkdir -p certificates

# Generate CA certificate
openssl genrsa -out certificates/hypermesh-ca.key 4096
openssl req -new -x509 -days 365 -key certificates/hypermesh-ca.key \
    -out certificates/hypermesh-ca.crt \
    -subj "/C=US/ST=CA/L=San Francisco/O=HyperMesh/OU=Local Testing/CN=HyperMesh Local CA"

# Generate server certificate with Subject Alternative Names
openssl genrsa -out certificates/hypermesh-server.key 4096

# Create OpenSSL config with SAN
cat > certificates/hypermesh-server.conf << EOF
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

# Generate certificate signing request and signed certificate
openssl req -new -key certificates/hypermesh-server.key \
    -out certificates/hypermesh-server.csr \
    -config certificates/hypermesh-server.conf

openssl x509 -req -in certificates/hypermesh-server.csr \
    -CA certificates/hypermesh-ca.crt \
    -CAkey certificates/hypermesh-ca.key \
    -CAcreateserial -out certificates/hypermesh-server.crt \
    -days 365 -extensions v3_req \
    -extfile certificates/hypermesh-server.conf

# Set proper permissions
chmod 600 certificates/*.key
chmod 644 certificates/*.crt
```

## Domain Mappings

The following domains are configured for local testing:

| Service | Domain | Description |
|---------|--------|-------------|
| **Main Dashboard** | `hypermesh.online:8443` | Primary HyperMesh interface |
| **TrustChain Authority** | `trust.hypermesh.online:8443` | Certificate authority and DNS |
| **Caesar Economics** | `caesar.hypermesh.online:8443` | Economic incentive system |
| **Catalog VM System** | `catalog.hypermesh.online:8443` | Virtual machine execution |
| **STOQ Transport** | `stoq.hypermesh.online:8443` | QUIC transport protocol |
| **NGauge Platform** | `ngauge.hypermesh.online:8443` | User engagement platform |

## Configuration Files

### Development Configuration

The setup creates `config/development-local.toml` with local-friendly settings:

- Reduced performance targets suitable for local testing
- Relaxed consensus requirements
- Local certificate paths
- Debug logging enabled
- IPv4 compatibility for development

### Production Configuration

Use `config/production.toml` for production-like testing:

- Full performance targets (40 Gbps STOQ)
- Mandatory four-proof consensus
- IPv6-only networking
- Embedded CA and DNS
- Production security settings

## Platform Support

### Linux

```bash
# Ubuntu/Debian
sudo apt-get install openssl curl nettools-ping

# CentOS/RHEL/Fedora
sudo yum install openssl curl iputils

# Run setup
sudo ./infrastructure/dns/local-dns-setup.sh setup
```

### macOS

```bash
# Install prerequisites (if needed)
brew install openssl curl

# Run setup
sudo ./infrastructure/dns/local-dns-setup.sh setup
```

### Windows

```powershell
# Run in Administrator PowerShell
./infrastructure/dns/local-dns-setup.sh setup

# Or use Windows Subsystem for Linux (WSL)
wsl sudo ./infrastructure/dns/local-dns-setup.sh setup
```

## Alternative Setup Methods

### Docker-based DNS (No hosts file modification)

```bash
# Start DNS container with Dnsmasq
cd infrastructure/dns
docker-compose -f docker-dns-setup.yml up -d

# Configure your system to use 127.0.0.1 as DNS server
# This varies by operating system
```

### Browser Configuration

For browsers that don't respect system DNS:

1. **Import CA Certificate**:
   - Open browser settings
   - Go to Certificate Management
   - Import `certificates/hypermesh-ca.crt` as trusted CA

2. **Disable Certificate Validation** (Development only):
   - Chrome: `--ignore-certificate-errors --disable-web-security`
   - Firefox: Set `security.mixed_content.block_active_content` to false

## Testing and Validation

### Test DNS Resolution

```bash
# Test all domains
./infrastructure/dns/local-dns-setup.sh test

# Test individual domain
ping trust.hypermesh.online
nslookup caesar.hypermesh.online
```

### Test HTTPS Connections

```bash
# Test with curl (ignore self-signed certificates)
curl -k https://hypermesh.online:8443
curl -k https://trust.hypermesh.online:8443
curl -k https://caesar.hypermesh.online:8443

# Test with browser (import CA cert first)
# Open: https://hypermesh.online:8443
```

### Server Status

```bash
# Check if server is running
./deploy-internet2.sh status

# View server logs
tail -f logs/server.log

# Check listening ports
netstat -tuln | grep 8443
```

## Troubleshooting

### Common Issues

1. **Permission denied modifying /etc/hosts**
   ```bash
   # Run with sudo
   sudo ./infrastructure/dns/local-dns-setup.sh setup
   ```

2. **DNS not resolving**
   ```bash
   # Clear DNS cache
   sudo systemctl flush-dns      # Linux
   sudo dscacheutil -flushcache  # macOS
   ipconfig /flushdns             # Windows
   ```

3. **Certificate errors in browser**
   ```bash
   # Import CA certificate or use --insecure flag
   curl -k https://hypermesh.online:8443
   ```

4. **Server not responding**
   ```bash
   # Check if server is running
   ./deploy-internet2.sh status
   
   # Start server if needed
   ./deploy-internet2.sh start
   
   # Check logs for errors
   tail -f logs/server.log
   ```

5. **Port 8443 already in use**
   ```bash
   # Find what's using the port
   sudo lsof -i :8443
   
   # Kill conflicting process or change port in config
   ```

### IPv6 Issues

If IPv6 is not working:

```bash
# Test IPv6 connectivity
ping6 ::1

# Enable IPv6 if disabled
sudo sysctl -w net.ipv6.conf.all.disable_ipv6=0

# Check IPv6 addresses
ip -6 addr show
```

### Firewall Issues

If connections are blocked:

```bash
# Allow port 8443 through firewall
sudo ufw allow 8443                    # Ubuntu
sudo firewall-cmd --add-port=8443/tcp  # CentOS/RHEL
```

## Cleanup

### Remove DNS Setup

```bash
# Remove all DNS entries and certificates
./infrastructure/dns/local-dns-setup.sh remove

# Or manually remove from /etc/hosts
sudo sed -i '/hypermesh.online/d' /etc/hosts
```

### Remove Docker Setup

```bash
cd infrastructure/dns
docker-compose -f docker-dns-setup.yml down -v
```

## Development Workflow

### Typical Development Session

```bash
# 1. Set up local DNS (one-time)
sudo ./infrastructure/dns/local-dns-setup.sh setup

# 2. Start the Internet 2.0 server
./deploy-internet2.sh start

# 3. Test all domains
./infrastructure/dns/local-dns-setup.sh test

# 4. Open frontend in browser
open https://hypermesh.online:8443

# 5. Develop and test...

# 6. Restart server after changes
./deploy-internet2.sh restart

# 7. View logs
tail -f logs/server.log
```

### Team Setup

For team members to set up the same environment:

```bash
# Clone repository
git clone <repository-url>
cd web3

# Run one-command setup
make local-setup

# Or manual setup
sudo ./infrastructure/dns/local-dns-setup.sh setup
./deploy-internet2.sh start
```

## Integration with Frontend

### API Client Configuration

Update the frontend API client to use domain names:

```typescript
// Before (localhost)
const API_BASE = 'https://localhost:8443';

// After (domain-based)
const API_ENDPOINTS = {
  trustchain: 'https://trust.hypermesh.online:8443',
  caesar: 'https://caesar.hypermesh.online:8443',
  catalog: 'https://catalog.hypermesh.online:8443',
  stoq: 'https://stoq.hypermesh.online:8443',
  ngauge: 'https://ngauge.hypermesh.online:8443',
  main: 'https://hypermesh.online:8443'
};
```

### STOQ Protocol Configuration

Configure STOQ connections to use proper domain names:

```typescript
const stoqConfig = {
  endpoint: 'stoq.hypermesh.online:8443',
  protocol: 'quic',
  certificates: {
    ca: '/certificates/hypermesh-ca.crt',
    verify: false // For local development
  }
};
```

## Security Considerations

### Development Security

- **Self-signed certificates**: Only for local development
- **Relaxed validation**: Consensus and validation reduced for testing
- **Debug logging**: May expose sensitive information
- **Insecure connections**: Browser security reduced for testing

### Production Preparation

Before production deployment:

1. **Real certificates**: Use Let's Encrypt or commercial CA
2. **Full consensus**: Enable mandatory four-proof validation
3. **IPv6-only**: Remove IPv4 compatibility
4. **Security headers**: Enable all security headers
5. **Logging**: Reduce log level to info/warn only

## Performance Monitoring

### Local Development Metrics

The local setup provides basic performance monitoring:

- **Connection counts**: Active HTTPS connections
- **Certificate validation**: Time to validate certificates
- **DNS resolution**: Local DNS query response times
- **STOQ protocol**: Transport layer performance

### Monitoring URLs

- Server status: `https://hypermesh.online:8443/health`
- Metrics endpoint: `https://hypermesh.online:8443/metrics`
- DNS web UI: `http://localhost:5380` (Docker setup only)

---

This DNS setup provides a robust local testing environment that mirrors the production architecture while being suitable for development and testing workflows.