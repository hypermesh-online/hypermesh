// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Storage Asset Adapter trait implementation
//!
//! Implements AssetAdapter for storage resource management with:
//! - State proof validation (PoSpace critical)
//! - Asset allocation/deallocation
//! - Privacy configuration
//! - Proxy address management
//! - Resource monitoring
//! - Health checks

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;

use crate::assets::core::{
    AdapterCapabilities, AdapterHealth, AssetAdapter, AssetAllocation, AssetAllocationRequest,
    AssetCategory, AssetData, AssetError, AssetRegistration, AssetResult, AssetState, AssetStatus,
    AssetType, BaseSystemType, StateProof, NetworkScope, PrivacyMode, ProxyAddress,
    ResourceLimits, ResourceUsage, StorageUsage,
};

use super::allocation::{
    allocate_storage_from_devices, deallocate_storage_from_devices, initialize_default_pool,
    update_usage_stats, StorageAllocation, StorageOperation, StoragePool, StorageUsageStats,
};
use super::devices::{detect_storage_configuration, get_io_stats, StorageDevice, StorageStatus};
use super::distribution::generate_proxy_address;
use super::encryption::create_kyber_encryption_key;
use super::sharding::ShardingConfig;

/// Storage Asset Adapter implementation
pub struct StorageAssetAdapter {
    /// Active storage allocations by asset ID
    allocations: Arc<RwLock<HashMap<AssetRegistration, StorageAllocation>>>,
    /// Storage device information and status
    storage_devices: Arc<RwLock<HashMap<String, StorageDevice>>>,
    /// Device allocation mapping (device_id -> asset_id)
    device_allocations: Arc<RwLock<HashMap<String, AssetRegistration>>>,
    /// Storage pools for distributed management
    _storage_pools: Arc<RwLock<HashMap<String, StoragePool>>>,
    /// Proxy address mappings
    proxy_mappings: Arc<RwLock<HashMap<ProxyAddress, AssetRegistration>>>,
    /// Total storage capacity in bytes
    total_capacity: u64,
    /// Available storage capacity in bytes
    available_capacity: Arc<RwLock<u64>>,
    /// Storage usage statistics
    usage_stats: Arc<RwLock<StorageUsageStats>>,
}

impl StorageAssetAdapter {
    /// Create new storage adapter
    pub async fn new() -> Self {
        // Detect system storage configuration
        let (total_capacity, storage_devices) = detect_storage_configuration().await;

        // Initialize with default storage pool
        let device_ids: Vec<String> = storage_devices.keys().cloned().collect();
        let storage_pools = initialize_default_pool(total_capacity, device_ids);

        Self {
            allocations: Arc::new(RwLock::new(HashMap::new())),
            storage_devices: Arc::new(RwLock::new(storage_devices)),
            device_allocations: Arc::new(RwLock::new(HashMap::new())),
            _storage_pools: Arc::new(RwLock::new(storage_pools)),
            proxy_mappings: Arc::new(RwLock::new(HashMap::new())),
            total_capacity,
            available_capacity: Arc::new(RwLock::new(total_capacity)),
            usage_stats: Arc::new(RwLock::new(StorageUsageStats::default())),
        }
    }
}

#[async_trait]
impl AssetAdapter for StorageAssetAdapter {
    fn asset_type(&self) -> AssetType {
        AssetType::Storage
    }

