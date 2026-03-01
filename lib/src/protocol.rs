// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Protocol requirement types (R1, R4, R13, R14)
//!
//! Types that implement specific hard requirements from `papers/HYPERMESH.md`
//! Section 3. These are shared across all crates.

use serde::{Deserialize, Serialize};
use std::fmt;

use super::types::{AssetId, ContentHash, NodeId};

// ---------------------------------------------------------------------------
// Adaptive Reed-Solomon parameters (R14)
// ---------------------------------------------------------------------------

/// Adaptive Reed-Solomon erasure coding parameters (R14).
///
/// Parameters scale with asset size: more shards for larger assets.
/// After creation, shards are immutable content-addressed units and
/// must NEVER be sub-divided (breaks BLAKE3 hash mappings).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ErasureCodingParams {
    /// Number of data shards (k). Minimum 1.
    pub data_shards: u16,
    /// Number of parity shards (n - k). Minimum 1.
    pub parity_shards: u16,
}

/// Errors from `ErasureCodingParams` construction and validation.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ErasureCodingError {
    #[error("data_shards must be >= 1, got {0}")]
    ZeroDataShards(u16),
    #[error("parity_shards must be >= 1, got {0}")]
    ZeroParityShards(u16),
    #[error("total shards ({total}) exceeds Reed-Solomon limit of 256")]
    TotalExceedsLimit { total: u32 },
}

impl ErasureCodingParams {
    /// Default parameters: 10 data + 4 parity (current standard).
    pub const DEFAULT_DATA: u16 = 10;
    pub const DEFAULT_PARITY: u16 = 4;
    /// Reed-Solomon maximum total shards.
    pub const MAX_TOTAL: u32 = 256;

    /// Create with validation.
    pub fn new(data_shards: u16, parity_shards: u16) -> Result<Self, ErasureCodingError> {
        let params = Self { data_shards, parity_shards };
        params.validate()?;
        Ok(params)
    }

    /// Validate parameters against Reed-Solomon constraints.
    pub fn validate(&self) -> Result<(), ErasureCodingError> {
        if self.data_shards == 0 {
            return Err(ErasureCodingError::ZeroDataShards(self.data_shards));
        }
        if self.parity_shards == 0 {
            return Err(ErasureCodingError::ZeroParityShards(self.parity_shards));
        }
        let total = self.data_shards as u32 + self.parity_shards as u32;
        if total > Self::MAX_TOTAL {
            return Err(ErasureCodingError::TotalExceedsLimit { total });
        }
        Ok(())
    }

    /// Total shard count (data + parity).
    pub fn total_shards(&self) -> u16 {
        self.data_shards + self.parity_shards
    }

    /// Scale parameters based on asset size.
    ///
    /// Thresholds:
    /// - < 1 MB: 4 data + 2 parity (small files)
    /// - < 100 MB: 10 data + 4 parity (default)
    /// - < 1 GB: 20 data + 8 parity (large files)
    /// - < 10 GB: 40 data + 16 parity (very large files)
    /// - >= 10 GB: 80 data + 32 parity (massive files)
    pub fn for_asset_size(bytes: u64) -> Self {
        const MB: u64 = 1_000_000;
        const GB: u64 = 1_000_000_000;

        let (data, parity) = if bytes < MB {
            (4, 2)
        } else if bytes < 100 * MB {
            (10, 4)
        } else if bytes < GB {
            (20, 8)
        } else if bytes < 10 * GB {
            (40, 16)
        } else {
            (80, 32)
        };

        Self {
            data_shards: data,
            parity_shards: parity,
        }
    }
}

impl Default for ErasureCodingParams {
    fn default() -> Self {
        Self {
            data_shards: Self::DEFAULT_DATA,
            parity_shards: Self::DEFAULT_PARITY,
        }
    }
}

impl fmt::Display for ErasureCodingParams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RS({}-of-{})",
            self.data_shards,
            self.total_shards()
        )
    }
}

// ---------------------------------------------------------------------------
// Genesis hardware capabilities (R1)
// ---------------------------------------------------------------------------

