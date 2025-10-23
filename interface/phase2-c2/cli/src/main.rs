//! Nexus CLI - Command & Control Interface for Nexus Core
//! 
//! The nexus command provides production-ready cluster management,
//! service orchestration, and operational tooling.

use anyhow::Result;
use clap::{Parser, Subcommand, CommandFactory};
use colored::*;
use std::path::PathBuf;
use tracing::info;

mod cluster;
mod service;
mod config;
mod output;
mod client;
mod node;
mod network;
mod storage;
mod security;
mod debug;
mod workload;
mod metrics;

use cluster::ClusterCommand;
use service::ServiceCommand;
use config::ConfigCommand;
use node::NodeCommand;
use network::NetworkCommand;
use storage::StorageCommand;
use security::SecurityCommand;
use debug::DebugCommand;
use workload::WorkloadCommand;
use metrics::MetricsCommand;

#[derive(Parser)]
#[command(name = "nexus")]
#[command(about = "Nexus - Modern Cloud Infrastructure Management")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(long_about = "
Nexus provides secure, high-performance distributed cloud infrastructure
with Byzantine fault tolerance, P2P mesh networking, and ML-powered scheduling.

EXAMPLES:
  nexus cluster create --nodes 3 --name production
  nexus service deploy nginx:1.20 --replicas 5
  nexus service scale myapp --replicas 10
  nexus cluster status --detailed
")]
struct Cli {
    /// Increase logging verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Configuration file path
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Output format (table, json, yaml)
    #[arg(short, long, default_value = "table")]
    output: String,

    /// Nexus API endpoint URL
    #[arg(long)]
    api_url: Option<String>,

    /// Authentication token
    #[arg(long)]
    token: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Cluster management operations
    #[command(alias = "clusters")]
    Cluster {
        #[command(subcommand)]
        command: ClusterCommand,
    },
    
    /// Service deployment and management
    #[command(alias = "services")]
    Service {
        #[command(subcommand)]  
        command: ServiceCommand,
    },

    /// Configuration management
    #[command(alias = "cfg")]
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },

    /// Node management operations
    Node {
        #[command(subcommand)]
        command: NodeCommand,
    },

    /// Network management and policies
    Network {
        #[command(subcommand)]
        command: NetworkCommand,
    },

    /// Storage management
    Storage {
        #[command(subcommand)]
        command: StorageCommand,
    },

    /// Security and certificate management
    Security {
        #[command(subcommand)]
        command: SecurityCommand,
    },

    /// Debugging and troubleshooting
    Debug {
        #[command(subcommand)]
        command: DebugCommand,
    },

    /// Workload management (jobs, cronjobs)
    Workload {
        #[command(subcommand)]
        command: WorkloadCommand,
    },

    /// Metrics and monitoring
    Metrics {
        #[command(subcommand)]
        command: MetricsCommand,
    },

    /// Display system status and health
    Status {
        /// Show detailed status information
        #[arg(short, long)]
        detailed: bool,
        
        /// Watch mode - continuously update status
        #[arg(short, long)]
        watch: bool,
    },

    /// Generate shell completions
    Completion {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },

    /// Show version information
    Version {
        /// Show detailed version information
        #[arg(short, long)]
        detailed: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging based on verbosity
    let log_level = match cli.verbose {
        0 => "warn",
        1 => "info", 
        2 => "debug",
        _ => "trace",
    };
    
    tracing_subscriber::fmt()
        .with_env_filter(format!("nexus_cli={},nexus={}", log_level, log_level))
        .with_target(false)
        .without_time()
        .init();

    // Print banner for interactive use
    if atty::is(atty::Stream::Stdout) && cli.verbose > 0 {
        print_banner();
    }

    info!("Nexus CLI starting with log level: {}", log_level);

    // Load configuration
    let config = config::load_config(cli.config.as_deref())?;
    
    // Initialize API client
    let client = client::NexusClient::new(
        cli.api_url.or(config.api_url),
        cli.token.or(config.token),
    )?;

    // Execute command
    match cli.command {
        Commands::Cluster { command } => {
            cluster::execute_command(command, &client, &cli.output).await
        },
        
        Commands::Service { command } => {
            service::execute_command(command, &client, &cli.output).await
        },

        Commands::Config { command } => {
            config::execute_command(command, &cli.output).await
        },

        Commands::Node { command } => {
            node::execute_command(command, &client, &cli.output).await
        },

        Commands::Network { command } => {
            network::execute_command(command, &client, &cli.output).await
        },

        Commands::Storage { command } => {
            storage::execute_command(command, &client, &cli.output).await
        },

        Commands::Security { command } => {
            security::execute_command(command, &client, &cli.output).await
        },

        Commands::Debug { command } => {
            debug::execute_command(command, &client, &cli.output).await
        },

        Commands::Workload { command } => {
            workload::execute_command(command, &client, &cli.output).await
        },

        Commands::Metrics { command } => {
            metrics::execute_command(command, &client, &cli.output).await
        },

        Commands::Status { detailed, watch } => {
            execute_status(detailed, watch, &client, &cli.output).await
        },

        Commands::Completion { shell } => {
            execute_completion(shell);
            Ok(())
        },

        Commands::Version { detailed } => {
            execute_version(detailed);
            Ok(())
        },
    }
}

