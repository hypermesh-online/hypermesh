// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase G.1 cross-network transfer wire protocol.
//!
//! Five message types travel across STOQ between source and target nodes
//! during a cross-network asset transfer:
//!
//! 1. [`TransferLockMessage`] — broadcast by the source after writing a
//!    [`TransferLockEntry`](super::asset_transfer::TransferLockEntry) to
//!    its own chain. Tag: `TAG_TRANSFER_LOCK` (0x40).
//! 2. [`TransferRegisterRequest`] — sent point-to-point from source to
//!    the target peer with the shard manifest and lock proof. Tag:
//!    `TAG_TRANSFER_REGISTER_REQ` (0x41).
//! 3. [`TransferRegisterAck`] — target's response after writing a
//!    [`TransferRegistrationEntry`](super::asset_transfer::TransferRegistrationEntry)
//!    to its own chain. Tag: `TAG_TRANSFER_REGISTER_ACK` (0x42).
//! 4. [`TransferRelease`] — broadcast by the source after receiving the
//!    ack and writing the release entry. Tag: `TAG_TRANSFER_RELEASE` (0x43).
//! 5. [`TransferRollback`] — broadcast on rejection / timeout to restore
//!    pre-transfer state. Tag: `TAG_TRANSFER_ROLLBACK` (0x44).
//!
//! State machine driven by [`TransferCoordinator`](super::transfer_coordinator::TransferCoordinator):
//!
//! ```text
//! Initiated → Locked → ShardsHandedOff → Registered → Released   (happy path)
//! Initiated → Locked → ShardsHandedOff → Failed → RolledBack     (target rejection)
//! Initiated → Locked → TimedOut → RolledBack                     (no response)
//! ```

use hypermesh_lib::{AssetId, BlockchainScope, ContentHash};
use serde::{Deserialize, Serialize};
use trustchain::proof_of_state::StateProof;

use super::asset_transfer::RollbackReason;

/// Cross-node transfer state — extends the legacy in-memory
/// [`TransferStatus`](super::asset_transfer::TransferStatus) state machine
/// with explicit cross-node milestones.
///
/// Backward-compatibility note: existing single-node tests keep using
/// `TransferStatus`. `CoordinatorState` is the cross-node state used by
/// [`TransferCoordinator`](super::transfer_coordinator::TransferCoordinator).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CoordinatorState {
    /// Coordinator created the transfer but has not yet broadcast Lock.
    Initiated,
    /// `TAG_TRANSFER_LOCK` written to source chain and broadcast.
    Locked,
    /// All shards handed off to the target peer; awaiting register-ack.
    ShardsHandedOff,
    /// Target acknowledged registration; release pending.
    Registered,
    /// Source wrote the release entry and broadcast `TAG_TRANSFER_RELEASE`.
    Released,
    /// Register-ack deadline elapsed; rollback in progress.
    TimedOut,
    /// Target explicitly rejected registration.
    Failed,
    /// Lock released after failure or timeout.
    RolledBack,
}

impl std::fmt::Display for CoordinatorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Initiated => write!(f, "Initiated"),
            Self::Locked => write!(f, "Locked"),
            Self::ShardsHandedOff => write!(f, "ShardsHandedOff"),
            Self::Registered => write!(f, "Registered"),
            Self::Released => write!(f, "Released"),
            Self::TimedOut => write!(f, "TimedOut"),
            Self::Failed => write!(f, "Failed"),
            Self::RolledBack => write!(f, "RolledBack"),
        }
    }
}

/// Hex-encoded peer certificate fingerprint — identifies the target node
/// by its FALCON-1024 cert hash. Federation-signed peers are validated
/// via [`FederationManager::is_federation_signed`](trustchain::ca::FederationManager::is_federation_signed).
pub type PeerCertFingerprint = String;

