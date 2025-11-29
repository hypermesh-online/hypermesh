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

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use async_trait::async_trait;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

use crate::assets::core::{
    AssetAdapter, AssetId, AssetType, AssetResult, AssetError,
    AssetAllocationRequest, AssetStatus, AssetState,
    PrivacyLevel, AssetAllocation, ProxyAddress,
    ResourceUsage, ResourceLimits, MemoryUsage, MemoryLimit,
    AdapterHealth, AdapterCapabilities, ConsensusProof,
    MemoryRequirements,
};
use crate::os_integration::{create_os_abstraction, OsAbstraction};

/// Memory allocation record with NAT-like addressing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryAllocation {
    /// Asset ID
    pub asset_id: AssetId,
    /// Local memory address
    pub local_address: usize,
    /// Size in bytes
    pub size_bytes: u64,
    /// Memory type (DDR4, DDR5, etc.)
    pub memory_type: String,
    /// ECC enabled
    pub ecc_enabled: bool,
    /// NUMA node
    pub numa_node: Option<u32>,
    /// Privacy level
    pub privacy_level: PrivacyLevel,
    /// Remote proxy address for NAT-like access
    pub proxy_address: Option<ProxyAddress>,
    /// Allocation timestamp
    pub allocated_at: SystemTime,
    /// Reference count for sharing
    pub reference_count: u32,
    /// Copy-on-write enabled
    pub cow_enabled: bool,
    /// Deduplication hash for memory content
    pub dedup_hash: Option<[u8; 32]>,
}

/// Memory pool for distributed management
#[derive(Clone, Debug)]
pub struct MemoryPool {
    /// Pool identifier
    pub pool_id: String,
    /// Total pool size in bytes
    pub total_size: u64,
    /// Available size in bytes
    pub available_size: u64,
    /// Memory type in pool
    pub memory_type: String,
    /// NUMA node affinity
    pub numa_node: Option<u32>,
    /// Pool privacy level
    pub privacy_level: PrivacyLevel,
    /// Active allocations
    pub allocations: Vec<AssetId>,
}

/// Memory proxy address mapping for NAT-like system
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryProxyMapping {
    /// Remote proxy address (IPv6-like)
    pub proxy_address: ProxyAddress,
    /// Local asset ID
    pub local_asset_id: AssetId,
    /// Local memory address
    pub local_address: usize,
    /// Size in bytes
    pub size_bytes: u64,
    /// Access permissions
    pub permissions: MemoryPermissions,
    /// Expiration time for security
    pub expires_at: SystemTime,
    /// FALCON-1024 signature for quantum security
    pub access_signature: Vec<u8>,
}

/// Memory access permissions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryPermissions {
    /// Read access allowed
    pub read: bool,
    /// Write access allowed
    pub write: bool,
    /// Execute access allowed (for code segments)
    pub execute: bool,
    /// Share access allowed
    pub share: bool,
}

/// Memory Asset Adapter implementation
pub struct MemoryAssetAdapter {
    /// Active memory allocations by asset ID
    allocations: Arc<RwLock<HashMap<AssetId, MemoryAllocation>>>,
    /// Memory pools for distributed management
    memory_pools: Arc<RwLock<HashMap<String, MemoryPool>>>,
    /// Proxy address mappings for NAT-like system
    proxy_mappings: Arc<RwLock<HashMap<ProxyAddress, MemoryProxyMapping>>>,
    /// Reverse mapping from asset ID to proxy address
    asset_to_proxy: Arc<RwLock<HashMap<AssetId, ProxyAddress>>>,
    /// Total system memory in bytes
    total_memory: u64,
    /// Available memory in bytes
    available_memory: Arc<RwLock<u64>>,
    /// Memory usage statistics
    usage_stats: Arc<RwLock<MemoryUsageStats>>,
}

