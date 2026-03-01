// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! CPU adapter types: allocation records, core info, scheduling, and statistics.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use sysinfo::System;
use tokio::sync::RwLock;

use crate::assets::core::{AssetRegistration, PrivacyMode, ProxyAddress};

/// CPU core allocation record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CpuAllocation {
    pub asset_id: AssetRegistration,
    pub allocated_cores: Vec<u32>,
    pub architecture: String,
    pub frequency_mhz: u32,
    pub enabled_features: Vec<String>,
    pub numa_node: Option<u32>,
    pub privacy_level: PrivacyMode,
    pub isolation_enabled: bool,
    pub time_slice_ms: u32,
    pub priority: u8,
    pub allocated_at: SystemTime,
    pub last_accessed: SystemTime,
    pub current_utilization: f32,
}

/// CPU core information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CpuCore {
    pub core_id: u32,
    pub physical_id: u32,
    pub is_logical: bool,
    pub numa_node: u32,
    pub current_frequency_mhz: u32,
    pub base_frequency_mhz: u32,
    pub max_frequency_mhz: u32,
    pub status: CoreStatus,
    pub allocated_to: Option<AssetRegistration>,
    pub temperature_celsius: Option<f32>,
}

/// CPU core status
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CoreStatus {
    Available,
    Allocated,
    InUse,
    Maintenance,
    Failed,
}

/// CPU scheduler for time-based allocation
#[derive(Clone, Debug)]
pub struct CpuScheduler {
    pub algorithm: SchedulingAlgorithm,
    pub time_slice_ms: u32,
    pub priority_levels: u8,
    pub preemption_enabled: bool,
}

/// Scheduling algorithms
#[derive(Clone, Debug)]
pub enum SchedulingAlgorithm {
    RoundRobin,
    Priority,
    Cfs,
    RealTime,
}

/// CPU usage statistics
#[derive(Clone, Debug, Default)]
pub struct CpuUsageStats {
    pub total_allocations: u64,
    pub total_deallocations: u64,
    pub active_allocations: u64,
    pub total_cpu_time_ms: u64,
    pub average_utilization: f32,
    pub peak_utilization: f32,
    pub context_switches: u64,
}

/// CPU operations for statistics
#[derive(Clone, Debug)]
pub(crate) enum CpuOperation {
    Allocate,
    Deallocate,
}

/// Internal shared state for the CPU adapter.
pub(crate) struct CpuAdapterState {
    pub allocations: Arc<RwLock<HashMap<AssetRegistration, CpuAllocation>>>,
    pub cpu_cores: Arc<RwLock<HashMap<u32, CpuCore>>>,
    pub core_allocations: Arc<RwLock<HashMap<u32, AssetRegistration>>>,
    pub proxy_mappings: Arc<RwLock<HashMap<ProxyAddress, AssetRegistration>>>,
    pub _scheduler: Arc<RwLock<CpuScheduler>>,
    pub total_cores: u32,
    pub usage_stats: Arc<RwLock<CpuUsageStats>>,
    pub system_info: Arc<RwLock<System>>,
}