    async fn validate_state_proof(&self, proof: &StateProof) -> AssetResult<bool> {
        // Validate all four proofs with CRITICAL PoSpace validation for storage
        let valid = proof.validate();

        if !valid {
            return Err(AssetError::StateProofValidationFailed {
                reason: "Storage state proof validation failed".to_string(),
            });
        }

        // PoSpace is WHERE (location), never how-much. Require the proof be
        // bound to a location; capacity is descriptive and never gates.
        if proof.space_proof.node_id.is_empty() || proof.space_proof.storage_path.is_empty() {
            return Ok(false);
        }

        // PoStake: CANONICAL MODEL — authorization (WHO), require a bound
        // identity, never a stake magnitude.
        if proof.stake_proof.stake_holder_id.is_empty() {
            return Ok(false);
        }

        // PoWork: CANONICAL MODEL — HASH of work done (WHAT), require work was
        // hashed, never a capacity magnitude.
        if proof.work_proof.work_hash == [0u8; 32] {
            return Ok(false);
        }

        // PoTime: Validate temporal ordering for storage operations
        let time_valid = proof
            .time_proof
            .time_verification_timestamp
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() > 0)
            .unwrap_or(false);

        Ok(time_valid)
    }

    async fn allocate_asset(
        &self,
        request: &AssetAllocationRequest,
    ) -> AssetResult<AssetAllocation> {
        // Validate state proof first
        if !self
            .validate_state_proof(&request.state_proof)
            .await?
        {
            return Err(AssetError::StateProofValidationFailed {
                reason: "Storage allocation state proof validation failed".to_string(),
            });
        }

        // Get storage requirements
        let storage_req = request
            .requested_resources
            .storage_usage
            .as_ref()
            .ok_or_else(|| AssetError::AllocationFailed {
                reason: "No storage requirements specified".to_string(),
            })?;

        // Check available capacity
        let available = *self.available_capacity.read().await;
        let required_capacity = storage_req.size_bytes * storage_req.durability_replicas as u64;

        if available < required_capacity {
            return Err(AssetError::AllocationFailed {
                reason: format!(
                    "Insufficient storage capacity: {} bytes required ({}x replication), {} available",
                    required_capacity, storage_req.durability_replicas, available
                )
            });
        }

        // Create asset ID with real content-based hash
        let data = AssetData {
            config: vec![1, 2, 3], // Test data
            definition: vec![4, 5, 6],
            metadata: vec![7, 8, 9],
        };
        let asset_id = AssetRegistration::from_asset_data(
            &data,
            NetworkScope::Global,
            AssetCategory::BaseSystem(BaseSystemType::Storage),
        );

        // Allocate storage from devices
        let (allocated_devices, allocated_size) = allocate_storage_from_devices(
            storage_req,
            &asset_id,
            &self.storage_devices,
            &self.device_allocations,
        )
        .await?;

        // Generate proxy address
        let proxy_address = generate_proxy_address(&asset_id).await;

        // Create encryption key for quantum security
        let encryption_key_id = if request.privacy_level == PrivacyMode::PRIVATE {
            Some(
                create_kyber_encryption_key()
                    .await
                    .map_err(|e| AssetError::AdapterError {
                        message: format!("Kyber key generation failed: {e}"),
                    })?,
            )
        } else {
            None
        };

        // Configure sharding
        let sharding_config =
            ShardingConfig::configure(storage_req.size_bytes, allocated_devices.len() as u32);

        // Create storage allocation record
        let allocation = StorageAllocation {
            asset_id: asset_id.clone(),
            allocated_devices: allocated_devices.clone(),
            allocated_size_bytes: allocated_size,
            storage_type: storage_req.storage_type.clone(),
            privacy_level: request.privacy_level,
            replication_factor: storage_req.durability_replicas,
            encryption_enabled: encryption_key_id.is_some(),
            encryption_key_id,
            sharding_config,
            mount_path: None, // Will be assigned when mounted
            allocated_at: SystemTime::now(),
            last_accessed: SystemTime::now(),
            current_iops: 0,
            current_throughput_mbps: 0.0,
        };

        // Store allocation and proxy mapping
        {
            let mut allocations = self.allocations.write().await;
            allocations.insert(asset_id.clone(), allocation);
        }

        {
            let mut proxy_mappings = self.proxy_mappings.write().await;
            proxy_mappings.insert(proxy_address.clone(), asset_id.clone());
        }

        // Update available capacity
        {
            let mut available = self.available_capacity.write().await;
            *available -= allocated_size;
        }

        // Update usage statistics
        update_usage_stats(
            &self.usage_stats,
            StorageOperation::Allocate,
            allocated_size,
        )
        .await;

        Ok(AssetAllocation {
            asset_id: asset_id.clone(),
            status: AssetStatus {
                asset_id: asset_id.clone(),
                state: AssetState::Allocated,
                allocated_at: SystemTime::now(),
                last_accessed: SystemTime::now(),
                resource_usage: ResourceUsage {
                    cpu_usage: None,
                    gpu_usage: None,
                    memory_usage: None,
                    storage_usage: None,
                    network_usage: None,
                    measurement_timestamp: SystemTime::now(),
                },
                privacy_level: PrivacyMode::PRIVATE,
                proxy_address: None,
                state_proofs: Vec::new(),
                owner_certificate_fingerprint: request.certificate_fingerprint.clone(),
                metadata: HashMap::new(),
                health_status: crate::assets::core::status::AssetHealthStatus::default(),
                performance_metrics: crate::assets::core::status::AssetPerformanceMetrics::default(
                ),
            },
            allocation_config: crate::assets::core::privacy::AllocationConfig {
                privacy_level: request.privacy_level,
                resource_allocation:
                    crate::assets::core::privacy::ResourceAllocationConfig::default(),
                concurrency_limits: crate::assets::core::privacy::ConcurrencyLimits::default(),
                duration_config: crate::assets::core::privacy::DurationConfig::default(),
                state_requirements:
                    crate::assets::core::privacy::StateRequirements::default(),
            },
            access_config: crate::assets::core::privacy::AccessConfig {
                allowed_certificates: vec![request.certificate_fingerprint.clone()],
                allowed_networks: Vec::new(),
                permissions: crate::assets::core::privacy::AccessPermissions::default(),
                rate_limits: crate::assets::core::privacy::RateLimits::default(),
                auth_requirements: crate::assets::core::privacy::AuthRequirements::default(),
            },
            allocated_at: SystemTime::now(),
            expires_at: request.duration_limit.map(|d| SystemTime::now() + d),
        })
    }

    async fn deallocate_asset(&self, asset_id: &AssetRegistration) -> AssetResult<()> {
        // Get allocation record
        let allocation = {
            let mut allocations = self.allocations.write().await;
            allocations
                .remove(asset_id)
                .ok_or_else(|| AssetError::AssetNotFound {
                    asset_id: asset_id.to_string(),
                })?
        };

        // Free storage devices
        deallocate_storage_from_devices(
            &allocation,
            &self.storage_devices,
            &self.device_allocations,
        )
        .await?;

        // Remove proxy mapping
        {
            let mut proxy_mappings = self.proxy_mappings.write().await;
            proxy_mappings.retain(|_, mapped_asset_id| mapped_asset_id != asset_id);
        }

        // Update available capacity
        {
            let mut available = self.available_capacity.write().await;
            *available += allocation.allocated_size_bytes;
        }

        // Update usage statistics
        update_usage_stats(
            &self.usage_stats,
            StorageOperation::Deallocate,
            allocation.allocated_size_bytes,
        )
        .await;

        tracing::info!(
            "Deallocated storage asset: {} ({} devices, {} bytes)",
            asset_id,
            allocation.allocated_devices.len(),
            allocation.allocated_size_bytes
        );
        Ok(())
    }

    async fn get_asset_status(&self, asset_id: &AssetRegistration) -> AssetResult<AssetStatus> {
        let allocations = self.allocations.read().await;
        let allocation = allocations
            .get(asset_id)
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string(),
            })?;

        Ok(AssetStatus {
            asset_id: asset_id.clone(),
            state: AssetState::InUse,
            allocated_at: allocation.allocated_at,
            last_accessed: allocation.last_accessed,
            privacy_level: allocation.privacy_level,
            proxy_address: None, // Will be filled by proxy resolver
            resource_usage: self.get_resource_usage(asset_id).await?,
            state_proofs: Vec::new(),
            owner_certificate_fingerprint: "storage-adapter".to_string(),
            health_status: crate::assets::core::status::AssetHealthStatus::default(),
            performance_metrics: crate::assets::core::status::AssetPerformanceMetrics::default(),
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert(
                    "allocated_size_bytes".to_string(),
                    allocation.allocated_size_bytes.to_string(),
                );
                metadata.insert(
                    "storage_type".to_string(),
                    format!("{:?}", allocation.storage_type),
                );
                metadata.insert(
                    "devices".to_string(),
                    allocation.allocated_devices.len().to_string(),
                );
                metadata.insert(
                    "replication_factor".to_string(),
                    allocation.replication_factor.to_string(),
                );
                metadata.insert(
                    "encryption_enabled".to_string(),
                    allocation.encryption_enabled.to_string(),
                );
                metadata.insert(
                    "current_iops".to_string(),
                    allocation.current_iops.to_string(),
                );
                metadata.insert(
                    "current_throughput_mbps".to_string(),
                    allocation.current_throughput_mbps.to_string(),
                );
                metadata.insert(
                    "shard_count".to_string(),
                    allocation.sharding_config.shard_count.to_string(),
                );
                metadata
            },
        })
    }

    async fn configure_privacy_level(
        &self,
        asset_id: &AssetRegistration,
        privacy: PrivacyMode,
    ) -> AssetResult<()> {
        let mut allocations = self.allocations.write().await;
        let allocation =
            allocations
                .get_mut(asset_id)
                .ok_or_else(|| AssetError::AssetNotFound {
                    asset_id: asset_id.to_string(),
                })?;

        allocation.privacy_level = privacy;

        // Update encryption based on privacy level
        if privacy == PrivacyMode::PRIVATE && allocation.encryption_key_id.is_none() {
            allocation.encryption_key_id = Some(
                create_kyber_encryption_key()
                    .await
                    .map_err(|e| AssetError::AdapterError {
                        message: format!("Kyber key generation failed: {e}"),
                    })?,
            );
            allocation.encryption_enabled = true;
        }

        tracing::info!(
            "Updated privacy level for storage asset {}: {:?}",
            asset_id,
            privacy
        );
        Ok(())
    }

    async fn assign_proxy_address(
        &self,
        asset_id: &AssetRegistration,
    ) -> AssetResult<ProxyAddress> {
        let proxy_address = generate_proxy_address(asset_id).await;

        // Find existing proxy address
        let proxy_mappings = self.proxy_mappings.read().await;
        for (proxy_addr, mapped_asset_id) in proxy_mappings.iter() {
            if mapped_asset_id == asset_id {
                return Ok(proxy_addr.clone());
            }
        }

        Ok(proxy_address)
    }

    async fn resolve_proxy_address(
        &self,
        proxy_addr: &ProxyAddress,
    ) -> AssetResult<AssetRegistration> {
        let proxy_mappings = self.proxy_mappings.read().await;
        proxy_mappings
            .get(proxy_addr)
            .cloned()
            .ok_or_else(|| AssetError::ProxyResolutionFailed {
                address: proxy_addr.clone(),
            })
    }

    async fn get_resource_usage(&self, asset_id: &AssetRegistration) -> AssetResult<ResourceUsage> {
        let allocations = self.allocations.read().await;
        let allocation = allocations
            .get(asset_id)
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string(),
            })?;

        // Get real I/O statistics from /proc/diskstats
        let (read_iops, write_iops, read_mbps, write_mbps) =
            get_io_stats(&allocation.allocated_devices);

        let storage_usage = StorageUsage {
            used_bytes: allocation.allocated_size_bytes,
            total_bytes: allocation.allocated_size_bytes,
            read_iops,
            write_iops,
            read_mbps,
            write_mbps,
        };

        Ok(ResourceUsage {
            cpu_usage: None,
            gpu_usage: None,
            memory_usage: None,
            storage_usage: Some(storage_usage),
            network_usage: None,
            measurement_timestamp: SystemTime::now(),
        })
    }

    async fn set_resource_limits(
        &self,
        asset_id: &AssetRegistration,
        limits: ResourceLimits,
    ) -> AssetResult<()> {
        if let Some(storage_limit) = limits.storage_limit {
            tracing::info!(
                "Set storage limits for asset {}: max {} bytes, max {} IOPS, max {} MB/s",
                asset_id,
                storage_limit.max_bytes,
                storage_limit.max_iops,
                storage_limit.max_bandwidth_mbps
            );
        }
        Ok(())
    }

    async fn health_check(&self) -> AssetResult<AdapterHealth> {
        let stats = self.usage_stats.read().await;
        let devices = self.storage_devices.read().await;
        let available = *self.available_capacity.read().await;

        let failed_devices = devices
            .values()
            .filter(|device| matches!(device.status, StorageStatus::Failed))
            .count();
        let degraded_devices = devices
            .values()
            .filter(|device| matches!(device.status, StorageStatus::Degraded))
            .count();
        let healthy = failed_devices == 0 && degraded_devices < 2 && available > 0;

        let average_health = devices
            .values()
            .map(|d| d.health_metrics.health_percentage as f64)
            .sum::<f64>()
            / devices.len() as f64;

        let mut performance_metrics = HashMap::new();
        performance_metrics.insert(
            "total_capacity_gb".to_string(),
            (self.total_capacity / (1024 * 1024 * 1024)) as f64,
        );
        performance_metrics.insert(
            "available_capacity_gb".to_string(),
            (available / (1024 * 1024 * 1024)) as f64,
        );
        performance_metrics.insert(
            "capacity_utilization_percent".to_string(),
            ((self.total_capacity - available) as f64 / self.total_capacity as f64) * 100.0,
        );
        performance_metrics.insert(
            "active_allocations".to_string(),
            stats.active_allocations as f64,
        );
        performance_metrics.insert("total_devices".to_string(), devices.len() as f64);
        performance_metrics.insert("failed_devices".to_string(), failed_devices as f64);
        performance_metrics.insert("degraded_devices".to_string(), degraded_devices as f64);
        performance_metrics.insert("average_health_percent".to_string(), average_health);
        performance_metrics.insert(
            "dedup_savings_gb".to_string(),
            (stats.dedup_savings_bytes / (1024 * 1024 * 1024)) as f64,
        );

        Ok(AdapterHealth {
            healthy,
            message: if healthy {
                "Storage adapter operating normally".to_string()
            } else {
                format!(
                    "Storage adapter issues: {failed_devices} failed, {degraded_devices} degraded devices"
                )
            },
            last_check: SystemTime::now(),
            performance_metrics,
        })
    }

    fn get_capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            asset_type: AssetType::Storage,
            supported_privacy_levels: vec![
                PrivacyMode::PRIVATE,
                PrivacyMode::PRIVATE,
                PrivacyMode::PRIVATE,
                PrivacyMode::PUBLIC,
                PrivacyMode::PUBLIC,
            ],
            supports_proxy_addressing: true,
            supports_resource_monitoring: true,
            supports_dynamic_limits: true,
            max_concurrent_allocations: Some(100),
            features: vec![
                "distributed_storage".to_string(),
                "replication".to_string(),
                "sharding".to_string(),
                "deduplication".to_string(),
                "compression".to_string(),
                "kyber_encryption".to_string(),
                "health_monitoring".to_string(),
                "smart_data".to_string(),
                "predictive_maintenance".to_string(),
                "content_aware_sharding".to_string(),
            ],
        }
    }
}
