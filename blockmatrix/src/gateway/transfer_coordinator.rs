// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase G.1: cross-node transfer choreography.
//!
//! [`TransferCoordinator`] orchestrates the wire-level state machine:
//!
//! ```text
//! Initiated → Locked → ShardsHandedOff → Registered → Released   (happy path, cross-node)
//! Initiated → Locked → ShardsHandedOff → Failed → RolledBack     (target rejection)
//! Initiated → Locked → TimedOut → RolledBack                     (no response from target)
//! ```
//!
//! On the source side it locks the asset, broadcasts `TAG_TRANSFER_LOCK`,
//! sends `TAG_TRANSFER_REGISTER_REQ` point-to-point to the target peer,
//! awaits the ack with a deadline, and on success writes a release entry
//! and broadcasts `TAG_TRANSFER_RELEASE`. Failure paths persist a
//! [`TransferReleaseEntry`] with the [`RollbackReason`] and broadcast
//! `TAG_TRANSFER_ROLLBACK`.
//!
//! Phase G.1 lands the wire protocol, the cross-node state machine, the
//! initial coordinator, and a [`TransferTransport`] trait for testability.
//! Restart-recovery (`resume_in_flight`) and full multi-host integration
//! testing are scoped to Phase G.2.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use hypermesh_lib::{AssetId, BlockchainScope};
use serde::{Deserialize, Serialize};
use tokio::sync::{oneshot, Mutex as TokioMutex, RwLock};
use tracing::{debug, info, warn};
use trustchain::proof_of_state::StateProof;

use super::asset_transfer::{
    RollbackReason, TransferLockEntry, TransferReceipt, TransferRegistrationEntry,
    TransferReleaseEntry,
};
use super::transfer_protocol::{
    CoordinatorState, PeerCertFingerprint, ShardManifestEntry, TransferLockMessage,
    TransferRegisterAck, TransferRegisterRequest, TransferRelease, TransferRollback,
};
use super::GatewayError;
use crate::blockchain::block::{BlockAssetEntry, StoragePointer};
use crate::blockchain::NodeBlockchain;

/// Default deadline for awaiting `TAG_TRANSFER_REGISTER_ACK` from the
/// target peer. Configurable per coordinator via
/// [`TransferCoordinator::set_register_timeout`].
pub const DEFAULT_REGISTER_TIMEOUT: Duration = Duration::from_secs(30);

/// Abstract transport used by [`TransferCoordinator`] to ship wire
/// messages between source and target nodes. The real implementation
/// wraps STOQ; tests use an in-process mock that captures sent messages.
#[async_trait]
pub trait TransferTransport: Send + Sync {
    /// Broadcast a `TAG_TRANSFER_LOCK` payload to all connected peers.
    async fn broadcast_lock(&self, msg: TransferLockMessage) -> Result<(), GatewayError>;

    /// Send a `TAG_TRANSFER_REGISTER_REQ` to a specific peer and await
    /// the matching `TAG_TRANSFER_REGISTER_ACK`. Implementations are
    /// responsible for honouring `deadline`; the coordinator additionally
    /// guards with its own `tokio::time::timeout` so a misbehaving
    /// transport cannot stall the state machine.
    async fn send_register_request(
        &self,
        peer: &PeerCertFingerprint,
        req: TransferRegisterRequest,
        deadline: Duration,
    ) -> Result<TransferRegisterAck, GatewayError>;

    /// Broadcast a `TAG_TRANSFER_RELEASE` payload.
    async fn broadcast_release(&self, msg: TransferRelease) -> Result<(), GatewayError>;

    /// Broadcast a `TAG_TRANSFER_ROLLBACK` payload.
    async fn broadcast_rollback(&self, msg: TransferRollback) -> Result<(), GatewayError>;
}

/// Federation gate consulted by `initiate` before any wire traffic. Wraps
/// [`trustchain::ca::FederationManager::is_federation_signed`] so that
/// blockmatrix does not need to depend on FederationManager directly when
/// running in alpha-default inert mode.
#[async_trait]
pub trait FederationGate: Send + Sync {
    /// Returns `true` iff the peer's certificate is federation-signed and
    /// the peer is not currently in `Untrusted` state.
    async fn allow_peer(&self, peer: &PeerCertFingerprint) -> bool;
}

/// Always-allow gate — used by tests and by Phase G.1 when no federation
/// manager is wired (alpha-default inert).
pub struct AllowAllFederation;

#[async_trait]
impl FederationGate for AllowAllFederation {
    async fn allow_peer(&self, _peer: &PeerCertFingerprint) -> bool {
        true
    }
}

/// Always-deny gate — used by tests to assert that unfederated peers are
/// rejected before any wire traffic.
pub struct DenyAllFederation;

#[async_trait]
impl FederationGate for DenyAllFederation {
    async fn allow_peer(&self, _peer: &PeerCertFingerprint) -> bool {
        false
    }
}

/// Tracked state for a single in-flight transfer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoordinatedTransfer {
    pub transfer_id: String,
    pub asset_id: AssetId,
    pub source_chain_id: String,
    pub target_chain_id: String,
    pub source_scope: BlockchainScope,
    pub target_scope: BlockchainScope,
    pub target_peer: PeerCertFingerprint,
    pub state: CoordinatorState,
    pub source_lock_block_hash: Option<String>,
    pub target_register_block_hash: Option<String>,
    pub source_release_block_hash: Option<String>,
    pub manifest: Vec<ShardManifestEntry>,
    pub last_error: Option<String>,
    /// State proof committed at lock time — reused for release/receipt
    /// entries so every transfer-related block carries a valid proof.
    pub state_proof: StateProof,
}

