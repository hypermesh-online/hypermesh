//! P2P Distribution Module for Catalog
//!
//! Provides decentralized package distribution using STOQ protocol and DHT-based discovery

pub mod stoq_transport;
pub mod dht;
pub mod content_addressing;
pub mod package_manager;
pub mod peer_discovery;

use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

use crate::assets::{AssetPackage, AssetPackageId};
use crate::security::{SecurityManager, SecurityConfig, VerificationResult};
use stoq_transport::StoqTransportLayer;
use dht::{DhtNetwork, NodeId};
use content_addressing::{ContentAddress, MerkleTree};
use package_manager::PackageManager;
use peer_discovery::PeerDiscovery;

/// P2P Distribution system for Catalog assets
pub struct P2PDistribution {
    /// STOQ transport layer for P2P communication
    transport: Arc<StoqTransportLayer>,
    /// DHT network for package discovery
    dht: Arc<DhtNetwork>,
    /// Content addressing system
    content_store: Arc<ContentStore>,
    /// Package manager for local storage
    package_manager: Arc<PackageManager>,
    /// Peer discovery service
    peer_discovery: Arc<PeerDiscovery>,
    /// Security manager for package verification
    security_manager: Arc<SecurityManager>,
    /// Distribution configuration
    config: DistributionConfig,
    /// Active transfers
    active_transfers: Arc<RwLock<HashMap<AssetPackageId, TransferState>>>,
    /// Performance metrics
    metrics: Arc<DistributionMetrics>,
}

/// Distribution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionConfig {
    /// Local storage directory
    pub storage_dir: PathBuf,
    /// Maximum concurrent transfers
    pub max_concurrent_transfers: usize,
    /// Chunk size for package splitting (bytes)
    pub chunk_size: usize,
    /// Replication factor for packages
    pub replication_factor: usize,
    /// DHT bootstrap nodes
    pub bootstrap_nodes: Vec<String>,
    /// Enable bandwidth management
    pub enable_bandwidth_management: bool,
    /// Maximum upload bandwidth (bytes/sec)
    pub max_upload_bandwidth: Option<u64>,
    /// Maximum download bandwidth (bytes/sec)
    pub max_download_bandwidth: Option<u64>,
    /// Enable package seeding after download
    pub auto_seed: bool,
    /// Package cache size (number of packages)
    pub cache_size: usize,
    /// Enable incremental updates
    pub enable_incremental_updates: bool,
    /// NAT traversal configuration
    pub nat_traversal: NatTraversalConfig,
    /// Security configuration
    pub security: SecurityConfig,
    /// Enforce signature verification
    pub require_signatures: bool,
    /// Allow packages from unverified publishers
    pub allow_unverified_publishers: bool,
}

/// NAT traversal configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatTraversalConfig {
    /// Enable UPnP port mapping
    pub enable_upnp: bool,
    /// Enable STUN for NAT detection
    pub enable_stun: bool,
    /// STUN servers
    pub stun_servers: Vec<String>,
    /// Enable relay fallback
    pub enable_relay: bool,
    /// Relay servers
    pub relay_servers: Vec<String>,
}

/// Transfer state for active package transfers
#[derive(Debug, Clone)]
pub struct TransferState {
    /// Package ID being transferred
    pub package_id: AssetPackageId,
    /// Transfer direction
    pub direction: TransferDirection,
    /// Current progress (0.0 - 1.0)
    pub progress: f64,
    /// Transfer speed (bytes/sec)
    pub speed: u64,
    /// Peers involved in transfer
    pub peers: Vec<NodeId>,
    /// Transfer start time
    pub started_at: std::time::Instant,
    /// Estimated time remaining
    pub eta: Option<std::time::Duration>,
    /// Transfer status
    pub status: TransferStatus,
}

/// Transfer direction
#[derive(Debug, Clone, PartialEq)]
pub enum TransferDirection {
    Upload,
    Download,
    Bidirectional,
}

/// Transfer status
#[derive(Debug, Clone, PartialEq)]
pub enum TransferStatus {
    Pending,
    Active,
    Paused,
    Completed,
    Failed(String),
    Cancelled,
}

/// Content store for content-addressed storage
pub struct ContentStore {
    /// Storage backend
    storage: Arc<dyn ContentStorage>,
    /// Content index
    index: Arc<RwLock<ContentIndex>>,
    /// Merkle trees for packages
    merkle_trees: Arc<RwLock<HashMap<AssetPackageId, MerkleTree>>>,
}

