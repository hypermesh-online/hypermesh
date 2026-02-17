// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use anyhow::{anyhow, Result};
use bytes::{Bytes, Buf};
use h3::client::SendRequest;
use http::{HeaderMap, Method, Request, Response, StatusCode, Uri};
use std::time::Duration;
use tracing::{debug, error, warn};

use crate::middleware::RequestIdMiddleware;
use crate::pool::ConnectionPool;

/// HTTP/3 proxy for forwarding requests to backend servers
pub struct Http3Proxy {
    pool: ConnectionPool,
    timeout: Duration,
}

impl Http3Proxy {
    pub fn new(pool: ConnectionPool, timeout: Duration) -> Self {
        Self { pool, timeout }
    }

    /// Forward a request to the backend server
    pub async fn forward_request(
        &self,
        mut req: Request<()>,
        body: Option<Bytes>,
    ) -> Result<Response<Bytes>> {
        // Get connection from pool
        let mut send_request = self.pool.get_connection().await?;

        // Prepare the request - extract values before mutable borrow
        let method_str = req.method().as_str().to_string();
        let path_str = req.uri().path_and_query().map(|pq| pq.as_str().to_string()).unwrap_or_else(|| "/".to_string());

        // Now do mutable borrow
        let headers = req.headers_mut();

        // Ensure we have required pseudo-headers for HTTP/3
        if !headers.contains_key(":method") {
            headers.insert(
                ":method",
                http::HeaderValue::from_str(&method_str)?,
            );
        }

        if !headers.contains_key(":path") {
            headers.insert(
                ":path",
                http::HeaderValue::from_str(&path_str)?,
            );
        }

        if !headers.contains_key(":scheme") {
            headers.insert(":scheme", http::HeaderValue::from_static("https"));
        }

        // Forward the request with timeout
        let response = tokio::time::timeout(self.timeout, async {
            self.send_h3_request(&mut send_request, req, body).await
        })
        .await??;

        Ok(response)
    }

    /// Send HTTP/3 request to backend
    async fn send_h3_request(
        &self,
        send_request: &mut SendRequest<h3_quinn::OpenStreams, Bytes>,
        req: Request<()>,
        body: Option<Bytes>,
    ) -> Result<Response<Bytes>> {
        // Create the HTTP/3 request
        let (parts, _) = req.into_parts();
        let h3_req = Request::from_parts(parts, ());

        // Send request
        let mut stream = send_request.send_request(h3_req).await?;

        // Send body if present
        if let Some(data) = body {
            stream.send_data(data).await?;
        }

        // Finish sending
        stream.finish().await?;

        // Receive response
        let resp = stream.recv_response().await?;

        // Read response body
        let mut body = Vec::new();
        while let Some(mut chunk) = stream.recv_data().await? {
            body.extend_from_slice(&chunk.copy_to_bytes(chunk.remaining()));
        }

        // Convert to HTTP response
        let (parts, _) = resp.into_parts();
        let response = Response::from_parts(parts, Bytes::from(body));

        Ok(response)
    }

    /// Forward with retry logic
    pub async fn forward_with_retry(
        &self,
        req: Request<()>,
        body: Option<Bytes>,
        max_attempts: u32,
        base_delay: Duration,
    ) -> Result<Response<Bytes>> {
        let mut attempt = 0;
        let mut delay = base_delay;

        loop {
            attempt += 1;

            match self.forward_request(req.clone(), body.clone()).await {
                Ok(response) => {
                    debug!("Request forwarded successfully on attempt {}", attempt);
                    return Ok(response);
                }
                Err(e) if attempt < max_attempts => {
                    warn!(
                        "Request failed on attempt {}/{}: {}. Retrying in {:?}",
                        attempt, max_attempts, e, delay
                    );

                    // Mark connection as potentially unhealthy
                    // self.pool.mark_unhealthy(&conn);

                    tokio::time::sleep(delay).await;

                    // Exponential backoff
                    delay = delay.saturating_mul(2);
                }
                Err(e) => {
                    error!(
                        "Request failed after {} attempts: {}",
                        max_attempts, e
                    );
                    return Err(e);
                }
            }
        }
    }
}

/// Helper to transform backend paths
pub fn transform_backend_path(original_path: &str, backend_prefix: &str) -> String {
    // Remove the /api/v1/{service} prefix for backend
    if original_path.starts_with(backend_prefix) {
        original_path[backend_prefix.len()..].to_string()
    } else {
        original_path.to_string()
    }
}

/// Helper to build backend URI
pub fn build_backend_uri(
    original_uri: &Uri,
    backend_host: &str,
    transformed_path: &str,
) -> Result<Uri> {
    let mut parts = original_uri.clone().into_parts();

    // Set scheme
    parts.scheme = Some(http::uri::Scheme::HTTPS);

    // Set authority (host:port)
    parts.authority = Some(backend_host.parse()?);

    // Set path and query
    let path_and_query = if let Some(query) = original_uri.query() {
        format!("{}?{}", transformed_path, query)
    } else {
        transformed_path.to_string()
    };

    parts.path_and_query = Some(path_and_query.parse()?);

    Ok(Uri::from_parts(parts)?)
}