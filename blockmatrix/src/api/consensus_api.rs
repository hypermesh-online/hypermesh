//! HyperMesh Consensus API Setup
//!
//! This module provides the setup functions for the HyperMesh consensus STOQ API server.
//! It registers all required handlers and configures the server for TrustChain integration.

use std::sync::Arc;
use anyhow::{Result, anyhow};
use tracing::{info, instrument};

use stoq::api::StoqApiServer;
use stoq::transport::{StoqTransport, TransportConfig};

use crate::consensus::validation_service::ConsensusValidationService;
use crate::consensus::stoq_handlers::{
    ValidateCertificateHandler,
    ValidateProofsHandler,
    ValidationStatusHandler,
    ConsensusHealthHandler,
};

/// Consensus API configuration
#[derive(Debug, Clone)]
pub struct ConsensusApiConfig {
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

impl Default for ConsensusApiConfig {
    fn default() -> Self {
        Self {
            bind_address: "::".to_string(), // IPv6 all interfaces
            port: 9292, // STOQ default port for consensus
            max_concurrent_validations: 100,
            enable_logging: true,
            enable_cache: true,
        }
    }
}

/// Create and configure the consensus API server
#[instrument(skip(validation_service))]
pub async fn create_consensus_api_server(
    validation_service: Arc<dyn ConsensusValidationService>,
    config: ConsensusApiConfig,
) -> Result<Arc<StoqApiServer>> {
    info!("Creating HyperMesh consensus API server on {}:{}",
          config.bind_address, config.port);

    // Create STOQ transport configuration
    let transport_config = TransportConfig {
        bind_address: config.bind_address.parse()
            .map_err(|e| anyhow!("Invalid bind address: {}", e))?,
        port: config.port,
        max_connections: Some(config.max_concurrent_validations),
        ..Default::default()
    };

    // Create STOQ transport
    let transport = Arc::new(
        StoqTransport::new(transport_config)
            .await
            .map_err(|e| anyhow!("Failed to create STOQ transport: {}", e))?
    );

    // Create API server
    let server = Arc::new(StoqApiServer::new(transport));

    // Register consensus handlers
    server.register_handler(Arc::new(
        ValidateCertificateHandler::new(validation_service.clone())
    ));

    server.register_handler(Arc::new(
        ValidateProofsHandler::new(validation_service.clone())
    ));

    server.register_handler(Arc::new(
        ValidationStatusHandler::new(validation_service.clone())
    ));

    server.register_handler(Arc::new(
        ConsensusHealthHandler
    ));

    info!("Registered {} consensus API handlers", 4);
    info!("Consensus API server configured successfully");

    Ok(server)
}

/// Create a minimal consensus API server for testing
pub async fn create_test_consensus_server(
    validation_service: Arc<dyn ConsensusValidationService>,
) -> Result<Arc<StoqApiServer>> {
    let config = ConsensusApiConfig {
        bind_address: "[::1]".to_string(), // IPv6 localhost
        port: 19292, // Test port
        max_concurrent_validations: 10,
        enable_logging: false,
        enable_cache: false,
    };

    create_consensus_api_server(validation_service, config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_consensus_api_server() {
        // TODO: Implement test with mock validation service
    }
}