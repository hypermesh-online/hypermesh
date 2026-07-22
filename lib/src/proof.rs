// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Canonical Proof of State types (single source of truth).
//!
//! These types define the four-proof Proof of State system used across all
//! HyperMesh crates. TrustChain re-exports them and attaches the real
//! generation (hardware assessment, NTP) and FALCON-1024 signature logic;
//! STOQ / BlockMatrix consume these shapes directly without a TrustChain dep.
//!
//! CANONICAL MODEL (asset-pos-model-canonical):
//! - **PoStake = WHO / AUTHORIZATION** — a FALCON identity binding
//!   (`stake_holder` + `stake_holder_id`). There is **no** stake amount / coin
//!   magnitude. Authorization, never a quantity.
//! - **PoWork = WHAT (hash of work done)** — `work_hash: [u8; 32]` is the
//!   BLAKE3 of the work performed, NOT a resource-capacity number. There is no
//!   `computational_power`, `pid`, `workload_type`, or `work_state`.
//! - **PoSpace = WHERE** and **PoTime = WHEN** are location / temporal proofs.
//! - Capacity is a *descriptive* asset attribute (`authz::CapacityProfile`),
//!   never a proof and never a gate.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{Duration, SystemTime};

use crate::types::ProofType;

// ---------------------------------------------------------------------------
// SpaceProof — WHERE
// ---------------------------------------------------------------------------

/// Proof of Space: WHERE the asset is stored (storage commitment / location).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpaceProof {
    /// Node providing storage.
    pub node_id: String,
    /// Storage location path (IPv6 network path).
    pub storage_path: String,
    /// Bytes actually stored.
    pub total_size: u64,
    /// Total storage capacity (descriptive; not a gate).
    pub total_storage: u64,
    /// Content integrity hash.
    pub file_hash: String,
    /// When the proof was created.
    pub proof_timestamp: SystemTime,
}

impl SpaceProof {
    /// Canonical proof type discriminant.
    pub fn proof_type() -> ProofType {
        ProofType::Space
    }

    /// Construct a space proof for `node_id` at `storage_path` advertising
    /// `total_storage`.
    pub fn new(node_id: String, storage_path: String, total_storage: u64) -> Self {
        Self {
            node_id,
            storage_path,
            total_size: 0,
            total_storage,
            file_hash: String::new(),
            proof_timestamp: SystemTime::now(),
        }
    }

    /// Structural validity: non-empty node id and stored ≤ capacity.
    ///
    /// NOTE: `total_storage` is descriptive capacity — it is NOT gated against
    /// any minimum. WHERE is answered by a present, self-consistent proof.
    pub fn is_structurally_valid(&self) -> bool {
        !self.node_id.is_empty() && self.total_size <= self.total_storage
    }
}

impl fmt::Display for SpaceProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SpaceProof(node={}, stored={}B, capacity={}B)",
            self.node_id, self.total_size, self.total_storage,
        )
    }
}

impl PartialEq for SpaceProof {
    fn eq(&self, other: &Self) -> bool {
        self.node_id == other.node_id
            && self.storage_path == other.storage_path
            && self.total_size == other.total_size
            && self.total_storage == other.total_storage
            && self.file_hash == other.file_hash
    }
}

impl Default for SpaceProof {
    fn default() -> Self {
        Self::new(
            "test-node".to_string(),
            "/tmp/test".to_string(),
            1024 * 1024 * 1024,
        )
    }
}

// ---------------------------------------------------------------------------
// StakeProof — WHO / AUTHORIZATION (no magnitude)
// ---------------------------------------------------------------------------

/// Proof of Stake: WHO owns / is authorized. This is an **authorization**
/// (FALCON identity binding), NOT an economic magnitude. There is no stake
/// amount.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StakeProof {
    /// Entity owning / authorized for the asset (e.g. CA, node, service).
    pub stake_holder: String,
    /// Identity of the validating node (BLAKE3 hex of the FALCON pubkey).
    pub stake_holder_id: String,
    /// When the authorization was created.
    pub stake_timestamp: SystemTime,
}

