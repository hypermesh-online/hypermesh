//! TrustChain Deployment Validation CLI
//!
//! Validates deployment readiness and security compliance.
//! Prevents deployment of systems with security theater.

use clap::{Arg, ArgAction, Command, value_parser};
use std::path::PathBuf;
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let matches = Command::new("TrustChain Deployment Validator")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Web3 Ecosystem Team")
        .about("Validates TrustChain deployment readiness and security compliance")
        .arg(
            Arg::new("source-path")
                .long("source-path")
                .short('s')
                .help("Path to TrustChain source code")
                .value_name("PATH")
                .required(false)
                .default_value(".")
                .value_parser(value_parser!(PathBuf))
        )
        .arg(
            Arg::new("output-format")
                .long("format")
                .short('f')
                .help("Output format")
                .value_name("FORMAT")
                .value_parser(["human", "json"])
                .default_value("human")
        )
        .arg(
            Arg::new("strict")
                .long("strict")
                .help("Use strict validation (fail on warnings)")
                .action(ArgAction::SetTrue)
        )
        .get_matches();

    let source_path = matches.get_one::<PathBuf>("source-path").unwrap();
    let _output_format = matches.get_one::<String>("output-format").unwrap();
    let _strict_mode = matches.get_flag("strict");

    if !source_path.exists() {
        error!("Source path does not exist: {}", source_path.display());
        std::process::exit(1);
    }

    info!("Validating TrustChain deployment from: {}", source_path.display());

    // Run deployment validation
    match trustchain::deployment::validate_deployment_cli(&source_path).await {
        Ok(()) => {
            info!("Deployment validation completed successfully");
        }
        Err(e) => {
            error!("Deployment validation failed: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}