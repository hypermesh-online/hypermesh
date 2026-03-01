// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Node Bootstrap Module
//!
//! CRITICAL ARCHITECTURE: TrustChain and BlockMatrix are ONE SYSTEM, not separate components.
//! Every node starts with:
//! 1. Unique genesis block (own blockchain)
//! 2. Self-signed localhost certificate
//! 3. DNS initialized with localhost → self
//! 4. Privacy mode determines network participation

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::blockchain::block::Block;
use crate::blockchain::node_chain::NodeBlockchain;
use crate::matrix::coordinate::MatrixCoordinate;

/// Re-export canonical PrivacyMode from hypermesh-lib.
///
/// PrivacyMode is a two-axis struct { scope: AccessScope, tracked: bool } with
/// three named presets: ANONYMOUS, PRIVATE, PUBLIC.
pub use hypermesh_lib::PrivacyMode;

/// Self-signed certificate for localhost
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalhostCertificate {
    /// Subject (always "localhost")
    pub subject: String,
    /// Issuer (self)
    pub issuer: String,
    /// Certificate valid from
    pub not_before: SystemTime,
    /// Certificate valid until
    pub not_after: SystemTime,
    /// Certificate fingerprint (BLAKE3)
    pub fingerprint: String,
    /// Always true for bootstrap
    pub is_self_signed: bool,
    /// Raw certificate data (FALCON-1024 signature)
    pub certificate_data: Vec<u8>,
}

/// DNS resolver for node
#[derive(Debug, Clone)]
pub struct DnsResolver {
    /// DNS records (name → IP)
    records: Arc<RwLock<HashMap<String, IpAddr>>>,
}

impl Default for DnsResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl DnsResolver {
    /// Create new DNS resolver
    pub fn new() -> Self {
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register DNS record
    pub async fn register(&self, name: String, addr: IpAddr) {
        info!("DNS registered: {} → {}", name, addr);
        self.records.write().await.insert(name, addr);
    }

    /// Resolve DNS name
    pub async fn resolve(&self, name: &str) -> Option<IpAddr> {
        self.records.read().await.get(name).copied()
    }

    /// Get all records
    pub async fn all_records(&self) -> HashMap<String, IpAddr> {
        self.records.read().await.clone()
    }
}

/// Node bootstrap state
pub struct NodeBootstrap {
    /// Unique genesis block for THIS node
    genesis_block: Block,

    /// Node's independent blockchain
    blockchain: Arc<NodeBlockchain>,

    /// Self-signed certificate for localhost
    localhost_cert: LocalhostCertificate,

    /// DNS resolver (starts with localhost only)
    dns: DnsResolver,

    /// Current privacy mode
    privacy_mode: Arc<RwLock<PrivacyMode>>,

    /// Node's matrix coordinate
    node_coordinate: MatrixCoordinate,

    /// Bootstrap timestamp
    bootstrapped_at: SystemTime,
}

impl NodeBootstrap {
    /// Initialize a new node with self-sufficient bootstrap
    ///
    /// This creates:
    /// 1. Unique genesis block for this node
    /// 2. Self-signed localhost certificate
    /// 3. DNS with localhost → ::1
    /// 4. Default to Private mode (no network)
    pub async fn initialize(node_coordinate: MatrixCoordinate) -> Result<Self> {
        info!(
            "Initializing node at ({}, {}, {}) with self-sufficient bootstrap",
            node_coordinate.x, node_coordinate.y, node_coordinate.z
        );

        // 1. Create unique genesis block for THIS node
        let genesis_block = Block::genesis(node_coordinate);
        info!("Created genesis block: {}", genesis_block.hash);

        // 2. Initialize blockchain with genesis
        let blockchain = Arc::new(NodeBlockchain::new(node_coordinate));

        // 3. Self-sign certificate for localhost
        let localhost_cert = Self::generate_localhost_certificate()?;
        info!("Generated self-signed localhost certificate");

        // 4. Initialize DNS with localhost → ::1
        let dns = DnsResolver::new();
        dns.register(
            "localhost".to_string(),
            IpAddr::from([0, 0, 0, 0, 0, 0, 0, 1]), // ::1
        )
        .await;
        info!("DNS initialized with localhost → ::1");

        // 5. Default to Private mode (localhost only)
        let privacy_mode = Arc::new(RwLock::new(PrivacyMode::PRIVATE));

        let bootstrap = Self {
            genesis_block,
            blockchain,
            localhost_cert,
            dns,
            privacy_mode,
            node_coordinate,
            bootstrapped_at: SystemTime::now(),
        };

        info!("Node bootstrap complete - running in Private mode (localhost only)");
        Ok(bootstrap)
    }

