//! Integration tests for Nexus component interactions
//!
//! Tests how components work together in realistic scenarios.

pub mod consensus_integration;
pub mod transport_integration;
pub mod state_integration;
pub mod ebpf_integration;
pub mod api_integration;
pub mod consensus_container_integration;
pub mod sprint2_validation;
pub mod sprint2_performance;
pub mod sprint2_byzantine;

use crate::{TestResult, init_test_logging};
use tracing::{info, error};

/// Run all integration tests
pub async fn run_all_integration_tests() -> TestResult {
    init_test_logging();
    info!("Starting integration test suite");

    let mut failed_tests = Vec::new();

    let test_suites = vec![
        ("consensus", consensus_integration::run_consensus_integration_tests),
        ("transport", transport_integration::run_transport_integration_tests),
        ("state", state_integration::run_state_integration_tests),
        ("ebpf", ebpf_integration::run_ebpf_integration_tests),
        ("api", api_integration::run_api_integration_tests),
    ];

    for (test_name, test_fn) in test_suites {
        info!("Running {} integration tests", test_name);
        
        match test_fn().await {
            Ok(()) => {
                info!("✅ {} integration tests passed", test_name);
            }
            Err(e) => {
                error!("❌ {} integration tests failed: {}", test_name, e);
                failed_tests.push(test_name);
            }
        }
    }

    if failed_tests.is_empty() {
        info!("🎉 All integration tests passed!");
        Ok(())
    } else {
        Err(format!("Integration tests failed for: {}", failed_tests.join(", ")).into())
    }
}

/// Integration test environment setup
pub struct IntegrationTestEnv {
    pub temp_dirs: Vec<tempfile::TempDir>,
    pub test_ports: Vec<u16>,
    pub cleanup_handlers: Vec<Box<dyn Fn() + Send + Sync>>,
}

impl IntegrationTestEnv {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut env = Self {
            temp_dirs: Vec::new(),
            test_ports: Vec::new(),
            cleanup_handlers: Vec::new(),
        };
        
        // Setup test directories
        for _ in 0..3 {
            env.temp_dirs.push(tempfile::TempDir::new()?);
        }
        
        // Allocate test ports
        for _ in 0..5 {
            env.test_ports.push(crate::test_utils::find_available_port()?);
        }
        
        Ok(env)
    }
    
    pub fn get_temp_dir(&self, index: usize) -> &tempfile::TempDir {
        &self.temp_dirs[index]
    }
    
    pub fn get_port(&self, index: usize) -> u16 {
        self.test_ports[index]
    }
    
    pub fn add_cleanup_handler<F>(&mut self, handler: F) 
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.cleanup_handlers.push(Box::new(handler));
    }
}

impl Drop for IntegrationTestEnv {
    fn drop(&mut self) {
        for handler in &self.cleanup_handlers {
            handler();
        }
    }
}

/// Test cluster configuration
pub struct TestCluster {
    pub nodes: Vec<TestNode>,
    pub env: IntegrationTestEnv,
}

pub struct TestNode {
    pub id: nexus_shared::NodeId,
    pub config: nexus_shared::NexusConfig,
    pub port: u16,
    pub data_dir: String,
}

impl TestCluster {
    pub async fn new(node_count: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let env = IntegrationTestEnv::new().await?;
        let mut nodes = Vec::new();
        
        for i in 0..node_count {
            let node_id = nexus_shared::NodeId::random();
            let port = env.get_port(i);
            let data_dir = env.get_temp_dir(i).path().to_string_lossy().to_string();
            
            let mut config = nexus_shared::NexusConfig::default();
            config.transport.port = port;
            config.node.data_dir = data_dir.clone();
            config.node.name = format!("test-node-{}", i);
            
            nodes.push(TestNode {
                id: node_id,
                config,
                port,
                data_dir,
            });
        }
        
        Ok(Self { nodes, env })
    }
    
    pub fn bootstrap_addresses(&self) -> Vec<String> {
        self.nodes
            .iter()
            .map(|node| format!("127.0.0.1:{}", node.port))
            .collect()
    }
}