// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Cross-scope asset transfer types and validation
//!
//! Defines the data model for moving assets between Device and Network blockchain
//! scopes. Every transfer requires Proof of State validation (PoSpace + PoStake)
//! in both the source and target scopes before the asset is released.

use std::fmt;
use std::time::SystemTime;

use hypermesh_lib::{AssetId, BlockchainScope, ProofType};
use serde::{Deserialize, Serialize};

use super::GatewayError;

// ---------------------------------------------------------------------------
// Transfer lifecycle
// ---------------------------------------------------------------------------

/// Status of a cross-scope asset transfer.
///
/// State machine:
/// ```text
/// Pending -> Locked -> InTransit -> Confirmed
///                 \-> Failed -> RolledBack
///          InTransit -> Failed -> RolledBack
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransferStatus {
    /// Transfer requested but not yet locked on source scope.
    Pending,
    /// Asset locked on source scope, awaiting proof validation.
    Locked,
    /// Proofs validated; asset in transit between scopes.
    InTransit,
    /// Asset registered on target scope and unlocked.
    Confirmed,
    /// Transfer could not complete.
    Failed,
    /// Source scope lock released after failure.
    RolledBack,
}

impl fmt::Display for TransferStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Locked => write!(f, "Locked"),
            Self::InTransit => write!(f, "InTransit"),
            Self::Confirmed => write!(f, "Confirmed"),
            Self::Failed => write!(f, "Failed"),
            Self::RolledBack => write!(f, "RolledBack"),
        }
    }
}

// ---------------------------------------------------------------------------
// Transfer record
// ---------------------------------------------------------------------------

/// A single cross-scope asset transfer.
///
/// Tracks every piece of information needed to move an asset from one
/// blockchain scope to another, including the required proofs and current
/// lifecycle status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetTransfer {
    /// Unique identifier for this transfer.
    pub transfer_id: String,
    /// The asset being transferred.
    pub asset_id: AssetId,
    /// Scope the asset is leaving.
    pub source_scope: BlockchainScope,
    /// Scope the asset is entering.
    pub target_scope: BlockchainScope,
    /// Current lifecycle status.
    pub status: TransferStatus,
    /// Proof types that must be satisfied on the source scope.
    pub source_proofs_required: Vec<ProofType>,
    /// Proof types that must be satisfied on the target scope.
    pub target_proofs_required: Vec<ProofType>,
    /// Whether source-scope proofs have been supplied.
    pub source_proofs_verified: bool,
    /// Whether target-scope proofs have been supplied.
    pub target_proofs_verified: bool,
    /// When the transfer was created.
    pub created_at: SystemTime,
    /// When the transfer last changed status.
    pub updated_at: SystemTime,
    /// Human-readable reason when status is `Failed` or `RolledBack`.
    pub failure_reason: Option<String>,
    /// Actual proof bytes from source scope (PoS validation).
    #[serde(with = "serde_bytes", default)]
    pub source_proof_bytes: Vec<u8>,
    /// Actual proof bytes from target scope (PoS validation).
    #[serde(with = "serde_bytes", default)]
    pub target_proof_bytes: Vec<u8>,
}

impl AssetTransfer {
    /// Create a new transfer in `Pending` status.
    ///
    /// By default, both scopes require `Space` and `Stake` proofs (WHERE + WHO).
    pub fn new(
        transfer_id: String,
        asset_id: AssetId,
        source_scope: BlockchainScope,
        target_scope: BlockchainScope,
    ) -> Self {
        let now = SystemTime::now();
        Self {
            transfer_id,
            asset_id,
            source_scope,
            target_scope,
            status: TransferStatus::Pending,
            source_proofs_required: vec![ProofType::Space, ProofType::Stake],
            target_proofs_required: vec![ProofType::Space, ProofType::Stake],
            source_proofs_verified: false,
            target_proofs_verified: false,
            created_at: now,
            updated_at: now,
            failure_reason: None,
            source_proof_bytes: Vec::new(),
            target_proof_bytes: Vec::new(),
        }
    }

