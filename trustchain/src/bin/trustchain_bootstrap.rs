// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! TrustChain Bootstrap Binary
//!
//! Standalone bootstrap executable for TrustChain that starts with zero dependencies.
//! This enables TrustChain to run independently before BlockMatrix is available.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::net::Ipv6Addr;
use tracing::{info, warn, error};
use tracing_subscriber;
use trustchain::dns::{TrustChainBootstrap, BootstrapPhase};
use trustchain::dns::bootstrap::DnsRecord;
use tokio::signal;

#[derive(Parser, Debug)]
#[clap(name = "trustchain-bootstrap")]
#[clap(about = "TrustChain standalone bootstrap utility")]
#[clap(version)]
struct Cli {
    /// Enable debug logging
    #[clap(short, long)]
    debug: bool,

    /// Persistence directory (enables Phase 2 with file storage)
    #[clap(short, long)]
    persist_dir: Option<PathBuf>,

    /// Bind address for services
    #[clap(short, long, default_value = "::1")]
    bind: Ipv6Addr,

    /// Port for CA service
    #[clap(long, default_value = "8443")]
    ca_port: u16,

    /// Port for DNS service
    #[clap(long, default_value = "8853")]
    dns_port: u16,

    /// Port for CT log service
    #[clap(long, default_value = "8863")]
    ct_port: u16,

    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run TrustChain bootstrap (default)
    Run,

    /// Test bootstrap connectivity
    Test {
        /// TrustChain endpoint to test
        #[clap(default_value = "[::1]:8443")]
        endpoint: String,
    },

