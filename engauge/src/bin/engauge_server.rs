// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Engauge Server Binary
//!
//! Standalone STOQ-compatible server for HyperMesh analytics and metrics.
//! Listens on [::1]:9296 by default.

use anyhow::Result;
use std::sync::Arc;
use tracing::info;

use engauge::api::stoq_api::{EngaugeAppState, EngaugeStoqApi, EngaugeStoqConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize rustls crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("engauge=info".parse()?)
                .add_directive("info".parse()?),
        )
        .init();

    info!("Starting Engauge STOQ server");

    // Parse CLI args
    let args: Vec<String> = std::env::args().collect();
    let mut bind_address = "[::1]:9296".to_string();
    let mut service_name = "engauge".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--bind" | "-b" => {
                if i + 1 < args.len() {
                    bind_address = args[i + 1].clone();
                    i += 1;
                }
            }
            "--service-name" | "-s" => {
                if i + 1 < args.len() {
                    service_name = args[i + 1].clone();
                    i += 1;
                }
            }
            "--help" | "-h" => {
                println!("Engauge STOQ Server");
                println!();
                println!("Usage: engauge-server [OPTIONS]");
                println!();
                println!("Options:");
                println!("  --bind, -b <ADDR>           Bind address (default: [::1]:9296)");
                println!("  --service-name, -s <NAME>   Service name (default: engauge)");
                println!("  --help, -h                  Show this help message");
                return Ok(());
            }
            _ => {}
        }
        i += 1;
    }

    // Create application state
    let app_state = Arc::new(EngaugeAppState::new());

    // Create config
    let config = EngaugeStoqConfig {
        bind_address: bind_address.clone(),
        service_name,
        enable_logging: true,
    };

    // Create API server
    let api = EngaugeStoqApi::new(config, app_state);

    info!("Engauge STOQ server configured for {}", bind_address);

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

    info!("Engauge server stopped");
    Ok(())
}
