//! Configuration management and CLI settings

use anyhow::Result;
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Show current configuration
    Show {
        /// Show global configuration
        #[arg(short, long)]
        global: bool,
    },
    
    /// Set configuration value
    Set {
        /// Configuration key (e.g., api_url, token)
        key: String,
        
        /// Configuration value
        value: String,
        
        /// Set in global configuration
        #[arg(short, long)]
        global: bool,
    },
    
    /// Get configuration value
    Get {
        /// Configuration key
        key: String,
    },
    
    /// Initialize configuration file
    Init {
        /// Force overwrite existing configuration
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NexusConfig {
    pub api_url: Option<String>,
    pub token: Option<String>,
    pub default_cluster: Option<String>,
    pub output_format: Option<String>,
    pub timeout_seconds: Option<u64>,
    pub verify_tls: Option<bool>,
}

impl Default for NexusConfig {
    fn default() -> Self {
        Self {
            api_url: Some("https://localhost:8443".to_string()),
            token: None,
            default_cluster: None,
            output_format: Some("table".to_string()),
            timeout_seconds: Some(30),
            verify_tls: Some(true),
        }
    }
}

pub fn load_config(config_path: Option<&Path>) -> Result<NexusConfig> {
    let config_file = if let Some(path) = config_path {
        path.to_path_buf()
    } else {
        get_default_config_path()?
    };

    if !config_file.exists() {
        return Ok(NexusConfig::default());
    }

    let content = std::fs::read_to_string(&config_file)?;
    let config: NexusConfig = toml::from_str(&content)?;
    Ok(config)
}

pub fn save_config(config: &NexusConfig, config_path: Option<&Path>) -> Result<()> {
    let config_file = if let Some(path) = config_path {
        path.to_path_buf()
    } else {
        get_default_config_path()?
    };

    // Create parent directories if they don't exist
    if let Some(parent) = config_file.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let content = toml::to_string_pretty(config)?;
    std::fs::write(&config_file, content)?;
    Ok(())
}

pub async fn execute_command(
    command: ConfigCommand,
    output_format: &str,
) -> Result<()> {
    match command {
        ConfigCommand::Show { global } => {
            show_config(global, output_format).await
        },

        ConfigCommand::Set { key, value, global } => {
            set_config(&key, &value, global).await
        },

        ConfigCommand::Get { key } => {
            get_config(&key).await
        },

        ConfigCommand::Init { force } => {
            init_config(force).await
        },
    }
}

async fn show_config(global: bool, output_format: &str) -> Result<()> {
    use colored::*;
    
    let config = load_config(None)?;
    
    println!("{}", "Current Configuration".bright_blue().bold());
    println!("{}", "====================".bright_blue());
    println!();

    match output_format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&config)?);
        },
        "yaml" => {
            println!("{}", serde_yaml::to_string(&config)?);
        },
        _ => {
            // Table format
            if let Some(api_url) = &config.api_url {
                println!("  {} {}", "API URL:".bright_white(), api_url.bright_cyan());
            }
            
            if let Some(cluster) = &config.default_cluster {
                println!("  {} {}", "Default Cluster:".bright_white(), cluster.bright_cyan());
            }
            
            if let Some(format) = &config.output_format {
                println!("  {} {}", "Output Format:".bright_white(), format.bright_cyan());
            }
            
            if let Some(timeout) = &config.timeout_seconds {
                println!("  {} {}s", "Timeout:".bright_white(), timeout.to_string().bright_cyan());
            }
            
            if let Some(verify_tls) = &config.verify_tls {
                println!("  {} {}", "Verify TLS:".bright_white(), 
                         if *verify_tls { "enabled".bright_green() } else { "disabled".bright_red() });
            }
            
            println!("  {} {}", "Token:".bright_white(), 
                     if config.token.is_some() { "configured".bright_green() } else { "not set".bright_red() });
        }
    }

    let config_path = get_default_config_path()?;
    println!();
    println!("  {} {}", "Config file:".dimmed(), config_path.display().to_string().dimmed());

    Ok(())
}