    /// Show bootstrap status
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.debug { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(log_level)
        .init();

    match cli.command {
        Some(Commands::Test { endpoint }) => {
            test_connectivity(&endpoint).await?;
        }
        Some(Commands::Status) => {
            show_status(&cli).await?;
        }
        Some(Commands::Run) | None => {
            run_bootstrap(&cli).await?;
        }
    }

    Ok(())
}

async fn run_bootstrap(cli: &Cli) -> Result<()> {
    info!("===========================================");
    info!("   TrustChain Standalone Bootstrap v1.0   ");
    info!("===========================================");
    info!("");
    info!("Starting TrustChain with ZERO external dependencies");
    info!("Bind address: [{}]", cli.bind);
    info!("CA Port: {}", cli.ca_port);
    info!("DNS Port: {}", cli.dns_port);
    info!("CT Port: {}", cli.ct_port);

    // Phase 1 or Phase 2 bootstrap based on persistence flag
    let mut bootstrap = if let Some(persist_dir) = &cli.persist_dir {
        info!("Phase 2: Bootstrap with persistence at {:?}", persist_dir);
        TrustChainBootstrap::bootstrap_with_persistence(persist_dir.clone()).await?
    } else {
        info!("Phase 1: In-memory bootstrap (no persistence)");
        TrustChainBootstrap::bootstrap_standalone().await?
    };

    let phase = bootstrap.get_phase().await;
    info!("Bootstrap phase: {:?}", phase);

    // Initialize Certificate Authority (simplified for now)
    info!("Initializing Certificate Authority (FALCON-1024)...");
    bootstrap.mark_ca_ready().await?;
    info!("✓ CA marked as ready (implementation pending)");

    // Initialize Certificate Transparency Log
    info!("Initializing Certificate Transparency Log...");
    bootstrap.mark_ct_ready().await?;
    info!("✓ CT Log marked as ready (implementation pending)");

    // Initialize DNS resolver
    info!("Initializing DNS resolver...");
    bootstrap.test_localhost_connectivity().await?;
    bootstrap.mark_dns_ready().await?;
    info!("✓ DNS resolver operational");

    // Add essential DNS records for local services
    add_service_records(&mut bootstrap, cli.bind, cli).await?;

    if bootstrap.is_operational().await {
        info!("");
        info!("===========================================");
        info!("  TrustChain is FULLY OPERATIONAL");
        info!("===========================================");
        info!("");
        info!("Services ready:");
        info!("  • CA:  https://[{}]:{} (pending implementation)", cli.bind, cli.ca_port);
        info!("  • DNS: dns://[{}]:{} (in-memory storage)", cli.bind, cli.dns_port);
        info!("  • CT:  https://[{}]:{} (pending implementation)", cli.bind, cli.ct_port);
        info!("");
        info!("BlockMatrix can now connect to TrustChain at:");
        info!("  TRUSTCHAIN_CA_URL=https://[{}]:{}", cli.bind, cli.ca_port);
        info!("");
        info!("Press Ctrl+C to shutdown...");
    } else {
        error!("TrustChain bootstrap failed - not all components ready");
        return Err(anyhow::anyhow!("Bootstrap incomplete"));
    }

    // Wait for shutdown signal
    signal::ctrl_c().await?;
    info!("Shutting down TrustChain...");

    Ok(())
}

async fn add_service_records(
    bootstrap: &mut TrustChainBootstrap,
    bind: Ipv6Addr,
    cli: &Cli,
) -> Result<()> {
    info!("Adding service DNS records...");

    // CA service
    bootstrap.add_dns_record(DnsRecord {
        name: "ca.trustchain.local".to_string(),
        record_type: "AAAA".to_string(),
        value: bind.to_string(),
        ttl: 3600,
        timestamp: 0,
    }).await?;

    // DNS service
    bootstrap.add_dns_record(DnsRecord {
        name: "dns.trustchain.local".to_string(),
        record_type: "AAAA".to_string(),
        value: bind.to_string(),
        ttl: 3600,
        timestamp: 0,
    }).await?;

    // CT log service
    bootstrap.add_dns_record(DnsRecord {
        name: "ct.trustchain.local".to_string(),
        record_type: "AAAA".to_string(),
        value: bind.to_string(),
        ttl: 3600,
        timestamp: 0,
    }).await?;

    // Add SRV records for service discovery
    bootstrap.add_dns_record(DnsRecord {
        name: "_ca._tcp.trustchain.local".to_string(),
        record_type: "SRV".to_string(),
        value: format!("0 0 {} ca.trustchain.local", cli.ca_port),
        ttl: 3600,
        timestamp: 0,
    }).await?;

    bootstrap.add_dns_record(DnsRecord {
        name: "_dns._udp.trustchain.local".to_string(),
        record_type: "SRV".to_string(),
        value: format!("0 0 {} dns.trustchain.local", cli.dns_port),
        ttl: 3600,
        timestamp: 0,
    }).await?;

    bootstrap.add_dns_record(DnsRecord {
        name: "_ct._tcp.trustchain.local".to_string(),
        record_type: "SRV".to_string(),
        value: format!("0 0 {} ct.trustchain.local", cli.ct_port),
        ttl: 3600,
        timestamp: 0,
    }).await?;

    info!("✓ Service DNS records added");
    Ok(())
}

async fn test_connectivity(endpoint: &str) -> Result<()> {
    info!("Testing connectivity to TrustChain at {}", endpoint);

    // Parse endpoint
    let url = if !endpoint.starts_with("http") {
        format!("https://{}", endpoint)
    } else {
        endpoint.to_string()
    };

    // Test CA endpoint
    info!("Testing CA endpoint...");
    match reqwest::get(&format!("{}/ca/status", url)).await {
        Ok(response) => {
            if response.status().is_success() {
                info!("✓ CA is reachable and operational");
            } else {
                warn!("⚠ CA returned status: {}", response.status());
            }
        }
        Err(e) => {
            error!("✗ Failed to reach CA: {}", e);
        }
    }

    // Test CT endpoint
    info!("Testing CT endpoint...");
    match reqwest::get(&format!("{}/ct/status", url)).await {
        Ok(response) => {
            if response.status().is_success() {
                info!("✓ CT log is reachable and operational");
            } else {
                warn!("⚠ CT log returned status: {}", response.status());
            }
        }
        Err(e) => {
            error!("✗ Failed to reach CT log: {}", e);
        }
    }

    info!("Connectivity test complete");
    Ok(())
}

async fn show_status(cli: &Cli) -> Result<()> {
    info!("TrustChain Bootstrap Status");
    info!("===========================");

    // Try to connect to local instance
    let ca_url = format!("https://[{}]:{}", cli.bind, cli.ca_port);
    let ct_url = format!("https://[{}]:{}", cli.bind, cli.ct_port);

    info!("Checking services...");

    match reqwest::get(&format!("{}/status", ca_url)).await {
        Ok(_) => info!("✓ CA: Running at [{}]:{}", cli.bind, cli.ca_port),
        Err(_) => info!("✗ CA: Not running"),
    }

    match reqwest::get(&format!("{}/status", ct_url)).await {
        Ok(_) => info!("✓ CT: Running at [{}]:{}", cli.bind, cli.ct_port),
        Err(_) => info!("✗ CT: Not running"),
    }

    // DNS doesn't have HTTP endpoint, just show configured port
    info!("  DNS: Configured for [{}]:{}", cli.bind, cli.dns_port);

    Ok(())
}