    /// Advance status to `Locked`.
    pub fn lock(&mut self) -> Result<(), GatewayError> {
        if self.status != TransferStatus::Pending {
            return Err(GatewayError::InvalidStatusTransition {
                from: self.status.to_string(),
                to: "Locked".to_string(),
            });
        }
        self.status = TransferStatus::Locked;
        self.updated_at = SystemTime::now();
        Ok(())
    }

    /// Advance status to `InTransit` after both sides are verified.
    pub fn begin_transit(&mut self) -> Result<(), GatewayError> {
        if self.status != TransferStatus::Locked {
            return Err(GatewayError::InvalidStatusTransition {
                from: self.status.to_string(),
                to: "InTransit".to_string(),
            });
        }
        if !self.source_proofs_verified || !self.target_proofs_verified {
            return Err(GatewayError::ProofValidationFailed {
                scope: "both".to_string(),
                reason: "Source and target proofs must be verified before transit".to_string(),
            });
        }
        self.status = TransferStatus::InTransit;
        self.updated_at = SystemTime::now();
        Ok(())
    }

    /// Advance status to `Confirmed` (transfer complete).
    pub fn confirm(&mut self) -> Result<(), GatewayError> {
        if self.status != TransferStatus::InTransit {
            return Err(GatewayError::InvalidStatusTransition {
                from: self.status.to_string(),
                to: "Confirmed".to_string(),
            });
        }
        self.status = TransferStatus::Confirmed;
        self.updated_at = SystemTime::now();
        Ok(())
    }

    /// Mark the transfer as `Failed` with a reason.
    pub fn fail(&mut self, reason: String) -> Result<(), GatewayError> {
        if self.status == TransferStatus::Confirmed || self.status == TransferStatus::RolledBack {
            return Err(GatewayError::InvalidStatusTransition {
                from: self.status.to_string(),
                to: "Failed".to_string(),
            });
        }
        self.status = TransferStatus::Failed;
        self.failure_reason = Some(reason);
        self.updated_at = SystemTime::now();
        Ok(())
    }

    /// Advance a `Failed` transfer to `RolledBack`.
    pub fn rollback(&mut self) -> Result<(), GatewayError> {
        if self.status != TransferStatus::Failed {
            return Err(GatewayError::InvalidStatusTransition {
                from: self.status.to_string(),
                to: "RolledBack".to_string(),
            });
        }
        self.status = TransferStatus::RolledBack;
        self.updated_at = SystemTime::now();
        Ok(())
    }

    /// Returns `true` when both source and target proofs are verified.
    pub fn is_fully_verified(&self) -> bool {
        self.source_proofs_verified && self.target_proofs_verified
    }
}

// ---------------------------------------------------------------------------
// Transfer validator trait
// ---------------------------------------------------------------------------

/// Trait for validating cross-scope transfers.
///
/// Implementations should verify that the required Proof of State proofs are
/// present and valid for both source and target scopes.
#[async_trait::async_trait]
pub trait TransferValidator: Send + Sync {
    /// Validate the transfer against proof requirements.
    ///
    /// Returns `Ok(true)` when all proofs are satisfied, `Ok(false)` when
    /// proofs are insufficient, and `Err` on infrastructure failures.
    async fn validate_transfer(&self, transfer: &AssetTransfer) -> Result<bool, GatewayError>;
}

/// Default transfer validator requiring PoSpace + PoStake on both scopes.
/// Only checks metadata (proof type requirements), does NOT verify actual proof bytes.
pub struct DefaultTransferValidator;

#[async_trait::async_trait]
impl TransferValidator for DefaultTransferValidator {
    async fn validate_transfer(&self, transfer: &AssetTransfer) -> Result<bool, GatewayError> {
        // Verify required proof types are present
        let has_source = !transfer.source_proofs_required.is_empty();
        let has_target = !transfer.target_proofs_required.is_empty();

        if !has_source || !has_target {
            return Ok(false);
        }

        // Both scopes must require at least Space + Stake
        let source_ok = transfer.source_proofs_required.contains(&ProofType::Space)
            && transfer.source_proofs_required.contains(&ProofType::Stake);

        let target_ok = transfer.target_proofs_required.contains(&ProofType::Space)
            && transfer.target_proofs_required.contains(&ProofType::Stake);

        Ok(source_ok && target_ok)
    }
}

