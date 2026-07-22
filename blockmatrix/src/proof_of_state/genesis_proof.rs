// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Genesis Proof of State generation from real hardware assessment.
//!
//! Per R1: hardware assessed, not self-reported. A node MUST produce a
//! functioning local blockchain from a genesis block on boot with zero
//! network connectivity.
//!
//! Per R2: Four proofs required for every state claim. During genesis
//! (Phase 0 bootstrap), the node has no peers. The proofs reflect
//! the node's own assessed capabilities.
//!
//! Per section 8.2: "Usage IS verification. There are no proactive challenges."
//! Genesis proofs are valid because the node is proving its own assessed
//! capabilities to itself.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::matrix::coordinate::MatrixCoordinate;
use crate::os_integration::{DeviceFingerprint, OsAbstraction};
use trustchain::proof_of_state::proof::{SpaceProof, StakeProof, TimeProof, WorkProof};
use trustchain::proof_of_state::StateProof;

use crate::assets::core::authz::{
    AuthorizationSet, CapacityDimension, CapacityProfile,
};

/// Prefix marking a recoverable device-fingerprint binding inside a proof
/// string field. The continuity gate parses `PoStake.stake_holder` for this
/// prefix to recover the genesis-recorded fingerprint hex.
pub const DEVICE_BINDING_PREFIX: &str = "device_fp:";

/// The single temporal INPUT to the genesis path (S3.0/B2).
///
/// A genesis block that peers must ADOPT has to be reproducible: two parties
/// holding the same inputs must derive a byte-identical block. Before S3.0 the
/// genesis proofs read `SystemTime::now()` in three places and derived the
/// PoTime nonce from the wall clock, so genesis was unreproducible by
/// construction and adoption could never be verified.
///
/// Every clock read on the genesis path is now lifted into this ONE explicit
/// value. For a device genesis it is the moment of first boot, captured once by
/// the daemon (`GenesisEpoch::now`) and thereafter recorded on-chain. For a
/// network genesis (S3.6) it is part of the network definition, so every joiner
/// re-derives the identical block.
///
/// LIVE handshake / runtime proofs are untouched: they keep `SystemTime::now()`
/// and their freshness nonces. Determinism applies to genesis ONLY.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenesisEpoch(SystemTime);

impl GenesisEpoch {
    /// Read the wall clock ONCE, at the moment a genesis is first created.
    ///
    /// This is the only sanctioned clock read on the genesis path, and it is
    /// deliberately explicit at the call site: after this the epoch is an
    /// input like any other, recorded in the block and replayable.
    pub fn now() -> Self {
        Self(SystemTime::now())
    }

    /// Build a genesis epoch from a fixed Unix-seconds value — the form a
    /// network definition carries.
    pub fn from_unix_secs(secs: u64) -> Self {
        Self(UNIX_EPOCH + Duration::from_secs(secs))
    }

    /// The epoch as a `SystemTime` (what the proofs stamp).
    pub fn as_system_time(self) -> SystemTime {
        self.0
    }

    /// Nanoseconds since the Unix epoch — the canonical byte form folded into
    /// derived values (e.g. the deterministic PoTime nonce).
    pub fn unix_nanos(self) -> u128 {
        self.0
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    }
}

/// Number of BLAKE3 iterations the PoWork challenge answer folds over. Small
/// enough to be sub-millisecond on an R13 device, large enough that the
/// answer is a real (non-trivial) function of the fingerprint + nonce.
const WORK_CHALLENGE_ITERATIONS: u32 = 4096;

