// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use bytes::Bytes;
use http::{HeaderMap, HeaderValue, Method, Request, Response, StatusCode};
use std::time::Instant;
use tracing::{debug, info};
use uuid::Uuid;

use crate::config::CorsConfig;

/// Middleware for adding CORS headers to responses
pub struct CorsMiddleware {
    config: CorsConfig,
}

impl CorsMiddleware {
    pub fn new(config: CorsConfig) -> Self {
        Self { config }
    }

    /// Apply CORS headers to a response
    pub fn apply_cors(&self, response: &mut Response<Bytes>) {
        let headers = response.headers_mut();

        // Set allowed origin
        if !self.config.allowed_origins.is_empty() {
            // For simplicity, we're using the first origin. In production, check against request origin
            headers.insert(
                "access-control-allow-origin",
                HeaderValue::from_str(&self.config.allowed_origins[0]).unwrap(),
            );
        }

        // Set allowed methods
        let methods = self.config.allowed_methods.join(", ");
        headers.insert(
            "access-control-allow-methods",
            HeaderValue::from_str(&methods).unwrap(),
        );

        // Set allowed headers
        let allowed_headers = self.config.allowed_headers.join(", ");
        headers.insert(
            "access-control-allow-headers",
            HeaderValue::from_str(&allowed_headers).unwrap(),
        );

        // Set credentials flag
        if self.config.allow_credentials {
            headers.insert(
                "access-control-allow-credentials",
                HeaderValue::from_static("true"),
            );
        }

        // Set max age
        headers.insert(
            "access-control-max-age",
            HeaderValue::from_str(&self.config.max_age.to_string()).unwrap(),
        );
    }

    /// Handle preflight OPTIONS request
    pub fn handle_preflight(&self) -> Response<Bytes> {
        let mut response = Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(Bytes::new())
            .unwrap();

        self.apply_cors(&mut response);
        response
    }
}

/// Middleware for request logging and tracing
pub struct LoggingMiddleware {
    start_time: Instant,
    request_id: Uuid,
}

impl LoggingMiddleware {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            request_id: Uuid::new_v4(),
        }
    }

    /// Log incoming request
    pub fn log_request(&self, req: &Request<()>) {
        info!(
            request_id = %self.request_id,
            method = %req.method(),
            path = %req.uri().path(),
            "Incoming request"
        );

        debug!(
            request_id = %self.request_id,
            headers = ?req.headers(),
            "Request headers"
        );
    }

    /// Log outgoing response
    pub fn log_response(&self, response: &Response<Bytes>) {
        let duration = self.start_time.elapsed();

        info!(
            request_id = %self.request_id,
            status = response.status().as_u16(),
            duration_ms = duration.as_millis(),
            "Response sent"
        );

        debug!(
            request_id = %self.request_id,
            headers = ?response.headers(),
            "Response headers"
        );
    }

    /// Get the request ID
    pub fn request_id(&self) -> Uuid {
        self.request_id
    }
}

/// Middleware for adding request IDs
pub struct RequestIdMiddleware;

impl RequestIdMiddleware {
    /// Add request ID to headers
    pub fn add_request_id(headers: &mut HeaderMap, request_id: Uuid) {
        headers.insert(
            "x-request-id",
            HeaderValue::from_str(&request_id.to_string()).unwrap(),
        );
    }

    /// Extract request ID from headers or generate new one
    pub fn get_or_create_request_id(headers: &HeaderMap) -> Uuid {
        headers
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(Uuid::new_v4)
    }
}

/// Circuit breaker for backend failures
pub struct CircuitBreaker {
    failure_count: std::sync::atomic::AtomicUsize,
    last_failure: std::sync::RwLock<Option<Instant>>,
    threshold: usize,
    timeout: std::time::Duration,
}

impl CircuitBreaker {
    pub fn new(threshold: usize, timeout: std::time::Duration) -> Self {
        Self {
            failure_count: std::sync::atomic::AtomicUsize::new(0),
            last_failure: std::sync::RwLock::new(None),
            threshold,
            timeout,
        }
    }

    /// Check if circuit is open (failing)
    pub fn is_open(&self) -> bool {
        let count = self.failure_count.load(std::sync::atomic::Ordering::Relaxed);
        if count >= self.threshold {
            // Check if we should reset based on timeout
            if let Ok(last) = self.last_failure.read() {
                if let Some(last_time) = *last {
                    if last_time.elapsed() > self.timeout {
                        // Reset the circuit
                        self.failure_count
                            .store(0, std::sync::atomic::Ordering::Relaxed);
                        return false;
                    }
                }
            }
            true
        } else {
            false
        }
    }

    /// Record a successful request
    pub fn record_success(&self) {
        self.failure_count
            .store(0, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record a failed request
    pub fn record_failure(&self) {
        self.failure_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if let Ok(mut last) = self.last_failure.write() {
            *last = Some(Instant::now());
        }
    }
}