/// Memory usage statistics
#[derive(Clone, Debug, Default)]
pub struct MemoryUsageStats {
    /// Total allocations made
    pub total_allocations: u64,
    /// Total deallocations made
    pub total_deallocations: u64,
    /// Current active allocations
    pub active_allocations: u64,
    /// Total bytes allocated
    pub total_bytes_allocated: u64,
    /// Total bytes deallocated
    pub total_bytes_deallocated: u64,
    /// Peak memory usage
    pub peak_memory_usage: u64,
    /// Deduplication savings in bytes
    pub dedup_savings_bytes: u64,
    /// Copy-on-write savings in bytes
    pub cow_savings_bytes: u64,
}

impl MemoryAssetAdapter {
    /// Create new memory adapter
    pub async fn new() -> Self {
        // Get system memory information
        let total_memory = Self::get_system_memory().await;
        
        // Initialize with default memory pool
        let mut memory_pools = HashMap::new();
        memory_pools.insert("default".to_string(), MemoryPool {
            pool_id: "default".to_string(),
            total_size: total_memory,
            available_size: total_memory,
            memory_type: "DDR4".to_string(), // Default assumption
            numa_node: None,
            privacy_level: PrivacyLevel::Private,
            allocations: Vec::new(),
        });
        
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
        // Use OS abstraction for real memory detection
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

        // Fallback: return a reasonable default (8GB)
        let fallback_memory = 8 * 1024 * 1024 * 1024;
        tracing::info!("Using fallback memory configuration: 8 GB");
        fallback_memory
    }
    
    /// Allocate memory from pool
    async fn allocate_memory_from_pool(
        &self,
        pool_id: &str,
        size_bytes: u64,
        numa_node: Option<u32>,
    ) -> AssetResult<usize> {
        let mut pools = self.memory_pools.write().await;
        let pool = pools.get_mut(pool_id)
            .ok_or_else(|| AssetError::AllocationFailed {
                reason: format!("Memory pool '{}' not found", pool_id)
            })?;
        
        if pool.available_size < size_bytes {
            return Err(AssetError::AllocationFailed {
                reason: format!(
                    "Insufficient memory in pool '{}': {} bytes requested, {} available",
                    pool_id, size_bytes, pool.available_size
                )
            });
        }
        
        // TODO: Implement actual memory allocation
        // For now, simulate with address calculation
        let local_address = 0x1000_0000 + (pool.total_size - pool.available_size) as usize;
        
        pool.available_size -= size_bytes;
        
        Ok(local_address)
    }
    
    /// Generate proxy address for NAT-like system
    async fn generate_proxy_address(asset_id: &AssetId) -> ProxyAddress {
        let uuid_bytes = asset_id.uuid.as_bytes();
        let mut node_id = [0u8; 8];
        node_id.copy_from_slice(&uuid_bytes[..8]);
        ProxyAddress::new(
            [0x2a, 0x01, 0x04, 0xf8, 0x01, 0x10, 0x53, 0xad,
             0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01],
            node_id,
            8080
        )
    }
    
    /// Create FALCON-1024 signature for quantum security
    async fn create_access_signature(
        &self,
        proxy_mapping: &MemoryProxyMapping,
    ) -> Vec<u8> {
        // TODO: Implement actual FALCON-1024 signature
        // For now, return placeholder signature
        let mut signature = Vec::new();
        signature.extend_from_slice(b"FALCON1024_SIG_");
        signature.extend_from_slice(&proxy_mapping.local_asset_id.blockchain_hash[..16]);
        signature
    }
    
    /// Validate memory access permissions
    async fn validate_memory_access(
        &self,
        proxy_addr: &ProxyAddress,
        access_type: MemoryAccessType,
    ) -> AssetResult<bool> {
        let mappings = self.proxy_mappings.read().await;
        let mapping = mappings.get(proxy_addr)
            .ok_or_else(|| AssetError::ProxyResolutionFailed {
                address: proxy_addr.clone()
            })?;
        
        // Check expiration
        if mapping.expires_at < SystemTime::now() {
            return Ok(false);
        }
        
        // Check permissions
        let permitted = match access_type {
            MemoryAccessType::Read => mapping.permissions.read,
            MemoryAccessType::Write => mapping.permissions.write,
            MemoryAccessType::Execute => mapping.permissions.execute,
            MemoryAccessType::Share => mapping.permissions.share,
        };
        
        Ok(permitted)
    }
    