impl StakeProof {
    /// Canonical proof type discriminant.
    pub fn proof_type() -> ProofType {
        ProofType::Stake
    }

    /// Construct an authorization proof binding `stake_holder_id` (identity) to
    /// a human-readable `stake_holder`.
    pub fn new(stake_holder: String, stake_holder_id: String) -> Self {
        Self {
            stake_holder,
            stake_holder_id,
            stake_timestamp: SystemTime::now(),
        }
    }

    /// Construct an authorization proof with an EXPLICIT timestamp.
    ///
    /// Determinism seam (S3.0/B2): the genesis path must be a pure function of
    /// its inputs, so it cannot read the wall clock. Live/runtime paths keep
    /// using [`new`](Self::new), which stamps `SystemTime::now()`.
    pub fn new_at(
        stake_holder: String,
        stake_holder_id: String,
        stake_timestamp: SystemTime,
    ) -> Self {
        Self {
            stake_holder,
            stake_holder_id,
            stake_timestamp,
        }
    }

    /// Structural validity: authorization requires a bound identity (WHO), not
    /// a magnitude. A non-empty `stake_holder_id` is the identity binding.
    pub fn is_structurally_valid(&self) -> bool {
        !self.stake_holder_id.is_empty()
    }
}

impl fmt::Display for StakeProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "StakeProof(holder={}, id={})",
            self.stake_holder, self.stake_holder_id,
        )
    }
}

impl PartialEq for StakeProof {
    fn eq(&self, other: &Self) -> bool {
        self.stake_holder == other.stake_holder
            && self.stake_holder_id == other.stake_holder_id
            && self.stake_timestamp == other.stake_timestamp
    }
}

impl Default for StakeProof {
    fn default() -> Self {
        Self::new("test".to_string(), "test-001".to_string())
    }
}

// ---------------------------------------------------------------------------
// WorkProof — WHAT (hash of work done)
// ---------------------------------------------------------------------------

/// Proof of Work: WHAT work was done, captured as the BLAKE3 hash of that
/// work. This is NOT a resource-capacity figure.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkProof {
    /// Entity that performed / requested the work.
    pub owner_id: String,
    /// Unique work identifier.
    pub workload_id: String,
    /// BLAKE3 hash of the work performed (the proof — WHAT/HOW).
    pub work_hash: [u8; 32],
    /// When the proof was created.
    pub proof_timestamp: SystemTime,
}

impl WorkProof {
    /// Canonical proof type discriminant.
    pub fn proof_type() -> ProofType {
        ProofType::Work
    }

    /// Construct a work proof binding `owner_id` and `workload_id` to the
    /// BLAKE3 `work_hash` of the work performed.
    pub fn new(owner_id: String, workload_id: String, work_hash: [u8; 32]) -> Self {
        Self {
            owner_id,
            workload_id,
            work_hash,
            proof_timestamp: SystemTime::now(),
        }
    }

    /// Construct a work proof with an EXPLICIT timestamp.
    ///
    /// Determinism seam (S3.0/B2): used by the genesis path, which must not
    /// read the wall clock. Live/runtime paths keep [`new`](Self::new).
    pub fn new_at(
        owner_id: String,
        workload_id: String,
        work_hash: [u8; 32],
        proof_timestamp: SystemTime,
    ) -> Self {
        Self {
            owner_id,
            workload_id,
            work_hash,
            proof_timestamp,
        }
    }

    /// Compute a work proof by hashing `work_bytes`.
    pub fn from_work(owner_id: String, workload_id: String, work_bytes: &[u8]) -> Self {
        Self::new(owner_id, workload_id, *blake3::hash(work_bytes).as_bytes())
    }

    /// Structural validity: WHAT requires a non-empty owner and a non-zero
    /// work hash (i.e. some work was actually hashed).
    pub fn is_structurally_valid(&self) -> bool {
        !self.owner_id.is_empty() && self.work_hash != [0u8; 32]
    }
}