/// Hardware capability assessment for genesis block (R1).
///
/// Assessed by the system, NOT self-reported. Used to instantiate
/// genesis assets as IPv6-addressed resources with Proof of State.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareCapabilities {
    /// Number of CPU cores
    pub cpu_cores: u16,
    /// CPU clock speed in MHz
    pub cpu_clock_mhz: u32,
    /// Total RAM in bytes
    pub ram_bytes: u64,
    /// Total storage in bytes
    pub storage_bytes: u64,
    /// Network bandwidth in bits per second
    pub network_bandwidth_bps: u64,
    /// Whether a GPU is available
    pub gpu_available: bool,
    /// GPU VRAM in bytes (None if no GPU)
    pub gpu_vram_bytes: Option<u64>,
}

impl HardwareCapabilities {
    /// R13 minimum device specifications.
    pub const MIN_NETWORK_BPS: u64 = 1_000_000;       // 1 Mb/s
    pub const MIN_STORAGE_BYTES: u64 = 50_000_000_000; // 50 GB
    pub const MIN_RAM_BYTES: u64 = 4_000_000_000;      // 4 GB
    pub const MIN_CPU_CORES: u16 = 2;
    pub const MIN_CPU_CLOCK_MHZ: u32 = 1_000;          // 1 GHz

    /// Check whether this hardware meets the R13 minimum spec.
    ///
    /// Minimum: 1 Mb/s network, 50GB storage, 4GB RAM, 2-core 1GHz CPU.
    pub fn meets_minimum_spec(&self) -> bool {
        self.cpu_cores >= Self::MIN_CPU_CORES
            && self.cpu_clock_mhz >= Self::MIN_CPU_CLOCK_MHZ
            && self.ram_bytes >= Self::MIN_RAM_BYTES
            && self.storage_bytes >= Self::MIN_STORAGE_BYTES
            && self.network_bandwidth_bps >= Self::MIN_NETWORK_BPS
    }

    /// List which minimum requirements are not met.
    pub fn unmet_requirements(&self) -> Vec<&'static str> {
        let mut unmet = Vec::new();
        if self.cpu_cores < Self::MIN_CPU_CORES {
            unmet.push("cpu_cores < 2");
        }
        if self.cpu_clock_mhz < Self::MIN_CPU_CLOCK_MHZ {
            unmet.push("cpu_clock_mhz < 1000 (1 GHz)");
        }
        if self.ram_bytes < Self::MIN_RAM_BYTES {
            unmet.push("ram_bytes < 4 GB");
        }
        if self.storage_bytes < Self::MIN_STORAGE_BYTES {
            unmet.push("storage_bytes < 50 GB");
        }
        if self.network_bandwidth_bps < Self::MIN_NETWORK_BPS {
            unmet.push("network_bandwidth_bps < 1 Mb/s");
        }
        unmet
    }
}

impl fmt::Display for HardwareCapabilities {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HW({}C@{}MHz, {}MB RAM, {}GB disk, {}bps net{})",
            self.cpu_cores,
            self.cpu_clock_mhz,
            self.ram_bytes / 1_000_000,
            self.storage_bytes / 1_000_000_000,
            self.network_bandwidth_bps,
            if self.gpu_available { ", GPU" } else { "" },
        )
    }
}

// ---------------------------------------------------------------------------
// Shard commitment (R12)
// ---------------------------------------------------------------------------

/// Shard commitment hash — BLAKE3 of sorted shard placements (R12).
///
/// Used in block headers to commit to the exact set of shard locations.
/// Placements must be sorted by shard index before hashing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ShardCommitment([u8; 32]);

impl ShardCommitment {
    /// Create from raw bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Compute commitment from sorted placement data.
    pub fn compute(sorted_placement_data: &[u8]) -> Self {
        let hash = blake3::hash(sorted_placement_data);
        Self(*hash.as_bytes())
    }

    /// Get the raw bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Verify that given placement data matches this commitment.
    pub fn verify(&self, sorted_placement_data: &[u8]) -> bool {
        Self::compute(sorted_placement_data) == *self
    }
}

impl fmt::Display for ShardCommitment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ShardCommit({:02x}{:02x}{:02x}{:02x}…)",
            self.0[0], self.0[1], self.0[2], self.0[3]
        )
    }
}

// ---------------------------------------------------------------------------
// Genesis capability detail types (R1)
// ---------------------------------------------------------------------------