/// Content index for fast lookups
#[derive(Debug, Default)]
pub struct ContentIndex {
    /// Content addresses by package ID
    pub by_package: HashMap<AssetPackageId, Vec<ContentAddress>>,
    /// Package IDs by content address
    pub by_content: HashMap<ContentAddress, Vec<AssetPackageId>>,
    /// Content metadata
    pub metadata: HashMap<ContentAddress, ContentMetadata>,
}

/// Content metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentMetadata {
    /// Content size in bytes
    pub size: u64,
    /// Content hash (SHA-256)
    pub hash: String,
    /// Content type
    pub content_type: String,
    /// Compression algorithm used
    pub compression: Option<CompressionType>,
    /// Encryption algorithm used
    pub encryption: Option<EncryptionType>,
    /// Number of chunks
    pub chunk_count: usize,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Compression type for content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionType {
    None,
    Gzip,
    Zstd,
    Lz4,
    Brotli,
}

/// Encryption type for content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionType {
    None,
    Aes256Gcm,
    ChaCha20Poly1305,
}

/// Content storage trait
#[async_trait::async_trait]
pub trait ContentStorage: Send + Sync {
    /// Store content chunk
    async fn store_chunk(&self, address: &ContentAddress, data: &[u8]) -> Result<()>;

    /// Retrieve content chunk
    async fn get_chunk(&self, address: &ContentAddress) -> Result<Option<Vec<u8>>>;

    /// Check if content exists
    async fn has_chunk(&self, address: &ContentAddress) -> Result<bool>;

    /// Delete content chunk
    async fn delete_chunk(&self, address: &ContentAddress) -> Result<()>;

    /// Get storage statistics
    async fn get_stats(&self) -> Result<StorageStats>;
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    /// Total storage used (bytes)
    pub used_space: u64,
    /// Available space (bytes)
    pub available_space: u64,
    /// Number of stored chunks
    pub chunk_count: usize,
    /// Number of stored packages
    pub package_count: usize,
}

/// Distribution metrics
#[derive(Debug, Default)]
pub struct DistributionMetrics {
    /// Total bytes uploaded
    pub bytes_uploaded: Arc<std::sync::atomic::AtomicU64>,
    /// Total bytes downloaded
    pub bytes_downloaded: Arc<std::sync::atomic::AtomicU64>,
    /// Active upload connections
    pub active_uploads: Arc<std::sync::atomic::AtomicUsize>,
    /// Active download connections
    pub active_downloads: Arc<std::sync::atomic::AtomicUsize>,
    /// Successful transfers
    pub successful_transfers: Arc<std::sync::atomic::AtomicU64>,
    /// Failed transfers
    pub failed_transfers: Arc<std::sync::atomic::AtomicU64>,
    /// Average transfer speed (bytes/sec)
    pub avg_transfer_speed: Arc<RwLock<f64>>,
    /// Connected peers
    pub connected_peers: Arc<std::sync::atomic::AtomicUsize>,
}

impl P2PDistribution {
    /// Create a new P2P distribution system
    pub async fn new(config: DistributionConfig) -> Result<Self> {
        // Initialize security manager
        let security_manager = Arc::new(
            SecurityManager::new(config.security.clone())
                .await
                .context("Failed to initialize security manager")?
        );

        // Initialize STOQ transport layer
        let transport = Arc::new(
            StoqTransportLayer::new(config.clone())
                .await
                .context("Failed to initialize STOQ transport")?
        );

        // Initialize DHT network
        let dht = Arc::new(
            DhtNetwork::new(
                transport.clone(),
                config.bootstrap_nodes.clone(),
            )
            .await
            .context("Failed to initialize DHT network")?
        );

        // Initialize content store
        let storage = create_storage_backend(&config)?;
        let content_store = Arc::new(ContentStore {
            storage: Arc::new(storage),
            index: Arc::new(RwLock::new(ContentIndex::default())),
            merkle_trees: Arc::new(RwLock::new(HashMap::new())),
        });

        // Initialize package manager
        let package_manager = Arc::new(
            PackageManager::new(
                content_store.clone(),
                config.storage_dir.clone(),
            )
            .await?
        );

        // Initialize peer discovery
        let peer_discovery = Arc::new(
            PeerDiscovery::new(
                transport.clone(),
                dht.clone(),
            )
            .await?
        );

        Ok(Self {
            transport,
            dht,
            content_store,
            package_manager,
            peer_discovery,
            security_manager,
            config,
            active_transfers: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(DistributionMetrics::default()),
        })
    }

