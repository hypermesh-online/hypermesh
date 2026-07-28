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

use stoq::transport::{
    AnonymousCertificateStrategy, AuthenticatedCertificateStrategy, CertificateStrategy,
};

// ---------------------------------------------------------------------------
// Well-known local service endpoints
// ---------------------------------------------------------------------------

/// A local service DNS entry mapping a name to an IPv6 loopback port.
pub struct ServiceEntry {
    /// Service name (used as DNS key, e.g. "catalog")
    pub name: &'static str,
    /// Port the service binds on [::1]
    pub port: u16,
}

/// Default local service DNS entries. Every node registers these
/// in bootstrap DNS so that `dns.resolve("catalog")` returns `::1`.
pub const LOCAL_SERVICES: &[ServiceEntry] = &[
    ServiceEntry { name: "blockmatrix", port: 9292 },
    ServiceEntry { name: "caesar", port: 9294 },
    ServiceEntry { name: "catalog", port: 9295 },
    ServiceEntry { name: "trust", port: 8444 },
    ServiceEntry { name: "ngauge", port: 9296 },
];

// ---------------------------------------------------------------------------
// Certificate strategy selection
// ---------------------------------------------------------------------------

/// Select the correct [`CertificateStrategy`] for the given privacy mode.
///
/// - **Anonymous**: Ephemeral self-signed certs, no CA/CT involvement. Cert
///   rotation is a no-op (each connection gets a fresh cert anyway).
/// - **Private**: Authenticated via local TrustChain (`local://trustchain`).
/// - **Public**: Authenticated via global TrustChain (`quic://trust.hypermesh.online`).
///
/// Any other two-axis combination (e.g. `Bounded + untracked`) falls back to
/// Anonymous because untracked modes should never call a CA.
pub fn select_certificate_strategy(
    privacy_mode: &PrivacyMode,
    node_id: &str,
    common_name: &str,
    ipv6_addresses: Vec<std::net::Ipv6Addr>,
) -> Arc<dyn CertificateStrategy> {
    if !privacy_mode.tracked {
        // Untracked modes (Anonymous, Bounded+untracked) use ephemeral certs.
        // No CA, no CT, no renewal. Each connection gets a fresh keypair.
        Arc::new(AnonymousCertificateStrategy::new())
    } else if privacy_mode.scope == hypermesh_lib::AccessScope::Bounded {
        // Private (Bounded + tracked) uses the local TrustChain CA.
        Arc::new(AuthenticatedCertificateStrategy::new(
            "local://trustchain".to_string(),
            node_id.to_string(),
            common_name.to_string(),
            ipv6_addresses,
            "Private".to_string(),
        ))
    } else {
        // Public (Unbounded + tracked) uses the global TrustChain CA.
        Arc::new(AuthenticatedCertificateStrategy::new(
            "quic://trust.hypermesh.online".to_string(),
            node_id.to_string(),
            common_name.to_string(),
            ipv6_addresses,
            "Public".to_string(),
        ))
    }
}

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

    /// Resolve service name to socket address (IP + port).
    /// Uses well-known port table for local services.
    pub async fn resolve_service(&self, name: &str) -> Option<std::net::SocketAddr> {
        let ip = self.resolve(name).await?;
        let port = LOCAL_SERVICES.iter().find(|s| s.name == name).map(|s| s.port)?;
        Some(std::net::SocketAddr::new(ip, port))
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

/// Legacy coordinate-derived data-dir key (`node_{x}_{y}_{z}`).
///
/// D5: this is the MIGRATION-WINDOW alias only. A node's state used to live
/// under `data_dir/node_{x}_{y}_{z}/`, keyed by the matrix coordinate string.
/// The canonical key is now the device identity ([`state_dir_key`]); this
/// function survives ONLY to locate a pre-migration data dir so an existing
/// install can be adopted (see [`adopt_legacy_state_dir`]). It touches no chain
/// bytes — it was always a filesystem path key.
pub fn node_id(coord: &MatrixCoordinate) -> String {
    format!("node_{}_{}_{}", coord.x, coord.y, coord.z)
}

/// The coordinate-INDEPENDENT identity directory for a data dir.
///
/// D5: the device identity (`BLAKE3(falcon_pubkey)`) is what keys the data dir,
/// so the identity itself cannot live UNDER that key without a chicken-and-egg.
/// It lives at a fixed `data_dir/identity/` instead — loadable before the key
/// is known, and shared by every command that needs the node's keypair.
pub fn identity_dir(data_dir: &std::path::Path) -> std::path::PathBuf {
    data_dir.join("identity")
}

/// The canonical data-dir key: the device identity hex (`BLAKE3(falcon_pubkey)`).
///
/// D5 "node ≡ asset ≡ index": the node's on-disk state is keyed by WHO the node
/// is, not WHERE it sits in the matrix. Passed to `PersistenceManager` as the
/// storage sub-directory and used as the runtime node-id string.
pub fn state_dir_key(device_node_id: &str) -> String {
    device_node_id.to_string()
}

/// D5 Part 1 — adopt a pre-migration in-tree identity directory.
///
/// Resolves the coordinate-independent [`identity_dir`] and, if it does not yet
/// exist while a legacy `data_dir/{legacy_key}/identity` does, migrates the
/// legacy identity up so the device keypair (and therefore the derived data-dir
/// key) is unchanged across the upgrade. A fresh node has neither directory and
/// this is a no-op — `load_or_create` then creates the new location directly.
///
/// Idempotent: once the new location exists it is used as-is; the legacy copy is
/// never allowed to clobber it.
pub fn adopt_legacy_identity(
    data_dir: &std::path::Path,
    legacy_key: &str,
) -> Result<std::path::PathBuf> {
    let new_identity = identity_dir(data_dir);
    if new_identity.exists() {
        return Ok(new_identity);
    }
    let legacy_identity = data_dir.join(legacy_key).join("identity");
    if legacy_identity.exists() {
        if let Some(parent) = new_identity.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                anyhow!("failed to create data dir {}: {e}", parent.display())
            })?;
        }
        std::fs::rename(&legacy_identity, &new_identity).map_err(|e| {
            anyhow!(
                "failed to adopt legacy identity {} -> {}: {e}",
                legacy_identity.display(),
                new_identity.display()
            )
        })?;
        info!(
            "Adopted legacy identity {} -> {} (D5 data-dir migration)",
            legacy_identity.display(),
            new_identity.display()
        );
    }
    Ok(new_identity)
}

