# HTTP/3 Production Deployment Guide

## Overview

This guide covers the deployment of TrustChain and BlockMatrix HTTP/3 servers in production environments. These servers provide high-performance, secure communication using the QUIC protocol over IPv6 with full CORS support.

## Architecture

```
┌─────────────────────────────────────────────────┐
│            HTTP/3 Server Infrastructure         │
├─────────────────────────────────────────────────┤
│                                                 │
│  ┌──────────────────┐  ┌──────────────────┐   │
│  │   TrustChain     │  │   BlockMatrix    │   │
│  │   HTTP/3 Server  │  │   HTTP/3 Server  │   │
│  │   Port: 50053    │  │   Port: 8446     │   │
│  └────────┬─────────┘  └────────┬─────────┘   │
│           │                      │              │
│           └──────────┬───────────┘              │
│                      │                          │
│              ┌───────▼────────┐                │
│              │  QUIC/UDP      │                │
│              │  IPv6 Stack    │                │
│              └────────────────┘                │
└─────────────────────────────────────────────────┘
```

## Prerequisites

### System Requirements

- **Operating System**: Linux (Ubuntu 20.04+, RHEL 8+, or compatible)
- **Memory**: Minimum 2GB RAM (4GB recommended)
- **CPU**: 2+ cores recommended for production
- **Network**: IPv6 enabled
- **Ports**: UDP 50053 and 8446 must be accessible

### Software Dependencies

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install system dependencies
sudo apt-get update && sudo apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    lsof \
    bc
```

### Firewall Configuration

```bash
# Open QUIC/UDP ports
sudo ufw allow 50053/udp comment 'TrustChain HTTP/3'
sudo ufw allow 8446/udp comment 'BlockMatrix HTTP/3'

# For iptables
sudo iptables -A INPUT -p udp --dport 50053 -j ACCEPT
sudo iptables -A INPUT -p udp --dport 8446 -j ACCEPT
sudo ip6tables -A INPUT -p udp --dport 50053 -j ACCEPT
sudo ip6tables -A INPUT -p udp --dport 8446 -j ACCEPT
```

## Installation

### 1. Clone Repository

```bash
git clone https://github.com/hypermesh-online/web3-ecosystem.git
cd web3-ecosystem
```

### 2. Build Servers

```bash
# Build in release mode for production
cd trustchain
cargo build --release --bin trustchain-http3-server

