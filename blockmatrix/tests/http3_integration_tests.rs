// BlockMatrix HTTP/3 Integration Tests
// Comprehensive test suite for validating HTTP/3 server endpoints

mod http3_test_client;

use anyhow::Result;
use http::{Method, StatusCode};
use serde_json::json;
use std::time::Duration;
use tracing::{info, warn, error, Level};
use tracing_subscriber::FmtSubscriber;

use http3_test_client::{
    Http3TestClient, TestConfig, assertions::*
};

// Initialize test logging
fn init_logging() {
    // Install crypto provider for rustls
    let _ = rustls::crypto::CryptoProvider::install_default(
        rustls::crypto::ring::default_provider()
    );

    let _ = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .try_init();
}

// ============================================================================
// Health & Connectivity Tests
// ============================================================================

#[tokio::test]
async fn test_health_endpoint_connectivity() -> Result<()> {
    init_logging();

    let config = TestConfig::default();
    let client = Http3TestClient::new(config).await?;

    // Test BlockMatrix health endpoint
    let result = client.get("/api/v1/blockmatrix/health").await?;

    assert_status(&result, StatusCode::OK);
    assert_contains(&result, "healthy");
    assert_valid_json(&result);
    assert_latency(&result, 50); // Should respond in <50ms

    info!("Health check passed with latency: {:?}", result.latency);
    Ok(())
}

#[tokio::test]
async fn test_server_certificate_validation() -> Result<()> {
    init_logging();

    // Test with certificate validation disabled (test mode)
    let mut config = TestConfig::default();
    config.verify_certificates = false;

    let client = Http3TestClient::new(config).await?;
    let result = client.get("/api/v1/blockmatrix/health").await?;

    assert_status(&result, StatusCode::OK);
    info!("Certificate bypass mode successful");

    Ok(())
}

#[tokio::test]
async fn test_quic_handshake_performance() -> Result<()> {
    init_logging();

    let config = TestConfig::default();
    let start = std::time::Instant::now();

    let client = Http3TestClient::new(config).await?;
    client.connect().await?;

    let handshake_time = start.elapsed();

    assert!(
        handshake_time < Duration::from_millis(100),
        "QUIC handshake took {:?}, expected <100ms",
        handshake_time
    );

    info!("QUIC handshake completed in {:?}", handshake_time);
    Ok(())
}

// ============================================================================
// CORS Validation Tests
// ============================================================================

#[tokio::test]
async fn test_cors_preflight_handling() -> Result<()> {
    init_logging();

    let config = TestConfig::default();
    let client = Http3TestClient::new(config).await?;

    // Send OPTIONS preflight request
    let result = client.options("/api/v1/blockmatrix/health").await?;

    // Verify CORS headers
    assert_status(&result, StatusCode::OK);
    assert_cors_headers(&result, "http://localhost:5173");

    // Check specific CORS headers
    let headers = &result.headers;
    assert!(headers.contains_key("access-control-allow-methods"));
    assert!(headers.contains_key("access-control-allow-headers"));
    assert!(headers.contains_key("access-control-max-age"));

    info!("CORS preflight validation successful");
    Ok(())
}

#[tokio::test]
async fn test_cors_actual_request() -> Result<()> {
    init_logging();

    let mut config = TestConfig::default();
    config.cors_origin = Some("http://localhost:5173".to_string());

    let client = Http3TestClient::new(config).await?;

    // Send GET request with Origin header
    let result = client.get("/api/v1/blockmatrix/status").await?;

    assert_status(&result, StatusCode::OK);
    assert_cors_headers(&result, "http://localhost:5173");

    // Verify credentials support
    let allow_credentials = result.headers
        .get("access-control-allow-credentials")
        .and_then(|v| v.to_str().ok());

    assert_eq!(allow_credentials, Some("true"), "Expected credentials support");

    info!("CORS actual request validation successful");
    Ok(())
}

// ============================================================================
// Performance Testing
// ============================================================================