    /// Generate self-signed certificate for localhost
    fn generate_localhost_certificate() -> Result<LocalhostCertificate> {
        use blake3::Hasher;

        let now = SystemTime::now();
        let valid_duration = Duration::from_secs(365 * 24 * 60 * 60); // 1 year

        // Generate certificate data (placeholder - would use real FALCON-1024 in production)
        let cert_data = b"SELF-SIGNED-LOCALHOST-CERTIFICATE".to_vec();

        // Calculate fingerprint
        let mut hasher = Hasher::new();
        hasher.update(&cert_data);
        let fingerprint = hasher.finalize().to_hex().to_string();

        Ok(LocalhostCertificate {
            subject: "localhost".to_string(),
            issuer: "self".to_string(),
            not_before: now,
            not_after: now + valid_duration,
            fingerprint,
            is_self_signed: true,
            certificate_data: cert_data,
        })
    }

    /// Transition to different privacy mode
    ///
    /// CRITICAL: Network participation is OPTIONAL based on privacy mode
    pub async fn set_privacy_mode(&self, mode: PrivacyMode) -> Result<()> {
        let current_mode = *self.privacy_mode.read().await;

        if current_mode == mode {
            info!("Already in {:?} mode", mode);
            return Ok(());
        }

        info!("Transitioning from {:?} to {:?} mode", current_mode, mode);

        if mode == PrivacyMode::PRIVATE {
            // Stay localhost only - no changes needed
            info!("Private mode: localhost only, no network participation");
        } else if mode == PrivacyMode::ANONYMOUS {
            // Enable ephemeral connections (no DNS registration)
            info!("Anonymous mode: ephemeral connections enabled, no DNS registration");
        } else if mode == PrivacyMode::PUBLIC {
            // Register DNS as blockchain asset
            // Connect to network head
            // Participate in consensus
            info!("Public mode: registering with network");
            self.register_with_network().await?;
        } else {
            info!("Custom privacy mode: {:?}", mode);
        }

        *self.privacy_mode.write().await = mode;
        Ok(())
    }

    /// Register with network (Public mode only)
    async fn register_with_network(&self) -> Result<()> {
        info!("Registering node with network");

        // 1. Register DNS name as blockchain asset
        // DNS registration requires full Proof of State:
        // - PoSpace (WHERE): Node's matrix position + storage commitment
        // - PoStake (WHO): Ownership, economic stake in the name
        // - PoWork (WHAT): Computational proof of registration work
        // - PoTime (WHEN): Temporal ordering, prevents replay attacks

        // TODO: Implement DNS-as-Asset registration
        // TODO: Connect to network head
        // TODO: Enable consensus participation
        // TODO: Start CAESAR rewards

        warn!("Network registration not yet implemented");
        Ok(())
    }

    /// Get current privacy mode
    pub async fn privacy_mode(&self) -> PrivacyMode {
        *self.privacy_mode.read().await
    }

    /// Get genesis block
    pub fn genesis_block(&self) -> &Block {
        &self.genesis_block
    }

    /// Get blockchain
    pub fn blockchain(&self) -> Arc<NodeBlockchain> {
        self.blockchain.clone()
    }

    /// Get localhost certificate
    pub fn localhost_certificate(&self) -> &LocalhostCertificate {
        &self.localhost_cert
    }

    /// Get DNS resolver
    pub fn dns(&self) -> &DnsResolver {
        &self.dns
    }

    /// Get node coordinate
    pub fn node_coordinate(&self) -> &MatrixCoordinate {
        &self.node_coordinate
    }

    /// Get bootstrap timestamp
    pub fn bootstrapped_at(&self) -> SystemTime {
        self.bootstrapped_at
    }

