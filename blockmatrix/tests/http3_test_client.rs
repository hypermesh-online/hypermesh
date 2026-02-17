// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// BlockMatrix HTTP/3 Test Client
// Comprehensive test client for validating HTTP/3 server endpoints

use anyhow::{Context, Result};
use bytes::{Bytes, Buf};
use h3::client::SendRequest;
use h3_quinn::quinn;
use http::{HeaderMap, Method, Request, Response, StatusCode};
use quinn::{ClientConfig, Endpoint, TransportConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, info, warn, error};

/// Performance metrics for tracking test results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_latency_ms: f64,
    pub min_latency_ms: f64,
    pub max_latency_ms: f64,
    pub latencies: Vec<f64>,
    pub error_counts: HashMap<String, u64>,
    pub status_counts: HashMap<u16, u64>,
}

impl PerformanceMetrics {
    fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            total_latency_ms: 0.0,
            min_latency_ms: f64::MAX,
            max_latency_ms: 0.0,
            latencies: Vec::new(),
            error_counts: HashMap::new(),
            status_counts: HashMap::new(),
        }
    }

    fn record_success(&mut self, latency: Duration, status: StatusCode) {
        let latency_ms = latency.as_secs_f64() * 1000.0;

        self.total_requests += 1;
        self.successful_requests += 1;
        self.total_latency_ms += latency_ms;
        self.latencies.push(latency_ms);

        if latency_ms < self.min_latency_ms {
            self.min_latency_ms = latency_ms;
        }
        if latency_ms > self.max_latency_ms {
            self.max_latency_ms = latency_ms;
        }

        *self.status_counts.entry(status.as_u16()).or_insert(0) += 1;
    }

    fn record_failure(&mut self, error: &str) {
        self.total_requests += 1;
        self.failed_requests += 1;
        *self.error_counts.entry(error.to_string()).or_insert(0) += 1;
    }

    pub fn calculate_percentiles(&self) -> (f64, f64, f64) {
        if self.latencies.is_empty() {
            return (0.0, 0.0, 0.0);
        }

        let mut sorted = self.latencies.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let p50_idx = (sorted.len() as f64 * 0.50) as usize;
        let p95_idx = (sorted.len() as f64 * 0.95) as usize;
        let p99_idx = (sorted.len() as f64 * 0.99) as usize;

        let p50 = sorted[p50_idx.min(sorted.len() - 1)];
        let p95 = sorted[p95_idx.min(sorted.len() - 1)];
        let p99 = sorted[p99_idx.min(sorted.len() - 1)];

        (p50, p95, p99)
    }

    pub fn get_average_latency(&self) -> f64 {
        if self.successful_requests == 0 {
            return 0.0;
        }
        self.total_latency_ms / self.successful_requests as f64
    }

    pub fn get_success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        (self.successful_requests as f64 / self.total_requests as f64) * 100.0
    }
}

/// Test configuration for HTTP/3 client
#[derive(Debug, Clone)]
pub struct TestConfig {
    pub server_addr: SocketAddr,
    pub server_name: String,
    pub timeout: Duration,
    pub max_concurrent_streams: u32,
    pub verify_certificates: bool,
    pub cors_origin: Option<String>,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            server_addr: SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 8446),
            server_name: "localhost".to_string(),
            timeout: Duration::from_secs(30),
            max_concurrent_streams: 100,
            verify_certificates: false,
            cors_origin: Some("http://localhost:5173".to_string()),
        }
    }
}

/// Result of an HTTP/3 test request
#[derive(Debug)]
pub struct TestResult {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: String,
    pub latency: Duration,
    pub request_id: String,
}

/// HTTP/3 Test Client with connection pooling and metrics
pub struct Http3TestClient {
    endpoint: Endpoint,
    config: TestConfig,
    metrics: Arc<Mutex<PerformanceMetrics>>,
    send_request: Arc<Mutex<Option<SendRequest<h3_quinn::OpenStreams, Bytes>>>>,
    request_counter: AtomicU64,
}

