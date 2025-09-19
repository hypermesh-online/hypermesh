# Web3 Ecosystem Domain Configuration

## Current Domain Structure
**Primary Domain**: hypermesh.online
**Migration Status**: ✅ Complete (from internet2.org)
**Date**: September 19, 2025

## Domain Mapping

| Service | Domain | Port | Purpose |
|---------|--------|------|---------|
| Main Dashboard | hypermesh.online | 8443 | Primary entry point |
| TrustChain | trust.hypermesh.online | 8443 | Certificate Authority |
| Caesar | caesar.hypermesh.online | 8443 | Economic system |
| Catalog | catalog.hypermesh.online | 8443 | VM/Compute catalog |
| STOQ | stoq.hypermesh.online | 8443 | Protocol gateway |
| NGauge | ngauge.hypermesh.online | 8443 | Engagement platform |

## DNS Setup

### Local Development
```bash
# Run setup script
./setup-local-dns.sh

# Or manual configuration in /etc/hosts:
::1 hypermesh.online trust.hypermesh.online caesar.hypermesh.online
::1 catalog.hypermesh.online stoq.hypermesh.online ngauge.hypermesh.online
```

### Production DNS
- **Type**: AAAA records (IPv6 only)
- **TTL**: 300 seconds
- **Provider**: Cloudflare recommended
- **DDoS Protection**: Enabled via Cloudflare

## Certificate Configuration
- **Root CA**: hypermesh.online
- **Wildcard**: *.hypermesh.online
- **Validity**: 90 days with auto-renewal
- **Algorithm**: FALCON-1024 (quantum-resistant)

## IPv6 Configuration
- **Primary**: All services IPv6-only
- **Fallback**: IPv4 through proxy only
- **Localhost**: ::1 (not 127.0.0.1)
- **Any Address**: :: (not 0.0.0.0)

## Port Configuration
- **HTTPS/QUIC**: 8443 (unified)
- **DNS-over-STOQ**: 8853
- **Monitoring**: 9090 (Prometheus)
- **Metrics**: 9091 (internal)

## Validation
```bash
# Test domain resolution
dig AAAA trust.hypermesh.online

# Test certificate
openssl s_client -connect trust.hypermesh.online:8443

# Test STOQ connectivity
curl -k https://trust.hypermesh.online:8443/health
```

## Troubleshooting
1. **DNS not resolving**: Check /etc/hosts or local DNS server
2. **Certificate errors**: Regenerate with setup script
3. **Connection refused**: Verify services running on port 8443
4. **IPv6 issues**: Enable IPv6 in system settings