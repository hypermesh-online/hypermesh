// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Storage allocation and capacity management
//!
//! Features:
//! - Device allocation with replication support
//! - Capacity tracking and management
//! - Usage statistics
//! - Storage pool management

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

use crate::assets::core::{
    AssetId, AssetError, AssetResult, PrivacyLevel, StorageRequirements, StorageType,
};
use super::sharding::ShardingConfig;
use super::devices::{StorageDevice, StorageStatus};

/// Storage allocation record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageAllocation {
    /// Asset ID
    pub asset_id: AssetId,
    /// Allocated storage devices
    pub allocated_devices: Vec<String>,
    /// Total allocated size in bytes
    pub allocated_size_bytes: u64,
    /// Storage type (SSD, NVMe, HDD, etc.)
    pub storage_type: StorageType,
    /// Replication factor
    pub replication_factor: u32,
    /// Encryption enabled
    pub encryption_enabled: bool,
    /// Encryption key ID (Kyber quantum-resistant)
    pub encryption_key_id: Option<String>,
    /// Sharding configuration
    pub sharding_config: ShardingConfig,
    /// Privacy level
    pub privacy_level: PrivacyLevel,
    /// Mount path for access
    pub mount_path: Option<String>,
    /// Allocation timestamp
    pub allocated_at: SystemTime,
    /// Last accessed timestamp
    pub last_accessed: SystemTime,
    /// Current IOPS
    pub current_iops: u32,
    /// Current throughput in MB/s
    pub current_throughput_mbps: f32,
}

/// Storage pool for distributed management
#[derive(Clone, Debug)]
pub struct StoragePool {
    /// Pool identifier
    pub pool_id: String,
    /// Total pool capacity in bytes
    pub total_capacity: u64,
    /// Available capacity in bytes
    pub available_capacity: u64,
    /// Storage type in pool
    pub storage_type: StorageType,
    /// Pool privacy level
    pub privacy_level: PrivacyLevel,
    /// Devices in pool
    pub devices: Vec<String>,
    /// Active allocations
    pub allocations: Vec<AssetId>,
    /// Pool health status
    pub health_status: PoolHealthStatus,
}

/// Storage pool health status
#[derive(Clone, Debug)]
pub enum PoolHealthStatus {
    /// Pool is healthy
    Healthy,
    /// Pool is degraded but functional
    Degraded,
    /// Pool is critical
    Critical,
    /// Pool has failed
    Failed,
}

/// Storage usage statistics
#[derive(Clone, Debug, Default)]
pub struct StorageUsageStats {
    /// Total allocations made
    pub total_allocations: u64,
    /// Total deallocations made
    pub total_deallocations: u64,
    /// Current active allocations
    pub active_allocations: u64,
    /// Total bytes allocated
    pub total_bytes_allocated: u64,
    /// Total read operations
    pub total_read_ops: u64,
    /// Total write operations
    pub total_write_ops: u64,
    /// Total bytes read
    pub total_bytes_read: u64,
    /// Total bytes written
    pub total_bytes_written: u64,
    /// Deduplication savings in bytes
    pub dedup_savings_bytes: u64,
    /// Compression savings in bytes
    pub compression_savings_bytes: u64,
}

/// Storage operations for statistics
#[derive(Clone, Debug)]
#[allow(dead_code)] // Variants for future storage operation tracking
pub enum StorageOperation {
    Allocate,
    Deallocate,
    Read,
    Write,
}

