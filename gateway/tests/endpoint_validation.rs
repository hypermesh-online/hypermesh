// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// Gateway Endpoint Validation Test Suite
// Comprehensive tests for all 10 Week 1 Priority endpoints
//
// Tests all endpoints across Gateway, BlockMatrix, and TrustChain servers:
// 1. HTTP/3 connectivity via h3 client
// 2. CORS headers verification
// 3. ApiResponse format validation
// 4. Response time measurements (target <500ms P95)
// 5. Error handling and validation

use anyhow::{Context, Result};
use bytes::{Buf, Bytes};
use h3_quinn::quinn;
use http::{HeaderMap, Method, Request, StatusCode};
use quinn::{ClientConfig, Endpoint, TransportConfig};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

/// API Response wrapper format used by all endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<ErrorInfo>,
    request_id: String,
    timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ErrorInfo {
    code: String,
    message: String,
    details: Option<Value>,
}

/// Test result for a single endpoint
#[derive(Debug, Clone)]
struct EndpointTestResult {
    endpoint: String,
    method: Method,
    status: StatusCode,
    response_time_ms: f64,
    cors_headers_present: bool,
    api_format_valid: bool,
    body: String,
    #[allow(dead_code)]
    headers: HeaderMap,
    success: bool,
    error: Option<String>,
}

/// Performance metrics aggregation
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PerformanceMetrics {
    endpoint: String,
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    response_times: Vec<f64>,
    p50_ms: f64,
    p95_ms: f64,
    p99_ms: f64,
    average_ms: f64,
}

impl PerformanceMetrics {
    fn calculate_percentiles(times: &mut [f64]) -> (f64, f64, f64) {
        if times.is_empty() {
            return (0.0, 0.0, 0.0);
        }

        times.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let p50_idx = (times.len() as f64 * 0.50) as usize;
        let p95_idx = (times.len() as f64 * 0.95) as usize;
        let p99_idx = (times.len() as f64 * 0.99) as usize;

        let p50 = times[p50_idx.min(times.len() - 1)];
        let p95 = times[p95_idx.min(times.len() - 1)];
        let p99 = times[p99_idx.min(times.len() - 1)];

        (p50, p95, p99)
    }
}

/// HTTP/3 test client for endpoint validation
struct EndpointTestClient {
    endpoint: Endpoint,
    gateway_addr: SocketAddr,
    test_results: Vec<EndpointTestResult>,
    start_time: Instant,
}

impl EndpointTestClient {
    async fn new() -> Result<Option<Self>> {
        Self::new_with_port(8446).await
    }

