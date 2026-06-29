// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Windows hardware probe.
//!
//! Core count comes from `std::thread::available_parallelism()` (the std,
//! dependency-free way to read the host's parallelism on Windows); memory and
//! storage come from the shared `sysinfo` readers, which back onto the Win32
//! system-information APIs.

#![cfg(windows)]

use async_trait::async_trait;

use crate::assets::core::AssetResult;
use crate::assets::system_adapter::os::{
    probe_cpu_shared, probe_memory_shared, probe_storage_shared,
};
use crate::assets::system_adapter::probe::{
    CpuMetrics, HardwareProbe, MemoryMetrics, StorageMetrics,
};

/// Hardware probe for Windows targets.
#[derive(Clone, Copy, Debug, Default)]
pub struct WindowsHardwareProbe;

impl WindowsHardwareProbe {
    /// Create a new Windows hardware probe.
    pub fn new() -> Self {
        Self
    }

    /// Logical core count via std, falling back to 1 if the OS cannot report it.
    fn core_count() -> u32 {
        std::thread::available_parallelism()
            .map(|n| u32::try_from(n.get()).unwrap_or(u32::MAX))
            .unwrap_or(1)
    }
}

#[async_trait]
impl HardwareProbe for WindowsHardwareProbe {
    async fn probe_cpu(&self) -> AssetResult<CpuMetrics> {
        Ok(probe_cpu_shared(Self::core_count())?)
    }

    async fn probe_memory(&self) -> AssetResult<MemoryMetrics> {
        Ok(probe_memory_shared()?)
    }

    async fn probe_storage(&self) -> AssetResult<StorageMetrics> {
        Ok(probe_storage_shared()?)
    }
}