    /// Publish a package to the P2P network
    pub async fn publish(&self, package: AssetPackage) -> Result<AssetPackageId> {
        let package_id = package.get_package_id();

        // Store package locally
        let content_addresses = self.package_manager
            .store_package(&package)
            .await
            .context("Failed to store package locally")?;

        // Create and store Merkle tree
        let merkle_tree = MerkleTree::from_package(&package)?;
        {
            let mut trees = self.content_store.merkle_trees.write().await;
            trees.insert(package_id, merkle_tree.clone());
        }

        // Announce package availability on DHT
        self.dht
            .announce_package(package_id, content_addresses.clone())
            .await
            .context("Failed to announce package on DHT")?;

        // Start seeding if auto-seed is enabled
        if self.config.auto_seed {
            self.start_seeding(package_id).await?;
        }

        // Update metrics
        self.metrics.bytes_uploaded.fetch_add(
            package.calculate_size() as u64,
            std::sync::atomic::Ordering::Relaxed,
        );

        Ok(package_id)
    }

    /// Download a package from the P2P network
    pub async fn download(&self, package_id: &AssetPackageId) -> Result<AssetPackage> {
        // Discover peers with the package
        let peers = self.dht
            .find_package_peers(package_id)
            .await
            .context("Failed to find package peers")?;

        if peers.is_empty() {
            return Err(anyhow::anyhow!("No peers found with package {}", package_id));
        }

        // Create transfer state
        let transfer_state = TransferState {
            package_id: *package_id,
            direction: TransferDirection::Download,
            progress: 0.0,
            speed: 0,
            peers: peers.clone(),
            started_at: std::time::Instant::now(),
            eta: None,
            status: TransferStatus::Active,
        };

        {
            let mut transfers = self.active_transfers.write().await;
            transfers.insert(*package_id, transfer_state);
        }

        // Download package chunks from peers
        let package = self.package_manager
            .download_from_peers(package_id, &peers, self.transport.clone())
            .await
            .context("Failed to download package from peers")?;

        // Verify package integrity using Merkle tree
        self.verify_package_integrity(&package).await?;

        // Verify package signature and security
        if self.config.require_signatures {
            let verification_result = self.security_manager
                .verify_package(&package)
                .await
                .context("Failed to verify package security")?;

            if !verification_result.verified {
                // Update transfer state to failed
                {
                    let mut transfers = self.active_transfers.write().await;
                    if let Some(state) = transfers.get_mut(package_id) {
                        state.status = TransferStatus::Failed(
                            "Package signature verification failed".to_string()
                        );
                    }
                }

                self.metrics.failed_transfers.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                return Err(anyhow::anyhow!(
                    "Package verification failed: {:?}",
                    verification_result.errors
                ));
            }

            // Check if publisher is allowed
            if !self.config.allow_unverified_publishers && verification_result.publisher.is_none() {
                return Err(anyhow::anyhow!("Package from unverified publisher not allowed"));
            }

            // Update publisher reputation based on successful download
            if let Some(publisher) = verification_result.publisher {
                self.security_manager
                    .update_reputation(&publisher.trustchain_id, true, None)
                    .await?;
            }
        }

        // Update transfer state
        {
            let mut transfers = self.active_transfers.write().await;
            if let Some(state) = transfers.get_mut(package_id) {
                state.status = TransferStatus::Completed;
                state.progress = 1.0;
            }
        }

        // Start seeding if auto-seed is enabled
        if self.config.auto_seed {
            self.start_seeding(*package_id).await?;
        }

        // Update metrics
        self.metrics.bytes_downloaded.fetch_add(
            package.calculate_size() as u64,
            std::sync::atomic::Ordering::Relaxed,
        );
        self.metrics.successful_transfers.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Ok(package)
    }

    /// Search for packages using DHT
    pub async fn search(&self, query: &str) -> Result<Vec<AssetPackageId>> {
        self.dht
            .search_packages(query)
            .await
            .context("Failed to search packages on DHT")
    }

    /// Get current transfer status
    pub async fn get_transfer_status(&self, package_id: &AssetPackageId) -> Option<TransferState> {
        let transfers = self.active_transfers.read().await;
        transfers.get(package_id).cloned()
    }

    /// Cancel an active transfer
    pub async fn cancel_transfer(&self, package_id: &AssetPackageId) -> Result<()> {
        let mut transfers = self.active_transfers.write().await;
        if let Some(state) = transfers.get_mut(package_id) {
            state.status = TransferStatus::Cancelled;
            // TODO: Clean up active connections
            Ok(())
        } else {
            Err(anyhow::anyhow!("No active transfer for package {}", package_id))
        }
    }

    /// Start seeding a package
    async fn start_seeding(&self, package_id: AssetPackageId) -> Result<()> {
        // Register as a seeder in DHT
        self.dht
            .register_as_seeder(package_id)
            .await
            .context("Failed to register as seeder")?;

        // Start listening for download requests
        self.transport
            .listen_for_package_requests(package_id, self.package_manager.clone())
            .await?;

        Ok(())
    }