impl fmt::Display for WorkProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "WorkProof(owner={}, workload={}, hash={})",
            self.owner_id,
            self.workload_id,
            hex::encode(&self.work_hash[..4]),
        )
    }
}

impl PartialEq for WorkProof {
    fn eq(&self, other: &Self) -> bool {
        self.owner_id == other.owner_id
            && self.workload_id == other.workload_id
            && self.work_hash == other.work_hash
    }
}

impl Default for WorkProof {
    fn default() -> Self {
        Self::new(
            "test-owner".to_string(),
            "test-workload".to_string(),
            [1u8; 32],
        )
    }
}

// ---------------------------------------------------------------------------
// TimeProof — WHEN
// ---------------------------------------------------------------------------

/// Proof of Time: WHEN it occurred (temporal ordering, replay prevention).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimeProof {
    /// Network time synchronization offset.
    pub network_time_offset: Duration,
    /// When the proof was created.
    pub time_verification_timestamp: SystemTime,
    /// Nonce to prevent replay.
    pub nonce: u64,
    /// Cryptographic proof hash (BLAKE3).
    pub proof_hash: Vec<u8>,
}

impl TimeProof {
    /// Canonical proof type discriminant.
    pub fn proof_type() -> ProofType {
        ProofType::Time
    }

    /// Construct a time proof for `network_time_offset`, deriving a nonce and a
    /// BLAKE3 proof hash over the offset/timestamp/nonce.
    pub fn new(network_time_offset: Duration) -> Self {
        let time_verification_timestamp = SystemTime::now();
        // Nonce derived from wall-clock nanos (no rand dep in lib).
        let nonce = time_verification_timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1)
            .max(1);

        let proof_hash = {
            let mut hasher = blake3::Hasher::new();
            hasher.update(&network_time_offset.as_micros().to_le_bytes());
            let timestamp_micros = time_verification_timestamp
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_micros())
                .unwrap_or(0);
            hasher.update(&timestamp_micros.to_le_bytes());
            hasher.update(&nonce.to_le_bytes());
            hasher.finalize().as_bytes().to_vec()
        };

        Self {
            network_time_offset,
            time_verification_timestamp,
            nonce,
            proof_hash,
        }
    }

    /// Construct a time proof with an EXPLICIT timestamp and nonce.
    ///
    /// Determinism seam (S3.0/B2). [`new`](Self::new) derives BOTH the
    /// timestamp and the nonce from the wall clock, which is exactly what a
    /// LIVE proof needs (replay freshness) and exactly what a GENESIS proof
    /// cannot have (it must be reproducible by anyone holding the same
    /// inputs). Genesis supplies its epoch and a nonce derived from the
    /// genesis inputs; nothing on the handshake / runtime path may use this.
    ///
    /// The `proof_hash` is computed over (offset, timestamp, nonce) with the
    /// same construction as [`new`](Self::new), so
    /// [`is_structurally_valid`](Self::is_structurally_valid) holds.
    pub fn new_at(
        network_time_offset: Duration,
        time_verification_timestamp: SystemTime,
        nonce: u64,
    ) -> Self {
        let proof_hash = {
            let mut hasher = blake3::Hasher::new();
            hasher.update(&network_time_offset.as_micros().to_le_bytes());
            let timestamp_micros = time_verification_timestamp
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_micros())
                .unwrap_or(0);
            hasher.update(&timestamp_micros.to_le_bytes());
            hasher.update(&nonce.to_le_bytes());
            hasher.finalize().as_bytes().to_vec()
        };

        Self {
            network_time_offset,
            time_verification_timestamp,
            nonce,
            proof_hash,
        }
    }

    /// Recompute the expected proof hash and compare — structural validity.
    pub fn is_structurally_valid(&self) -> bool {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.network_time_offset.as_micros().to_le_bytes());
        let timestamp_micros = self
            .time_verification_timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_micros())
            .unwrap_or(0);
        hasher.update(&timestamp_micros.to_le_bytes());
        hasher.update(&self.nonce.to_le_bytes());
        hasher.finalize().as_bytes().to_vec() == self.proof_hash
    }

    /// Serialize for network transmission.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.network_time_offset.as_micros().to_le_bytes());
        let timestamp_micros = self
            .time_verification_timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_micros())
            .unwrap_or(0);
        bytes.extend_from_slice(&timestamp_micros.to_le_bytes());
        bytes.extend_from_slice(&self.nonce.to_le_bytes());
        bytes.extend_from_slice(&self.proof_hash);
        bytes
    }

    /// Deserialize from network transmission.
    pub fn from_bytes(data: &[u8]) -> Result<Self, crate::error::HypermeshError> {
        if data.len() < 40 {
            return Err(crate::error::HypermeshError::Validation(
                "Invalid data length for TimeProof".to_string(),
            ));
        }
        let network_time_offset_bytes: [u8; 16] = data[0..16]
            .try_into()
            .map_err(|_| crate::error::HypermeshError::Validation("Invalid offset slice".into()))?;
        let network_time_offset =
            Duration::from_micros(u128::from_le_bytes(network_time_offset_bytes) as u64);

        let timestamp_bytes: [u8; 16] = data[16..32]
            .try_into()
            .map_err(|_| crate::error::HypermeshError::Validation("Invalid ts slice".into()))?;
        let timestamp_micros = u128::from_le_bytes(timestamp_bytes) as u64;
        let time_verification_timestamp =
            SystemTime::UNIX_EPOCH + Duration::from_micros(timestamp_micros);

        let nonce_bytes: [u8; 8] = data[32..40]
            .try_into()
            .map_err(|_| crate::error::HypermeshError::Validation("Invalid nonce slice".into()))?;
        let nonce = u64::from_le_bytes(nonce_bytes);

        let proof_hash = data[40..].to_vec();

        Ok(Self {
            network_time_offset,
            time_verification_timestamp,
            nonce,
            proof_hash,
        })
    }
}

