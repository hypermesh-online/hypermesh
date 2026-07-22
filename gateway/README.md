# Gateway

HTTP/3 + STOQ gateway for `*.hypermesh.online`, serving as the clearnet entry point and inter-network bridge for the HyperMesh ecosystem.

**Status**: 100% Complete (alpha) | 24 files | ~7,000 lines | 155 tests

## Four Roles

| Role | Protocol | Purpose |
|------|----------|---------|
| **Bootstrap** | HTTP/3 -> STOQ | Initial STOQ connection info + bootstrap tokens at `trust.hypermesh.online` |
| **Inbound Proxy** | HTTP/3 -> HyperMesh | Clearnet access to HyperMesh dashboards (resource, ngauge, caesar, catalog) |
| **Outbound Proxy** | HyperMesh -> HTTP/3 | Bridge HyperMesh resources to non-HyperMesh clearnet endpoints |
| **Inter-Network** | STOQ <-> STOQ | Bridge between federated/private/public HyperMesh networks |

## Architecture

```
                Internet (HTTP/3)
                      |
          +-----------+-----------+
          |                       |
    HTTP/3 :8443            STOQ :8444
          |                       |
    +-----v-----------------------v-----+
    |            Gateway                 |
    |                                    |
    |  Router -> Auth -> Rate Limiter    |
    |  -> Load Balancer -> Backend       |
    |                                    |
    |  ScopeRouter (Device <-> Network)  |
    |  FederationBridge (cross-network)  |
    +------------------------------------+
```

Dual-listener: HTTP/3 on port 8443 + STOQ on port 8444 via `tokio::select`.

## 20 Modules

`auth`, `bootstrap`, `config`, `domain_router`, `error`, `federation`, `gateway_mode`, `health`, `inbound`, `load_balancer`, `middleware`, `outbound`, `pool`, `proxy`, `rate_limiter`, `router`, `scope_bridge_proxy`, `scope_router`, `stoq_bridge`, `stoq_listener`, `tls`

## Key Features

### TLS
- `TlsProvider` supports three certificate sources: File (PEM/DER), TrustChain (FALCON-1024), SelfSigned (rcgen)
- Multi-domain SNI routing for `*.hypermesh.online` wildcard

### Authentication
- PoS authentication and session management
- Bootstrap token flow (HTTP/3 to STOQ transition)
- `AuthResult`: Authenticated | BootstrapRequired | Rejected | Anonymous

### Cross-Scope Routing
- `ScopeRouter` routes between Device and Network blockchain scopes
- `ScopeBridge` manages Lock -> Transfer -> Unlock lifecycle for cross-scope asset transfers
- `RouteDecision`: Direct | ViaGateway | ViaFederation | Denied

### Rate Limiting and DDoS Protection
- Token bucket algorithm: per-IP (100 rps), per-identity (200 burst), global (10K rps)
- Payload limit: 10 MB
- Connection limit: 50 per IP

### Load Balancing
- RoundRobin, LeastConnections, WeightedRoundRobin, HealthAware

### Federation
- `FederationBridge` with trust levels: Full, Conditional, Untrusted
- Max peers limit, policy gates per trust level

### Resilience
- Connection pool with health checks (10 persistent connections per backend)
- Circuit breaker (opens after 5 failures, 30s cooldown)
- Automatic retry with exponential backoff (3 attempts)
- CORS middleware and request ID propagation

## Domain Routing

| Domain/Path | Backend |
|-------------|---------|
| `trust.hypermesh.online` | TrustChain |
| `caesar.hypermesh.online` | Caesar |
| `catalog.hypermesh.online` | Catalog |
| `/health` | Gateway internal |

## Quick Start

```bash
cargo build -p gateway --release
cargo test -p gateway
```

## Recent Changes
- **STOQ bridge wired into main.rs**: Dual HTTP/3 + STOQ listener active in production binary
- **Real PoS authentication**: WireSignedProof FALCON-1024 signature verification
- **TOML config loader**: Configuration file support with environment variable overrides

## Configuration

- `GatewayMode`: Bootstrap | InboundProxy | OutboundProxy | InterNetwork
- `CertificateSource`: File | TrustChain | SelfSigned
- Outbound proxy with allowlist filtering
- Privacy-mode-aware forwarding (Anonymous/Private/Public)

## License

Business Source License 1.1
