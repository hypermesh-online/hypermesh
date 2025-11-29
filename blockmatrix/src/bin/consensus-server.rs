//! HyperMesh Consensus Server Binary
//!
//! Standalone consensus validation server that provides STOQ API endpoints
//! for TrustChain and other services to validate certificates and proofs.

use std::sync::Arc;
use anyhow::{Result, anyhow};
use clap::{Arg, Command};
use tracing::{info, warn, error};
use tracing_subscriber;

use blockmatrix::consensus::{
    ConsensusEngine,
    ConsensusValidationService,
    ConsensusConfig,
    NodeId,
};
use blockmatrix::api::consensus_api::{
    create_consensus_api_server,
    ConsensusApiConfig,
};

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
                .default_value("::")
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Listen port")
                .default_value("9292")
        )
        .arg(
            Arg::new("node-id")
                .short('n')
                .long("node-id")
                .value_name("ID")
                .help("Node identifier")
                .default_value("hypermesh-consensus-1")
        )
        .arg(
            Arg::new("log-level")
                .short('l')
                .long("log-level")
                .value_name("LEVEL")
                .help("Log level (trace, debug, info, warn, error)")
                .default_value("info")
        )
        .arg(
            Arg::new("max-validations")
                .long("max-validations")
                .value_name("NUM")
                .help("Maximum concurrent validations")
                .default_value("100")
        )
        .arg(
            Arg::new("cache")
                .long("cache")
                .help("Enable validation result caching")
                .takes_value(false)
        )
        .get_matches();

    // Initialize logging
    let log_level = matches.get_one::<String>("log-level").unwrap();
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
    let bind_address = matches.get_one::<String>("bind").unwrap().to_string();
    let port: u16 = matches.get_one::<String>("port")
        .unwrap()
        .parse()
        .map_err(|e| anyhow!("Invalid port: {}", e))?;
    let node_id = matches.get_one::<String>("node-id").unwrap().to_string();
    let max_validations: usize = matches.get_one::<String>("max-validations")
        .unwrap()
        .parse()
        .map_err(|e| anyhow!("Invalid max-validations: {}", e))?;
    let enable_cache = matches.contains_id("cache");

    info!("Configuration:");
    info!("  Node ID: {}", node_id);
    info!("  Bind address: {}:{}", bind_address, port);
    info!("  Max concurrent validations: {}", max_validations);
    info!("  Cache enabled: {}", enable_cache);

    // Create node ID
    let node_id = NodeId::new(node_id);

    // Create consensus configuration
    let consensus_config = ConsensusConfig::default();

    // Initialize consensus engine
    info!("Initializing consensus engine...");
    let consensus_engine = Arc::new(
        ConsensusEngine::new(node_id.clone(), consensus_config)
            .await
            .map_err(|e| anyhow!("Failed to create consensus engine: {}", e))?
    );

    // Create validation service
    info!("Creating validation service...");
    let validation_service = Arc::new(
        ConsensusValidationService::new(
            consensus_engine,
            node_id,
            Default::default(),
        )
        .await
        .map_err(|e| anyhow!("Failed to create validation service: {}", e))?
    );

    // Create API configuration
    let api_config = ConsensusApiConfig {
        bind_address,
        port,
        max_concurrent_validations: max_validations,
        enable_logging: true,
        enable_cache,
    };

    // Create and start API server
    info!("Starting STOQ API server...");
    let api_server = create_consensus_api_server(validation_service, api_config).await?;

    info!("HyperMesh Consensus Server is ready");
    info!("Accepting validation requests on port {}", port);
    info!("Press Ctrl-C to stop");

    // Handle shutdown signal
    let api_server_clone = api_server.clone();
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                info!("Received shutdown signal");
                api_server_clone.stop();
            }
            Err(e) => {
                error!("Failed to listen for shutdown signal: {}", e);
            }
        }
    });

    // Start listening for API requests
    match api_server.listen().await {
        Ok(()) => {
            info!("API server stopped gracefully");
        }
        Err(e) => {
            error!("API server error: {}", e);
            return Err(anyhow!("Server failed: {}", e));
        }
    }

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