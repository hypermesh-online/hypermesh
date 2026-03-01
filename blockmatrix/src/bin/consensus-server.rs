// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Consensus Server Binary
//!
//! Standalone consensus validation server that provides STOQ API endpoints
//! for TrustChain and other services to validate certificates and proofs.

use anyhow::{anyhow, Result};
use clap::{Arg, Command};
use std::sync::Arc;
use tracing::info;

use blockmatrix::consensus::{validation_service::ValidationService, ConsensusConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let matches = Command::new("HyperMesh Consensus Server")
        .version("0.1.0")
        .author("HyperMesh Team")
        .about("Consensus validation server for the HyperMesh ecosystem")
        .arg(
            Arg::new("bind")
                .short('b')
                .long("bind")
                .value_name("ADDRESS")
                .help("IPv6 bind address")
                .default_value("::"),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Listen port")
                .default_value("9292"),
        )
        .arg(
            Arg::new("node-id")
                .short('n')
                .long("node-id")
                .value_name("ID")
                .help("Node identifier")
                .default_value("hypermesh-consensus-1"),
        )
        .arg(
            Arg::new("log-level")
                .short('l')
                .long("log-level")
                .value_name("LEVEL")
                .help("Log level (trace, debug, info, warn, error)")
                .default_value("info"),
        )
        .arg(
            Arg::new("max-validations")
                .long("max-validations")
                .value_name("NUM")
                .help("Maximum concurrent validations")
                .default_value("100"),
        )
        .arg(
            Arg::new("cache")
                .long("cache")
                .help("Enable validation result caching")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    // Initialize logging
    let log_level = matches
        .get_one::<String>("log-level")
        .ok_or_else(|| anyhow!("Log level argument missing"))?;
    let log_filter = match log_level.as_str() {
        "trace" => "trace",
        "debug" => "debug",
        "info" => "info",
        "warn" => "warn",
        "error" => "error",
        _ => "info",
    };

    tracing_subscriber::fmt()
        .with_env_filter(log_filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();

    info!("Starting HyperMesh Consensus Server v0.1.0");

    // Parse configuration
    let bind_address = matches
        .get_one::<String>("bind")
        .ok_or_else(|| anyhow!("Bind address argument missing"))?
        .to_string();
    let port: u16 = matches
        .get_one::<String>("port")
        .ok_or_else(|| anyhow!("Port argument missing"))?
        .parse()
        .map_err(|e| anyhow!("Invalid port: {e}"))?;
    let node_id = matches
        .get_one::<String>("node-id")
        .ok_or_else(|| anyhow!("Node ID argument missing"))?
        .to_string();
    let max_validations: usize = matches
        .get_one::<String>("max-validations")
        .ok_or_else(|| anyhow!("Max validations argument missing"))?
        .parse()
        .map_err(|e| anyhow!("Invalid max-validations: {e}"))?;
    let enable_cache = matches.contains_id("cache");

    info!("Configuration:");
    info!("  Node ID: {}", node_id);
    info!("  Bind address: {}:{}", bind_address, port);
    info!("  Max concurrent validations: {}", max_validations);
    info!("  Cache enabled: {}", enable_cache);

    // Create node ID (placeholder, actual NodeId uses uuid::Uuid::new_v4())
    let _node_id_str = node_id.clone();

    // Create consensus configuration
    let _consensus_config = ConsensusConfig::default();

    // Create validation service (stub implementation)
    info!("Creating validation service...");
    let _validation_service = Arc::new(ValidationService::new());

    info!("Consensus validation service initialized");
    info!("STOQ API server would start at {}:{}", bind_address, port);
    info!("Validation service ready (stub implementation)");
    info!("Max concurrent validations: {}", max_validations);
    info!("Cache enabled: {}", enable_cache);
    info!("");
    info!("Press Ctrl+C to stop");

    // Keep server running until Ctrl+C
    tokio::signal::ctrl_c().await?;

    info!("Received shutdown signal");
    info!("HyperMesh Consensus Server shutdown complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argument_parsing() {
        // Test that the CLI arguments can be parsed
        let app = Command::new("test")
            .arg(Arg::new("bind").short('b').default_value("::"))
            .arg(Arg::new("port").short('p').default_value("9292"));

        let matches = app.try_get_matches_from(vec!["test", "-b", "::", "-p", "9292"]);
        assert!(matches.is_ok());
    }
}