/// Returned to the caller of [`TransferCoordinator::initiate`] on success.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferReceiptOutcome {
    pub transfer_id: String,
    pub source_block_hash: String,
    pub target_block_hash: String,
    pub completed_at: i64,
}

/// Cross-node transfer orchestrator. Wired into `DaemonState` as an
/// `Option<Arc<TransferCoordinator>>` so alpha-default inert behaviour is
/// preserved (the IPC handler returns
/// [`GatewayError::CoordinatorNotConfigured`] when None).
pub struct TransferCoordinator {
    /// Local source chain — used for writing lock/release entries.
    blockchain: Arc<NodeBlockchain>,
    /// Wire transport (real STOQ impl or mock).
    transport: Arc<dyn TransferTransport>,
    /// Federation gate consulted before any wire traffic.
    federation: Arc<dyn FederationGate>,
    /// In-flight transfers keyed by transfer_id.
    transfers: Arc<RwLock<HashMap<String, CoordinatedTransfer>>>,
    /// Local network/chain identifier.
    local_chain_id: String,
    /// Register-ack deadline.
    register_timeout: RwLock<Duration>,
    /// Pending register-ack waiters keyed by transfer_id.
    ///
    /// Production transports (STOQ-backed) register a oneshot sender
    /// here before broadcasting `TAG_TRANSFER_REGISTER_REQ`. When a
    /// matching `TAG_TRANSFER_REGISTER_ACK` arrives, the wire handler
    /// fires the sender to wake the awaiting `initiate()` future.
    /// Mock transports that respond synchronously do not use this map.
    pending_acks: Arc<TokioMutex<HashMap<String, oneshot::Sender<TransferRegisterAck>>>>,
}

impl TransferCoordinator {
    /// Construct a coordinator. `federation` defaults to
    /// [`AllowAllFederation`] when callers want alpha-default inert
    /// behaviour; pass a real adapter wrapping `FederationManager` once
    /// federation is opted-in.
    pub fn new(
        blockchain: Arc<NodeBlockchain>,
        transport: Arc<dyn TransferTransport>,
        federation: Arc<dyn FederationGate>,
        local_chain_id: String,
    ) -> Self {
        Self {
            blockchain,
            transport,
            federation,
            transfers: Arc::new(RwLock::new(HashMap::new())),
            local_chain_id,
            register_timeout: RwLock::new(DEFAULT_REGISTER_TIMEOUT),
            pending_acks: Arc::new(TokioMutex::new(HashMap::new())),
        }
    }

    /// Register a oneshot sender that will be fired when an ack with a
    /// matching `transfer_id` arrives via
    /// [`Self::deliver_register_ack`].
    ///
    /// Production STOQ transports call this before broadcasting
    /// `TAG_TRANSFER_REGISTER_REQ` so the in-flight `initiate` future
    /// can be woken when the wire-side handler receives the ack.
    pub async fn register_ack_waiter(
        &self,
        transfer_id: String,
        tx: oneshot::Sender<TransferRegisterAck>,
    ) {
        self.pending_acks.lock().await.insert(transfer_id, tx);
    }

    /// Deliver a received `TAG_TRANSFER_REGISTER_ACK` to the awaiting
    /// `initiate` future. Called by the wire handler when the ack
    /// arrives over a different stream than the one that sent the
    /// request.
    ///
    /// Returns `true` when a waiter was present and the ack was
    /// delivered, `false` otherwise (unknown transfer_id, late ack).
    pub async fn deliver_register_ack(&self, ack: TransferRegisterAck) -> bool {
        let waiter = {
            let mut guard = self.pending_acks.lock().await;
            guard.remove(&ack.transfer_id)
        };
        match waiter {
            Some(tx) => tx.send(ack).is_ok(),
            None => false,
        }
    }

    /// Override the default register-ack deadline.
    pub async fn set_register_timeout(&self, dur: Duration) {
        *self.register_timeout.write().await = dur;
    }

    /// Snapshot the in-memory transfer for an ID.
    pub async fn get_transfer(&self, transfer_id: &str) -> Option<CoordinatedTransfer> {
        self.transfers.read().await.get(transfer_id).cloned()
    }

    /// List all tracked transfers (any state).
    pub async fn list_transfers(&self) -> Vec<CoordinatedTransfer> {
        self.transfers.read().await.values().cloned().collect()
    }

