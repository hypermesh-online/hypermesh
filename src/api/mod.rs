//! HyperMesh API module
//!
//! This module provides RESTful API endpoints for interacting with
//! the HyperMesh system, including extension management.

pub mod extensions;

use axum::Router;
use tokio::net::TcpListener;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::info;

use crate::HyperMeshSystem;

/// API server configuration
#[derive(Debug, Clone)]
pub struct ApiConfig {
    /// Server bind address
    pub bind_address: SocketAddr,
    /// Enable CORS
    pub enable_cors: bool,
    /// API prefix path
    pub api_prefix: String,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            bind_address: ([0, 0, 0, 0], 3000).into(),
            enable_cors: true,
            api_prefix: "/api/v1".to_string(),
        }
    }
}

/// Create the main API router
pub fn create_api_router(system: Arc<HyperMeshSystem>) -> Router {
    let extension_state = extensions::ExtensionApiState {
        manager: system.extension_manager(),
    };

    Router::new()
        .nest("/extensions", extensions::create_extension_router(extension_state))
        .route("/health", axum::routing::get(extensions::health_check))
        .route("/ws", axum::routing::get(extensions::websocket::websocket_handler))
}

/// Start the API server
pub async fn start_api_server(
    system: Arc<HyperMeshSystem>,
    config: ApiConfig,
) -> anyhow::Result<()> {
    let router = create_api_router(system);

    // Add CORS if enabled
    let app = if config.enable_cors {
        use tower_http::cors::{CorsLayer, Any};
        router.layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        )
    } else {
        router
    };

    // Add API prefix
    let app = Router::new().nest(&config.api_prefix, app);

    info!("Starting API server on {}", config.bind_address);

    let listener = TcpListener::bind(config.bind_address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}