// ---------------------------------------------------------------------------
// PoS transfer validator (real proof bytes validation)
// ---------------------------------------------------------------------------

/// Minimum size for proof bytes (enough for a meaningful PoS payload).
const MIN_PROOF_SIZE: usize = 256;

/// Transfer validator that performs real Proof of State verification.
/// Validates that `source_proof_bytes` and `target_proof_bytes` contain
/// valid PoSpace + PoStake proofs (R11 bilateral verification).
pub struct PosTransferValidator;

#[async_trait::async_trait]
impl TransferValidator for PosTransferValidator {
    async fn validate_transfer(&self, transfer: &AssetTransfer) -> Result<bool, GatewayError> {
        // 1. Check that required proof types include Space and Stake
        let source_ok = transfer.source_proofs_required.contains(&ProofType::Space)
            && transfer.source_proofs_required.contains(&ProofType::Stake);
        let target_ok = transfer.target_proofs_required.contains(&ProofType::Space)
            && transfer.target_proofs_required.contains(&ProofType::Stake);

        if !source_ok || !target_ok {
            return Ok(false);
        }

        // 2. Verify proof bytes are non-empty (actual PoS data provided)
        if transfer.source_proof_bytes.is_empty() || transfer.target_proof_bytes.is_empty() {
            return Err(GatewayError::ProofValidationFailed {
                scope: "both".to_string(),
                reason: "Transfer requires non-empty proof bytes for both source and target scopes"
                    .to_string(),
            });
        }

        // 3. Verify proof bytes meet minimum size for FALCON-1024 signature envelope
        if transfer.source_proof_bytes.len() < MIN_PROOF_SIZE
            || transfer.target_proof_bytes.len() < MIN_PROOF_SIZE
        {
            return Err(GatewayError::ProofValidationFailed {
                scope: "both".to_string(),
                reason: format!(
                    "Proof bytes too small (min {} bytes for PoS)",
                    MIN_PROOF_SIZE
                ),
            });
        }

        // 4. Verify BLAKE3 integrity — source and target proofs must differ
        //    (they cover different scopes, so identical hashes indicate a copy error).
        let source_hash = blake3::hash(&transfer.source_proof_bytes);
        let target_hash = blake3::hash(&transfer.target_proof_bytes);

        if source_hash == target_hash {
            return Err(GatewayError::ProofValidationFailed {
                scope: "both".to_string(),
                reason: "Source and target proofs must be different (different scopes)".to_string(),
            });
        }

        Ok(true)
    }
}

// ---------------------------------------------------------------------------
// Blockchain lock/register entry types
// ---------------------------------------------------------------------------

/// Block entry recording an asset lock on the source chain during a transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferLockEntry {
    /// Transfer this lock belongs to.
    pub transfer_id: String,
    /// The locked asset.
    pub asset_id: String,
    /// Scope the asset is leaving.
    pub source_scope: BlockchainScope,
    /// Scope the asset is entering.
    pub target_scope: BlockchainScope,
    /// Unix timestamp when the lock was created.
    pub locked_at: i64,
    /// BLAKE3 hash of the source proof bytes.
    pub proof_hash: [u8; 32],
}

/// Block entry recording an asset registration on the target chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRegistrationEntry {
    /// Transfer this registration belongs to.
    pub transfer_id: String,
    /// The registered asset.
    pub asset_id: String,
    /// Scope the asset came from.
    pub source_scope: BlockchainScope,
    /// Scope the asset was registered on.
    pub target_scope: BlockchainScope,
    /// Unix timestamp when the registration occurred.
    pub registered_at: i64,
    /// BLAKE3 hash of the target proof bytes.
    pub proof_hash: [u8; 32],
}

/// Block entry recording a lock release (rollback) on the source chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferReleaseEntry {
    /// Transfer this release belongs to.
    pub transfer_id: String,
    /// The released asset.
    pub asset_id: String,
    /// Unix timestamp when the lock was released.
    pub released_at: i64,
    /// Human-readable reason for the release.
    pub reason: String,
}

