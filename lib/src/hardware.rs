// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Hardware capability types for genesis block assessment (R1, R13).
//!
//! All types here represent assessed (not self-reported) hardware capabilities
//! used during node genesis to instantiate IPv6-addressed assets with Proof of State.

use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Summary hardware capabilities (R1, R13)
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
    pub const MIN_NETWORK_BPS: u64 = 1_000_000; // 1 Mb/s
    pub const MIN_STORAGE_BYTES: u64 = 50_000_000_000; // 50 GB
    pub const MIN_RAM_BYTES: u64 = 4_000_000_000; // 4 GB
    pub const MIN_CPU_CORES: u16 = 2;
    pub const MIN_CPU_CLOCK_MHZ: u32 = 1_000; // 1 GHz

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
// Detailed capability types (R1)
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

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
        let json = serde_json::to_string(&hw).expect("test: serialize");
        let back: HardwareCapabilities =
            serde_json::from_str(&json).expect("test: deserialize");
        assert_eq!(hw, back);
    }

    #[test]
    fn storage_type_display() {
        assert_eq!(StorageType::Nvme.to_string(), "NVMe");
        assert_eq!(StorageType::Ssd.to_string(), "SSD");
        assert_eq!(StorageType::Hdd.to_string(), "HDD");
        assert_eq!(StorageType::Unknown.to_string(), "Unknown");
    }
}
