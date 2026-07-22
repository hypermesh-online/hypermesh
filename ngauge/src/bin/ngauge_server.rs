// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! NGauge Server Binary
//!
//! Standalone STOQ-compatible server for HyperMesh analytics and metrics.
//! Listens on [::1]:9296 by default.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

use ngauge::api::stoq_api::{NGaugeAppState, NGaugeStoqApi, NGaugeStoqConfig};
use ngauge::ingestion::{IngestionConfig, MetricsIngestionPipeline};
use ngauge::udp_ingest::{self, UdpIngestConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize rustls crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("ngauge=info".parse()?)
                .add_directive("info".parse()?),
        )
        .init();

    info!("Starting NGauge STOQ server");

    // Parse CLI args
    let args: Vec<String> = std::env::args().collect();
    let mut bind_address = "[::1]:9296".to_string();
    let mut udp_bind_address = udp_ingest::DEFAULT_UDP_BIND.to_string();
    let mut service_name = "ngauge".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--bind" | "-b" => {
                if i + 1 < args.len() {
                    bind_address = args[i + 1].clone();
                    i += 1;
                }
            }
            "--udp-bind" => {
                if i + 1 < args.len() {
                    udp_bind_address = args[i + 1].clone();
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
                println!("NGauge STOQ Server");
                println!();
                println!("Usage: ngauge-server [OPTIONS]");
                println!();
                println!("Options:");
                println!("  --bind, -b <ADDR>           QUIC bind address (default: [::1]:9296)");
                println!("  --udp-bind <ADDR>           UDP metrics bind address (default: [::1]:9297)");
                println!("  --service-name, -s <NAME>   Service name (default: ngauge)");
                println!("  --help, -h                  Show this help message");
                return Ok(());
            }
            _ => {}
        }
        i += 1;
    }

    // Create application state
    let app_state = Arc::new(NGaugeAppState::new());

    // Create config
    let config = NGaugeStoqConfig {
        bind_address: bind_address.clone(),
        service_name,
        enable_logging: true,
    };

    // Create API server
    let api = NGaugeStoqApi::new(config, app_state.clone());

    info!("NGauge STOQ server configured for {}", bind_address);

    // Create metrics ingestion pipeline for UDP listener
    let pipeline = Arc::new(Mutex::new(MetricsIngestionPipeline::new(
        IngestionConfig::default(),
    )));

    // Start UDP metrics ingestion listener
    let udp_config = UdpIngestConfig {
        bind_address: udp_bind_address.clone(),
    };
    let udp_pipeline = pipeline.clone();
    let udp_state = app_state.clone();
    let udp_handle = tokio::spawn(async move {
        if let Err(e) = udp_ingest::run_udp_ingest(udp_config, udp_pipeline, udp_state).await {
            warn!("UDP metrics listener failed: {e}");
        }
    });

    // Run with graceful shutdown
    tokio::select! {
        result = api.serve() => {
            if let Err(e) = result {
                tracing::error!("Server error: {}", e);
            }
        }
        _ = udp_handle => {
            warn!("UDP listener exited unexpectedly");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
    }

    info!("NGauge server stopped");
    Ok(())
}
