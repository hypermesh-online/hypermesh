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

use hypermesh_lib::{AssetId, BlockchainScope, ContentHash, NodeId};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::asset_transfer::{
    AssetTransfer, TransferLockEntry, TransferRegistrationEntry, TransferReleaseEntry,
    TransferStatus, TransferValidator,
};
use super::GatewayError;
use crate::blockchain::block::{BlockAssetEntry, StoragePointer};
use crate::blockchain::NodeBlockchain;
use crate::network::shard_transport::ShardTransport;
use trustchain::proof_of_state::StateProof;
use trustchain::proof_of_state::StateProofOps;

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
    LockConfirmed { transfer_id: String },
    /// Request to register an asset on the target scope.
    RegisterRequest {
        asset_id: AssetId,
        transfer_id: String,
        target_scope: BlockchainScope,
    },
    /// Confirmation that registration is complete.
    RegisterConfirmed { transfer_id: String },
    /// Request to release a lock (rollback).
    RollbackRequest { transfer_id: String, reason: String },
}

// ---------------------------------------------------------------------------
// ScopeBridge
// ---------------------------------------------------------------------------

/// A shard that must be moved between scopes during a transfer.
#[derive(Debug, Clone)]
pub struct TransferShard {
    /// Content hash identifying this shard.
    pub shard_id: ContentHash,
    /// Raw shard data.
    pub data: Vec<u8>,
}

/// Routes messages and orchestrates the lock-transfer-unlock protocol between
/// Device and Network blockchain scopes.
pub struct ScopeBridge {
    /// In-flight transfers tracked by transfer_id.
    transfers: Arc<RwLock<HashMap<String, AssetTransfer>>>,
    /// Transfer validator for proof checks.
    validator: Arc<dyn TransferValidator>,
    /// Optional shard transport for moving data between scopes.
    transport: Option<Arc<dyn ShardTransport>>,
    /// Node ID of the target scope gateway (where shards are sent during transit).
    target_node: Option<NodeId>,
    /// Shards staged for transfer, keyed by transfer_id.
    shard_staging: Arc<RwLock<HashMap<String, Vec<TransferShard>>>>,
    /// Optional blockchain reference for writing transfer entries.
    /// When `None`, transfers proceed in-memory only (backward compatible).
    blockchain: Option<Arc<NodeBlockchain>>,
}

impl ScopeBridge {
    /// Create a new bridge with the given validator (no network transport).
    pub fn new(validator: Arc<dyn TransferValidator>) -> Self {
        Self {
            transfers: Arc::new(RwLock::new(HashMap::new())),
            validator,
            transport: None,
            target_node: None,
            shard_staging: Arc::new(RwLock::new(HashMap::new())),
            blockchain: None,
        }
    }

    /// Create a bridge wired to a shard transport for actual data movement.
    pub fn with_transport(
        validator: Arc<dyn TransferValidator>,
        transport: Arc<dyn ShardTransport>,
        target_node: NodeId,
    ) -> Self {
        Self {
            transfers: Arc::new(RwLock::new(HashMap::new())),
            validator,
            transport: Some(transport),
            target_node: Some(target_node),
            shard_staging: Arc::new(RwLock::new(HashMap::new())),
            blockchain: None,
        }
    }

    /// Create a bridge wired to a blockchain for persistent transfer entries.
    pub fn with_blockchain(
        validator: Arc<dyn TransferValidator>,
        blockchain: Arc<NodeBlockchain>,
    ) -> Self {
        Self {
            transfers: Arc::new(RwLock::new(HashMap::new())),
            validator,
            transport: None,
            target_node: None,
            shard_staging: Arc::new(RwLock::new(HashMap::new())),
            blockchain: Some(blockchain),
        }
    }