/// Hardware assessment data used to build genesis proofs.
///
/// Collected from `OsAbstraction` at boot time. Each field maps to
/// one or more of the four proof dimensions. The `device_fingerprint` is
/// the AUTHENTICATION INPUT that binds the proofs to this physical machine
/// (device-auth invariant) — it is folded into all four proofs.
pub struct HardwareAssessment {
    pub cpu_cores: u32,
    pub cpu_mhz: u32,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
    pub storage_available_bytes: u64,
    /// Node ID used inside the proofs. When a canonical device identity is
    /// available this is `BLAKE3(falcon_pubkey)`; otherwise a fingerprint- or
    /// coordinate-derived label. See `from_os_with_identity`.
    pub node_id: String,
    pub coordinate: MatrixCoordinate,
    /// Device fingerprint = BLAKE3 of machine-id/DMI/disk-serial/MAC.
    /// Authenticates the physical device; folded into all four proofs.
    pub device_fingerprint: DeviceFingerprint,
    /// Serial of the disk backing the largest mount (PoSpace binding).
    pub disk_serial: Option<String>,
}

impl HardwareAssessment {
    /// Build from OsAbstraction detections. Falls back to sensible
    /// minimums when individual detections fail.
    ///
    /// The `node_id` is used verbatim as the proof identity. Callers that
    /// have the canonical `BLAKE3(falcon_pubkey)` identity should pass it
    /// here so the collapsed node ID flows into every proof.
    pub fn from_os(
        os: &dyn OsAbstraction,
        node_id: &str,
        coordinate: MatrixCoordinate,
    ) -> Self {
        let (cpu_cores, cpu_mhz) = match os.detect_cpu() {
            Ok(cpu) => (
                cpu.cores as u32,
                cpu.frequency_mhz.unwrap_or(1000) as u32,
            ),
            Err(_) => (num_cpus::get() as u32, 1000),
        };

        let memory_bytes = os
            .detect_memory()
            .map(|m| m.total_bytes)
            .unwrap_or(4 * 1024 * 1024 * 1024); // 4GB fallback per R13

        let (storage_bytes, storage_available_bytes) = os
            .detect_storage()
            .ok()
            .and_then(|devs| {
                devs.iter()
                    .max_by_key(|d| d.total_bytes)
                    .map(|d| (d.total_bytes, d.available_bytes))
            })
            .unwrap_or((50 * 1024 * 1024 * 1024, 25 * 1024 * 1024 * 1024)); // 50GB fallback per R13

        // Device-auth: capture the fingerprint UNCONDITIONALLY (enforcement
        // is gated elsewhere, but the capture always happens).
        let device_fingerprint = os.device_fingerprint();
        let disk_serial = os.primary_disk_serial();

        Self {
            cpu_cores,
            cpu_mhz,
            memory_bytes,
            storage_bytes,
            storage_available_bytes,
            node_id: node_id.to_string(),
            coordinate,
            device_fingerprint,
            disk_serial,
        }
    }

    /// Hex of the captured device fingerprint (for logging + audit).
    pub fn fingerprint_hex(&self) -> String {
        self.device_fingerprint.hex()
    }
}

/// Generate a valid Proof of State for genesis/hardware-registration blocks.
///
/// The proof uses real hardware assessment data and answers the four canonical
/// questions — WHERE (PoSpace, location), WHO (PoStake, authorization), WHAT
/// (PoWork, work hash), WHEN (PoTime, temporal) — with present, self-consistent
/// proofs.
///
/// CANONICAL MODEL: PoStake is an AUTHORIZATION — the self-owner device identity
/// binding — NOT a magnitude. There is no stake amount and no minimum-stake
/// threshold; genesis admission requires a bound identity (WHO), not a number.
/// Hardware capacity figures are descriptive asset attributes (the resource
/// adapter's `CapacityProfile`), never a proof gate.
///
/// DETERMINISM (S3.0/B2): this function is a PURE function of `hw` (device
/// fingerprint, coordinate, node identity, capacity) and `epoch`. It reads no
/// clock and draws no randomness, so two nodes holding identical inputs produce
/// a byte-identical proof — and therefore a byte-identical genesis block.
pub fn generate_genesis_proof(hw: &HardwareAssessment, epoch: GenesisEpoch) -> StateProof {
    let stake = build_stake_proof(hw, epoch);
    let time = build_time_proof(hw, epoch);
    let space = build_space_proof(hw, epoch);
    let work = build_work_proof(hw, epoch);

    StateProof::new(stake, time, space, work)
}

