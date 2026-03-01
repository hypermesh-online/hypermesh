// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Gateway module for cross-scope asset transfers and scope bridging
//!
//! The gateway is a distribution orchestration layer that coordinates moving
//! assets between the Device and Network blockchain scopes. It sits on top of
//! the existing blockchain and asset infrastructure.
//!
//! # Architecture
//!
//! - **GatewayManager** -- top-level coordinator that exposes the public API.
//! - **ScopeBridge** -- handles the lock-transfer-unlock protocol between scopes.
//! - **AssetTransfer** -- data model and validation for individual transfers.
//!
//! Every cross-scope transfer requires Proof of State validation (at minimum
//! PoSpace + PoStake) in both the source and target scopes before the asset is
//! released.

pub mod asset_transfer;
pub mod scope_bridge;

pub use asset_transfer::{
    AssetTransfer, DefaultTransferValidator, TransferStatus, TransferValidator,
};
pub use scope_bridge::{BridgeMessage, ScopeBridge};

use std::sync::Arc;

use hypermesh_lib::{AssetId, BlockchainScope};
use tracing::info;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors produced by gateway operations.
#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    /// Transfer not found in the gateway.
    #[error("Transfer not found: {transfer_id}")]
    TransferNotFound { transfer_id: String },

    /// Invalid state machine transition.
    #[error("Invalid status transition from {from} to {to}")]
    InvalidStatusTransition { from: String, to: String },

    /// Proof of State validation failed for a scope.
    #[error("Proof validation failed on {scope} scope: {reason}")]
    ProofValidationFailed { scope: String, reason: String },

    /// Source and target scope are the same.
    #[error("Source and target scopes are identical: {scope}")]
    SameScopeTransfer { scope: String },

    /// Asset is already involved in an active transfer.
    #[error("Asset {asset_id} already has an active transfer: {transfer_id}")]
    AssetAlreadyInTransfer {
        asset_id: String,
        transfer_id: String,
    },
}

// ---------------------------------------------------------------------------
// GatewayManager
// ---------------------------------------------------------------------------

/// Top-level coordinator for cross-scope asset transfers.
///
/// `GatewayManager` exposes a simple interface for initiating, validating, and
/// listing transfers. Internally it delegates the protocol work to `ScopeBridge`.
pub struct GatewayManager {
    /// The scope bridge that handles the actual transfer protocol.
    bridge: ScopeBridge,
    /// Counter for generating unique transfer IDs.
    next_id: Arc<tokio::sync::Mutex<u64>>,
}

impl Default for GatewayManager {
    fn default() -> Self {
        Self::new()
    }
}

impl GatewayManager {
    /// Create a new gateway with the default transfer validator.
    pub fn new() -> Self {
        Self::with_validator(Arc::new(DefaultTransferValidator))
    }

    /// Create a new gateway with a custom transfer validator.
    pub fn with_validator(validator: Arc<dyn TransferValidator>) -> Self {
        Self {
            bridge: ScopeBridge::new(validator),
            next_id: Arc::new(tokio::sync::Mutex::new(1)),
        }
    }