/// Allocate storage from devices
pub async fn allocate_storage_from_devices(
    storage_req: &StorageRequirements,
    asset_id: &AssetId,
    storage_devices: &Arc<RwLock<HashMap<String, StorageDevice>>>,
    device_allocations: &Arc<RwLock<HashMap<String, AssetId>>>,
) -> AssetResult<(Vec<String>, u64)> {
    let mut devices = storage_devices.write().await;
    let mut device_allocs = device_allocations.write().await;
    let mut allocated_devices = Vec::new();
    let mut total_allocated_size = 0u64;

    // Find devices matching storage type
    let mut suitable_devices: Vec<String> = devices
        .iter()
        .filter(|(_, device)| {
            matches!(device.status, StorageStatus::Available) &&
            device.storage_type == storage_req.storage_type &&
            device.available_capacity_bytes >= storage_req.size_bytes &&
            device.max_iops >= storage_req.min_iops.unwrap_or(0) &&
            device.max_throughput_mbps >= storage_req.min_bandwidth_mbps.unwrap_or(0)
        })
        .map(|(device_id, _)| device_id.clone())
        .collect();

    // Sort by available capacity (largest first)
    suitable_devices.sort_by_key(|device_id| {
        let device = devices.get(device_id).unwrap();
        std::cmp::Reverse(device.available_capacity_bytes)
    });

    // Allocate storage with replication
    let size_per_replica = storage_req.size_bytes;
    let required_replicas = storage_req.durability_replicas;

    if suitable_devices.len() < required_replicas as usize {
        return Err(AssetError::AllocationFailed {
            reason: format!(
                "Insufficient storage devices for replication: {} required, {} available",
                required_replicas, suitable_devices.len()
            )
        });
    }

    // Allocate to multiple devices for replication
    for device_id in suitable_devices.iter().take(required_replicas as usize) {
        let device = devices.get_mut(device_id).unwrap();

        if device.available_capacity_bytes < size_per_replica {
            continue; // Skip if insufficient space
        }

        device.status = StorageStatus::Allocated;
        device.allocated_to = Some(asset_id.clone());
        device.available_capacity_bytes -= size_per_replica;

        device_allocs.insert(device_id.clone(), asset_id.clone());
        allocated_devices.push(device_id.clone());
        total_allocated_size += size_per_replica;
    }

    if allocated_devices.len() < required_replicas as usize {
        // Rollback partial allocation
        for device_id in &allocated_devices {
            let device = devices.get_mut(device_id).unwrap();
            device.status = StorageStatus::Available;
            device.allocated_to = None;
            device.available_capacity_bytes += size_per_replica;
            device_allocs.remove(device_id);
        }

        return Err(AssetError::AllocationFailed {
            reason: "Insufficient storage capacity across available devices".to_string()
        });
    }

    Ok((allocated_devices, total_allocated_size))
}

/// Deallocate storage from devices
pub async fn deallocate_storage_from_devices(
    allocation: &StorageAllocation,
    storage_devices: &Arc<RwLock<HashMap<String, StorageDevice>>>,
    device_allocations: &Arc<RwLock<HashMap<String, AssetId>>>,
) -> AssetResult<()> {
    let mut devices = storage_devices.write().await;
    let mut device_allocs = device_allocations.write().await;

    let size_per_device = allocation.allocated_size_bytes / allocation.allocated_devices.len() as u64;

    for device_id in &allocation.allocated_devices {
        if let Some(device) = devices.get_mut(device_id) {
            device.status = StorageStatus::Available;
            device.allocated_to = None;
            device.available_capacity_bytes += size_per_device;
        }
        device_allocs.remove(device_id);
    }

    Ok(())
}

/// Update usage statistics
pub async fn update_usage_stats(
    usage_stats: &Arc<RwLock<StorageUsageStats>>,
    operation: StorageOperation,
    bytes: u64,
) {
    let mut stats = usage_stats.write().await;

    match operation {
        StorageOperation::Allocate => {
            stats.total_allocations += 1;
            stats.active_allocations += 1;
            stats.total_bytes_allocated += bytes;
        },
        StorageOperation::Deallocate => {
            stats.total_deallocations += 1;
            stats.active_allocations = stats.active_allocations.saturating_sub(1);
            stats.total_bytes_allocated = stats.total_bytes_allocated.saturating_sub(bytes);
        },
        StorageOperation::Read => {
            stats.total_read_ops += 1;
            stats.total_bytes_read += bytes;
        },
        StorageOperation::Write => {
            stats.total_write_ops += 1;
            stats.total_bytes_written += bytes;
        },
    }
}

/// Initialize default storage pool
pub fn initialize_default_pool(
    total_capacity: u64,
    device_ids: Vec<String>,
) -> HashMap<String, StoragePool> {
    let mut storage_pools = HashMap::new();
    storage_pools.insert("default".to_string(), StoragePool {
        pool_id: "default".to_string(),
        total_capacity,
        available_capacity: total_capacity,
        storage_type: StorageType::Ssd, // Default assumption
        privacy_level: PrivacyLevel::PRIVATE,
        devices: device_ids,
        allocations: Vec::new(),
        health_status: PoolHealthStatus::Healthy,
    });
    storage_pools
}