/// PoStake (WHO): device-bound node identity — an AUTHORIZATION, never a
/// magnitude.
///
/// CANONICAL MODEL: PoStake answers WHO (identity binding), never "how much".
/// There is NO stake amount — the hardware capacity figure it used to carry now
/// lives in the resource adapter's [`CapacityProfile`] (see
/// [`genesis_capacity_profile`]), which is descriptive and never gated.
///
/// - `stake_holder_id` = the canonical device node ID (`BLAKE3(falcon_pubkey)`
///   when available), NOT `genesis_node_{coord}`.
/// - `stake_holder` carries the RECOVERABLE device-fingerprint binding
///   (`device_fp:<hex>`) so the continuity gate can read it back and reject a
///   copied identity on a different machine.
fn build_stake_proof(hw: &HardwareAssessment, epoch: GenesisEpoch) -> StakeProof {
    StakeProof::new_at(
        format!("{}{}", DEVICE_BINDING_PREFIX, hw.device_fingerprint.hex()),
        hw.node_id.clone(),
        epoch.as_system_time(),
    )
}

/// PoSpace (WHERE): device-bound storage commitment.
///
/// Binds `node_id` + `storage_path` + `file_hash` to the device fingerprint
/// and disk serial. The `file_hash` is a commitment over the fingerprint +
/// disk serial + storage path, so tampering with the recorded device binding
/// is detectable. `node_id` is the device node ID (not the coord string) —
/// the coordinate is a DERIVED attribute recorded in `storage_path`.
fn build_space_proof(hw: &HardwareAssessment, epoch: GenesisEpoch) -> SpaceProof {
    let storage_path = format!(
        "/hypermesh/storage/{}#cell=({},{},{})",
        hw.node_id, hw.coordinate.x, hw.coordinate.y, hw.coordinate.z
    );

    // Storage commitment bound to the physical device (fingerprint + disk).
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"hypermesh-pospace-commitment-v1");
    hasher.update(storage_path.as_bytes());
    hasher.update(&hw.storage_bytes.to_le_bytes());
    hasher.update(&hw.device_fingerprint.digest);
    hasher.update(hw.disk_serial.as_deref().unwrap_or("").as_bytes());
    let file_hash = hasher.finalize().to_hex().to_string();

    let used = hw.storage_bytes.saturating_sub(hw.storage_available_bytes);

    SpaceProof {
        node_id: hw.node_id.clone(),
        storage_path,
        total_size: used,
        total_storage: hw.storage_bytes,
        file_hash,
        // Determinism (B2): the genesis epoch, never the live clock.
        proof_timestamp: epoch.as_system_time(),
    }
}

/// PoTime (WHEN): the genesis epoch, with a DERIVED (not clock-drawn) nonce.
///
/// The offset stays a real, bounded reading — the sub-second remainder of the
/// genesis epoch — rather than a hardcoded zero, so it remains inside
/// `StateRequirements` bounds. Both the timestamp and the nonce are functions
/// of the genesis inputs only.
///
/// WHY NO FRESHNESS NONCE HERE (B2): a nonce exists to make a proof
/// unrepeatable, which is precisely what a genesis block must NOT be — an
/// adopting peer has to re-derive it. Genesis is never presented as a live
/// liveness claim, so it needs no replay window. The nonce still varies per
/// device and per epoch (it is BLAKE3 over the fingerprint, node id and epoch),
/// so distinct genesis blocks stay distinct. LIVE handshake and runtime proofs
/// continue to use `TimeProof::new`, which draws its nonce from the wall clock
/// — replay protection is untouched everywhere except genesis.
fn build_time_proof(hw: &HardwareAssessment, epoch: GenesisEpoch) -> TimeProof {
    let nanos = epoch.unix_nanos();
    let offset = Duration::from_nanos((nanos % 1_000_000_000) as u64);

    let mut hasher = blake3::Hasher::new();
    hasher.update(b"hypermesh-genesis-potime-nonce-v1");
    hasher.update(&hw.device_fingerprint.digest);
    hasher.update(hw.node_id.as_bytes());
    hasher.update(&nanos.to_le_bytes());
    let digest = hasher.finalize();
    let mut nonce_bytes = [0u8; 8];
    nonce_bytes.copy_from_slice(&digest.as_bytes()[..8]);
    let nonce = u64::from_le_bytes(nonce_bytes).max(1);

    TimeProof::new_at(offset, epoch.as_system_time(), nonce)
}

