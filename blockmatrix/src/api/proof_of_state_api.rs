// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Proof of State API Setup
//!
//! This module provides the setup functions for the HyperMesh Proof of State STOQ API server.
//! It registers all required handlers and configures the server for TrustChain integration.

use anyhow::{anyhow, Result};
use std::sync::Arc;
use tracing::{info, instrument};

use stoq::api::StoqApiServer;
use stoq::transport::{StoqTransport, TransportConfig};

use crate::proof_of_state::stoq_handlers::{
    StateProofHealthHandler, ValidateCertificateHandler, ValidateProofsHandler,
    ValidationStatusHandler,
};
use crate::proof_of_state::validation_service::{StateProofValidationService, ValidationService};

/// Proof of State API configuration
#[derive(Debug, Clone)]
pub struct ProofOfStateApiConfig {
    /// Bind address for the STOQ server (IPv6)
    pub bind_address: String,
    /// Port for the STOQ server
    pub port: u16,
    /// Maximum concurrent validations
    pub max_concurrent_validations: usize,
    /// Enable request logging
    pub enable_logging: bool,
    /// Cache validation results
    pub enable_cache: bool,
}

impl Default for ProofOfStateApiConfig {
    fn default() -> Self {
        Self {
            bind_address: "::".to_string(),
            port: 9292,
            max_concurrent_validations: 100,
            enable_logging: true,
            enable_cache: true,
        }
    }
}

/// Create and configure the state proof API server with ValidationService
#[instrument(skip(validation_service))]
pub async fn create_proof_of_state_api_server_with_service(
    validation_service: Arc<ValidationService>,
    config: ProofOfStateApiConfig,
) -> Result<Arc<StoqApiServer>> {
    info!(
        "Creating HyperMesh Proof of State API server on {}:{}",
        config.bind_address, config.port
    );

    let transport_config = TransportConfig {
        bind_address: config
            .bind_address
            .parse()
            .map_err(|e| anyhow!("Invalid bind address: {e}"))?,
        port: config.port,
        max_connections: Some(config.max_concurrent_validations as u32),
        ..Default::default()
    };

    let transport = Arc::new(
        StoqTransport::new(transport_config)
            .await
            .map_err(|e| anyhow!("Failed to create STOQ transport: {e}"))?,
    );

    let server = Arc::new(StoqApiServer::new(transport));

    // Register state proof handlers
    server.register_handler(Arc::new(ValidateCertificateHandler::new(
        validation_service.clone() as Arc<dyn StateProofValidationService>,
    )));

    server.register_handler(Arc::new(ValidateProofsHandler::new(
        validation_service.clone(),
    )));

    server.register_handler(Arc::new(ValidationStatusHandler));

    server.register_handler(Arc::new(StateProofHealthHandler));

    info!("Registered 4 state proof API handlers");
    info!("State proof API server configured successfully");

    Ok(server)
}

/// Create and configure the state proof API server
#[instrument(skip(_validation_service))]
pub async fn create_proof_of_state_api_server(
    _validation_service: Arc<dyn StateProofValidationService>,
    config: ProofOfStateApiConfig,
) -> Result<Arc<StoqApiServer>> {
    let service = Arc::new(ValidationService::new());
    create_proof_of_state_api_server_with_service(service, config).await
}

/// Create a minimal state proof API server for testing
pub async fn create_test_state_proof_server(
    validation_service: Arc<ValidationService>,
) -> Result<Arc<StoqApiServer>> {
    let config = ProofOfStateApiConfig {
        bind_address: "[::1]".to_string(),
        port: 19292,
        max_concurrent_validations: 10,
        enable_logging: false,
        enable_cache: false,
    };

    create_proof_of_state_api_server_with_service(validation_service, config).await
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_create_proof_of_state_api_server() {
        // Test server creation with default config
    }
}
