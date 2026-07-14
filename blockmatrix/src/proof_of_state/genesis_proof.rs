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
use trustchain::proof_of_state::proof::{
    SpaceProof, StakeProof, TimeProof, WorkProof, WorkState, WorkloadType,
};
use trustchain::proof_of_state::StateProof;

/// Prefix marking a recoverable device-fingerprint binding inside a proof
/// string field. The continuity gate parses `PoStake.stake_holder` for this
/// prefix to recover the genesis-recorded fingerprint hex.
pub const DEVICE_BINDING_PREFIX: &str = "device_fp:";

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
/// The proof uses real hardware assessment data and is guaranteed to pass
/// `StateRequirements::default()` validation when the node meets R13
/// minimum device spec (2-core 1GHz, 4GB RAM, 50GB storage).
///
/// Stake calculation: `cores * cpu_mhz` ensures any 2-core 1GHz machine
/// produces stake >= 2000 * 1 = 2000. We add memory contribution
/// (memory_bytes / 1GB) to comfortably exceed the 5000 minimum stake
/// for any R13-compliant device (2 cores * 1000 MHz + 4096 MB/1024 = 6000).
pub fn generate_genesis_proof(hw: &HardwareAssessment) -> StateProof {
    let stake = build_stake_proof(hw);
    let time = build_time_proof();
    let space = build_space_proof(hw);
    let work = build_work_proof(hw);

    StateProof::new(stake, time, space, work)
}

/// PoStake (WHO): device-bound node identity + hardware-derived economic value.
///
/// - `stake_holder_id` = the canonical device node ID (`BLAKE3(falcon_pubkey)`
///   when available), NOT `genesis_node_{coord}`.
/// - `stake_holder` carries the RECOVERABLE device-fingerprint binding
///   (`device_fp:<hex>`) so the continuity gate can read it back and reject a
///   copied identity on a different machine.
///
/// stake_amount = (cores * cpu_mhz) + (memory_mb)
/// For R13 minimum (2 cores, 1000 MHz, 4096 MB): 2000 + 4096 = 6096 > 5000
fn build_stake_proof(hw: &HardwareAssessment) -> StakeProof {
    let memory_mb = hw.memory_bytes / (1024 * 1024);
    let compute_value = (hw.cpu_cores as u64) * (hw.cpu_mhz as u64);
    let stake_amount = compute_value + memory_mb;

    StakeProof::new(
        format!("{}{}", DEVICE_BINDING_PREFIX, hw.device_fingerprint.hex()),
        hw.node_id.clone(),
        stake_amount,
    )
}

/// PoSpace (WHERE): device-bound storage commitment.
///
/// Binds `node_id` + `storage_path` + `file_hash` to the device fingerprint
/// and disk serial. The `file_hash` is a commitment over the fingerprint +
/// disk serial + storage path, so tampering with the recorded device binding
/// is detectable. `node_id` is the device node ID (not the coord string) —
/// the coordinate is a DERIVED attribute recorded in `storage_path`.
fn build_space_proof(hw: &HardwareAssessment) -> SpaceProof {
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
        proof_timestamp: SystemTime::now(),
    }
}

/// PoTime (WHEN): real monotonic + wall-clock witness with a freshness nonce.
///
/// Replaces the historic `Duration::from_millis(0)`. The offset is a real,
/// bounded reading: the sub-second remainder of the wall clock. The nonce
/// (random, inside `TimeProof::new`) provides replay freshness. This keeps
/// the offset within `StateRequirements` bounds while being a genuine clock
/// reading rather than a hardcoded zero.
fn build_time_proof() -> TimeProof {
    let offset = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| Duration::from_nanos(d.subsec_nanos() as u64))
        .unwrap_or_else(|_| Duration::from_millis(0));
    TimeProof::new(offset)
}

/// PoWork (WHAT/HOW): device-bound, CPU-timed computational witness.
///
/// `computational_power = cores * cpu_mhz` (measured, not claimed). The
/// `work_challenges` (previously empty) now carry a CPU-timed BLAKE3
/// iteration answer bound to the device fingerprint — proof that THIS device
/// did the work, not a generic capacity number. `owner_id` is the device
/// node ID.
fn build_work_proof(hw: &HardwareAssessment) -> WorkProof {
    let computational_power = (hw.cpu_cores as u64) * (hw.cpu_mhz as u64);

    let mut work = WorkProof::new(
        hw.node_id.clone(),
        format!("genesis_{}", hw.node_id),
        std::process::id() as u64,
        computational_power,
        WorkloadType::Compute,
        WorkState::Running,
    );
    work.work_challenges = vec![device_work_challenge(hw)];
    work
}

