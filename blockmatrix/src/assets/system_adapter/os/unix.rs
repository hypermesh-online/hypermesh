// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! UNIX hardware probe.
//!
//! Core count comes from `num_cpus::get()` (honours cgroup/affinity limits on
//! Linux); memory and storage come from the shared `sysinfo` readers. Mirrors
//! the same `df`/`/proc`-backed reality TrustChain's `SpaceProof` already
//! measures, but cross-platform via `sysinfo`'s `Disks`.

#![cfg(unix)]

use async_trait::async_trait;

use crate::assets::core::AssetResult;
use crate::assets::system_adapter::os::{
    probe_cpu_shared, probe_memory_shared, probe_storage_shared,
};
use crate::assets::system_adapter::probe::{
    CpuMetrics, HardwareProbe, MemoryMetrics, StorageMetrics,
};

/// Hardware probe for UNIX-family targets (Linux, macOS, BSD).
#[derive(Clone, Copy, Debug, Default)]
pub struct UnixHardwareProbe;

impl UnixHardwareProbe {
    /// Create a new UNIX hardware probe.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl HardwareProbe for UnixHardwareProbe {
    async fn probe_cpu(&self) -> AssetResult<CpuMetrics> {
        // num_cpus respects scheduler affinity / cgroup quotas on Linux.
        let core_count = u32::try_from(num_cpus::get()).unwrap_or(u32::MAX);
        Ok(probe_cpu_shared(core_count)?)
    }

    async fn probe_memory(&self) -> AssetResult<MemoryMetrics> {
        Ok(probe_memory_shared()?)
    }

    async fn probe_storage(&self) -> AssetResult<StorageMetrics> {
        Ok(probe_storage_shared()?)
    }
}