    async fn new_with_port(port: u16) -> Result<Option<Self>> {
        // Initialize rustls crypto provider
        let _ = rustls::crypto::ring::default_provider().install_default();

        // Server address (defaults to BlockMatrix backend at 8446, gateway would be 8443)
        let gateway_addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), port);

        // Create self-signed cert acceptance config
        let mut tls_config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AcceptAllCerts))
            .with_no_client_auth();

        // Use STOQ ALPN protocol to match server configuration
        // STOQ servers advertise "stoq/1.0" not standard "h3"
        tls_config.alpn_protocols = vec![b"stoq/1.0".to_vec()];

        // Configure transport
        let mut transport_config = TransportConfig::default();
        transport_config.max_idle_timeout(Some(Duration::from_secs(30).try_into()?));
        transport_config.keep_alive_interval(Some(Duration::from_secs(10)));

        let mut client_config = ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(tls_config)?,
        ));
        client_config.transport_config(Arc::new(transport_config));

        // Create endpoint
        let mut endpoint = Endpoint::client("[::]:0".parse()?)?;
        endpoint.set_default_client_config(client_config);

        // Probe whether the server is actually reachable before committing
        let connecting = match endpoint.connect(gateway_addr, "localhost") {
            Ok(c) => c,
            Err(_) => {
                eprintln!(
                    "Server at {gateway_addr} not reachable, skipping endpoint validation tests"
                );
                return Ok(None);
            }
        };

        match tokio::time::timeout(Duration::from_secs(3), connecting).await {
            Ok(Ok(conn)) => {
                conn.close(0u32.into(), b"probe");
            }
            _ => {
                eprintln!(
                    "Server at {gateway_addr} not accepting connections, skipping endpoint validation tests"
                );
                return Ok(None);
            }
        }

        Ok(Some(Self {
            endpoint,
            gateway_addr,
            test_results: Vec::new(),
            start_time: Instant::now(),
        }))
    }

    async fn test_endpoint(
        &mut self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<EndpointTestResult> {
        let start = Instant::now();

        // Connect to gateway
        let connection = self
            .endpoint
            .connect(self.gateway_addr, "localhost")?
            .await
            .context("Failed to establish QUIC connection")?;

        // Establish HTTP/3 session
        let quinn_conn = h3_quinn::Connection::new(connection);
        let (mut driver, mut send_request) = h3::client::new(quinn_conn).await?;

        // Spawn driver
        tokio::spawn(async move {
            let _ = driver.wait_idle().await;
        });

        // Build request
        let request_builder = Request::builder()
            .method(method.clone())
            .uri(path)
            .header("host", "localhost")
            .header("content-type", "application/json")
            .header("origin", "http://localhost:5173");

        // Add body if provided
        let request = if let Some(ref _json_body) = body {
            request_builder.body(())?
        } else {
            request_builder.body(())?
        };

        // Send request
        let mut stream = send_request
            .send_request(request)
            .await
            .context("Failed to send HTTP/3 request")?;

        // Send body if present
        if let Some(json_body) = body {
            let body_bytes = serde_json::to_vec(&json_body)?;
            stream.send_data(Bytes::from(body_bytes)).await?;
        }
        stream.finish().await?;

        // Receive response
        let response = stream.recv_response().await?;
        let status = response.status();
        let headers = response.headers().clone();

        // Read response body
        let mut body_bytes = Vec::new();
        while let Some(mut chunk) = stream.recv_data().await? {
            body_bytes.extend_from_slice(&chunk.copy_to_bytes(chunk.remaining()));
        }

        let body_str = String::from_utf8_lossy(&body_bytes).to_string();
        let elapsed = start.elapsed();
        let response_time_ms = elapsed.as_secs_f64() * 1000.0;

        // Check CORS headers
        let cors_headers_present = headers.contains_key("access-control-allow-origin")
            && headers.contains_key("access-control-allow-methods")
            && headers.contains_key("access-control-allow-headers");

        // Validate API response format
        let api_format_valid = if status.is_success() {
            if let Ok(json) = serde_json::from_str::<Value>(&body_str) {
                json.get("success").is_some()
                    && json.get("request_id").is_some()
                    && json.get("timestamp").is_some()
            } else {
                false
            }
        } else {
            true // Error responses may have different format
        };

        // Determine overall success
        let success = status.is_success()
            && cors_headers_present
            && api_format_valid
            && response_time_ms < 500.0;

        let result = EndpointTestResult {
            endpoint: path.to_string(),
            method,
            status,
            response_time_ms,
            cors_headers_present,
            api_format_valid,
            body: body_str,
            headers,
            success,
            error: if !success {
                Some(format!(
                    "Status: {status}, CORS: {cors_headers_present}, Format: {api_format_valid}, Time: {response_time_ms:.2}ms"
                ))
            } else {
                None
            },
        };

        self.test_results.push(result.clone());
        Ok(result)
    }

    async fn test_all_endpoints(&mut self) -> Result<Vec<EndpointTestResult>> {
        let mut results = Vec::new();

        info!("Testing 10 Week 1 Priority Endpoints via Gateway");
        info!("=================================================");

        // 1. HyperMesh System Status
        info!("\n1. Testing GET /api/v1/hypermesh/system/status");
        match self
            .test_endpoint(Method::GET, "/api/v1/hypermesh/system/status", None)
            .await
        {
            Ok(result) => {
                Self::print_result(&result);
                results.push(result);
            }
            Err(e) => error!("   FAILED: {}", e),
        }

        // 2. STOQ System Health
        info!("\n2. Testing GET /api/v1/stoq/system/health");
        match self
            .test_endpoint(Method::GET, "/api/v1/stoq/system/health", None)
            .await
        {
            Ok(result) => {
                Self::print_result(&result);
                results.push(result);
            }
            Err(e) => error!("   FAILED: {}", e),
        }

        // 3. Byzantine Fault Detections
        info!("\n3. Testing GET /api/v1/hypermesh/byzantine/detections");
        match self
            .test_endpoint(Method::GET, "/api/v1/hypermesh/byzantine/detections", None)
            .await
        {
            Ok(result) => {
                Self::print_result(&result);
                results.push(result);
            }
            Err(e) => error!("   FAILED: {}", e),
        }

        // 4. Asset Listing
        info!("\n4. Testing GET /api/v1/hypermesh/assets");
        match self
            .test_endpoint(Method::GET, "/api/v1/hypermesh/assets", None)
            .await
        {
            Ok(result) => {
                Self::print_result(&result);
                results.push(result);
            }
            Err(e) => error!("   FAILED: {}", e),
        }

        // 5. Resource Allocations
        info!("\n5. Testing GET /api/v1/hypermesh/allocations");
        match self
            .test_endpoint(Method::GET, "/api/v1/hypermesh/allocations", None)
            .await
        {
            Ok(result) => {
                Self::print_result(&result);
                results.push(result);
            }
            Err(e) => error!("   FAILED: {}", e),
        }

        // 6. STOQ Connections
        info!("\n6. Testing GET /api/v1/stoq/connections");
        match self
            .test_endpoint(Method::GET, "/api/v1/stoq/connections", None)
            .await
        {
            Ok(result) => {
                Self::print_result(&result);
                results.push(result);
            }
            Err(e) => error!("   FAILED: {}", e),
        }

        // 7. Node Health
        info!("\n7. Testing GET /api/v1/hypermesh/nodes/health");
        match self
            .test_endpoint(Method::GET, "/api/v1/hypermesh/nodes/health", None)
            .await
        {
            Ok(result) => {
                Self::print_result(&result);
                results.push(result);
            }
            Err(e) => error!("   FAILED: {}", e),
        }

        // 8. Performance Metrics
        info!("\n8. Testing GET /api/v1/stoq/metrics/performance");
        match self
            .test_endpoint(Method::GET, "/api/v1/stoq/metrics/performance", None)
            .await
        {
            Ok(result) => {
                Self::print_result(&result);
                results.push(result);
            }
            Err(e) => error!("   FAILED: {}", e),
        }

        // 9. Specific Connection Details
        info!("\n9. Testing GET /api/v1/stoq/connections/test-123");
        match self
            .test_endpoint(Method::GET, "/api/v1/stoq/connections/test-123", None)
            .await
        {
            Ok(result) => {
                Self::print_result(&result);
                results.push(result);
            }
            Err(e) => error!("   FAILED: {}", e),
        }

        // 10. TrustChain Certificate Authentication
        info!("\n10. Testing POST /api/v1/trustchain/auth/certificate");
        let cert_request = json!({
            "certificate_pem": "-----BEGIN CERTIFICATE-----\nMIIBkTCB+wIJAKHHIG...\n-----END CERTIFICATE-----",
            "purpose": "authentication",
            "timestamp": SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()
        });

        match self
            .test_endpoint(
                Method::POST,
                "/api/v1/trustchain/auth/certificate",
                Some(cert_request),
            )
            .await
        {
            Ok(result) => {
                Self::print_result(&result);
                results.push(result);
            }
            Err(e) => error!("   FAILED: {}", e),
        }

        Ok(results)
    }

    async fn test_error_handling(&mut self) -> Result<Vec<EndpointTestResult>> {
        let mut results = Vec::new();

        info!("\nTesting Error Handling");
        info!("=======================");

        // Test 404 - Invalid path
        info!("\nTesting 404 - Invalid Path");
        match self
            .test_endpoint(Method::GET, "/api/v1/invalid/endpoint", None)
            .await
        {
            Ok(result) => {
                if result.status == StatusCode::NOT_FOUND {
                    info!("   ✅ 404 handling works correctly");
                } else {
                    warn!("   ⚠️ Expected 404, got {}", result.status);
                }
                results.push(result);
            }
            Err(e) => error!("   FAILED: {}", e),
        }

        // Test 400 - Malformed request
        info!("\nTesting 400 - Malformed Request");
        let malformed = json!({
            "invalid_field": "test",
            "missing_required": true
        });

        match self
            .test_endpoint(
                Method::POST,
                "/api/v1/trustchain/auth/certificate",
                Some(malformed),
            )
            .await
        {
            Ok(result) => {
                if result.status == StatusCode::BAD_REQUEST {
                    info!("   ✅ 400 handling works correctly");
                } else {
                    warn!("   ⚠️ Expected 400, got {}", result.status);
                }
                results.push(result);
            }
            Err(e) => error!("   FAILED: {}", e),
        }

        // Test OPTIONS - CORS preflight
        info!("\nTesting OPTIONS - CORS Preflight");
        match self
            .test_endpoint(Method::OPTIONS, "/api/v1/hypermesh/system/status", None)
            .await
        {
            Ok(result) => {
                if result.cors_headers_present {
                    info!("   ✅ CORS preflight works correctly");
                } else {
                    warn!("   ⚠️ CORS headers missing in preflight");
                }
                results.push(result);
            }
            Err(e) => error!("   FAILED: {}", e),
        }

        Ok(results)
    }

    async fn run_performance_tests(
        &mut self,
        iterations: usize,
    ) -> Result<Vec<PerformanceMetrics>> {
        let mut metrics_map: HashMap<String, Vec<f64>> = HashMap::new();

        info!(
            "\nRunning Performance Tests ({} iterations per endpoint)",
            iterations
        );
        info!("=======================================================");

        let endpoints = vec![
            (Method::GET, "/api/v1/hypermesh/system/status"),
            (Method::GET, "/api/v1/stoq/system/health"),
            (Method::GET, "/api/v1/hypermesh/byzantine/detections"),
            (Method::GET, "/api/v1/hypermesh/assets"),
            (Method::GET, "/api/v1/hypermesh/allocations"),
        ];

        for (method, path) in &endpoints {
            info!("\nTesting {} {} ({} times)", method, path, iterations);
            let mut times = Vec::new();

            for i in 0..iterations {
                match self.test_endpoint(method.clone(), path, None).await {
                    Ok(result) => {
                        times.push(result.response_time_ms);
                        if i == 0 {
                            info!("   First request: {:.2}ms", result.response_time_ms);
                        }
                    }
                    Err(e) => {
                        error!("   Request {} failed: {}", i + 1, e);
                    }
                }

                // Small delay between requests
                tokio::time::sleep(Duration::from_millis(10)).await;
            }

            metrics_map.insert(path.to_string(), times);
        }

        // Calculate metrics
        let mut all_metrics = Vec::new();

        for (endpoint, mut times) in metrics_map {
            if times.is_empty() {
                continue;
            }

            let (p50, p95, p99) = PerformanceMetrics::calculate_percentiles(&mut times);
            let average = times.iter().sum::<f64>() / times.len() as f64;

            let metrics = PerformanceMetrics {
                endpoint: endpoint.clone(),
                total_requests: times.len() as u64,
                successful_requests: times.len() as u64,
                failed_requests: 0,
                response_times: times,
                p50_ms: p50,
                p95_ms: p95,
                p99_ms: p99,
                average_ms: average,
            };

            info!("\n{} Performance:", endpoint);
            info!("   Average: {:.2}ms", average);
            info!("   P50: {:.2}ms, P95: {:.2}ms, P99: {:.2}ms", p50, p95, p99);

            if p95 < 500.0 {
                info!("   ✅ P95 < 500ms target");
            } else {
                warn!("   ⚠️ P95 exceeds 500ms target");
            }

            all_metrics.push(metrics);
        }

        Ok(all_metrics)
    }

    fn print_result(result: &EndpointTestResult) {
        if result.success {
            info!("   ✅ PASSED");
        } else {
            warn!("   ❌ FAILED");
            if let Some(error) = &result.error {
                warn!("      Error: {}", error);
            }
        }

        info!("   Status: {}", result.status);
        info!("   Response Time: {:.2}ms", result.response_time_ms);
        info!(
            "   CORS Headers: {}",
            if result.cors_headers_present {
                "✓"
            } else {
                "✗"
            }
        );
        info!(
            "   API Format: {}",
            if result.api_format_valid {
                "✓"
            } else {
                "✗"
            }
        );

        // Show body preview
        if !result.body.is_empty() {
            let preview = if result.body.len() > 200 {
                format!("{}...", &result.body[..200])
            } else {
                result.body.clone()
            };
            debug!("   Body Preview: {}", preview);
        }
    }

    fn generate_report(&self) -> String {
        let total_tests = self.test_results.len();
        let passed = self.test_results.iter().filter(|r| r.success).count();
        let failed = total_tests - passed;
        let success_rate = if total_tests > 0 {
            (passed as f64 / total_tests as f64) * 100.0
        } else {
            0.0
        };

        let mut report = String::new();
        report.push_str(&format!("\n{}\n", "=".repeat(60)));
        report.push_str("ENDPOINT VALIDATION TEST REPORT\n");
        report.push_str(&format!("{}\n", "=".repeat(60)));
        report.push_str(&format!(
            "Test Duration: {:.2}s\n",
            self.start_time.elapsed().as_secs_f64()
        ));
        report.push_str(&format!("Total Tests: {total_tests}\n"));
        report.push_str(&format!("Passed: {passed} ({success_rate:.1}%)\n"));
        report.push_str(&format!("Failed: {failed}\n"));

        report.push_str("\nEndpoint Results:\n");
        report.push_str("-----------------\n");

        for result in &self.test_results {
            let status_icon = if result.success { "✅" } else { "❌" };
            report.push_str(&format!(
                "{} {} {} - {} ({:.2}ms)\n",
                status_icon, result.method, result.endpoint, result.status, result.response_time_ms
            ));

            if !result.success {
                if let Some(error) = &result.error {
                    report.push_str(&format!("     Error: {error}\n"));
                }
            }
        }

        // Performance summary
        if !self.test_results.is_empty() {
            let mut response_times: Vec<f64> = self
                .test_results
                .iter()
                .map(|r| r.response_time_ms)
                .collect();

            let (p50, p95, p99) = PerformanceMetrics::calculate_percentiles(&mut response_times);
            let average = response_times.iter().sum::<f64>() / response_times.len() as f64;

            report.push_str("\nPerformance Summary:\n");
            report.push_str("--------------------\n");
            report.push_str(&format!("Average Response Time: {average:.2}ms\n"));
            report.push_str(&format!("P50: {p50:.2}ms\n"));
            report.push_str(&format!("P95: {p95:.2}ms\n"));
            report.push_str(&format!("P99: {p99:.2}ms\n"));

            if p95 < 500.0 {
                report.push_str("✅ P95 meets <500ms target\n");
            } else {
                report.push_str("⚠️ P95 exceeds 500ms target\n");
            }
        }

        // CORS compliance
        let cors_compliant = self
            .test_results
            .iter()
            .filter(|r| r.cors_headers_present)
            .count();

        report.push_str("\nCORS Compliance:\n");
        report.push_str("----------------\n");
        report.push_str(&format!(
            "{cors_compliant}/{total_tests} endpoints have proper CORS headers\n"
        ));

        // API format compliance
        let format_compliant = self
            .test_results
            .iter()
            .filter(|r| r.api_format_valid)
            .count();

        report.push_str("\nAPI Format Compliance:\n");
        report.push_str("----------------------\n");
        report.push_str(&format!(
            "{format_compliant}/{total_tests} responses match ApiResponse format\n"
        ));

        report.push_str(&format!("\n{}\n", "=".repeat(60)));

        report
    }
}

