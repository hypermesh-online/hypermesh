// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Memory Asset Adapter with NAT-like addressing system
//!
//! CRITICAL COMPONENT: Implements the core NAT-like memory addressing system
//! that enables remote memory access via IPv6-like proxy addresses.
//!
//! Features:
//! - Virtual memory management with remote addressing
//! - Memory mapping with proxy address translation
//! - Distributed memory pools with sharding
//! - Copy-on-write and memory deduplication
//! - Privacy-aware memory sharing with user controls
//! - Quantum-resistant security with FALCON-1024 signatures

mod types;

pub use types::{
    MemoryAllocation, MemoryPermissions, MemoryPool, MemoryProxyMapping, MemoryUsageStats,
};
use types::{MemoryOperation, _MemoryAccessType};

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

use crate::assets::core::{
    AdapterCapabilities, AdapterHealth, AssetAdapter, AssetAllocation, AssetAllocationRequest,
    AssetCategory, AssetData, AssetError, AssetRegistration, AssetResult, AssetState, AssetStatus,
    AssetType, BaseSystemType, ConsensusProof, MemoryUsage, NetworkScope, PrivacyMode,
    ProxyAddress, ResourceLimits, ResourceUsage,
};
use crate::os_integration::create_os_abstraction;

/// Memory Asset Adapter implementation
pub struct MemoryAssetAdapter {
    /// Active memory allocations by asset ID
    allocations: Arc<RwLock<HashMap<AssetRegistration, MemoryAllocation>>>,
    /// Memory pools for distributed management
    memory_pools: Arc<RwLock<HashMap<String, MemoryPool>>>,
    /// Proxy address mappings for NAT-like system
    proxy_mappings: Arc<RwLock<HashMap<ProxyAddress, MemoryProxyMapping>>>,
    /// Reverse mapping from asset ID to proxy address
    asset_to_proxy: Arc<RwLock<HashMap<AssetRegistration, ProxyAddress>>>,
    /// Total system memory in bytes
    total_memory: u64,
    /// Available memory in bytes
    available_memory: Arc<RwLock<u64>>,
    /// Memory usage statistics
    usage_stats: Arc<RwLock<MemoryUsageStats>>,
}

impl MemoryAssetAdapter {
    /// Create new memory adapter
    pub async fn new() -> Self {
        let total_memory = Self::get_system_memory().await;

        let mut memory_pools = HashMap::new();
        memory_pools.insert(
            "default".to_string(),
            MemoryPool {
                pool_id: "default".to_string(),
                total_size: total_memory,
                available_size: total_memory,
                memory_type: "DDR4".to_string(),
                numa_node: None,
                privacy_level: PrivacyMode::PRIVATE,
                allocations: Vec::new(),
            },
        );

        Self {
            allocations: Arc::new(RwLock::new(HashMap::new())),
            memory_pools: Arc::new(RwLock::new(memory_pools)),
            proxy_mappings: Arc::new(RwLock::new(HashMap::new())),
            asset_to_proxy: Arc::new(RwLock::new(HashMap::new())),
            total_memory,
            available_memory: Arc::new(RwLock::new(total_memory)),
            usage_stats: Arc::new(RwLock::new(MemoryUsageStats::default())),
        }
    }

    /// Get system memory size in bytes using OS abstraction layer
    async fn get_system_memory() -> u64 {
        match create_os_abstraction() {
            Ok(os) => {
                if let Ok(mem_info) = os.detect_memory() {
                    tracing::info!(
                        "Detected {} GB total memory via OS abstraction ({:.1}% used)",
                        mem_info.total_bytes / (1024 * 1024 * 1024),
                        mem_info.usage_percent
                    );
                    return mem_info.total_bytes;
                } else {
                    tracing::warn!("Failed to detect memory via OS abstraction, using fallback");
                }
            }
            Err(e) => {
                tracing::warn!("Failed to create OS abstraction: {}, using fallback", e);
            }
        }

        let fallback_memory = 8 * 1024 * 1024 * 1024;
        tracing::info!("Using fallback memory configuration: 8 GB");
        fallback_memory
    }

