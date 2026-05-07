// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! TrustChain STOQ Server - Pure STOQ transport, no HTTP
//!
//! This server provides TrustChain services over STOQ protocol (QUIC/IPv6).
//! No HTTP dependencies - pure STOQ transport for all certificate operations.

use anyhow::Result;
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info};

use trustchain::{
    api::stoq_api::{TrustChainStoqApi, TrustChainStoqConfig},
    ca::{certificate_store::CertificateStore, TrustChainCA},
    config::TrustChainConfig,
    dns::DnsResolver,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Install rustls crypto provider
    rustls::crypto::ring::default_provider()
        .install_default()
        .ok(); // Ignore error if already installed

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,trustchain=debug,stoq=debug".into()),
        )
        .init();

    info!("Starting TrustChain STOQ Server (Pure STOQ, No HTTP)");
    info!("Protocol: STOQ (QUIC over IPv6)");
    info!("Transport: End-to-end encrypted QUIC");

    // Load configuration (env var -> ~/.hypermesh/trustchain.toml -> /etc -> defaults)
    let (tc_config, config_source) = TrustChainConfig::load()?;
    match &config_source {
        Some(path) => info!("Configuration loaded from: {}", path),
        None => info!("Using default configuration (no config file found)"),
    }

    // Initialize TrustChain CA
    info!("Initializing TrustChain Certificate Authority...");
    let ca = Arc::new(TrustChainCA::new(tc_config.ca.clone()).await?);

    // Initialize DNS resolver
    info!("Initializing DNS resolver...");
    let resolver = Arc::new(DnsResolver::new(tc_config.dns.clone()).await?);

    // Configure STOQ API
    let stoq_bind = format!("[::1]:{}", tc_config.api.port);
    let config = TrustChainStoqConfig {
        bind_address: stoq_bind.clone(),
        service_name: "trustchain".to_string(),
        enable_logging: true,
    };

    // Create STOQ API server
    info!("Creating TrustChain STOQ API server...");
    let api = Arc::new(TrustChainStoqApi::new(ca, resolver, config).await?);

    // Phase F.2 — register CRL lookup handler so federation peers can
    // query our revocation list over STOQ (mirror of TAG_CRL_REQUEST 0x33).
    let cert_store = Arc::new(CertificateStore::new().await?);
    api.register_crl_handler(cert_store);

    info!("TrustChain STOQ server ready");
    info!("Listening on stoq://{}", stoq_bind);
    info!("Available STOQ endpoints:");
    info!("  - stoq://{}/trustchain/health", stoq_bind);
    info!("  - stoq://{}/trustchain/validate_certificate", stoq_bind);
    info!("  - stoq://{}/trustchain/issue_certificate", stoq_bind);
    info!("  - stoq://{}/trustchain/resolve_dns", stoq_bind);
    info!("  - stoq://{}/trustchain/crl/lookup", stoq_bind);
    info!("Press Ctrl+C to shutdown gracefully");

    // Start server with graceful shutdown
    let api_handle = api.clone();
    let server_task = tokio::spawn(async move {
        if let Err(e) = api_handle.serve().await {
            error!("STOQ server error: {}", e);
        }
    });

    // Wait for shutdown signal
    shutdown_signal().await;

    info!("Shutting down TrustChain STOQ server...");
    api.stop();

    // Wait for server task to complete (5s timeout)
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), server_task).await;

    info!("TrustChain STOQ server shutdown complete");

    Ok(())
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal");
        },
        _ = terminate => {
            info!("Received terminate signal");
        },
    }
}