    /// Initiate a cross-scope asset transfer.
    ///
    /// Creates the transfer record, registers it with the bridge, and returns
    /// the transfer ID. The transfer starts in `Pending` status.
    ///
    /// # Errors
    ///
    /// Returns `GatewayError::SameScopeTransfer` if source and target are identical,
    /// or `GatewayError::AssetAlreadyInTransfer` if the asset is already being
    /// transferred.
    pub async fn transfer_asset(
        &self,
        asset_id: AssetId,
        from_scope: BlockchainScope,
        to_scope: BlockchainScope,
    ) -> Result<String, GatewayError> {
        // Reject same-scope transfers
        if from_scope == to_scope {
            return Err(GatewayError::SameScopeTransfer {
                scope: from_scope.to_string(),
            });
        }

        // Check for active transfers on this asset
        if let Some(status) = self.bridge.sync_asset_state(&asset_id).await? {
            // Find the active transfer to report its ID
            let transfers = self.bridge.list_transfers().await;
            if let Some(active) = transfers.iter().find(|t| {
                t.asset_id == asset_id
                    && t.status != TransferStatus::Confirmed
                    && t.status != TransferStatus::RolledBack
            }) {
                return Err(GatewayError::AssetAlreadyInTransfer {
                    asset_id: asset_id.to_string(),
                    transfer_id: active.transfer_id.clone(),
                });
            }
            // Shouldn't reach here, but guard against it
            let _ = status;
        }

        // Generate transfer ID
        let transfer_id = {
            let mut counter = self.next_id.lock().await;
            let id = format!("gw-tx-{}", *counter);
            *counter += 1;
            id
        };

        let transfer =
            AssetTransfer::new(transfer_id.clone(), asset_id.clone(), from_scope, to_scope);

        info!(
            "Initiating transfer {} for asset {} ({} -> {})",
            transfer_id, asset_id, from_scope, to_scope
        );

        self.bridge.register_transfer(transfer).await;
        Ok(transfer_id)
    }

    /// Validate and execute a transfer to completion.
    ///
    /// Drives the transfer through the full lifecycle:
    /// Lock -> Validate -> Transit -> Confirm.
    /// Returns the final `TransferStatus`.
    pub async fn validate_transfer(
        &self,
        transfer_id: &str,
    ) -> Result<TransferStatus, GatewayError> {
        self.bridge.bridge_transfer(transfer_id).await
    }

    /// List all pending (non-terminal) transfers.
    pub async fn list_pending_transfers(&self) -> Vec<AssetTransfer> {
        self.bridge
            .list_transfers()
            .await
            .into_iter()
            .filter(|t| {
                t.status != TransferStatus::Confirmed && t.status != TransferStatus::RolledBack
            })
            .collect()
    }

    /// List all transfers regardless of status.
    pub async fn list_all_transfers(&self) -> Vec<AssetTransfer> {
        self.bridge.list_transfers().await
    }