    /// Verify package integrity using Merkle tree
    async fn verify_package_integrity(&self, package: &AssetPackage) -> Result<()> {
        let package_id = package.get_package_id();

        let trees = self.content_store.merkle_trees.read().await;
        if let Some(merkle_tree) = trees.get(&package_id) {
            merkle_tree.verify_package(package)?;
        } else {
            return Err(anyhow::anyhow!("Merkle tree not found for package {}", package_id));
        }

        Ok(())
    }

    /// Get distribution metrics
    pub fn get_metrics(&self) -> &DistributionMetrics {
        &self.metrics
    }

    /// Get connected peers count
    pub async fn get_peer_count(&self) -> usize {
        self.peer_discovery.get_connected_peers().await.len()
    }
}

/// Create storage backend based on configuration
fn create_storage_backend(config: &DistributionConfig) -> Result<impl ContentStorage> {
    // For now, use a simple file-based storage
    // This can be extended to support different backends (RocksDB, S3, etc.)
    FileBasedStorage::new(config.storage_dir.clone())
}

/// File-based content storage implementation
pub struct FileBasedStorage {
    base_path: PathBuf,
}

impl FileBasedStorage {
    fn new(base_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&base_path)?;
        Ok(Self { base_path })
    }

    fn get_chunk_path(&self, address: &ContentAddress) -> PathBuf {
        let hex = address.to_hex();
        // Use first 2 chars as directory for better file system performance
        self.base_path
            .join(&hex[..2])
            .join(&hex[2..4])
            .join(&hex)
    }
}

#[async_trait::async_trait]
impl ContentStorage for FileBasedStorage {
    async fn store_chunk(&self, address: &ContentAddress, data: &[u8]) -> Result<()> {
        let path = self.get_chunk_path(address);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(path, data).await?;
        Ok(())
    }

    async fn get_chunk(&self, address: &ContentAddress) -> Result<Option<Vec<u8>>> {
        let path = self.get_chunk_path(address);
        if path.exists() {
            let data = tokio::fs::read(path).await?;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    async fn has_chunk(&self, address: &ContentAddress) -> Result<bool> {
        Ok(self.get_chunk_path(address).exists())
    }

    async fn delete_chunk(&self, address: &ContentAddress) -> Result<()> {
        let path = self.get_chunk_path(address);
        if path.exists() {
            tokio::fs::remove_file(path).await?;
        }
        Ok(())
    }

    async fn get_stats(&self) -> Result<StorageStats> {
        // Simple implementation - can be optimized
        let mut used_space = 0u64;
        let mut chunk_count = 0usize;

        let mut entries = tokio::fs::read_dir(&self.base_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_file() {
                let metadata = entry.metadata().await?;
                used_space += metadata.len();
                chunk_count += 1;
            }
        }

        Ok(StorageStats {
            used_space,
            available_space: 0, // TODO: Get actual available space
            chunk_count,
            package_count: 0, // TODO: Track package count
        })
    }
}

impl Default for DistributionConfig {
    fn default() -> Self {
        Self {
            storage_dir: PathBuf::from("~/.catalog/p2p"),
            max_concurrent_transfers: 10,
            chunk_size: 1024 * 1024, // 1MB chunks
            replication_factor: 3,
            bootstrap_nodes: vec![
                "stoq://bootstrap1.hypermesh.online:8080".to_string(),
                "stoq://bootstrap2.hypermesh.online:8080".to_string(),
            ],
            enable_bandwidth_management: true,
            max_upload_bandwidth: None,
            max_download_bandwidth: None,
            auto_seed: true,
            cache_size: 100,
            enable_incremental_updates: true,
            nat_traversal: NatTraversalConfig {
                enable_upnp: true,
                enable_stun: true,
                stun_servers: vec![
                    "stun.hypermesh.online:3478".to_string(),
                ],
                enable_relay: true,
                relay_servers: vec![
                    "relay.hypermesh.online:8081".to_string(),
                ],
            },
            security: SecurityConfig::default(),
            require_signatures: true,
            allow_unverified_publishers: false,
        }
    }
}

// Helper trait for AssetPackage
impl AssetPackage {
    /// Calculate total package size
    pub fn calculate_size(&self) -> usize {
        let mut size = 0;

        // Add main content size
        size += self.content.main_content.len();

        // Add file contents
        for content in self.content.file_contents.values() {
            size += content.len();
        }

        // Add binary contents
        for content in self.content.binary_contents.values() {
            size += content.len();
        }

        size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_p2p_distribution_creation() {
        let config = DistributionConfig::default();
        let distribution = P2PDistribution::new(config).await;
        assert!(distribution.is_ok());
    }
}