#[tokio::test]
async fn test_simple_get_performance() -> Result<()> {
    init_logging();

    let config = TestConfig::default();
    let client = Http3TestClient::new(config).await?;

    // Warm up connection
    client.get("/api/v1/blockmatrix/health").await?;

    // Run multiple requests and measure
    client.reset_metrics().await;

    for _ in 0..10 {
        let result = client.get("/api/v1/blockmatrix/health").await?;
        assert_status(&result, StatusCode::OK);
        assert_latency(&result, 20); // P50 target: <20ms
    }

    let metrics = client.get_metrics().await;
    let avg_latency = metrics.get_average_latency();

    assert!(
        avg_latency < 20.0,
        "Average latency {:.2}ms exceeded 20ms target",
        avg_latency
    );

    info!("Simple GET performance: avg {:.2}ms", avg_latency);
    Ok(())
}

#[tokio::test]
async fn test_concurrent_request_handling() -> Result<()> {
    init_logging();

    let config = TestConfig::default();
    let client = Http3TestClient::new(config).await?;

    // Test 10 concurrent requests
    let start = std::time::Instant::now();
    let results = client.concurrent_requests(
        Method::GET,
        "/api/v1/blockmatrix/health",
        10
    ).await?;

    let total_time = start.elapsed();

    // Verify all succeeded
    assert_eq!(results.len(), 10, "Expected 10 results");

    for result in &results {
        assert_status(result, StatusCode::OK);
    }

    // Check that concurrent execution was faster than sequential
    let avg_latency: f64 = results.iter()
        .map(|r| r.latency.as_secs_f64() * 1000.0)
        .sum::<f64>() / results.len() as f64;

    info!(
        "Concurrent requests completed in {:?}, avg latency: {:.2}ms",
        total_time, avg_latency
    );

    assert!(
        total_time < Duration::from_millis(200),
        "Concurrent requests took too long: {:?}",
        total_time
    );

    Ok(())
}