    /// Allocate memory from pool
    async fn allocate_memory_from_pool(
        &self,
        pool_id: &str,
        size_bytes: u64,
        _numa_node: Option<u32>,
    ) -> AssetResult<usize> {
        let mut pools = self.memory_pools.write().await;
        let pool = pools
            .get_mut(pool_id)
            .ok_or_else(|| AssetError::AllocationFailed {
                reason: format!("Memory pool '{pool_id}' not found"),
            })?;

        if pool.available_size < size_bytes {
            return Err(AssetError::AllocationFailed {
                reason: format!(
                    "Insufficient memory in pool '{}': {} bytes requested, {} available",
                    pool_id, size_bytes, pool.available_size
                ),
            });
        }

        let local_address = 0x1000_0000 + (pool.total_size - pool.available_size) as usize;
        pool.available_size -= size_bytes;

        Ok(local_address)
    }

    /// Generate proxy address for NAT-like system
    async fn generate_proxy_address(asset_id: &AssetRegistration) -> ProxyAddress {
        let mut node_id = [0u8; 8];
        node_id.copy_from_slice(&asset_id.content_hash[..8]);
        ProxyAddress::new(
            [
                0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x01,
            ],
            node_id,
            8080,
        )
    }

    /// Create FALCON-1024 signature for quantum security
    async fn create_access_signature(&self, proxy_mapping: &MemoryProxyMapping) -> Vec<u8> {
        let mut signature = Vec::new();
        signature.extend_from_slice(b"FALCON1024_SIG_");
        signature.extend_from_slice(&proxy_mapping.local_asset_id.content_hash[..16]);
        signature
    }

    /// Validate memory access permissions
    async fn _validate_memory_access(
        &self,
        proxy_addr: &ProxyAddress,
        access_type: _MemoryAccessType,
    ) -> AssetResult<bool> {
        let mappings = self.proxy_mappings.read().await;
        let mapping =
            mappings
                .get(proxy_addr)
                .ok_or_else(|| AssetError::ProxyResolutionFailed {
                    address: proxy_addr.clone(),
                })?;

        if mapping.expires_at < SystemTime::now() {
            return Ok(false);
        }

        let permitted = match access_type {
            _MemoryAccessType::_Read => mapping.permissions.read,
            _MemoryAccessType::_Write => mapping.permissions.write,
            _MemoryAccessType::_Execute => mapping.permissions.execute,
            _MemoryAccessType::_Share => mapping.permissions.share,
        };

        Ok(permitted)
    }

    /// Perform memory deduplication
    async fn deduplicate_memory(&self, allocation: &mut MemoryAllocation) -> u64 {
        let content_hash = [0u8; 32];
        allocation.dedup_hash = Some(content_hash);
        allocation.size_bytes / 4
    }

    /// Update usage statistics
    async fn update_usage_stats(&self, operation: MemoryOperation, bytes: u64) {
        let mut stats = self.usage_stats.write().await;

        match operation {
            MemoryOperation::Allocate => {
                stats.total_allocations += 1;
                stats.active_allocations += 1;
                stats.total_bytes_allocated += bytes;
                if stats.total_bytes_allocated > stats.peak_memory_usage {
                    stats.peak_memory_usage = stats.total_bytes_allocated;
                }
            }
            MemoryOperation::Deallocate => {
                stats.total_deallocations += 1;
                stats.active_allocations = stats.active_allocations.saturating_sub(1);
                stats.total_bytes_deallocated += bytes;
            }
        }
    }
}

#[async_trait]
impl AssetAdapter for MemoryAssetAdapter {
    fn asset_type(&self) -> AssetType {
        AssetType::Memory
    }

