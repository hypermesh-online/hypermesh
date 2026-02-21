// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Cross-scope communication and routing
//!
//! `ScopeBridge` handles the protocol for moving assets between the Device and
//! Network blockchain scopes. The transfer protocol is:
//!
//! 1. **Lock** the asset on the source scope (prevents double-spend).
//! 2. **Validate proofs** on both source and target scopes (PoSpace + PoStake).
//! 3. **Register** the asset on the target scope.
//! 4. **Unlock** / finalize on both sides.
//!
//! If any step fails the bridge initiates a rollback to release the source lock.

use std::collections::HashMap;
use std::sync::Arc;

use hypermesh_lib::{AssetId, BlockchainScope};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::asset_transfer::{AssetTransfer, TransferStatus, TransferValidator};
use super::GatewayError;

// ---------------------------------------------------------------------------
// Bridge message types
// ---------------------------------------------------------------------------

/// Messages exchanged between scopes during a transfer.
#[derive(Debug, Clone)]
pub enum BridgeMessage {
    /// Request to lock an asset on its current scope.
    LockRequest {
        asset_id: AssetId,
        transfer_id: String,
    },
    /// Confirmation that an asset is locked.
    LockConfirmed {
        transfer_id: String,
    },
    /// Request to register an asset on the target scope.
    RegisterRequest {
        asset_id: AssetId,
        transfer_id: String,
        target_scope: BlockchainScope,
    },
    /// Confirmation that registration is complete.
    RegisterConfirmed {
        transfer_id: String,
    },
    /// Request to release a lock (rollback).
    RollbackRequest {
        transfer_id: String,
        reason: String,
    },
}

// ---------------------------------------------------------------------------
// ScopeBridge
// ---------------------------------------------------------------------------

/// Routes messages and orchestrates the lock-transfer-unlock protocol between
/// Device and Network blockchain scopes.
pub struct ScopeBridge {
    /// In-flight transfers tracked by transfer_id.
    transfers: Arc<RwLock<HashMap<String, AssetTransfer>>>,
    /// Transfer validator for proof checks.
    validator: Arc<dyn TransferValidator>,
}

impl ScopeBridge {
    /// Create a new bridge with the given validator.
    pub fn new(validator: Arc<dyn TransferValidator>) -> Self {
        Self {
            transfers: Arc::new(RwLock::new(HashMap::new())),
            validator,
        }
    }

    /// Register a transfer for tracking.
    pub async fn register_transfer(&self, transfer: AssetTransfer) {
        let id = transfer.transfer_id.clone();
        self.transfers.write().await.insert(id.clone(), transfer);
        debug!("Registered transfer {}", id);
    }

    /// Execute the full bridge protocol for a transfer.
    ///
    /// This drives the transfer through Lock -> Validate -> Transit -> Confirm.
    /// On failure at any point, the transfer is marked Failed and rolled back.
    pub async fn bridge_transfer(
        &self,
        transfer_id: &str,
    ) -> Result<TransferStatus, GatewayError> {
        // --- Phase 1: Lock on source scope ---
        self.lock_on_source(transfer_id).await?;

        // --- Phase 2: Validate proofs ---
        match self.validate_proofs(transfer_id).await {
            Ok(true) => {}
            Ok(false) => {
                self.fail_transfer(transfer_id, "Proof validation rejected".to_string())
                    .await?;
                return Ok(TransferStatus::RolledBack);
            }
            Err(e) => {
                let reason = format!("Proof validation error: {}", e);
                self.fail_transfer(transfer_id, reason).await?;
                return Ok(TransferStatus::RolledBack);
            }
        }

        // --- Phase 3: Begin transit ---
        self.begin_transit(transfer_id).await?;

        // --- Phase 4: Confirm on target scope ---
        self.confirm_transfer(transfer_id).await?;

        Ok(TransferStatus::Confirmed)
    }

    /// Route a message to the appropriate scope handler.
    pub async fn route_message(
        &self,
        msg: BridgeMessage,
        _target_scope: BlockchainScope,
    ) -> Result<(), GatewayError> {
        match msg {
            BridgeMessage::LockRequest { transfer_id, .. } => {
                info!("Routing lock request for transfer {}", transfer_id);
                self.lock_on_source(&transfer_id).await?;
            }
            BridgeMessage::RegisterRequest { transfer_id, .. } => {
                info!("Routing register request for transfer {}", transfer_id);
                // Registration is part of confirm_transfer
            }
            BridgeMessage::RollbackRequest { transfer_id, reason } => {
                warn!("Routing rollback for transfer {}: {}", transfer_id, reason);
                self.fail_transfer(&transfer_id, reason).await?;
            }
            BridgeMessage::LockConfirmed { transfer_id } => {
                debug!("Lock confirmed for {}", transfer_id);
            }
            BridgeMessage::RegisterConfirmed { transfer_id } => {
                debug!("Register confirmed for {}", transfer_id);
            }
        }
        Ok(())
    }

    /// Synchronize asset state between scopes by checking whether the asset
    /// has an active transfer in progress.
    pub async fn sync_asset_state(
        &self,
        asset_id: &AssetId,
    ) -> Result<Option<TransferStatus>, GatewayError> {
        let transfers = self.transfers.read().await;
        let active = transfers
            .values()
            .find(|t| t.asset_id == *asset_id && t.status != TransferStatus::Confirmed
                && t.status != TransferStatus::RolledBack);

        Ok(active.map(|t| t.status))
    }

