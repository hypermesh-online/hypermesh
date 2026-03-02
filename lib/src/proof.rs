// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Canonical Proof of State types
//!
//! These types define the four-proof Proof of State system used across all
//! HyperMesh crates. Validation logic lives in TrustChain; these are the
//! shared data structures.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

use crate::types::{AssetId, ContentHash, MatrixPosition, NodeId, ProofType};

// ---------------------------------------------------------------------------
// SpaceProof — WHERE
// ---------------------------------------------------------------------------

/// Proof of Space: storage location and physical/network position
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpaceProof {
    /// Node providing storage
    pub node_id: NodeId,
    /// Position in the Block-MATRIX topology
    pub matrix_position: MatrixPosition,
    /// Bytes currently stored
    pub stored_bytes: u64,
    /// Bytes committed (capacity reserved)
    pub committed_bytes: u64,
    /// Hash of the stored content
    pub content_hash: ContentHash,
    /// UTC millisecond timestamp
    pub timestamp_ms: i64,
}

impl fmt::Display for SpaceProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SpaceProof(node={}, stored={}B, committed={}B)",
            self.node_id, self.stored_bytes, self.committed_bytes,
        )
    }
}

// ---------------------------------------------------------------------------
// StakeProof — WHO
// ---------------------------------------------------------------------------

/// Proof of Stake: ownership, access rights, and economic stake
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StakeProof {
    /// Node holding the stake
    pub node_id: NodeId,
    /// Optional asset being staked against
    pub asset_id: Option<AssetId>,
    /// Stake amount (smallest unit)
    pub stake_amount: u64,
    /// Opaque signature — crypto lives in TrustChain
    pub signature: Vec<u8>,
    /// UTC millisecond timestamp
    pub timestamp_ms: i64,
}

impl fmt::Display for StakeProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "StakeProof(node={}, stake={}, sig={}B)",
            self.node_id,
            self.stake_amount,
            self.signature.len(),
        )
    }
}

// ---------------------------------------------------------------------------
// WorkProof — WHAT/HOW
// ---------------------------------------------------------------------------

/// Classification of computational work performed
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkCategory {
    /// CPU/GPU computation
    Compute,
    /// Routing, transport, relay
    Network,
    /// Read/write, replication
    Storage,
    /// Signing, verification, key exchange
    Cryptographic,
    /// Proof checking, consensus participation
    Validation,
}

impl fmt::Display for WorkCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkCategory::Compute => write!(f, "Compute"),
            WorkCategory::Network => write!(f, "Network"),
            WorkCategory::Storage => write!(f, "Storage"),
            WorkCategory::Cryptographic => write!(f, "Cryptographic"),
            WorkCategory::Validation => write!(f, "Validation"),
        }
    }
}

/// Proof of Work: computational resources and processing
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkProof {
    /// Node performing the work
    pub node_id: NodeId,
    /// Compute units expended
    pub compute_units: u64,
    /// Category of work performed
    pub work_category: WorkCategory,
    /// Opaque challenge/response proof — crypto lives in TrustChain
    pub challenge_proof: Vec<u8>,
    /// UTC millisecond timestamp
    pub timestamp_ms: i64,
}

impl fmt::Display for WorkProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "WorkProof(node={}, units={}, category={})",
            self.node_id, self.compute_units, self.work_category,
        )
    }
}

// ---------------------------------------------------------------------------
// TimeProof — WHEN
// ---------------------------------------------------------------------------

/// Proof of Time: temporal ordering and timestamp validation
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimeProof {
    /// Offset from reference time
    pub time_offset: Duration,
    /// Random nonce for uniqueness
    pub nonce: u64,
    /// Opaque proof hash — crypto lives in TrustChain
    pub proof_hash: Vec<u8>,
    /// UTC millisecond timestamp
    pub timestamp_ms: i64,
}

impl fmt::Display for TimeProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TimeProof(offset={:?}, nonce={})",
            self.time_offset, self.nonce,
        )
    }
}

// ---------------------------------------------------------------------------
// ProofOfState — composite of all four proofs
// ---------------------------------------------------------------------------

/// Complete Proof of State: WHERE + WHO + WHAT/HOW + WHEN
///
/// Every asset and block in HyperMesh requires all four proofs.
/// This is the canonical composite type used across all crates.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProofOfState {
    /// WHERE — storage and location proof
    pub space: SpaceProof,
    /// WHO — ownership and stake proof
    pub stake: StakeProof,
    /// WHAT/HOW — computation proof
    pub work: WorkProof,
    /// WHEN — temporal ordering proof
    pub time: TimeProof,
}

impl ProofOfState {
    /// Create a new composite proof from its four components
    pub fn new(
        space: SpaceProof,
        stake: StakeProof,
        work: WorkProof,
        time: TimeProof,
    ) -> Self {
        Self { space, stake, work, time }
    }

