//! Integration Test Harness
//!
//! Framework for spinning up real nodes, establishing connections,
//! and coordinating multi-component testing.

use anyhow::{Result, Context};
use std::collections::HashMap;
use std::net::{Ipv6Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::{info, warn, error, debug};

/// Test node configuration
#[derive(Debug, Clone)]
pub struct NodeConfig {
    pub node_id: String,
    pub ipv6_address: Ipv6Addr,
    pub port: u16,
    pub data_dir: PathBuf,
    pub enable_trustchain: bool,
    pub enable_stoq: bool,
    pub enable_blockmatrix: bool,
}

impl NodeConfig {
    pub fn new(node_id: impl Into<String>, base_port: u16) -> Self {
        let node_id = node_id.into();
        Self {
            ipv6_address: Ipv6Addr::LOCALHOST,
            port: base_port,
            data_dir: PathBuf::from(format!("/tmp/web3-integration-test/{}", node_id)),
            node_id,
            enable_trustchain: true,
            enable_stoq: true,
            enable_blockmatrix: true,
        }
    }
}

/// Test node instance
pub struct TestNode {
    pub config: NodeConfig,
    pub trustchain: Option<Arc<trustchain::TrustChain>>,
    pub stoq_endpoint: Option<Arc<RwLock<stoq::transport::StoqTransport>>>,
    pub blockmatrix_node: Option<Arc<RwLock<blockmatrix::HyperMeshSystem>>>,
}

impl TestNode {
    /// Start the test node with all configured components
    pub async fn start(config: NodeConfig) -> Result<Self> {
        info!("Starting test node: {}", config.node_id);

        // Create data directory
        std::fs::create_dir_all(&config.data_dir)
            .context("Failed to create node data directory")?;

        let mut node = Self {
            config: config.clone(),
            trustchain: None,
            stoq_endpoint: None,
            blockmatrix_node: None,
        };

        // Initialize TrustChain CA if enabled
        if config.enable_trustchain {
            node.init_trustchain().await?;
        }

        // Initialize STOQ transport if enabled
        if config.enable_stoq {
            node.init_stoq().await?;
        }

        // Initialize BlockMatrix node if enabled
        if config.enable_blockmatrix {
            node.init_blockmatrix().await?;
        }

        info!("Test node started: {}", config.node_id);
        Ok(node)
    }

    /// Initialize TrustChain CA
    async fn init_trustchain(&mut self) -> Result<()> {
        debug!("Initializing TrustChain for node {}", self.config.node_id);

        let trustchain_config = trustchain::TrustChainSecurityConfig {
            base_config: trustchain::TrustChainConfig::localhost_testing(),
            security_config: Default::default(),
            mandatory_consensus: false, // Disable for testing simplicity
        };

        let trustchain = trustchain::TrustChain::new_with_security(trustchain_config)
            .await
            .context("Failed to initialize TrustChain")?;

        self.trustchain = Some(Arc::new(trustchain));
        Ok(())
    }

    /// Initialize STOQ transport
    async fn init_stoq(&mut self) -> Result<()> {
        debug!("Initializing STOQ transport for node {}", self.config.node_id);

        // Use STOQ's actual API: StoqTransport::new_optimized()
        let stoq_transport = stoq::transport::StoqTransport::new_optimized(
            self.config.ipv6_address,
            self.config.port,
            stoq::config::TransportConfig::default(),
        )
        .await
        .context("Failed to initialize STOQ transport")?;

        self.stoq_endpoint = Some(Arc::new(RwLock::new(stoq_transport)));
        Ok(())
    }

    /// Initialize BlockMatrix node
    async fn init_blockmatrix(&mut self) -> Result<()> {
        debug!("Initializing BlockMatrix for node {}", self.config.node_id);

        // Use BlockMatrix's actual API: HyperMeshSystem
        let hypermesh_config = blockmatrix::HyperMeshConfig::default();
        let hypermesh_system = blockmatrix::HyperMeshSystem::new(hypermesh_config)
            .await
            .context("Failed to initialize HyperMesh system")?;

        // Store as any placeholder for now (type mismatch resolution)
        self.blockmatrix_node = Some(Arc::new(RwLock::new(hypermesh_system)));
        Ok(())
    }

    /// Get certificate from TrustChain CA
    pub async fn get_certificate(&self, subject: &str) -> Result<trustchain::IssuedCertificate> {
        let trustchain = self.trustchain.as_ref()
            .context("TrustChain not initialized")?;

        let cert_request = trustchain::CertificateRequest {
            subject: subject.to_string(),
            subject_alt_names: vec![],
            public_key_info: vec![1, 2, 3, 4], // Mock public key
            validity_days: 90,
        };

        trustchain.issue_certificate(cert_request)
            .await
            .context("Failed to issue certificate")
    }

    /// Shutdown the node
    pub async fn shutdown(self) -> Result<()> {
        info!("Shutting down test node: {}", self.config.node_id);

        // Cleanup resources
        drop(self.trustchain);
        drop(self.stoq_endpoint);
        drop(self.blockmatrix_node);

        // Remove data directory
        if self.config.data_dir.exists() {
            std::fs::remove_dir_all(&self.config.data_dir)
                .context("Failed to remove node data directory")?;
        }

        Ok(())
    }
}

/// Test context managing multiple nodes
pub struct TestContext {
    pub nodes: HashMap<String, TestNode>,
    pub test_name: String,
}

impl TestContext {
    pub fn new(test_name: impl Into<String>) -> Self {
        let test_name = test_name.into();
        info!("Creating test context: {}", test_name);
        Self {
            nodes: HashMap::new(),
            test_name,
        }
    }

    /// Add a node to the test context
    pub async fn add_node(&mut self, config: NodeConfig) -> Result<()> {
        let node_id = config.node_id.clone();
        let node = TestNode::start(config).await?;
        self.nodes.insert(node_id, node);
        Ok(())
    }

    /// Get a node by ID
    pub fn get_node(&self, node_id: &str) -> Result<&TestNode> {
        self.nodes.get(node_id)
            .context(format!("Node not found: {}", node_id))
    }

    /// Wait for all nodes to be ready
    pub async fn wait_for_ready(&self, timeout_secs: u64) -> Result<()> {
        info!("Waiting for all nodes to be ready (timeout: {}s)", timeout_secs);

        timeout(
            Duration::from_secs(timeout_secs),
            async {
                for (node_id, node) in &self.nodes {
                    debug!("Checking readiness for node: {}", node_id);
                    // Basic readiness check - components initialized
                    if node.config.enable_trustchain && node.trustchain.is_none() {
                        return Err(anyhow::anyhow!("TrustChain not ready for node {}", node_id));
                    }
                    if node.config.enable_stoq && node.stoq_endpoint.is_none() {
                        return Err(anyhow::anyhow!("STOQ not ready for node {}", node_id));
                    }
                    if node.config.enable_blockmatrix && node.blockmatrix_node.is_none() {
                        return Err(anyhow::anyhow!("BlockMatrix not ready for node {}", node_id));
                    }
                }
                Ok(())
            }
        )
        .await
        .context("Timeout waiting for nodes to be ready")??;

        info!("All nodes ready");
        Ok(())
    }

    /// Shutdown all nodes
    pub async fn shutdown(self) -> Result<()> {
        info!("Shutting down test context: {}", self.test_name);

        for (node_id, node) in self.nodes {
            if let Err(e) = node.shutdown().await {
                error!("Failed to shutdown node {}: {}", node_id, e);
            }
        }

        Ok(())
    }
}

/// Integration test harness
pub struct IntegrationTestHarness {
    pub name: String,
    pub timeout: Duration,
}

impl IntegrationTestHarness {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            timeout: Duration::from_secs(300), // 5 minute default timeout
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Run a test with the harness
    pub async fn run<F, Fut>(&self, test_fn: F) -> Result<()>
    where
        F: FnOnce(TestContext) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        info!("Running integration test: {}", self.name);

        let context = TestContext::new(self.name.clone());

        let result = timeout(self.timeout, test_fn(context)).await;

        match result {
            Ok(Ok(())) => {
                info!("Integration test PASSED: {}", self.name);
                Ok(())
            }
            Ok(Err(e)) => {
                error!("Integration test FAILED: {} - {}", self.name, e);
                Err(e)
            }
            Err(_) => {
                error!("Integration test TIMEOUT: {} ({}s)", self.name, self.timeout.as_secs());
                Err(anyhow::anyhow!("Test timeout after {}s", self.timeout.as_secs()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_harness_basic() {
        let harness = IntegrationTestHarness::new("test_harness_basic");

        harness.run(|mut ctx| async move {
            // Add a test node
            let config = NodeConfig::new("test-node-1", 19000);
            ctx.add_node(config).await?;

            // Verify node exists
            let node = ctx.get_node("test-node-1")?;
            assert_eq!(node.config.node_id, "test-node-1");

            ctx.shutdown().await?;
            Ok(())
        })
        .await
        .expect("Test harness basic test failed");
    }
}
