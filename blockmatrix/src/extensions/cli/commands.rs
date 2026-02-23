// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! CLI command definitions for HyperMesh Extension Management
//!
//! Contains all clap-derived command types and output format enums.

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Extension management CLI commands
#[derive(Debug, Parser)]
#[command(name = "hypermesh-extensions")]
#[command(about = "HyperMesh Extension Management", long_about = None)]
pub struct ExtensionCli {
    /// Extension management commands
    #[command(subcommand)]
    pub command: ExtensionCommand,

    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

/// Extension management commands
#[derive(Debug, Subcommand)]
pub enum ExtensionCommand {
    /// List all loaded extensions
    List {
        /// Output format (table, json, yaml)
        #[arg(short, long, default_value = "table")]
        format: OutputFormat,

        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,

        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },

    /// Load an extension
    Load {
        /// Extension path or ID
        extension: String,

        /// Force load even if already loaded
        #[arg(short, long)]
        force: bool,

        /// Skip signature verification
        #[arg(long)]
        skip_verification: bool,

        /// Configuration file for the extension
        #[arg(short, long)]
        config: Option<PathBuf>,
    },

    /// Unload an extension
    Unload {
        /// Extension ID
        extension_id: String,

        /// Force unload even if in use
        #[arg(short, long)]
        force: bool,
    },

    /// Get extension information
    Info {
        /// Extension ID
        extension_id: String,

        /// Output format
        #[arg(short, long, default_value = "table")]
        format: OutputFormat,

        /// Show resource usage
        #[arg(long)]
        show_resources: bool,

        /// Show configuration
        #[arg(long)]
        show_config: bool,
    },

    /// Reload an extension
    Reload {
        /// Extension ID
        extension_id: String,

        /// New configuration file
        #[arg(short, long)]
        config: Option<PathBuf>,
    },

    /// Pause an extension
    Pause {
        /// Extension ID
        extension_id: String,
    },

    /// Resume a paused extension
    Resume {
        /// Extension ID
        extension_id: String,
    },

    /// Install extension from marketplace
    Install {
        /// Package name or URL
        package: String,

        /// Installation directory
        #[arg(short, long)]
        directory: Option<PathBuf>,

        /// Version to install
        #[arg(short, long)]
        version: Option<String>,

        /// Include optional dependencies
        #[arg(long)]
        with_optional: bool,
    },

    /// Search for extensions in marketplace
    Search {
        /// Search query
        query: String,

        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,

        /// Maximum results
        #[arg(long, default_value = "20")]
        limit: usize,

        /// Output format
        #[arg(short, long, default_value = "table")]
        format: OutputFormat,
    },

    /// Update an installed extension
    Update {
        /// Extension ID
        extension_id: String,

        /// Target version (latest if not specified)
        #[arg(short, long)]
        version: Option<String>,

        /// Backup current version
        #[arg(long)]
        backup: bool,
    },

    /// Configure extension settings
    Config {
        /// Extension ID
        extension_id: String,

        /// Configuration subcommand
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Validate extensions
    Validate {
        /// Extension ID (all if not specified)
        extension_id: Option<String>,

        /// Output validation report
        #[arg(long)]
        report: bool,

        /// Fix issues if possible
        #[arg(long)]
        fix: bool,
    },

    /// Show extension metrics
    Metrics {
        /// Extension ID (all if not specified)
        extension_id: Option<String>,

        /// Output format
        #[arg(short, long, default_value = "table")]
        format: OutputFormat,

        /// Time range (e.g., "1h", "24h", "7d")
        #[arg(long)]
        range: Option<String>,
    },

    /// Execute extension-specific command
    Exec {
        /// Extension ID
        extension_id: String,

        /// Command to execute
        command: String,

        /// Command arguments
        args: Vec<String>,

        /// Input JSON file
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Output format
        #[arg(short, long, default_value = "json")]
        format: OutputFormat,
    },

    /// Manage extension cache
    Cache {
        /// Cache action
        #[command(subcommand)]
        action: CacheAction,
    },
}

/// Configuration sub-commands
#[derive(Debug, Subcommand)]
pub enum ConfigAction {
    /// Show current configuration
    Show,

    /// Set configuration value
    Set {
        /// Configuration key
        key: String,

        /// Configuration value
        value: String,
    },

    /// Get configuration value
    Get {
        /// Configuration key
        key: String,
    },

    /// Reset to default configuration
    Reset,

    /// Export configuration to file
    Export {
        /// Output file path
        path: PathBuf,
    },

    /// Import configuration from file
    Import {
        /// Input file path
        path: PathBuf,
    },
}

/// Cache management sub-commands
#[derive(Debug, Subcommand)]
pub enum CacheAction {
    /// Clear extension cache
    Clear {
        /// Extension ID (all if not specified)
        extension_id: Option<String>,
    },

    /// Show cache statistics
    Stats,

    /// Verify cache integrity
    Verify,

    /// Prune old cache entries
    Prune {
        /// Age threshold (e.g., "30d")
        #[arg(long, default_value = "30d")]
        older_than: String,
    },
}

/// Output format for CLI commands
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OutputFormat {
    Table,
    Json,
    Yaml,
    Text,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "table" => Ok(OutputFormat::Table),
            "json" => Ok(OutputFormat::Json),
            "yaml" => Ok(OutputFormat::Yaml),
            "text" => Ok(OutputFormat::Text),
            _ => Err(format!("Unknown output format: {}", s)),
        }
    }
}
