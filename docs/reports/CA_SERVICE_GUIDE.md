# TrustChain CA Service - Quick Start Guide

## Overview

The TrustChain CA service provides certificate issuance for BlockMatrix nodes to enable secure STOQ communication. This guide covers local development usage and production deployment.

## Local Development

### Starting the CA Service

```bash
# Start local CA (listens on [::1]:8443)
/home/persist/repos/projects/web3/scripts/start-ca.sh local

# Or manually:
cd /home/persist/repos/projects/web3
./target/release/trustchain_ca

# Verify CA is running:
ss -uln | grep 8443
# Expected: UNCONN 0 0 [::1]:8443 [::]:*
```

### Stopping the CA Service

```bash
# If started with script (interactive mode):
Ctrl+C

# If started manually or in background:
pkill trustchain_ca

# Or find PID and kill:
ps aux | grep trustchain_ca
kill <PID>
```

### CA Service Logs

```bash
# View logs:
tail -f /tmp/ca.log

# Check for errors:
grep -i error /tmp/ca.log

# Check certificate issuance:
grep "Certificate issued" /tmp/ca.log
```

## Testing Multi-Node Communication

### 1. Start the CA Service

```bash
/home/persist/repos/projects/web3/scripts/start-ca.sh local
```

### 2. Rebuild Node Binary (with updated STOQ)

```bash
cd /home/persist/repos/projects/web3
cargo build -p blockmatrix --bin node
```

### 3. Start Multi-Node Network

```bash
# Start 3 nodes in public mode
/home/persist/repos/projects/web3/scripts/start-multi-node.sh 3 public

# Monitor node logs:
tail -f /tmp/blockmatrix-nodes/node-*.log

# Check for certificate requests:
grep "Requesting certificate" /tmp/blockmatrix-nodes/*.log

# Check for successful connections:
grep "Connected" /tmp/blockmatrix-nodes/*.log
```

## CA Service Endpoints

All endpoints use STOQ/QUIC protocol on UDP port 8443.

### GET /health
Health check endpoint.

**Response:**
```json
{
  "status": "healthy",
  "ca_id": "trustchain-ca-local",
  "mode": "LocalhostTesting",
  "uptime_seconds": 123
}
```

### GET /ca/root
Get the root CA certificate.

**Response:**
```json
{
  "certificate_pem": "-----BEGIN CERTIFICATE-----\n...",
  "fingerprint": "abc123..."
}
```

### POST /certificate/issue
Issue a new certificate (full control).

**Request:**
```json
{
  "node_id": "node-001",
  "common_name": "node1.localhost",
  "ipv6_addresses": ["::1", "2001:db8::1"],
  "san_entries": ["node1.localhost", "*.node1.localhost"]
}
```

**Response:**
```json
{
  "certificate_pem": "-----BEGIN CERTIFICATE-----\n...",
  "chain_pem": "-----BEGIN CERTIFICATE-----\n...",
  "serial_number": "abc123...",
  "fingerprint": "def456...",
  "expires_at": 1234567890
}
```

### GET /certificate?node_id=X&common_name=Y
Simple certificate request (auto-issue in dev mode).

**Example:**
```
GET /certificate?node_id=node-001&common_name=node1.localhost
```

**Response:**
```json
{
  "certificate_pem": "-----BEGIN CERTIFICATE-----\n...",
  "chain_pem": "-----BEGIN CERTIFICATE-----\n...",
  "serial_number": "abc123..."
}
```

### POST /certificate/validate
Validate a certificate chain.

**Request:**
```json
{
  "certificate_pem": "-----BEGIN CERTIFICATE-----\n..."
}
```

**Response:**
```json
{
  "valid": true,
  "common_name": "node1.localhost",
  "issuer": "trustchain-ca-local",
  "expires_at": 1234567890
}
```

## Production Deployment

### Building for Production

```bash
cd /home/persist/repos/projects/web3/trustchain
cargo build --bin trustchain_ca --release
```

### Starting in Production Mode

```bash
# Start production CA (binds to all IPv6 interfaces)
./target/release/trustchain_ca --production

# Or with custom port:
./target/release/trustchain_ca --production --port 8443

# Or with custom bind address:
./target/release/trustchain_ca --production --bind "[2001:db8::1]:8443"
```

### Production Configuration

**Key Differences from Development:**
- Binds to `[::]:8443` (all interfaces) instead of `[::1]:8443`
- Auto-issuance disabled (requires valid consensus proofs)
- More stringent validation
- Production consensus requirements

### GCP Deployment

1. **Create GCP VM with IPv6**:
   ```bash
   gcloud compute instances create trustchain-ca \
     --zone=us-central1-a \
     --machine-type=n1-standard-2 \
     --network-interface=network-tier=PREMIUM,subnet=default,stack-type=IPV4_IPV6 \
     --image-family=ubuntu-2204-lts \
     --image-project=ubuntu-os-cloud
   ```