/// D5 Part 1 — adopt pre-migration coordinate-keyed chain state.
///
/// If the identity-keyed state dir (`data_dir/{state_key}`) does not exist while
/// a legacy `data_dir/{legacy_key}` does, the legacy directory is renamed onto
/// the new key so the node keeps its persisted chain, certificate, matrix state
/// and shards. [`adopt_legacy_identity`] must run FIRST so the legacy identity
/// has already been moved to its own location and does not travel with the
/// rename. A fresh node (no legacy dir) is a no-op — the state dir is created
/// under the new key by the persistence manager.
///
/// Idempotent and fail-safe: an already-migrated node (new dir present) is left
/// untouched, and a legacy key equal to the new key is skipped.
pub fn adopt_legacy_state_dir(
    data_dir: &std::path::Path,
    legacy_key: &str,
    state_key: &str,
) -> Result<()> {
    if legacy_key == state_key {
        return Ok(());
    }
    let new_dir = data_dir.join(state_key);
    if new_dir.exists() {
        return Ok(());
    }
    let legacy_dir = data_dir.join(legacy_key);
    if legacy_dir.is_dir() {
        std::fs::rename(&legacy_dir, &new_dir).map_err(|e| {
            anyhow!(
                "failed to adopt legacy state dir {} -> {}: {e}",
                legacy_dir.display(),
                new_dir.display()
            )
        })?;
        info!(
            "Adopted legacy chain state {} -> {} (D5 data-dir migration)",
            legacy_dir.display(),
            new_dir.display()
        );
    }
    Ok(())
}

