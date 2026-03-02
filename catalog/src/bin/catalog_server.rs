// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Catalog Server Binary
//!
//! Standalone STOQ server for the HyperMesh Asset Catalog.
//! Listens on [::1]:9295 by default.

use anyhow::Result;
use std::sync::Arc;
use tracing::info;

use catalog::api::stoq_api::{CatalogAppState, CatalogStoqApi, CatalogStoqConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize rustls crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("catalog=info".parse()?)
                .add_directive("info".parse()?),
        )
        .init();

    info!("Starting Catalog STOQ server");

    // Parse CLI args
    let args: Vec<String> = std::env::args().collect();
    let mut bind_address = "[::1]:9295".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--bind" | "-b" => {
                if i + 1 < args.len() {
                    bind_address = args[i + 1].clone();
                    i += 1;
                }
            }
            "--help" | "-h" => {
                println!("Catalog STOQ Server");
                println!();
                println!("Usage: catalog-server [OPTIONS]");
                println!();
                println!("Options:");
                println!("  --bind, -b <ADDR>    Bind address (default: [::1]:9295)");
                println!("  --help, -h           Show this help message");
                return Ok(());
            }
            _ => {}
        }
        i += 1;
    }

    // Create application state
    let app_state = Arc::new(CatalogAppState::new());

    // Create STOQ API server config
    let config = CatalogStoqConfig {
        bind_address: bind_address.clone(),
        service_name: "catalog".to_string(),
        enable_logging: true,
    };

    // Create and start API server
    let api = Arc::new(CatalogStoqApi::new(config, app_state).await?);

    info!("Catalog STOQ server listening on {}", bind_address);

    // Run with graceful shutdown
    tokio::select! {
        result = api.serve() => {
            if let Err(e) = result {
                tracing::error!("Server error: {}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
    }

    info!("Catalog server stopped");
    Ok(())
}
