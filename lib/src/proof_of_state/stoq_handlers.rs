//! HyperMesh Consensus STOQ API Handlers
//!
//! Provides STOQ API handlers for consensus validation services.
//! This module implements the handlers that TrustChain needs to validate certificates
//! and four-proof sets through HyperMesh consensus.

use async_trait::async_trait;
use std::sync::Arc;
use std::collections::HashMap;
use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use tracing::{info, debug, warn, instrument};
use bytes::Bytes;

use stoq::api::{ApiHandler, ApiRequest, ApiResponse, ApiError};

use super::validation_service::{
    ConsensusValidationService,
    CertificateValidationRequest,
    FourProofValidationRequest,
    ValidationResult,
};

/// Status request for checking validation progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusRequest {
    pub request_id: String,
}

/// Certificate validation handler - implements the consensus/validate_certificate endpoint
pub struct ValidateCertificateHandler {
    validation_service: Arc<ConsensusValidationService>,
}

impl ValidateCertificateHandler {
    pub fn new(validation_service: Arc<ConsensusValidationService>) -> Self {
        Self { validation_service }
    }
}

#[async_trait]
impl ApiHandler for ValidateCertificateHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling certificate validation request: {}", request.id);

        // Deserialize request
        let validation_request: CertificateValidationRequest =
            serde_json::from_slice(&request.payload)
                .map_err(|e| ApiError::InvalidRequest(format!("Invalid certificate request: {}", e)))?;

        // Call validation service - use the correct method name
        let result = self.validation_service
            .validate_certificate_request(validation_request)
            .await
            .map_err(|e| ApiError::HandlerError(format!("Certificate validation failed: {}", e)))?;

        // Serialize response
        let payload = serde_json::to_vec(&result)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        Ok(ApiResponse {
            request_id: request.id,
            success: true,
            payload: payload.into(),
            error: None,
            metadata: HashMap::new(),
        })
    }

    fn path(&self) -> &str {
        "consensus/validate_certificate"
    }
}

/// Four-proof validation handler - implements the consensus/validate_proofs endpoint
pub struct ValidateProofsHandler {
    validation_service: Arc<ConsensusValidationService>,
}

impl ValidateProofsHandler {
    pub fn new(validation_service: Arc<ConsensusValidationService>) -> Self {
        Self { validation_service }
    }
}

#[async_trait]
impl ApiHandler for ValidateProofsHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling four-proof validation request: {}", request.id);

        // Deserialize request
        let validation_request: FourProofValidationRequest =
            serde_json::from_slice(&request.payload)
                .map_err(|e| ApiError::InvalidRequest(format!("Invalid four-proof request: {}", e)))?;

        // Call validation service - use the correct method name
        let result = self.validation_service
            .validate_four_proof_set(validation_request)
            .await
            .map_err(|e| ApiError::HandlerError(format!("Four-proof validation failed: {}", e)))?;

        // Serialize response
        let payload = serde_json::to_vec(&result)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        Ok(ApiResponse {
            request_id: request.id,
            success: true,
            payload: payload.into(),
            error: None,
            metadata: HashMap::new(),
        })
    }

    fn path(&self) -> &str {
        "consensus/validate_proofs"
    }
}

/// Validation status handler - implements the consensus/validation_status endpoint
pub struct ValidationStatusHandler {
    validation_service: Arc<ConsensusValidationService>,
}

impl ValidationStatusHandler {
    pub fn new(validation_service: Arc<ConsensusValidationService>) -> Self {
        Self { validation_service }
    }
}

#[async_trait]
impl ApiHandler for ValidationStatusHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        debug!("Handling validation status request: {}", request.id);

        // Deserialize request
        let status_request: StatusRequest =
            serde_json::from_slice(&request.payload)
                .map_err(|e| ApiError::InvalidRequest(format!("Invalid status request: {}", e)))?;

        // Get validation status
        let result = self.validation_service
            .get_validation_status(&status_request.request_id)
            .await
            .map_err(|e| ApiError::HandlerError(format!("Failed to get validation status: {}", e)))?;

        // Serialize response
        let payload = serde_json::to_vec(&result)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        Ok(ApiResponse {
            request_id: request.id,
            success: true,
            payload: payload.into(),
            error: None,
            metadata: HashMap::new(),
        })
    }

    fn path(&self) -> &str {
        "consensus/validation_status"
    }
}

/// Health check handler for consensus service monitoring
pub struct ConsensusHealthHandler;

#[async_trait]
impl ApiHandler for ConsensusHealthHandler {
    async fn handle(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        #[derive(Serialize)]
        struct HealthStatus {
            status: String,
            service: String,
            version: String,
        }

        let health = HealthStatus {
            status: "healthy".to_string(),
            service: "hypermesh-consensus".to_string(),
            version: "0.1.0".to_string(), // TODO: Get from build-time constant
        };

        let payload = serde_json::to_vec(&health)
            .map_err(|e| ApiError::SerializationError(e.to_string()))?;

        Ok(ApiResponse {
            request_id: request.id,
            success: true,
            payload: payload.into(),
            error: None,
            metadata: HashMap::new(),
        })
    }

    fn path(&self) -> &str {
        "consensus/health"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    // Mock validation service for testing
    struct MockValidationService;

    #[tokio::test]
    async fn test_validate_certificate_handler() {
        // TODO: Implement test with mock validation service
    }

    #[tokio::test]
    async fn test_validate_proofs_handler() {
        // TODO: Implement test with mock validation service
    }

    #[tokio::test]
    async fn test_validation_status_handler() {
        // TODO: Implement test with mock validation service
    }

    #[tokio::test]
    async fn test_health_handler() {
        let handler = ConsensusHealthHandler;
        let request = ApiRequest {
            id: "test-1".to_string(),
            service: "test".to_string(),
            method: "consensus/health".to_string(),
            payload: Bytes::new(),
            metadata: HashMap::new(),
        };

        let response = handler.handle(request).await.unwrap();
        assert!(response.success);
        assert!(response.error.is_none());

        // Verify health response can be deserialized
        let health: serde_json::Value = serde_json::from_slice(&response.payload).unwrap();
        assert_eq!(health["status"], "healthy");
        assert_eq!(health["service"], "hypermesh-consensus");
    }
}