impl fmt::Display for TimeProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TimeProof(offset={:?}, nonce={})",
            self.network_time_offset, self.nonce,
        )
    }
}

impl PartialEq for TimeProof {
    fn eq(&self, other: &Self) -> bool {
        let self_micros = self
            .time_verification_timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_micros())
            .unwrap_or(0);
        let other_micros = other
            .time_verification_timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_micros())
            .unwrap_or(0);
        self.network_time_offset == other.network_time_offset
            && self_micros == other_micros
            && self.nonce == other.nonce
            && self.proof_hash == other.proof_hash
    }
}

impl Default for TimeProof {
    fn default() -> Self {
        Self::new(Duration::from_secs(0))
    }
}

// ---------------------------------------------------------------------------
// StateProof — the single canonical composite of all four proofs
// ---------------------------------------------------------------------------

/// Complete Proof of State: WHERE + WHO + WHAT/HOW + WHEN.
///
/// Every asset and block in HyperMesh requires all four proofs. This is **the**
/// canonical composite type; there is no second composite anywhere in the
/// workspace. TrustChain re-exports it and attaches generation / crypto logic
/// via the `StateProofOps` extension trait; STOQ and BlockMatrix consume this
/// shape directly without a TrustChain dependency.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StateProof {
    /// WHO — ownership / authorization proof (identity binding, no magnitude).
    pub stake_proof: StakeProof,
    /// WHEN — temporal ordering proof.
    pub time_proof: TimeProof,
    /// WHERE — storage location proof.
    pub space_proof: SpaceProof,
    /// WHAT/HOW — hash of work done.
    pub work_proof: WorkProof,

    /// S3.2 — ASSET LINEAGE: identity of the prior entry for the SAME asset,
    /// or `None` when this is the asset's own genesis (the first time the asset
    /// appears on this chain).
    ///
    /// The value is the lowercase hex of the predecessor entry's `proof_hash`
    /// (`BLAKE3(serialize(predecessor state_proof))`). Two properties make that
    /// the right identity:
    ///
    /// 1. **It is already hash-committed.** `Block::calculate_hash` folds
    ///    `(asset_hash || proof_hash)` for every entry, so naming a predecessor
    ///    by its `proof_hash` names something the predecessor's block hash
    ///    already covers — a lineage pointer cannot reference a phantom entry
    ///    without breaking that block's hash.
    /// 2. **It addresses an ENTRY, not a block.** One block legitimately
    ///    carries entries for many assets (`register_asset_records` batches the
    ///    whole hardware assessment into one block) and may carry more than one
    ///    entry for the same asset. A block hash would be ambiguous at exactly
    ///    the granularity the asset chain needs.
    ///
    /// Because this field lives INSIDE the proof, it is covered by
    /// `proof_hash = BLAKE3(serialize(state_proof))` — which makes each entry's
    /// identity depend on its predecessor's, i.e. a genuine per-asset hash
    /// chain — and it is FALCON-signed by H3's `signed_proof` envelope. It
    /// therefore inherits authentication and hash-commitment without adding a
    /// single field to `Block` or `BlockAssetEntry`.
    ///
    /// `#[serde(default)]` keeps the JSON wire path tolerant; the persisted
    /// on-disk format is bincode (positional), so this IS a format change for
    /// already-persisted chains — see the S3.2 notes.
    #[serde(default)]
    pub prev_asset_entry: Option<String>,

    /// S3.2 — per-asset sequence number. `0` for the asset's genesis entry,
    /// incremented by exactly one for every subsequent entry of that asset.
    ///
    /// Redundant with the prev-pointer walk by design: it makes a truncated or
    /// re-rooted lineage detectable in O(1) at accept time rather than only by
    /// walking the whole history.
    #[serde(default)]
    pub asset_seq: u64,
}