/// PoWork (WHAT): the HASH of work done — a device-bound, CPU-timed BLAKE3
/// iteration answer.
///
/// CANONICAL MODEL: PoWork is the BLAKE3 hash of the work performed (WHAT), NOT
/// a resource-capacity figure. The `work_hash` is `iterate(BLAKE3,
/// fingerprint || node_id, N)` — real CPU work bound to THIS physical device
/// (via the fingerprint), not a generic capacity number. `owner_id` is the
/// device node ID.
fn build_work_proof(hw: &HardwareAssessment, epoch: GenesisEpoch) -> WorkProof {
    WorkProof::new_at(
        hw.node_id.clone(),
        format!("genesis_{}", hw.node_id),
        device_work_hash(hw),
        epoch.as_system_time(),
    )
}

/// Descriptive hardware capacity of the node, recorded as an adapter
/// [`CapacityProfile`] rather than as any proof magnitude.
///
/// CANONICAL MODEL: capacity is a DESCRIPTIVE asset attribute, never a proof and
/// never a gate. This is where the historic `cores * cpu_mhz + memory_mb`
/// hardware figure now lives — as advertised capacity dimensions, not as PoS.
pub fn genesis_capacity_profile(hw: &HardwareAssessment) -> CapacityProfile {
    let memory_mb = hw.memory_bytes / (1024 * 1024);
    let compute_units = (hw.cpu_cores as u64) * (hw.cpu_mhz as u64);
    CapacityProfile {
        dimensions: vec![
            CapacityDimension { name: "cpu_cores".into(), total_units: hw.cpu_cores as u64 },
            CapacityDimension { name: "cpu_mhz".into(), total_units: hw.cpu_mhz as u64 },
            CapacityDimension { name: "compute_units".into(), total_units: compute_units },
            CapacityDimension { name: "memory_mb".into(), total_units: memory_mb },
            CapacityDimension {
                name: "storage_bytes".into(),
                total_units: hw.storage_bytes,
            },
        ],
    }
}

/// Genesis authorization: the node self-owns its genesis assets (distribution
/// right), with no grants yet. Owner identity is the device node ID.
pub fn genesis_authorization(hw: &HardwareAssessment) -> AuthorizationSet {
    AuthorizationSet::with_owner(hw.node_id.clone())
}

/// Compute a CPU-timed BLAKE3 iteration answer bound to the device.
///
/// The answer is `iterate(BLAKE3, fingerprint || node_id, N)`. This IS the
/// PoWork hash (WHAT): it ties the proof to the physical device (via the
/// fingerprint) and is a real function of N CPU-bound BLAKE3 iterations —
/// evidence that work was actually performed at genesis, not a capacity claim.
fn device_work_hash(hw: &HardwareAssessment) -> [u8; 32] {
    let mut acc = {
        let mut h = blake3::Hasher::new();
        h.update(b"hypermesh-powork-challenge-v1");
        h.update(&hw.device_fingerprint.digest);
        h.update(hw.node_id.as_bytes());
        *h.finalize().as_bytes()
    };
    for _ in 0..WORK_CHALLENGE_ITERATIONS {
        acc = *blake3::hash(&acc).as_bytes();
    }
    acc
}

