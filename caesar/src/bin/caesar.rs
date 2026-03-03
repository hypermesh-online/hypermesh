// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Caesar EVP server binary.
//!
//! Launches the Caesar Ephemeral Value Protocol STOQ API server.

use std::sync::Arc;
use tokio::sync::RwLock;

use caesar::api::stoq_api::{CaesarAppState, CaesarStoqApi, CaesarStoqConfig};
use caesar::{CaesarConfig, CaesarProtocol};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // Parse CLI args
    let args: Vec<String> = std::env::args().collect();
    let mut bind_address = "[::1]:9294".to_string();
    let mut service_name = "caesar".to_string();

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
                println!("Caesar EVP Server");
                println!();
                println!("Usage: caesar [OPTIONS]");
                println!();
                println!("Options:");
                println!("  --bind, -b <ADDR>           Bind address (default: [::1]:9294)");
                println!("  --service-name, -s <NAME>   Service name (default: caesar)");
                println!("  --help, -h                  Show this help message");
                return Ok(());
            }
            _ => {}
        }
        i += 1;
    }

    let stoq_config = CaesarStoqConfig {
        bind_address: bind_address.clone(),
        service_name,
        enable_logging: true,
    };

    println!();
    println!("  Caesar Ephemeral Value Protocol");
    println!("  --------------------------------");
    println!("  bind: {}", stoq_config.bind_address);
    println!("  service: {}", stoq_config.service_name);
    println!();

    // Initialize protocol
    let caesar_config = CaesarConfig::default();
    let protocol = CaesarProtocol::new(caesar_config).await?;

    // Build shared application state
    let app_state = Arc::new(CaesarAppState {
        protocol: Arc::new(RwLock::new(protocol)),
    });

    // Create STOQ API server
    let api = Arc::new(
        CaesarStoqApi::new(stoq_config, app_state)
            .await?,
    );

    // Spawn server and wait for Ctrl+C
    let server = Arc::clone(&api);
    let server_handle = tokio::spawn(async move { server.serve().await });

    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for Ctrl+C");

    println!();
    println!("  Shutting down...");
    api.stop();

    // Give the server loop a moment to observe the stop flag
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), server_handle).await;

    println!("  Caesar stopped.");
    Ok(())
}
