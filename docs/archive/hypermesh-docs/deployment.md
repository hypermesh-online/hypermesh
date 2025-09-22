# Production Deployment Guide

Complete guide for deploying Hypermesh Nexus in production environments.

## 🏗️ Deployment Types

### Local Development
**Single machine testing and development**

```bash
cd core/deploy
./deploy.sh deploy --type local --size 3 --env dev --verbose
```

**Characteristics:**
- Multiple processes on single machine
- Shared filesystem and networking
- Perfect for testing and development
- No external dependencies required

### Docker Deployment
**Containerized multi-node deployment**

```bash
./deploy.sh deploy --type docker --size 5 --env staging --verbose
```

**Requirements:**
- Docker 20.06+ with Compose v2
- 8GB+ RAM available for containers
- IPv6 networking support

**Features:**
- Isolated container networking
- Resource limits per container
- Easy horizontal scaling
- Production-like environment

### Systemd Deployment  
**Native Linux service deployment**

```bash
sudo ./deploy.sh deploy --type systemd --size 7 --env prod --verbose
```

**Requirements:**
- systemd-based Linux distribution
- Root access for service installation
- Dedicated network interfaces (recommended)

**Benefits:**
- Native OS integration
- Automatic startup/restart
- Journal logging integration
- Maximum performance

### Bare Metal Deployment
**Direct hardware deployment**

```bash
# On each physical machine
./deploy.sh deploy --type local --size 1 --env prod \
  --config /etc/nexus/node-config.toml
```

**Requirements:**
- Dedicated hardware per node
- Network connectivity between nodes
- Shared storage (optional, for state backup)

## 🌍 Environment Configurations

### Development Environment
```bash
export NEXUS_ENV=development
export RUST_LOG=debug
./deploy.sh deploy --type local --size 3 --env dev
```

**Settings:**
- Verbose logging enabled
- Reduced security for faster iteration  
- Small resource requirements
- Mock external services

### Staging Environment
```bash
./deploy.sh deploy --type docker --size 5 --env staging --verbose
```

**Settings:**
- Production-like configuration
- Real certificate management
- Full security policies
- Performance monitoring enabled

### Production Environment
```bash
sudo ./deploy.sh deploy --type systemd --size 7 --env prod \
  --config /etc/nexus/production.toml
```

**Settings:**
- Maximum security hardening
- High availability configuration
- Comprehensive monitoring
- Automated backup and recovery

## 📊 Sizing Guidelines

### Small Cluster (3 nodes)
```yaml
Resources per node:
- CPU: 2 cores minimum, 4 cores recommended
- RAM: 4GB minimum, 8GB recommended  
- Storage: 50GB minimum, 100GB recommended
- Network: 1Gbps minimum

Use cases:
- Development environments
- Small applications (<100 containers)
- Edge deployments
```

### Medium Cluster (5-7 nodes)
```yaml
Resources per node:
- CPU: 4 cores minimum, 8 cores recommended
- RAM: 8GB minimum, 16GB recommended
- Storage: 100GB minimum, 500GB recommended  
- Network: 1Gbps minimum, 10Gbps recommended

Use cases:
- Production workloads
- Medium applications (100-1000 containers)
- Multi-tenant environments
```

### Large Cluster (10+ nodes)  
```yaml
Resources per node:
- CPU: 8 cores minimum, 16 cores recommended
- RAM: 16GB minimum, 64GB recommended
- Storage: 500GB minimum, 2TB recommended
- Network: 10Gbps minimum, 25Gbps recommended

Use cases:
- Enterprise workloads
- Large applications (1000+ containers)  
- High-throughput data processing
```

## 🔧 Configuration Management

### Generate Base Configuration
```bash
# Generate template configuration
./deploy.sh deploy --dry-run --verbose --size 5 > base-config.yaml

# Customize for your environment
vim base-config.yaml

# Deploy with custom config
./deploy.sh deploy --config base-config.yaml
```