/// Certificate verifier that accepts all certificates (for testing)
#[derive(Debug)]
struct AcceptAllCerts;

impl rustls::client::danger::ServerCertVerifier for AcceptAllCerts {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
        ]
    }
}

#[tokio::test]
async fn test_all_week1_endpoints() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_test_writer()
        .finish();

    let _ = tracing::subscriber::set_global_default(subscriber);

    info!("Starting Endpoint Validation Tests");
    info!("==================================");
    info!("Testing 10 Week 1 Priority Endpoints");
    info!("NOTE: Testing directly against BlockMatrix backend at [::1]:8446");

    // Create test client (gracefully skip if server not running)
    let mut client = match EndpointTestClient::new().await? {
        Some(c) => c,
        None => {
            info!("Server not running, skipping endpoint validation tests");
            return Ok(());
        }
    };

    // Run endpoint tests
    let _endpoint_results = client.test_all_endpoints().await?;

    // Run error handling tests
    let _error_results = client.test_error_handling().await?;

    // Run performance tests (5 iterations per endpoint)
    let _performance_metrics = client.run_performance_tests(5).await?;

    // Generate final report
    let report = client.generate_report();
    println!("{report}");

    // Determine overall success
    let all_passed = client
        .test_results
        .iter()
        .all(|r| r.success || !r.endpoint.contains("/api/v1/"));

    if all_passed {
        info!("✅ ALL TESTS PASSED");
        Ok(())
    } else {
        let failed_count = client
            .test_results
            .iter()
            .filter(|r| !r.success && r.endpoint.contains("/api/v1/"))
            .count();
        panic!("❌ {failed_count} TESTS FAILED - See report above");
    }
}