    /// Source-side initiator. See module docs for the full state machine.
    pub async fn initiate(
        &self,
        asset_id: AssetId,
        target_chain_id: String,
        target_peer: PeerCertFingerprint,
        target_scope: BlockchainScope,
        manifest: Vec<ShardManifestEntry>,
        state_proof: StateProof,
    ) -> Result<TransferReceiptOutcome, GatewayError> {
        // 1. Federation gate — short-circuit before any blockchain or
        //    wire activity. Phase F.2 substrate provides the real impl;
        //    alpha-default inert uses AllowAllFederation.
        if !self.federation.allow_peer(&target_peer).await {
            return Err(GatewayError::FederationRejected {
                peer: target_peer.clone(),
                detail: "peer is not federation-signed or marked Untrusted".into(),
            });
        }

        let transfer_id = format!(
            "tx-{}-{}",
            chrono::Utc::now().timestamp_millis(),
            &asset_id.to_string()[..8.min(asset_id.to_string().len())]
        );

        // 2. Lock locally — write TransferLockEntry to the source chain.
        let lock_hash = self
            .write_lock_entry(
                &transfer_id,
                &asset_id,
                BlockchainScope::Device,
                target_scope,
                &state_proof,
            )
            .await?;

        // 3. Track in-flight state.
        let coord = CoordinatedTransfer {
            transfer_id: transfer_id.clone(),
            asset_id: asset_id.clone(),
            source_chain_id: self.local_chain_id.clone(),
            target_chain_id: target_chain_id.clone(),
            source_scope: BlockchainScope::Device,
            target_scope,
            target_peer: target_peer.clone(),
            state: CoordinatorState::Locked,
            source_lock_block_hash: Some(lock_hash.clone()),
            target_register_block_hash: None,
            source_release_block_hash: None,
            manifest: manifest.clone(),
            last_error: None,
            state_proof: state_proof.clone(),
        };
        self.transfers
            .write()
            .await
            .insert(transfer_id.clone(), coord);

        // 4. Broadcast TAG_TRANSFER_LOCK.
        let lock_msg = TransferLockMessage {
            transfer_id: transfer_id.clone(),
            asset_id: asset_id.clone(),
            source_chain_id: self.local_chain_id.clone(),
            target_chain_id: target_chain_id.clone(),
            locked_at: chrono::Utc::now().timestamp(),
            state_proof: state_proof.clone(),
            source_scope: BlockchainScope::Device,
            target_scope,
        };
        if let Err(e) = self.transport.broadcast_lock(lock_msg).await {
            warn!(
                "Transfer {} lock broadcast failed: {} — proceeding to register",
                transfer_id, e
            );
            // Broadcast failure is recoverable (peers gossip the lock
            // entry from the chain); we record the warning and continue.
        }

        // 5. Send register request and await ack.
        let timeout = *self.register_timeout.read().await;
        let req = TransferRegisterRequest {
            transfer_id: transfer_id.clone(),
            asset_id: asset_id.clone(),
            source_chain_id: self.local_chain_id.clone(),
            target_chain_id,
            shard_manifest: manifest,
            lock_block_hash: lock_hash.clone(),
            lock_state_proof: state_proof.clone(),
            source_scope: BlockchainScope::Device,
            target_scope,
            sent_at: chrono::Utc::now().timestamp(),
        };

        self.mark_state(&transfer_id, CoordinatorState::ShardsHandedOff)
            .await;

        let send_started_at = std::time::Instant::now();
        let send_result =
            tokio::time::timeout(timeout, self.transport.send_register_request(&target_peer, req, timeout))
                .await;

        let ack = match send_result {
            Ok(Ok(ack)) => ack,
            Ok(Err(e)) => {
                self.rollback(
                    &transfer_id,
                    RollbackReason::Internal {
                        detail: format!("send_register_request failed: {e}"),
                    },
                )
                .await?;
                return Err(e);
            }
            Err(_elapsed) => {
                let elapsed_ms = send_started_at.elapsed().as_millis() as u64;
                self.mark_state(&transfer_id, CoordinatorState::TimedOut).await;
                self.rollback(
                    &transfer_id,
                    RollbackReason::RegisterTimeout { elapsed_ms },
                )
                .await?;
                return Err(GatewayError::RegisterTimeout {
                    transfer_id,
                    elapsed_ms,
                });
            }
        };

        if !ack.accepted {
            let detail = ack.reason.unwrap_or_else(|| "unspecified".into());
            self.mark_state(&transfer_id, CoordinatorState::Failed).await;
            self.rollback(
                &transfer_id,
                RollbackReason::TargetRejected {
                    detail: detail.clone(),
                },
            )
            .await?;
            return Err(GatewayError::TargetRejected {
                transfer_id,
                reason: detail,
            });
        }

        // 6. Ack accepted — record target block hash and write release.
        {
            let mut guard = self.transfers.write().await;
            if let Some(t) = guard.get_mut(&transfer_id) {
                t.state = CoordinatorState::Registered;
                t.target_register_block_hash = Some(ack.target_block_hash.clone());
            }
        }

        let release_hash = self
            .write_release_entry(
                &transfer_id,
                &asset_id,
                "completed cross-network transfer",
                state_proof.clone(),
            )
            .await?;

        // 7. Write cross-chain receipt to source chain.
        let completed_at = chrono::Utc::now().timestamp();
        self.write_receipt_entry(
            &transfer_id,
            &asset_id,
            &release_hash,
            &ack.target_block_hash,
            BlockchainScope::Device,
            target_scope,
            completed_at,
            state_proof,
        )
        .await?;

        // 8. Broadcast TAG_TRANSFER_RELEASE.
        let rel = TransferRelease {
            transfer_id: transfer_id.clone(),
            target_block_hash: ack.target_block_hash.clone(),
            source_block_hash: release_hash.clone(),
            signature: Vec::new(),
            released_at: completed_at,
        };
        if let Err(e) = self.transport.broadcast_release(rel).await {
            warn!(
                "Transfer {} release broadcast failed: {} — receipts already on chain",
                transfer_id, e
            );
        }

        {
            let mut guard = self.transfers.write().await;
            if let Some(t) = guard.get_mut(&transfer_id) {
                t.state = CoordinatorState::Released;
                t.source_release_block_hash = Some(release_hash.clone());
            }
        }

        info!(
            "Transfer {} completed: src_hash={} tgt_hash={}",
            transfer_id, release_hash, ack.target_block_hash
        );

        Ok(TransferReceiptOutcome {
            transfer_id,
            source_block_hash: release_hash,
            target_block_hash: ack.target_block_hash,
            completed_at,
        })
    }

