// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// Example usage of HTTP/3 Test Client
// This example demonstrates how to use the test client for basic testing

use std::path::Path;
use std::env;

// Add the tests directory to the module path
#[path = "../tests/http3_test_client.rs"]
mod http3_test_client;

use anyhow::Result;
use http::{Method, StatusCode};
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::time::Duration;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use http3_test_client::{Http3TestClient, TestConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    info!("HTTP/3 Test Client Example");
    info!("===========================");

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let server_port: u16 = if args.len() > 1 {
        args[1].parse().unwrap_or(8446)
    } else {
        8446
    };

    // Configure test client
    let config = TestConfig {
        server_addr: SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), server_port),
        server_name: "localhost".to_string(),
        timeout: Duration::from_secs(10),
        max_concurrent_streams: 50,
        verify_certificates: false,
        cors_origin: Some("http://localhost:5173".to_string()),
    };

    info!("Connecting to server at {}", config.server_addr);

    // Create client
    let client = Http3TestClient::new(config).await?;

    // Try to connect
    match client.connect().await {
        Ok(_) => info!("✓ Connected successfully"),
        Err(e) => {
            info!("✗ Connection failed: {}", e);
            info!("Make sure the HTTP/3 server is running on port {}", server_port);
            info!("Start it with: cargo run --bin blockmatrix-http3-server");
            return Ok(());
        }
    }

    info!("\nRunning test scenarios...\n");

    // Test 1: Health endpoint
    info!("Test 1: Health Endpoint");
    match client.get("/api/v1/blockmatrix/health").await {
        Ok(result) => {
            info!("  Status: {}", result.status);
            info!("  Latency: {:?}", result.latency);
            info!("  Body preview: {}", &result.body.chars().take(100).collect::<String>());

            if result.status == StatusCode::OK {
                info!("  ✓ Health check passed");
            } else {
                info!("  ✗ Unexpected status code");
            }
        }
        Err(e) => {
            info!("  ✗ Request failed: {}", e);
        }
    }

    // Test 2: CORS preflight
    info!("\nTest 2: CORS Preflight");
    match client.options("/api/v1/blockmatrix/health").await {
        Ok(result) => {
            info!("  Status: {}", result.status);

            let has_cors = result.headers.contains_key("access-control-allow-origin");
            if has_cors {
                let origin = result.headers
                    .get("access-control-allow-origin")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("none");
                info!("  CORS Origin: {}", origin);
                info!("  ✓ CORS headers present");
            } else {
                info!("  ✗ CORS headers missing");
            }
        }
        Err(e) => {
            info!("  ✗ Request failed: {}", e);
        }
    }

    // Test 3: Performance test
    info!("\nTest 3: Performance Test (10 requests)");
    client.reset_metrics().await;

    for i in 0..10 {
        let result = client.get("/api/v1/blockmatrix/health").await?;
        if i == 0 {
            info!("  First request: {:?}", result.latency);
        }
    }

    let metrics = client.get_metrics().await;
    let (p50, p95, p99) = metrics.calculate_percentiles();

    info!("  Results:");
    info!("    Total requests: {}", metrics.total_requests);
    info!("    Success rate: {:.1}%", metrics.get_success_rate());
    info!("    Average latency: {:.2}ms", metrics.get_average_latency());
    info!("    P50: {:.2}ms, P95: {:.2}ms, P99: {:.2}ms", p50, p95, p99);

    // Test 4: Concurrent requests
    info!("\nTest 4: Concurrent Requests (5 parallel)");
    let start = std::time::Instant::now();

    let results = client.concurrent_requests(
        Method::GET,
        "/api/v1/blockmatrix/health",
        5
    ).await?;

    let total_time = start.elapsed();
    let success_count = results.iter().filter(|r| r.status.is_success()).count();

    info!("  Completed {} requests in {:?}", results.len(), total_time);
    info!("  Successful: {}/{}", success_count, results.len());

    if success_count == results.len() {
        info!("  ✓ All concurrent requests succeeded");
    } else {
        info!("  ✗ Some requests failed");
    }

    // Test 5: Error handling
    info!("\nTest 5: Error Handling (404)");
    match client.get("/api/v1/nonexistent/endpoint").await {
        Ok(result) => {
            info!("  Status: {}", result.status);
            if result.status == StatusCode::NOT_FOUND {
                info!("  ✓ Correctly returned 404");
            } else {
                info!("  ✗ Unexpected status code");
            }
        }
        Err(e) => {
            info!("  ✗ Request failed: {}", e);
        }
    }

    // Generate final report
    info!("\n{}", "=".repeat(50));
    info!("Final Performance Report");
    info!("{}", "=".repeat(50));

    let report = client.generate_report().await;
    for line in report.lines() {
        if !line.is_empty() {
            info!("{}", line);
        }
    }

    // Disconnect
    client.disconnect().await;
    info!("\n✓ Test client disconnected");

    Ok(())
}