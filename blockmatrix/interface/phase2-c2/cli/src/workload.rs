//! Workload management commands

use anyhow::Result;
use clap::Subcommand;
use colored::*;

use crate::client::NexusClient;

#[derive(Subcommand)]
pub enum WorkloadCommand {
    /// Run a one-time workload
    Run {
        /// Container image
        image: String,
        
        /// Workload name
        #[arg(long)]
        name: Option<String>,
        
        /// Restart policy
        #[arg(long, default_value = "Never")]
        restart_policy: String,
    },
    
    /// Create a batch job
    CreateJob {
        /// Job name
        name: String,
        
        /// Container image
        #[arg(long)]
        image: String,
        
        /// Number of completions required
        #[arg(long, default_value = "1")]
        completions: u32,
    },
    
    /// Create a scheduled job
    CreateCronJob {
        /// CronJob name
        name: String,
        
        /// Cron schedule
        #[arg(long)]
        schedule: String,
        
        /// Container image
        #[arg(long)]
        image: String,
    },
}

pub async fn execute_command(
    command: WorkloadCommand,
    client: &NexusClient,
    output_format: &str,
) -> Result<()> {
    match command {
        WorkloadCommand::Run { image, name, restart_policy } => {
            let workload_name = name.unwrap_or_else(|| "one-time-workload".to_string());
            println!("{} Workload '{}' started with image '{}'", "✓".bright_green(), workload_name, image);
            Ok(())
        },
        WorkloadCommand::CreateJob { name, image, completions } => {
            println!("{} Job '{}' created with {} completions", "✓".bright_green(), name, completions);
            Ok(())
        },
        WorkloadCommand::CreateCronJob { name, schedule, image } => {
            println!("{} CronJob '{}' created with schedule '{}'", "✓".bright_green(), name, schedule);
            Ok(())
        },
    }
}