    /// Target-side responder. Validates the lock, writes a registration
    /// entry and a cross-chain receipt to the local chain, returns the
    /// ack the source is awaiting.
    pub async fn handle_register_request(
        &self,
        req: TransferRegisterRequest,
        target_state_proof: StateProof,
    ) -> Result<TransferRegisterAck, GatewayError> {
        // Validate the lock state proof attached to the request.
        if !req.lock_state_proof.validate() {
            return Ok(TransferRegisterAck {
                transfer_id: req.transfer_id,
                target_block_hash: String::new(),
                state_proof: target_state_proof,
                accepted: false,
                reason: Some("source lock state proof failed validation".into()),
                acked_at: chrono::Utc::now().timestamp(),
            });
        }

        // Write registration entry to the target chain.
        let reg_hash = match self
            .write_register_entry(
                &req.transfer_id,
                &req.asset_id,
                req.source_scope,
                req.target_scope,
                &target_state_proof,
            )
            .await
        {
            Ok(h) => h,
            Err(e) => {
                return Ok(TransferRegisterAck {
                    transfer_id: req.transfer_id,
                    target_block_hash: String::new(),
                    state_proof: target_state_proof,
                    accepted: false,
                    reason: Some(format!("write registration failed: {e}")),
                    acked_at: chrono::Utc::now().timestamp(),
                });
            }
        };

        // Track on the target as well so audits can find the receipt
        // from either side.
        let coord = CoordinatedTransfer {
            transfer_id: req.transfer_id.clone(),
            asset_id: req.asset_id.clone(),
            source_chain_id: req.source_chain_id.clone(),
            target_chain_id: req.target_chain_id.clone(),
            source_scope: req.source_scope,
            target_scope: req.target_scope,
            target_peer: PeerCertFingerprint::new(),
            state: CoordinatorState::Registered,
            source_lock_block_hash: Some(req.lock_block_hash.clone()),
            target_register_block_hash: Some(reg_hash.clone()),
            source_release_block_hash: None,
            manifest: req.shard_manifest,
            last_error: None,
            state_proof: target_state_proof.clone(),
        };
        self.transfers
            .write()
            .await
            .insert(req.transfer_id.clone(), coord);

        // Write cross-chain receipt to target chain (mirror of source side).
        let completed_at = chrono::Utc::now().timestamp();
        self.write_receipt_entry(
            &req.transfer_id,
            &req.asset_id,
            &req.lock_block_hash,
            &reg_hash,
            req.source_scope,
            req.target_scope,
            completed_at,
            target_state_proof.clone(),
        )
        .await?;

        Ok(TransferRegisterAck {
            transfer_id: req.transfer_id,
            target_block_hash: reg_hash,
            state_proof: target_state_proof,
            accepted: true,
            reason: None,
            acked_at: completed_at,
        })
    }

