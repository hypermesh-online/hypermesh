// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use anyhow::Result;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use super::types::*;
use crate::{AssetPackage, AssetRegistration};

impl super::SharingProtocol {
    /// Download package from peer
    pub async fn download_package(&self, asset_id: &str, peer_id: &str) -> Result<AssetPackage> {
        // Check peer connection
        let connections = self.peer_connections.read().await;
        let connection = connections
            .get(peer_id)
            .ok_or_else(|| anyhow::anyhow!("Peer not connected"))?;

        // Check permissions
        self.check_permission(&connection.permission, peer_id)
            .await?;

        // Create transfer
        let transfer_id = uuid::Uuid::new_v4().to_string();
        // Parse asset_id from string to AssetRegistration
        let parsed_asset_id = AssetRegistration::from_hex_string(asset_id).unwrap_or_else(|_| {
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
                    },
                ),
            )
        });
        let transfer = ActiveTransfer {
            _id: transfer_id.clone(),
            peer_id: peer_id.to_string(),
            asset_id: parsed_asset_id.clone(),
            _direction: TransferDirection::Download,
            _priority: TransferPriority::Normal,
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
        let package = self
            .receive_package_with_limiting(peer_id, &parsed_asset_id)
            .await?;

        // Update stats
        self.update_contribution_stats(peer_id, package.size(), false)
            .await?;

        // Clean up transfer
        transfers.remove(&transfer_id);

        Ok(package)
    }

    /// Upload package to peer
    pub async fn upload_package(&self, package: &AssetPackage, peer_id: &str) -> Result<()> {
        // Check connection
        let connections = self.peer_connections.read().await;
        let connection = connections
            .get(peer_id)
            .ok_or_else(|| anyhow::anyhow!("Peer not connected"))?;

        // Check permissions
        self.check_permission(&connection.permission, peer_id)
            .await?;

        // Create transfer
        let transfer_id = uuid::Uuid::new_v4().to_string();
        // Parse package hash to AssetRegistration
        let parsed_asset_id = AssetRegistration::from_hex_string(&package.package_hash)
            .unwrap_or_else(|_| {
                // Fallback: create from hash bytes
                let mut hash_bytes = [0u8; 32];
                if let Ok(bytes) = hex::decode(&package.package_hash) {
                    hash_bytes[..bytes.len().min(32)]
                        .copy_from_slice(&bytes[..bytes.len().min(32)]);
                }
                AssetRegistration::new_from_hash(&hash_bytes)
            });
        let transfer = ActiveTransfer {
            _id: transfer_id.clone(),
            peer_id: peer_id.to_string(),
            asset_id: parsed_asset_id,
            _direction: TransferDirection::Upload,
            _priority: TransferPriority::Normal,
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
        self.update_contribution_stats(peer_id, package.size(), true)
            .await?;

        // Clean up transfer
        transfers.remove(&transfer_id);

        Ok(())
    }

    /// Process incoming protocol messages
    pub async fn process_message(
        &self,
        peer_id: &str,
        message: ProtocolMessage,
    ) -> Result<Option<ProtocolMessage>> {
        match message {
            ProtocolMessage::RequestPackage {
                asset_id,
                requester,
            } => {
                // Check permissions for the requested package
                let permissions = self.package_permissions.read().await;
                if let Some(perm) = permissions.get(&asset_id) {
                    if let Err(e) = self.check_permission(perm, &requester).await {
                        return Ok(Some(ProtocolMessage::Error {
                            code: 403,
                            message: format!("Permission denied: {e}"),
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
                    message: format!("Package '{asset_id}' not found locally"),
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
            ProtocolMessage::TransferAck {
                transfer_id,
                received_bytes,
            } => {
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

            stats.insert(
                id.clone(),
                TransferStats {
                    peer_id: transfer.peer_id.clone(),
                    asset_id: transfer.asset_id.clone(),
                    progress: transfer.bytes_transferred as f64 / transfer.total_size as f64,
                    speed,
                    estimated_time: if speed > 0 {
                        Duration::from_secs(
                            (transfer.total_size - transfer.bytes_transferred) / speed,
                        )
                    } else {
                        Duration::from_secs(0)
                    },
                },
            );
        }

        stats
    }

    // Helper methods

    pub(super) async fn check_permission(
        &self,
        permission: &SharePermission,
        peer_id: &str,
    ) -> Result<()> {
        match permission {
            SharePermission::Public => Ok(()),
            SharePermission::Private => Err(anyhow::anyhow!("Private package")),
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
                    Err(anyhow::anyhow!(
                        "Peer '{peer_id}' is not in trusted peers list"
                    ))
                }
            }
            SharePermission::Anonymous => Ok(()),
            SharePermission::Verified => {
                let connections = self.peer_connections.read().await;
                match connections.get(peer_id) {
                    Some(conn) if conn.quality_score > 0.0 => Ok(()),
                    Some(_) => Err(anyhow::anyhow!(
                        "Peer '{peer_id}' has no valid certificate verification"
                    )),
                    None => Err(anyhow::anyhow!(
                        "Peer '{peer_id}' is not connected; cannot verify certificate"
                    )),
                }
            }
        }
    }

    pub(super) async fn send_message(&self, peer_id: &str, message: ProtocolMessage) -> Result<()> {
        let serialized_size = serde_json::to_vec(&message)
            .map(|v| v.len() as u64)
            .unwrap_or(0);
        self.update_contribution_stats(peer_id, serialized_size, true)
            .await?;
        tracing::debug!(
            peer_id = %peer_id,
            bytes = serialized_size,
            "Message queued; network send deferred (requires STOQ)"
        );
        Ok(())
    }

    pub(super) async fn receive_package_with_limiting(
        &self,
        peer_id: &str,
        asset_id: &AssetRegistration,
    ) -> Result<AssetPackage> {
        let permits_needed = 10;
        let _permit = self
            .download_limiter
            .acquire_many(permits_needed as u32)
            .await?;

        Err(anyhow::anyhow!(
            "Awaiting network transfer from peer '{}' for asset '{}'; \
             requires STOQ transport (not available locally)",
            peer_id,
            asset_id.to_hex_string()
        ))
    }

    pub(super) async fn send_package_with_limiting(
        &self,
        package: &AssetPackage,
        peer_id: &str,
    ) -> Result<()> {
        let package_size = package.size();
        let permits_needed = ((package_size / 1024) + 1).min(u32::MAX as u64) as u32;
        let _permit = self.upload_limiter.acquire_many(permits_needed).await?;

        self.update_contribution_stats(peer_id, package_size, true)
            .await?;
        tracing::debug!(
            peer_id = %peer_id,
            package_id = %package.id(),
            bytes = package_size,
            "Package queued for upload; network send deferred (requires STOQ)"
        );
        Ok(())
    }

    pub(super) async fn update_contribution_stats(
        &self,
        peer_id: &str,
        bytes: u64,
        is_upload: bool,
    ) -> Result<()> {
        let mut stats = self.contribution_stats.write().await;
        let entry = stats
            .entry(peer_id.to_string())
            .or_insert_with(Default::default);

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

    pub(super) async fn get_available_bandwidth(&self) -> Result<u64> {
        let transfers = self.active_transfers.read().await;
        let used_bandwidth: u64 = transfers.values().map(|t| t.current_bandwidth).sum();

        Ok(self.max_bandwidth.saturating_sub(used_bandwidth))
    }

    pub(super) fn get_local_id(&self) -> String {
        "local".to_string()
    }
}
