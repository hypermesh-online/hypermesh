//! HTTP Gateway for STOQ Transport Layer
//!
//! Provides HTTP compatibility layer for serving static UI files
//! and handling HTTP-like requests over STOQ protocol.

use anyhow::{Result, Context};
use std::sync::Arc;
use std::collections::HashMap;
use tracing::{info, debug, warn};

use crate::static_server::{StaticFileServer, StaticFileResponse};

/// HTTP Gateway for handling HTTP-like requests over STOQ
pub struct HttpGateway {
    /// Static file server for UI assets
    static_server: Arc<StaticFileServer>,

    /// Route handlers for API endpoints
    route_handlers: HashMap<String, Box<dyn RouteHandler + Send + Sync>>,
}

/// Trait for handling specific routes
pub trait RouteHandler: Send + Sync {
    /// Handle a request and return a response
    fn handle(&self, request: &HttpRequest) -> Result<HttpResponse>;
}

/// HTTP-like request structure
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

/// HTTP-like response structure
#[derive(Debug)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpGateway {
    /// Create new HTTP gateway
    pub fn new(static_server: Arc<StaticFileServer>) -> Self {
        let route_handlers = HashMap::new();

        Self {
            static_server,
            route_handlers,
        }
    }

    /// Add route handlers (called after creation)
    pub fn add_route_handler(&mut self, path: String, handler: Box<dyn RouteHandler + Send + Sync>) {
        self.route_handlers.insert(path, handler);
    }

    /// Handle an HTTP-like request over STOQ
    pub async fn handle_request(&self, request: HttpRequest) -> Result<HttpResponse> {
        debug!("HTTP Gateway handling request: {} {}", request.method, request.path);

        // Check if this is an API route
        if request.path.starts_with("/api/") {
            return self.handle_api_request(request);
        }

        // Check if this is a WebSocket upgrade request
        if request.path.starts_with("/ws/") {
            return self.handle_websocket_request(request);
        }

        // Otherwise, serve static files
        self.handle_static_request(request).await
    }

    /// Handle API requests
    fn handle_api_request(&self, request: HttpRequest) -> Result<HttpResponse> {
        // Find matching route handler
        if let Some(handler) = self.route_handlers.get(&request.path) {
            return handler.handle(&request);
        }

        // No handler found - return 404
        Ok(HttpResponse {
            status: 404,
            headers: HashMap::new(),
            body: b"API endpoint not found".to_vec(),
        })
    }

    /// Handle WebSocket upgrade requests
    fn handle_websocket_request(&self, request: HttpRequest) -> Result<HttpResponse> {
        // WebSocket upgrade logic would go here
        // For now, return not implemented
        Ok(HttpResponse {
            status: 501,
            headers: HashMap::new(),
            body: b"WebSocket not implemented yet".to_vec(),
        })
    }

    /// Handle static file requests
    async fn handle_static_request(&self, request: HttpRequest) -> Result<HttpResponse> {
        // Only handle GET requests for static files
        if request.method != "GET" && request.method != "HEAD" {
            return Ok(HttpResponse {
                status: 405,
                headers: HashMap::new(),
                body: b"Method not allowed for static files".to_vec(),
            });
        }

        // Serve static file
        let response = self.static_server.serve(&request.path).await?;

        // Convert to HTTP response
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), response.content_type);
        headers.insert("Cache-Control".to_string(), response.cache_control);
        headers.insert(
            "Content-Length".to_string(),
            response.content.len().to_string(),
        );

        Ok(HttpResponse {
            status: response.status,
            headers,
            body: if request.method == "HEAD" {
                Vec::new()
            } else {
                response.content
            },
        })
    }

    /// Register a custom route handler
    pub fn register_route(
        &mut self,
        path: String,
        handler: Box<dyn RouteHandler + Send + Sync>,
    ) {
        self.route_handlers.insert(path, handler);
    }

    /// Parse STOQ message into HTTP request
    pub fn parse_stoq_message(message: &[u8]) -> Result<HttpRequest> {
        // Simple HTTP-like parsing
        // In production, this would be more robust
        let text = String::from_utf8_lossy(message);
        let lines: Vec<&str> = text.lines().collect();

        if lines.is_empty() {
            return Ok(HttpRequest {
                method: "GET".to_string(),
                path: "/".to_string(),
                headers: HashMap::new(),
                body: Vec::new(),
            });
        }

        // Parse request line
        let parts: Vec<&str> = lines[0].split_whitespace().collect();
        let method = parts.get(0).unwrap_or(&"GET").to_string();
        let path = parts.get(1).unwrap_or(&"/").to_string();

        // Parse headers
        let mut headers = HashMap::new();
        let mut body_start = 1;
        for (i, line) in lines.iter().enumerate().skip(1) {
            if line.is_empty() {
                body_start = i + 1;
                break;
            }
            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_string();
                let value = line[colon_pos + 1..].trim().to_string();
                headers.insert(key, value);
            }
        }

        // Parse body
        let body = if body_start < lines.len() {
            lines[body_start..].join("\n").into_bytes()
        } else {
            Vec::new()
        };

        Ok(HttpRequest {
            method,
            path,
            headers,
            body,
        })
    }

    /// Format HTTP response for STOQ transmission
    pub fn format_http_response(response: &HttpResponse) -> Vec<u8> {
        let mut output = Vec::new();

        // Status line
        let status_text = match response.status {
            200 => "OK",
            404 => "Not Found",
            405 => "Method Not Allowed",
            500 => "Internal Server Error",
            501 => "Not Implemented",
            _ => "Unknown",
        };
        output.extend_from_slice(
            format!("HTTP/1.1 {} {}\r\n", response.status, status_text).as_bytes(),
        );

        // Headers
        for (key, value) in &response.headers {
            output.extend_from_slice(format!("{}: {}\r\n", key, value).as_bytes());
        }

        // End of headers
        output.extend_from_slice(b"\r\n");

        // Body
        output.extend_from_slice(&response.body);

        output
    }
}