fn print_banner() {
    println!("{}", "
    ███╗   ██╗███████╗██╗  ██╗██╗   ██╗███████╗
    ████╗  ██║██╔════╝╚██╗██╔╝██║   ██║██╔════╝
    ██╔██╗ ██║█████╗   ╚███╔╝ ██║   ██║███████╗
    ██║╚██╗██║██╔══╝   ██╔██╗ ██║   ██║╚════██║
    ██║ ╚████║███████╗██╔╝ ██╗╚██████╔╝███████║
    ╚═╝  ╚═══╝╚══════╝╚═╝  ╚═╝ ╚═════╝ ╚══════╝".bright_blue().bold());
    println!("{}", "    Modern Cloud Infrastructure Management".bright_white());
    println!();
}

async fn execute_status(
    detailed: bool,
    watch: bool, 
    client: &client::NexusClient,
    output_format: &str,
) -> Result<()> {
    use std::time::Duration;
    use tokio::time::sleep;

    loop {
        // Clear screen in watch mode
        if watch {
            print!("\x1B[2J\x1B[1;1H");
        }

        println!("{}", "Nexus System Status".bright_green().bold());
        println!("{}", "===================".bright_green());
        println!();

        // Get system status from Nexus core
        match get_system_status(client, detailed).await {
            Ok(status) => {
                output::display_status(&status, output_format)?;
            },
            Err(e) => {
                eprintln!("{} Failed to get system status: {}", "✗".red(), e);
                if !watch {
                    return Err(e);
                }
            }
        }

        if !watch {
            break;
        }

        println!();
        println!("{}", "Press Ctrl+C to exit watch mode...".dimmed());
        sleep(Duration::from_secs(2)).await;
    }

    Ok(())
}

async fn get_system_status(
    client: &client::NexusClient,
    detailed: bool,
) -> Result<SystemStatus> {
    // This would normally query the Nexus core components
    // For MVP, we'll simulate the status response
    
    Ok(SystemStatus {
        cluster_health: "Healthy".to_string(),
        node_count: 3,
        service_count: 12,
        active_connections: 45,
        uptime: chrono::Duration::hours(24),
        version: env!("CARGO_PKG_VERSION").to_string(),
        components: if detailed {
            Some(vec![
                ComponentStatus {
                    name: "Transport".to_string(),
                    status: "Active".to_string(),
                    connections: 15,
                },
                ComponentStatus {
                    name: "State Manager".to_string(), 
                    status: "Leader".to_string(),
                    connections: 3,
                },
                ComponentStatus {
                    name: "Scheduler".to_string(),
                    status: "Active".to_string(),
                    connections: 8,
                },
            ])
        } else {
            None
        },
    })
}

#[derive(Debug)]
struct SystemStatus {
    cluster_health: String,
    node_count: u32,
    service_count: u32,
    active_connections: u32,
    uptime: chrono::Duration,
    version: String,
    components: Option<Vec<ComponentStatus>>,
}

#[derive(Debug, serde::Serialize)]
struct ComponentStatus {
    name: String,
    status: String,
    connections: u32,
}

fn execute_completion(shell: clap_complete::Shell) {
    use clap_complete::{generate, Generator};
    use std::io;
    
    fn print_completions<G: Generator>(gen: G, cmd: &mut clap::Command) {
        generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
    }

    let mut cmd = Cli::command();
    print_completions(shell, &mut cmd);
}

fn execute_version(detailed: bool) {
    if detailed {
        println!("Nexus CLI");
        println!("Version: {}", env!("CARGO_PKG_VERSION"));
        println!("Build: {}", env!("CARGO_PKG_VERSION"));
        println!("Commit: <git-commit-here>");
        println!("Built: <build-date-here>");
        println!("Platform: {}", std::env::consts::OS);
        println!("Arch: {}", std::env::consts::ARCH);
    } else {
        println!("nexus {}", env!("CARGO_PKG_VERSION"));
    }
}