/// Recover the device-fingerprint hex recorded in a genesis `StateProof`.
///
/// Reads `PoStake.stake_holder`, which the genesis path sets to
/// `device_fp:<hex>`. Returns `None` for legacy proofs that predate the
/// device-auth binding (so the continuity gate can distinguish "no recorded
/// fingerprint" from "mismatch").
pub fn recorded_fingerprint_hex(proof: &StateProof) -> Option<String> {
    proof
        .stake_proof
        .stake_holder
        .strip_prefix(DEVICE_BINDING_PREFIX)
        .map(|s| s.to_string())
}

/// Outcome of comparing a live device fingerprint against the one recorded
/// in a genesis block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContinuityDecision {
    /// Live fingerprint matches the genesis-recorded fingerprint.
    Match,
    /// Live fingerprint differs from genesis — a copied identity directory
    /// carried to a different physical machine. Carries the (short) recorded
    /// and live hexes for logging.
    Mismatch { recorded: String, live: String },
    /// The genesis block predates the device-auth binding (no recorded
    /// fingerprint). Continuity cannot be enforced without re-genesis.
    NoRecordedFingerprint,
}

impl ContinuityDecision {
    /// Whether this decision permits startup under the given enforcement mode.
    ///
    /// Device-auth invariant: under `require_hardware_auth`, ONLY `Match`
    /// permits startup — this is precisely what rejects a copied identity
    /// directory on a different machine. Without enforcement, all outcomes
    /// permit startup (a warning is logged by the caller) so normal dev runs
    /// keep working.
    pub fn permits_startup(&self, require_hardware_auth: bool) -> bool {
        if !require_hardware_auth {
            return true;
        }
        matches!(self, ContinuityDecision::Match)
    }
}