impl Http3TestClient {
    /// Create a new HTTP/3 test client
    pub async fn new(config: TestConfig) -> Result<Self> {
        // Create QUIC client configuration
        let client_crypto = if config.verify_certificates {
            create_secure_client_config()?
        } else {
            create_test_client_config()?
        };

        let mut client_config = ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(client_crypto)?
        ));

        // Configure transport
        let mut transport_config = TransportConfig::default();
        transport_config.max_concurrent_bidi_streams(config.max_concurrent_streams.into());
        transport_config.max_concurrent_uni_streams(config.max_concurrent_streams.into());
        transport_config.keep_alive_interval(Some(Duration::from_secs(5)));
        transport_config.max_idle_timeout(Some(config.timeout.try_into()?));

        client_config.transport_config(Arc::new(transport_config));

        // Create client endpoint
        let mut endpoint = Endpoint::client("[::]:0".parse()?)?;
        endpoint.set_default_client_config(client_config);

        Ok(Self {
            endpoint,
            config,
            metrics: Arc::new(Mutex::new(PerformanceMetrics::new())),
            send_request: Arc::new(Mutex::new(None)),
            request_counter: AtomicU64::new(0),
        })
    }

    /// Connect to the HTTP/3 server
    pub async fn connect(&self) -> Result<()> {
        info!("Connecting to HTTP/3 server at {}", self.config.server_addr);

        let conn = self.endpoint
            .connect(self.config.server_addr, &self.config.server_name)?
            .await
            .context("Failed to establish QUIC connection")?;

        info!("QUIC connection established to {}", self.config.server_addr);

        // Create HTTP/3 connection
        let quinn_conn = h3_quinn::Connection::new(conn);
        let (mut driver, send_request) = h3::client::new(quinn_conn).await?;

        // Spawn driver task
        tokio::spawn(async move {
            let result = driver.wait_idle().await;
            debug!("HTTP/3 driver completed: {:?}", result);
        });

        // Store send_request handle (connection is consumed by h3::client::new)
        *self.send_request.lock().await = Some(send_request);

        info!("HTTP/3 client connected successfully");
        Ok(())
    }

    /// Ensure we have an active connection
    async fn ensure_connected(&self) -> Result<()> {
        let has_connection = self.send_request.lock().await.is_some();
        if !has_connection {
            self.connect().await?;
        }
        Ok(())
    }

    /// Execute a GET request
    pub async fn get(&self, path: &str) -> Result<TestResult> {
        self.request(Method::GET, path, None, None).await
    }

    /// Execute a POST request
    pub async fn post(&self, path: &str, body: &[u8]) -> Result<TestResult> {
        self.request(Method::POST, path, Some(body), None).await
    }

    /// Execute an OPTIONS request (for CORS testing)
    pub async fn options(&self, path: &str) -> Result<TestResult> {
        let mut headers = HeaderMap::new();

        if let Some(origin) = &self.config.cors_origin {
            headers.insert("Origin", origin.parse()?);
            headers.insert("Access-Control-Request-Method", "POST".parse()?);
            headers.insert("Access-Control-Request-Headers", "Content-Type, Authorization".parse()?);
        }

        self.request(Method::OPTIONS, path, None, Some(headers)).await
    }

    /// Execute a custom HTTP request
    pub async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<&[u8]>,
        extra_headers: Option<HeaderMap>,
    ) -> Result<TestResult> {
        self.ensure_connected().await?;

        let start = Instant::now();
        let request_id = format!("req-{}", self.request_counter.fetch_add(1, Ordering::SeqCst));

        // Get send_request handle
        let send_request_guard = self.send_request.lock().await;
        let mut send_request = send_request_guard
            .as_ref()
            .context("No active connection")?
            .clone();

        // Build request
        let uri = format!("https://{}:{}{}",
            self.config.server_name,
            self.config.server_addr.port(),
            path
        );

        let mut req_builder = Request::builder()
            .uri(&uri)
            .method(method.clone())
            .header("User-Agent", "BlockMatrix-HTTP3-Test-Client/1.0")
            .header("X-Request-Id", &request_id);

        // Add CORS origin if configured
        if let Some(origin) = &self.config.cors_origin {
            req_builder = req_builder.header("Origin", origin);
        }

        // Add any extra headers
        if let Some(headers) = extra_headers {
            for (key, value) in headers.iter() {
                req_builder = req_builder.header(key, value);
            }
        }

        let req = req_builder.body(())?;

        debug!("Sending {} request to {}", method, path);

        // Send request and get response
        let mut stream = send_request.send_request(req).await
            .context("Failed to send request")?;

        // Send body if present
        if let Some(body_data) = body {
            stream.send_data(Bytes::copy_from_slice(body_data)).await
                .context("Failed to send request body")?;
        }

        stream.finish().await
            .context("Failed to finish request stream")?;

        // Receive response
        let response = stream.recv_response().await
            .context("Failed to receive response")?;

        let status = response.status();
        let headers = response.headers().clone();

        // Read response body
        let mut body_bytes = Vec::new();
        while let Some(mut chunk) = stream.recv_data().await? {
            let len = chunk.remaining();
            body_bytes.extend_from_slice(&chunk.copy_to_bytes(len));
        }

        let body_str = String::from_utf8_lossy(&body_bytes).to_string();
        let latency = start.elapsed();

        // Update metrics
        let mut metrics = self.metrics.lock().await;
        if status.is_success() {
            metrics.record_success(latency, status);
        } else {
            metrics.record_failure(&format!("HTTP {}", status.as_u16()));
        }

        debug!("Request {} completed in {:?} with status {}", request_id, latency, status);

        Ok(TestResult {
            status,
            headers,
            body: body_str,
            latency,
            request_id,
        })
    }

    /// Execute multiple concurrent requests
    pub async fn concurrent_requests(
        &self,
        method: Method,
        path: &str,
        count: usize,
    ) -> Result<Vec<TestResult>> {
        self.ensure_connected().await?;

        let mut handles = Vec::new();

        for _ in 0..count {
            let method_clone = method.clone();
            let path_clone = path.to_string();
            let client = self.clone_for_concurrent();

            let handle = tokio::spawn(async move {
                client.request(method_clone, &path_clone, None, None).await
            });

            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(Ok(result)) => results.push(result),
                Ok(Err(e)) => warn!("Request failed: {}", e),
                Err(e) => error!("Task failed: {}", e),
            }
        }

        Ok(results)
    }

    /// Clone client configuration for concurrent use
    pub fn clone_for_concurrent(&self) -> Self {
        Self {
            endpoint: self.endpoint.clone(),
            config: self.config.clone(),
            metrics: self.metrics.clone(),
            send_request: self.send_request.clone(),
            request_counter: AtomicU64::new(self.request_counter.load(Ordering::SeqCst)),
        }
    }

    /// Get performance metrics
    pub async fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.lock().await.clone()
    }

    /// Reset metrics
    pub async fn reset_metrics(&self) {
        *self.metrics.lock().await = PerformanceMetrics::new();
    }

    /// Generate performance report
    pub async fn generate_report(&self) -> String {
        let metrics = self.get_metrics().await;
        let (p50, p95, p99) = metrics.calculate_percentiles();

        format!(
            r#"
Performance Report
==================
Total Requests: {}
Successful: {} ({:.2}%)
Failed: {}

Latency Statistics:
- Average: {:.2}ms
- Min: {:.2}ms
- Max: {:.2}ms
- P50: {:.2}ms
- P95: {:.2}ms
- P99: {:.2}ms

Status Code Distribution:
{:?}

Error Distribution:
{:?}
"#,
            metrics.total_requests,
            metrics.successful_requests,
            metrics.get_success_rate(),
            metrics.failed_requests,
            metrics.get_average_latency(),
            if metrics.min_latency_ms == f64::MAX { 0.0 } else { metrics.min_latency_ms },
            metrics.max_latency_ms,
            p50, p95, p99,
            metrics.status_counts,
            metrics.error_counts
        )
    }

    /// Disconnect from the server
    pub async fn disconnect(&self) {
        *self.send_request.lock().await = None;
        info!("Disconnected from HTTP/3 server");
    }
}

