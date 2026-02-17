// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use http::{Request, Response, header::HeaderValue};
use std::time::Instant;
use tracing::{info, warn, error};
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

    pub fn log_response<B>(
        response: &Response<B>,
        request_id: &str,
        start_time: Instant,
    ) {
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

pub fn add_cors_headers<B>(response: &mut Response<B>) {
    let headers = response.headers_mut();

    // Safe: All header values are valid static strings that cannot fail
    headers.insert(
        http::header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("http://localhost:5173"),
    );
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