    /// Get a reference to the underlying scope bridge.
    pub fn bridge(&self) -> &ScopeBridge {
        &self.bridge
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_device_to_network_transfer() {
        let gw = GatewayManager::new();

        let tid = gw
            .transfer_asset(
                AssetId::from("cpu-001"),
                BlockchainScope::Device,
                BlockchainScope::Network,
            )
            .await
            .expect("test: initiate transfer");

        let status = gw.validate_transfer(&tid).await.expect("test: validate");
        assert_eq!(status, TransferStatus::Confirmed);
    }

    #[tokio::test]
    async fn test_network_to_device_transfer() {
        let gw = GatewayManager::new();

        let tid = gw
            .transfer_asset(
                AssetId::from("storage-002"),
                BlockchainScope::Network,
                BlockchainScope::Device,
            )
            .await
            .expect("test: initiate transfer");

        let status = gw.validate_transfer(&tid).await.expect("test: validate");
        assert_eq!(status, TransferStatus::Confirmed);
    }

    #[tokio::test]
    async fn test_same_scope_rejected() {
        let gw = GatewayManager::new();

        let err = gw
            .transfer_asset(
                AssetId::from("asset-x"),
                BlockchainScope::Device,
                BlockchainScope::Device,
            )
            .await
            .unwrap_err();

        assert!(matches!(err, GatewayError::SameScopeTransfer { .. }));
    }

    #[tokio::test]
    async fn test_duplicate_transfer_rejected() {
        let gw = GatewayManager::new();

        let _tid = gw
            .transfer_asset(
                AssetId::from("dup-asset"),
                BlockchainScope::Device,
                BlockchainScope::Network,
            )
            .await
            .expect("test: first transfer");

        let err = gw
            .transfer_asset(
                AssetId::from("dup-asset"),
                BlockchainScope::Device,
                BlockchainScope::Network,
            )
            .await
            .unwrap_err();

        assert!(matches!(err, GatewayError::AssetAlreadyInTransfer { .. }));
    }

    #[tokio::test]
    async fn test_list_pending_transfers() {
        let gw = GatewayManager::new();

        // Create two transfers
        let _t1 = gw
            .transfer_asset(
                AssetId::from("a1"),
                BlockchainScope::Device,
                BlockchainScope::Network,
            )
            .await
            .expect("test: t1");

        let t2 = gw
            .transfer_asset(
                AssetId::from("a2"),
                BlockchainScope::Network,
                BlockchainScope::Device,
            )
            .await
            .expect("test: t2");

        // Complete t2
        gw.validate_transfer(&t2).await.expect("test: complete t2");

        // Only t1 should be pending
        let pending = gw.list_pending_transfers().await;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].asset_id, AssetId::from("a1"));
    }

    #[tokio::test]
    async fn test_list_all_transfers() {
        let gw = GatewayManager::new();

        let tid = gw
            .transfer_asset(
                AssetId::from("all-test"),
                BlockchainScope::Device,
                BlockchainScope::Network,
            )
            .await
            .expect("test: create");

        gw.validate_transfer(&tid).await.expect("test: validate");

        let all = gw.list_all_transfers().await;
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].status, TransferStatus::Confirmed);
    }

    #[tokio::test]
    async fn test_transfer_id_counter_increments() {
        let gw = GatewayManager::new();

        let t1 = gw
            .transfer_asset(
                AssetId::from("inc-a"),
                BlockchainScope::Device,
                BlockchainScope::Network,
            )
            .await
            .expect("test: t1");

        // Complete first so the asset is no longer active
        gw.validate_transfer(&t1).await.expect("test: complete t1");

        let t2 = gw
            .transfer_asset(
                AssetId::from("inc-b"),
                BlockchainScope::Device,
                BlockchainScope::Network,
            )
            .await
            .expect("test: t2");

        assert_ne!(t1, t2);
        assert!(t1.starts_with("gw-tx-"));
        assert!(t2.starts_with("gw-tx-"));
    }

    /// Verify a rejected transfer validator causes rollback.
    #[tokio::test]
    async fn test_failed_validation_causes_rollback() {
        // Use a validator that always rejects
        struct RejectValidator;

        #[async_trait::async_trait]
        impl TransferValidator for RejectValidator {
            async fn validate_transfer(
                &self,
                _transfer: &AssetTransfer,
            ) -> Result<bool, GatewayError> {
                Ok(false)
            }
        }

        let gw = GatewayManager::with_validator(Arc::new(RejectValidator));

        let tid = gw
            .transfer_asset(
                AssetId::from("reject-me"),
                BlockchainScope::Device,
                BlockchainScope::Network,
            )
            .await
            .expect("test: initiate");

        let status = gw.validate_transfer(&tid).await.expect("test: validate");
        assert_eq!(status, TransferStatus::RolledBack);
    }

    #[tokio::test]
    async fn test_concurrent_transfers_different_assets() {
        let gw = Arc::new(GatewayManager::new());

        let gw1 = gw.clone();
        let gw2 = gw.clone();

        let (r1, r2) = tokio::join!(
            async move {
                let tid = gw1
                    .transfer_asset(
                        AssetId::from("conc-a"),
                        BlockchainScope::Device,
                        BlockchainScope::Network,
                    )
                    .await
                    .expect("test: conc-a");
                gw1.validate_transfer(&tid)
                    .await
                    .expect("test: conc-a validate")
            },
            async move {
                let tid = gw2
                    .transfer_asset(
                        AssetId::from("conc-b"),
                        BlockchainScope::Network,
                        BlockchainScope::Device,
                    )
                    .await
                    .expect("test: conc-b");
                gw2.validate_transfer(&tid)
                    .await
                    .expect("test: conc-b validate")
            },
        );

        assert_eq!(r1, TransferStatus::Confirmed);
        assert_eq!(r2, TransferStatus::Confirmed);
    }
}