cd ../blockmatrix
cargo build --release --bin blockmatrix-http3-server
```

### 3. Verify Build

```bash
ls -la target/release/*http3*
# Should show both server binaries
```

## Configuration

### Environment Variables

```bash
# Set production logging level
export RUST_LOG=info,quinn=warn,h3=warn

# Enable backtraces for debugging
export RUST_BACKTRACE=1

# Optional: Custom certificate paths
export CERT_PATH=/etc/http3/certs/server.crt
export KEY_PATH=/etc/http3/certs/server.key
```

### TLS Certificates

The servers use self-signed certificates by default. For production, you should use proper certificates:

```bash
# Create certificate directory
sudo mkdir -p /etc/http3/certs

# Generate production certificates (example using Let's Encrypt)
certbot certonly --standalone -d your-domain.com

# Copy certificates
sudo cp /etc/letsencrypt/live/your-domain.com/fullchain.pem /etc/http3/certs/server.crt
sudo cp /etc/letsencrypt/live/your-domain.com/privkey.pem /etc/http3/certs/server.key
```

## Deployment

### Quick Start

```bash
# Start both servers in production mode
./start-http3-production.sh

# Verify health
./validate-http3-health.sh

# Check logs
tail -f ~/.http3/logs/*.log
```

### Manual Deployment

```bash
# Start TrustChain server
cd trustchain
nohup ./target/release/trustchain-http3-server > /var/log/trustchain.log 2>&1 &

# Start BlockMatrix server
cd ../blockmatrix
nohup ./target/release/blockmatrix-http3-server > /var/log/blockmatrix.log 2>&1 &
```

### Systemd Service (Recommended)

Create service files for automatic startup and management:

#### TrustChain Service

```ini
# /etc/systemd/system/trustchain-http3.service
[Unit]
Description=TrustChain HTTP/3 Server
After=network.target
StartLimitIntervalSec=0

[Service]
Type=simple
Restart=always
RestartSec=5
User=http3
WorkingDirectory=/home/persist/repos/projects/web3/trustchain
ExecStart=/home/persist/repos/projects/web3/target/release/trustchain-http3-server
Environment="RUST_LOG=info,quinn=warn,h3=warn"
StandardOutput=append:/var/log/http3/trustchain.log
StandardError=append:/var/log/http3/trustchain.log

[Install]
WantedBy=multi-user.target
```

#### BlockMatrix Service

```ini
# /etc/systemd/system/blockmatrix-http3.service
[Unit]
Description=BlockMatrix HTTP/3 Server
After=network.target
StartLimitIntervalSec=0

[Service]
Type=simple
Restart=always
RestartSec=5
User=http3
WorkingDirectory=/home/persist/repos/projects/web3/blockmatrix
ExecStart=/home/persist/repos/projects/web3/target/release/blockmatrix-http3-server
Environment="RUST_LOG=info,quinn=warn,h3=warn"
StandardOutput=append:/var/log/http3/blockmatrix.log
StandardError=append:/var/log/http3/blockmatrix.log

[Install]
WantedBy=multi-user.target
```

#### Enable Services

```bash
# Create http3 user
sudo useradd -r -s /bin/false http3

# Create log directory
sudo mkdir -p /var/log/http3
sudo chown http3:http3 /var/log/http3

# Enable and start services
sudo systemctl daemon-reload
sudo systemctl enable trustchain-http3.service
sudo systemctl enable blockmatrix-http3.service
sudo systemctl start trustchain-http3.service
sudo systemctl start blockmatrix-http3.service

# Check status
sudo systemctl status trustchain-http3.service
sudo systemctl status blockmatrix-http3.service
```

## Monitoring

### Health Endpoints

- **TrustChain**: `https://[::1]:50053/health`
- **BlockMatrix**: `https://[::1]:8446/api/v1/blockmatrix/health`

### Automated Health Checks

```bash
# Run health check
./validate-http3-health.sh

# Setup cron job for regular checks
echo "*/5 * * * * /home/persist/repos/projects/web3/validate-http3-health.sh" | crontab -
```

### Log Monitoring

```bash
# View logs
tail -f /var/log/http3/trustchain.log
tail -f /var/log/http3/blockmatrix.log