/// Manifest entry describing a single shard that will be handed off to
/// the target node during a cross-network transfer. The receiver fetches
/// the bytes via the existing
/// [`ShardTransport`](crate::network::shard_transport::ShardTransport).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShardManifestEntry {
    /// BLAKE3 content hash of the shard.
    pub shard_id: ContentHash,
    /// Size in bytes (informational; receiver verifies via hash).
    pub size_bytes: u64,
    /// Optional source matrix coordinate for routing hints.
    pub source_matrix: Option<(i32, i32, i32)>,
}

/// `TAG_TRANSFER_LOCK` (0x40) wire payload.
///
/// Broadcast after the source writes its
/// [`TransferLockEntry`](super::asset_transfer::TransferLockEntry).
/// Carries the source state proof (PoSpace + PoStake) so peers can
/// independently audit the lock without re-fetching the source chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferLockMessage {
    /// Unique transfer ID.
    pub transfer_id: String,
    /// Asset being transferred.
    pub asset_id: AssetId,
    /// Source chain (network ID).
    pub source_chain_id: String,
    /// Target chain (network ID).
    pub target_chain_id: String,
    /// Unix timestamp of the lock.
    pub locked_at: i64,
    /// Source-side PoSpace + PoStake proof.
    pub state_proof: StateProof,
    /// Source scope.
    pub source_scope: BlockchainScope,
    /// Target scope.
    pub target_scope: BlockchainScope,
}

/// `TAG_TRANSFER_REGISTER_REQ` (0x41) wire payload — point-to-point from
/// source to target peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRegisterRequest {
    /// Transfer this request belongs to.
    pub transfer_id: String,
    /// Asset being transferred.
    pub asset_id: AssetId,
    /// Source chain (network ID).
    pub source_chain_id: String,
    /// Target chain (network ID).
    pub target_chain_id: String,
    /// All shards comprising the asset.
    pub shard_manifest: Vec<ShardManifestEntry>,
    /// Hash of the source-chain lock block — proves the lock exists.
    pub lock_block_hash: String,
    /// State proof attached to the lock entry (for bilateral verification).
    pub lock_state_proof: StateProof,
    /// Source scope.
    pub source_scope: BlockchainScope,
    /// Target scope.
    pub target_scope: BlockchainScope,
    /// Unix timestamp when the request was sent.
    pub sent_at: i64,
}

/// `TAG_TRANSFER_REGISTER_ACK` (0x42) wire payload — target's response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRegisterAck {
    /// Transfer this ack belongs to.
    pub transfer_id: String,
    /// Hash of the target-chain registration block.
    pub target_block_hash: String,
    /// Target-side state proof committing to the registration.
    pub state_proof: StateProof,
    /// Whether registration succeeded. `false` is functionally a
    /// rejection; the `reason` field carries detail. Sources that
    /// receive `accepted=false` immediately broadcast `TAG_TRANSFER_ROLLBACK`.
    pub accepted: bool,
    /// Optional reject reason when `accepted=false`.
    pub reason: Option<String>,
    /// Unix timestamp when the ack was generated.
    pub acked_at: i64,
}

/// `TAG_TRANSFER_RELEASE` (0x43) wire payload — broadcast by source on
/// successful completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRelease {
    /// Transfer ID.
    pub transfer_id: String,
    /// Hash of the target-chain registration block (cross-chain link).
    pub target_block_hash: String,
    /// Hash of the source-chain release block.
    pub source_block_hash: String,
    /// FALCON-1024 signature over `transfer_id || target_block_hash`
    /// (raw bytes). Empty in alpha; populated when threshold-mode is on.
    #[serde(with = "serde_bytes", default)]
    pub signature: Vec<u8>,
    /// Unix timestamp of release.
    pub released_at: i64,
}