### Node-Specific Configuration
```toml
# /etc/nexus/node-1.toml
[node]
id = "nexus-prod-1"
listen_address = "[2001:db8::1]:8080"
advertise_address = "[2001:db8::1]:8080"
data_directory = "/var/lib/nexus"
log_level = "info"

[transport]
max_connections = 10000
keep_alive_interval = "30s"
certificate_path = "/etc/nexus/certs"
private_key_path = "/etc/nexus/private"

[consensus]  
election_timeout = "5s"
heartbeat_interval = "1s"
batch_size = 100
max_log_entries = 10000

[security]
tls_min_version = "1.3"
cipher_suites = ["TLS_AES_256_GCM_SHA384"]
require_client_cert = true

[storage]
backend = "rocksdb"
sync_writes = true
cache_size = "1GB"
block_size = "16KB"

[resources]
max_memory = "32GB"
max_cpu_cores = 16
max_containers = 1000

[networking]
ipv6_only = true
dns_servers = ["2001:4860:4860::8888", "2001:4860:4860::8844"]
mtu = 1500
```

### Cluster Bootstrap Configuration
```toml
# Bootstrap node configuration
[bootstrap]
enabled = true
initial_cluster = [
  "nexus-prod-1=[2001:db8::1]:8080",
  "nexus-prod-2=[2001:db8::2]:8080", 
  "nexus-prod-3=[2001:db8::3]:8080"
]
```

## 🔐 Security Configuration

### Certificate Management
```bash
# Generate cluster CA
openssl genrsa -out ca-key.pem 4096
openssl req -new -x509 -days 3650 -key ca-key.pem -out ca-cert.pem

# Generate node certificates
for i in {1..7}; do
  openssl genrsa -out node-$i-key.pem 2048
  openssl req -new -key node-$i-key.pem -out node-$i.csr
  openssl x509 -req -in node-$i.csr -CA ca-cert.pem -CAkey ca-key.pem \
    -CAcreateserial -out node-$i-cert.pem -days 365
done

# Install certificates
sudo mkdir -p /etc/nexus/certs
sudo cp *.pem /etc/nexus/certs/
sudo chown -R nexus:nexus /etc/nexus/certs
sudo chmod 600 /etc/nexus/certs/*-key.pem
```

### Firewall Configuration
```bash
# Allow Nexus inter-node communication
sudo ufw allow 8080/tcp
sudo ufw allow 8080/udp

# Allow monitoring
sudo ufw allow 9090/tcp

# Allow API access (if enabled)
sudo ufw allow 6443/tcp

# Enable firewall
sudo ufw enable
```

### User and Service Setup
```bash
# Create nexus user
sudo adduser --system --group --home /var/lib/nexus nexus

# Create service directories
sudo mkdir -p /var/lib/nexus/{data,logs}
sudo mkdir -p /etc/nexus/{certs,config}

# Set permissions
sudo chown -R nexus:nexus /var/lib/nexus
sudo chown -R nexus:nexus /etc/nexus
```

## 📈 Monitoring Setup

### Prometheus Integration
```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
- job_name: 'nexus-nodes'
  static_configs:
  - targets: ['node1:9090', 'node2:9090', 'node3:9090']

- job_name: 'nexus-containers'
  static_configs:
  - targets: ['node1:9091', 'node2:9091', 'node3:9091']
```

### Grafana Dashboard
```bash
# Import Nexus dashboard
curl -O https://raw.githubusercontent.com/hypermesh/grafana-dashboards/main/nexus-cluster.json

# Or use the built-in metrics
cargo run --bin nexus-test -- metrics --duration 300 --real-time
```

### Log Aggregation
```bash
# Forward logs to centralized system
sudo journalctl -u nexus-node-1 -f | logger -t nexus-node-1

# Or use dedicated log shipping
sudo apt install filebeat
# Configure filebeat.yml to ship /var/log/nexus/*.log
```

## 🚀 Deployment Automation

