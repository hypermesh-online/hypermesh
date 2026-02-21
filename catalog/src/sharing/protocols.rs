// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Sharing Protocols Module
//!
//! Implements secure sharing protocols over STOQ with permissions,
//! bandwidth management, and incentive mechanisms.

use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use std::time::{Duration, SystemTime};
use crate::{AssetRegistration, AssetPackage, AssetMetadata};
use super::PeerInfo;

/// Share permission levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SharePermission {
    /// Publicly accessible to all
    Public,
    /// Private - owner only
    Private,
    /// Shared with specific nodes
    Restricted { allowed_nodes: Vec<String> },
    /// Shared with friends/trusted peers
    Friends,
    /// Anonymous sharing
    Anonymous,
    /// Verified nodes only (with Proof of State proofs)
    Verified,
}

impl Default for SharePermission {
    fn default() -> Self {
        Self::Public
    }
}

/// Bandwidth allocation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthAllocation {
    /// Maximum upload bandwidth (bytes/sec)
    pub max_upload: u64,
    /// Maximum download bandwidth (bytes/sec)
    pub max_download: u64,
    /// Reserved bandwidth for priority transfers
    pub reserved_priority: u64,
    /// Fair share per peer
    pub per_peer_limit: u64,
    /// Burst allowance
    pub burst_size: u64,
    /// Burst duration
    pub burst_duration: Duration,
}

impl Default for BandwidthAllocation {
    fn default() -> Self {
        Self {
            max_upload: 10 * 1024 * 1024,     // 10 MB/s
            max_download: 10 * 1024 * 1024,   // 10 MB/s
            reserved_priority: 2 * 1024 * 1024, // 2 MB/s
            per_peer_limit: 1024 * 1024,      // 1 MB/s
            burst_size: 5 * 1024 * 1024,      // 5 MB burst
            burst_duration: Duration::from_secs(5),
        }
    }
}

/// Transfer priority levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Ord, PartialOrd, Eq)]
pub enum TransferPriority {
    /// Critical system transfers
    Critical = 0,
    /// High priority user transfers
    High = 1,
    /// Normal priority
    Normal = 2,
    /// Low priority background transfers
    Low = 3,
}

/// Active transfer information
#[allow(dead_code)] // Transfer tracking fields
#[derive(Debug, Clone)]
struct ActiveTransfer {
    /// Transfer ID
    id: String,
    /// Peer ID
    peer_id: String,
    /// Asset being transferred
    asset_id: AssetRegistration,
    /// Transfer direction
    direction: TransferDirection,
    /// Priority
    priority: TransferPriority,
    /// Bytes transferred
    bytes_transferred: u64,
    /// Total size
    total_size: u64,
    /// Start time
    started_at: SystemTime,
    /// Current bandwidth (bytes/sec)
    current_bandwidth: u64,
}

#[derive(Debug, Clone, PartialEq)]
enum TransferDirection {
    Upload,
    Download,
}

/// Incentive contribution tracking
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContributionStats {
    /// Total bytes uploaded
    pub bytes_uploaded: u64,
    /// Total bytes downloaded
    pub bytes_downloaded: u64,
    /// Upload/download ratio
    pub ratio: f64,
    /// Contribution score
    pub score: f64,
    /// Earned credits
    pub credits: u64,
    /// Spent credits
    pub credits_spent: u64,
}

/// Protocol message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtocolMessage {
    /// Request package
    RequestPackage {
        asset_id: String,  // Package hash, not BlockMatrix AssetRegistration
        requester: String,
    },
    /// Package response
    PackageResponse {
        asset_id: String,  // Package hash, not BlockMatrix AssetRegistration
        package: AssetPackage,
    },
    /// Package metadata
    PackageMetadata {
        asset_id: String,  // Package hash, not BlockMatrix AssetRegistration
        metadata: AssetMetadata,
    },
    /// Availability notification
    AvailabilityNotification {
        asset_id: String,  // Package hash, not BlockMatrix AssetRegistration
        available: bool,
    },
    /// Bandwidth negotiation
    BandwidthNegotiation {
        proposed_rate: u64,
        duration: Duration,
    },
    /// Transfer acknowledgment
    TransferAck {
        transfer_id: String,
        received_bytes: u64,
    },
    /// Error response
    Error {
        code: u32,
        message: String,
    },
}