/// `TAG_TRANSFER_ROLLBACK` (0x44) wire payload — broadcast on rejection,
/// timeout, or unrecoverable error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRollback {
    /// Transfer ID.
    pub transfer_id: String,
    /// Why the rollback was triggered.
    pub reason: RollbackReason,
    /// Unix timestamp.
    pub rolled_back_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_message_round_trip() {
        let msg = TransferLockMessage {
            transfer_id: "tx-001".into(),
            asset_id: AssetId::from("asset-abc"),
            source_chain_id: "device-A".into(),
            target_chain_id: "network-B".into(),
            locked_at: 1_700_000_000,
            state_proof: StateProof::new_for_testing(),
            source_scope: BlockchainScope::Device,
            target_scope: BlockchainScope::Network,
        };
        let bytes = serde_json::to_vec(&msg).expect("test: serialize lock");
        let parsed: TransferLockMessage =
            serde_json::from_slice(&bytes).expect("test: deserialize lock");
        assert_eq!(parsed.transfer_id, "tx-001");
        assert_eq!(parsed.locked_at, 1_700_000_000);
        assert_eq!(parsed.source_scope, BlockchainScope::Device);
    }

    #[test]
    fn test_register_request_round_trip() {
        let req = TransferRegisterRequest {
            transfer_id: "tx-002".into(),
            asset_id: AssetId::from("asset-def"),
            source_chain_id: "src".into(),
            target_chain_id: "dst".into(),
            shard_manifest: vec![ShardManifestEntry {
                shard_id: ContentHash([0x33; 32]),
                size_bytes: 1024,
                source_matrix: Some((1, 2, 3)),
            }],
            lock_block_hash: "deadbeef".into(),
            lock_state_proof: StateProof::new_for_testing(),
            source_scope: BlockchainScope::Device,
            target_scope: BlockchainScope::Network,
            sent_at: 1,
        };
        let bytes = serde_json::to_vec(&req).expect("test: serialize req");
        let parsed: TransferRegisterRequest =
            serde_json::from_slice(&bytes).expect("test: deserialize req");
        assert_eq!(parsed.shard_manifest.len(), 1);
        assert_eq!(parsed.shard_manifest[0].size_bytes, 1024);
    }

    #[test]
    fn test_register_ack_round_trip() {
        let ack = TransferRegisterAck {
            transfer_id: "tx-003".into(),
            target_block_hash: "abc123".into(),
            state_proof: StateProof::new_for_testing(),
            accepted: false,
            reason: Some("hash mismatch".into()),
            acked_at: 2,
        };
        let bytes = serde_json::to_vec(&ack).expect("test: serialize ack");
        let parsed: TransferRegisterAck =
            serde_json::from_slice(&bytes).expect("test: deserialize ack");
        assert!(!parsed.accepted);
        assert_eq!(parsed.reason.as_deref(), Some("hash mismatch"));
    }

    #[test]
    fn test_release_round_trip() {
        let rel = TransferRelease {
            transfer_id: "tx-004".into(),
            target_block_hash: "tgt".into(),
            source_block_hash: "src".into(),
            signature: vec![0xAB, 0xCD],
            released_at: 3,
        };
        let bytes = serde_json::to_vec(&rel).expect("test: serialize rel");
        let parsed: TransferRelease =
            serde_json::from_slice(&bytes).expect("test: deserialize rel");
        assert_eq!(parsed.signature, vec![0xAB, 0xCD]);
    }

    #[test]
    fn test_rollback_round_trip() {
        let rb = TransferRollback {
            transfer_id: "tx-005".into(),
            reason: RollbackReason::RegisterTimeout { elapsed_ms: 5000 },
            rolled_back_at: 4,
        };
        let bytes = serde_json::to_vec(&rb).expect("test: serialize rb");
        let parsed: TransferRollback =
            serde_json::from_slice(&bytes).expect("test: deserialize rb");
        assert!(matches!(
            parsed.reason,
            RollbackReason::RegisterTimeout { elapsed_ms: 5000 }
        ));
    }

    #[test]
    fn test_coordinator_state_display() {
        assert_eq!(CoordinatorState::Initiated.to_string(), "Initiated");
        assert_eq!(CoordinatorState::ShardsHandedOff.to_string(), "ShardsHandedOff");
        assert_eq!(CoordinatorState::RolledBack.to_string(), "RolledBack");
    }
}
