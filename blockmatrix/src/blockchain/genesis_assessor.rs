// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Genesis capability assessment (R1, R10).
//!
//! Assesses local hardware on first boot and produces [`GenesisAssetRecord`]s
//! for each discovered resource. Each record gets an IPv6-encoded
//! [`AssetAddress`] derived from the node's matrix position and a BLAKE3
//! content hash of the capability data.
//!
//! Hardware probing is abstracted behind [`HardwareProbe`] so tests can
//! inject synthetic capabilities without touching real `/proc`.

use hypermesh_lib::{
    asset::{GenesisAssetRecord, SystemAssetKind},
    protocol::HardwareCapabilities,
    types::{AssetAddress, AssetId, ContentHash},
};

use crate::matrix::coordinate::MatrixCoordinate;

// ---------------------------------------------------------------------------
// HardwareProbe trait
// ---------------------------------------------------------------------------

/// Abstraction over real hardware detection.
///
/// Production code uses [`RealHardwareProbe`]; tests inject
/// [`SyntheticHardwareProbe`] with known values.
pub trait HardwareProbe: Send + Sync {
    /// Assess the hardware and return capabilities.
    fn assess(&self) -> Result<HardwareCapabilities, String>;
}

// ---------------------------------------------------------------------------
// RealHardwareProbe
// ---------------------------------------------------------------------------

/// Reads hardware metrics from `/proc` (Linux) via [`crate::metrics::hardware`].
pub struct RealHardwareProbe;

impl HardwareProbe for RealHardwareProbe {
    fn assess(&self) -> Result<HardwareCapabilities, String> {
        let metrics = crate::metrics::hardware::collect()
            .map_err(|e| format!("hardware collection failed: {e}"))?;

        // Estimate CPU clock from /proc/cpuinfo if available, else use 0.
        let cpu_clock_mhz = estimate_cpu_clock_mhz();

        // Network bandwidth estimation: use total bytes seen as a lower bound.
        // A real implementation would run an iperf-like self-test.
        let network_bandwidth_bps = (metrics.network.total_rx_bytes
            + metrics.network.total_tx_bytes)
            .max(HardwareCapabilities::MIN_NETWORK_BPS);

        Ok(HardwareCapabilities {
            cpu_cores: metrics.cpu.core_count as u16,
            cpu_clock_mhz,
            ram_bytes: metrics.memory.total_bytes,
            storage_bytes: metrics.storage.total_bytes,
            network_bandwidth_bps,
            gpu_available: false, // GPU detection deferred
            gpu_vram_bytes: None,
        })
    }
}

/// Best-effort CPU clock from `/proc/cpuinfo`.
fn estimate_cpu_clock_mhz() -> u32 {
    let content = match std::fs::read_to_string("/proc/cpuinfo") {
        Ok(c) => c,
        Err(_) => return 0,
    };
    for line in content.lines() {
        if line.starts_with("cpu MHz") {
            if let Some(val) = line.split(':').nth(1) {
                if let Ok(mhz) = val.trim().parse::<f64>() {
                    return mhz as u32;
                }
            }
        }
    }
    0
}

// ---------------------------------------------------------------------------
// SyntheticHardwareProbe
// ---------------------------------------------------------------------------

/// Probe that returns caller-supplied capabilities.
/// Intended for unit tests so hardware assessment logic can be exercised
/// without needing a real `/proc` filesystem.
pub struct SyntheticHardwareProbe {
    capabilities: HardwareCapabilities,
}

impl SyntheticHardwareProbe {
    /// Create with the given capabilities.
    pub fn new(capabilities: HardwareCapabilities) -> Self {
        Self { capabilities }
    }

    /// Create with capabilities that meet R13 minimum spec.
    pub fn meets_minimum() -> Self {
        Self {
            capabilities: HardwareCapabilities {
                cpu_cores: 4,
                cpu_clock_mhz: 2400,
                ram_bytes: 8_000_000_000,
                storage_bytes: 100_000_000_000,
                network_bandwidth_bps: 100_000_000,
                gpu_available: false,
                gpu_vram_bytes: None,
            },
        }
    }
}