async fn set_config(key: &str, value: &str, global: bool) -> Result<()> {
    use colored::*;
    
    let mut config = load_config(None)?;
    
    match key {
        "api_url" => {
            config.api_url = Some(value.to_string());
            println!("{} Set API URL to: {}", "✓".bright_green(), value.bright_cyan());
        },
        "token" => {
            config.token = Some(value.to_string());
            println!("{} Authentication token configured", "✓".bright_green());
        },
        "default_cluster" => {
            config.default_cluster = Some(value.to_string());
            println!("{} Set default cluster to: {}", "✓".bright_green(), value.bright_cyan());
        },
        "output_format" => {
            if !["table", "json", "yaml"].contains(&value) {
                return Err(anyhow::anyhow!("Invalid output format. Must be: table, json, or yaml"));
            }
            config.output_format = Some(value.to_string());
            println!("{} Set output format to: {}", "✓".bright_green(), value.bright_cyan());
        },
        "timeout" => {
            let timeout: u64 = value.parse()?;
            config.timeout_seconds = Some(timeout);
            println!("{} Set timeout to: {}s", "✓".bright_green(), timeout.to_string().bright_cyan());
        },
        "verify_tls" => {
            let verify: bool = value.parse()?;
            config.verify_tls = Some(verify);
            println!("{} Set TLS verification to: {}", "✓".bright_green(), 
                     if verify { "enabled".bright_green() } else { "disabled".bright_red() });
        },
        _ => {
            return Err(anyhow::anyhow!("Unknown configuration key: {}\n\
                Available keys: api_url, token, default_cluster, output_format, timeout, verify_tls", key));
        }
    }
    
    save_config(&config, None)?;
    Ok(())
}

async fn get_config(key: &str) -> Result<()> {
    use colored::*;
    
    let config = load_config(None)?;
    
    let value = match key {
        "api_url" => config.api_url.as_deref(),
        "token" => config.token.as_deref(),
        "default_cluster" => config.default_cluster.as_deref(),
        "output_format" => config.output_format.as_deref(),
        "timeout" => config.timeout_seconds.as_ref().map(|t| t.to_string()).as_deref(),
        "verify_tls" => config.verify_tls.as_ref().map(|t| t.to_string()).as_deref(),
        _ => {
            return Err(anyhow::anyhow!("Unknown configuration key: {}", key));
        }
    };
    
    if let Some(val) = value {
        println!("{}", val);
    } else {
        println!("{}", "not set".dimmed());
    }
    
    Ok(())
}

async fn init_config(force: bool) -> Result<()> {
    use colored::*;
    
    let config_path = get_default_config_path()?;
    
    if config_path.exists() && !force {
        return Err(anyhow::anyhow!(
            "Configuration file already exists at {}\nUse --force to overwrite", 
            config_path.display()
        ));
    }
    
    let default_config = NexusConfig::default();
    save_config(&default_config, None)?;
    
    println!("{} Configuration initialized at: {}", 
             "✓".bright_green(), 
             config_path.display().to_string().bright_cyan());
    
    println!();
    println!("{}", "Next steps:".bright_white().bold());
    println!("  1. Set your API endpoint: {} nexus config set api_url https://your-nexus-api.com", "nexus".bright_blue());
    println!("  2. Set your auth token:   {} nexus config set token your-auth-token", "nexus".bright_blue());
    println!("  3. Test connection:       {} nexus status", "nexus".bright_blue());
    
    Ok(())
}

fn get_default_config_path() -> Result<PathBuf> {
    if let Some(config_dir) = dirs::config_dir() {
        Ok(config_dir.join("nexus").join("config.toml"))
    } else {
        // Fallback to home directory
        if let Some(home_dir) = dirs::home_dir() {
            Ok(home_dir.join(".nexus").join("config.toml"))
        } else {
            Ok(PathBuf::from(".nexus-config.toml"))
        }
    }
}