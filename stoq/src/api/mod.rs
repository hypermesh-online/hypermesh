//! STOQ API Layer - Application protocol over STOQ transport
//!
//! Provides RPC-style API framework over STOQ protocol for inter-component communication.
//! Replaces HTTP REST APIs with STOQ-native request/response messaging.

use async_trait::async_trait;
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use bytes::Bytes;
use tracing::{info, debug, warn, error, instrument};

use crate::transport::{StoqTransport, Connection, Endpoint};

/// API request over STOQ protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRequest {
    /// Request ID for correlation
    pub id: String,
    /// Service name
    pub service: String,
    /// Method/endpoint path
    pub method: String,
    /// Request payload (JSON serialized)
    pub payload: Bytes,
    /// Request metadata
    pub metadata: HashMap<String, String>,
}

/// API response over STOQ protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    /// Request ID this responds to
    pub request_id: String,
    /// Success status
    pub success: bool,
    /// Response payload (JSON serialized)
    pub payload: Bytes,
    /// Error message if failed
    pub error: Option<String>,
    /// Response metadata
    pub metadata: HashMap<String, String>,
}

/// API error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiError {
    /// Handler not found
    NotFound(String),
    /// Invalid request format
    InvalidRequest(String),
    /// Handler execution failed
    HandlerError(String),
    /// Serialization/deserialization failed
    SerializationError(String),
    /// Transport error
    TransportError(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::NotFound(s) => write!(f, "Not found: {}", s),
            ApiError::InvalidRequest(s) => write!(f, "Invalid request: {}", s),
            ApiError::HandlerError(s) => write!(f, "Handler error: {}", s),
            ApiError::SerializationError(s) => write!(f, "Serialization error: {}", s),
            ApiError::TransportError(s) => write!(f, "Transport error: {}", s),
        }
    }
}

impl std::error::Error for ApiError {}

/// API handler trait for processing requests
#[async_trait]
pub trait ApiHandler: Send + Sync {
    /// Handle an API request
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError>;

    /// Get handler name/path
    fn path(&self) -> &str;
}

/// STOQ API Server - accepts connections and routes to handlers
pub struct StoqApiServer {
    /// STOQ transport
    transport: Arc<StoqTransport>,
    /// Registered API handlers
    handlers: Arc<RwLock<HashMap<String, Arc<dyn ApiHandler>>>>,
    /// Server running flag
    running: Arc<parking_lot::RwLock<bool>>,
}

impl StoqApiServer {
    /// Create new API server
    pub fn new(transport: Arc<StoqTransport>) -> Self {
        Self {
            transport,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(parking_lot::RwLock::new(false)),
        }
    }

    /// Register an API handler
    pub fn register_handler(&self, handler: Arc<dyn ApiHandler>) {
        let path = handler.path().to_string();
        let handler_path = path.clone();
        self.handlers.write().insert(path, handler);
        info!("Registered STOQ API handler: {}", handler_path);
    }

    /// Start accepting API requests
    #[instrument(skip(self))]
    pub async fn listen(&self) -> Result<()> {
        *self.running.write() = true;
        info!("STOQ API Server listening...");

        loop {
            // Check if still running
            if !*self.running.read() {
                info!("STOQ API Server stopping...");
                break;
            }

            // Accept incoming connection
            let connection = match self.transport.accept().await {
                Ok(conn) => conn,
                Err(e) => {
                    warn!("Failed to accept connection: {}", e);
                    continue;
                }
            };

            // Spawn handler for this connection
            let handlers = Arc::clone(&self.handlers);
            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection((*connection).clone(), handlers).await {
                    error!("Connection handler error: {}", e);
                }
            });
        }

        Ok(())
    }

    /// Handle a single connection's requests
    async fn handle_connection(
        connection: Connection,
        handlers: Arc<RwLock<HashMap<String, Arc<dyn ApiHandler>>>>,
    ) -> Result<()> {
        debug!("Handling new STOQ API connection");

        loop {
            // Accept bidirectional stream for request/response
            let (mut send, mut recv) = match connection.accept_bi().await {
                Ok(s) => s,
                Err(e) => {
                    debug!("Stream closed: {}", e);
                    break;
                }
            };

            // Read request
            let request_data = match recv.read_to_end(10 * 1024 * 1024).await {
                Ok(data) => data,
                Err(e) => {
                    warn!("Failed to read request: {}", e);
                    continue;
                }
            };

            // Deserialize request
            let request: ApiRequest = match bincode::deserialize(&request_data) {
                Ok(req) => req,
                Err(e) => {
                    warn!("Failed to deserialize request: {}", e);
                    let error_response = ApiResponse {
                        request_id: String::new(),
                        success: false,
                        payload: Bytes::new(),
                        error: Some(format!("Invalid request format: {}", e)),
                        metadata: HashMap::new(),
                    };
                    let _ = Self::send_response(&mut send, error_response).await;
                    continue;
                }
            };

            debug!("Received API request: {} {}", request.service, request.method);

            // Route to handler (scope the RwLock guard to avoid Send issues)
            let handler_path = format!("{}/{}", request.service, request.method);
            let handler = handlers.read().get(&handler_path).cloned();

            let response = match handler {
                Some(h) => {
                    match h.handle(request.clone()).await {
                        Ok(resp) => resp,
                        Err(e) => ApiResponse {
                            request_id: request.id.clone(),
                            success: false,
                            payload: Bytes::new(),
                            error: Some(e.to_string()),
                            metadata: HashMap::new(),
                        }
                    }
                }
                None => {
                    ApiResponse {
                        request_id: request.id.clone(),
                        success: false,
                        payload: Bytes::new(),
                        error: Some(format!("Handler not found: {}", handler_path)),
                        metadata: HashMap::new(),
                    }
                }
            };

            // Send response
            if let Err(e) = Self::send_response(&mut send, response).await {
                error!("Failed to send response: {}", e);
            }
        }

        debug!("STOQ API connection closed");
        Ok(())
    }

    /// Send API response over stream
    async fn send_response(
        send: &mut quinn::SendStream,
        response: ApiResponse,
    ) -> Result<()> {
        let response_data = bincode::serialize(&response)
            .map_err(|e| anyhow!("Failed to serialize response: {}", e))?;

        send.write_all(&response_data).await?;
        send.finish()?;

        Ok(())
    }

    /// Stop the server
    pub fn stop(&self) {
        *self.running.write() = false;
    }
}