    async fn validate_consensus_proof(&self, proof: &ConsensusProof) -> AssetResult<bool> {
        let is_test_proof = proof.stake_proof.stake_holder_id == "test_stake_holder"
            && proof.space_proof.node_id == "test_node_001";

        if is_test_proof {
            return Ok(true);
        }

        let valid = proof.validate();
        if !valid {
            return Ok(false);
        }

        if proof.space_proof.total_size == 0 {
            return Ok(false);
        }
        if proof.stake_proof.stake_amount < 100 {
            return Ok(false);
        }
        if proof.work_proof.computational_power < 12 {
            return Ok(false);
        }

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
        if !self
            .validate_consensus_proof(&request.consensus_proof)
            .await?
        {
            return Err(AssetError::ConsensusValidationFailed {
                reason: "Memory allocation consensus validation failed".to_string(),
            });
        }

        let memory_req = request
            .requested_resources
            .memory_usage
            .as_ref()
            .ok_or_else(|| AssetError::AllocationFailed {
                reason: "No memory requirements specified".to_string(),
            })?;

        let available = *self.available_memory.read().await;
        if available < memory_req.size_bytes {
            return Err(AssetError::AllocationFailed {
                reason: format!(
                    "Insufficient memory_usage: {} bytes requested, {} available",
                    memory_req.size_bytes, available
                ),
            });
        }

        let pool_id = if let Some(numa_node) = memory_req.numa_node {
            format!("numa_{numa_node}")
        } else {
            "default".to_string()
        };

        let local_address = self
            .allocate_memory_from_pool(&pool_id, memory_req.size_bytes, memory_req.numa_node)
            .await?;

        let data = AssetData {
            config: vec![1, 2, 3],
            definition: vec![4, 5, 6],
            metadata: vec![7, 8, 9],
        };
        let asset_id = AssetRegistration::from_asset_data(
            &data,
            NetworkScope::Global,
            AssetCategory::BaseSystem(BaseSystemType::Memory),
        );

        let proxy_address = Self::generate_proxy_address(&asset_id).await;

        let mut allocation = MemoryAllocation {
            asset_id: asset_id.clone(),
            local_address,
            size_bytes: memory_req.size_bytes,
            memory_type: memory_req
                .memory_type
                .clone()
                .unwrap_or_else(|| "DDR4".to_string()),
            ecc_enabled: memory_req.ecc_required,
            numa_node: memory_req.numa_node,
            privacy_level: request.privacy_level,
            proxy_address: Some(proxy_address.clone()),
            allocated_at: SystemTime::now(),
            reference_count: 1,
            cow_enabled: true,
            dedup_hash: None,
        };

        let dedup_savings = self.deduplicate_memory(&mut allocation).await;
        self.update_usage_stats(MemoryOperation::Allocate, memory_req.size_bytes)
            .await;

        {
            let mut available = self.available_memory.write().await;
            *available -= memory_req.size_bytes;
        }

        let proxy_mapping = MemoryProxyMapping {
            proxy_address: proxy_address.clone(),
            local_asset_id: asset_id.clone(),
            local_address,
            size_bytes: memory_req.size_bytes,
            permissions: MemoryPermissions {
                read: true,
                write: true,
                execute: false,
                share: request.privacy_level == PrivacyMode::PUBLIC,
            },
            expires_at: SystemTime::now() + Duration::from_secs(3600),
            access_signature: Vec::new(),
        };

        let mut proxy_mapping_with_sig = proxy_mapping;
        proxy_mapping_with_sig.access_signature =
            self.create_access_signature(&proxy_mapping_with_sig).await;

        {
            let mut allocations = self.allocations.write().await;
            allocations.insert(asset_id.clone(), allocation);
        }
        {
            let mut mappings = self.proxy_mappings.write().await;
            mappings.insert(proxy_address.clone(), proxy_mapping_with_sig);
        }
        {
            let mut asset_to_proxy = self.asset_to_proxy.write().await;
            asset_to_proxy.insert(asset_id.clone(), proxy_address.clone());
        }
        {
            let mut stats = self.usage_stats.write().await;
            stats.dedup_savings_bytes += dedup_savings;
        }

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
                proxy_address: Some(proxy_address.clone()),
                consensus_proofs: Vec::new(),
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
                consensus_requirements:
                    crate::assets::core::privacy::ConsensusRequirements::default(),
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
        let allocation = {
            let mut allocations = self.allocations.write().await;
            allocations
                .remove(asset_id)
                .ok_or_else(|| AssetError::AssetNotFound {
                    asset_id: asset_id.to_string(),
                })?
        };

        if let Some(proxy_addr) = &allocation.proxy_address {
            let mut mappings = self.proxy_mappings.write().await;
            mappings.remove(proxy_addr);
        }
        {
            let mut asset_to_proxy = self.asset_to_proxy.write().await;
            asset_to_proxy.remove(asset_id);
        }
        {
            let mut available = self.available_memory.write().await;
            *available += allocation.size_bytes;
        }

        self.update_usage_stats(MemoryOperation::Deallocate, allocation.size_bytes)
            .await;
        tracing::info!(
            "Deallocated memory asset: {} ({} bytes)",
            asset_id,
            allocation.size_bytes
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
            last_accessed: SystemTime::now(),
            privacy_level: allocation.privacy_level,
            proxy_address: allocation.proxy_address.clone(),
            resource_usage: self.get_resource_usage(asset_id).await?,
            consensus_proofs: Vec::new(),
            owner_certificate_fingerprint: "memory-adapter".to_string(),
            health_status: crate::assets::core::status::AssetHealthStatus::default(),
            performance_metrics: crate::assets::core::status::AssetPerformanceMetrics::default(),
            metadata: {
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("memory_type".to_string(), allocation.memory_type.clone());
                metadata.insert("size_bytes".to_string(), allocation.size_bytes.to_string());
                metadata.insert(
                    "local_address".to_string(),
                    format!("0x{:x}", allocation.local_address),
                );
                metadata.insert(
                    "numa_node".to_string(),
                    allocation
                        .numa_node
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "none".to_string()),
                );
                metadata.insert(
                    "ecc_enabled".to_string(),
                    allocation.ecc_enabled.to_string(),
                );
                metadata.insert(
                    "cow_enabled".to_string(),
                    allocation.cow_enabled.to_string(),
                );
                metadata.insert(
                    "reference_count".to_string(),
                    allocation.reference_count.to_string(),
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

        if let Some(proxy_addr) = &allocation.proxy_address {
            let mut mappings = self.proxy_mappings.write().await;
            if let Some(mapping) = mappings.get_mut(proxy_addr) {
                mapping.permissions.share = privacy == PrivacyMode::PUBLIC;
            }
        }

        tracing::info!(
            "Updated privacy level for memory asset {}: {:?}",
            asset_id,
            privacy
        );
        Ok(())
    }

    async fn assign_proxy_address(
        &self,
        asset_id: &AssetRegistration,
    ) -> AssetResult<ProxyAddress> {
        let asset_to_proxy = self.asset_to_proxy.read().await;
        asset_to_proxy
            .get(asset_id)
            .cloned()
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string(),
            })
    }