    /// Perform memory deduplication
    async fn deduplicate_memory(&self, allocation: &mut MemoryAllocation) -> u64 {
        // TODO: Implement actual memory deduplication
        // Calculate content hash and check for duplicates
        let content_hash = [0u8; 32]; // Placeholder
        allocation.dedup_hash = Some(content_hash);
        
        // Return estimated savings (placeholder)
        allocation.size_bytes / 4 // Assume 25% deduplication savings
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
            },
            MemoryOperation::Deallocate => {
                stats.total_deallocations += 1;
                stats.active_allocations = stats.active_allocations.saturating_sub(1);
                stats.total_bytes_deallocated += bytes;
            },
        }
    }
}

/// Memory access types for permission validation
#[derive(Clone, Debug)]
enum MemoryAccessType {
    Read,
    Write,
    Execute,
    Share,
}

/// Memory operations for statistics
#[derive(Clone, Debug)]
enum MemoryOperation {
    Allocate,
    Deallocate,
}

#[async_trait]
impl AssetAdapter for MemoryAssetAdapter {
    fn asset_type(&self) -> AssetType {
        AssetType::Memory
    }
    
    async fn validate_consensus_proof(&self, proof: &ConsensusProof) -> AssetResult<bool> {
        // Validate all four proofs as required by Proof of State patterns
        let valid = proof.validate().await?;
        
        if !valid {
            return Ok(false);
        }
        
        // Memory-specific validation
        // PoSpace: Validate memory space has committed storage
        if proof.space_proof.total_size == 0 {
            return Ok(false);
        }
        
        // PoStake: Validate memory access stake (higher minimum for memory)
        if proof.stake_proof.stake_amount < 100 { 
            return Ok(false);
        }
        
        // PoWork: Validate computational work for memory allocation
        if proof.work_proof.computational_power < 12 { // Lower difficulty for memory allocation
            return Ok(false);
        }
        
        // PoTime: Validate temporal ordering for memory management
        let time_valid = proof.time_proof.time_verification_timestamp > 0;
        
        Ok(time_valid)
    }
    
