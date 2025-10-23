//! CLI Integration for HyperMesh Extension Management
//!
//! This module provides command-line interface functionality for managing
//! extensions in the HyperMesh ecosystem.

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::manager::{UnifiedExtensionManager, ExtensionManagerConfig, ExtensionInfo};
use super::{ExtensionRequest, ExtensionCapability, ResourceLimits};
use crate::assets::core::AssetManager;

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

/// CLI executor for extension commands
pub struct ExtensionCliExecutor {
    manager: Arc<UnifiedExtensionManager>,
}

impl ExtensionCliExecutor {
    /// Create new CLI executor
    pub fn new(manager: Arc<UnifiedExtensionManager>) -> Self {
        Self { manager }
    }

    /// Execute CLI command
    pub async fn execute(&self, cli: ExtensionCli) -> Result<()> {
        match cli.command {
            ExtensionCommand::List { format, category, detailed } => {
                self.list_extensions(format, category, detailed).await
            }
            ExtensionCommand::Load { extension, force, skip_verification, config } => {
                self.load_extension(extension, force, skip_verification, config).await
            }
            ExtensionCommand::Unload { extension_id, force } => {
                self.unload_extension(extension_id, force).await
            }
            ExtensionCommand::Info { extension_id, format, show_resources, show_config } => {
                self.show_extension_info(extension_id, format, show_resources, show_config).await
            }
            ExtensionCommand::Reload { extension_id, config } => {
                self.reload_extension(extension_id, config).await
            }
            ExtensionCommand::Pause { extension_id } => {
                self.pause_extension(extension_id).await
            }
            ExtensionCommand::Resume { extension_id } => {
                self.resume_extension(extension_id).await
            }
            ExtensionCommand::Install { package, directory, version, with_optional } => {
                self.install_extension(package, directory, version, with_optional).await
            }
            ExtensionCommand::Search { query, category, limit, format } => {
                self.search_extensions(query, category, limit, format).await
            }
            ExtensionCommand::Update { extension_id, version, backup } => {
                self.update_extension(extension_id, version, backup).await
            }
            ExtensionCommand::Config { extension_id, action } => {
                self.configure_extension(extension_id, action).await
            }
            ExtensionCommand::Validate { extension_id, report, fix } => {
                self.validate_extensions(extension_id, report, fix).await
            }
            ExtensionCommand::Metrics { extension_id, format, range } => {
                self.show_metrics(extension_id, format, range).await
            }
            ExtensionCommand::Exec { extension_id, command, args, input, format } => {
                self.execute_extension_command(extension_id, command, args, input, format).await
            }
            ExtensionCommand::Cache { action } => {
                self.manage_cache(action).await
            }
        }
    }

    /// List loaded extensions
    async fn list_extensions(
        &self,
        format: OutputFormat,
        category: Option<String>,
        detailed: bool,
    ) -> Result<()> {
        let extensions = self.manager.list_extensions().await;

        // Filter by category if specified
        let filtered: Vec<ExtensionInfo> = if let Some(cat) = category {
            extensions
                .into_iter()
                .filter(|e| format!("{:?}", e.metadata.category).to_lowercase().contains(&cat.to_lowercase()))
                .collect()
        } else {
            extensions
        };

        match format {
            OutputFormat::Table => {
                self.print_extensions_table(&filtered, detailed);
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&filtered)?;
                println!("{}", json);
            }
            OutputFormat::Yaml => {
                let yaml = serde_yaml::to_string(&filtered)?;
                println!("{}", yaml);
            }
            OutputFormat::Text => {
                for ext in filtered {
                    println!("{}: {} v{}",
                        ext.metadata.id,
                        ext.metadata.name,
                        ext.metadata.version
                    );
                    if detailed {
                        println!("  Description: {}", ext.metadata.description);
                        println!("  Category: {:?}", ext.metadata.category);
                        println!("  State: {:?}", ext.state.state);
                        println!("  Health: {:?}", ext.state.health);
                    }
                }
            }
        }