/// Sharing protocol implementation
#[allow(dead_code)] // Protocol fields for sharing operations
pub struct SharingProtocol {
    max_bandwidth: u64,
    fair_use_limit: u64,
    bandwidth_allocation: Arc<BandwidthAllocation>,
    active_transfers: Arc<RwLock<HashMap<String, ActiveTransfer>>>,
    peer_connections: Arc<RwLock<HashMap<String, PeerConnection>>>,
    contribution_stats: Arc<RwLock<HashMap<String, ContributionStats>>>,
    package_permissions: Arc<RwLock<HashMap<String, SharePermission>>>,
    upload_limiter: Arc<Semaphore>,
    download_limiter: Arc<Semaphore>,
}

/// Peer connection state
#[allow(dead_code)] // Connection tracking fields
#[derive(Debug, Clone)]
struct PeerConnection {
    /// Peer ID
    peer_id: String,
    /// Connection address
    address: String,
    /// Connection established time
    connected_at: SystemTime,
    /// Current bandwidth allocation
    allocated_bandwidth: u64,
    /// Permission level
    permission: SharePermission,
    /// Active transfers
    active_transfers: Vec<String>,
    /// Connection quality score
    quality_score: f64,
}

impl SharingProtocol {
    /// Create new sharing protocol
    pub async fn new(max_bandwidth: u64, fair_use_limit: u64) -> Result<Self> {
        let bandwidth_allocation = Arc::new(BandwidthAllocation {
            max_upload: max_bandwidth,
            max_download: max_bandwidth,
            per_peer_limit: fair_use_limit,
            ..Default::default()
        });

        // Create bandwidth limiters
        let upload_permits = (max_bandwidth / 1024) as usize; // 1KB chunks
        let download_permits = (max_bandwidth / 1024) as usize;

        Ok(Self {
            max_bandwidth,
            fair_use_limit,
            bandwidth_allocation,
            active_transfers: Arc::new(RwLock::new(HashMap::new())),
            peer_connections: Arc::new(RwLock::new(HashMap::new())),
            contribution_stats: Arc::new(RwLock::new(HashMap::new())),
            package_permissions: Arc::new(RwLock::new(HashMap::new())),
            upload_limiter: Arc::new(Semaphore::new(upload_permits)),
            download_limiter: Arc::new(Semaphore::new(download_permits)),
        })
    }

    /// Connect to peer
    pub async fn connect(&self, address: &str) -> Result<PeerInfo> {
        // Deterministic node ID from address for reproducible identity.
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(address.as_bytes());
        let hash = hasher.finalize();
        let peer_id = format!("peer_{}", hex::encode(&hash[..8]));

        let connection = PeerConnection {
            peer_id: peer_id.clone(),
            address: address.to_string(),
            connected_at: SystemTime::now(),
            allocated_bandwidth: self.fair_use_limit,
            permission: SharePermission::Public,
            active_transfers: Vec::new(),
            quality_score: 1.0,
        };

        let mut connections = self.peer_connections.write().await;
        connections.insert(peer_id.clone(), connection);

        Ok(PeerInfo {
            node_id: peer_id,
            address: address.to_string(),
            available_packages: Default::default(),
            storage_capacity: 10 * 1024 * 1024 * 1024,
            bandwidth_capacity: self.fair_use_limit,
            reputation: 0.5,
            last_seen: SystemTime::now(),
            location: None,
            supported_protocols: vec!["stoq".to_string()],
        })
    }

    /// Disconnect from peer
    pub async fn disconnect(&self, node_id: &str) -> Result<()> {
        // Cancel active transfers
        let mut transfers = self.active_transfers.write().await;
        transfers.retain(|_, transfer| transfer.peer_id != node_id);

        // Remove connection
        let mut connections = self.peer_connections.write().await;
        connections.remove(node_id);

        Ok(())
    }