impl HardwareProbe for SyntheticHardwareProbe {
    fn assess(&self) -> Result<HardwareCapabilities, String> {
        Ok(self.capabilities.clone())
    }
}

// ---------------------------------------------------------------------------
// GenesisAssessor
// ---------------------------------------------------------------------------

/// Assesses hardware and produces genesis asset records (R1, R10).
///
/// On first boot a node calls [`GenesisAssessor::assess`] to discover its
/// resources. Each resource becomes an IPv6-addressed asset recorded in the
/// genesis block.
pub struct GenesisAssessor<P: HardwareProbe> {
    probe: P,
    coordinate: MatrixCoordinate,
}

impl<P: HardwareProbe> GenesisAssessor<P> {
    /// Create a new assessor for the given matrix position.
    pub fn new(probe: P, coordinate: MatrixCoordinate) -> Self {
        Self { probe, coordinate }
    }

    /// Assess hardware and produce genesis asset records.
    ///
    /// Returns one [`GenesisAssetRecord`] for each detected resource kind:
    /// Cpu, Memory, Storage, Network, and optionally Gpu.
    ///
    /// The `genesis_block_hash` is initially zeroed; the caller must
    /// back-fill it after the genesis block is actually created.
    pub fn assess(&self) -> Result<Vec<GenesisAssetRecord>, String> {
        let caps = self.probe.assess()?;

        if !caps.meets_minimum_spec() {
            let unmet = caps.unmet_requirements();
            return Err(format!(
                "hardware does not meet R13 minimum spec: {}",
                unmet.join(", ")
            ));
        }

        let mut records = Vec::with_capacity(5);

        // CPU asset
        records.push(self.build_record(SystemAssetKind::Cpu, &caps)?);
        // Memory asset
        records.push(self.build_record(SystemAssetKind::Memory, &caps)?);
        // Storage asset
        records.push(self.build_record(SystemAssetKind::Storage, &caps)?);
        // Network asset
        records.push(self.build_record(SystemAssetKind::Network, &caps)?);

        // GPU (optional)
        if caps.gpu_available {
            records.push(self.build_record(SystemAssetKind::Gpu, &caps)?);
        }

        tracing::info!(
            "Genesis assessment complete: {} assets at ({},{},{})",
            records.len(),
            self.coordinate.x,
            self.coordinate.y,
            self.coordinate.z,
        );

        Ok(records)
    }

    /// Build a single genesis record for the given asset kind.
    fn build_record(
        &self,
        kind: SystemAssetKind,
        caps: &HardwareCapabilities,
    ) -> Result<GenesisAssetRecord, String> {
        // Content hash = BLAKE3(kind_id || capabilities_bytes)
        let content_hash = compute_asset_content_hash(kind, caps);

        // IPv6 asset address from matrix position + content hash
        let address = AssetAddress::new(
            self.coordinate.x as i64,
            self.coordinate.y as i64,
            self.coordinate.z as i64,
            &content_hash,
        )
        .map_err(|e| format!("address error for {kind}: {e}"))?;

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Ok(GenesisAssetRecord {
            asset_id: AssetId::from(format!("genesis-{}-{}", kind.type_name(), content_hash)),
            kind,
            address,
            capabilities: caps.clone(),
            genesis_block_hash: ContentHash::zeroed(), // back-filled later
            assessed_at: now_ms,
        })
    }
}