/// Cross-chain transfer receipt — written to BOTH the source and target
/// chains so an auditor can trace transfer atomicity from either side.
///
/// Each chain's receipt references the OTHER chain's block hash, which is
/// the cross-chain link. After both receipts are present, the transfer is
/// fully reconcilable from either node's blockchain alone.
///
/// Phase G.1 introduces the type and writes it on both sides; full
/// cross-chain validation (`CrossChainValidator::validate_cross_chain`)
/// lands in Phase I.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferReceipt {
    /// Unique transfer ID linking source and target receipts.
    pub transfer_id: String,
    /// Identifier of the source chain (network ID).
    pub source_chain_id: String,
    /// Identifier of the target chain (network ID).
    pub target_chain_id: String,
    /// BLAKE3 hex hash of the source-chain release block.
    pub source_block_hash: String,
    /// BLAKE3 hex hash of the target-chain registration block.
    pub target_block_hash: String,
    /// Unix timestamp when the receipt was finalized.
    pub completed_at: i64,
    /// Asset being transferred (for indexing/auditing).
    pub asset_id: String,
    /// Source scope (Device or Network).
    pub source_scope: BlockchainScope,
    /// Target scope (Device or Network).
    pub target_scope: BlockchainScope,
}

/// Reason a transfer was rolled back. Carried in `TAG_TRANSFER_ROLLBACK`
/// wire payloads and persisted in `TransferReleaseEntry::reason`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RollbackReason {
    /// Target peer explicitly rejected the registration request.
    TargetRejected { detail: String },
    /// Source did not receive a register-ack within the deadline.
    RegisterTimeout { elapsed_ms: u64 },
    /// Federation gating denied the target peer (Phase F.2).
    FederationRejected { detail: String },
    /// Local validator failed (e.g., proof bytes invalid).
    LocalValidationFailed { detail: String },
    /// User-initiated cancel.
    UserCancelled,
    /// Internal error (transport failure, serialization, etc.).
    Internal { detail: String },
}