# Monitor for errors
grep ERROR /var/log/http3/*.log

# Check restart events
tail -f /var/log/http3/restarts.log
```

### Performance Monitoring

```bash
# Monitor resource usage
htop -p $(pgrep -d, -f "http3-server")

# Check connection count
ss -tunlp | grep -E "(50053|8446)"

# Monitor network traffic
iftop -P -n -N -i any -f "port 50053 or port 8446"
```

### Metrics Collection

The servers expose Prometheus-compatible metrics:

- **TrustChain Metrics**: `https://[::1]:50053/metrics`
- **BlockMatrix Metrics**: `https://[::1]:8446/metrics`

```yaml
# prometheus.yml configuration
scrape_configs:
  - job_name: 'trustchain'
    scheme: https
    tls_config:
      insecure_skip_verify: true
    static_configs:
      - targets: ['[::1]:50053']

  - job_name: 'blockmatrix'
    scheme: https
    tls_config:
      insecure_skip_verify: true
    static_configs:
      - targets: ['[::1]:8446']
```

## High Availability

### Load Balancing

For production deployments with multiple instances:

```nginx
# nginx.conf for HTTP/3 load balancing
upstream trustchain_backend {
    server [::1]:50053 weight=1;
    server [::1]:50054 weight=1;
}

upstream blockmatrix_backend {
    server [::1]:8446 weight=1;
    server [::1]:8447 weight=1;
}
```

### Failover Configuration

The production scripts include automatic restart on failure:

```bash
# Monitor processes are automatically started
ps aux | grep monitor.pid

# Manual failover
./stop-http3-production.sh
./start-http3-production.sh
```

## Troubleshooting

### Common Issues

#### 1. Server Won't Start

```bash
# Check if ports are in use
lsof -i :50053
lsof -i :8446

# Check logs
tail -n 100 /var/log/http3/*.log

# Verify binary exists
ls -la target/release/*http3-server
```

#### 2. Connection Refused

```bash
# Check firewall
sudo ufw status
sudo iptables -L -n | grep -E "(50053|8446)"

# Verify IPv6
ip -6 addr show

# Test UDP connectivity
nc -zvu ::1 50053
```

#### 3. Performance Issues

```bash
# Check system resources
free -h
df -h
iostat -x 1

# Monitor server performance
./validate-http3-health.sh

# Check for connection limits
ulimit -n
```

### Debug Mode

For troubleshooting, run servers in debug mode:

```bash
# Enable verbose logging
export RUST_LOG=debug,quinn=debug,h3=debug

# Run server in foreground
./target/release/trustchain-http3-server
```

## Upgrade Procedures

### Rolling Updates

1. **Build new version**:
```bash
git pull
cargo build --release --bin trustchain-http3-server
cargo build --release --bin blockmatrix-http3-server
```

2. **Deploy to staging**:
```bash
# Test on non-production port
TRUSTCHAIN_PORT=50054 ./start-http3-production.sh
./validate-http3-health.sh
```

3. **Production deployment**:
```bash
# Stop old version
./stop-http3-production.sh

# Start new version
./start-http3-production.sh

# Verify
./validate-http3-health.sh
```

### Rollback Procedure

```bash
# Keep previous binaries
cp target/release/*http3-server target/release/*.backup

# If issues, restore
cp target/release/*.backup target/release/*http3-server
./stop-http3-production.sh
./start-http3-production.sh
```

## Security Considerations

### TLS Configuration

- Use TLS 1.3 minimum
- Generate strong certificates (RSA 4096 or EC P-384)
- Rotate certificates regularly
- Implement certificate pinning for clients

### Network Security

```bash
# Restrict source IPs (example)
sudo iptables -A INPUT -p udp --dport 50053 -s 192.168.0.0/16 -j ACCEPT
sudo iptables -A INPUT -p udp --dport 50053 -j DROP
```

### Process Isolation

```bash
# Run as non-root user
sudo useradd -r -s /bin/false http3
sudo chown -R http3:http3 /var/log/http3

# Set resource limits
echo "http3 soft nofile 65536" >> /etc/security/limits.conf
echo "http3 hard nofile 65536" >> /etc/security/limits.conf
```

## Performance Tuning

### System Optimization

```bash
# Increase UDP buffers
echo "net.core.rmem_max = 134217728" >> /etc/sysctl.conf
echo "net.core.wmem_max = 134217728" >> /etc/sysctl.conf
echo "net.ipv4.udp_mem = 65536 131072 262144" >> /etc/sysctl.conf
echo "net.ipv4.udp_rmem_min = 8192" >> /etc/sysctl.conf
echo "net.ipv4.udp_wmem_min = 8192" >> /etc/sysctl.conf
sudo sysctl -p
```

### Application Tuning

```rust
// In server configuration
const MAX_CONCURRENT_STREAMS: u64 = 1000;
const MAX_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(15);
```

## Support

### Logs Location

- **Application Logs**: `/var/log/http3/*.log` or `~/.http3/logs/*.log`
- **System Logs**: `/var/log/syslog`, `journalctl -u trustchain-http3`
- **Restart Logs**: `/var/log/http3/restarts.log`

### Getting Help

- **Documentation**: Check this guide and inline code documentation
- **Issues**: Report bugs via GitHub issues
- **Monitoring**: Use health check endpoints and metrics

### Emergency Procedures

```bash
# Emergency stop all HTTP/3 servers
pkill -f http3-server

# Clear all state and restart
rm -rf ~/.http3/pids/*
./start-http3-production.sh

# Full system restart
sudo systemctl restart trustchain-http3
sudo systemctl restart blockmatrix-http3
```

## Appendix

### Performance Benchmarks

Expected performance metrics:

- **Connection Establishment**: <10ms
- **Request Latency**: <5ms (P50), <10ms (P95), <20ms (P99)
- **Throughput**: >10,000 requests/second per server
- **Concurrent Connections**: >50,000 per server
- **Memory Usage**: <500MB per server under normal load

### Compliance

The HTTP/3 implementation follows:

- **RFC 9114**: HTTP/3 specification
- **RFC 9000**: QUIC transport protocol
- **RFC 9001**: QUIC TLS integration
- **CORS**: W3C Cross-Origin Resource Sharing specification

### Version History

- **v1.0.0**: Initial HTTP/3 implementation with basic endpoints
- **v1.1.0**: Added CORS support and performance optimizations
- **v1.2.0**: Production hardening and monitoring improvements