2. **Configure Firewall**:
   ```bash
   gcloud compute firewall-rules create allow-trustchain-ca \
     --allow=udp:8443 \
     --source-ranges=0.0.0.0/0,::/0 \
     --description="Allow TrustChain CA traffic"
   ```

3. **Configure DNS**:
   ```
   trust.hypermesh.online AAAA 2001:db8::1  (VM's IPv6)
   ```

4. **Deploy Binary**:
   ```bash
   scp target/release/trustchain_ca vm:/usr/local/bin/
   ssh vm "chmod +x /usr/local/bin/trustchain_ca"
   ```

5. **Create Systemd Service**:
   ```ini
   [Unit]
   Description=TrustChain CA Service
   After=network.target

   [Service]
   Type=simple
   User=trustchain
   ExecStart=/usr/local/bin/trustchain_ca --production
   Restart=always
   RestartSec=10

   [Install]
   WantedBy=multi-user.target
   ```

6. **Start Service**:
   ```bash
   systemctl enable trustchain-ca
   systemctl start trustchain-ca
   systemctl status trustchain-ca
   ```

## Monitoring

### Key Metrics to Monitor

- **Certificate Issuance Rate**: Certificates/second
- **Issuance Latency**: Time to issue certificate (<35ms target)
- **Error Rate**: Failed certificate requests
- **Uptime**: Service availability (99.9% target)

### Log Analysis

```bash
# Certificate issuance count:
grep "Certificate issued" /tmp/ca.log | wc -l

# Average issuance time:
grep "Certificate issued" /tmp/ca.log | grep -oP '\d+ms' | awk '{sum+=$1; count++} END {print sum/count "ms"}'

# Error rate:
grep -i error /tmp/ca.log | wc -l

# Connection attempts:
grep "Requesting certificate" /tmp/ca.log | wc -l
```

## Troubleshooting

### CA Won't Start

**Check port availability:**
```bash
ss -uln | grep 8443
# If already bound, find and stop the process
lsof -i :8443
```

**Check binary exists:**
```bash
ls -la /home/persist/repos/projects/web3/target/release/trustchain_ca
```

**Check logs for errors:**
```bash
tail -50 /tmp/ca.log | grep -i error
```

### Nodes Can't Connect

**Verify CA is listening:**
```bash
ss -uln | grep 8443
# Should show: UNCONN 0 0 [::1]:8443 [::]:*
```

**Check STOQ endpoint configuration:**
```bash
# In /home/persist/repos/projects/web3/stoq/src/transport/certificates.rs
# Should have: trustchain_endpoint: Some("quic://[::1]:8443".to_string())
```

**Verify IPv6 is enabled:**
```bash
ping6 -c 3 ::1
```

**Check firewall:**
```bash
sudo iptables -L -n | grep 8443
```

### Certificate Issuance Fails

**Check consensus proof validation:**
```bash
grep "consensus validation" /tmp/ca.log
```

**Verify node identity:**
```bash
grep "node_id" /tmp/ca.log
```

**Check certificate store:**
```bash
# In production, verify certificate database isn't corrupted
```

## Security Considerations

### Development Mode
- Self-signed root CA (NOT suitable for production)
- Auto-issuance enabled (convenience over security)
- Accepts testing consensus proofs
- Localhost-only binding

### Production Mode
- Proper root CA (consider HSM storage)
- Auto-issuance disabled
- Validates real consensus proofs
- Rate limiting recommended
- Certificate revocation list (CRL) needed
- Audit logging required
- Regular security updates

## Performance Tuning

### For High Load

Edit certificate pool size in CA config:
```rust
// In trustchain_ca.rs
let ca_config = CAConfig {
    // ... other config
    cert_pool_size: 1000,  // Increase for high load
    // ...
};
```

### Connection Limits

```bash
# Increase ulimit for CA process:
ulimit -n 65536

# Update systemd service:
[Service]
LimitNOFILE=65536
```

## Support & Contact

For issues or questions:
- Check logs: `/tmp/ca.log`
- Review documentation: This guide
- Test connectivity: `ss -uln | grep 8443`
- Verify CA process: `ps aux | grep trustchain_ca`

## Quick Commands Reference

```bash
# Start CA (local dev)
/home/persist/repos/projects/web3/scripts/start-ca.sh local

# Start CA (production)
./target/release/trustchain_ca --production

# Check CA status
ss -uln | grep 8443
ps aux | grep trustchain_ca

# View logs
tail -f /tmp/ca.log

# Stop CA
pkill trustchain_ca

# Test multi-node
/home/persist/repos/projects/web3/scripts/start-multi-node.sh 3 public

# Check node certificate requests
grep "certificate" /tmp/blockmatrix-nodes/*.log
```

## Next Steps

1. Rebuild node binary with updated STOQ
2. Test multi-node communication with local CA
3. Validate certificate issuance and STOQ connections
4. Plan production deployment to trust.hypermesh.online
5. Implement monitoring and alerting
6. Set up certificate rotation automation