    /// All four proof types present in this system
    pub fn proof_types() -> &'static [ProofType] {
        &[ProofType::Space, ProofType::Stake, ProofType::Work, ProofType::Time]
    }

    /// List of proof types present (always all four for a valid ProofOfState)
    pub fn present_proofs(&self) -> Vec<ProofType> {
        Self::proof_types().to_vec()
    }
}

impl fmt::Display for ProofOfState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ProofOfState({}, {}, {}, {})",
            self.space, self.stake, self.work, self.time,
        )
    }
}

// ---------------------------------------------------------------------------
// Validatable trait — interface only, TrustChain implements
// ---------------------------------------------------------------------------

/// Validation interface for proof types
///
/// TrustChain provides the real implementations with crypto verification.
/// This trait defines the contract that all crates can program against.
pub trait Validatable {
    /// Check whether this item passes structural validation.
    ///
    /// Returns `Ok(())` on success, or a descriptive error on failure.
    fn validate(&self) -> Result<(), crate::error::HypermeshError>;
}

// ---------------------------------------------------------------------------
// ProofValidationResult
// ---------------------------------------------------------------------------

/// Per-proof validation result
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProofValidationResult {
    pub space_valid: bool,
    pub stake_valid: bool,
    pub work_valid: bool,
    pub time_valid: bool,
}

impl ProofValidationResult {
    /// True only if all four proofs are valid
    pub fn is_valid(&self) -> bool {
        self.space_valid && self.stake_valid && self.work_valid && self.time_valid
    }

    /// Confidence score: fraction of valid proofs (0.0–1.0)
    pub fn confidence(&self) -> f64 {
        let valid_count = [
            self.space_valid,
            self.stake_valid,
            self.work_valid,
            self.time_valid,
        ]
        .iter()
        .filter(|&&v| v)
        .count();
        valid_count as f64 / 4.0
    }

    /// Which proof types failed validation
    pub fn failed_proofs(&self) -> Vec<ProofType> {
        let mut failed = Vec::new();
        if !self.space_valid {
            failed.push(ProofType::Space);
        }
        if !self.stake_valid {
            failed.push(ProofType::Stake);
        }
        if !self.work_valid {
            failed.push(ProofType::Work);
        }
        if !self.time_valid {
            failed.push(ProofType::Time);
        }
        failed
    }
}