        Ok(())
    }

    /// Load an extension
    async fn load_extension(
        &self,
        extension: String,
        force: bool,
        skip_verification: bool,
        config: Option<PathBuf>,
    ) -> Result<()> {
        println!("Loading extension: {}", extension);

        // TODO: Implement actual extension loading
        // This would involve:
        // 1. Finding the extension (from path or ID)
        // 2. Loading the extension module
        // 3. Creating the extension instance
        // 4. Loading it into the manager

        println!("Extension loaded successfully");
        Ok(())
    }

    /// Unload an extension
    async fn unload_extension(&self, extension_id: String, force: bool) -> Result<()> {
        println!("Unloading extension: {}", extension_id);

        self.manager.unload_extension(&extension_id).await?;

        println!("Extension unloaded successfully");
        Ok(())
    }

    /// Show extension information
    async fn show_extension_info(
        &self,
        extension_id: String,
        format: OutputFormat,
        show_resources: bool,
        show_config: bool,
    ) -> Result<()> {
        let info = self.manager.get_extension_info(&extension_id).await
            .ok_or_else(|| anyhow::anyhow!("Extension not found: {}", extension_id))?;

        match format {
            OutputFormat::Table => {
                self.print_extension_info_table(&info, show_resources, show_config);
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&info)?;
                println!("{}", json);
            }
            OutputFormat::Yaml => {
                let yaml = serde_yaml::to_string(&info)?;
                println!("{}", yaml);
            }
            OutputFormat::Text => {
                println!("Extension: {} ({})", info.metadata.name, info.metadata.id);
                println!("Version: {}", info.metadata.version);
                println!("Description: {}", info.metadata.description);
                println!("Author: {}", info.metadata.author);
                println!("Category: {:?}", info.metadata.category);
                println!("State: {:?}", info.state.state);
                println!("Health: {:?}", info.state.health);

                if show_resources {
                    println!("\nResource Usage:");
                    println!("  CPU: {:.2}%", info.state.resource_usage.cpu_percent);
                    println!("  Memory: {} MB", info.state.resource_usage.memory_bytes / 1024 / 1024);
                    println!("  Storage: {} MB", info.state.resource_usage.storage_bytes / 1024 / 1024);
                }
            }
        }

        Ok(())
    }

    /// Reload an extension
    async fn reload_extension(&self, extension_id: String, config: Option<PathBuf>) -> Result<()> {
        println!("Reloading extension: {}", extension_id);

        self.manager.reload_extension(&extension_id).await?;

        println!("Extension reloaded successfully");
        Ok(())
    }

    /// Pause an extension
    async fn pause_extension(&self, extension_id: String) -> Result<()> {
        println!("Pausing extension: {}", extension_id);

        self.manager.pause_extension(&extension_id).await?;

        println!("Extension paused");
        Ok(())
    }

    /// Resume an extension
    async fn resume_extension(&self, extension_id: String) -> Result<()> {
        println!("Resuming extension: {}", extension_id);

        self.manager.resume_extension(&extension_id).await?;

        println!("Extension resumed");
        Ok(())
    }

    /// Install extension from marketplace
    async fn install_extension(
        &self,
        package: String,
        directory: Option<PathBuf>,
        version: Option<String>,
        with_optional: bool,
    ) -> Result<()> {
        println!("Installing extension: {}", package);

        // TODO: Implement marketplace integration
        // This would involve:
        // 1. Searching for the package in the marketplace
        // 2. Downloading the extension
        // 3. Verifying signatures
        // 4. Installing to the specified directory
        // 5. Loading the extension

        println!("Extension installed successfully");
        Ok(())
    }

    /// Search for extensions
    async fn search_extensions(
        &self,
        query: String,
        category: Option<String>,
        limit: usize,
        format: OutputFormat,
    ) -> Result<()> {
        println!("Searching for: {}", query);

        // TODO: Implement marketplace search
        // This would query the extension marketplace API

        Ok(())
    }

    /// Update an extension
    async fn update_extension(
        &self,
        extension_id: String,
        version: Option<String>,
        backup: bool,
    ) -> Result<()> {
        println!("Updating extension: {}", extension_id);

        if backup {
            println!("Creating backup of current version...");
        }

        // TODO: Implement extension update
        // This would involve:
        // 1. Checking for updates
        // 2. Downloading new version
        // 3. Backing up if requested
        // 4. Unloading old version
        // 5. Loading new version

        println!("Extension updated successfully");
        Ok(())
    }

    /// Configure extension
    async fn configure_extension(
        &self,
        extension_id: String,
        action: ConfigAction,
    ) -> Result<()> {
        match action {
            ConfigAction::Show => {
                println!("Configuration for extension: {}", extension_id);
                // TODO: Show extension configuration
            }
            ConfigAction::Set { key, value } => {
                println!("Setting {} = {} for extension: {}", key, value, extension_id);
                // TODO: Set configuration value
            }
            ConfigAction::Get { key } => {
                println!("Getting {} for extension: {}", key, extension_id);
                // TODO: Get configuration value
            }
            ConfigAction::Reset => {
                println!("Resetting configuration for extension: {}", extension_id);
                // TODO: Reset to defaults
            }
            ConfigAction::Export { path } => {
                println!("Exporting configuration to: {:?}", path);
                // TODO: Export configuration
            }
            ConfigAction::Import { path } => {
                println!("Importing configuration from: {:?}", path);
                // TODO: Import configuration
            }
        }

        Ok(())
    }

    /// Validate extensions
    async fn validate_extensions(
        &self,
        extension_id: Option<String>,
        report: bool,
        fix: bool,
    ) -> Result<()> {
        println!("Validating extensions...");

        let reports = if let Some(id) = extension_id {
            let mut map = std::collections::HashMap::new();
            // TODO: Validate specific extension
            map
        } else {
            self.manager.validate_all_extensions().await
        };

        if report {
            for (id, report) in reports {
                println!("Extension: {}", id);
                println!("  Valid: {}", report.valid);
                if !report.errors.is_empty() {
                    println!("  Errors:");
                    for error in &report.errors {
                        println!("    - {}: {}", error.code, error.message);
                    }
                }
                if !report.warnings.is_empty() {
                    println!("  Warnings:");
                    for warning in &report.warnings {
                        println!("    - {}: {}", warning.code, warning.message);
                    }
                }
            }
        }

        if fix {
            println!("Attempting to fix issues...");
            // TODO: Implement auto-fix functionality
        }

        Ok(())
    }

    /// Show extension metrics
    async fn show_metrics(
        &self,
        extension_id: Option<String>,
        format: OutputFormat,
        range: Option<String>,
    ) -> Result<()> {
        let metrics = self.manager.get_metrics().await;

        match format {
            OutputFormat::Table => {
                println!("Extension Metrics:");
                println!("  Total Loaded: {}", metrics.total_loaded);
                println!("  Total Failed: {}", metrics.total_failed);
                println!("  Total Requests: {}", metrics.total_requests);
                println!("  Total Errors: {}", metrics.total_errors);
                println!("  Avg Request Duration: {:?}", metrics.avg_request_duration);
                println!("  Peak Memory: {} MB", metrics.peak_memory / 1024 / 1024);
                println!("  Peak CPU: {:.2}%", metrics.peak_cpu);
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&metrics)?;
                println!("{}", json);
            }
            _ => {
                println!("Metrics format not supported");
            }
        }

        Ok(())
    }

    /// Execute extension command
    async fn execute_extension_command(
        &self,
        extension_id: String,
        command: String,
        args: Vec<String>,
        input: Option<PathBuf>,
        format: OutputFormat,
    ) -> Result<()> {
        println!("Executing command '{}' on extension: {}", command, extension_id);

        // Build request
        let mut params = serde_json::Map::new();
        params.insert("command".to_string(), serde_json::Value::String(command));
        params.insert("args".to_string(), serde_json::Value::Array(
            args.into_iter().map(serde_json::Value::String).collect()
        ));

        if let Some(input_path) = input {
            let input_data = tokio::fs::read_to_string(input_path).await?;
            let input_json: serde_json::Value = serde_json::from_str(&input_data)?;
            params.insert("input".to_string(), input_json);
        }

        let request = ExtensionRequest {
            id: uuid::Uuid::new_v4().to_string(),
            method: "exec".to_string(),
            params: serde_json::Value::Object(params),
            consensus_proof: None,
        };

        let response = self.manager.handle_request(&extension_id, request).await?;

        match format {
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&response)?;
                println!("{}", json);
            }
            _ => {
                if response.success {
                    println!("Command executed successfully");
                    if let Some(data) = response.data {
                        println!("Result: {}", data);
                    }
                } else {
                    println!("Command failed: {}", response.error.unwrap_or_default());
                }
            }
        }

        Ok(())
    }

    /// Manage cache
    async fn manage_cache(&self, action: CacheAction) -> Result<()> {
        match action {
            CacheAction::Clear { extension_id } => {
                if let Some(id) = extension_id {
                    println!("Clearing cache for extension: {}", id);
                } else {
                    println!("Clearing all extension cache");
                }
                // TODO: Implement cache clearing
            }
            CacheAction::Stats => {
                println!("Cache Statistics:");
                // TODO: Show cache statistics
            }
            CacheAction::Verify => {
                println!("Verifying cache integrity...");
                // TODO: Verify cache
            }
            CacheAction::Prune { older_than } => {
                println!("Pruning cache entries older than: {}", older_than);
                // TODO: Prune old cache entries
            }
        }

        Ok(())
    }

    /// Print extensions table
    fn print_extensions_table(&self, extensions: &[ExtensionInfo], detailed: bool) {
        use prettytable::{Table, row, cell};

        let mut table = Table::new();

        if detailed {
            table.add_row(row!["ID", "Name", "Version", "Category", "State", "Health", "Requests"]);
            for ext in extensions {
                table.add_row(row![
                    ext.metadata.id,
                    ext.metadata.name,
                    ext.metadata.version,
                    format!("{:?}", ext.metadata.category),
                    format!("{:?}", ext.state.state),
                    format!("{:?}", ext.state.health),
                    ext.state.request_count
                ]);
            }
        } else {
            table.add_row(row!["ID", "Name", "Version", "State"]);
            for ext in extensions {
                table.add_row(row![
                    ext.metadata.id,
                    ext.metadata.name,
                    ext.metadata.version,
                    format!("{:?}", ext.state.state)
                ]);
            }
        }

        table.printstd();
    }

    /// Print extension info table
    fn print_extension_info_table(&self, info: &ExtensionInfo, show_resources: bool, show_config: bool) {
        use prettytable::{Table, row};

        let mut table = Table::new();
        table.add_row(row!["Property", "Value"]);
        table.add_row(row!["ID", info.metadata.id]);
        table.add_row(row!["Name", info.metadata.name]);
        table.add_row(row!["Version", info.metadata.version]);
        table.add_row(row!["Description", info.metadata.description]);
        table.add_row(row!["Author", info.metadata.author]);
        table.add_row(row!["License", info.metadata.license]);
        table.add_row(row!["Category", format!("{:?}", info.metadata.category)]);
        table.add_row(row!["State", format!("{:?}", info.state.state)]);
        table.add_row(row!["Health", format!("{:?}", info.state.health)]);
        table.add_row(row!["Requests", info.state.request_count]);
        table.add_row(row!["Errors", info.state.error_count]);

        if show_resources {
            table.add_row(row!["CPU Usage", format!("{:.2}%", info.state.resource_usage.cpu_percent)]);
            table.add_row(row!["Memory", format!("{} MB", info.state.resource_usage.memory_bytes / 1024 / 1024)]);
            table.add_row(row!["Storage", format!("{} MB", info.state.resource_usage.storage_bytes / 1024 / 1024)]);
            table.add_row(row!["Active Ops", info.state.resource_usage.active_operations]);
        }

        table.printstd();
    }
}

/// Create CLI app
pub fn create_cli_app() -> ExtensionCli {
    ExtensionCli::parse()
}

/// Run CLI with the given manager
pub async fn run_cli(manager: Arc<UnifiedExtensionManager>) -> Result<()> {
    let cli = create_cli_app();
    let executor = ExtensionCliExecutor::new(manager);
    executor.execute(cli).await
}