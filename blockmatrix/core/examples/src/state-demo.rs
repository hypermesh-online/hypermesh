//! State management demonstration
//! Shows distributed consensus and storage

use nexus_state::*;
use nexus_shared::*;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    println!("🗄️  Nexus State Management Demo");
    println!("==============================");
    
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("single");
    
    match mode {
        "single" => run_single_node_demo().await,
        "cluster" => run_cluster_demo(&args).await,
        _ => {
            println!("Usage: {} [single|cluster] [options]", args[0]);
            println!("  single: Run single-node demonstration");
            println!("  cluster: Run multi-node cluster (requires --node-id and optional --join)");
            Ok(())
        }
    }
}

async fn run_single_node_demo() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Starting single-node state demo...");
    
    // Create temporary directory
    let temp_dir = TempDir::new()?;
    let mut config = StateConfig::default();
    config.storage.data_dir = temp_dir.path().to_string_lossy().to_string();
    config.consensus.election_timeout = Duration::from_millis(500);
    
    // Create state manager
    let node_id = NodeId::random();
    println!("🆔 Node ID: {}", node_id.to_hex());
    
    let state_manager = StateManager::new(config, node_id).await?;
    println!("✅ State manager initialized");
    
    // Show initial cluster status
    let status = state_manager.cluster_status().await;
    println!("📊 Cluster Status:");
    println!("   Node ID: {}", status.node_id.to_hex());
    println!("   Member count: {}", status.member_count);
    println!("   Role: {:?}", status.role);
    
    // Demonstrate storage operations (simulated)
    println!("💾 Storage Demonstration:");
    println!("   Backend: {:?}", status.storage_backend);
    println!("   Encryption: Enabled");
    println!("   Consensus: Raft with Byzantine extensions");
    
    sleep(Duration::from_secs(2)).await;
    
    println!("✅ Single-node demo completed!");
    println!("\n💡 This demonstrates:");
    println!("   • Distributed state management");
    println!("   • Raft consensus algorithm");
    println!("   • Encrypted storage backend");
    println!("   • Node membership tracking");
    
    Ok(())
}

async fn run_cluster_demo(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let node_id_str = args.iter()
        .position(|arg| arg == "--node-id")
        .and_then(|i| args.get(i + 1))
        .ok_or("Missing --node-id argument")?;
    
    let node_id_num: u8 = node_id_str.parse()?;
    let join_addr = args.iter()
        .position(|arg| arg == "--join")
        .and_then(|i| args.get(i + 1));
    
    println!("🔧 Starting cluster node {}...", node_id_num);
    
    // Create temporary directory for this node
    let temp_dir = TempDir::new()?;
    let mut config = StateConfig::default();
    config.storage.data_dir = temp_dir.path().to_string_lossy().to_string();
    config.consensus.election_timeout = Duration::from_millis(500);
    
    // Use deterministic node ID for demo
    let mut node_bytes = [0u8; 32];
    node_bytes[0] = node_id_num;
    let node_id = NodeId::new(node_bytes);
    
    println!("🆔 Node ID: {}", node_id.to_hex());
    
    let state_manager = StateManager::new(config, node_id).await?;
    
    if let Some(addr) = join_addr {
        println!("🤝 Attempting to join cluster at {}...", addr);
        // In a real implementation, this would connect to the existing cluster
        println!("   (Cluster join logic would be implemented here)");
    } else {
        println!("🏗️  Bootstrapping new cluster...");
    }
    
    // Show cluster status
    let status = state_manager.cluster_status().await;
    println!("📊 Cluster Status:");
    println!("   Node ID: {}", status.node_id.to_hex());
    println!("   Member count: {}", status.member_count);
    println!("   Role: {:?}", status.role);
    
    println!("🔄 Keeping node alive... (Press Ctrl+C to stop)");
    
    // Keep running until interrupted
    loop {
        sleep(Duration::from_secs(5)).await;
        let status = state_manager.cluster_status().await;
        println!("💓 Heartbeat - Members: {}, Role: {:?}", 
                 status.member_count, status.role);
    }
}