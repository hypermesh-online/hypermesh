# HTTP/3 Gateway

Unified HTTP/3 gateway for the HyperMesh ecosystem, routing requests to backend services.

## Architecture

```
                    ┌─────────────────┐
                    │   UI (5173)     │
                    └────────┬────────┘
                             │
                         HTTP/3
                             │
                    ┌────────▼────────┐
                    │  Gateway (8443) │
                    │                  │
                    │  - Routing       │
                    │  - CORS          │
                    │  - Pooling       │
                    │  - Retry         │
                    └────┬───────┬────┘
                         │       │
                    HTTP/3      HTTP/3
                         │       │
            ┌────────────▼───┐ ┌─▼──────────────┐
            │ TrustChain     │ │ BlockMatrix    │
            │   (50053)      │ │    (8446)      │
            └────────────────┘ └────────────────┘
```

## Features

- **HTTP/3 over QUIC**: Modern, efficient protocol
- **Service Routing**: Path-based routing to backend services
- **Connection Pooling**: 10 persistent connections per backend
- **Automatic Retry**: Exponential backoff with max 3 attempts
- **Circuit Breaker**: Prevents cascading failures
- **CORS Support**: Full CORS for browser compatibility
- **Request Tracing**: X-Request-ID propagation
- **Health Checks**: Real-time backend status monitoring

## Configuration

Environment variables:
- `GATEWAY_LISTEN_ADDR`: Gateway listening address (default: `[::]:8443`)
- `TRUSTCHAIN_ADDR`: TrustChain backend (default: `[::1]:50053`)
- `BLOCKMATRIX_ADDR`: BlockMatrix backend (default: `[::1]:8446`)
- `CERT_PATH`: TLS certificate path
- `KEY_PATH`: TLS private key path
- `LOG_LEVEL`: Logging level (default: `info`)

## Routing Rules

| Path Pattern | Backend | Port |
|--------------|---------|------|
| `/api/v1/trustchain/*` | TrustChain | 50053 |
| `/api/v1/blockmatrix/*` | BlockMatrix | 8446 |
| `/api/v1/hypermesh/*` | BlockMatrix | 8446 |
| `/api/v1/stoq/*` | BlockMatrix | 8446 |
| `/api/v1/caesar/*` | BlockMatrix | 8446 |
| `/health` | Gateway | - |

## API Endpoints

### Health Check
```
GET /health

Response:
{
  "status": "healthy",
  "backends": {
    "trustchain": { "status": "up", "latency_ms": 2 },
    "blockmatrix": { "status": "up", "latency_ms": 1 }
  },
  "version": "0.1.0",
  "statistics": { ... }
}
```

## Performance Characteristics

- **Gateway Overhead**: <5ms per request
- **Concurrent Connections**: 1000+ supported
- **Connection Pool**: 10 connections per backend
- **Reconnection Time**: <100ms on failure
- **Request Timeout**: 10 seconds default
- **Retry Delay**: 100ms base, exponential backoff

## Building

```bash
cd gateway
cargo build --release
```

## Running

```bash
# Start the gateway
cargo run --release

# Or with custom configuration
GATEWAY_LISTEN_ADDR=[::]:8443 \
TRUSTCHAIN_ADDR=[::1]:50053 \
BLOCKMATRIX_ADDR=[::1]:8446 \
cargo run --release
```

## Testing

```bash
# Run all tests
cargo test

# Run with logging
RUST_LOG=gateway=debug cargo test -- --nocapture
```

## Deployment

1. Ensure TLS certificates are available at `/home/persist/repos/projects/web3/certs/`
2. Start backend services (TrustChain on 50053, BlockMatrix on 8446)
3. Start the gateway: `./target/release/gateway`
4. Verify health: `curl https://localhost:8443/health`

## Monitoring

The gateway exposes detailed metrics through the `/health` endpoint:
- Backend health status
- Connection pool statistics
- Request counts and latencies
- Circuit breaker status

## Security

- TLS 1.3 with strong ciphers
- Certificate validation for backends
- Request ID tracking for audit trails
- No sensitive data in logs
- CORS configured for specific origins only

## Architecture Details

### Connection Pooling
- Maintains persistent HTTP/3 connections
- Automatic health checks and reconnection
- Load balancing across connections
- Connection reuse for efficiency

### Circuit Breaker
- Opens after 5 consecutive failures
- 30-second cooldown period
- Automatic recovery attempts
- Prevents cascade failures

### Retry Logic
- Maximum 3 attempts per request
- Exponential backoff: 100ms, 200ms, 400ms
- Only retries on network errors
- Preserves request ID across retries