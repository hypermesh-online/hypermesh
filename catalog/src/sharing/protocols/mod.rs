// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Sharing Protocols Module
//!
//! Implements secure sharing protocols over STOQ with permissions,
//! bandwidth management, and incentive mechanisms.

mod types;
mod transfers;

pub use types::*;

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use std::time::SystemTime;

use crate::AssetRegistration;
use super::PeerInfo;

/// Sharing protocol implementation
pub struct SharingProtocol {
    pub(super) max_bandwidth: u64,
    pub(super) fair_use_limit: u64,
    pub(super) _bandwidth_allocation: Arc<BandwidthAllocation>,
    pub(super) active_transfers: Arc<RwLock<HashMap<String, ActiveTransfer>>>,
    pub(super) peer_connections: Arc<RwLock<HashMap<String, PeerConnection>>>,
    pub(super) contribution_stats: Arc<RwLock<HashMap<String, ContributionStats>>>,
    pub(super) package_permissions: Arc<RwLock<HashMap<String, SharePermission>>>,
    pub(super) upload_limiter: Arc<Semaphore>,
    pub(super) download_limiter: Arc<Semaphore>,
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
            _bandwidth_allocation: bandwidth_allocation,
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
        let hash = blake3::hash(address.as_bytes());
        let peer_id = format!("peer_{}", hex::encode(&hash.as_bytes()[..8]));

        let connection = PeerConnection {
            _peer_id: peer_id.clone(),
            _address: address.to_string(),
            _connected_at: SystemTime::now(),
            allocated_bandwidth: self.fair_use_limit,
            permission: SharePermission::Public,
            _active_transfers: Vec::new(),
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
            trust_weight: 0.5,
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
            duration: std::time::Duration::from_secs(60),
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
