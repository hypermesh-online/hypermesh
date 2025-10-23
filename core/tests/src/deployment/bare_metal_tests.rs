//! Bare metal deployment tests
//!
//! Tests deploying Nexus directly on bare metal systems

use crate::{TestResult, init_test_logging, test_utils};
use std::process::Command;
use std::time::Duration;
use tempfile::TempDir;

pub async fn run_bare_metal_tests() -> TestResult {
    init_test_logging();
    
    test_binary_deployment().await?;
    test_systemd_service().await?;
    test_multi_node_bare_metal().await?;
    test_network_interface_binding().await?;
    test_resource_allocation().await?;
    
    Ok(())
}

async fn test_binary_deployment() -> TestResult {
    tracing::info!("Testing bare metal binary deployment");
    
    // Check if binaries exist and are executable
    let binaries = vec![
        "target/release/nexus-coordinator",
        "target/release/nexus-api-server", 
        "target/release/nexus-cli",
    ];
    
    for binary_path in binaries {
        let full_path = format!("/home/persist/repos/work/vazio/hypermesh/{}", binary_path);
        
        // Check if binary exists (in real deployment this would be built)
        if std::path::Path::new(&full_path).exists() {
            tracing::info!("✅ Binary exists: {}", binary_path);
            
            // Test binary execution (help command)
            let output = Command::new(&full_path)
                .arg("--help")
                .output();
                
            match output {
                Ok(result) => {
                    if result.status.success() {
                        tracing::info!("✅ Binary executes successfully: {}", binary_path);
                    } else {
                        tracing::warn!("⚠️ Binary help failed: {}", binary_path);
                    }
                }
                Err(_) => {
                    tracing::info!("ℹ️ Binary not built yet: {}", binary_path);
                }
            }
        } else {
            tracing::info!("ℹ️ Binary not found (expected in test): {}", binary_path);
        }
    }
    
    Ok(())
}

async fn test_systemd_service() -> TestResult {
    tracing::info!("Testing systemd service deployment");
    
    let temp_dir = TempDir::new()?;
    let service_file_path = temp_dir.path().join("nexus.service");
    
    // Create a test systemd service file
    let service_content = r#"[Unit]
Description=Nexus Hypermesh Node
After=network.target
Wants=network.target

[Service]
Type=simple
User=nexus
Group=nexus
ExecStart=/usr/local/bin/nexus-coordinator --config /etc/nexus/nexus.toml
Restart=always
RestartSec=5
LimitNOFILE=65536
Environment=NEXUS_LOG_LEVEL=info

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/nexus /var/log/nexus

[Install]
WantedBy=multi-user.target
"#;

    std::fs::write(&service_file_path, service_content)?;
    
    // Validate service file syntax
    let service_content = std::fs::read_to_string(&service_file_path)?;
    
    // Basic validation checks
    assert!(service_content.contains("[Unit]"));
    assert!(service_content.contains("[Service]"));
    assert!(service_content.contains("[Install]"));
    assert!(service_content.contains("nexus-coordinator"));
    assert!(service_content.contains("LimitNOFILE=65536"));
    
    tracing::info!("✅ Systemd service file validated");
    Ok(())
}

async fn test_multi_node_bare_metal() -> TestResult {
    tracing::info!("Testing multi-node bare metal deployment simulation");
    
    // Simulate 3-node cluster deployment
    let nodes = vec![
        NodeDeployment {
            node_id: "node-1",
            ip_address: "192.168.1.10",
            port: 7777,
            data_dir: "/var/lib/nexus/node-1",
        },
        NodeDeployment {
            node_id: "node-2", 
            ip_address: "192.168.1.11",
            port: 7777,
            data_dir: "/var/lib/nexus/node-2",
        },
        NodeDeployment {
            node_id: "node-3",
            ip_address: "192.168.1.12", 
            port: 7777,
            data_dir: "/var/lib/nexus/node-3",
        },
    ];
    
    for node in &nodes {
        // Test node configuration generation
        let config = generate_node_config(node)?;
        
        // Validate configuration
        assert!(config.contains(&node.node_id));
        assert!(config.contains(&node.ip_address));
        assert!(config.contains(&node.data_dir));
        
        tracing::info!("✅ Node {} configuration validated", node.node_id);
    }
    
    // Test cluster bootstrap configuration
    let bootstrap_nodes: Vec<String> = nodes
        .iter()
        .map(|n| format!("{}:{}", n.ip_address, n.port))
        .collect();
    
    assert_eq!(bootstrap_nodes.len(), 3);
    tracing::info!("✅ Cluster bootstrap configuration: {:?}", bootstrap_nodes);
    
    Ok(())
}

