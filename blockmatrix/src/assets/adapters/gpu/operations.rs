// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! GPU adapter operations - allocation, proxy, context, and stats management.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;

use crate::assets::core::{
    AssetId, AssetResult, AssetError,
    ProxyAddress, GpuRequirements, ConsensusProof,
};

use super::types::*;

impl GpuAssetAdapter {
    /// Create new GPU adapter
    pub async fn new() -> Self {
        let (total_devices, gpu_devices) = Self::detect_gpu_configuration().await;

        Self {
            allocations: Arc::new(RwLock::new(HashMap::new())),
            gpu_devices: Arc::new(RwLock::new(gpu_devices)),
            device_allocations: Arc::new(RwLock::new(HashMap::new())),
            gpu_contexts: Arc::new(RwLock::new(HashMap::new())),
            proxy_mappings: Arc::new(RwLock::new(HashMap::new())),
            total_devices,
            usage_stats: Arc::new(RwLock::new(GpuUsageStats::default())),
        }
    }

    /// Allocate GPU devices based on requirements
    pub(crate) async fn allocate_gpu_devices(
        &self,
        gpu_req: &GpuRequirements,
        asset_id: &AssetId,
    ) -> AssetResult<(Vec<u32>, u64)> {
        let mut devices = self.gpu_devices.write().await;
        let mut device_allocations = self.device_allocations.write().await;
        let mut allocated_devices = Vec::new();
        let mut total_allocated_memory = 0u64;

        // Find available devices matching requirements
        let mut available_devices: Vec<u32> = devices
            .iter()
            .filter(|(_, device)| {
                matches!(device.status, GpuStatus::Available) &&
                device.available_memory_bytes >= gpu_req.min_memory_mb.unwrap_or(0) as u64 * 1024 * 1024 &&
                gpu_req.compute_capability.as_ref().map_or(true, |cc| device.compute_capability >= *cc)
            })
            .map(|(device_id, _)| *device_id)
            .collect();

        // Sort by available memory (largest first)
        available_devices.sort_by_key(|device_id| {
            let device = devices.get(device_id).expect("device_id from filtered iterator must exist");
            std::cmp::Reverse(device.available_memory_bytes)
        });

        // Check if we have enough devices
        if available_devices.len() < gpu_req.units as usize {
            return Err(AssetError::AllocationFailed {
                reason: format!(
                    "Insufficient GPU devices: {} requested, {} available",
                    gpu_req.units, available_devices.len()
                )
            });
        }

        // Allocate the requested number of devices
        let memory_per_device = gpu_req.min_memory_mb.unwrap_or(1024) as u64 * 1024 * 1024;

        for &device_id in available_devices.iter().take(gpu_req.units as usize) {
            let device = devices.get_mut(&device_id).ok_or_else(|| AssetError::AllocationFailed {
                reason: format!("GPU device {} disappeared during allocation", device_id),
            })?;

            if device.available_memory_bytes < memory_per_device {
                continue;
            }

            device.status = GpuStatus::Allocated;
            device.allocated_to = Some(asset_id.clone());
            device.available_memory_bytes -= memory_per_device;

            device_allocations.insert(device_id, asset_id.clone());
            allocated_devices.push(device_id);
            total_allocated_memory += memory_per_device;
        }

        if allocated_devices.len() < gpu_req.units as usize {
            // Rollback partial allocation
            for &device_id in &allocated_devices {
                if let Some(device) = devices.get_mut(&device_id) {
                    device.status = GpuStatus::Available;
                    device.allocated_to = None;
                    device.available_memory_bytes += memory_per_device;
                }
                device_allocations.remove(&device_id);
            }

            return Err(AssetError::AllocationFailed {
                reason: "Insufficient GPU memory across available devices".to_string()
            });
        }

        Ok((allocated_devices, total_allocated_memory))
    }

    /// Generate proxy address for GPU access
    pub(crate) async fn generate_proxy_address(asset_id: &AssetId) -> ProxyAddress {
        let mut node_id = [0u8; 8];
        node_id.copy_from_slice(&asset_id.content_hash[..8]);
        ProxyAddress::new(
            [0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad,
             0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01],
            node_id,
            8080
        )
    }

    /// Create GPU compute context for isolation
    pub(crate) async fn create_gpu_context(&self, asset_id: &AssetId, device_id: u32) -> String {
        let context_id = format!("gpu_ctx_{}_{}", device_id, hex::encode(&asset_id.content_hash[..8]));

        let context = GpuContext {
            context_id: context_id.clone(),
            asset_id: asset_id.clone(),
            device_id,
            allocated_memory: 0,
            compute_streams: vec![0, 1],
            created_at: SystemTime::now(),
            last_activity: SystemTime::now(),
        };

        let mut contexts = self.gpu_contexts.write().await;
        contexts.insert(context_id.clone(), context);

        context_id
    }

    /// Accelerate consensus proof validation using GPU
    pub(crate) async fn accelerate_consensus_validation(&self, proof: &ConsensusProof) -> AssetResult<bool> {
        Ok(proof.validate())
    }

    /// Update usage statistics
    pub(crate) async fn update_usage_stats(&self, operation: GpuOperation, _devices: u32, memory_bytes: u64) {
        let mut stats = self.usage_stats.write().await;

        match operation {
            GpuOperation::Allocate => {
                stats.total_allocations += 1;
                stats.active_allocations += 1;
                stats.total_memory_allocated += memory_bytes;
            },
            GpuOperation::Deallocate => {
                stats.total_deallocations += 1;
                stats.active_allocations = stats.active_allocations.saturating_sub(1);
                stats.total_memory_allocated = stats.total_memory_allocated.saturating_sub(memory_bytes);
            },
            GpuOperation::Compute => {
                stats.compute_operations += 1;
            },
            GpuOperation::MemoryTransfer => {
                stats.memory_transfers += 1;
            },
        }
    }
}
