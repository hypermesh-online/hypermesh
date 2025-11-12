//! HyperMesh Consensus STOQ API - Replaces HTTP warp server
//!
//! Provides consensus validation services over STOQ protocol instead of HTTP.

use async_trait::async_trait;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use tracing::{info, debug, warn, instrument};

use stoq::api::{ApiHandler, ApiRequest, ApiResponse, ApiError};
use stoq::StoqApiServer;
use stoq::transport::StoqTransport;

use super::validation_service::{
    ConsensusValidationService, CertificateValidationRequest, FourProofValidationRequest,
    ValidationResult, ValidationStatus,
};

/// Consensus API configuration over STOQ
#[derive(Debug, Clone)]
pub struct ConsensusStoqConfig {
    /// STOQ bind address (IPv6)
    pub bind_address: String,
    /// Maximum request size (bytes)
    pub max_request_size: usize,
    /// Enable validation caching
    pub enable_cache: bool,
}

impl Default for ConsensusStoqConfig {
    fn default() -> Self {
        Self {
            bind_address: "[::1]:9292".to_string(), // IPv6 localhost, STOQ default port
            max_request_size: 10 * 1024 * 1024, // 10MB
            enable_cache: true,
        }
    }
}

/// Certificate validation handler
pub struct CertificateValidationHandler {
    service: Arc<ConsensusValidationService>,
}

impl CertificateValidationHandler {
    pub fn new(service: Arc<ConsensusValidationService>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl ApiHandler for CertificateValidationHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling certificate validation request: {}", request.id);

        // Deserialize request payload
        let cert_request: CertificateValidationRequest = serde_json::from_slice(&request.payload)
            .map_err(|e| ApiError::InvalidRequest(format!("Invalid certificate request: {}", e)))?;

        // Validate certificate through consensus
        let validation_result = self.service.validate_certificate(cert_request).await
            .map_err(|e| ApiError::HandlerError(e.to_string()))?;

        // Serialize response
        let payload = serde_json::to_vec(&validation_result)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        Ok(ApiResponse {
            request_id: request.id,
            success: true,
            payload: payload.into(),
            error: None,
            metadata: std::collections::HashMap::new(),
        })
    }

    fn path(&self) -> &str {
        "consensus/validate_certificate"
    }
}

/// Four-proof validation handler
pub struct FourProofValidationHandler {
    service: Arc<ConsensusValidationService>,
}

impl FourProofValidationHandler {
    pub fn new(service: Arc<ConsensusValidationService>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl ApiHandler for FourProofValidationHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling four-proof validation request: {}", request.id);

        // Deserialize request payload
        let proof_request: FourProofValidationRequest = serde_json::from_slice(&request.payload)
            .map_err(|e| ApiError::InvalidRequest(format!("Invalid proof request: {}", e)))?;

        // Validate through four-proof consensus
        let validation_result = self.service.validate_four_proofs(proof_request).await
            .map_err(|e| ApiError::HandlerError(e.to_string()))?;

        // Serialize response
        let payload = serde_json::to_vec(&validation_result)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        Ok(ApiResponse {
            request_id: request.id,
            success: true,
            payload: payload.into(),
            error: None,
            metadata: std::collections::HashMap::new(),
        })
    }

    fn path(&self) -> &str {
        "consensus/validate_proofs"
    }
}

/// Health check handler
pub struct HealthCheckHandler;

#[async_trait]
impl ApiHandler for HealthCheckHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        #[derive(Serialize)]
        struct HealthStatus {
            status: String,
            service: String,
        }

        let health = HealthStatus {
            status: "healthy".to_string(),
            service: "hypermesh-consensus".to_string(),
        };

        let payload = serde_json::to_vec(&health)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        Ok(ApiResponse {
            request_id: request.id,
            success: true,
            payload: payload.into(),
            error: None,
            metadata: std::collections::HashMap::new(),
        })
    }

    fn path(&self) -> &str {
        "consensus/health"
    }
}

/// Consensus STOQ API Server - Replaces warp HTTP server
pub struct ConsensusStoqApi {
    server: Arc<StoqApiServer>,
    config: ConsensusStoqConfig,
}

impl ConsensusStoqApi {
    /// Create new consensus API server over STOQ
    #[instrument(skip(service))]
    pub async fn new(
        service: Arc<ConsensusValidationService>,
        config: ConsensusStoqConfig,
    ) -> Result<Self> {
        info!("Creating Consensus STOQ API server on {}", config.bind_address);

        // Create STOQ transport
        let transport_config = stoq::TransportConfig {
            bind_address: config.bind_address.parse()
                .map_err(|e| anyhow!("Invalid bind address: {}", e))?,
            port: 9292, // STOQ default port
            ..Default::default()
        };

        let transport = Arc::new(StoqTransport::new(transport_config).await?);

        // Create API server
        let server = Arc::new(StoqApiServer::new(transport));

        // Register handlers
        server.register_handler(Arc::new(CertificateValidationHandler::new(Arc::clone(&service))));
        server.register_handler(Arc::new(FourProofValidationHandler::new(Arc::clone(&service))));
        server.register_handler(Arc::new(HealthCheckHandler));

        info!("Consensus STOQ API handlers registered");

        Ok(Self { server, config })
    }

    /// Start the API server
    #[instrument(skip(self))]
    pub async fn serve(self: Arc<Self>) -> Result<()> {
        info!("Starting Consensus STOQ API server...");
        self.server.listen().await
    }

    /// Stop the server gracefully
    pub fn stop(&self) {
        info!("Stopping Consensus STOQ API server");
        self.server.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Add STOQ API integration tests
}
