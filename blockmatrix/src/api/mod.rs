//! HyperMesh API module
//!
//! This module provides STOQ protocol APIs for interacting with
//! the HyperMesh system, including extension management.

pub mod extensions;
pub mod consensus_api;

use std::sync::Arc;
use std::net::Ipv6Addr;
use anyhow::Result;
use tracing::info;

use stoq::{StoqApiServer, ApiHandler};
use stoq::transport::{StoqTransport, TransportConfig};

use crate::HyperMeshSystem;

/// API server configuration
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// Server bind address (IPv6)
    pub bind_address: Ipv6Addr,
    /// Server port
    pub port: u16,
    /// Enable request logging
    pub enable_logging: bool,
    /// API service name
    pub service_name: String,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            bind_address: Ipv6Addr::UNSPECIFIED, // [::]
            port: 3000,
            enable_logging: true,
            service_name: "hypermesh".to_string(),
        }
    }
}

/// Create the main STOQ API server
pub async fn create_api_server(system: Arc<HyperMeshSystem>) -> Result<StoqApiServer> {
    let config = ApiConfig::default();

    // Create STOQ transport
    let transport_config = TransportConfig {
        bind_address: config.bind_address,
        port: config.port,
        ..Default::default()
    };

    let transport = Arc::new(StoqTransport::new(transport_config).await?);

    // Create STOQ server
    let server = StoqApiServer::new(transport);

    // Register extension handlers
    let extension_handlers = extensions::create_extension_handlers(system.extension_manager());
    for handler in extension_handlers {
        server.register_handler(handler);
    }

    Ok(server)
}

/// Start the API server
pub async fn start_api_server(
    system: Arc<HyperMeshSystem>,
    config: ApiConfig,
) -> Result<()> {
    // Create STOQ transport
    let transport_config = TransportConfig {
        bind_address: config.bind_address,
        port: config.port,
        ..Default::default()
    };

    let transport = Arc::new(StoqTransport::new(transport_config).await?);

    // Create STOQ server
    let server = StoqApiServer::new(transport);

    // Register extension handlers
    let extension_handlers = extensions::create_extension_handlers(system.extension_manager());
    for handler in extension_handlers {
        server.register_handler(handler);
    }

    info!(
        "Starting STOQ API server on [{}]:{}",
        config.bind_address,
        config.port
    );

    // Start listening
    server.listen().await?;

    Ok(())
}