impl StateProof {
    /// Create a new composite proof from its four components.
    ///
    /// Asset lineage starts at the asset's genesis (`prev = None, seq = 0`);
    /// the chain's write chokepoint stamps the real lineage when the entry is
    /// appended. This keeps genesis construction a pure, deterministic function
    /// of its inputs (S3.0/B2).
    pub fn new(
        stake_proof: StakeProof,
        time_proof: TimeProof,
        space_proof: SpaceProof,
        work_proof: WorkProof,
    ) -> Self {
        Self {
            stake_proof,
            time_proof,
            space_proof,
            work_proof,
            prev_asset_entry: None,
            asset_seq: 0,
        }
    }

    /// S3.2: does this proof claim to be the asset's FIRST entry?
    pub fn is_asset_genesis(&self) -> bool {
        self.prev_asset_entry.is_none() && self.asset_seq == 0
    }

    /// All four proof types present in this system.
    pub fn proof_types() -> &'static [ProofType] {
        &[ProofType::Space, ProofType::Stake, ProofType::Work, ProofType::Time]
    }

    /// List of proof types present (always all four for a valid `StateProof`).
    pub fn present_proofs(&self) -> Vec<ProofType> {
        Self::proof_types().to_vec()
    }

    /// Structural validity of all four proofs (binary pass/fail).
    pub fn is_structurally_valid(&self) -> bool {
        self.space_proof.is_structurally_valid()
            && self.stake_proof.is_structurally_valid()
            && self.work_proof.is_structurally_valid()
            && self.time_proof.is_structurally_valid()
    }

    /// Binary structural validation. Alias of [`Self::is_structurally_valid`]
    /// retained as the canonical spelling used across the workspace.
    pub fn validate(&self) -> bool {
        self.is_structurally_valid()
    }

    /// Validate against network requirements.
    ///
    /// CANONICAL MODEL: proofs answer WHO (authorization) / WHAT (work hash) /
    /// WHERE (location) / WHEN (time), never a magnitude. There is NO minimum
    /// stake / storage / compute gate — the only quantitative bound is the
    /// temporal freshness of the WHEN proof.
    pub fn validate_with_requirements(&self, requirements: &StateRequirements) -> bool {
        // WHO must be authorized (identity bound). No amount check.
        if self.stake_proof.stake_holder_id.is_empty() {
            return false;
        }
        // WHEN: proof must be temporally fresh (a time bound, not a magnitude).
        if self.time_proof.network_time_offset > requirements.max_time_offset {
            return false;
        }
        self.validate()
    }

    /// Serialize for network transmission.
    pub fn to_bytes(&self) -> Result<Vec<u8>, crate::error::HypermeshError> {
        bincode::serialize(self).map_err(|e| {
            crate::error::HypermeshError::StateProof(format!(
                "Failed to serialize StateProof: {e}"
            ))
        })
    }

    /// Deserialize from network transmission.
    pub fn from_bytes(data: &[u8]) -> Result<Self, crate::error::HypermeshError> {
        bincode::deserialize(data).map_err(|e| {
            crate::error::HypermeshError::StateProof(format!(
                "Failed to deserialize StateProof: {e}"
            ))
        })
    }

    /// BLAKE3 hash of the canonical serialization.
    pub fn hash(&self) -> Result<[u8; 32], crate::error::HypermeshError> {
        let bytes = self.to_bytes()?;
        Ok(*blake3::hash(&bytes).as_bytes())
    }

    /// Build a structurally valid proof for tests.
    ///
    /// Only compiled for test builds or when the `test-utils` feature is on, so
    /// test proofs cannot reach a production binary.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn new_for_testing() -> Self {
        let mut space = SpaceProof::new(
            "test_node_001".to_string(),
            "test_storage_path".to_string(),
            100 * 1024 * 1024 * 1024,
        );
        space.total_size = 50 * 1024 * 1024 * 1024;
        space.file_hash =
            "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2".to_string();

        Self {
            stake_proof: StakeProof::new(
                "test_stake_holder".to_string(),
                "test_node_001".to_string(),
            ),
            time_proof: TimeProof::new(Duration::from_secs(1)),
            space_proof: space,
            work_proof: WorkProof::from_work(
                "test_owner".to_string(),
                "test_workload_001".to_string(),
                b"test-work-material",
            ),
            prev_asset_entry: None,
            asset_seq: 0,
        }
    }

    /// Alias of [`Self::new_for_testing`] retained for existing call sites.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn default_for_testing() -> Self {
        Self::new_for_testing()
    }
}