/// CPU capabilities assessed at genesis (R1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CpuCapabilities {
    /// Number of physical cores.
    pub core_count: u32,
    /// Base clock speed in MHz.
    pub clock_mhz: u32,
    /// Architecture identifier (e.g., "x86_64", "aarch64").
    pub architecture: String,
}

/// GPU capabilities assessed at genesis (R1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuCapabilities {
    /// GPU model identifier.
    pub model: String,
    /// VRAM in bytes.
    pub vram_bytes: u64,
    /// Compute units (CUDA cores, shader units, etc.).
    pub compute_units: u32,
}

/// Storage capabilities assessed at genesis (R1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageCapabilities {
    /// Total available storage in bytes.
    pub total_bytes: u64,
    /// Storage type.
    pub storage_type: StorageType,
    /// Estimated sequential read speed in bytes/sec.
    pub read_speed_bps: u64,
    /// Estimated sequential write speed in bytes/sec.
    pub write_speed_bps: u64,
}

/// Storage medium type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StorageType {
    Hdd,
    Ssd,
    Nvme,
    Unknown,
}

impl fmt::Display for StorageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hdd => write!(f, "HDD"),
            Self::Ssd => write!(f, "SSD"),
            Self::Nvme => write!(f, "NVMe"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Network capabilities assessed at genesis (R1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkCapabilities {
    /// Measured bandwidth in bits per second.
    pub bandwidth_bps: u64,
    /// Average latency to nearest peers in microseconds.
    pub latency_us: u64,
    /// Whether IPv6 is natively supported.
    pub ipv6_native: bool,
}

/// Memory capabilities assessed at genesis (R1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryCapabilities {
    /// Total RAM in bytes.
    pub total_bytes: u64,
    /// Available RAM in bytes at time of assessment.
    pub available_bytes: u64,
}

// ---------------------------------------------------------------------------
// Cross-crate validation helpers (R4)
// ---------------------------------------------------------------------------

/// Verify a BLAKE3 content hash matches the given data.
pub fn validate_blake3_hash(hash: &ContentHash, data: &[u8]) -> bool {
    let computed = blake3::hash(data);
    hash.0 == *computed.as_bytes()
}

/// Check NodeId format validity.
///
/// Valid: non-empty, max 128 chars, alphanumeric + hyphen/underscore/dot.
pub fn validate_node_id(id: &NodeId) -> bool {
    if id.0.is_empty() || id.0.len() > 128 {
        return false;
    }
    id.0.chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
}

/// Check AssetId format validity.
///
/// Valid: non-empty string.
pub fn validate_asset_id(id: &AssetId) -> bool {
    !id.0.is_empty()
}

/// Compute a BLAKE3 content hash from raw data.
pub fn compute_blake3_hash(data: &[u8]) -> ContentHash {
    let hash = blake3::hash(data);
    ContentHash(*hash.as_bytes())
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- ErasureCodingParams ---

    #[test]
    fn erasure_coding_default() {
        let params = ErasureCodingParams::default();
        assert_eq!(params.data_shards, 10);
        assert_eq!(params.parity_shards, 4);
        assert_eq!(params.total_shards(), 14);
        assert!(params.validate().is_ok());
    }

    #[test]
    fn erasure_coding_new_valid() {
        let params = ErasureCodingParams::new(20, 8)
            .expect("test: valid params");
        assert_eq!(params.data_shards, 20);
        assert_eq!(params.parity_shards, 8);
        assert_eq!(params.total_shards(), 28);
    }

    #[test]
    fn erasure_coding_zero_data_shards() {
        let result = ErasureCodingParams::new(0, 4);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("data_shards"));
    }

    #[test]
    fn erasure_coding_zero_parity_shards() {
        let result = ErasureCodingParams::new(10, 0);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("parity_shards"));
    }

    #[test]
    fn erasure_coding_exceeds_rs_limit() {
        let result = ErasureCodingParams::new(200, 100);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("256"));
    }

    #[test]
    fn erasure_coding_at_rs_limit() {
        let params = ErasureCodingParams::new(200, 56)
            .expect("test: at limit");
        assert_eq!(params.total_shards(), 256);
    }

    #[test]
    fn erasure_coding_for_asset_size() {
        let small = ErasureCodingParams::for_asset_size(500_000);
        assert_eq!(small.data_shards, 4);
        assert_eq!(small.parity_shards, 2);

        let medium = ErasureCodingParams::for_asset_size(50_000_000);
        assert_eq!(medium.data_shards, 10);
        assert_eq!(medium.parity_shards, 4);

        let large = ErasureCodingParams::for_asset_size(500_000_000);
        assert_eq!(large.data_shards, 20);
        assert_eq!(large.parity_shards, 8);

        let vlarge = ErasureCodingParams::for_asset_size(5_000_000_000);
        assert_eq!(vlarge.data_shards, 40);
        assert_eq!(vlarge.parity_shards, 16);

        let massive = ErasureCodingParams::for_asset_size(50_000_000_000);
        assert_eq!(massive.data_shards, 80);
        assert_eq!(massive.parity_shards, 32);
    }

    #[test]
    fn erasure_coding_display() {
        let params = ErasureCodingParams::default();
        assert_eq!(params.to_string(), "RS(10-of-14)");
    }

    #[test]
    fn erasure_coding_serde_roundtrip() {
        let params = ErasureCodingParams::new(20, 8)
            .expect("test: valid params");
        let json = serde_json::to_string(&params)
            .expect("test: serialize");
        let back: ErasureCodingParams = serde_json::from_str(&json)
            .expect("test: deserialize");
        assert_eq!(params, back);
    }

    // --- HardwareCapabilities ---

    fn sample_hardware() -> HardwareCapabilities {
        HardwareCapabilities {
            cpu_cores: 4,
            cpu_clock_mhz: 2400,
            ram_bytes: 8_000_000_000,
            storage_bytes: 100_000_000_000,
            network_bandwidth_bps: 10_000_000,
            gpu_available: false,
            gpu_vram_bytes: None,
        }
    }

    #[test]
    fn hardware_meets_minimum_spec() {
        let hw = sample_hardware();
        assert!(hw.meets_minimum_spec());
        assert!(hw.unmet_requirements().is_empty());
    }

    #[test]
    fn hardware_below_minimum_cpu_cores() {
        let mut hw = sample_hardware();
        hw.cpu_cores = 1;
        assert!(!hw.meets_minimum_spec());
        assert!(hw.unmet_requirements().contains(&"cpu_cores < 2"));
    }

    #[test]
    fn hardware_below_minimum_cpu_clock() {
        let mut hw = sample_hardware();
        hw.cpu_clock_mhz = 500;
        assert!(!hw.meets_minimum_spec());
        assert!(hw.unmet_requirements().iter().any(|s| s.contains("1 GHz")));
    }

    #[test]
    fn hardware_below_minimum_ram() {
        let mut hw = sample_hardware();
        hw.ram_bytes = 2_000_000_000;
        assert!(!hw.meets_minimum_spec());
        assert!(hw.unmet_requirements().iter().any(|s| s.contains("4 GB")));
    }

    #[test]
    fn hardware_below_minimum_storage() {
        let mut hw = sample_hardware();
        hw.storage_bytes = 10_000_000_000;
        assert!(!hw.meets_minimum_spec());
        assert!(hw.unmet_requirements().iter().any(|s| s.contains("50 GB")));
    }

    #[test]
    fn hardware_below_minimum_network() {
        let mut hw = sample_hardware();
        hw.network_bandwidth_bps = 500_000;
        assert!(!hw.meets_minimum_spec());
        assert!(hw.unmet_requirements().iter().any(|s| s.contains("1 Mb/s")));
    }

    #[test]
    fn hardware_exactly_at_minimum() {
        let hw = HardwareCapabilities {
            cpu_cores: 2,
            cpu_clock_mhz: 1_000,
            ram_bytes: 4_000_000_000,
            storage_bytes: 50_000_000_000,
            network_bandwidth_bps: 1_000_000,
            gpu_available: false,
            gpu_vram_bytes: None,
        };
        assert!(hw.meets_minimum_spec());
    }

    #[test]
    fn hardware_with_gpu() {
        let hw = HardwareCapabilities {
            gpu_available: true,
            gpu_vram_bytes: Some(8_000_000_000),
            ..sample_hardware()
        };
        assert!(hw.meets_minimum_spec());
        assert!(hw.gpu_available);
        assert_eq!(hw.gpu_vram_bytes, Some(8_000_000_000));
    }

    #[test]
    fn hardware_multiple_failures() {
        let hw = HardwareCapabilities {
            cpu_cores: 1,
            cpu_clock_mhz: 500,
            ram_bytes: 1_000_000_000,
            storage_bytes: 1_000_000_000,
            network_bandwidth_bps: 100_000,
            gpu_available: false,
            gpu_vram_bytes: None,
        };
        assert!(!hw.meets_minimum_spec());
        assert_eq!(hw.unmet_requirements().len(), 5);
    }

    #[test]
    fn hardware_display() {
        let hw = sample_hardware();
        let s = hw.to_string();
        assert!(s.contains("4C@2400MHz"), "got: {s}");
        assert!(s.contains("RAM"), "got: {s}");
    }

    #[test]
    fn hardware_serde_roundtrip() {
        let hw = sample_hardware();
        let json = serde_json::to_string(&hw)
            .expect("test: serialize");
        let back: HardwareCapabilities = serde_json::from_str(&json)
            .expect("test: deserialize");
        assert_eq!(hw, back);
    }

    // --- Validation helpers ---

    #[test]
    fn validate_blake3_hash_correct() {
        let data = b"hello hypermesh";
        let hash = compute_blake3_hash(data);
        assert!(validate_blake3_hash(&hash, data));
    }

    #[test]
    fn validate_blake3_hash_incorrect() {
        let data = b"hello hypermesh";
        let hash = compute_blake3_hash(data);
        assert!(!validate_blake3_hash(&hash, b"different data"));
    }

    #[test]
    fn validate_blake3_hash_empty() {
        let hash = compute_blake3_hash(b"");
        assert!(validate_blake3_hash(&hash, b""));
    }

    #[test]
    fn compute_blake3_hash_deterministic() {
        let data = b"test data";
        let hash1 = compute_blake3_hash(data);
        let hash2 = compute_blake3_hash(data);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn validate_node_id_valid() {
        assert!(validate_node_id(&NodeId::from("node-alpha_01.test")));
        assert!(validate_node_id(&NodeId::from("a")));
        assert!(validate_node_id(&NodeId::from("node123")));
    }

    #[test]
    fn validate_node_id_empty() {
        assert!(!validate_node_id(&NodeId::from("")));
    }

    #[test]
    fn validate_node_id_too_long() {
        let long = "a".repeat(129);
        assert!(!validate_node_id(&NodeId::from(long.as_str())));
    }

    #[test]
    fn validate_node_id_invalid_chars() {
        assert!(!validate_node_id(&NodeId::from("node alpha!")));
        assert!(!validate_node_id(&NodeId::from("node@host")));
    }

    #[test]
    fn validate_asset_id_valid() {
        assert!(validate_asset_id(&AssetId::from("asset-001")));
    }

    #[test]
    fn validate_asset_id_empty() {
        assert!(!validate_asset_id(&AssetId::from("")));
    }

    // --- ShardCommitment ---

    #[test]
    fn shard_commitment_compute_and_verify() {
        let data = b"shard0:node-a,shard1:node-b,shard2:node-c";
        let commitment = ShardCommitment::compute(data);
        assert!(commitment.verify(data));
        assert!(!commitment.verify(b"different placements"));
    }

    #[test]
    fn shard_commitment_deterministic() {
        let data = b"sorted placement data";
        let c1 = ShardCommitment::compute(data);
        let c2 = ShardCommitment::compute(data);
        assert_eq!(c1, c2);
    }

    #[test]
    fn shard_commitment_from_bytes_roundtrip() {
        let data = b"test";
        let commitment = ShardCommitment::compute(data);
        let rebuilt = ShardCommitment::from_bytes(*commitment.as_bytes());
        assert_eq!(commitment, rebuilt);
    }

    #[test]
    fn shard_commitment_display() {
        let commitment = ShardCommitment::compute(b"test");
        let s = commitment.to_string();
        assert!(s.starts_with("ShardCommit("), "got: {s}");
    }

    // --- StorageType ---

    #[test]
    fn storage_type_display() {
        assert_eq!(StorageType::Nvme.to_string(), "NVMe");
        assert_eq!(StorageType::Ssd.to_string(), "SSD");
        assert_eq!(StorageType::Hdd.to_string(), "HDD");
        assert_eq!(StorageType::Unknown.to_string(), "Unknown");
    }
}