/// Pure device-continuity decision: compare a live fingerprint hex against the
/// fingerprint recorded in a genesis proof.
///
/// This is the testable core of the continuity gate (`verify_device_continuity`
/// in the node binary), separated from the OS read so the reject-a-copy
/// behaviour is unit-tested without needing a second physical machine or root.
///
/// A copied identity directory run on machine B produces a DIFFERENT
/// `live_hex` than the `recorded` hex written at genesis on machine A →
/// `Mismatch` → rejected under `--require-hardware-auth`.
pub fn evaluate_continuity(recorded: Option<&str>, live_hex: &str) -> ContinuityDecision {
    match recorded {
        Some(recorded_hex) if recorded_hex == live_hex => ContinuityDecision::Match,
        Some(recorded_hex) => ContinuityDecision::Mismatch {
            recorded: recorded_hex.to_string(),
            live: live_hex.to_string(),
        },
        None => ContinuityDecision::NoRecordedFingerprint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::os_integration::{DeviceFingerprint, DeviceIdentifiers};
    use trustchain::proof_of_state::StateRequirements;

    fn test_fingerprint(seed: &str) -> DeviceFingerprint {
        DeviceFingerprint::compose(DeviceIdentifiers {
            machine_id: Some(format!("machine-{seed}")),
            product_uuid: Some(format!("uuid-{seed}")),
            board_serial: Some(format!("board-{seed}")),
            product_serial: None,
            primary_disk_serial: Some(format!("disk-{seed}")),
            primary_mac: Some("aa:bb:cc:dd:ee:ff".to_string()),
        })
    }

    /// A FIXED epoch — used wherever the assertion is about determinism.
    fn fixed_epoch() -> GenesisEpoch {
        GenesisEpoch::from_unix_secs(1_780_000_000)
    }

    /// A live epoch — used wherever the assertion is about `validate()`, which
    /// rejects an authorization older than 30 days.
    fn live_epoch() -> GenesisEpoch {
        GenesisEpoch::now()
    }

    fn test_hw(cores: u32, mhz: u32, mem_gb: u64, storage_gb: u64) -> HardwareAssessment {
        HardwareAssessment {
            cpu_cores: cores,
            cpu_mhz: mhz,
            memory_bytes: mem_gb * 1024 * 1024 * 1024,
            storage_bytes: storage_gb * 1024 * 1024 * 1024,
            storage_available_bytes: (storage_gb / 2) * 1024 * 1024 * 1024,
            node_id: "test-genesis-node".to_string(),
            coordinate: MatrixCoordinate::new(1, 2, 3).expect("test: valid coord"),
            device_fingerprint: test_fingerprint("A"),
            disk_serial: Some("disk-A".to_string()),
        }
    }

    #[test]
    fn r13_minimum_device_passes_default_requirements() {
        // R13: 2-core 1GHz, 4GB RAM, 50GB storage
        let hw = test_hw(2, 1000, 4, 50);
        let proof = generate_genesis_proof(&hw, live_epoch());
        let reqs = StateRequirements::default();

        assert!(
            proof.validate_with_requirements(&reqs),
            "R13 minimum device must pass default requirements: \
             identity={} (bound?), location={} (bound?), work_hash_set={}",
            proof.stake_proof.stake_holder_id,
            proof.space_proof.storage_path,
            proof.work_proof.work_hash != [0u8; 32],
        );
    }

    #[test]
    fn high_end_device_passes_default_requirements() {
        let hw = test_hw(16, 3500, 64, 2000);
        let proof = generate_genesis_proof(&hw, live_epoch());
        let reqs = StateRequirements::default();
        assert!(proof.validate_with_requirements(&reqs));
    }

    #[test]
    fn proof_contains_correct_coordinate() {
        // The coordinate is now a DERIVED attribute recorded inside the
        // storage_path (node_id is the device node ID, not the coord string).
        let hw = test_hw(4, 2000, 8, 100);
        let proof = generate_genesis_proof(&hw, live_epoch());
        assert_eq!(proof.space_proof.node_id, "test-genesis-node");
        assert!(
            proof.space_proof.storage_path.contains("cell=(1,2,3)"),
            "coordinate must be recorded in storage_path: {}",
            proof.space_proof.storage_path
        );
    }

    #[test]
    fn stake_holder_carries_recoverable_fingerprint() {
        let hw = test_hw(4, 2000, 8, 100);
        let proof = generate_genesis_proof(&hw, live_epoch());
        let recovered =
            recorded_fingerprint_hex(&proof).expect("genesis proof must record fingerprint");
        assert_eq!(recovered, hw.device_fingerprint.hex());
        // stake_holder_id is the device node ID, not genesis_node_{coord}.
        assert_eq!(proof.stake_proof.stake_holder_id, "test-genesis-node");
    }

    #[test]
    fn different_device_produces_different_fingerprint_binding() {
        let mut hw_a = test_hw(4, 2000, 8, 100);
        hw_a.device_fingerprint = test_fingerprint("A");
        let mut hw_b = test_hw(4, 2000, 8, 100);
        hw_b.device_fingerprint = test_fingerprint("B");

        let proof_a = generate_genesis_proof(&hw_a, live_epoch());
        let proof_b = generate_genesis_proof(&hw_b, live_epoch());

        // Same node label, DIFFERENT device -> different recorded fingerprint
        // and different PoSpace commitment. This is what lets the continuity
        // gate reject a copied identity on a different machine.
        assert_ne!(
            recorded_fingerprint_hex(&proof_a),
            recorded_fingerprint_hex(&proof_b),
        );
        assert_ne!(proof_a.space_proof.file_hash, proof_b.space_proof.file_hash);
    }

    #[test]
    fn work_hash_is_populated_and_device_bound() {
        // PoWork is the HASH of work done (WHAT). The genesis work hash is the
        // BLAKE3 iteration answer bound to the device fingerprint — non-zero,
        // and DIFFERENT for a different device.
        let hw_a = test_hw(4, 2000, 8, 100);
        let proof_a = generate_genesis_proof(&hw_a, live_epoch());
        assert_ne!(
            proof_a.work_proof.work_hash, [0u8; 32],
            "work hash must be populated (real work performed)"
        );

        let mut hw_b = test_hw(4, 2000, 8, 100);
        hw_b.device_fingerprint = test_fingerprint("B");
        let proof_b = generate_genesis_proof(&hw_b, live_epoch());
        assert_ne!(
            proof_a.work_proof.work_hash, proof_b.work_proof.work_hash,
            "work hash must be device-bound (differs by fingerprint)"
        );
    }

    #[test]
    fn genesis_capacity_and_authorization_are_descriptive_and_self_owned() {
        // Hardware capacity lives in a descriptive CapacityProfile, NOT in any
        // proof; genesis assets are self-owned (distribution right).
        let hw = test_hw(2, 1000, 4, 50);
        let profile = genesis_capacity_profile(&hw);
        assert_eq!(profile.units("compute_units"), Some(2000));
        assert_eq!(profile.units("memory_mb"), Some(4096));

        let auth = genesis_authorization(&hw);
        assert!(auth.is_owner(&hw.node_id), "genesis node must self-own");
        assert!(auth.grants.is_empty(), "genesis has no grants yet");
    }

    #[test]
    fn time_proof_offset_is_real_and_bounded() {
        use trustchain::proof_of_state::proof::Proof;
        // B2: the offset is the sub-second remainder of the GENESIS EPOCH — a
        // real, bounded reading derived from an input rather than a hidden
        // clock read, and never a hardcoded zero-by-construction.
        let proof = generate_genesis_proof(&test_hw(2, 1000, 4, 50), live_epoch());
        assert!(proof.time_proof.validate());
        assert!(proof.time_proof.network_time_offset < std::time::Duration::from_secs(1));
    }

    #[test]
    fn genesis_proof_is_deterministic_for_identical_inputs() {
        // B2: the whole point — no clock read, no drawn nonce, so the same
        // inputs give the same bytes. Before S3.0 this was impossible:
        // `SystemTime::now()` appeared in three proofs and the PoTime nonce was
        // derived from the wall clock, so two calls one nanosecond apart
        // differed.
        let hw = test_hw(2, 1000, 4, 50);
        let a = generate_genesis_proof(&hw, fixed_epoch());
        let b = generate_genesis_proof(&hw, fixed_epoch());

        assert_eq!(a.stake_proof.stake_timestamp, b.stake_proof.stake_timestamp);
        assert_eq!(a.space_proof.proof_timestamp, b.space_proof.proof_timestamp);
        assert_eq!(a.work_proof.proof_timestamp, b.work_proof.proof_timestamp);
        assert_eq!(a.time_proof.nonce, b.time_proof.nonce);
        assert_eq!(a.time_proof.proof_hash, b.time_proof.proof_hash);
        assert_eq!(
            serde_json::to_vec(&a).expect("test: serialize"),
            serde_json::to_vec(&b).expect("test: serialize"),
            "genesis proof must be byte-identical for identical inputs",
        );
    }

    #[test]
    fn genesis_proof_differs_by_epoch_and_by_device() {
        let hw_a = test_hw(2, 1000, 4, 50);
        let mut hw_b = test_hw(2, 1000, 4, 50);
        hw_b.device_fingerprint = test_fingerprint("B");

        let base = generate_genesis_proof(&hw_a, fixed_epoch());
        let later = generate_genesis_proof(
            &hw_a,
            GenesisEpoch::from_unix_secs(1_780_000_001),
        );
        let other_device = generate_genesis_proof(&hw_b, fixed_epoch());

        assert_ne!(base.time_proof.nonce, later.time_proof.nonce);
        assert_ne!(base.time_proof.nonce, other_device.time_proof.nonce);
        assert_ne!(
            base.stake_proof.stake_timestamp,
            later.stake_proof.stake_timestamp,
        );
    }

    #[test]
    fn live_time_proof_still_draws_a_fresh_nonce() {
        // Guard: B2 must NOT have weakened live replay protection. Two live
        // `TimeProof::new` calls still differ.
        use trustchain::proof_of_state::proof::TimeProof;
        let a = TimeProof::new(std::time::Duration::from_millis(5));
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = TimeProof::new(std::time::Duration::from_millis(5));
        assert_ne!(a.nonce, b.nonce, "live proofs must stay unrepeatable");
    }

    #[test]
    fn proof_validates_individually() {
        use trustchain::proof_of_state::proof::Proof;

        let hw = test_hw(4, 2000, 8, 100);
        let proof = generate_genesis_proof(&hw, live_epoch());

        assert!(proof.stake_proof.validate(), "PoStake must pass");
        assert!(proof.time_proof.validate(), "PoTime must pass");
        assert!(proof.space_proof.validate(), "PoSpace must pass");
        assert!(proof.work_proof.validate(), "PoWork must pass");
    }

    #[test]
    fn stake_is_authorization_not_amount() {
        // CANONICAL MODEL: PoStake carries a bound identity (WHO), never an
        // amount. The R13 device is authorized because its identity is bound,
        // not because a magnitude clears a threshold.
        let hw = test_hw(2, 1000, 4, 50);
        let proof = generate_genesis_proof(&hw, live_epoch());
        assert!(
            !proof.stake_proof.stake_holder_id.is_empty(),
            "PoStake must carry a bound identity (authorization)"
        );
        assert_eq!(proof.stake_proof.stake_holder_id, hw.node_id);
    }

    #[test]
    fn continuity_same_device_matches() {
        // Genesis on device A; live boot on device A → Match, startup permitted
        // under both enforcement modes.
        let hw = test_hw(4, 2000, 8, 100);
        let proof = generate_genesis_proof(&hw, live_epoch());
        let recorded = recorded_fingerprint_hex(&proof);
        let live_hex = hw.device_fingerprint.hex();

        let decision = evaluate_continuity(recorded.as_deref(), &live_hex);
        assert_eq!(decision, ContinuityDecision::Match);
        assert!(decision.permits_startup(true));
        assert!(decision.permits_startup(false));
    }

    #[test]
    fn continuity_copied_identity_on_different_machine_is_rejected() {
        // Genesis recorded on device A.
        let mut hw_a = test_hw(4, 2000, 8, 100);
        hw_a.device_fingerprint = test_fingerprint("A");
        let genesis_a = generate_genesis_proof(&hw_a, live_epoch());
        let recorded = recorded_fingerprint_hex(&genesis_a);

        // The SAME genesis directory is copied to machine B, whose LIVE
        // fingerprint is different (different machine-id/DMI/disk/MAC).
        let live_b = test_fingerprint("B").hex();

        let decision = evaluate_continuity(recorded.as_deref(), &live_b);
        assert!(
            matches!(decision, ContinuityDecision::Mismatch { .. }),
            "copied identity on a different machine must be a Mismatch: {decision:?}"
        );
        // Under --require-hardware-auth the copy is REJECTED (startup denied).
        assert!(
            !decision.permits_startup(true),
            "copied identity dir must FAIL startup under --require-hardware-auth"
        );
        // Without enforcement, startup is permitted (caller logs a warning).
        assert!(decision.permits_startup(false));
    }

    #[test]
    fn continuity_legacy_genesis_without_fingerprint() {
        // A genesis predating the device-auth binding records no fingerprint.
        let live_hex = test_fingerprint("A").hex();
        let decision = evaluate_continuity(None, &live_hex);
        assert_eq!(decision, ContinuityDecision::NoRecordedFingerprint);
        // Cannot enforce continuity → denied under enforcement (re-genesis
        // required), permitted otherwise.
        assert!(!decision.permits_startup(true));
        assert!(decision.permits_startup(false));
    }
}