    /// Verify node is self-sufficient
    pub async fn verify_self_sufficient(&self) -> Result<()> {
        // 1. Verify genesis block exists
        if self.genesis_block.index != 0 {
            return Err(anyhow!("Genesis block index must be 0"));
        }

        // 2. Verify blockchain initialized
        let chain_stats = self.blockchain.get_stats().await;
        if chain_stats.total_blocks == 0 {
            return Err(anyhow!("Blockchain not initialized"));
        }

        // 3. Verify localhost certificate
        if !self.localhost_cert.is_self_signed {
            return Err(anyhow!("Localhost certificate must be self-signed"));
        }

        // 4. Verify DNS has localhost
        let localhost_addr = self
            .dns
            .resolve("localhost")
            .await
            .ok_or_else(|| anyhow!("DNS missing localhost entry"))?;

        if localhost_addr != IpAddr::from([0, 0, 0, 0, 0, 0, 0, 1]) {
            return Err(anyhow!("localhost must resolve to ::1"));
        }

        info!("Node self-sufficiency verified");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_node_bootstrap_initialization() {
        let coord = MatrixCoordinate::new(1, 2, 3).expect("test: valid coordinate");
        let bootstrap = NodeBootstrap::initialize(coord).await.expect("test: async operation");

        // Verify genesis block
        assert_eq!(bootstrap.genesis_block().index, 0);
        assert_eq!(bootstrap.genesis_block().node_coordinate, coord);

        // Verify localhost certificate
        let cert = bootstrap.localhost_certificate();
        assert_eq!(cert.subject, "localhost");
        assert!(cert.is_self_signed);

        // Verify DNS
        let localhost = bootstrap.dns().resolve("localhost").await;
        assert_eq!(localhost, Some(IpAddr::from([0, 0, 0, 0, 0, 0, 0, 1])));

        // Verify privacy mode
        assert_eq!(bootstrap.privacy_mode().await, PrivacyMode::PRIVATE);
    }

    #[tokio::test]
    async fn test_privacy_mode_transitions() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let bootstrap = NodeBootstrap::initialize(coord).await.expect("test: async operation");

        // Start in Private mode
        assert_eq!(bootstrap.privacy_mode().await, PrivacyMode::PRIVATE);

        // Transition to Anonymous
        bootstrap
            .set_privacy_mode(PrivacyMode::ANONYMOUS)
            .await
            .expect("test: expected success");
        assert_eq!(bootstrap.privacy_mode().await, PrivacyMode::ANONYMOUS);

        // Transition to Public (network registration)
        bootstrap
            .set_privacy_mode(PrivacyMode::PUBLIC)
            .await
            .expect("test: expected success");
        assert_eq!(bootstrap.privacy_mode().await, PrivacyMode::PUBLIC);

        // Transition back to Private
        bootstrap
            .set_privacy_mode(PrivacyMode::PRIVATE)
            .await
            .expect("test: expected success");
        assert_eq!(bootstrap.privacy_mode().await, PrivacyMode::PRIVATE);
    }

    #[tokio::test]
    async fn test_node_self_sufficiency() {
        let coord = MatrixCoordinate::new(5, 5, 5).expect("test: valid coordinate");
        let bootstrap = NodeBootstrap::initialize(coord).await.expect("test: async operation");

        // Verify self-sufficiency
        bootstrap.verify_self_sufficient().await.expect("test: async operation");
    }

    #[tokio::test]
    async fn test_unique_genesis_per_node() {
        let coord1 = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
        let coord2 = MatrixCoordinate::new(2, 2, 2).expect("test: valid coordinate");

        let bootstrap1 = NodeBootstrap::initialize(coord1).await.expect("test: async operation");
        let bootstrap2 = NodeBootstrap::initialize(coord2).await.expect("test: async operation");

        // Each node has unique genesis block
        assert_ne!(
            bootstrap1.genesis_block().hash,
            bootstrap2.genesis_block().hash
        );

        // Each genesis belongs to its node
        assert_eq!(bootstrap1.genesis_block().node_coordinate.x, 1);
        assert_eq!(bootstrap2.genesis_block().node_coordinate.x, 2);
    }
}