/// STOQ API Client - makes requests to remote services
pub struct StoqApiClient {
    /// STOQ transport
    transport: Arc<StoqTransport>,
    /// Connection pool to services
    connections: Arc<RwLock<HashMap<String, Connection>>>,
}

impl StoqApiClient {
    /// Create new API client
    pub fn new(transport: Arc<StoqTransport>) -> Self {
        Self {
            transport,
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Make an API call
    #[instrument(skip(self, payload))]
    pub async fn call<T, R>(
        &self,
        service: &str,
        method: &str,
        payload: &T,
    ) -> Result<R, ApiError>
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        // Serialize payload
        let payload_bytes = serde_json::to_vec(payload)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        // Create request
        let request = ApiRequest {
            id: uuid::Uuid::new_v4().to_string(),
            service: service.to_string(),
            method: method.to_string(),
            payload: Bytes::from(payload_bytes),
            metadata: HashMap::new(),
        };

        // Get or create connection to service
        let mut connection = self.get_connection(service).await
            .map_err(|e| ApiError::TransportError(e.to_string()))?;

        // Open bidirectional stream
        let (mut send, mut recv) = connection.open_bi().await
            .map_err(|e| ApiError::TransportError(e.to_string()))?;

        // Send request
        let request_data = bincode::serialize(&request)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        send.write_all(&request_data).await
            .map_err(|e| ApiError::TransportError(e.to_string()))?;
        send.finish()
            .map_err(|e| ApiError::TransportError(e.to_string()))?;

        // Receive response
        let response_data = recv.read_to_end(10 * 1024 * 1024).await
            .map_err(|e| ApiError::TransportError(e.to_string()))?;

        let response: ApiResponse = bincode::deserialize(&response_data)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        // Check success
        if !response.success {
            return Err(ApiError::HandlerError(
                response.error.unwrap_or_else(|| "Unknown error".to_string())
            ));
        }

        // Deserialize response payload
        let result: R = serde_json::from_slice(&response.payload)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        Ok(result)
    }

    /// Get or create connection to a service
    async fn get_connection(&self, service: &str) -> Result<Connection> {
        // Check if we have an existing connection
        if let Some(conn) = self.connections.read().get(service) {
            return Ok(conn.clone());
        }

        // Create new connection
        // TODO: Service discovery - resolve service name to endpoint
        let endpoint = self.resolve_service(service).await?;
        let connection = self.transport.connect(&endpoint).await?;

        // Store Arc in cache, return cloned Connection
        let conn_clone = (*connection).clone();
        self.connections.write().insert(service.to_string(), conn_clone.clone());

        Ok(conn_clone)
    }

    /// Resolve service name to endpoint (placeholder)
    async fn resolve_service(&self, service: &str) -> Result<Endpoint> {
        // TODO: Integrate with TrustChain DNS resolution
        // For now, use hardcoded localhost endpoints
        match service {
            "trustchain" => Ok(Endpoint {
                address: std::net::Ipv6Addr::LOCALHOST,
                port: 9293,
                server_name: Some("trustchain".to_string()),
            }),
            "hypermesh" => Ok(Endpoint {
                address: std::net::Ipv6Addr::LOCALHOST,
                port: 9292,
                server_name: Some("hypermesh".to_string()),
            }),
            "caesar" => Ok(Endpoint {
                address: std::net::Ipv6Addr::LOCALHOST,
                port: 9294,
                server_name: Some("caesar".to_string()),
            }),
            _ => Err(anyhow!("Unknown service: {}", service)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Add STOQ API integration tests
}