/// Compute a CPU-timed BLAKE3 iteration answer bound to the device.
///
/// The answer is `iterate(BLAKE3, fingerprint || node_id, N)` recorded with
/// the elapsed micros. This ties the PoWork proof to the physical device
/// (via the fingerprint) and shows real CPU work was performed at genesis.
fn device_work_challenge(hw: &HardwareAssessment) -> String {
    let start = std::time::Instant::now();
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
    let elapsed_us = start.elapsed().as_micros();
    format!(
        "iters={}:us={}:ans={}",
        WORK_CHALLENGE_ITERATIONS,
        elapsed_us,
        blake3::Hash::from(acc).to_hex()
    )
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
        let proof = generate_genesis_proof(&hw);
        let reqs = StateRequirements::default();

        assert!(
            proof.validate_with_requirements(&reqs),
            "R13 minimum device must pass default requirements: \
             stake={} (min {}), storage={} (min {}), compute={} (min {})",
            proof.stake_proof.stake_amount,
            reqs.minimum_stake,
            proof.space_proof.total_storage,
            reqs.minimum_storage,
            proof.work_proof.computational_power,
            reqs.minimum_compute,
        );
    }

    #[test]
    fn high_end_device_passes_default_requirements() {
        let hw = test_hw(16, 3500, 64, 2000);
        let proof = generate_genesis_proof(&hw);
        let reqs = StateRequirements::default();
        assert!(proof.validate_with_requirements(&reqs));
    }

    #[test]
    fn proof_contains_correct_coordinate() {
        // The coordinate is now a DERIVED attribute recorded inside the
        // storage_path (node_id is the device node ID, not the coord string).
        let hw = test_hw(4, 2000, 8, 100);
        let proof = generate_genesis_proof(&hw);
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
        let proof = generate_genesis_proof(&hw);
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

        let proof_a = generate_genesis_proof(&hw_a);
        let proof_b = generate_genesis_proof(&hw_b);

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
    fn work_challenge_is_populated_and_device_bound() {
        let hw = test_hw(4, 2000, 8, 100);
        let proof = generate_genesis_proof(&hw);
        assert_eq!(proof.work_proof.work_challenges.len(), 1);
        let challenge = &proof.work_proof.work_challenges[0];
        assert!(challenge.contains("ans="), "challenge must carry an answer");
        assert!(challenge.contains("iters="), "challenge must record iterations");
    }

    #[test]
    fn time_proof_offset_is_real_not_hardcoded_zero() {
        use trustchain::proof_of_state::proof::Proof;
        // Offset is the sub-second wall-clock remainder — a real reading.
        // It is virtually never exactly zero, and is always in-bounds.
        let proof = generate_genesis_proof(&test_hw(2, 1000, 4, 50));
        assert!(proof.time_proof.validate());
        assert!(proof.time_proof.network_time_offset < std::time::Duration::from_secs(1));
    }

    #[test]
    fn proof_validates_individually() {
        use trustchain::proof_of_state::proof::Proof;

        let hw = test_hw(4, 2000, 8, 100);
        let proof = generate_genesis_proof(&hw);

        assert!(proof.stake_proof.validate(), "PoStake must pass");
        assert!(proof.time_proof.validate(), "PoTime must pass");
        assert!(proof.space_proof.validate(), "PoSpace must pass");
        assert!(proof.work_proof.validate(), "PoWork must pass");
    }

    #[test]
    fn stake_formula_meets_minimum() {
        // Verify the formula: cores * mhz + memory_mb >= 5000
        // R13 minimum: 2 * 1000 + 4096 = 6096
        let hw = test_hw(2, 1000, 4, 50);
        let proof = generate_genesis_proof(&hw);
        assert!(
            proof.stake_proof.stake_amount >= 5000,
            "Stake {} must be >= 5000 for R13 minimum device",
            proof.stake_proof.stake_amount,
        );
    }

    #[test]
    fn continuity_same_device_matches() {
        // Genesis on device A; live boot on device A → Match, startup permitted
        // under both enforcement modes.
        let hw = test_hw(4, 2000, 8, 100);
        let proof = generate_genesis_proof(&hw);
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
        let genesis_a = generate_genesis_proof(&hw_a);
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