impl NodeBootstrap {
    /// Resume a previously persisted node.
    ///
    /// Instead of creating a fresh genesis/blockchain/certificate, this
    /// accepts pre-loaded state recovered from disk. DNS and service
    /// registration still happen (they're ephemeral, in-memory only).
    pub async fn resume(
        node_coordinate: MatrixCoordinate,
        blockchain: Arc<NodeBlockchain>,
        genesis_block: Block,
        localhost_cert: LocalhostCertificate,
    ) -> Result<Self> {
        info!(
            "Resuming node at ({}, {}, {}) from persisted state",
            node_coordinate.x, node_coordinate.y, node_coordinate.z
        );

        // DNS + service registration (ephemeral, always rebuilt)
        let dns = DnsResolver::new();
        dns.register(
            "localhost".to_string(),
            IpAddr::from([0, 0, 0, 0, 0, 0, 0, 1]),
        )
        .await;
        for service in LOCAL_SERVICES {
            dns.register(
                service.name.to_string(),
                IpAddr::from(std::net::Ipv6Addr::LOCALHOST),
            )
            .await;
        }

        let privacy_mode = Arc::new(RwLock::new(PrivacyMode::PRIVATE));

        info!(
            "Node resumed — genesis {}, chain height {}",
            genesis_block.hash,
            blockchain.get_height().await,
        );

        Ok(Self {
            genesis_block,
            blockchain,
            localhost_cert,
            dns,
            privacy_mode,
            node_coordinate,
            bootstrapped_at: SystemTime::now(),
        })
    }

    /// Initialize a new node with self-sufficient bootstrap
    ///
    /// This creates:
    /// 1. Unique genesis block for this node
    /// 2. Self-signed localhost certificate
    /// 3. DNS with localhost → ::1
    /// 4. Default to Private mode (no network)
    pub async fn initialize(node_coordinate: MatrixCoordinate) -> Result<Self> {
        Self::initialize_inner(node_coordinate, None, None, None).await
    }

    /// Initialize a new node whose genesis is bound to a canonical device
    /// identity (`BLAKE3(falcon_pubkey)` hex).
    ///
    /// Device-auth invariant: the genesis proofs collapse the three
    /// historical node IDs into `device_node_id` and fold the device
    /// fingerprint (captured from the OS) into all four proofs.
    pub async fn initialize_with_identity(
        node_coordinate: MatrixCoordinate,
        device_node_id: &str,
    ) -> Result<Self> {
        Self::initialize_inner(node_coordinate, Some(device_node_id), None, None).await
    }

    /// H3 variant of [`initialize_with_identity`](Self::initialize_with_identity)
    /// that also attaches a node signer so the fresh chain FALCON-signs the
    /// proof envelope of every locally-produced block.
    ///
    /// S3.0/B1: `block_sink` attaches the chain's durable write-through sink.
    /// The live daemon always supplies one; `None` keeps the memory-only
    /// behaviour used by library callers and tests.
    pub async fn initialize_with_identity_and_signer(
        node_coordinate: MatrixCoordinate,
        device_node_id: &str,
        signer: Arc<dyn hypermesh_lib::NodeSigner + Send + Sync>,
        block_sink: Option<Arc<dyn crate::blockchain::BlockSink>>,
    ) -> Result<Self> {
        Self::initialize_inner(
            node_coordinate,
            Some(device_node_id),
            Some(signer),
            block_sink,
        )
        .await
    }

