//! TrustChain STOQ Server - Pure STOQ transport, no HTTP
//!
//! This server provides TrustChain services over STOQ protocol (QUIC/IPv6).
//! No HTTP dependencies - pure STOQ transport for all certificate operations.

use std::sync::Arc;
use anyhow::Result;
use tracing::{info, error, warn};
use tokio::signal;

use trustchain::{
    ca::{TrustChainCA, CAConfig, CAMode},
    consensus::{ConsensusRequirements, HyperMeshClientConfig},
    dns::DnsResolver,
    config::DnsConfig,
    api::stoq_api::{TrustChainStoqApi, TrustChainStoqConfig},
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,trustchain=debug,stoq=debug".into())
        )
        .init();

    info!("🚀 Starting TrustChain STOQ Server (Pure STOQ, No HTTP)");
    info!("📡 Protocol: STOQ (QUIC over IPv6)");
    info!("🔒 Transport: End-to-end encrypted QUIC");

    // Initialize TrustChain CA
    info!("Initializing TrustChain Certificate Authority...");
    let ca_config = CAConfig {
        ca_id: "trustchain-stoq-ca".to_string(),
        bind_address: std::net::Ipv6Addr::LOCALHOST,
        port: 9294,
        cert_validity_days: 365,
        rotation_interval: std::time::Duration::from_secs(30 * 24 * 60 * 60), // 30 days
        mode: CAMode::Production,
        consensus_requirements: ConsensusRequirements {
            minimum_stake: 1000,
            max_time_offset: std::time::Duration::from_secs(5),
            minimum_storage: 1000000, // 1MB minimum
            minimum_compute: 100,
            byzantine_tolerance: 0.33,
        },
        hypermesh_client_config: HyperMeshClientConfig {
            request_timeout: std::time::Duration::from_secs(10),
            max_retries: 3,
            retry_backoff: std::time::Duration::from_secs(1),
            enable_caching: true,
            cache_ttl: std::time::Duration::from_secs(300),
        },
    };
    let ca = Arc::new(TrustChainCA::new(ca_config).await?);

    // Initialize DNS resolver
    info!("Initializing DNS resolver...");
    let dns_config = DnsConfig {
        server_id: "trustchain-dns".to_string(),
        bind_address: std::net::Ipv6Addr::LOCALHOST,
        quic_port: 9295,
        port: 53,
        dns_port: None, // IPv6-only, no legacy DNS
        upstream_resolvers: vec![
            std::net::Ipv6Addr::from([0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888]), // Google
            std::net::Ipv6Addr::from([0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1111]), // Cloudflare
        ],
        cache_ttl: std::time::Duration::from_secs(300),
        enable_cert_validation: true,
        trustchain_domains: vec![
            "hypermesh".to_string(),
            "caesar".to_string(),
            "trust".to_string(),
            "assets".to_string(),
        ],
        consensus_requirements: ConsensusRequirements {
            minimum_stake: 500,
            max_time_offset: std::time::Duration::from_secs(3),
            minimum_storage: 500000, // 500KB minimum for DNS
            minimum_compute: 50,
            byzantine_tolerance: 0.33,
        },
    };
    let resolver = Arc::new(DnsResolver::new(dns_config).await?);

    // Configure STOQ API
    let config = TrustChainStoqConfig {
        bind_address: "[::1]:9293".to_string(), // TrustChain default STOQ port
        service_name: "trustchain".to_string(),
        enable_logging: true,
    };

    // Create STOQ API server
    info!("Creating TrustChain STOQ API server...");
    let api = Arc::new(TrustChainStoqApi::new(ca, resolver, config).await?);

    info!("✅ TrustChain STOQ server ready");
    info!("🎯 Listening on stoq://[::1]:9293");
    info!("");
    info!("Available STOQ endpoints:");
    info!("  - stoq://[::1]:9293/trustchain/health");
    info!("  - stoq://[::1]:9293/trustchain/validate_certificate");
    info!("  - stoq://[::1]:9293/trustchain/issue_certificate");
    info!("  - stoq://[::1]:9293/trustchain/resolve_dns");
    info!("");
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

    info!("⏸️  Shutting down TrustChain STOQ server...");
    api.stop();

    // Wait for server task to complete
    let _ = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        server_task
    ).await;

    info!("👋 TrustChain STOQ server shutdown complete");

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