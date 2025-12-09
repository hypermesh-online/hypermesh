use http::{Request, Response};
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
    headers.insert(
        http::header::ACCESS_CONTROL_ALLOW_ORIGIN,
        "http://localhost:5173".parse().unwrap(),
    );
    headers.insert(
        http::header::ACCESS_CONTROL_ALLOW_METHODS,
        "GET, POST, PUT, DELETE, OPTIONS".parse().unwrap(),
    );
    headers.insert(
        http::header::ACCESS_CONTROL_ALLOW_HEADERS,
        "Content-Type, Authorization, X-Request-ID".parse().unwrap(),
    );
    headers.insert(
        http::header::ACCESS_CONTROL_MAX_AGE,
        "3600".parse().unwrap(),
    );
    headers.insert(
        http::header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
        "true".parse().unwrap(),
    );
}