    /// Reject a register request explicitly. Used by tests and by future
    /// admission policies; produces a non-accepted ack so the source
    /// rolls back.
    pub fn reject_register_request(
        req: &TransferRegisterRequest,
        reason: impl Into<String>,
    ) -> TransferRegisterAck {
        TransferRegisterAck {
            transfer_id: req.transfer_id.clone(),
            target_block_hash: String::new(),
            state_proof: StateProof::default(),
            accepted: false,
            reason: Some(reason.into()),
            acked_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Phase G.2 — scan the local blockchain for in-flight transfers
    /// (lock entries with no matching release) and rebuild in-memory
    /// state.
    ///
    /// Implementation: walks `NodeBlockchain::get_chain()` once,
    /// extracting every transfer-related entry from each block's
    /// [`StoragePointer::Local`] payload (the lock/register/release/
    /// receipt entries are JSON-serialized into the storage path during
    /// `append_block`). For each transfer ID:
    ///
    /// * a `lock` without a `release` → in-flight, restored as
    ///   [`CoordinatorState::Locked`]
    /// * a `lock` with a `release` (rollback or completion) → terminal,
    ///   skipped
    ///
    /// The restored transfers are inserted into the in-memory map and
    /// returned. Callers (typically daemon boot) can then decide whether
    /// to re-drive the state machine (re-send register-request) or
    /// surface them to operators.
    ///
    /// Safe to call when no transfers are in flight (returns empty).
    pub async fn resume_in_flight(&self) -> Result<Vec<CoordinatedTransfer>, GatewayError> {
        let chain = self.blockchain.get_chain().await;

        // Collect everything keyed by transfer_id. Locks need to land
        // first because a single block may carry multiple entries.
        let mut locks: HashMap<String, (TransferLockEntry, String, StateProof)> = HashMap::new();
        let mut releases: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        // Optional registration / receipt hashes — recorded so resumed
        // transfers carry the same target_register_block_hash and
        // source_release_block_hash they had pre-restart.
        let mut registrations: HashMap<String, (TransferRegistrationEntry, String)> =
            HashMap::new();
        let mut receipts: HashMap<String, TransferReceipt> = HashMap::new();

        for block in chain.into_iter() {
            let block_hash = block.hash.clone();
            for entry in block.entries.iter() {
                let payload_str = match &entry.storage_pointer {
                    StoragePointer::Local { path } => path,
                    _ => continue,
                };
                let payload_bytes = payload_str.as_bytes();

                // Each transfer-related block carries exactly one entry
                // type. Try lock first, then register, then release,
                // then receipt — first match wins.
                if let Ok(lock) = serde_json::from_slice::<TransferLockEntry>(payload_bytes) {
                    // TransferLockEntry has a `proof_hash` field unique
                    // to it; without that field a deserialize-from-arbitrary
                    // JSON could collide. Distinguish from
                    // TransferRegistrationEntry by the presence of
                    // `locked_at` vs `registered_at`. serde silently
                    // accepts missing fields when types overlap, so we
                    // disambiguate by trying the more specific shapes
                    // first below and re-checking on the parsed value.
                    if !lock.transfer_id.is_empty()
                        && lock.locked_at != 0
                        && payload_str.contains("\"locked_at\"")
                    {
                        locks.insert(
                            lock.transfer_id.clone(),
                            (lock, block_hash.clone(), entry.state_proof.clone()),
                        );
                        continue;
                    }
                }
                if let Ok(reg) =
                    serde_json::from_slice::<TransferRegistrationEntry>(payload_bytes)
                {
                    if !reg.transfer_id.is_empty()
                        && reg.registered_at != 0
                        && payload_str.contains("\"registered_at\"")
                    {
                        registrations
                            .insert(reg.transfer_id.clone(), (reg, block_hash.clone()));
                        continue;
                    }
                }
                if let Ok(rel) = serde_json::from_slice::<TransferReleaseEntry>(payload_bytes) {
                    if !rel.transfer_id.is_empty()
                        && rel.released_at != 0
                        && payload_str.contains("\"released_at\"")
                    {
                        releases.insert(rel.transfer_id);
                        continue;
                    }
                }
                if let Ok(rcpt) = serde_json::from_slice::<TransferReceipt>(payload_bytes) {
                    if !rcpt.transfer_id.is_empty()
                        && rcpt.completed_at != 0
                        && payload_str.contains("\"completed_at\"")
                    {
                        receipts.insert(rcpt.transfer_id.clone(), rcpt);
                        continue;
                    }
                }
            }
        }

        // For every lock without a release, reconstruct CoordinatedTransfer.
        let mut resumed: Vec<CoordinatedTransfer> = Vec::new();
        for (transfer_id, (lock, lock_block_hash, state_proof)) in locks.into_iter() {
            if releases.contains(&transfer_id) {
                // Terminal (either completed-with-release or rollback) — skip.
                continue;
            }
            let (target_register_block_hash, state) =
                match registrations.get(&transfer_id) {
                    Some((_, hash)) => {
                        // Registration present but no release → ack was
                        // received, release write was interrupted.
                        (Some(hash.clone()), CoordinatorState::Registered)
                    }
                    None => (None, CoordinatorState::Locked),
                };

            // Target chain id is best-effort: receipt has it explicitly,
            // otherwise we fall back to local_chain_id (single-chain
            // alpha) so the caller can repopulate from peer state.
            let target_chain_id = receipts
                .get(&transfer_id)
                .map(|r| r.target_chain_id.clone())
                .unwrap_or_else(|| self.local_chain_id.clone());

            let coord = CoordinatedTransfer {
                transfer_id: transfer_id.clone(),
                asset_id: AssetId::from(lock.asset_id.as_str()),
                source_chain_id: self.local_chain_id.clone(),
                target_chain_id,
                source_scope: lock.source_scope,
                target_scope: lock.target_scope,
                target_peer: PeerCertFingerprint::new(),
                state,
                source_lock_block_hash: Some(lock_block_hash),
                target_register_block_hash,
                source_release_block_hash: None,
                manifest: Vec::new(), // not preserved on chain (carried by req)
                last_error: None,
                state_proof,
            };

            self.transfers
                .write()
                .await
                .insert(transfer_id.clone(), coord.clone());
            resumed.push(coord);
        }

        if !resumed.is_empty() {
            info!(
                "TransferCoordinator::resume_in_flight restored {} in-flight transfer(s)",
                resumed.len()
            );
        } else {
            debug!("TransferCoordinator::resume_in_flight found no in-flight transfers");
        }

        Ok(resumed)
    }

    // -----------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------

    async fn mark_state(&self, transfer_id: &str, state: CoordinatorState) {
        let mut guard = self.transfers.write().await;
        if let Some(t) = guard.get_mut(transfer_id) {
            t.state = state;
        }
    }

    async fn rollback(
        &self,
        transfer_id: &str,
        reason: RollbackReason,
    ) -> Result<(), GatewayError> {
        let (asset_id, state_proof) = {
            let guard = self.transfers.read().await;
            match guard.get(transfer_id) {
                Some(t) => (t.asset_id.to_string(), t.state_proof.clone()),
                None => (String::new(), StateProof::default()),
            }
        };
        let release_hash = self
            .write_release_entry(
                transfer_id,
                &AssetId::from(asset_id.as_str()),
                &reason.to_string(),
                state_proof,
            )
            .await?;

        {
            let mut guard = self.transfers.write().await;
            if let Some(t) = guard.get_mut(transfer_id) {
                t.state = CoordinatorState::RolledBack;
                t.source_release_block_hash = Some(release_hash);
                t.last_error = Some(reason.to_string());
            }
        }

        let rb = TransferRollback {
            transfer_id: transfer_id.to_string(),
            reason,
            rolled_back_at: chrono::Utc::now().timestamp(),
        };
        if let Err(e) = self.transport.broadcast_rollback(rb).await {
            warn!(
                "Transfer {} rollback broadcast failed: {} — local rollback recorded on chain",
                transfer_id, e
            );
        }
        Ok(())
    }

    async fn write_lock_entry(
        &self,
        transfer_id: &str,
        asset_id: &AssetId,
        source_scope: BlockchainScope,
        target_scope: BlockchainScope,
        state_proof: &StateProof,
    ) -> Result<String, GatewayError> {
        let entry = TransferLockEntry {
            transfer_id: transfer_id.to_string(),
            asset_id: asset_id.to_string(),
            source_scope,
            target_scope,
            locked_at: chrono::Utc::now().timestamp(),
            proof_hash: *blake3::hash(b"transfer-lock").as_bytes(),
        };
        let bytes = serde_json::to_vec(&entry).map_err(|e| GatewayError::TransportFailure {
            transfer_id: transfer_id.to_string(),
            detail: format!("serialize lock entry: {e}"),
        })?;
        self.append_block(&bytes, "lock", state_proof.clone()).await
    }

    async fn write_register_entry(
        &self,
        transfer_id: &str,
        asset_id: &AssetId,
        source_scope: BlockchainScope,
        target_scope: BlockchainScope,
        state_proof: &StateProof,
    ) -> Result<String, GatewayError> {
        let entry = TransferRegistrationEntry {
            transfer_id: transfer_id.to_string(),
            asset_id: asset_id.to_string(),
            source_scope,
            target_scope,
            registered_at: chrono::Utc::now().timestamp(),
            proof_hash: *blake3::hash(b"transfer-register").as_bytes(),
        };
        let bytes = serde_json::to_vec(&entry).map_err(|e| GatewayError::TransportFailure {
            transfer_id: transfer_id.to_string(),
            detail: format!("serialize register entry: {e}"),
        })?;
        self.append_block(&bytes, "register", state_proof.clone())
            .await
    }

    async fn write_release_entry(
        &self,
        transfer_id: &str,
        asset_id: &AssetId,
        reason: &str,
        state_proof: StateProof,
    ) -> Result<String, GatewayError> {
        let entry = TransferReleaseEntry {
            transfer_id: transfer_id.to_string(),
            asset_id: asset_id.to_string(),
            released_at: chrono::Utc::now().timestamp(),
            reason: reason.to_string(),
        };
        let bytes = serde_json::to_vec(&entry).map_err(|e| GatewayError::TransportFailure {
            transfer_id: transfer_id.to_string(),
            detail: format!("serialize release entry: {e}"),
        })?;
        self.append_block(&bytes, "release", state_proof).await
    }

    async fn write_receipt_entry(
        &self,
        transfer_id: &str,
        asset_id: &AssetId,
        source_block_hash: &str,
        target_block_hash: &str,
        source_scope: BlockchainScope,
        target_scope: BlockchainScope,
        completed_at: i64,
        state_proof: StateProof,
    ) -> Result<String, GatewayError> {
        let receipt = TransferReceipt {
            transfer_id: transfer_id.to_string(),
            source_chain_id: self.local_chain_id.clone(),
            target_chain_id: self.local_chain_id.clone(),
            source_block_hash: source_block_hash.to_string(),
            target_block_hash: target_block_hash.to_string(),
            completed_at,
            asset_id: asset_id.to_string(),
            source_scope,
            target_scope,
        };
        let bytes = serde_json::to_vec(&receipt).map_err(|e| GatewayError::TransportFailure {
            transfer_id: transfer_id.to_string(),
            detail: format!("serialize receipt entry: {e}"),
        })?;
        self.append_block(&bytes, "receipt", state_proof).await
    }

    /// Append a serialized transfer entry as a block on the local chain.
    /// Returns the BLAKE3 hex hash of the new block.
    async fn append_block(
        &self,
        entry_bytes: &[u8],
        label: &str,
        state_proof: StateProof,
    ) -> Result<String, GatewayError> {
        let asset_hash = *blake3::hash(entry_bytes).as_bytes();
        let proof_hash =
            *blake3::hash(format!("transfer-{label}").as_bytes()).as_bytes();

        let registration = crate::assets::core::AssetRegistration::from_asset_data(
            &crate::assets::core::asset_id::AssetData {
                config: Vec::new(),
                definition: entry_bytes.to_vec(),
                metadata: format!("transfer-{label}").into_bytes(),
            },
            crate::assets::core::asset_id::NetworkScope::Global,
            crate::assets::core::asset_id::AssetCategory::BaseSystem(
                crate::assets::core::asset_id::BaseSystemType::Blockchain,
            ),
        );

        let block_entry = BlockAssetEntry {
            asset_hash,
            proof_hash,
            state_proof,
            storage_pointer: StoragePointer::Local {
                path: String::from_utf8_lossy(entry_bytes).to_string(),
            },
            registration,
        };

        let block = self
            .blockchain
            .add_block(vec![block_entry])
            .await
            .map_err(|e| GatewayError::TransportFailure {
                transfer_id: label.to_string(),
                detail: format!("blockchain add_block ({label}): {e}"),
            })?;

        Ok(block.hash)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use hypermesh_lib::ContentHash;
    use std::sync::Mutex as StdMutex;

    /// In-process mock transport that captures sent messages and lets a
    /// test register a programmable target responder.
    pub struct MockTransport {
        pub locks: Arc<StdMutex<Vec<TransferLockMessage>>>,
        pub releases: Arc<StdMutex<Vec<TransferRelease>>>,
        pub rollbacks: Arc<StdMutex<Vec<TransferRollback>>>,
        responder: Arc<StdMutex<Option<Box<dyn Fn(TransferRegisterRequest) -> RegisterReply + Send>>>>,
    }

    pub enum RegisterReply {
        Ack(TransferRegisterAck),
        Reject(String),
        Silent, // simulate timeout — never returns
    }

    impl MockTransport {
        pub fn new() -> Self {
            Self {
                locks: Arc::new(StdMutex::new(Vec::new())),
                releases: Arc::new(StdMutex::new(Vec::new())),
                rollbacks: Arc::new(StdMutex::new(Vec::new())),
                responder: Arc::new(StdMutex::new(None)),
            }
        }

        pub fn set_responder<F>(&self, f: F)
        where
            F: Fn(TransferRegisterRequest) -> RegisterReply + Send + 'static,
        {
            *self.responder.lock().expect("test: responder lock") = Some(Box::new(f));
        }
    }

    #[async_trait]
    impl TransferTransport for MockTransport {
        async fn broadcast_lock(&self, msg: TransferLockMessage) -> Result<(), GatewayError> {
            self.locks.lock().expect("test: locks lock").push(msg);
            Ok(())
        }

        async fn send_register_request(
            &self,
            _peer: &PeerCertFingerprint,
            req: TransferRegisterRequest,
            _deadline: Duration,
        ) -> Result<TransferRegisterAck, GatewayError> {
            let reply = {
                let guard = self.responder.lock().expect("test: responder lock");
                match guard.as_ref() {
                    Some(f) => f(req.clone()),
                    None => RegisterReply::Ack(TransferRegisterAck {
                        transfer_id: req.transfer_id.clone(),
                        target_block_hash: format!("tgt-{}", req.transfer_id),
                        state_proof: StateProof::new_for_testing(),
                        accepted: true,
                        reason: None,
                        acked_at: 0,
                    }),
                }
            };
            match reply {
                RegisterReply::Ack(ack) => Ok(ack),
                RegisterReply::Reject(detail) => Ok(TransferRegisterAck {
                    transfer_id: req.transfer_id,
                    target_block_hash: String::new(),
                    state_proof: StateProof::default(),
                    accepted: false,
                    reason: Some(detail),
                    acked_at: 0,
                }),
                RegisterReply::Silent => {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    Err(GatewayError::TransportFailure {
                        transfer_id: req.transfer_id,
                        detail: "silent (should have timed out)".into(),
                    })
                }
            }
        }

        async fn broadcast_release(&self, msg: TransferRelease) -> Result<(), GatewayError> {
            self.releases.lock().expect("test: releases lock").push(msg);
            Ok(())
        }

        async fn broadcast_rollback(&self, msg: TransferRollback) -> Result<(), GatewayError> {
            self.rollbacks
                .lock()
                .expect("test: rollbacks lock")
                .push(msg);
            Ok(())
        }
    }

    fn make_blockchain() -> Arc<NodeBlockchain> {
        use crate::matrix::coordinate::MatrixCoordinate;
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        Arc::new(NodeBlockchain::new(coord))
    }

    fn sample_manifest() -> Vec<ShardManifestEntry> {
        vec![ShardManifestEntry {
            shard_id: ContentHash([0xAB; 32]),
            size_bytes: 64,
            source_matrix: None,
        }]
    }

    #[tokio::test]
    async fn test_coordinator_unfederated_peer_short_circuits() {
        let bc = make_blockchain();
        let transport = Arc::new(MockTransport::new());
        let coord = TransferCoordinator::new(
            bc.clone(),
            transport.clone(),
            Arc::new(DenyAllFederation),
            "src-chain".into(),
        );

        let err = coord
            .initiate(
                AssetId::from("asset-deny"),
                "tgt-chain".into(),
                "peer-fingerprint-x".into(),
                BlockchainScope::Network,
                sample_manifest(),
                StateProof::new_for_testing(),
            )
            .await
            .unwrap_err();

        assert!(matches!(err, GatewayError::FederationRejected { .. }));
        // No wire traffic, no chain growth.
        assert!(transport
            .locks
            .lock()
            .expect("test: locks lock")
            .is_empty());
    }

    #[tokio::test]
    async fn test_coordinator_happy_path() {
        let bc = make_blockchain();
        let initial_height = bc.get_height().await;
        let transport = Arc::new(MockTransport::new());
        let coord = TransferCoordinator::new(
            bc.clone(),
            transport.clone(),
            Arc::new(AllowAllFederation),
            "src-chain".into(),
        );

        let outcome = coord
            .initiate(
                AssetId::from("asset-happy"),
                "tgt-chain".into(),
                "peer-fp-1".into(),
                BlockchainScope::Network,
                sample_manifest(),
                StateProof::new_for_testing(),
            )
            .await
            .expect("test: initiate");

        // Source chain should now have lock + release + receipt blocks.
        let new_height = bc.get_height().await;
        assert_eq!(new_height, initial_height + 3, "expected 3 new blocks");

        assert!(!outcome.source_block_hash.is_empty());
        assert!(outcome.target_block_hash.starts_with("tgt-"));

        let locks = transport.locks.lock().expect("test: locks lock").clone();
        assert_eq!(locks.len(), 1);
        let releases = transport.releases.lock().expect("test: releases lock").clone();
        assert_eq!(releases.len(), 1);

        let stored = coord.get_transfer(&outcome.transfer_id).await.expect("test: tracked");
        assert_eq!(stored.state, CoordinatorState::Released);
    }

    #[tokio::test]
    async fn test_coordinator_target_rejection() {
        let bc = make_blockchain();
        let transport = Arc::new(MockTransport::new());
        transport.set_responder(|_req| RegisterReply::Reject("hash mismatch".into()));

        let coord = TransferCoordinator::new(
            bc.clone(),
            transport.clone(),
            Arc::new(AllowAllFederation),
            "src-chain".into(),
        );

        let err = coord
            .initiate(
                AssetId::from("asset-reject"),
                "tgt-chain".into(),
                "peer-fp-2".into(),
                BlockchainScope::Network,
                sample_manifest(),
                StateProof::new_for_testing(),
            )
            .await
            .unwrap_err();

        assert!(matches!(err, GatewayError::TargetRejected { .. }));
        let rollbacks = transport
            .rollbacks
            .lock()
            .expect("test: rollbacks lock")
            .clone();
        assert_eq!(rollbacks.len(), 1);
        assert!(matches!(
            rollbacks[0].reason,
            RollbackReason::TargetRejected { .. }
        ));
    }

    #[tokio::test]
    async fn test_coordinator_register_timeout() {
        let bc = make_blockchain();
        let transport = Arc::new(MockTransport::new());
        transport.set_responder(|_req| RegisterReply::Silent);

        let coord = TransferCoordinator::new(
            bc.clone(),
            transport.clone(),
            Arc::new(AllowAllFederation),
            "src-chain".into(),
        );
        coord.set_register_timeout(Duration::from_millis(150)).await;

        let err = coord
            .initiate(
                AssetId::from("asset-timeout"),
                "tgt-chain".into(),
                "peer-fp-3".into(),
                BlockchainScope::Network,
                sample_manifest(),
                StateProof::new_for_testing(),
            )
            .await
            .unwrap_err();

        assert!(matches!(err, GatewayError::RegisterTimeout { .. }));
        let rollbacks = transport
            .rollbacks
            .lock()
            .expect("test: rollbacks lock")
            .clone();
        assert_eq!(rollbacks.len(), 1);
        assert!(matches!(
            rollbacks[0].reason,
            RollbackReason::RegisterTimeout { .. }
        ));
    }

    #[tokio::test]
    async fn test_handle_register_request_writes_target_chain() {
        let bc = make_blockchain();
        let initial_height = bc.get_height().await;
        let transport = Arc::new(MockTransport::new());
        let coord = TransferCoordinator::new(
            bc.clone(),
            transport,
            Arc::new(AllowAllFederation),
            "tgt-chain".into(),
        );

        let req = TransferRegisterRequest {
            transfer_id: "tx-tgt-1".into(),
            asset_id: AssetId::from("asset-tgt"),
            source_chain_id: "src-chain".into(),
            target_chain_id: "tgt-chain".into(),
            shard_manifest: sample_manifest(),
            lock_block_hash: "src-lock-hash".into(),
            lock_state_proof: StateProof::new_for_testing(),
            source_scope: BlockchainScope::Device,
            target_scope: BlockchainScope::Network,
            sent_at: 0,
        };

        let ack = coord
            .handle_register_request(req, StateProof::new_for_testing())
            .await
            .expect("test: handle register");

        assert!(ack.accepted, "ack must be accepted");
        assert!(!ack.target_block_hash.is_empty());

        // Target chain grew by registration + receipt = 2 blocks.
        let new_height = bc.get_height().await;
        assert_eq!(new_height, initial_height + 2);
    }

    #[tokio::test]
    async fn test_resume_in_flight_empty_chain_returns_empty() {
        let bc = make_blockchain();
        let transport = Arc::new(MockTransport::new());
        let coord = TransferCoordinator::new(
            bc,
            transport,
            Arc::new(AllowAllFederation),
            "any".into(),
        );
        let resumed = coord
            .resume_in_flight()
            .await
            .expect("test: empty chain returns ok");
        assert!(resumed.is_empty());
    }

    /// Phase G.2: chain-scan recovery picks up locks without matching
    /// release entries, ignores fully-completed transfers, and ignores
    /// rollbacks (which are themselves release entries).
    #[tokio::test]
    async fn test_resume_in_flight_chain_scan_g2() {
        let bc = make_blockchain();
        let transport = Arc::new(MockTransport::new());
        let coord = TransferCoordinator::new(
            bc.clone(),
            transport,
            Arc::new(AllowAllFederation),
            "src-chain".into(),
        );
        let proof = StateProof::new_for_testing();

        // Seed three transfers:
        //   tx-complete: lock + release (should NOT be in resume list)
        //   tx-inflight: lock only      (SHOULD be in resume list)
        //   tx-rolled-back: lock + rollback-release (NOT in resume list)
        coord
            .write_lock_entry(
                "tx-complete",
                &AssetId::from("asset-complete"),
                BlockchainScope::Device,
                BlockchainScope::Network,
                &proof,
            )
            .await
            .expect("test: lock complete");
        coord
            .write_release_entry(
                "tx-complete",
                &AssetId::from("asset-complete"),
                "completed cross-network transfer",
                proof.clone(),
            )
            .await
            .expect("test: release complete");

        coord
            .write_lock_entry(
                "tx-inflight",
                &AssetId::from("asset-inflight"),
                BlockchainScope::Device,
                BlockchainScope::Network,
                &proof,
            )
            .await
            .expect("test: lock inflight");

        coord
            .write_lock_entry(
                "tx-rolled-back",
                &AssetId::from("asset-rolled"),
                BlockchainScope::Device,
                BlockchainScope::Network,
                &proof,
            )
            .await
            .expect("test: lock rolled");
        coord
            .write_release_entry(
                "tx-rolled-back",
                &AssetId::from("asset-rolled"),
                "TargetRejected: hash mismatch",
                proof.clone(),
            )
            .await
            .expect("test: release rolled");

        // Build a NEW coordinator on the same chain — simulates daemon
        // restart with persisted on-chain state but empty in-memory map.
        let transport2 = Arc::new(MockTransport::new());
        let coord2 = TransferCoordinator::new(
            bc,
            transport2,
            Arc::new(AllowAllFederation),
            "src-chain".into(),
        );

        let resumed = coord2
            .resume_in_flight()
            .await
            .expect("test: resume_in_flight scan");

        assert_eq!(
            resumed.len(),
            1,
            "expected exactly 1 in-flight transfer, got {:?}",
            resumed.iter().map(|t| &t.transfer_id).collect::<Vec<_>>()
        );
        assert_eq!(resumed[0].transfer_id, "tx-inflight");
        assert_eq!(resumed[0].state, CoordinatorState::Locked);
        assert!(resumed[0].source_lock_block_hash.is_some());
        assert!(resumed[0].source_release_block_hash.is_none());

        // The in-memory map was repopulated.
        let stored = coord2.get_transfer("tx-inflight").await;
        assert!(stored.is_some(), "in-memory map updated by resume scan");
    }
}