impl fmt::Display for StateProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "StateProof({}, {}, {}, {})",
            self.space_proof, self.stake_proof, self.work_proof, self.time_proof,
        )
    }
}

// ---------------------------------------------------------------------------
// StateRequirements — network validation bounds
// ---------------------------------------------------------------------------

/// Requirements a [`StateProof`] is validated against.
///
/// CANONICAL MODEL: proofs answer WHO / WHAT / WHERE / WHEN, never a magnitude.
/// There is NO minimum stake / storage / compute — the only quantitative bound
/// is the WHEN proof's freshness (`max_time_offset`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StateRequirements {
    /// Maximum tolerated clock offset for synchronization (WHEN freshness).
    pub max_time_offset: Duration,
}

impl Default for StateRequirements {
    fn default() -> Self {
        Self { max_time_offset: Duration::from_secs(60) }
    }
}

impl StateRequirements {
    /// Production bound: 30s clock freshness.
    pub fn production() -> Self {
        Self { max_time_offset: Duration::from_secs(30) }
    }

    /// Relaxed bound for localhost testing: 300s clock freshness.
    pub fn localhost_testing() -> Self {
        Self { max_time_offset: Duration::from_secs(300) }
    }
}

// ---------------------------------------------------------------------------
// WireSignedProof — canonical FALCON-signed proof envelope
// ---------------------------------------------------------------------------