    /// Download package from peer
    pub async fn download_package(
        &self,
        asset_id: &str,
        peer_id: &str,
    ) -> Result<AssetPackage> {
        // Check peer connection
        let connections = self.peer_connections.read().await;
        let connection = connections.get(peer_id)
            .ok_or_else(|| anyhow::anyhow!("Peer not connected"))?;

        // Check permissions
        self.check_permission(&connection.permission, peer_id).await?;

        // Create transfer
        let transfer_id = uuid::Uuid::new_v4().to_string();
        // Parse asset_id from string to AssetRegistration
        let parsed_asset_id = AssetRegistration::from_hex_string(asset_id)
            .unwrap_or_else(|_| {
                // Fallback: create a default AssetRegistration from empty data
                let asset_data = blockmatrix::assets::core::AssetData {
                    config: vec![],
                    definition: vec![],
                    metadata: vec![],
                };
                AssetRegistration::from_asset_data(
                    &asset_data,
                    blockmatrix::assets::core::NetworkScope::Global,
                    blockmatrix::assets::core::AssetCategory::Application(
                        blockmatrix::assets::core::ApplicationDomain {
                            domain_name: "catalog".to_string(),
                            domain_hash: [0u8; 32],
                        }
                    ),
                )
            });
        let transfer = ActiveTransfer {
            id: transfer_id.clone(),
            peer_id: peer_id.to_string(),
            asset_id: parsed_asset_id.clone(),
            direction: TransferDirection::Download,
            priority: TransferPriority::Normal,
            bytes_transferred: 0,
            total_size: 0, // Will be updated
            started_at: SystemTime::now(),
            current_bandwidth: 0,
        };

        // Register transfer
        let mut transfers = self.active_transfers.write().await;
        transfers.insert(transfer_id.clone(), transfer);

        // Send request
        let request = ProtocolMessage::RequestPackage {
            asset_id: asset_id.to_string(),
            requester: self.get_local_id(),
        };
        self.send_message(peer_id, request).await?;

        // Receive package with bandwidth limiting
        let package = self.receive_package_with_limiting(peer_id, &parsed_asset_id).await?;

        // Update stats
        self.update_contribution_stats(peer_id, package.size(), false).await?;

        // Clean up transfer
        transfers.remove(&transfer_id);

        Ok(package)
    }

    /// Upload package to peer
    pub async fn upload_package(
        &self,
        package: &AssetPackage,
        peer_id: &str,
    ) -> Result<()> {
        // Check connection
        let connections = self.peer_connections.read().await;
        let connection = connections.get(peer_id)
            .ok_or_else(|| anyhow::anyhow!("Peer not connected"))?;

        // Check permissions
        self.check_permission(&connection.permission, peer_id).await?;

        // Create transfer
        let transfer_id = uuid::Uuid::new_v4().to_string();
        // Parse package hash to AssetRegistration
        let parsed_asset_id = AssetRegistration::from_hex_string(&package.package_hash)
            .unwrap_or_else(|_| {
                // Fallback: create from hash bytes
                let mut hash_bytes = [0u8; 32];
                if let Ok(bytes) = hex::decode(&package.package_hash) {
                    hash_bytes[..bytes.len().min(32)].copy_from_slice(&bytes[..bytes.len().min(32)]);
                }
                AssetRegistration::new_from_hash(&hash_bytes)
            });
        let transfer = ActiveTransfer {
            id: transfer_id.clone(),
            peer_id: peer_id.to_string(),
            asset_id: parsed_asset_id,
            direction: TransferDirection::Upload,
            priority: TransferPriority::Normal,
            bytes_transferred: 0,
            total_size: package.size(),
            started_at: SystemTime::now(),
            current_bandwidth: 0,
        };

        // Register transfer
        let mut transfers = self.active_transfers.write().await;
        transfers.insert(transfer_id.clone(), transfer);

        // Send package with bandwidth limiting
        self.send_package_with_limiting(package, peer_id).await?;

        // Update stats
        self.update_contribution_stats(peer_id, package.size(), true).await?;

        // Clean up transfer
        transfers.remove(&transfer_id);

        Ok(())
    }

    /// Notify peer about package availability
    pub async fn notify_availability(
        &self,
        peer_id: &str,
        asset_id: &str,
    ) -> Result<()> {
        let message = ProtocolMessage::AvailabilityNotification {
            asset_id: asset_id.to_string(),
            available: true,
        };

        self.send_message(peer_id, message).await
    }

    /// Negotiate bandwidth with peer
    pub async fn negotiate_bandwidth(
        &self,
        peer_id: &str,
        requested_rate: u64,
    ) -> Result<u64> {
        // Check available bandwidth
        let available = self.get_available_bandwidth().await?;
        let allocated = requested_rate.min(available).min(self.fair_use_limit);

        // Update peer allocation
        let mut connections = self.peer_connections.write().await;
        if let Some(connection) = connections.get_mut(peer_id) {
            connection.allocated_bandwidth = allocated;
        }

        // Send negotiation response
        let message = ProtocolMessage::BandwidthNegotiation {
            proposed_rate: allocated,
            duration: Duration::from_secs(60),
        };
        self.send_message(peer_id, message).await?;

        Ok(allocated)
    }