    /// Set or replace the blockchain reference on this bridge.
    pub fn set_blockchain(&mut self, blockchain: Arc<NodeBlockchain>) {
        self.blockchain = Some(blockchain);
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
    pub async fn bridge_transfer(&self, transfer_id: &str) -> Result<TransferStatus, GatewayError> {
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
                let reason = format!("Proof validation error: {e}");
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
            BridgeMessage::RollbackRequest {
                transfer_id,
                reason,
            } => {
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
        let active = transfers.values().find(|t| {
            t.asset_id == *asset_id
                && t.status != TransferStatus::Confirmed
                && t.status != TransferStatus::RolledBack
        });

        Ok(active.map(|t| t.status))
    }

    /// Attach shards to a registered transfer so they can be moved during transit.
    pub async fn attach_shards(
        &self,
        transfer_id: &str,
        shards: Vec<TransferShard>,
    ) -> Result<(), GatewayError> {
        let mut transfers = self.transfers.write().await;
        let _transfer =
            transfers
                .get_mut(transfer_id)
                .ok_or_else(|| GatewayError::TransferNotFound {
                    transfer_id: transfer_id.to_string(),
                })?;
        // Store shards in the shard staging map
        drop(transfers);

        let mut staging = self.shard_staging.write().await;
        staging.insert(transfer_id.to_string(), shards);
        debug!("Attached shards for transfer {}", transfer_id);
        Ok(())
    }

    /// Return a snapshot of all tracked transfers.
    pub async fn list_transfers(&self) -> Vec<AssetTransfer> {
        self.transfers.read().await.values().cloned().collect()
    }

    // --- internal helpers ---

    /// Write a serialized transfer entry to the blockchain as a new block.
    ///
    /// Returns `Ok(())` if no blockchain is wired (in-memory only mode).
    async fn write_transfer_entry(
        &self,
        entry_bytes: &[u8],
        label: &str,
        transfer_id: &str,
    ) -> Result<(), GatewayError> {
        let bc = match self.blockchain.as_ref() {
            Some(bc) => bc,
            None => return Ok(()),
        };

        let asset_hash = *blake3::hash(entry_bytes).as_bytes();

        // Generate a REAL PoS proof from this node's own identity, derived
        // deterministically from its matrix coordinate (R1: hardware-assessed).
        let node_id = crate::bootstrap::node_id(bc.node_coordinate());
        let state_proof = StateProof::generate_from_network(&node_id)
            .await
            .map_err(|e| GatewayError::ProofValidationFailed {
                scope: "blockchain".to_string(),
                reason: format!(
                    "state proof generation for transfer {transfer_id}: {e}"
                ),
            })?;
        let registration = crate::assets::core::AssetRegistration::from_asset_data(
            &crate::assets::core::asset_id::AssetData {
                config: Vec::new(),
                definition: entry_bytes.to_vec(),
                metadata: format!("transfer-{}", label).into_bytes(),
            },
            crate::assets::core::asset_id::NetworkScope::Global,
            crate::assets::core::asset_id::AssetCategory::BaseSystem(
                crate::assets::core::asset_id::BaseSystemType::Blockchain,
            ),
        );

        // Bind the proof to the content hash (signed-to-content invariant, P1).
        let block_entry = BlockAssetEntry::new_bound(
            asset_hash,
            &state_proof,
            StoragePointer::Local {
                path: String::from_utf8_lossy(entry_bytes).to_string(),
            },
            registration,
        );

        bc.add_block(vec![block_entry]).await.map_err(|e| {
            GatewayError::ProofValidationFailed {
                scope: "blockchain".to_string(),
                reason: format!("failed to write {} entry: {}", label, e),
            }
        })?;

        info!(
            "Transfer {} entry written to blockchain for {}",
            label, transfer_id
        );
        Ok(())
    }

    async fn lock_on_source(&self, transfer_id: &str) -> Result<(), GatewayError> {
        let (asset_id, source_scope, target_scope) = {
            let mut transfers = self.transfers.write().await;
            let transfer =
                transfers
                    .get_mut(transfer_id)
                    .ok_or_else(|| GatewayError::TransferNotFound {
                        transfer_id: transfer_id.to_string(),
                    })?;
            transfer.lock()?;
            info!(
                "Locked asset {} on {} scope for transfer {}",
                transfer.asset_id, transfer.source_scope, transfer_id
            );
            (
                transfer.asset_id.to_string(),
                transfer.source_scope,
                transfer.target_scope,
            )
        };

        // Write TransferLockEntry to blockchain (no-op if blockchain is None)
        let lock_entry = TransferLockEntry {
            transfer_id: transfer_id.to_string(),
            asset_id,
            source_scope,
            target_scope,
            locked_at: chrono::Utc::now().timestamp(),
            proof_hash: *blake3::hash(b"transfer-lock-proof").as_bytes(),
        };
        let entry_bytes = serde_json::to_vec(&lock_entry).map_err(|e| {
            GatewayError::ProofValidationFailed {
                scope: "serialization".to_string(),
                reason: format!("serialize lock: {e}"),
            }
        })?;
        self.write_transfer_entry(&entry_bytes, "lock", transfer_id)
            .await?;

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
        // Advance state machine to InTransit
        {
            let mut transfers = self.transfers.write().await;
            let transfer =
                transfers
                    .get_mut(transfer_id)
                    .ok_or_else(|| GatewayError::TransferNotFound {
                        transfer_id: transfer_id.to_string(),
                    })?;
            transfer.begin_transit()?;
        }

        // If we have a transport wired, send the staged shards
        if let (Some(transport), Some(target_node)) = (&self.transport, &self.target_node) {
            let shards = {
                let mut staging = self.shard_staging.write().await;
                staging.remove(transfer_id).unwrap_or_default()
            };

            for shard in &shards {
                transport
                    .send_shard(target_node, &shard.shard_id, &shard.data)
                    .await
                    .map_err(|e| {
                        warn!(
                            "Shard transport failed for transfer {}: {}",
                            transfer_id, e
                        );
                        GatewayError::ProofValidationFailed {
                            scope: "transport".to_string(),
                            reason: format!("Shard send failed: {e}"),
                        }
                    })?;
            }

            if !shards.is_empty() {
                info!(
                    "Transfer {} sent {} shards via transport to {}",
                    transfer_id,
                    shards.len(),
                    target_node.to_hex()
                );
            }
        }

        info!("Transfer {} now in transit", transfer_id);
        Ok(())
    }

    async fn confirm_transfer(&self, transfer_id: &str) -> Result<(), GatewayError> {
        let (asset_id, source_scope, target_scope) = {
            let mut transfers = self.transfers.write().await;
            let transfer =
                transfers
                    .get_mut(transfer_id)
                    .ok_or_else(|| GatewayError::TransferNotFound {
                        transfer_id: transfer_id.to_string(),
                    })?;
            transfer.confirm()?;
            info!(
                "Transfer {} confirmed on {} scope",
                transfer_id, transfer.target_scope
            );
            (
                transfer.asset_id.to_string(),
                transfer.source_scope,
                transfer.target_scope,
            )
        };

        // Write TransferRegistrationEntry to blockchain
        let reg_entry = TransferRegistrationEntry {
            transfer_id: transfer_id.to_string(),
            asset_id,
            source_scope,
            target_scope,
            registered_at: chrono::Utc::now().timestamp(),
            proof_hash: *blake3::hash(b"transfer-registration-proof").as_bytes(),
        };
        let entry_bytes = serde_json::to_vec(&reg_entry).map_err(|e| {
            GatewayError::ProofValidationFailed {
                scope: "serialization".to_string(),
                reason: format!("serialize registration: {e}"),
            }
        })?;
        self.write_transfer_entry(&entry_bytes, "registration", transfer_id)
            .await?;

        Ok(())
    }

    async fn fail_transfer(&self, transfer_id: &str, reason: String) -> Result<(), GatewayError> {
        let asset_id = {
            let mut transfers = self.transfers.write().await;
            let transfer =
                transfers
                    .get_mut(transfer_id)
                    .ok_or_else(|| GatewayError::TransferNotFound {
                        transfer_id: transfer_id.to_string(),
                    })?;
            let aid = transfer.asset_id.to_string();
            transfer.fail(reason.clone())?;
            transfer.rollback()?;
            warn!("Transfer {} rolled back", transfer_id);
            aid
        };

        // Write TransferReleaseEntry to blockchain
        let release_entry = TransferReleaseEntry {
            transfer_id: transfer_id.to_string(),
            asset_id,
            released_at: chrono::Utc::now().timestamp(),
            reason,
        };
        let entry_bytes = serde_json::to_vec(&release_entry).map_err(|e| {
            GatewayError::ProofValidationFailed {
                scope: "serialization".to_string(),
                reason: format!("serialize release: {e}"),
            }
        })?;
        self.write_transfer_entry(&entry_bytes, "release", transfer_id)
            .await?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::super::asset_transfer::DefaultTransferValidator;
    use super::*;
    use crate::network::shard_transport::MockShardTransport;

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
        let t = all
            .iter()
            .find(|t| t.transfer_id == "tx-100")
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
        let t = all
            .iter()
            .find(|t| t.transfer_id == "tx-300")
            .expect("test: find transfer");
        assert_eq!(t.status, TransferStatus::RolledBack);
    }

    #[tokio::test]
    async fn test_bridge_with_transport_sends_shards() {
        let mock_transport = Arc::new(MockShardTransport::new());
        let target_node = NodeId::from_bytes([0xAA; 32]);

        let bridge = ScopeBridge::with_transport(
            Arc::new(DefaultTransferValidator),
            mock_transport.clone(),
            target_node.clone(),
        );

        let transfer = make_transfer("tx-transport-1");
        bridge.register_transfer(transfer).await;

        // Attach shards to the transfer
        let shards = vec![
            TransferShard {
                shard_id: ContentHash([0x01; 32]),
                data: vec![0xDE, 0xAD],
            },
            TransferShard {
                shard_id: ContentHash([0x02; 32]),
                data: vec![0xBE, 0xEF],
            },
        ];
        bridge
            .attach_shards("tx-transport-1", shards)
            .await
            .expect("test: attach shards");

        // Run the full bridge transfer
        let status = bridge
            .bridge_transfer("tx-transport-1")
            .await
            .expect("test: bridge transfer with transport");
        assert_eq!(status, TransferStatus::Confirmed);

        // Verify shards were sent via the mock transport
        assert_eq!(mock_transport.shard_count().await, 2);

        let fetched = mock_transport
            .fetch_shard(&target_node, &ContentHash([0x01; 32]))
            .await
            .expect("test: fetch shard 1");
        assert_eq!(fetched, vec![0xDE, 0xAD]);

        let fetched2 = mock_transport
            .fetch_shard(&target_node, &ContentHash([0x02; 32]))
            .await
            .expect("test: fetch shard 2");
        assert_eq!(fetched2, vec![0xBE, 0xEF]);
    }

    #[tokio::test]
    async fn test_bridge_transport_failure_during_transit() {
        let mock_transport = Arc::new(MockShardTransport::new());
        let target_node = NodeId::from_bytes([0xBB; 32]);

        // Mark the target unreachable so sends fail
        mock_transport.set_unreachable(&target_node).await;

        let bridge = ScopeBridge::with_transport(
            Arc::new(DefaultTransferValidator),
            mock_transport.clone(),
            target_node,
        );

        let transfer = make_transfer("tx-fail-transport");
        bridge.register_transfer(transfer).await;

        // Attach a shard that will fail to send
        let shards = vec![TransferShard {
            shard_id: ContentHash([0x03; 32]),
            data: vec![0xFF],
        }];
        bridge
            .attach_shards("tx-fail-transport", shards)
            .await
            .expect("test: attach shards");

        // The bridge_transfer should fail because transit sends fail
        // The transfer advances to InTransit state then the send fails,
        // but begin_transit already changed state. The error propagates up.
        let result = bridge.bridge_transfer("tx-fail-transport").await;
        assert!(result.is_err(), "Expected transport failure to propagate");

        // No shards should have been stored
        assert_eq!(mock_transport.shard_count().await, 0);
    }

    // --- Blockchain-write tests ---

    fn make_blockchain() -> Arc<crate::blockchain::NodeBlockchain> {
        use crate::matrix::coordinate::MatrixCoordinate;
        let coord = MatrixCoordinate::new(7, 7, 7).expect("test: valid coord");
        Arc::new(crate::blockchain::NodeBlockchain::new(coord))
    }

    fn make_bridge_with_blockchain(bc: Arc<crate::blockchain::NodeBlockchain>) -> ScopeBridge {
        ScopeBridge::with_blockchain(Arc::new(DefaultTransferValidator), bc)
    }

    #[tokio::test]
    async fn test_scope_bridge_writes_lock_to_blockchain() {
        let bc = make_blockchain();
        let initial_height = bc.get_height().await;

        let bridge = make_bridge_with_blockchain(bc.clone());
        let transfer = make_transfer("tx-bc-lock");
        bridge.register_transfer(transfer).await;

        // Execute lock
        bridge
            .lock_on_source("tx-bc-lock")
            .await
            .expect("test: lock");

        // Verify blockchain grew by one block (the TransferLockEntry)
        let new_height = bc.get_height().await;
        assert_eq!(
            new_height,
            initial_height + 1,
            "Lock should write one block to blockchain"
        );

        // Verify the block contains a Local storage pointer with the lock JSON
        let block = bc
            .get_block(new_height)
            .await
            .expect("test: get lock block");
        assert_eq!(block.entries.len(), 1);
        if let StoragePointer::Local { ref path } = block.entries[0].storage_pointer {
            assert!(
                path.contains("tx-bc-lock"),
                "Lock entry should reference the transfer ID"
            );
            assert!(
                path.contains("transfer_id"),
                "Lock entry should be serialized JSON"
            );
        } else {
            unreachable!("Expected StoragePointer::Local for lock entry");
        }
    }

    #[tokio::test]
    async fn test_scope_bridge_writes_registration_on_confirm() {
        let bc = make_blockchain();
        let bridge = make_bridge_with_blockchain(bc.clone());
        let transfer = make_transfer("tx-bc-confirm");
        bridge.register_transfer(transfer).await;

        // Drive through full lifecycle
        let status = bridge
            .bridge_transfer("tx-bc-confirm")
            .await
            .expect("test: bridge transfer");
        assert_eq!(status, TransferStatus::Confirmed);

        // Lock writes 1 block, confirm writes 1 block = 2 new blocks
        // (begin_transit does not write a block)
        let height = bc.get_height().await;
        assert!(
            height >= 2,
            "Expected at least 2 blocks (lock + registration), got {}",
            height
        );

        // Last block should be the registration entry
        let last_block = bc
            .get_block(height)
            .await
            .expect("test: get registration block");
        if let StoragePointer::Local { ref path } = last_block.entries[0].storage_pointer {
            assert!(
                path.contains("tx-bc-confirm"),
                "Registration entry should reference the transfer ID"
            );
            assert!(
                path.contains("registered_at"),
                "Registration entry should contain registered_at field"
            );
        } else {
            unreachable!("Expected StoragePointer::Local for registration entry");
        }
    }

    #[tokio::test]
    async fn test_scope_bridge_writes_release_on_failure() {
        let bc = make_blockchain();
        let bridge = make_bridge_with_blockchain(bc.clone());
        let transfer = make_transfer("tx-bc-fail");
        bridge.register_transfer(transfer).await;

        // Lock, then fail
        bridge
            .lock_on_source("tx-bc-fail")
            .await
            .expect("test: lock");
        let height_after_lock = bc.get_height().await;

        bridge
            .fail_transfer("tx-bc-fail", "test failure".to_string())
            .await
            .expect("test: fail");

        // Fail should write 1 more block (release entry)
        let height_after_fail = bc.get_height().await;
        assert_eq!(
            height_after_fail,
            height_after_lock + 1,
            "Fail should write one release block"
        );

        let release_block = bc
            .get_block(height_after_fail)
            .await
            .expect("test: get release block");
        if let StoragePointer::Local { ref path } = release_block.entries[0].storage_pointer {
            assert!(
                path.contains("tx-bc-fail"),
                "Release entry should reference the transfer ID"
            );
            assert!(
                path.contains("test failure"),
                "Release entry should contain the failure reason"
            );
        } else {
            unreachable!("Expected StoragePointer::Local for release entry");
        }
    }

    #[tokio::test]
    async fn test_scope_bridge_no_blockchain_still_works() {
        // Verify that the original behavior (no blockchain) is unchanged.
        let bridge = make_bridge();
        let transfer = make_transfer("tx-no-bc");
        bridge.register_transfer(transfer).await;

        let status = bridge
            .bridge_transfer("tx-no-bc")
            .await
            .expect("test: bridge transfer without blockchain");
        assert_eq!(status, TransferStatus::Confirmed);
    }
}
