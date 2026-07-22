<!-- Copyright © 2026 Hypermesh Foundation. All rights reserved. -->

# Exposing Services via Public Gateway

This guide covers how the HyperMesh gateway bridges clearnet HTTP/3 traffic to internal STOQ services, and how to make your node publicly accessible.

## Overview

HyperMesh services communicate internally over STOQ (a QUIC-based protocol with eBPF acceleration). The gateway at `trust.hypermesh.online` acts as the bridge between the clearnet (HTTP/3) and the internal mesh (STOQ). This allows non-HyperMesh clients — browsers, API consumers, monitoring tools — to access services without running a HyperMesh node themselves.

The gateway serves four roles:

1. **Clearnet Bootstrap**: Provides initial connection information so new nodes can join the mesh
2. **Inbound Proxy**: Routes clearnet HTTP/3 requests to internal STOQ services (dashboards, APIs)
3. **Outbound Proxy**: Bridges HyperMesh resources to non-HyperMesh clearnet endpoints
4. **Inter-Network Gateway**: Bridges between federated/private/public HyperMesh networks (STOQ-to-STOQ)

## How SNI Routing Works

The gateway uses TLS Server Name Indication (SNI) to route incoming connections. When a client connects, the gateway inspects the hostname in the TLS handshake and forwards the connection to the appropriate internal service.

```
Client connects to ngauge.hypermesh.online:8443
  -> Gateway reads SNI: "ngauge.hypermesh.online"
  -> Routes to ngauge-server at [::1]:9296 via STOQ
  -> Returns response to client over HTTP/3
```

All subdomains of `*.hypermesh.online` resolve to the gateway. The gateway handles TLS termination and protocol translation (HTTP/3 to STOQ).

## Available Public Endpoints

| Endpoint | Port | Service | Status |
|----------|------|---------|--------|
| `trust.hypermesh.online` | 8443 | Main gateway — health checks, dashboard, node bootstrap | Active |
| `ngauge.hypermesh.online` | 8443 | Engagement analytics and metrics | Active |
| `caesar.hypermesh.online` | 8443 | Caesar EVP network access | Future |
| `catalog.hypermesh.online` | 8443 | Public asset catalog | Future |

### Health check

```bash
curl -k https://trust.hypermesh.online:8443/health
```

### Bootstrap endpoint

New nodes use the bootstrap endpoint to get initial STOQ connection parameters:

```bash
curl -k https://trust.hypermesh.online:8443/bootstrap
```

This returns STOQ connection details (addresses, ports, initial certificates) so the new node can transition from HTTP/3 to native STOQ communication.

## Running a Public Node

To make your node part of the public HyperMesh mesh, set the privacy mode to `public`.

### Configure privacy mode

Edit `/etc/hypermesh/hypermesh.toml`:

```toml
[node]
privacy_mode = "public"
```

Or start with the flag:

```bash
hypermesh --privacy public
```

### Configure the gateway

Edit `/etc/hypermesh/gateway.toml`:

```toml
[gateway]
mode = "bootstrap"
http3_port = 8443
stoq_port = 8444

[tls]
# In alpha, use self-signed certificates
source = "self-signed"
# In production, use TrustChain-issued certificates
# source = "trustchain"
# endpoint = "quic://trust.hypermesh.online"

[rate_limit]
per_ip_rps = 100
per_identity_burst = 200
global_rps = 10000
max_payload_bytes = 10485760
max_connections_per_ip = 50
```

### Open the gateway port

```bash
sudo ufw allow 8443/udp
```

Only port 8443 needs to be exposed publicly. All internal STOQ ports (8444, 9292-9296) should remain firewalled.

### Verify public access

From an external machine:

```bash
curl -k https://<your-ipv6-address>:8443/health
```

## TrustChain Certificates

Public endpoints require TrustChain-issued certificates for authentication.

### Alpha phase (current)

During alpha, the gateway uses self-signed certificates. Clients must use the `-k` flag (or equivalent) to skip certificate verification. This is expected and temporary.

### Production phase (future)

In production, public nodes obtain certificates from the global TrustChain CA at `quic://trust.hypermesh.online`. These certificates use FALCON-1024 post-quantum signatures. Clients that trust the HyperMesh root certificate will verify connections without `-k`.

Certificate rotation is automatic. The gateway re-keys periodically without dropping active connections.

## CAESAR Rewards

Running a public node earns CAESAR rewards. The public privacy mode has the maximum reward multiplier.

| Privacy Mode | Reward Multiplier | Tracking |
|-------------|-------------------|----------|
| Anonymous | 0.0x (no rewards) | None |
| Private | 0.5x | Within group |
| Public | 1.0x (maximum) | Full transparency |

Rewards are calculated based on:

- **Storage contributions**: Hosting shards for other nodes
- **Network relay**: Forwarding STOQ packets through the mesh
- **Compute**: Processing asset operations for the network
- **Uptime**: Consistent availability increases reward weight

CAESAR rewards are tracked on the public Network blockchain. Your node's contributions and earnings are visible to all participants.

### Check your rewards

```bash
hypermesh caesar balance
```

### View contribution history

```bash
hypermesh caesar history
```

## Privacy Implications

Running a public node means full transparency:

- **Identity**: Your node's identity (TrustChain certificate, matrix position) is visible on the Network blockchain
- **Contributions**: Storage, compute, and network contributions are recorded and publicly auditable
- **Asset hosting**: Shards you host for the network are tracked (the shard contents remain encrypted)
- **Transactions**: CAESAR reward transactions are visible on the public chain

This is by design. Public mode maximizes trust and rewards by maximizing transparency. If you need privacy, use [Private mode](PRIVATE_NETWORK.md) instead.

## Load Balancing

The gateway supports four load balancing strategies for distributing requests across backend services:

| Strategy | Behavior |
|----------|----------|
| Round Robin | Equal distribution across backends |
| Least Connections | Routes to the backend with fewest active connections |
| Weighted Round Robin | Distributes based on configured weights |
| Health-Aware | Prefers backends passing health checks |

Configure in `/etc/hypermesh/gateway.toml`:

```toml
[load_balancer]
strategy = "health_aware"
health_check_interval_secs = 30
```

## DDoS Protection

The gateway includes built-in rate limiting:

- **Per-IP**: 100 requests per second
- **Per-identity**: 200 burst capacity (authenticated via PoS)
- **Global**: 10,000 requests per second total
- **Payload limit**: 10 MB maximum per request
- **Connection limit**: 50 concurrent connections per IP

These defaults are configured in `gateway.toml` and can be adjusted based on your node's capacity.

## Next Steps

- [Installing HyperMesh](INSTALL.md) — prerequisite setup
- [Private Network Integration](PRIVATE_INTEGRATION.md) — bridge your public node to private networks via federation