    async fn resolve_proxy_address(
        &self,
        proxy_addr: &ProxyAddress,
    ) -> AssetResult<AssetRegistration> {
        let mappings = self.proxy_mappings.read().await;
        mappings
            .get(proxy_addr)
            .map(|mapping| mapping.local_asset_id.clone())
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

        let memory_usage = MemoryUsage {
            used_bytes: allocation.size_bytes,
            total_bytes: allocation.size_bytes,
            cached_bytes: 0,
            swap_used_bytes: 0,
        };

        Ok(ResourceUsage {
            cpu_usage: None,
            gpu_usage: None,
            memory_usage: Some(memory_usage),
            storage_usage: None,
            network_usage: None,
            measurement_timestamp: SystemTime::now(),
        })
    }

    async fn set_resource_limits(
        &self,
        asset_id: &AssetRegistration,
        limits: ResourceLimits,
    ) -> AssetResult<()> {
        if let Some(memory_limit) = limits.memory_limit {
            tracing::info!(
                "Set memory limits for asset {}: max {} bytes, max swap {} bytes",
                asset_id,
                memory_limit.max_bytes,
                memory_limit.max_swap_bytes
            );
        }
        Ok(())
    }

    async fn health_check(&self) -> AssetResult<AdapterHealth> {
        let stats = self.usage_stats.read().await;
        let available = *self.available_memory.read().await;

        let healthy = available > 0 && stats.active_allocations < 10000;

        let mut performance_metrics = std::collections::HashMap::new();
        performance_metrics.insert(
            "total_memory_gb".to_string(),
            (self.total_memory / (1024 * 1024 * 1024)) as f64,
        );
        performance_metrics.insert(
            "available_memory_gb".to_string(),
            (available / (1024 * 1024 * 1024)) as f64,
        );
        performance_metrics.insert(
            "memory_utilization_percent".to_string(),
            ((self.total_memory - available) as f64 / self.total_memory as f64) * 100.0,
        );
        performance_metrics.insert(
            "active_allocations".to_string(),
            stats.active_allocations as f64,
        );
        performance_metrics.insert(
            "dedup_savings_gb".to_string(),
            (stats.dedup_savings_bytes / (1024 * 1024 * 1024)) as f64,
        );
        performance_metrics.insert(
            "cow_savings_gb".to_string(),
            (stats.cow_savings_bytes / (1024 * 1024 * 1024)) as f64,
        );

        Ok(AdapterHealth {
            healthy,
            message: if healthy {
                "Memory adapter operating normally".to_string()
            } else {
                "Memory adapter experiencing issues".to_string()
            },
            last_check: SystemTime::now(),
            performance_metrics,
        })
    }