impl fmt::Display for ProofValidationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ProofValidation(valid={}, confidence={:.0}%)",
            self.is_valid(),
            self.confidence() * 100.0,
        )
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AssetId, ContentHash, MatrixPosition, NodeId};
    use std::time::Duration;

    fn sample_space() -> SpaceProof {
        SpaceProof {
            node_id: NodeId::from_public_key(b"node-alpha"),
            matrix_position: MatrixPosition { x: 1.0, y: 2.0, z: 3.0 },
            stored_bytes: 1024,
            committed_bytes: 4096,
            content_hash: ContentHash::from_bytes([0xAB; 32]),
            timestamp_ms: 1700000000000,
        }
    }

    fn sample_stake() -> StakeProof {
        StakeProof {
            node_id: NodeId::from_public_key(b"node-alpha"),
            asset_id: Some(AssetId::from("asset-001")),
            stake_amount: 500,
            signature: vec![0xDE, 0xAD],
            timestamp_ms: 1700000000000,
        }
    }

    fn sample_work() -> WorkProof {
        WorkProof {
            node_id: NodeId::from_public_key(b"node-alpha"),
            compute_units: 42,
            work_category: WorkCategory::Compute,
            challenge_proof: vec![0xCA, 0xFE],
            timestamp_ms: 1700000000000,
        }
    }

    fn sample_time() -> TimeProof {
        TimeProof {
            time_offset: Duration::from_millis(150),
            nonce: 99,
            proof_hash: vec![0xBE, 0xEF],
            timestamp_ms: 1700000000000,
        }
    }

    // --- SpaceProof ---

    #[test]
    fn space_proof_display() {
        let p = sample_space();
        let s = format!("{}", p);
        assert!(s.contains("SpaceProof"));
        assert!(s.contains("\u{2026}"), "NodeId display should contain ellipsis");
        assert!(s.contains("1024"));
    }

    #[test]
    fn space_proof_serde_roundtrip() {
        let p = sample_space();
        let json = serde_json::to_string(&p).expect("test: serialize");
        let back: SpaceProof = serde_json::from_str(&json).expect("test: deserialize");
        assert_eq!(p, back);
    }

    // --- StakeProof ---

    #[test]
    fn stake_proof_display() {
        let p = sample_stake();
        let s = format!("{}", p);
        assert!(s.contains("StakeProof"));
        assert!(s.contains("500"));
    }

    #[test]
    fn stake_proof_serde_roundtrip() {
        let p = sample_stake();
        let json = serde_json::to_string(&p).expect("test: serialize");
        let back: StakeProof = serde_json::from_str(&json).expect("test: deserialize");
        assert_eq!(p, back);
    }

    #[test]
    fn stake_proof_no_asset() {
        let mut p = sample_stake();
        p.asset_id = None;
        let json = serde_json::to_string(&p).expect("test: serialize");
        let back: StakeProof = serde_json::from_str(&json).expect("test: deserialize");
        assert_eq!(back.asset_id, None);
    }

    // --- WorkProof ---

    #[test]
    fn work_proof_display() {
        let p = sample_work();
        let s = format!("{}", p);
        assert!(s.contains("WorkProof"));
        assert!(s.contains("Compute"));
    }

    #[test]
    fn work_proof_serde_roundtrip() {
        let p = sample_work();
        let json = serde_json::to_string(&p).expect("test: serialize");
        let back: WorkProof = serde_json::from_str(&json).expect("test: deserialize");
        assert_eq!(p, back);
    }

    #[test]
    fn work_category_all_variants() {
        let categories = [
            WorkCategory::Compute,
            WorkCategory::Network,
            WorkCategory::Storage,
            WorkCategory::Cryptographic,
            WorkCategory::Validation,
        ];
        for cat in &categories {
            let json = serde_json::to_string(cat).expect("test: serialize");
            let back: WorkCategory = serde_json::from_str(&json).expect("test: deserialize");
            assert_eq!(*cat, back);
        }
    }

    // --- TimeProof ---

    #[test]
    fn time_proof_display() {
        let p = sample_time();
        let s = format!("{}", p);
        assert!(s.contains("TimeProof"));
        assert!(s.contains("99"));
    }

    #[test]
    fn time_proof_serde_roundtrip() {
        let p = sample_time();
        let json = serde_json::to_string(&p).expect("test: serialize");
        let back: TimeProof = serde_json::from_str(&json).expect("test: deserialize");
        assert_eq!(p, back);
    }

    // --- ProofOfState ---

    #[test]
    fn proof_of_state_new_and_display() {
        let pos = ProofOfState::new(
            sample_space(),
            sample_stake(),
            sample_work(),
            sample_time(),
        );
        let s = format!("{}", pos);
        assert!(s.contains("ProofOfState"));
    }

    #[test]
    fn proof_of_state_proof_types() {
        assert_eq!(ProofOfState::proof_types().len(), 4);
    }

    #[test]
    fn proof_of_state_present_proofs() {
        let pos = ProofOfState::new(
            sample_space(),
            sample_stake(),
            sample_work(),
            sample_time(),
        );
        assert_eq!(pos.present_proofs().len(), 4);
    }

    #[test]
    fn proof_of_state_serde_roundtrip() {
        let pos = ProofOfState::new(
            sample_space(),
            sample_stake(),
            sample_work(),
            sample_time(),
        );
        let json = serde_json::to_string(&pos).expect("test: serialize");
        let back: ProofOfState = serde_json::from_str(&json).expect("test: deserialize");
        assert_eq!(pos, back);
    }

    // --- ProofValidationResult ---

    #[test]
    fn validation_result_all_valid() {
        let r = ProofValidationResult {
            space_valid: true,
            stake_valid: true,
            work_valid: true,
            time_valid: true,
        };
        assert!(r.is_valid());
        assert_eq!(r.confidence(), 1.0);
        assert!(r.failed_proofs().is_empty());
    }

    #[test]
    fn validation_result_partial() {
        let r = ProofValidationResult {
            space_valid: true,
            stake_valid: false,
            work_valid: true,
            time_valid: false,
        };
        assert!(!r.is_valid());
        assert_eq!(r.confidence(), 0.5);
        let failed = r.failed_proofs();
        assert_eq!(failed.len(), 2);
        assert!(failed.contains(&ProofType::Stake));
        assert!(failed.contains(&ProofType::Time));
    }

    #[test]
    fn validation_result_none_valid() {
        let r = ProofValidationResult {
            space_valid: false,
            stake_valid: false,
            work_valid: false,
            time_valid: false,
        };
        assert!(!r.is_valid());
        assert_eq!(r.confidence(), 0.0);
        assert_eq!(r.failed_proofs().len(), 4);
    }

    #[test]
    fn validation_result_display() {
        let r = ProofValidationResult {
            space_valid: true,
            stake_valid: true,
            work_valid: false,
            time_valid: true,
        };
        let s = format!("{}", r);
        assert!(s.contains("75%"));
    }

    // --- WorkCategory Display ---

    #[test]
    fn work_category_display() {
        assert_eq!(format!("{}", WorkCategory::Compute), "Compute");
        assert_eq!(format!("{}", WorkCategory::Network), "Network");
        assert_eq!(format!("{}", WorkCategory::Storage), "Storage");
        assert_eq!(format!("{}", WorkCategory::Cryptographic), "Cryptographic");
        assert_eq!(format!("{}", WorkCategory::Validation), "Validation");
    }
}
