# Web3 Ecosystem Deployment Guide

## Prerequisites

### System Requirements
- **OS**: Linux (Ubuntu 22.04+ recommended)
- **CPU**: 8+ cores
- **RAM**: 32GB minimum
- **Storage**: 500GB SSD
- **Network**: IPv6 enabled, 10 Gbps connection

### Software Requirements
- Docker 24.0+
- Docker Compose 2.20+
- Rust 1.75+
- Node.js 20+
- Git 2.40+

## Quick Start

### 1. Clone Repository
```bash
git clone https://github.com/hypermesh-online/web3-ecosystem.git
cd web3-ecosystem
```

### 2. Environment Setup
```bash
# Copy environment template
cp .env.example .env

# Generate secrets
./scripts/generate-secrets.sh

# Configure IPv6 (if not enabled)
sudo sysctl -w net.ipv6.conf.all.disable_ipv6=0
```

### 3. Build Components
```bash
# Build all components
make build-all

# Or build individually
make build-trustchain
make build-stoq
make build-hypermesh
make build-caesar
```

### 4. Start Services
```bash
# Start entire stack
docker-compose up -d

# Verify services
docker-compose ps
./scripts/health-check.sh
```

## Component Deployment

### TrustChain CA
```bash
# Deploy TrustChain
cd trustchain
cargo build --release

# Initialize CA (first time only)
./target/release/trustchain init \
  --root-key-type rsa4096 \
  --validity-days 365 \
  --output-dir ./certs

# Start CA server
./target/release/trustchain serve \
  --bind "[::]:8443" \
  --cert ./certs/ca.crt \
  --key ./certs/ca.key
```

### STOQ Transport
```bash
# Deploy STOQ
cd stoq
cargo build --release

# Start STOQ server
./target/release/stoq-server \
  --bind "[::]:443" \
  --cert /path/to/cert.pem \
  --key /path/to/key.pem \
  --trustchain-url "https://trust.hypermesh.online"
```

### HyperMesh Nodes
```bash
# Deploy HyperMesh
cd hypermesh
cargo build --release

# Initialize node
./target/release/hypermesh init \
  --node-id $(uuidgen) \
  --data-dir ./data

# Start node
./target/release/hypermesh start \
  --bind "[::]:8545" \
  --trustchain "https://trust.hypermesh.online" \
  --bootstrap "stoq://seed.hypermesh.online"
```

### Caesar Economics
```bash
# Deploy smart contracts
cd caesar
npm install
npx hardhat compile

# Deploy to network
npx hardhat deploy \
  --network mainnet \
  --tags caesar-token,senate-dao,agora-dex

# Start API server
npm run start:api
```

## Production Configuration

### TLS Certificates

#### Development (Self-Signed)
```bash
# Generate self-signed certificate
openssl req -x509 -newkey rsa:4096 \
  -keyout key.pem -out cert.pem \
  -days 365 -nodes \
  -subj "/CN=*.hypermesh.online"
```

#### Staging (Let's Encrypt)
```bash
# Install certbot
sudo apt-get install certbot

# Obtain certificate
sudo certbot certonly \
  --standalone \
  -d trust.hypermesh.online \
  -d api.hypermesh.online
```

#### Production (HSM)
```bash
# Configure PKCS#11
export PKCS11_MODULE=/usr/lib/softhsm/libsofthsm2.so
export PKCS11_PIN=your-pin

# Initialize HSM
pkcs11-tool --module $PKCS11_MODULE \
  --init-token --label "HyperMesh" \
  --so-pin $PKCS11_PIN
```

### Database Setup

#### PostgreSQL
```bash
# Install PostgreSQL
sudo apt-get install postgresql-14

# Create database
sudo -u postgres createdb hypermesh
sudo -u postgres createuser hypermesh_user

# Run migrations
cd infrastructure/sql
psql -U hypermesh_user -d hypermesh -f schema.sql
```