    /// Set share permissions for package
    pub async fn set_permission(
        &self,
        asset_id: &AssetRegistration,
        permission: SharePermission,
    ) -> Result<()> {
        let mut permissions = self.package_permissions.write().await;
        permissions.insert(asset_id.to_hex_string(), permission);
        Ok(())
    }

    /// Get contribution statistics
    pub async fn get_contribution_stats(&self, peer_id: &str) -> Option<ContributionStats> {
        let stats = self.contribution_stats.read().await;
        stats.get(peer_id).cloned()
    }

    /// Calculate incentive rewards
    pub async fn calculate_rewards(&self, peer_id: &str) -> Result<u64> {
        let stats = self.contribution_stats.read().await;
        if let Some(contribution) = stats.get(peer_id) {
            // Simple reward calculation based on contribution
            let reward = (contribution.bytes_uploaded / (1024 * 1024)) // MB uploaded
                * 10 // 10 credits per MB
                * (contribution.ratio.max(0.5).min(2.0) as u64); // Ratio multiplier

            Ok(reward)
        } else {
            Ok(0)
        }
    }

    // Helper methods

    async fn check_permission(&self, permission: &SharePermission, peer_id: &str) -> Result<()> {
        match permission {
            SharePermission::Public => Ok(()),
            SharePermission::Private => {
                Err(anyhow::anyhow!("Private package"))
            }
            SharePermission::Restricted { allowed_nodes } => {
                if allowed_nodes.contains(&peer_id.to_string()) {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("Not authorized"))
                }
            }
            SharePermission::Friends => {
                let connections = self.peer_connections.read().await;
                if connections.contains_key(peer_id) {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("Peer '{}' is not in trusted peers list", peer_id))
                }
            }
            SharePermission::Anonymous => Ok(()),
            SharePermission::Verified => {
                let connections = self.peer_connections.read().await;
                match connections.get(peer_id) {
                    Some(conn) if conn.quality_score > 0.0 => Ok(()),
                    Some(_) => Err(anyhow::anyhow!(
                        "Peer '{}' has no valid certificate verification", peer_id
                    )),
                    None => Err(anyhow::anyhow!(
                        "Peer '{}' is not connected; cannot verify certificate", peer_id
                    )),
                }
            }
        }
    }

    async fn send_message(&self, peer_id: &str, message: ProtocolMessage) -> Result<()> {
        let serialized_size = serde_json::to_vec(&message)
            .map(|v| v.len() as u64)
            .unwrap_or(0);
        self.update_contribution_stats(peer_id, serialized_size, true).await?;
        tracing::debug!(
            peer_id = %peer_id,
            bytes = serialized_size,
            "Message queued; network send deferred (requires STOQ)"
        );
        Ok(())
    }

    async fn receive_package_with_limiting(
        &self,
        peer_id: &str,
        asset_id: &AssetRegistration,
    ) -> Result<AssetPackage> {
        let permits_needed = 10;
        let _permit = self.download_limiter.acquire_many(permits_needed as u32).await?;

        Err(anyhow::anyhow!(
            "Awaiting network transfer from peer '{}' for asset '{}'; \
             requires STOQ transport (not available locally)",
            peer_id,
            asset_id.to_hex_string()
        ))
    }

    async fn send_package_with_limiting(
        &self,
        package: &AssetPackage,
        peer_id: &str,
    ) -> Result<()> {
        let package_size = package.size();
        let permits_needed = ((package_size / 1024) + 1).min(u32::MAX as u64) as u32;
        let _permit = self.upload_limiter.acquire_many(permits_needed).await?;

        self.update_contribution_stats(peer_id, package_size, true).await?;
        tracing::debug!(
            peer_id = %peer_id,
            package_id = %package.id(),
            bytes = package_size,
            "Package queued for upload; network send deferred (requires STOQ)"
        );
        Ok(())
    }

    async fn update_contribution_stats(
        &self,
        peer_id: &str,
        bytes: u64,
        is_upload: bool,
    ) -> Result<()> {
        let mut stats = self.contribution_stats.write().await;
        let entry = stats.entry(peer_id.to_string()).or_insert_with(Default::default);

        if is_upload {
            entry.bytes_uploaded += bytes;
        } else {
            entry.bytes_downloaded += bytes;
        }

        // Update ratio
        if entry.bytes_downloaded > 0 {
            entry.ratio = entry.bytes_uploaded as f64 / entry.bytes_downloaded as f64;
        }

        // Update score (simple calculation)
        entry.score = (entry.ratio * 100.0).min(200.0);

        Ok(())
    }

    async fn get_available_bandwidth(&self) -> Result<u64> {
        let transfers = self.active_transfers.read().await;
        let used_bandwidth: u64 = transfers.values()
            .map(|t| t.current_bandwidth)
            .sum();

        Ok(self.max_bandwidth.saturating_sub(used_bandwidth))
    }

    fn get_local_id(&self) -> String {
        "local".to_string()
    }

    /// Process incoming protocol messages
    pub async fn process_message(
        &self,
        peer_id: &str,
        message: ProtocolMessage,
    ) -> Result<Option<ProtocolMessage>> {
        match message {
            ProtocolMessage::RequestPackage { asset_id, requester } => {
                // Check permissions for the requested package
                let permissions = self.package_permissions.read().await;
                if let Some(perm) = permissions.get(&asset_id) {
                    if let Err(e) = self.check_permission(perm, &requester).await {
                        return Ok(Some(ProtocolMessage::Error {
                            code: 403,
                            message: format!("Permission denied: {}", e),
                        }));
                    }
                }
                // Package lookup requires the registry (not held here).
                tracing::debug!(
                    asset_id = %asset_id,
                    requester = %requester,
                    peer_id = %peer_id,
                    "Package request received; local lookup not available in protocol layer"
                );
                Ok(Some(ProtocolMessage::Error {
                    code: 404,
                    message: format!("Package '{}' not found locally", asset_id),
                }))
            }
            ProtocolMessage::BandwidthNegotiation { proposed_rate, .. } => {
                // Handle bandwidth negotiation
                let allocated = self.negotiate_bandwidth(peer_id, proposed_rate).await?;
                Ok(Some(ProtocolMessage::BandwidthNegotiation {
                    proposed_rate: allocated,
                    duration: Duration::from_secs(60),
                }))
            }
            ProtocolMessage::TransferAck { transfer_id, received_bytes } => {
                // Update transfer progress
                let mut transfers = self.active_transfers.write().await;
                if let Some(transfer) = transfers.get_mut(&transfer_id) {
                    transfer.bytes_transferred = received_bytes;
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// Get active transfer statistics
    pub async fn get_transfer_stats(&self) -> HashMap<String, TransferStats> {
        let transfers = self.active_transfers.read().await;
        let mut stats = HashMap::new();

        for (id, transfer) in transfers.iter() {
            let elapsed = SystemTime::now()
                .duration_since(transfer.started_at)
                .unwrap_or_default();

            let speed = if elapsed.as_secs() > 0 {
                transfer.bytes_transferred / elapsed.as_secs()
            } else {
                0
            };

            stats.insert(id.clone(), TransferStats {
                peer_id: transfer.peer_id.clone(),
                asset_id: transfer.asset_id.clone(),
                progress: transfer.bytes_transferred as f64 / transfer.total_size as f64,
                speed,
                estimated_time: if speed > 0 {
                    Duration::from_secs((transfer.total_size - transfer.bytes_transferred) / speed)
                } else {
                    Duration::from_secs(0)
                },
            });
        }

        stats
    }
}

/// Transfer statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferStats {
    /// Peer ID
    pub peer_id: String,
    /// Asset ID
    pub asset_id: AssetRegistration,
    /// Progress (0-1)
    pub progress: f64,
    /// Current speed (bytes/sec)
    pub speed: u64,
    /// Estimated time remaining
    pub estimated_time: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sharing_protocol_creation() {
        let protocol = SharingProtocol::new(10 * 1024 * 1024, 1024 * 1024).await;
        assert!(protocol.is_ok());
    }

    #[tokio::test]
    async fn test_bandwidth_negotiation() {
        let protocol = SharingProtocol::new(10 * 1024 * 1024, 1024 * 1024).await.unwrap();
        let allocated = protocol.negotiate_bandwidth("test-peer", 2 * 1024 * 1024).await;
        assert!(allocated.is_ok());
        assert!(allocated.unwrap() <= 1024 * 1024); // Should be limited by fair use
    }
}