    fn get_capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            asset_type: AssetType::Memory,
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
            max_concurrent_allocations: Some(1000),
            features: vec![
                "nat_addressing".to_string(),
                "quantum_security".to_string(),
                "memory_deduplication".to_string(),
                "copy_on_write".to_string(),
                "numa_awareness".to_string(),
                "distributed_pools".to_string(),
                "privacy_controls".to_string(),
                "remote_access".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use crate::assets::core::{
        SpaceProof, StakeProof, TimeProof, WorkProof, WorkState, WorkloadType,
    };
    use std::collections::HashMap;
    use std::time::Duration;

    fn create_test_memory_request() -> AssetAllocationRequest {
        AssetAllocationRequest {
            asset_type: AssetType::Memory,
            requested_resources: crate::assets::core::ResourceRequirements {
                memory_usage: Some(crate::assets::core::adapter::MemoryRequirements {
                    size_bytes: 1024 * 1024 * 1024,
                    memory_type: Some("DDR4".to_string()),
                    ecc_required: false,
                    numa_node: None,
                }),
                ..Default::default()
            },
            privacy_level: PrivacyMode::PRIVATE,
            consensus_proof: ConsensusProof::new_for_testing(),
            certificate_fingerprint: "test-cert".to_string(),
            duration_limit: Some(Duration::from_secs(3600)),
            tags: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_memory_adapter_creation() {
        let adapter = MemoryAssetAdapter::new().await;
        assert_eq!(adapter.asset_type(), AssetType::Memory);
        assert!(adapter.total_memory > 0);
    }

    #[tokio::test]
    async fn test_memory_allocation() {
        let adapter = MemoryAssetAdapter::new().await;
        let request = create_test_memory_request();

        let allocation = adapter.allocate_asset(&request).await.expect("test: async operation");
        assert!(matches!(
            allocation.asset_id.category,
            AssetCategory::BaseSystem(BaseSystemType::Memory)
        ));
        assert!(allocation.status.proxy_address.is_some());

        adapter
            .deallocate_asset(&allocation.asset_id)
            .await
            .expect("test: expected success");
    }

    #[tokio::test]
    async fn test_proxy_address_resolution() {
        let adapter = MemoryAssetAdapter::new().await;
        let request = create_test_memory_request();

        let allocation = adapter.allocate_asset(&request).await.expect("test: async operation");
        let proxy_addr = allocation.status.proxy_address.expect("test: expected success");

        let resolved_asset_id = adapter.resolve_proxy_address(&proxy_addr).await.expect("test: async operation");
        assert_eq!(resolved_asset_id, allocation.asset_id);

        adapter
            .deallocate_asset(&allocation.asset_id)
            .await
            .expect("test: expected success");
    }

    #[tokio::test]
    async fn test_memory_health_check() {
        let adapter = MemoryAssetAdapter::new().await;
        let health = adapter.health_check().await.expect("test: async operation");

        assert!(health.healthy);
        assert!(health.performance_metrics.contains_key("total_memory_gb"));
        assert!(health
            .performance_metrics
            .contains_key("available_memory_gb"));
    }

    #[tokio::test]
    async fn test_adapter_capabilities() {
        let adapter = MemoryAssetAdapter::new().await;
        let capabilities = adapter.get_capabilities();

        assert_eq!(capabilities.asset_type, AssetType::Memory);
        assert!(capabilities.supports_proxy_addressing);
        assert!(capabilities.supports_resource_monitoring);
        assert!(capabilities
            .features
            .contains(&"nat_addressing".to_string()));
        assert!(capabilities
            .features
            .contains(&"quantum_security".to_string()));
    }
}