#### Redis
```bash
# Install Redis
sudo apt-get install redis-server

# Configure for production
sudo vim /etc/redis/redis.conf
# Set: maxmemory 8gb
# Set: maxmemory-policy allkeys-lru

sudo systemctl restart redis
```

### Monitoring Stack

#### Prometheus
```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'hypermesh'
    static_configs:
      - targets:
        - 'trustchain:9090'
        - 'stoq:9091'
        - 'hypermesh:9092'
        - 'caesar:9093'
```

```bash
docker run -d \
  -p 3000:3000 \

# Import dashboards
curl -X POST http://admin:admin@localhost:3000/api/dashboards/db \
  -H "Content-Type: application/json" \
  -d @dashboards/hypermesh.json
```

## Kubernetes Deployment

### Helm Installation
```bash
# Add HyperMesh repository
helm repo add hypermesh https://charts.hypermesh.online
helm repo update

# Install with custom values
helm install hypermesh hypermesh/ecosystem \
  --namespace hypermesh \
  --create-namespace \
  -f values.yaml
```

### values.yaml Example
```yaml
global:
  imageTag: v1.0.0
  storageClass: fast-ssd

trustchain:
  replicas: 3
  resources:
    requests:
      cpu: 2
      memory: 4Gi

stoq:
  replicas: 5
  autoscaling:
    enabled: true
    minReplicas: 5
    maxReplicas: 50

hypermesh:
  nodes: 10
  storage:
    size: 100Gi

caesar:
  network: mainnet
  contracts:
    token: "0x..."
    dao: "0x..."
```

## Performance Tuning

### Kernel Parameters
```bash
# /etc/sysctl.conf
net.core.rmem_max = 134217728
net.core.wmem_max = 134217728
net.ipv6.conf.all.forwarding = 1
net.ipv4.tcp_congestion_control = bbr
fs.file-max = 2097152
```

### STOQ Optimization
```toml
# stoq.toml
[transport]
max_streams = 10000
stream_window = 16777216
connection_window = 67108864

[performance]
cpu_cores = 8
io_threads = 16
zero_copy = true
```

## Security Hardening

### Firewall Rules
```bash
# Allow only required ports
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow 443/tcp   # HTTPS/STOQ
sudo ufw allow 8443/tcp  # TrustChain
sudo ufw allow 8545/tcp  # HyperMesh
sudo ufw enable
```

### SELinux/AppArmor
```bash
# Enable SELinux
sudo setenforce 1

# Apply HyperMesh policy
sudo semodule -i policies/hypermesh.pp
```

## Backup & Recovery

### Automated Backups
```bash
# backup.sh
#!/bin/bash
BACKUP_DIR=/backups/$(date +%Y%m%d)
mkdir -p $BACKUP_DIR

# Backup TrustChain certificates
cp -r /var/lib/trustchain/certs $BACKUP_DIR/

# Backup HyperMesh state
docker exec hypermesh hypermesh backup \
  --output $BACKUP_DIR/hypermesh.tar.gz

# Backup database
pg_dump -U hypermesh_user hypermesh \
  > $BACKUP_DIR/database.sql

# Upload to S3
aws s3 sync $BACKUP_DIR s3://hypermesh-backups/
```

### Disaster Recovery
```bash
# Restore from backup
./scripts/restore.sh --backup-id 20240321 \
  --components trustchain,hypermesh,database
```

## Troubleshooting

### Common Issues

#### IPv6 Connectivity
```bash
# Test IPv6
ping6 ::1

# Enable if disabled
sudo sysctl -w net.ipv6.conf.all.disable_ipv6=0
```

#### Certificate Issues
```bash
# Verify certificate
openssl x509 -in cert.pem -text -noout

# Check expiry
openssl x509 -in cert.pem -enddate -noout
```

#### Performance Issues
```bash
# Check resource usage
docker stats

# Analyze bottlenecks
./scripts/performance-analysis.sh
```

## Support

- Documentation: https://docs.hypermesh.online
- GitHub Issues: https://github.com/hypermesh-online/ecosystem/issues
- Discord: https://discord.gg/hypermesh
- Email: support@hypermesh.online