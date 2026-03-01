// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use http::{header::HeaderValue, Request, Response};
use std::time::Instant;
use tracing::{info, warn};
use uuid::Uuid;

pub struct RequestLogger;

impl RequestLogger {
    pub fn log_request<B>(request: &Request<B>, request_id: &str) {
        info!(
            request_id = %request_id,
            method = %request.method(),
            path = %request.uri().path(),
            "Incoming request"
        );
    }

    pub fn log_response<B>(response: &Response<B>, request_id: &str, start_time: Instant) {
        let duration_ms = start_time.elapsed().as_millis();
        let status = response.status();

        if status.is_success() {
            info!(
                request_id = %request_id,
                status = %status,
                duration_ms = %duration_ms,
                "Request completed"
            );
        } else {
            warn!(
                request_id = %request_id,
                status = %status,
                duration_ms = %duration_ms,
                "Request failed"
            );
        }
    }

    pub fn generate_request_id() -> String {
        Uuid::new_v4().to_string()
    }
}

/// Default CORS origin used when no specific origin is configured
pub const DEFAULT_CORS_ORIGIN: &str = "http://localhost:5173";

pub fn add_cors_headers<B>(response: &mut Response<B>) {
    add_cors_headers_with_origin(response, DEFAULT_CORS_ORIGIN);
}

pub fn add_cors_headers_with_origin<B>(response: &mut Response<B>, origin: &str) {
    let headers = response.headers_mut();

    if let Ok(origin_val) = origin.parse() {
        headers.insert(http::header::ACCESS_CONTROL_ALLOW_ORIGIN, origin_val);
    }
    headers.insert(
        http::header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, POST, PUT, DELETE, OPTIONS"),
    );
    headers.insert(
        http::header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("Content-Type, Authorization, X-Request-ID"),
    );
    headers.insert(
        http::header::ACCESS_CONTROL_MAX_AGE,
        HeaderValue::from_static("3600"),
    );
}