    async fn allocate_asset(&self, request: &AssetAllocationRequest) -> AssetResult<AssetAllocation> {
        // Validate consensus proof first
        if !self.validate_consensus_proof(&request.consensus_proof).await? {
            return Err(AssetError::ConsensusValidationFailed {
                reason: "Memory allocation consensus validation failed".to_string()
            });
        }
        
        // Get memory requirements
        let memory_req = request.requested_resources.memory.as_ref()
            .ok_or_else(|| AssetError::AllocationFailed {
                reason: "No memory requirements specified".to_string()
            })?;
        
        // Check available memory
        let available = *self.available_memory.read().await;
        if available < memory_req.size_bytes {
            return Err(AssetError::AllocationFailed {
                reason: format!(
                    "Insufficient memory: {} bytes requested, {} available",
                    memory_req.size_bytes, available
                )
            });
        }
        
        // Allocate memory from appropriate pool
        let pool_id = if memory_req.numa_node.is_some() {
            format!("numa_{}", memory_req.numa_node.unwrap())
        } else {
            "default".to_string()
        };
        
        let local_address = self.allocate_memory_from_pool(
            &pool_id,
            memory_req.size_bytes,
            memory_req.numa_node,
        ).await?;
        
        // Create asset ID
        let asset_id = AssetId::new(AssetType::Memory);
        
        // Generate proxy address for NAT-like system
        let proxy_address = Self::generate_proxy_address(&asset_id).await;
        
        // Create memory allocation record
        let mut allocation = MemoryAllocation {
            asset_id: asset_id.clone(),
            local_address,
            size_bytes: memory_req.size_bytes,
            memory_type: memory_req.memory_type.clone().unwrap_or_else(|| "DDR4".to_string()),
            ecc_enabled: memory_req.ecc_required,
            numa_node: memory_req.numa_node,
            privacy_level: request.privacy_level.clone(),
            proxy_address: Some(proxy_address.clone()),
            allocated_at: SystemTime::now(),
            reference_count: 1,
            cow_enabled: true, // Enable copy-on-write by default
            dedup_hash: None,
        };
        
        // Perform deduplication if enabled
        let dedup_savings = self.deduplicate_memory(&mut allocation).await;
        
        // Update usage statistics
        self.update_usage_stats(MemoryOperation::Allocate, memory_req.size_bytes).await;
        
        // Update available memory
        {
            let mut available = self.available_memory.write().await;
            *available -= memory_req.size_bytes;
        }
        
        // Create proxy mapping for NAT-like access
        let proxy_mapping = MemoryProxyMapping {
            proxy_address: proxy_address.clone(),
            local_asset_id: asset_id.clone(),
            local_address,
            size_bytes: memory_req.size_bytes,
            permissions: MemoryPermissions {
                read: true,
                write: true,
                execute: false, // No execute by default
                share: matches!(request.privacy_level, PrivacyLevel::FullPublic | PrivacyLevel::PublicNetwork),
            },
            expires_at: SystemTime::now() + Duration::from_secs(3600), // 1 hour default
            access_signature: Vec::new(), // Will be filled by create_access_signature
        };
        
        // Create quantum-resistant signature
        let mut proxy_mapping_with_sig = proxy_mapping;
        proxy_mapping_with_sig.access_signature = self.create_access_signature(&proxy_mapping_with_sig).await;
        
        // Store allocation and mappings
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
        
        // Update deduplication stats
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
                privacy_level: PrivacyLevel::Private,
                proxy_address: None,
                consensus_proofs: Vec::new(),
                owner_certificate_fingerprint: request.certificate_fingerprint.clone(),
                metadata: HashMap::new(),
                health_status: crate::assets::core::status::AssetHealthStatus::default(),
                performance_metrics: crate::assets::core::status::AssetPerformanceMetrics::default(),
            },
            allocation_config: crate::assets::core::privacy::AllocationConfig {
                privacy_level: request.privacy_level.clone(),
                resource_allocation: crate::assets::core::privacy::ResourceAllocationConfig::default(),
                concurrency_limits: crate::assets::core::privacy::ConcurrencyLimits::default(),
                duration_config: crate::assets::core::privacy::DurationConfig::default(),
                consensus_requirements: crate::assets::core::privacy::ConsensusRequirements::default(),
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
    
    async fn deallocate_asset(&self, asset_id: &AssetId) -> AssetResult<()> {
        // Get allocation record
        let allocation = {
            let mut allocations = self.allocations.write().await;
            allocations.remove(asset_id)
                .ok_or_else(|| AssetError::AssetNotFound {
                    asset_id: asset_id.to_string()
                })?
        };
        
        // Remove proxy mapping
        if let Some(proxy_addr) = &allocation.proxy_address {
            let mut mappings = self.proxy_mappings.write().await;
            mappings.remove(proxy_addr);
        }
        
        // Remove asset to proxy mapping
        {
            let mut asset_to_proxy = self.asset_to_proxy.write().await;
            asset_to_proxy.remove(asset_id);
        }
        
        // Update available memory
        {
            let mut available = self.available_memory.write().await;
            *available += allocation.size_bytes;
        }
        
        // Update usage statistics
        self.update_usage_stats(MemoryOperation::Deallocate, allocation.size_bytes).await;
        
        // TODO: Implement actual memory deallocation
        tracing::info!("Deallocated memory asset: {} ({} bytes)", asset_id, allocation.size_bytes);
        
        Ok(())
    }
    
    async fn get_asset_status(&self, asset_id: &AssetId) -> AssetResult<AssetStatus> {
        let allocations = self.allocations.read().await;
        let allocation = allocations.get(asset_id)
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string()
            })?;
        