    async fn initialize_inner(
        node_coordinate: MatrixCoordinate,
        device_node_id: Option<&str>,
        signer: Option<Arc<dyn hypermesh_lib::NodeSigner + Send + Sync>>,
        block_sink: Option<Arc<dyn crate::blockchain::BlockSink>>,
    ) -> Result<Self> {
        info!(
            "Initializing node at ({}, {}, {}) with self-sufficient bootstrap",
            node_coordinate.x, node_coordinate.y, node_coordinate.z
        );

        // 1. Create unique genesis block for THIS node. When a canonical
        //    device identity is supplied, bind the genesis proofs to it.
        let genesis_block = match device_node_id {
            Some(id) => Block::genesis_with_identity(node_coordinate, id),
            None => Block::genesis(node_coordinate),
        };
        info!("Created genesis block: {}", genesis_block.hash);

        // 2. Initialize blockchain with the SAME genesis we just built.
        //    NodeBlockchain::new would call Block::genesis again — and each
        //    call takes a fresh `GenesisEpoch::now()` (S3.0/B2: the epoch is
        //    now the ONE explicit temporal input, rather than three hidden
        //    `SystemTime::now()` reads plus a clock-derived nonce), so the two
        //    genesis blocks would still have different hashes. Block 1 would
        //    chain off the in-memory genesis while disk holds the persisted
        //    one, breaking restart replay.
        //
        //    Block-accept validation uses `default()` StateRequirements.
        //    PoStake is AUTHORIZATION (to whom an asset belongs), not a numeric
        //    magnitude, so the hardening is H3's FALCON signature + signer↔owner
        //    binding — not a raised stake floor.
        let mut chain = NodeBlockchain::from_genesis(node_coordinate, genesis_block.clone());
        // H3: attach the signer (when supplied) so `add_block` FALCON-signs the
        // proof envelope of every locally-produced block.
        if let Some(s) = signer {
            chain = chain.with_signer(s);
        }
        // S3.0/B1: attach the durable block sink so every block added after
        // genesis survives a restart.
        if let Some(sink) = block_sink {
            chain = chain.with_persistence(sink);
        }
        let blockchain = Arc::new(chain);

        // 3. Self-sign certificate for localhost
        let localhost_cert = Self::generate_fresh_certificate()?;
        info!("Generated self-signed localhost certificate");

        // 4. Initialize DNS with localhost → ::1
        let dns = DnsResolver::new();
        dns.register(
            "localhost".to_string(),
            IpAddr::from([0, 0, 0, 0, 0, 0, 0, 1]), // ::1
        )
        .await;
        info!("DNS initialized with localhost → ::1");

        // Register well-known local services
        for service in LOCAL_SERVICES {
            dns.register(
                service.name.to_string(),
                IpAddr::from(std::net::Ipv6Addr::LOCALHOST),
            )
            .await;
        }
        info!("Registered {} local service DNS entries", LOCAL_SERVICES.len());

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

    /// Generate a fresh self-signed certificate for localhost.
    ///
    /// Public so the binary can generate a fallback certificate when
    /// resuming a node whose certificate file is missing.
    pub fn generate_fresh_certificate() -> Result<LocalhostCertificate> {
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
            // Participate in state proof validation
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
        // TODO: Enable state proof participation
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
        assert!(bootstrap.genesis_block().belongs_to_node(&coord));

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

    /// Regression test for the double-genesis chain integrity bug:
    /// `bootstrap.genesis_block` is persisted to disk while
    /// `bootstrap.blockchain.head` is used to compute block 1's
    /// `previous_hash`. If the two diverge (because `Block::genesis` is
    /// non-deterministic via TimeProof's SystemTime+nonce), reload after
    /// restart fails with "Chain integrity violation at block 1".
    #[tokio::test]
    async fn genesis_in_bootstrap_matches_blockchain_head() {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let bootstrap = NodeBootstrap::initialize(coord)
            .await
            .expect("test: initialize");

        let head = bootstrap
            .blockchain()
            .get_head()
            .await
            .expect("test: head must exist immediately after initialize");

        assert_eq!(
            bootstrap.genesis_block().hash,
            head.hash,
            "bootstrap.genesis_block.hash must equal blockchain.head.hash; \
             otherwise block 1 will be persisted with previous_hash that \
             disagrees with the on-disk genesis, breaking restart replay"
        );
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

    #[test]
    fn test_select_certificate_strategy_anonymous() {
        let strategy = select_certificate_strategy(
            &PrivacyMode::ANONYMOUS,
            "node-1",
            "localhost",
            vec![std::net::Ipv6Addr::LOCALHOST],
        );
        assert_eq!(strategy.strategy_name(), "Anonymous");
        assert!(!strategy.requires_certificate());
    }

    #[test]
    fn test_select_certificate_strategy_private() {
        let strategy = select_certificate_strategy(
            &PrivacyMode::PRIVATE,
            "node-1",
            "localhost",
            vec![std::net::Ipv6Addr::LOCALHOST],
        );
        assert_eq!(strategy.strategy_name(), "Private");
        assert!(strategy.requires_certificate());
    }

    #[test]
    fn test_select_certificate_strategy_public() {
        let strategy = select_certificate_strategy(
            &PrivacyMode::PUBLIC,
            "node-1",
            "localhost",
            vec![std::net::Ipv6Addr::LOCALHOST],
        );
        assert_eq!(strategy.strategy_name(), "Public");
        assert!(strategy.requires_certificate());
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
        assert!(bootstrap1.genesis_block().belongs_to_node(&coord1));
        assert!(bootstrap2.genesis_block().belongs_to_node(&coord2));
    }
}