### CI/CD Integration
```yaml
# .github/workflows/deploy.yml
name: Deploy to Production
on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    
    - name: Build and Test
      run: |
        cd core/tests
        cargo run --bin nexus-test -- all --detailed-report
    
    - name: Deploy to Staging
      run: |
        cd core/deploy  
        ./deploy.sh deploy --type docker --size 5 --env staging
    
    - name: Integration Tests
      run: |
        cargo run --bin nexus-test -- staging --size 5 --deployment docker
    
    - name: Deploy to Production
      if: success()
      run: |
        ./deploy.sh deploy --type systemd --size 7 --env prod
```

### Ansible Playbook
```yaml
# deploy-nexus.yml
- hosts: nexus_cluster
  become: yes
  vars:
    nexus_version: "v1.0.0"
    cluster_size: 7
    
  tasks:
  - name: Install dependencies
    apt:
      name: [build-essential, pkg-config, clang, llvm]
      state: present
      
  - name: Create nexus user
    user:
      name: nexus
      system: yes
      home: /var/lib/nexus
      
  - name: Deploy nexus binary
    copy:
      src: target/release/nexus
      dest: /usr/local/bin/nexus
      mode: '0755'
      
  - name: Install configuration
    template:
      src: nexus.toml.j2
      dest: /etc/nexus/nexus.toml
      
  - name: Start nexus service
    systemd:
      name: nexus
      enabled: yes
      state: started
```

## 🔄 Operations

### Health Checks
```bash
# Check cluster status
./deploy.sh status

# Check individual node
curl -k https://[2001:db8::1]:8080/health

# Detailed health information
cargo run --bin nexus-test -- deploy --nodes 7 --health-check
```

### Scaling Operations
```bash
# Scale up cluster
./deploy.sh deploy --type systemd --size 9 --env prod

# Scale down cluster (graceful)
./deploy.sh scale --size 5 --graceful

# Rolling update
./deploy.sh update --rolling --max-unavailable 1
```

### Backup and Recovery
```bash
# Backup cluster state
./deploy.sh backup --output /backup/nexus-$(date +%Y%m%d).tar.gz

# Restore from backup
./deploy.sh restore --input /backup/nexus-20250828.tar.gz

# Point-in-time recovery
./deploy.sh restore --timestamp 2025-08-28T12:00:00Z
```

### Troubleshooting
```bash
# View logs
./deploy.sh logs --follow --node nexus-prod-1

# Debug mode
RUST_LOG=debug ./deploy.sh deploy --type local --size 1

# Network connectivity test
cargo run --bin nexus-test -- unit --component networking

# Consensus health check
cargo run --bin nexus-test -- unit --component consensus
```

## 📋 Pre-Production Checklist

### Infrastructure
- [ ] Hardware/VM resources provisioned
- [ ] Network connectivity verified (IPv6 + QUIC)
- [ ] DNS resolution configured
- [ ] Load balancer configuration (if applicable)
- [ ] Storage provisioned and tested

### Security
- [ ] TLS certificates generated and installed
- [ ] Firewall rules configured
- [ ] User accounts and permissions set
- [ ] Security scanning completed
- [ ] Vulnerability assessment performed

### Configuration
- [ ] Node configurations validated
- [ ] Resource limits configured
- [ ] Logging and monitoring setup
- [ ] Backup procedures tested
- [ ] Recovery procedures validated

### Testing
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Load testing completed
- [ ] Chaos engineering tests run
- [ ] Security penetration testing done

### Operations
- [ ] Monitoring dashboards configured
- [ ] Alerting rules defined
- [ ] Runbooks documented  
- [ ] Incident response procedures defined
- [ ] On-call rotation established

---

## 🎯 Next Steps

- **[API Reference](api/)** - Programmatic cluster management
- **[Examples](../examples/)** - Real-world deployment scenarios
- **[Developer Guide](developer-guide.md)** - Contributing and customization

Ready for production? Your Kubernetes replacement awaits! 🚀