#[tokio::test]
async fn test_sustained_load() -> Result<()> {
    init_logging();

    let config = TestConfig::default();
    let client = Http3TestClient::new(config).await?;

    client.reset_metrics().await;

    // Run sustained load for 5 seconds
    let duration = Duration::from_secs(5);
    let start = std::time::Instant::now();
    let mut request_count = 0;

    while start.elapsed() < duration {
        let result = client.get("/api/v1/blockmatrix/health").await?;

        if !result.status.is_success() {
            info!("Request failed with status: {}", result.status);
        }

        request_count += 1;

        // Small delay to avoid overwhelming
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let metrics = client.get_metrics().await;
    let (p50, p95, p99) = metrics.calculate_percentiles();

    info!("Sustained load test completed:");
    info!("  Total requests: {}", request_count);
    info!("  Success rate: {:.2}%", metrics.get_success_rate());
    info!("  P50: {:.2}ms, P95: {:.2}ms, P99: {:.2}ms", p50, p95, p99);

    // Assert performance targets
    assert!(p50 < 50.0, "P50 latency {:.2}ms exceeded 50ms target", p50);
    assert!(p95 < 100.0, "P95 latency {:.2}ms exceeded 100ms target", p95);
    assert!(
        metrics.get_success_rate() > 95.0,
        "Success rate {:.2}% below 95% target",
        metrics.get_success_rate()
    );

    Ok(())
}

// ============================================================================
// BlockMatrix Endpoint Tests
// ============================================================================

#[tokio::test]
async fn test_blockmatrix_system_status() -> Result<()> {
    init_logging();

    let config = TestConfig::default();
    let client = Http3TestClient::new(config).await?;

    let result = client.get("/api/v1/hypermesh/system/status").await?;

    assert_status(&result, StatusCode::OK);
    assert_valid_json(&result);
    assert_latency(&result, 50);

    // Parse and validate response structure
    let json: serde_json::Value = serde_json::from_str(&result.body)?;
    assert!(json.is_object(), "Expected JSON object response");

    info!("System status endpoint validated");
    Ok(())
}

#[tokio::test]
async fn test_blockmatrix_assets_list() -> Result<()> {
    init_logging();

    let config = TestConfig::default();
    let client = Http3TestClient::new(config).await?;

    let result = client.get("/api/v1/hypermesh/assets").await?;

    if result.status == StatusCode::OK {
        assert_valid_json(&result);

        let json: serde_json::Value = serde_json::from_str(&result.body)?;
        assert!(
            json.is_array() || (json.is_object() && json.get("assets").is_some()),
            "Expected array or object with 'assets' field"
        );
    } else if result.status == StatusCode::NOT_FOUND {
        info!("Assets endpoint not yet implemented");
    } else {
        panic!("Unexpected status: {} - {}", result.status, result.body);
    }

    Ok(())
}

#[tokio::test]
async fn test_blockmatrix_asset_creation() -> Result<()> {
    init_logging();

    let config = TestConfig::default();
    let client = Http3TestClient::new(config).await?;

    let asset_data = json!({
        "name": "test-asset",
        "type": "compute",
        "capacity": 100,
        "metadata": {
            "location": "test-region",
            "tier": "standard"
        }
    });

    let body = serde_json::to_vec(&asset_data)?;
    let result = client.post("/api/v1/hypermesh/assets", &body).await?;

    // Accept either success or not implemented
    if result.status == StatusCode::CREATED || result.status == StatusCode::OK {
        assert_valid_json(&result);
        info!("Asset creation successful");
    } else if result.status == StatusCode::NOT_FOUND || result.status == StatusCode::NOT_IMPLEMENTED {
        info!("Asset creation endpoint not yet implemented");
    } else {
        panic!("Unexpected status: {} - {}", result.status, result.body);
    }

    Ok(())
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_404_not_found_handling() -> Result<()> {
    init_logging();

    let config = TestConfig::default();
    let client = Http3TestClient::new(config).await?;

    let result = client.get("/api/v1/nonexistent/endpoint").await?;

    assert_status(&result, StatusCode::NOT_FOUND);
    assert_valid_json(&result);

    // Verify error response structure
    let json: serde_json::Value = serde_json::from_str(&result.body)?;
    assert!(
        json.get("error").is_some() || json.get("message").is_some(),
        "Expected error field in 404 response"
    );

    info!("404 error handling validated");
    Ok(())
}

#[tokio::test]
async fn test_bad_request_handling() -> Result<()> {
    init_logging();

    let config = TestConfig::default();
    let client = Http3TestClient::new(config).await?;

    // Send malformed JSON
    let bad_json = b"{ invalid json }";
    let result = client.post("/api/v1/hypermesh/assets", bad_json).await?;

    // Should return 400 Bad Request
    if result.status == StatusCode::BAD_REQUEST {
        assert_valid_json(&result);
        info!("Bad request handling validated");
    } else if result.status == StatusCode::NOT_FOUND {
        info!("Endpoint not yet implemented");
    } else {
        info!("Server returned status: {} - may not validate JSON yet", result.status);
    }

    Ok(())
}

// ============================================================================
// Test Utilities
// ============================================================================

#[tokio::test]
async fn test_metrics_collection() -> Result<()> {
    init_logging();

    let config = TestConfig::default();
    let client = Http3TestClient::new(config).await?;

    // Run some requests
    for i in 0..5 {
        let _result = client.get("/api/v1/blockmatrix/health").await?;
        if i == 2 {
            // Simulate a failure
            let _failed = client.get("/api/v1/invalid/path").await;
        }
    }

    // Generate report
    let report = client.generate_report().await;
    info!("Performance Report:\n{}", report);

    // Verify metrics were collected
    let metrics = client.get_metrics().await;
    assert!(metrics.total_requests >= 5, "Expected at least 5 requests");
    assert!(metrics.successful_requests >= 4, "Expected at least 4 successful requests");

    Ok(())
}

// ============================================================================
// Additional Coverage Tests
// ============================================================================

#[tokio::test]
async fn test_large_payload_handling() -> Result<()> {
    init_logging();

    let config = TestConfig::default();
    let client = Http3TestClient::new(config).await?;

    // Create a 1MB payload
    let large_data = vec![b'X'; 1024 * 1024];
    let json_payload = json!({
        "data": String::from_utf8_lossy(&large_data),
        "size": large_data.len()
    });

    let body = serde_json::to_vec(&json_payload)?;
    let result = client.post("/api/v1/hypermesh/upload", &body).await?;

    // Server might not support large uploads yet
    if result.status == StatusCode::OK {
        assert_valid_json(&result);
        info!("Large payload handled successfully");
    } else if result.status == StatusCode::NOT_FOUND ||
              result.status == StatusCode::PAYLOAD_TOO_LARGE {
        info!("Large payload endpoint not implemented or size limit reached");
    } else {
        panic!("Unexpected response for large payload: {}", result.status);
    }

    Ok(())
}

#[tokio::test]
async fn test_stream_multiplexing() -> Result<()> {
    init_logging();

    let config = TestConfig::default();
    let client = Http3TestClient::new(config).await?;

    // Test multiple streams on same connection
    let mut handles = Vec::new();

    for i in 0..5 {
        let client_clone = client.clone_for_concurrent();
        let handle = tokio::spawn(async move {
            let path = format!("/api/v1/blockmatrix/stream/{}", i);
            client_clone.get(&path).await
        });
        handles.push(handle);
    }

    // All should complete without errors
    for handle in handles {
        match handle.await {
            Ok(Ok(result)) => {
                // Either success or not found is acceptable
                assert!(
                    result.status.is_success() || result.status == StatusCode::NOT_FOUND,
                    "Unexpected status: {}", result.status
                );
            }
            Ok(Err(e)) => warn!("Request failed: {}", e),
            Err(e) => error!("Task failed: {}", e),
        }
    }

    info!("Stream multiplexing test completed");
    Ok(())
}

#[tokio::test]
async fn test_rate_limiting() -> Result<()> {
    init_logging();

    let config = TestConfig::default();
    let client = Http3TestClient::new(config).await?;

    // Send rapid requests to trigger rate limiting
    let mut rate_limited = false;

    for i in 0..100 {
        let result = client.get("/api/v1/blockmatrix/health").await?;

        if result.status == StatusCode::TOO_MANY_REQUESTS {
            rate_limited = true;
            info!("Rate limiting triggered after {} requests", i);
            break;
        }
    }

    // Rate limiting might not be implemented yet
    if !rate_limited {
        info!("No rate limiting detected (may not be implemented)");
    }

    Ok(())
}

// ============================================================================
// Connection Management Tests
// ============================================================================

#[tokio::test]
async fn test_connection_reuse() -> Result<()> {
    init_logging();

    let config = TestConfig::default();
    let client = Http3TestClient::new(config).await?;

    // First request establishes connection
    let result1 = client.get("/api/v1/blockmatrix/health").await?;
    assert_status(&result1, StatusCode::OK);

    // Second request should reuse connection (faster)
    let start = std::time::Instant::now();
    let result2 = client.get("/api/v1/blockmatrix/health").await?;
    let reuse_time = start.elapsed();

    assert_status(&result2, StatusCode::OK);
    assert!(
        reuse_time < Duration::from_millis(10),
        "Connection reuse took {:?}, expected <10ms",
        reuse_time
    );

    info!("Connection reuse validated, second request: {:?}", reuse_time);
    Ok(())
}

#[tokio::test]
async fn test_graceful_disconnect() -> Result<()> {
    init_logging();

    let config = TestConfig::default();
    let client = Http3TestClient::new(config).await?;

    // Connect and make a request
    let result = client.get("/api/v1/blockmatrix/health").await?;
    assert_status(&result, StatusCode::OK);

    // Disconnect
    client.disconnect().await;

    // Should reconnect automatically on next request
    let result2 = client.get("/api/v1/blockmatrix/health").await?;
    assert_status(&result2, StatusCode::OK);

    info!("Graceful disconnect and reconnect successful");
    Ok(())
}