# HTTP/3 Test Client Documentation

## Overview

The HTTP/3 test client provides comprehensive testing capabilities for validating BlockMatrix and TrustChain HTTP/3 server endpoints. This framework supports all test categories defined in Sprint 5.1, including health checks, CORS validation, performance testing, and load testing.

## Architecture

### Core Components

1. **Http3TestClient**: Main client class with connection pooling and metrics
2. **PerformanceMetrics**: Tracks latency, success rates, and error distribution
3. **TestConfig**: Configuration for server endpoints and test parameters
4. **Assertions Module**: Helper functions for validating responses

### Test Categories

- **Health & Connectivity**: Basic server connectivity and health monitoring
- **CORS Validation**: Cross-Origin Resource Sharing compliance
- **Performance Testing**: Latency and throughput validation
- **Error Handling**: Proper error response validation
- **Concurrent Requests**: Multi-stream handling validation
- **Load Testing**: Sustained performance under load

## Usage

### Running Tests

```bash
# Run all HTTP/3 integration tests
cargo test --test http3_integration_tests

# Run specific test category
cargo test --test http3_integration_tests test_health

# Run with verbose output
cargo test --test http3_integration_tests -- --nocapture

# Run performance tests only
cargo test --test http3_integration_tests test_performance
```

### Starting Test Servers

Before running tests, ensure the HTTP/3 servers are running:

```bash
# Start BlockMatrix server (port 8446)
cargo run --bin blockmatrix-http3-server

# Start TrustChain server (port 9293)
cargo run --bin trustchain-http3-server
```

## Test Client API

### Basic Usage

```rust
use http3_test_client::{Http3TestClient, TestConfig};

// Create client with default config
let config = TestConfig::default();
let client = Http3TestClient::new(config).await?;

// Send GET request
let result = client.get("/api/v1/blockmatrix/health").await?;

// Validate response
assert_eq!(result.status, StatusCode::OK);
assert!(result.body.contains("healthy"));
```

### Custom Configuration

```rust
let mut config = TestConfig {
    server_addr: SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 8446),
    server_name: "localhost".to_string(),
    timeout: Duration::from_secs(30),
    max_concurrent_streams: 100,
    verify_certificates: false,
    cors_origin: Some("http://localhost:5173".to_string()),
};
```

### Performance Testing

```rust
// Run multiple requests and collect metrics
for _ in 0..100 {
    let result = client.get("/api/v1/blockmatrix/health").await?;
}

// Generate performance report
let report = client.generate_report().await;
println!("{}", report);

// Get specific metrics
let metrics = client.get_metrics().await;
let (p50, p95, p99) = metrics.calculate_percentiles();
println!("P50: {}ms, P95: {}ms, P99: {}ms", p50, p95, p99);
```

### Concurrent Testing

```rust
// Send 10 concurrent requests
let results = client.concurrent_requests(
    Method::GET,
    "/api/v1/blockmatrix/health",
    10
).await?;

// Validate all succeeded
for result in &results {
    assert_eq!(result.status, StatusCode::OK);
}
```

### CORS Testing

```rust
// Send OPTIONS preflight request
let result = client.options("/api/v1/blockmatrix/health").await?;

// Validate CORS headers
assert_cors_headers(&result, "http://localhost:5173");
```

## Performance Targets

Based on Sprint 5.1 requirements:

- **P50 latency**: <20ms for simple GET requests
- **P95 latency**: <50ms for critical paths
- **P99 latency**: <100ms for complex operations
- **Success rate**: >95% under normal load
- **Throughput**: 1000+ requests/second sustained

## Test Coverage

### BlockMatrix Endpoints (10 endpoints)

- `/api/v1/hypermesh/system/status` - System status and metrics
- `/api/v1/hypermesh/assets` - Asset management
- `/api/v1/hypermesh/allocations` - Resource allocations
- `/api/v1/hypermesh/node/health` - Node health checks
- `/api/v1/hypermesh/byzantine/detections` - Byzantine fault detection
- `/api/v1/hypermesh/remote-proxies` - Proxy management
- `/api/v1/hypermesh/consensus/validate` - Consensus validation

### TrustChain Endpoints (8 endpoints)

- `/api/v1/trustchain/health` - Service health
- `/api/v1/trustchain/certificates` - Certificate management
- `/api/v1/trustchain/auth/certificate` - Certificate authentication
- `/api/v1/trustchain/trust/hierarchy` - Trust hierarchy
- `/api/v1/trustchain/dns/resolve` - DNS resolution
- `/api/v1/trustchain/stats` - Dashboard statistics

## Assertions Library

The test client includes comprehensive assertion helpers:

```rust
use http3_test_client::assertions::*;

// Status code validation
assert_status(&result, StatusCode::OK);

// Content validation
assert_contains(&result, "expected text");

// JSON validation
assert_valid_json(&result);

// CORS header validation
assert_cors_headers(&result, "http://localhost:5173");

// Performance validation
assert_latency(&result, 50); // Max 50ms
```

## Metrics and Reporting

### Performance Metrics

The client tracks:
- Total requests sent
- Successful vs failed requests
- Latency statistics (min, max, average, percentiles)
- Status code distribution
- Error type distribution

### Report Generation

```rust
let report = client.generate_report().await;
```

Sample output:
```
Performance Report
==================
Total Requests: 1000
Successful: 995 (99.50%)
Failed: 5

Latency Statistics:
- Average: 15.32ms
- Min: 8.45ms
- Max: 87.23ms
- P50: 14.21ms
- P95: 24.56ms
- P99: 45.78ms
```

## Troubleshooting

### Common Issues

1. **Connection Refused**
   - Ensure HTTP/3 server is running on the correct port
   - Check IPv6 connectivity is enabled

2. **Certificate Errors**
   - Set `verify_certificates: false` for test environments
   - Use proper certificates in production

3. **Timeout Errors**
   - Increase timeout in TestConfig
   - Check network connectivity
   - Verify server is responding

4. **CORS Failures**
   - Ensure server has CORS middleware configured
   - Check origin header matches expected value

## CI/CD Integration

### GitHub Actions Example

```yaml
http3-tests:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v2

    - name: Start servers
      run: |
        cargo build --release
        ./target/release/blockmatrix-http3-server &
        ./target/release/trustchain-http3-server &
        sleep 5

    - name: Run tests
      run: cargo test --test http3_integration_tests

    - name: Generate report
      if: always()
      run: cargo test --test http3_integration_tests test_metrics_collection
```

## Extending the Test Suite

### Adding New Tests

1. Create test function in `http3_integration_tests.rs`
2. Use Http3TestClient for requests
3. Apply assertions to validate responses
4. Document expected behavior

### Custom Assertions

```rust
pub fn assert_custom_header(result: &TestResult, header: &str, value: &str) {
    let actual = result.headers
        .get(header)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    assert_eq!(actual, value,
        "Expected header {} to be '{}', got '{}'",
        header, value, actual
    );
}
```

## Next Steps

1. **Implement remaining endpoint tests** for full coverage
2. **Add load testing scenarios** for production validation
3. **Integrate with CI/CD pipeline** for automated testing
4. **Create performance benchmarks** for regression detection
5. **Add security testing** for vulnerability scanning

## References

- [Sprint 5.1 Test Specification](./docs/sprint-5.1-http3-test-suite-specification.md)
- [HTTP/3 RFC](https://www.rfc-editor.org/rfc/rfc9114.html)
- [QUIC Protocol](https://www.rfc-editor.org/rfc/rfc9000.html)
- [h3 Crate Documentation](https://docs.rs/h3/)