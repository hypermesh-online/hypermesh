// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! OS-specific hardware probes.
//!
//! `sysinfo` provides the cross-platform memory and disk reads, so the bulk of
//! the probe is shared here. The per-OS modules differ only in how they count
//! CPU cores (the most OS-divergent figure):
//!   - UNIX  -> `num_cpus::get()`
//!   - Windows -> `std::thread::available_parallelism()`
//!
//! The right probe is selected by `cfg` at the [`super`] re-export, so exactly
//! one compiles for the active target.

#[cfg(unix)]
pub mod unix;

#[cfg(windows)]
pub mod windows;

use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};

use crate::assets::core::AssetError;

use super::probe::{CpuMetrics, MemoryMetrics, StorageMetrics};

/// Shared, cross-platform CPU read via `sysinfo`, given an OS-supplied core count.
///
/// `core_count` comes from the per-OS probe (`num_cpus` / `available_parallelism`);
/// frequency and utilisation come from `sysinfo`'s global CPU view. A single
/// refresh is sufficient for frequency; utilisation is a coarse instantaneous
/// sample (sysinfo needs two refreshes for a precise delta, which a one-shot
/// probe deliberately avoids).
pub(crate) fn probe_cpu_shared(core_count: u32) -> Result<CpuMetrics, AssetError> {
    let mut system =
        System::new_with_specifics(RefreshKind::new().with_cpu(CpuRefreshKind::everything()));
    system.refresh_cpu();

    let global = system.global_cpu_info();
    let frequency_mhz = u32::try_from(global.frequency()).unwrap_or(u32::MAX);
    let utilization_percent = global.cpu_usage().clamp(0.0, 100.0);

    if core_count == 0 {
        return Err(AssetError::AdapterError {
            message: "hardware probe reported zero CPU cores".to_string(),
        });
    }

    Ok(CpuMetrics {
        logical_cores: core_count,
        frequency_mhz,
        utilization_percent,
    })
}

/// Shared, cross-platform memory read via `sysinfo` (bytes).
pub(crate) fn probe_memory_shared() -> Result<MemoryMetrics, AssetError> {
    let mut system =
        System::new_with_specifics(RefreshKind::new().with_memory(MemoryRefreshKind::everything()));
    system.refresh_memory();

    let total_bytes = system.total_memory();
    if total_bytes == 0 {
        return Err(AssetError::AdapterError {
            message: "hardware probe reported zero total memory".to_string(),
        });
    }

    Ok(MemoryMetrics {
        total_bytes,
        used_bytes: system.used_memory(),
        total_swap_bytes: system.total_swap(),
        used_swap_bytes: system.used_swap(),
    })
}

/// Shared, cross-platform storage read via `sysinfo` `Disks` (bytes).
///
/// Sums capacity over fixed (non-removable) disks so removable media do not
/// inflate the host's reported backing store. Falls back to including all disks
/// only if no fixed disk is reported (e.g. exotic mounts), so the figure is
/// never spuriously zero on a machine that clearly has storage.
pub(crate) fn probe_storage_shared() -> Result<StorageMetrics, AssetError> {
    let disks = Disks::new_with_refreshed_list();

    let (mut total_bytes, mut available_bytes) = sum_fixed_disks(&disks);

    if total_bytes == 0 {
        // No fixed disk distinguished -- fall back to every reported disk.
        let (t, a) = sum_all_disks(&disks);
        total_bytes = t;
        available_bytes = a;
    }

    if total_bytes == 0 {
        return Err(AssetError::AdapterError {
            message: "hardware probe reported zero storage capacity".to_string(),
        });
    }

    Ok(StorageMetrics {
        total_bytes,
        available_bytes,
    })
}

/// Sum capacity/availability over fixed (non-removable) disks.
fn sum_fixed_disks(disks: &Disks) -> (u64, u64) {
    disks
        .iter()
        .filter(|disk| !disk.is_removable())
        .fold((0u64, 0u64), |(total, avail), disk| {
            (
                total.saturating_add(disk.total_space()),
                avail.saturating_add(disk.available_space()),
            )
        })
}

/// Sum capacity/availability over every reported disk.
fn sum_all_disks(disks: &Disks) -> (u64, u64) {
    disks.iter().fold((0u64, 0u64), |(total, avail), disk| {
        (
            total.saturating_add(disk.total_space()),
            avail.saturating_add(disk.available_space()),
        )
    })
}