/// Health check handler
struct HealthCheckHandler;

impl RouteHandler for HealthCheckHandler {
    fn handle(&self, _request: &HttpRequest) -> Result<HttpResponse> {
        let body = r#"{
            "status": "healthy",
            "service": "hypermesh",
            "protocol": "stoq"
        }"#;

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        Ok(HttpResponse {
            status: 200,
            headers,
            body: body.as_bytes().to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::static_server::StaticServerConfig;

    #[test]
    fn test_parse_http_request() {
        let request_str = "GET /index.html HTTP/1.1\r\n\
                          Host: localhost\r\n\
                          User-Agent: test\r\n\
                          \r\n";

        let request = HttpGateway::parse_stoq_message(request_str.as_bytes()).unwrap();

        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/index.html");
        assert_eq!(request.headers.get("Host").unwrap(), "localhost");
        assert_eq!(request.headers.get("User-Agent").unwrap(), "test");
    }

    #[test]
    fn test_format_http_response() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "text/plain".to_string());

        let response = HttpResponse {
            status: 200,
            headers,
            body: b"Hello World".to_vec(),
        };

        let formatted = HttpGateway::format_http_response(&response);
        let formatted_str = String::from_utf8_lossy(&formatted);

        assert!(formatted_str.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(formatted_str.contains("Content-Type: text/plain\r\n"));
        assert!(formatted_str.ends_with("Hello World"));
    }

    #[tokio::test]
    async fn test_health_check() {
        let static_config = StaticServerConfig::default();
        let static_server = Arc::new(StaticFileServer::new(static_config));
        let gateway = HttpGateway::new(static_server);

        let request = HttpRequest {
            method: "GET".to_string(),
            path: "/api/health".to_string(),
            headers: HashMap::new(),
            body: Vec::new(),
        };

        let response = gateway.handle_request(request).await.unwrap();

        assert_eq!(response.status, 200);
        assert_eq!(
            response.headers.get("Content-Type").unwrap(),
            "application/json"
        );
        assert!(String::from_utf8_lossy(&response.body).contains("healthy"));
    }
}