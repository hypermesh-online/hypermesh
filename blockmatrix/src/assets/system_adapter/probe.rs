// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Hardware probing abstraction.
//!
//! Defines the [`HardwareProbe`] trait: a small, cross-platform surface for
//! reading REAL device metrics (CPU / memory / storage) off the host. Concrete
//! probes live in [`super::os`] and are selected by target `cfg`.
//!
//! Probes return [`crate::assets::core::AssetResult`] so their output composes
//! directly with the [`crate::assets::core::AssetAdapter`] implementation in
//! [`super::system_adapter`] without an extra error-mapping layer.

use async_trait::async_trait;

use crate::assets::core::AssetResult;

/// Real CPU metrics read from the host.
#[derive(Clone, Debug, PartialEq)]
pub struct CpuMetrics {
    /// Number of logical CPU cores (>= 1 on any real machine).
    pub logical_cores: u32,
    /// Best-effort current core frequency in MHz (0 if the OS does not report it).
    pub frequency_mhz: u32,
    /// Aggregate CPU utilisation across all cores, 0.0..=100.0.
    pub utilization_percent: f32,
}

/// Real memory metrics read from the host (all values in bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct MemoryMetrics {
    /// Total physical RAM in bytes (> 0 on any real machine).
    pub total_bytes: u64,
    /// Currently used physical RAM in bytes.
    pub used_bytes: u64,
    /// Total swap in bytes (0 if no swap configured).
    pub total_swap_bytes: u64,
    /// Currently used swap in bytes.
    pub used_swap_bytes: u64,
}

/// Real storage metrics read from the host (all values in bytes).
///
/// Aggregated across every fixed (non-removable) disk the OS reports, so the
/// figure reflects the machine's total addressable backing store.
#[derive(Clone, Debug, PartialEq)]
pub struct StorageMetrics {
    /// Total storage capacity in bytes (summed over fixed disks).
    pub total_bytes: u64,
    /// Available (free) storage in bytes.
    pub available_bytes: u64,
}

impl StorageMetrics {
    /// Used storage in bytes (`total - available`, saturating).
    pub fn used_bytes(&self) -> u64 {
        self.total_bytes.saturating_sub(self.available_bytes)
    }
}

/// Cross-platform hardware probe.
///
/// One implementation per OS family (see [`super::os`]). Every method performs
/// a live read of the host -- there is no cached or self-reported state. This is
/// the substrate the Proof-of-State binding measures against.
#[async_trait]
pub trait HardwareProbe: Send + Sync {
    /// Probe live CPU metrics.
    async fn probe_cpu(&self) -> AssetResult<CpuMetrics>;

    /// Probe live memory metrics.
    async fn probe_memory(&self) -> AssetResult<MemoryMetrics>;

    /// Probe live storage metrics.
    async fn probe_storage(&self) -> AssetResult<StorageMetrics>;
}