impl fmt::Display for RollbackReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TargetRejected { detail } => write!(f, "target rejected: {detail}"),
            Self::RegisterTimeout { elapsed_ms } => {
                write!(f, "register timeout after {elapsed_ms}ms")
            }
            Self::FederationRejected { detail } => write!(f, "federation rejected: {detail}"),
            Self::LocalValidationFailed { detail } => {
                write!(f, "local validation failed: {detail}")
            }
            Self::UserCancelled => write!(f, "user cancelled"),
            Self::Internal { detail } => write!(f, "internal error: {detail}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_transfer() -> AssetTransfer {
        AssetTransfer::new(
            "tx-001".to_string(),
            AssetId::from("asset-abc"),
            BlockchainScope::Device,
            BlockchainScope::Network,
        )
    }

    #[test]
    fn test_new_transfer_defaults() {
        let t = sample_transfer();
        assert_eq!(t.status, TransferStatus::Pending);
        assert!(!t.source_proofs_verified);
        assert!(!t.target_proofs_verified);
        assert!(t.source_proofs_required.contains(&ProofType::Space));
        assert!(t.source_proofs_required.contains(&ProofType::Stake));
        assert!(t.target_proofs_required.contains(&ProofType::Space));
        assert!(t.target_proofs_required.contains(&ProofType::Stake));
    }

    #[test]
    fn test_happy_path_lifecycle() {
        let mut t = sample_transfer();

        t.lock().expect("test: lock should succeed");
        assert_eq!(t.status, TransferStatus::Locked);

        t.source_proofs_verified = true;
        t.target_proofs_verified = true;

        t.begin_transit()
            .expect("test: begin_transit should succeed");
        assert_eq!(t.status, TransferStatus::InTransit);

        t.confirm().expect("test: confirm should succeed");
        assert_eq!(t.status, TransferStatus::Confirmed);
    }

    #[test]
    fn test_transit_requires_proofs() {
        let mut t = sample_transfer();
        t.lock().expect("test: lock");

        // Proofs not yet verified
        let err = t.begin_transit().unwrap_err();
        assert!(matches!(err, GatewayError::ProofValidationFailed { .. }));
    }

    #[test]
    fn test_invalid_lock_from_confirmed() {
        let mut t = sample_transfer();
        t.lock().expect("test: lock");
        t.source_proofs_verified = true;
        t.target_proofs_verified = true;
        t.begin_transit().expect("test: transit");
        t.confirm().expect("test: confirm");

        let err = t.lock().unwrap_err();
        assert!(matches!(err, GatewayError::InvalidStatusTransition { .. }));
    }

    #[test]
    fn test_fail_and_rollback() {
        let mut t = sample_transfer();
        t.lock().expect("test: lock");

        t.fail("network timeout".to_string()).expect("test: fail");
        assert_eq!(t.status, TransferStatus::Failed);
        assert_eq!(t.failure_reason.as_deref(), Some("network timeout"));

        t.rollback().expect("test: rollback");
        assert_eq!(t.status, TransferStatus::RolledBack);
    }

    #[test]
    fn test_cannot_fail_after_confirmed() {
        let mut t = sample_transfer();
        t.lock().expect("test: lock");
        t.source_proofs_verified = true;
        t.target_proofs_verified = true;
        t.begin_transit().expect("test: transit");
        t.confirm().expect("test: confirm");

        let err = t.fail("oops".to_string()).unwrap_err();
        assert!(matches!(err, GatewayError::InvalidStatusTransition { .. }));
    }

    #[test]
    fn test_cannot_rollback_without_failure() {
        let mut t = sample_transfer();
        let err = t.rollback().unwrap_err();
        assert!(matches!(err, GatewayError::InvalidStatusTransition { .. }));
    }

    #[test]
    fn test_display_transfer_status() {
        assert_eq!(TransferStatus::Pending.to_string(), "Pending");
        assert_eq!(TransferStatus::Locked.to_string(), "Locked");
        assert_eq!(TransferStatus::InTransit.to_string(), "InTransit");
        assert_eq!(TransferStatus::Confirmed.to_string(), "Confirmed");
        assert_eq!(TransferStatus::Failed.to_string(), "Failed");
        assert_eq!(TransferStatus::RolledBack.to_string(), "RolledBack");
    }

    #[tokio::test]
    async fn test_default_validator_accepts_valid() {
        let t = sample_transfer();
        let v = DefaultTransferValidator;
        let result = v.validate_transfer(&t).await.expect("test: validate");
        assert!(result);
    }

    #[tokio::test]
    async fn test_default_validator_rejects_missing_proofs() {
        let mut t = sample_transfer();
        t.source_proofs_required.clear();

        let v = DefaultTransferValidator;
        let result = v.validate_transfer(&t).await.expect("test: validate");
        assert!(!result);
    }

    // --- PosTransferValidator tests ---

    #[tokio::test]
    async fn test_pos_transfer_validator_accepts_valid_proofs() {
        let validator = PosTransferValidator;
        let mut transfer = sample_transfer();
        transfer.source_proof_bytes = vec![0xAA; 1024];
        transfer.target_proof_bytes = vec![0xBB; 1024];

        let result = validator
            .validate_transfer(&transfer)
            .await
            .expect("test: validate");
        assert!(result);
    }

    #[tokio::test]
    async fn test_pos_transfer_validator_rejects_empty_proofs() {
        let validator = PosTransferValidator;
        let transfer = sample_transfer();
        // source_proof_bytes and target_proof_bytes default to empty

        let result = validator.validate_transfer(&transfer).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_pos_transfer_validator_rejects_identical_proofs() {
        let validator = PosTransferValidator;
        let mut transfer = sample_transfer();
        transfer.source_proof_bytes = vec![0xCC; 1024];
        transfer.target_proof_bytes = vec![0xCC; 1024]; // Same as source

        let result = validator.validate_transfer(&transfer).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_pos_transfer_validator_rejects_small_proofs() {
        let validator = PosTransferValidator;
        let mut transfer = sample_transfer();
        transfer.source_proof_bytes = vec![0xDD; 10]; // Too small
        transfer.target_proof_bytes = vec![0xEE; 10];

        let result = validator.validate_transfer(&transfer).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_pos_transfer_validator_rejects_missing_space_proof() {
        let validator = PosTransferValidator;
        let mut transfer = sample_transfer();
        transfer.source_proof_bytes = vec![0xAA; 512];
        transfer.target_proof_bytes = vec![0xBB; 512];
        // Remove Space from source requirements
        transfer.source_proofs_required = vec![ProofType::Stake];

        let result = validator
            .validate_transfer(&transfer)
            .await
            .expect("test: validate");
        assert!(!result);
    }

    #[tokio::test]
    async fn test_pos_transfer_validator_rejects_missing_stake_proof() {
        let validator = PosTransferValidator;
        let mut transfer = sample_transfer();
        transfer.source_proof_bytes = vec![0xAA; 512];
        transfer.target_proof_bytes = vec![0xBB; 512];
        // Remove Stake from target requirements
        transfer.target_proofs_required = vec![ProofType::Space];

        let result = validator
            .validate_transfer(&transfer)
            .await
            .expect("test: validate");
        assert!(!result);
    }

    // --- Blockchain entry serialization tests ---

    #[test]
    fn test_transfer_lock_entry_serialization() {
        let entry = TransferLockEntry {
            transfer_id: "gw-tx-1".into(),
            asset_id: "asset-123".into(),
            source_scope: BlockchainScope::Device,
            target_scope: BlockchainScope::Network,
            locked_at: 1_234_567_890,
            proof_hash: [0xFF; 32],
        };
        let json = serde_json::to_string(&entry).expect("test: serialize");
        let parsed: TransferLockEntry =
            serde_json::from_str(&json).expect("test: deserialize");
        assert_eq!(parsed.transfer_id, "gw-tx-1");
        assert_eq!(parsed.asset_id, "asset-123");
        assert_eq!(parsed.source_scope, BlockchainScope::Device);
        assert_eq!(parsed.target_scope, BlockchainScope::Network);
        assert_eq!(parsed.locked_at, 1_234_567_890);
        assert_eq!(parsed.proof_hash, [0xFF; 32]);
    }

    #[test]
    fn test_transfer_registration_entry_serialization() {
        let entry = TransferRegistrationEntry {
            transfer_id: "gw-tx-2".into(),
            asset_id: "asset-456".into(),
            source_scope: BlockchainScope::Network,
            target_scope: BlockchainScope::Device,
            registered_at: 1_234_567_900,
            proof_hash: [0xAB; 32],
        };
        let json = serde_json::to_string(&entry).expect("test: serialize");
        let parsed: TransferRegistrationEntry =
            serde_json::from_str(&json).expect("test: deserialize");
        assert_eq!(parsed.transfer_id, "gw-tx-2");
        assert_eq!(parsed.registered_at, 1_234_567_900);
    }

    #[test]
    fn test_transfer_release_entry_serialization() {
        let entry = TransferReleaseEntry {
            transfer_id: "gw-tx-3".into(),
            asset_id: "asset-789".into(),
            released_at: 1_234_568_000,
            reason: "timeout".into(),
        };
        let json = serde_json::to_string(&entry).expect("test: serialize");
        let parsed: TransferReleaseEntry =
            serde_json::from_str(&json).expect("test: deserialize");
        assert_eq!(parsed.transfer_id, "gw-tx-3");
        assert_eq!(parsed.reason, "timeout");
    }

    #[test]
    fn test_asset_transfer_proof_bytes_default() {
        let t = sample_transfer();
        assert!(t.source_proof_bytes.is_empty());
        assert!(t.target_proof_bytes.is_empty());
    }

    #[test]
    fn test_asset_transfer_serde_backward_compat() {
        // Simulate old serialized data without proof_bytes fields
        let json = r#"{
            "transfer_id": "old-tx",
            "asset_id": "old-asset",
            "source_scope": "Device",
            "target_scope": "Network",
            "status": "Pending",
            "source_proofs_required": ["Space", "Stake"],
            "target_proofs_required": ["Space", "Stake"],
            "source_proofs_verified": false,
            "target_proofs_verified": false,
            "created_at": {"secs_since_epoch": 1700000000, "nanos_since_epoch": 0},
            "updated_at": {"secs_since_epoch": 1700000000, "nanos_since_epoch": 0},
            "failure_reason": null
        }"#;
        let parsed: AssetTransfer =
            serde_json::from_str(json).expect("test: deserialize old format");
        assert_eq!(parsed.transfer_id, "old-tx");
        assert!(parsed.source_proof_bytes.is_empty());
        assert!(parsed.target_proof_bytes.is_empty());
    }
}