/// Wire format for FALCON-signed state proofs.
///
/// Wraps serialized proof bytes with a FALCON-1024 detached signature over
/// `BLAKE3(proof_bytes || nonce)`, the signer's public key, and a
/// replay-prevention nonce. TrustChain generates and verifies these; the shape
/// lives here so STOQ / BlockMatrix can carry it without a TrustChain dep.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireSignedProof {
    /// Serialized proof (e.g. JSON `StateProof`).
    pub proof_bytes: Vec<u8>,
    /// FALCON-1024 detached signature over `BLAKE3(proof_bytes || nonce)`.
    pub signature: Vec<u8>,
    /// Signer's full FALCON-1024 public key.
    pub signer_pubkey: Vec<u8>,
    /// Random nonce to prevent replay attacks.
    pub nonce: [u8; 32],
}

impl WireSignedProof {
    /// Raw FALCON-1024 public key that signed this proof.
    pub fn signer_pubkey_bytes(&self) -> &[u8] {
        &self.signer_pubkey
    }

    /// True iff this proof was signed by `pubkey` (raw FALCON bytes).
    pub fn signer_matches(&self, pubkey: &[u8]) -> bool {
        self.signer_pubkey == pubkey
    }
}

// ---------------------------------------------------------------------------
// Validatable trait — interface only, TrustChain implements
// ---------------------------------------------------------------------------

/// Validation interface for proof types. TrustChain provides the real
/// implementations with crypto verification.
pub trait Validatable {
    /// Check whether this item passes structural validation.
    fn validate(&self) -> Result<(), crate::error::HypermeshError>;
}

// ---------------------------------------------------------------------------
// ProofValidationResult
// ---------------------------------------------------------------------------

/// Per-proof validation result.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProofValidationResult {
    pub space_valid: bool,
    pub stake_valid: bool,
    pub work_valid: bool,
    pub time_valid: bool,
}

impl ProofValidationResult {
    /// True only if all four proofs are valid.
    pub fn is_valid(&self) -> bool {
        self.space_valid && self.stake_valid && self.work_valid && self.time_valid
    }

    /// Fraction of valid proofs (0.0–1.0).
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

    /// Which proof types failed validation.
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

    fn sample_space() -> SpaceProof {
        let mut p = SpaceProof::new("node-alpha".into(), "/hypermesh/a".into(), 4096);
        p.total_size = 1024;
        p.file_hash = "abcd".into();
        p
    }

    fn sample_stake() -> StakeProof {
        StakeProof::new("holder-a".into(), "node-alpha".into())
    }

    fn sample_work() -> WorkProof {
        WorkProof::new("node-alpha".into(), "wl-1".into(), [0xAB; 32])
    }

    fn sample_time() -> TimeProof {
        TimeProof::new(Duration::from_millis(150))
    }

    // --- SpaceProof ---

    #[test]
    fn space_proof_display_and_valid() {
        let p = sample_space();
        assert!(format!("{p}").contains("SpaceProof"));
        assert!(p.is_structurally_valid());
    }

    #[test]
    fn space_proof_serde_roundtrip() {
        let p = sample_space();
        let json = serde_json::to_string(&p).expect("test: serialize");
        let back: SpaceProof = serde_json::from_str(&json).expect("test: deserialize");
        assert_eq!(p, back);
    }

    #[test]
    fn space_proof_capacity_not_gated() {
        // Tiny capacity is fine — capacity is descriptive, never a minimum.
        let p = SpaceProof::new("n".into(), "/p".into(), 1);
        assert!(p.is_structurally_valid());
    }

    // --- StakeProof (authorization, no magnitude) ---

    #[test]
    fn stake_proof_is_authorization_not_amount() {
        let p = sample_stake();
        assert!(p.is_structurally_valid(), "bound identity authorizes");
        assert!(format!("{p}").contains("node-alpha"));
    }

    #[test]
    fn stake_proof_empty_identity_invalid() {
        let p = StakeProof::new("holder".into(), String::new());
        assert!(!p.is_structurally_valid(), "no identity => not authorized");
    }

