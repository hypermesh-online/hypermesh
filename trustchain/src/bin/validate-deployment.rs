// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! TrustChain Deployment Validation CLI
//!
//! Validates deployment readiness and security compliance.
//! Prevents deployment of systems with security theater.

use anyhow::{anyhow, Result};
use clap::{value_parser, Arg, ArgAction, Command};
use std::path::PathBuf;
use tracing::{error, info};

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
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(
            Arg::new("output-format")
                .long("format")
                .short('f')
                .help("Output format")
                .value_name("FORMAT")
                .value_parser(["human", "json"])
                .default_value("human"),
        )
        .arg(
            Arg::new("strict")
                .long("strict")
                .help("Use strict validation (fail on warnings)")
                .action(ArgAction::SetTrue),
        )
        .get_matches();

    let source_path = matches
        .get_one::<PathBuf>("source-path")
        .ok_or_else(|| anyhow!("Source path argument missing"))?;
    let _output_format = matches
        .get_one::<String>("output-format")
        .ok_or_else(|| anyhow!("Output format argument missing"))?;
    let _strict_mode = matches.get_flag("strict");

    if !source_path.exists() {
        error!("Source path does not exist: {}", source_path.display());
        std::process::exit(1);
    }

    info!(
        "Validating TrustChain deployment from: {}",
        source_path.display()
    );

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
