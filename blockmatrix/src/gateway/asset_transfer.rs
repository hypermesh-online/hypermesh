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
}
