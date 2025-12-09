# HTTP/3 Server Deployment Guide

## Overview

Both TrustChain and BlockMatrix HTTP/3 servers are now deployed using **STOQ transport** instead of direct Quinn. This provides the full benefits of the STOQ protocol layer including:

- Adaptive network tier optimization
- Connection pooling and multiplexing
- Memory pool optimizations
- Zero-copy operations
- Quantum-resistant cryptography support (FALCON)

## Server Configuration

### TrustChain HTTP/3 Server
- **URL**: `https://[::1]:50053`
- **Protocol**: HTTP/3 over QUIC (UDP)
- **Transport**: STOQ with Cubic congestion control
- **Endpoints**: See `/api/v1/trustchain/*` routes

### BlockMatrix HTTP/3 Server
- **URL**: `https://[::1]:8446`
- **Protocol**: HTTP/3 over QUIC (UDP)
- **Transport**: STOQ with Cubic congestion control
- **Endpoints**: See `/api/v1/blockmatrix/*` routes

## Start/Stop Scripts

### Start Both Servers
```bash
cd /home/persist/repos/projects/web3
./start-http3-servers.sh
```

### Stop Both Servers
```bash
cd /home/persist/repos/projects/web3
./stop-http3-servers.sh
```

### Check Server Health
```bash
cd /home/persist/repos/projects/web3
./check-http3-health.sh
```

## Verification

Both servers listen on **UDP** (not TCP) because they use QUIC as the transport protocol:

```bash
# Verify servers are listening
ss -ulnp6 | grep -E "50053|8446"

# Expected output:
# UNCONN ... [::1]:50053 ... trustchain-http
# UNCONN ... [::1]:8446 ... blockmatrix-htt
```

## Testing HTTP/3 Endpoints

**Note**: Regular `curl` does not support HTTP/3 well. You need either:

1. **curl with HTTP/3 support** (compiled with --with-nghttp3)
2. **HTTP/3 client libraries** (h3, hyper, etc.)
3. **Browser with HTTP/3 enabled** (Chrome, Firefox with QUIC flag)

### Using curl with HTTP/3 (if available)
```bash
# TrustChain health endpoint
curl --http3 -k https://[::1]:50053/api/v1/trustchain/health | jq

# BlockMatrix health endpoint
curl --http3 -k https://[::1]:8446/api/v1/blockmatrix/health | jq
```

### Using Rust HTTP/3 Client
See `/home/persist/repos/projects/web3/tests/http3_client_test.rs` for example client implementation.

## API Endpoints

### TrustChain Endpoints

#### Health & Monitoring
- `GET /api/v1/trustchain/health` - Server health status
- `GET /api/v1/trustchain/status` - Node status and metrics
- `GET /api/v1/trustchain/metrics` - Performance metrics

#### Certificate Management
- `GET /api/v1/trustchain/certificates` - List certificates
- `POST /api/v1/trustchain/certificates/issue` - Issue new certificate
- `POST /api/v1/trustchain/certificates/validate` - Validate certificate
- `GET /api/v1/trustchain/certificates/{id}` - Get specific certificate
- `POST /api/v1/trustchain/certificates/revoke` - Revoke certificate

#### DNS-as-Asset
- `POST /api/v1/trustchain/dns/resolve` - Resolve DNS record
- `GET /api/v1/trustchain/dns/zones` - List DNS zones
- `POST /api/v1/trustchain/dns/register` - Register new domain
- `GET /api/v1/trustchain/dns/record/{domain}` - Get domain records

#### Consensus
- `GET /api/v1/trustchain/consensus/status` - Consensus status
- `POST /api/v1/trustchain/consensus/validate` - Validate consensus proofs
- `GET /api/v1/trustchain/consensus/proofs/{asset_id}` - Get asset proofs

### BlockMatrix Endpoints