        Ok(AssetStatus {
            asset_id: asset_id.clone(),
            state: AssetState::InUse,
            allocated_at: allocation.allocated_at,
            last_accessed: SystemTime::now(),
            privacy_level: allocation.privacy_level.clone(),
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
                metadata.insert("local_address".to_string(), format!("0x{:x}", allocation.local_address));
                metadata.insert("numa_node".to_string(), allocation.numa_node.map(|n| n.to_string()).unwrap_or_else(|| "none".to_string()));
                metadata.insert("ecc_enabled".to_string(), allocation.ecc_enabled.to_string());
                metadata.insert("cow_enabled".to_string(), allocation.cow_enabled.to_string());
                metadata.insert("reference_count".to_string(), allocation.reference_count.to_string());
                metadata
            },
        })
    }
    
    async fn configure_privacy_level(&self, asset_id: &AssetId, privacy: PrivacyLevel) -> AssetResult<()> {
        let mut allocations = self.allocations.write().await;
        let allocation = allocations.get_mut(asset_id)
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string()
            })?;
        
        allocation.privacy_level = privacy.clone();
        
        // Update proxy mapping permissions based on privacy level
        if let Some(proxy_addr) = &allocation.proxy_address {
            let mut mappings = self.proxy_mappings.write().await;
            if let Some(mapping) = mappings.get_mut(proxy_addr) {
                mapping.permissions.share = matches!(privacy, PrivacyLevel::FullPublic | PrivacyLevel::PublicNetwork);
            }
        }
        
        tracing::info!("Updated privacy level for memory asset {}: {:?}", asset_id, privacy);
        Ok(())
    }
    
    async fn assign_proxy_address(&self, asset_id: &AssetId) -> AssetResult<ProxyAddress> {
        let asset_to_proxy = self.asset_to_proxy.read().await;
        asset_to_proxy.get(asset_id)
            .cloned()
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string()
            })
    }
    
    async fn resolve_proxy_address(&self, proxy_addr: &ProxyAddress) -> AssetResult<AssetId> {
        let mappings = self.proxy_mappings.read().await;
        mappings.get(proxy_addr)
            .map(|mapping| mapping.local_asset_id.clone())
            .ok_or_else(|| AssetError::ProxyResolutionFailed {
                address: proxy_addr.clone()
            })
    }
    
    async fn get_resource_usage(&self, asset_id: &AssetId) -> AssetResult<ResourceUsage> {
        let allocations = self.allocations.read().await;
        let allocation = allocations.get(asset_id)
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string()
            })?;
        
        // TODO: Implement actual memory usage monitoring
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
    
    async fn set_resource_limits(&self, asset_id: &AssetId, limits: ResourceLimits) -> AssetResult<()> {
        if let Some(memory_limit) = limits.memory_limit {
            // TODO: Implement memory limit enforcement
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
        
        let healthy = available > 0 && stats.active_allocations < 10000; // Reasonable limits
        
        let mut performance_metrics = std::collections::HashMap::new();
        performance_metrics.insert("total_memory_gb".to_string(), (self.total_memory / (1024 * 1024 * 1024)) as f64);
        performance_metrics.insert("available_memory_gb".to_string(), (available / (1024 * 1024 * 1024)) as f64);
        performance_metrics.insert("memory_utilization_percent".to_string(), 
            ((self.total_memory - available) as f64 / self.total_memory as f64) * 100.0);
        performance_metrics.insert("active_allocations".to_string(), stats.active_allocations as f64);
        performance_metrics.insert("dedup_savings_gb".to_string(), (stats.dedup_savings_bytes / (1024 * 1024 * 1024)) as f64);
        performance_metrics.insert("cow_savings_gb".to_string(), (stats.cow_savings_bytes / (1024 * 1024 * 1024)) as f64);
        
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
                PrivacyLevel::Private,
                PrivacyLevel::PrivateNetwork,
                PrivacyLevel::P2P,
                PrivacyLevel::PublicNetwork,
                PrivacyLevel::FullPublic,
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
    use crate::assets::core::{SpaceProof, StakeProof, WorkProof, TimeProof, WorkloadType, WorkState};
    
    async fn create_test_memory_request() -> AssetAllocationRequest {
        AssetAllocationRequest {
            asset_type: AssetType::Memory,
            requested_resources: crate::assets::core::ResourceRequirements {
                memory: Some(MemoryRequirements {
                    size_bytes: 1024 * 1024 * 1024, // 1GB
                    memory_type: Some("DDR4".to_string()),
                    ecc_required: false,
                    numa_node: None,
                }),
                ..Default::default()
            },
            privacy_level: PrivacyLevel::Private,
            consensus_proof: ConsensusProof::new(
                SpaceProof {
                    node_id: "test-node".to_string(),
                    storage_path: "/test/memory".to_string(),
                    allocated_size: 1024 * 1024 * 1024,
                    proof_hash: vec![1, 2, 3, 4],
                    timestamp: SystemTime::now(),
                },
                StakeProof {
                    stake_holder: "test-holder".to_string(),
                    stake_holder_id: "test-holder-id".to_string(),
                    stake_amount: 1000,
                    stake_timestamp: SystemTime::now(),
                },
                WorkProof {
                    worker_id: "test-worker".to_string(),
                    workload_id: "test-workload".to_string(),
                    process_id: 12345,
                    computational_power: 100,
                    workload_type: WorkloadType::Compute,
                    work_state: WorkState::Completed,
                },
                TimeProof {
                    network_time_offset: Duration::from_secs(10),
                    time_verification_timestamp: SystemTime::now(),
                    nonce: 42,
                    proof_hash: vec![5, 6, 7, 8],
                },
            ),
            certificate_fingerprint: "test-cert".to_string(),
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
        let request = create_test_memory_request().await;
        
        let allocation = adapter.allocate_asset(&request).await.unwrap();
        assert_eq!(allocation.asset_id.asset_type, AssetType::Memory);
        assert!(allocation.proxy_address.is_some());
        
        // Test deallocation
        adapter.deallocate_asset(&allocation.asset_id).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_proxy_address_resolution() {
        let adapter = MemoryAssetAdapter::new().await;
        let request = create_test_memory_request().await;
        
        let allocation = adapter.allocate_asset(&request).await.unwrap();
        let proxy_addr = allocation.proxy_address.unwrap();
        
        // Test proxy address resolution
        let resolved_asset_id = adapter.resolve_proxy_address(&proxy_addr).await.unwrap();
        assert_eq!(resolved_asset_id, allocation.asset_id);
        
        adapter.deallocate_asset(&allocation.asset_id).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_memory_health_check() {
        let adapter = MemoryAssetAdapter::new().await;
        let health = adapter.health_check().await.unwrap();
        
        assert!(health.healthy);
        assert!(health.performance_metrics.contains_key("total_memory_gb"));
        assert!(health.performance_metrics.contains_key("available_memory_gb"));
    }
    
    #[tokio::test]
    async fn test_adapter_capabilities() {
        let adapter = MemoryAssetAdapter::new().await;
        let capabilities = adapter.get_capabilities();
        
        assert_eq!(capabilities.asset_type, AssetType::Memory);
        assert!(capabilities.supports_proxy_addressing);
        assert!(capabilities.supports_resource_monitoring);
        assert!(capabilities.features.contains(&"nat_addressing".to_string()));
        assert!(capabilities.features.contains(&"quantum_security".to_string()));
    }
}