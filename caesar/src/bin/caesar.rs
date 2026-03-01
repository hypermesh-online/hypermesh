// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Caesar EVP server binary.
//!
//! Launches the Caesar Ephemeral Value Protocol STOQ API server with
//! default configuration. CLI argument parsing is intentionally deferred
//! to a future sprint -- this binary is the minimal server launcher.

use std::sync::Arc;
use tokio::sync::RwLock;

use caesar::api::stoq_api::{CaesarAppState, CaesarStoqApi, CaesarStoqConfig};
use caesar::{CaesarConfig, CaesarProtocol};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let stoq_config = CaesarStoqConfig::default();

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
            .await
            .expect("failed to create Caesar STOQ API server"),
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