    #[test]
    fn stake_proof_serde_roundtrip() {
        let p = sample_stake();
        let json = serde_json::to_string(&p).expect("test: serialize");
        let back: StakeProof = serde_json::from_str(&json).expect("test: deserialize");
        assert_eq!(p, back);
    }

    // --- WorkProof (hash-centric) ---

    #[test]
    fn work_proof_is_hash_centric() {
        let p = WorkProof::from_work("owner".into(), "wl".into(), b"the work");
        assert_eq!(p.work_hash, *blake3::hash(b"the work").as_bytes());
        assert!(p.is_structurally_valid());
    }

    #[test]
    fn work_proof_zero_hash_invalid() {
        let p = WorkProof::new("owner".into(), "wl".into(), [0u8; 32]);
        assert!(!p.is_structurally_valid(), "no work hashed => invalid");
    }

    #[test]
    fn work_proof_serde_roundtrip() {
        let p = sample_work();
        let json = serde_json::to_string(&p).expect("test: serialize");
        let back: WorkProof = serde_json::from_str(&json).expect("test: deserialize");
        assert_eq!(p, back);
    }

    // --- TimeProof ---

    #[test]
    fn time_proof_valid_and_display() {
        let p = sample_time();
        assert!(p.is_structurally_valid());
        assert!(format!("{p}").contains("TimeProof"));
    }

    #[test]
    fn time_proof_bytes_roundtrip() {
        let p = sample_time();
        let bytes = p.to_bytes();
        let back = TimeProof::from_bytes(&bytes).expect("test: from_bytes");
        assert_eq!(p, back);
    }

    // --- StateProof ---

    #[test]
    fn state_proof_new_valid_and_types() {
        let pos = StateProof::new(sample_stake(), sample_time(), sample_space(), sample_work());
        assert!(pos.is_structurally_valid());
        assert_eq!(StateProof::proof_types().len(), 4);
        assert_eq!(pos.present_proofs().len(), 4);
        assert!(format!("{pos}").contains("StateProof"));
    }

    #[test]
    fn state_proof_serde_roundtrip() {
        let pos = StateProof::new(sample_stake(), sample_time(), sample_space(), sample_work());
        let json = serde_json::to_string(&pos).expect("test: serialize");
        let back: StateProof = serde_json::from_str(&json).expect("test: deserialize");
        assert_eq!(pos, back);
    }

    #[test]
    fn state_proof_bytes_roundtrip_and_hash() {
        let pos = StateProof::new(sample_stake(), sample_time(), sample_space(), sample_work());
        let bytes = pos.to_bytes().expect("test: to_bytes");
        let back = StateProof::from_bytes(&bytes).expect("test: from_bytes");
        assert_eq!(pos, back);
        assert_eq!(pos.hash().expect("test: hash"), back.hash().expect("test: hash"));
    }

    #[test]
    fn state_proof_requirements_reject_unbound_identity() {
        let mut pos = StateProof::new(sample_stake(), sample_time(), sample_space(), sample_work());
        let reqs = StateRequirements::default();
        assert!(pos.validate_with_requirements(&reqs));

        // WHO must be bound — authorization, not a magnitude.
        pos.stake_proof.stake_holder_id = String::new();
        assert!(!pos.validate_with_requirements(&reqs));
    }

    // --- WireSignedProof ---

    #[test]
    fn wire_signed_proof_signer_helpers() {
        let wire = WireSignedProof {
            proof_bytes: vec![1, 2, 3],
            signature: vec![9],
            signer_pubkey: vec![0xAA, 0xBB],
            nonce: [0u8; 32],
        };
        assert_eq!(wire.signer_pubkey_bytes(), &[0xAA, 0xBB]);
        assert!(wire.signer_matches(&[0xAA, 0xBB]));
        assert!(!wire.signer_matches(&[0xCC]));
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
    fn validation_result_display() {
        let r = ProofValidationResult {
            space_valid: true,
            stake_valid: true,
            work_valid: false,
            time_valid: true,
        };
        assert!(format!("{r}").contains("75%"));
    }
}