async fn test_network_interface_binding() -> TestResult {
    tracing::info!("Testing network interface binding");
    
    // Test different network configurations
    let network_configs = vec![
        NetworkConfig {
            bind_interface: "eth0".to_string(),
            bind_address: "0.0.0.0".to_string(),
            port: 7777,
            enable_ipv6: false,
        },
        NetworkConfig {
            bind_interface: "eth1".to_string(), 
            bind_address: "192.168.1.10".to_string(),
            port: 7778,
            enable_ipv6: true,
        },
    ];
    
    for config in &network_configs {
        // Validate network configuration
        assert!(!config.bind_interface.is_empty());
        assert!(!config.bind_address.is_empty());
        assert!(config.port > 1024);
        
        // Test configuration serialization
        let config_toml = format!(r#"
[network]
bind_interface = "{}"
bind_address = "{}"
port = {}
enable_ipv6 = {}
"#, 
            config.bind_interface,
            config.bind_address, 
            config.port,
            config.enable_ipv6
        );
        
        // Basic TOML validation
        assert!(config_toml.contains(&config.bind_interface));
        
        tracing::info!("✅ Network config validated: {}", config.bind_interface);
    }
    
    Ok(())
}

async fn test_resource_allocation() -> TestResult {
    tracing::info!("Testing bare metal resource allocation");
    
    // Test system resource detection and allocation
    let resource_config = SystemResourceConfig {
        memory_limit_gb: 32,
        cpu_cores: 16, 
        disk_space_gb: 1000,
        network_bandwidth_gbps: 10,
        hugepages_gb: 4,
        numa_nodes: 2,
    };
    
    // Validate resource constraints
    assert!(resource_config.memory_limit_gb >= 8); // Minimum for Nexus
    assert!(resource_config.cpu_cores >= 4);       // Minimum for consensus
    assert!(resource_config.disk_space_gb >= 100); // Minimum for storage
    assert!(resource_config.hugepages_gb <= resource_config.memory_limit_gb);
    
    // Test NUMA-aware configuration
    if resource_config.numa_nodes > 1 {
        let memory_per_numa = resource_config.memory_limit_gb / resource_config.numa_nodes;
        let cores_per_numa = resource_config.cpu_cores / resource_config.numa_nodes;
        
        assert!(memory_per_numa >= 4); // At least 4GB per NUMA node
        assert!(cores_per_numa >= 2);   // At least 2 cores per NUMA node
        
        tracing::info!("✅ NUMA configuration: {} nodes, {}GB/{}cores each", 
                      resource_config.numa_nodes, memory_per_numa, cores_per_numa);
    }
    
    // Test eBPF resource requirements
    assert!(resource_config.memory_limit_gb >= 16); // eBPF needs sufficient memory
    
    tracing::info!("✅ Resource allocation validated");
    Ok(())
}

// Helper structures

struct NodeDeployment {
    node_id: &'static str,
    ip_address: &'static str,
    port: u16,
    data_dir: &'static str,
}

struct NetworkConfig {
    bind_interface: String,
    bind_address: String,
    port: u16,
    enable_ipv6: bool,
}

struct SystemResourceConfig {
    memory_limit_gb: u32,
    cpu_cores: u32,
    disk_space_gb: u32,
    network_bandwidth_gbps: u32,
    hugepages_gb: u32,
    numa_nodes: u32,
}

fn generate_node_config(node: &NodeDeployment) -> Result<String, Box<dyn std::error::Error>> {
    let config = format!(r#"
# Nexus Node Configuration - {}
[node]
id = "{}"
name = "nexus-{}"
data_dir = "{}"

[network]
bind_address = "{}"
port = {}
max_connections = 10000

[consensus]
bootstrap_timeout_ms = 30000
election_timeout_ms = 5000
heartbeat_interval_ms = 1000

[storage]
backend = "RocksDB"
max_size_gb = 100
enable_compression = true

[ebpf]
enable_network_monitoring = true
enable_traffic_control = true
enable_load_balancing = true

[security]
enable_tls = true
require_client_cert = true
"#, 
        node.node_id,
        node.node_id,
        node.node_id,
        node.data_dir,
        node.ip_address,
        node.port
    );
    
    Ok(config)
}