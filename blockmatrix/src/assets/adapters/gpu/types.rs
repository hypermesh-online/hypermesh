// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! GPU type definitions - allocation records, device info, and status types.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

use crate::assets::core::{
    AssetRegistration, PrivacyLevel, ProxyAddress,
};

/// GPU allocation record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GpuAllocation {
    /// Asset ID
    pub asset_id: AssetRegistration,
    /// Allocated GPU device IDs
    pub allocated_devices: Vec<u32>,
    /// GPU memory allocated in bytes
    pub allocated_memory_bytes: u64,
    /// Compute capability required
    pub compute_capability: String,
    /// Nova engine features enabled (Vulkan compute, Ray tracing, etc.)
    pub enabled_features: Vec<String>,
    /// Privacy level
    pub privacy_level: PrivacyLevel,
    /// Process isolation enabled
    pub isolation_enabled: bool,
    /// GPU compute priority (0-255)
    pub compute_priority: u8,
    /// Allocation timestamp
    pub allocated_at: SystemTime,
    /// Last accessed timestamp
    pub last_accessed: SystemTime,
    /// Current GPU utilization percentage
    pub current_utilization: f32,
    /// Current memory utilization percentage
    pub memory_utilization: f32,
    /// GPU context handle
    pub context_handle: Option<String>,
}

/// GPU device information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GpuDevice {
    /// Device ID
    pub device_id: u32,
    /// Device name
    pub device_name: String,
    /// Compute capability (e.g., "8.6" for RTX 30xx)
    pub compute_capability: String,
    /// Total memory in bytes
    pub total_memory_bytes: u64,
    /// Available memory in bytes
    pub available_memory_bytes: u64,
    /// Vulkan compute units
    pub vulkan_compute_units: u32,
    /// Nova execution units
    pub nova_execution_units: u32,
    /// Base clock in MHz
    pub base_clock_mhz: u32,
    /// Memory clock in MHz
    pub memory_clock_mhz: u32,
    /// PCI bus ID
    pub pci_bus_id: String,
    /// Current status
    pub status: GpuStatus,
    /// Current allocation asset ID
    pub allocated_to: Option<AssetRegistration>,
    /// Temperature in Celsius
    pub temperature_celsius: Option<f32>,
    /// Power consumption in watts
    pub power_watts: Option<f32>,
}

/// GPU device status
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GpuStatus {
    /// GPU is available for allocation
    Available,
    /// GPU is allocated but idle
    Allocated,
    /// GPU is actively computing
    Computing,
    /// GPU is in maintenance mode
    Maintenance,
    /// GPU has failed
    Failed,
}

/// GPU compute context for isolation
#[derive(Clone, Debug)]
pub struct GpuContext {
    /// Context ID
    pub context_id: String,
    /// Associated asset ID
    pub asset_id: AssetRegistration,
    /// Device ID
    pub device_id: u32,
    /// Memory allocated to context
    pub allocated_memory: u64,
    /// Compute streams
    pub compute_streams: Vec<u32>,
    /// Created timestamp
    pub created_at: SystemTime,
    /// Last activity timestamp
    pub last_activity: SystemTime,
}

/// GPU Asset Adapter implementation
pub struct GpuAssetAdapter {
    /// Active GPU allocations by asset ID
    pub(crate) allocations: Arc<RwLock<HashMap<AssetRegistration, GpuAllocation>>>,
    /// GPU device information and status
    pub(crate) gpu_devices: Arc<RwLock<HashMap<u32, GpuDevice>>>,
    /// Device allocation mapping (device_id -> asset_id)
    pub(crate) device_allocations: Arc<RwLock<HashMap<u32, AssetRegistration>>>,
    /// GPU compute contexts
    pub(crate) gpu_contexts: Arc<RwLock<HashMap<String, GpuContext>>>,
    /// Proxy address mappings
    pub(crate) proxy_mappings: Arc<RwLock<HashMap<ProxyAddress, AssetRegistration>>>,
    /// Total GPU devices available
    pub(crate) total_devices: u32,
    /// GPU usage statistics
    pub(crate) usage_stats: Arc<RwLock<GpuUsageStats>>,
}

/// GPU usage statistics
#[derive(Clone, Debug, Default)]
pub struct GpuUsageStats {
    pub total_allocations: u64,
    pub total_deallocations: u64,
    pub active_allocations: u64,
    pub total_memory_allocated: u64,
    pub average_utilization: f32,
    pub peak_utilization: f32,
    pub compute_operations: u64,
    pub memory_transfers: u64,
}

/// GPU operations for statistics
#[derive(Clone, Debug)]
#[allow(dead_code)] // Variants for future GPU operation tracking
pub(crate) enum GpuOperation {
    Allocate,
    Deallocate,
    Compute,
    MemoryTransfer,
}