#### Health & Monitoring
- `GET /api/v1/blockmatrix/health` - Server health status
- `GET /api/v1/blockmatrix/status` - Node and matrix status

#### Asset Management
- `GET /api/v1/blockmatrix/assets` - List all assets
- `POST /api/v1/blockmatrix/assets/allocate` - Allocate new asset
- `GET /api/v1/blockmatrix/assets/{asset_id}` - Get specific asset details

## STOQ Transport Configuration

Both servers use the following STOQ configuration:

```rust
TransportConfig {
    bind_address: Ipv6Addr::LOCALHOST,
    port: 50053 | 8446,
    max_connections: None, // Unlimited
    connection_timeout: 30s,
    enable_migration: true,
    enable_0rtt: true,
    max_idle_timeout: 60s,
    cert_rotation_interval: 24h,
    max_concurrent_streams: 100,
    send_buffer_size: 2MB,
    receive_buffer_size: 2MB,
    connection_pool_size: 10,
    enable_zero_copy: true,
    enable_memory_pool: true,
    frame_batch_size: 64,
    enable_large_send_offload: true,
    congestion_control: Cubic,
    enable_falcon_crypto: false,
}
```

## Logs

Server logs are written to:
- TrustChain: `/tmp/http3-logs/trustchain-http3.log`
- BlockMatrix: `/tmp/http3-logs/blockmatrix-http3.log`

View logs in real-time:
```bash
# TrustChain
tail -f /tmp/http3-logs/trustchain-http3.log

# BlockMatrix
tail -f /tmp/http3-logs/blockmatrix-http3.log
```

## Troubleshooting

### Servers won't start
1. Check if ports are already in use: `ss -ulnp6 | grep -E "50053|8446"`
2. Check logs for errors: `cat /tmp/http3-logs/*.log`
3. Ensure binaries are built: `cargo build --bin trustchain-http3-server --bin blockmatrix-http3-server`

### Can't connect to servers
1. Verify servers are running: `ps aux | grep http3-server`
2. Verify UDP ports are listening: `ss -ulnp6 | grep -E "50053|8446"`
3. Use HTTP/3-capable client (not regular curl)
4. Check firewall rules for IPv6 localhost

### Performance issues
1. Check STOQ transport metrics in logs
2. Monitor CPU/memory usage: `top -p $(pgrep http3-server)`
3. Review STOQ configuration for optimization opportunities

## Production Deployment

For production deployment:

1. **Change bind address** from `::1` to actual IPv6 address
2. **Enable FALCON** quantum-resistant cryptography
3. **Configure proper certificates** via TrustChain CA
4. **Set connection limits** for DoS protection
5. **Enable eBPF** transport acceleration (if available)
6. **Monitor metrics** and adjust STOQ configuration
7. **Use production-grade** certificate management

## Integration with UI

The servers have CORS headers enabled for UI integration. UI can make requests from browsers with HTTP/3 support:

```javascript
// Example using fetch with HTTP/3
const response = await fetch('https://[::1]:50053/api/v1/trustchain/health');
const data = await response.json();
console.log(data);
```

**Note**: Browser must support HTTP/3 and the server certificate must be trusted.

## Next Steps

1. **Create HTTP/3 client wrapper** for easier testing
2. **Add UI integration** with proper HTTP/3 support
3. **Implement monitoring dashboard** for STOQ metrics
4. **Production certificate** setup via TrustChain
5. **Load testing** with multiple concurrent connections
6. **Enable FALCON** for quantum-resistant security

## References

- STOQ Protocol: `/home/persist/repos/projects/web3/stoq/`
- TrustChain Server: `/home/persist/repos/projects/web3/trustchain/src/bin/trustchain-http3-server.rs`
- BlockMatrix Server: `/home/persist/repos/projects/web3/blockmatrix/src/bin/blockmatrix-http3-server.rs`
- STOQ Transport: `/home/persist/repos/projects/web3/stoq/src/transport/mod.rs`