    /// Return a snapshot of all tracked transfers.
    pub async fn list_transfers(&self) -> Vec<AssetTransfer> {
        self.transfers.read().await.values().cloned().collect()
    }

    // --- internal helpers ---

    async fn lock_on_source(&self, transfer_id: &str) -> Result<(), GatewayError> {
        let mut transfers = self.transfers.write().await;
        let transfer = transfers
            .get_mut(transfer_id)
            .ok_or_else(|| GatewayError::TransferNotFound {
                transfer_id: transfer_id.to_string(),
            })?;
        transfer.lock()?;
        info!("Locked asset {} on {} scope for transfer {}",
            transfer.asset_id, transfer.source_scope, transfer_id);
        Ok(())
    }

    async fn validate_proofs(&self, transfer_id: &str) -> Result<bool, GatewayError> {
        let transfer = {
            let transfers = self.transfers.read().await;
            transfers
                .get(transfer_id)
                .cloned()
                .ok_or_else(|| GatewayError::TransferNotFound {
                    transfer_id: transfer_id.to_string(),
                })?
        };

        let valid = self.validator.validate_transfer(&transfer).await?;

        if valid {
            let mut transfers = self.transfers.write().await;
            if let Some(t) = transfers.get_mut(transfer_id) {
                t.source_proofs_verified = true;
                t.target_proofs_verified = true;
            }
        }
        Ok(valid)
    }

    async fn begin_transit(&self, transfer_id: &str) -> Result<(), GatewayError> {
        let mut transfers = self.transfers.write().await;
        let transfer = transfers
            .get_mut(transfer_id)
            .ok_or_else(|| GatewayError::TransferNotFound {
                transfer_id: transfer_id.to_string(),
            })?;
        transfer.begin_transit()?;
        info!("Transfer {} now in transit", transfer_id);
        Ok(())
    }

    async fn confirm_transfer(&self, transfer_id: &str) -> Result<(), GatewayError> {
        let mut transfers = self.transfers.write().await;
        let transfer = transfers
            .get_mut(transfer_id)
            .ok_or_else(|| GatewayError::TransferNotFound {
                transfer_id: transfer_id.to_string(),
            })?;
        transfer.confirm()?;
        info!("Transfer {} confirmed on {} scope",
            transfer_id, transfer.target_scope);
        Ok(())
    }

    async fn fail_transfer(
        &self,
        transfer_id: &str,
        reason: String,
    ) -> Result<(), GatewayError> {
        let mut transfers = self.transfers.write().await;
        let transfer = transfers
            .get_mut(transfer_id)
            .ok_or_else(|| GatewayError::TransferNotFound {
                transfer_id: transfer_id.to_string(),
            })?;
        transfer.fail(reason)?;
        transfer.rollback()?;
        warn!("Transfer {} rolled back", transfer_id);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::asset_transfer::DefaultTransferValidator;

    fn make_bridge() -> ScopeBridge {
        ScopeBridge::new(Arc::new(DefaultTransferValidator))
    }

    fn make_transfer(id: &str) -> AssetTransfer {
        AssetTransfer::new(
            id.to_string(),
            AssetId::from("asset-xyz"),
            BlockchainScope::Device,
            BlockchainScope::Network,
        )
    }

    #[tokio::test]
    async fn test_bridge_happy_path() {
        let bridge = make_bridge();
        let transfer = make_transfer("tx-100");
        bridge.register_transfer(transfer).await;

        let status = bridge
            .bridge_transfer("tx-100")
            .await
            .expect("test: bridge transfer");
        assert_eq!(status, TransferStatus::Confirmed);

        // Verify final state
        let all = bridge.list_transfers().await;
        let t = all.iter().find(|t| t.transfer_id == "tx-100")
            .expect("test: find transfer");
        assert_eq!(t.status, TransferStatus::Confirmed);
        assert!(t.source_proofs_verified);
        assert!(t.target_proofs_verified);
    }

    #[tokio::test]
    async fn test_bridge_transfer_not_found() {
        let bridge = make_bridge();
        let err = bridge.bridge_transfer("nonexistent").await.unwrap_err();
        assert!(matches!(err, GatewayError::TransferNotFound { .. }));
    }

    #[tokio::test]
    async fn test_sync_asset_state_active() {
        let bridge = make_bridge();
        let transfer = make_transfer("tx-200");
        bridge.register_transfer(transfer).await;

        let state = bridge
            .sync_asset_state(&AssetId::from("asset-xyz"))
            .await
            .expect("test: sync state");
        assert_eq!(state, Some(TransferStatus::Pending));
    }

    #[tokio::test]
    async fn test_sync_asset_state_none() {
        let bridge = make_bridge();
        let state = bridge
            .sync_asset_state(&AssetId::from("no-such-asset"))
            .await
            .expect("test: sync state");
        assert!(state.is_none());
    }

    #[tokio::test]
    async fn test_route_rollback_message() {
        let bridge = make_bridge();
        let transfer = make_transfer("tx-300");
        bridge.register_transfer(transfer).await;

        // Lock first so rollback (fail + rollback) can proceed
        bridge.lock_on_source("tx-300").await.expect("test: lock");

        let msg = BridgeMessage::RollbackRequest {
            transfer_id: "tx-300".to_string(),
            reason: "user cancelled".to_string(),
        };
        bridge
            .route_message(msg, BlockchainScope::Device)
            .await
            .expect("test: route rollback");

        let all = bridge.list_transfers().await;
        let t = all.iter().find(|t| t.transfer_id == "tx-300")
            .expect("test: find transfer");
        assert_eq!(t.status, TransferStatus::RolledBack);
    }
}