#[tokio::test]
async fn test_individual_endpoints() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_test_writer()
        .finish();

    let _ = tracing::subscriber::set_global_default(subscriber);

    let mut client = match EndpointTestClient::new().await? {
        Some(c) => c,
        None => return Ok(()),
    };

    // Test each endpoint individually for detailed debugging
    let result = client
        .test_endpoint(Method::GET, "/api/v1/hypermesh/system/status", None)
        .await?;

    assert!(
        result.status.is_success(),
        "Status endpoint should return success"
    );
    assert!(
        result.cors_headers_present,
        "CORS headers should be present"
    );
    assert!(result.api_format_valid, "Response should match API format");
    assert!(
        result.response_time_ms < 500.0,
        "Response time should be < 500ms"
    );

    Ok(())
}

#[tokio::test]
async fn test_performance_targets() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_test_writer()
        .finish();

    let _ = tracing::subscriber::set_global_default(subscriber);

    let mut client = match EndpointTestClient::new().await? {
        Some(c) => c,
        None => return Ok(()),
    };

    // Run performance test with 20 iterations
    let metrics = client.run_performance_tests(20).await?;

    // Check P95 < 500ms for all endpoints
    for metric in metrics {
        assert!(
            metric.p95_ms < 500.0,
            "Endpoint {} P95 ({:.2}ms) should be < 500ms",
            metric.endpoint,
            metric.p95_ms
        );
    }

    Ok(())
}