/// Deterministic content hash for a genesis asset.
fn compute_asset_content_hash(
    kind: SystemAssetKind,
    caps: &HardwareCapabilities,
) -> ContentHash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&[kind.type_id()]);
    hasher.update(&caps.cpu_cores.to_le_bytes());
    hasher.update(&caps.cpu_clock_mhz.to_le_bytes());
    hasher.update(&caps.ram_bytes.to_le_bytes());
    hasher.update(&caps.storage_bytes.to_le_bytes());
    hasher.update(&caps.network_bandwidth_bps.to_le_bytes());
    hasher.update(&[caps.gpu_available as u8]);
    if let Some(vram) = caps.gpu_vram_bytes {
        hasher.update(&vram.to_le_bytes());
    }
    ContentHash::from_bytes(*hasher.finalize().as_bytes())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_coord() -> MatrixCoordinate {
        MatrixCoordinate::new(5, 10, 15).expect("test: valid coordinate")
    }

    fn meets_spec_caps() -> HardwareCapabilities {
        HardwareCapabilities {
            cpu_cores: 4,
            cpu_clock_mhz: 2400,
            ram_bytes: 8_000_000_000,
            storage_bytes: 100_000_000_000,
            network_bandwidth_bps: 100_000_000,
            gpu_available: false,
            gpu_vram_bytes: None,
        }
    }

    #[test]
    fn genesis_assess_produces_four_records_without_gpu() {
        let probe = SyntheticHardwareProbe::new(meets_spec_caps());
        let assessor = GenesisAssessor::new(probe, test_coord());
        let records = assessor.assess().expect("test: assessment");

        assert_eq!(records.len(), 4, "Cpu + Memory + Storage + Network");

        let kinds: Vec<_> = records.iter().map(|r| r.kind).collect();
        assert!(kinds.contains(&SystemAssetKind::Cpu));
        assert!(kinds.contains(&SystemAssetKind::Memory));
        assert!(kinds.contains(&SystemAssetKind::Storage));
        assert!(kinds.contains(&SystemAssetKind::Network));
    }

    #[test]
    fn genesis_assess_includes_gpu_when_available() {
        let mut caps = meets_spec_caps();
        caps.gpu_available = true;
        caps.gpu_vram_bytes = Some(8_000_000_000);

        let probe = SyntheticHardwareProbe::new(caps);
        let assessor = GenesisAssessor::new(probe, test_coord());
        let records = assessor.assess().expect("test: assessment");

        assert_eq!(records.len(), 5, "should include Gpu");
        let kinds: Vec<_> = records.iter().map(|r| r.kind).collect();
        assert!(kinds.contains(&SystemAssetKind::Gpu));
    }

    #[test]
    fn genesis_assess_rejects_below_minimum_spec() {
        let caps = HardwareCapabilities {
            cpu_cores: 1,
            cpu_clock_mhz: 500,
            ram_bytes: 1_000_000_000,
            storage_bytes: 10_000_000_000,
            network_bandwidth_bps: 100_000,
            gpu_available: false,
            gpu_vram_bytes: None,
        };
        let probe = SyntheticHardwareProbe::new(caps);
        let assessor = GenesisAssessor::new(probe, test_coord());
        let result = assessor.assess();

        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("R13"), "error should reference R13: {msg}");
    }

    #[test]
    fn genesis_records_have_valid_ipv6_addresses() {
        let probe = SyntheticHardwareProbe::meets_minimum();
        let assessor = GenesisAssessor::new(probe, test_coord());
        let records = assessor.assess().expect("test: assessment");

        for record in &records {
            assert!(
                record.address.is_hypermesh(),
                "address must be HyperMesh ULA"
            );
            let (x, y, z) = record.address.matrix_coords();
            assert_eq!(x, 5);
            assert_eq!(y, 10);
            assert_eq!(z, 15);
        }
    }

    #[test]
    fn genesis_content_hash_is_deterministic() {
        let caps = meets_spec_caps();
        let h1 = compute_asset_content_hash(SystemAssetKind::Cpu, &caps);
        let h2 = compute_asset_content_hash(SystemAssetKind::Cpu, &caps);
        assert_eq!(h1, h2, "same input must produce same hash");

        let h3 = compute_asset_content_hash(SystemAssetKind::Memory, &caps);
        assert_ne!(h1, h3, "different kind must produce different hash");
    }
}