// Helper function to create test client config (no cert verification)
fn create_test_client_config() -> Result<rustls::ClientConfig> {
    // Install the default crypto provider if not already installed
    let _ = rustls::crypto::CryptoProvider::install_default(
        rustls::crypto::ring::default_provider()
    );

    let mut config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(SkipServerVerification::new())
        .with_no_client_auth();

    config.alpn_protocols = vec![b"h3".to_vec()];
    Ok(config)
}

// Helper function to create secure client config
fn create_secure_client_config() -> Result<rustls::ClientConfig> {
    // For now, we'll use the same test config
    // In production, you would use proper root certificates
    create_test_client_config()
}

// Certificate verification bypass for testing
#[derive(Debug)]
struct SkipServerVerification;

impl SkipServerVerification {
    fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
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
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

// Test assertions module
pub mod assertions {
    use super::*;

    /// Assert that response has expected status code
    pub fn assert_status(result: &TestResult, expected: StatusCode) {
        assert_eq!(
            result.status, expected,
            "Expected status {} but got {} for request {}. Body: {}",
            expected, result.status, result.request_id, result.body
        );
    }

    /// Assert that response contains expected text
    pub fn assert_contains(result: &TestResult, text: &str) {
        assert!(
            result.body.contains(text),
            "Response body for {} does not contain '{}'. Body: {}",
            result.request_id, text, result.body
        );
    }

    /// Assert that response has CORS headers
    pub fn assert_cors_headers(result: &TestResult, origin: &str) {
        let headers = &result.headers;

        assert!(
            headers.contains_key("access-control-allow-origin"),
            "Missing Access-Control-Allow-Origin header"
        );

        let allow_origin = headers.get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        assert!(
            allow_origin == origin || allow_origin == "*",
            "Expected Access-Control-Allow-Origin to be '{}' or '*', got '{}'",
            origin, allow_origin
        );
    }

    /// Assert response time is within limit
    pub fn assert_latency(result: &TestResult, max_ms: u64) {
        let latency_ms = result.latency.as_millis() as u64;
        assert!(
            latency_ms <= max_ms,
            "Request {} took {}ms, exceeded limit of {}ms",
            result.request_id, latency_ms, max_ms
        );
    }

    /// Assert that JSON response is valid
    pub fn assert_valid_json(result: &TestResult) {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&result.body);
        assert!(
            parsed.is_ok(),
            "Response body for {} is not valid JSON: {}",
            result.request_id, result.body
        );
    }
}