// Main function for running as standalone binary
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    info!("Gateway Endpoint Validation Test Suite");
    info!("======================================");
    info!("This will test all 10 Week 1 Priority endpoints");
    info!("");
    info!("Prerequisites:");
    info!("1. Start gateway: cargo run --bin gateway");
    info!("2. Start BlockMatrix: cargo run --bin blockmatrix-http3-server");
    info!("3. Start TrustChain: cargo run --bin trustchain-http3-server");
    info!("");

    // Wait for user confirmation
    info!("Press Enter when all servers are running...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    // Create test client
    let mut client = match EndpointTestClient::new().await? {
        Some(c) => c,
        None => {
            info!("Server not running. Start servers first.");
            return Ok(());
        }
    };

    // Run all tests
    info!("\n1. Testing all endpoints...");
    let _ = client.test_all_endpoints().await?;

    info!("\n2. Testing error handling...");
    let _ = client.test_error_handling().await?;

    info!("\n3. Running performance tests (10 iterations)...");
    let _ = client.run_performance_tests(10).await?;

    // Generate and display final report
    let report = client.generate_report();
    println!("{report}");

    // Save report to file
    let report_path = "endpoint_validation_report.txt";
    std::fs::write(report_path, &report)?;
    info!("\nReport saved to: {}", report_path);

    Ok(())
}
