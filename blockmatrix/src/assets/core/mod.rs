//! HyperMesh Asset Management Core
//!
//! Universal asset management system where everything in HyperMesh is an Asset:
//! - Hardware resources (CPU, GPU, Memory, Storage)
//! - Containers and services
//! - Network resources and bandwidth
//! - User-defined assets
//!
//! All assets require Consensus Proof validation (PoSpace + PoStake + PoWork + PoTime)
//! and support user-configurable privacy levels with remote proxy addressing.

#![warn(missing_docs)]
#![deny(unsafe_code)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

// Submodules
pub mod asset_id;
pub mod adapter;
pub mod status;
pub mod privacy;
pub mod proxy;

// Re-exports
pub use asset_id::{AssetId, AssetType};
pub use adapter::{
    AssetAdapter, AssetAllocationRequest, ResourceRequirements, ResourceLimits, ResourceUsage,
    CpuRequirements, CpuUsage, CpuLimit,
    GpuRequirements, GpuUsage, GpuLimit,
    MemoryRequirements, MemoryUsage, MemoryLimit,
    StorageRequirements, StorageUsage, StorageLimit, StorageType,
    NetworkRequirements, NetworkUsage, NetworkLimit,
    ContainerRequirements, VolumeMount, PortMapping,
    AdapterHealth, AdapterCapabilities,
    EconomicRequirements, AssetPriority
};
pub use status::{AssetStatus, AssetState};
pub use privacy::{PrivacyLevel, AssetAllocation};
pub use proxy::{
    ProxyAddress, ProxyType, ProxyAddressResolver, ProxyNodeInfo, ProxyCapabilities, ProxyStatistics,
    // CRITICAL Remote Proxy/NAT system exports
    RemoteProxyManager, ProxyRouter, ProxyForwarder,
    TrustChainIntegration, QuantumSecurity, ShardedDataAccess,
    NATTranslator, GlobalAddress,
    ProxySystemStats, ProxyNetworkConfig,
};

/// Result type for asset operations
pub type AssetResult<T> = Result<T, AssetError>;

/// Asset management errors
#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    /// Asset not found
    #[error("Asset not found: {asset_id}")]
    AssetNotFound { asset_id: String },
    
    /// Consensus validation failed
    #[error("Consensus validation failed: {reason}")]
    ConsensusValidationFailed { reason: String },
    
    /// Invalid privacy level configuration
    #[error("Invalid privacy level: {level:?}")]
    InvalidPrivacyLevel { level: PrivacyLevel },
    
    /// Resource allocation failed
    #[error("Resource allocation failed: {reason}")]
    AllocationFailed { reason: String },
    
    /// Proxy address resolution failed
    #[error("Proxy address resolution failed: {address:?}")]
    ProxyResolutionFailed { address: ProxyAddress },
    
    /// Certificate validation failed
    #[error("Certificate validation failed: {fingerprint}")]
    CertificateValidationFailed { fingerprint: String },
    
    /// Adapter operation failed
    #[error("Adapter operation failed: {message}")]
    AdapterError { message: String },

    /// Validation error
    #[error("Validation error: {message}")]
    ValidationError { message: String },

    /// Network error
    #[error("Network error: {message}")]
    NetworkError { message: String },

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

// Import Proof of State Four-Proof Consensus System
pub use crate::consensus::proof_of_state_integration::{
    ConsensusProof, SpaceProof, StakeProof, WorkProof, TimeProof,
    WorkloadType, WorkState, ClientCredentials, Proof,
};

// All consensus proof types are now imported from Proof of State integration above

/// Core asset manager coordinating all asset operations
pub struct AssetManager {
    /// Registry of all assets by ID
    assets: Arc<RwLock<HashMap<AssetId, AssetStatus>>>,
    /// Registry of asset adapters by type
    adapters: Arc<RwLock<HashMap<AssetType, Box<dyn AssetAdapter>>>>,
    /// Proxy address resolver
    proxy_resolver: Arc<ProxyAddressResolver>,
    /// Consensus validation requirements
    consensus_requirements: ConsensusRequirements,
}

/// Consensus requirements configuration
#[derive(Clone, Debug)]
pub struct ConsensusRequirements {
    /// Require all four proofs (default: true)
    pub require_all_proofs: bool,
    /// Minimum stake amount required
    pub minimum_stake: u64,
    /// Maximum time offset allowed
    pub max_time_offset: Duration,
    /// Minimum computational power required
    pub minimum_compute_power: u64,
}

impl Default for ConsensusRequirements {
    fn default() -> Self {
        Self {
            require_all_proofs: true,
            minimum_stake: 1000,
            max_time_offset: Duration::from_secs(30),
            minimum_compute_power: 100,
        }
    }
}

impl AssetManager {
    /// Create new asset manager
    pub fn new() -> Self {
        Self {
            assets: Arc::new(RwLock::new(HashMap::new())),
            adapters: Arc::new(RwLock::new(HashMap::new())),
            proxy_resolver: Arc::new(ProxyAddressResolver::new()),
            consensus_requirements: ConsensusRequirements::default(),
        }
    }
    
    /// Register an asset adapter for a specific asset type
    pub async fn register_adapter(
        &self,
        asset_type: AssetType,
        adapter: Box<dyn AssetAdapter>,
    ) -> AssetResult<()> {
        let mut adapters = self.adapters.write().await;
        adapters.insert(asset_type.clone(), adapter);
        tracing::info!("Registered adapter for asset type: {:?}", asset_type);
        Ok(())
    }
    
    /// Allocate an asset with consensus proof validation
    pub async fn allocate_asset(
        &self,
        request: AssetAllocationRequest,
    ) -> AssetResult<AssetAllocation> {
        // Validate consensus proof first
        self.validate_consensus_proof(&request.consensus_proof).await?;
        
        // Get appropriate adapter
        let adapters = self.adapters.read().await;
        let adapter = adapters.get(&request.asset_type)
            .ok_or_else(|| AssetError::AdapterError {
                message: format!("No adapter found for asset type: {:?}", request.asset_type)
            })?;
        
        // Delegate to adapter
        let allocation = adapter.allocate_asset(&request).await?;
        
        // Register asset status
        let mut assets = self.assets.write().await;
        assets.insert(allocation.asset_id.clone(), allocation.status.clone());
        
        tracing::info!("Allocated asset: {}", allocation.asset_id);
        Ok(allocation)
    }
    
    /// Deallocate an asset
    pub async fn deallocate_asset(&self, asset_id: &AssetId) -> AssetResult<()> {
        // Get adapter for asset type
        let adapters = self.adapters.read().await;
        let adapter = adapters.get(&asset_id.asset_type)
            .ok_or_else(|| AssetError::AdapterError {
                message: format!("No adapter found for asset type: {:?}", asset_id.asset_type)
            })?;
        
        // Delegate to adapter
        adapter.deallocate_asset(asset_id).await?;
        
        // Remove from registry
        let mut assets = self.assets.write().await;
        assets.remove(asset_id);
        
        tracing::info!("Deallocated asset: {}", asset_id);
        Ok(())
    }
    
    /// Get current status of an asset
    pub async fn get_asset_status(&self, asset_id: &AssetId) -> AssetResult<AssetStatus> {
        // First check local registry
        {
            let assets = self.assets.read().await;
            if let Some(status) = assets.get(asset_id) {
                return Ok(status.clone());
            }
        }
        
        // If not in registry, query adapter
        let adapters = self.adapters.read().await;
        let adapter = adapters.get(&asset_id.asset_type)
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string()
            })?;
        
        adapter.get_asset_status(asset_id).await
    }
    
    /// Configure privacy level for an asset
    pub async fn configure_privacy(
        &self,
        asset_id: &AssetId,
        privacy_level: PrivacyLevel,
    ) -> AssetResult<()> {
        let adapters = self.adapters.read().await;
        let adapter = adapters.get(&asset_id.asset_type)
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string()
            })?;
        
        adapter.configure_privacy_level(asset_id, privacy_level).await
    }
    
    /// Assign proxy address for remote access
    pub async fn assign_proxy_address(&self, asset_id: &AssetId) -> AssetResult<ProxyAddress> {
        let adapters = self.adapters.read().await;
        let adapter = adapters.get(&asset_id.asset_type)
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string()
            })?;
        
        let proxy_address = adapter.assign_proxy_address(asset_id).await?;
        
        // Register with proxy resolver
        self.proxy_resolver.register_mapping(proxy_address.clone(), asset_id.clone()).await;
        
        Ok(proxy_address)
    }
    
    /// Resolve proxy address to asset ID
    pub async fn resolve_proxy_address(&self, proxy_addr: &ProxyAddress) -> AssetResult<AssetId> {
        self.proxy_resolver.resolve(proxy_addr).await
            .ok_or_else(|| AssetError::ProxyResolutionFailed {
                address: proxy_addr.clone()
            })
    }
    
    /// List all assets of a specific type
    pub async fn list_assets_by_type(&self, asset_type: AssetType) -> AssetResult<Vec<AssetStatus>> {
        let assets = self.assets.read().await;
        let filtered_assets: Vec<AssetStatus> = assets
            .iter()
            .filter(|(id, _)| id.asset_type == asset_type)
            .map(|(_, status)| status.clone())
            .collect();
        
        Ok(filtered_assets)
    }
    
    /// Get resource usage for an asset
    pub async fn get_resource_usage(&self, asset_id: &AssetId) -> AssetResult<ResourceUsage> {
        let adapters = self.adapters.read().await;
        let adapter = adapters.get(&asset_id.asset_type)
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string()
            })?;
        
        adapter.get_resource_usage(asset_id).await
    }
    
    /// Set resource limits for an asset
    pub async fn set_resource_limits(
        &self,
        asset_id: &AssetId,
        limits: ResourceLimits,
    ) -> AssetResult<()> {
        let adapters = self.adapters.read().await;
        let adapter = adapters.get(&asset_id.asset_type)
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string()
            })?;
        
        adapter.set_resource_limits(asset_id, limits).await
    }
    
    /// Validate consensus proof according to requirements using Proof of State Four-Proof System
    async fn validate_consensus_proof(&self, proof: &ConsensusProof) -> AssetResult<bool> {
        // Use Proof of State comprehensive validation first
        if let Err(e) = proof.validate_comprehensive().await {
            return Err(AssetError::ConsensusValidationFailed {
                reason: format!("Proof of State comprehensive validation failed: {}", e)
            });
        }
        
        // Basic validation check
        if !proof.validate() {
            return Err(AssetError::ConsensusValidationFailed {
                reason: "Basic consensus proof validation failed".to_string()
            });
        }
        
        // Check against HyperMesh asset requirements
        if self.consensus_requirements.require_all_proofs {
            // All four proofs must be present and valid (enforced by Proof of State)
            if proof.stake_proof.stake_amount < self.consensus_requirements.minimum_stake {
                return Err(AssetError::ConsensusValidationFailed {
                    reason: format!(
                        "Insufficient stake: {} < required {}",
                        proof.stake_proof.stake_amount,
                        self.consensus_requirements.minimum_stake
                    )
                });
            }
            
            if proof.time_proof.network_time_offset > self.consensus_requirements.max_time_offset {
                return Err(AssetError::ConsensusValidationFailed {
                    reason: "Time offset too large".to_string()
                });
            }
            
            if proof.work_proof.computational_power < self.consensus_requirements.minimum_compute_power {
                return Err(AssetError::ConsensusValidationFailed {
                    reason: "Insufficient computational power".to_string()
                });
            }
            
            // Validate storage space commitment (from Proof of State SpaceProof)
            if proof.space_proof.total_storage == 0 {
                return Err(AssetError::ConsensusValidationFailed {
                    reason: "No storage space committed".to_string()
                });
            }
        }
        
        Ok(true)
    }
    
    /// Get current asset statistics
    pub async fn get_asset_statistics(&self) -> AssetStatistics {
        let assets = self.assets.read().await;
        let mut stats = AssetStatistics::default();
        
        for (asset_id, status) in assets.iter() {
            match asset_id.asset_type {
                AssetType::Cpu => stats.cpu_assets += 1,
                AssetType::Gpu => stats.gpu_assets += 1,
                AssetType::Memory => stats.memory_assets += 1,
                AssetType::Storage => stats.storage_assets += 1,
                AssetType::Network => stats.network_assets += 1,
                AssetType::Container => stats.container_assets += 1,
                AssetType::Economic => stats.economic_assets += 1,
            }
            
            match status.state {
                AssetState::Available => stats.available_assets += 1,
                AssetState::Allocated => stats.allocated_assets += 1,
                AssetState::InUse => stats.in_use_assets += 1,
                AssetState::Maintenance => stats.maintenance_assets += 1,
                AssetState::Failed => stats.failed_assets += 1,
            }
        }
        
        stats.total_assets = assets.len();
        stats
    }
}

/// Asset system statistics
#[derive(Clone, Debug, Default)]
pub struct AssetStatistics {
    /// Total number of assets
    pub total_assets: usize,
    /// Assets by type
    pub cpu_assets: usize,
    pub gpu_assets: usize,
    pub memory_assets: usize,
    pub storage_assets: usize,
    pub network_assets: usize,
    pub container_assets: usize,
    pub economic_assets: usize,
    /// Assets by state
    pub available_assets: usize,
    pub allocated_assets: usize,
    pub in_use_assets: usize,
    pub maintenance_assets: usize,
    pub failed_assets: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_consensus_proof_validation() {
        // Test Proof of State Four-Proof Consensus System integration
        let stake_proof = StakeProof::new(
            "test-holder".to_string(),
            "test-holder-id".to_string(), 
            1000
        );
        
        let mut space_proof = SpaceProof::new(
            1024,
            "/test/path".to_string()
        );
        space_proof.node_id = "test-node".to_string();
        
        let work_proof = WorkProof::new(
            100,                         // computational_power
            "test-workload".to_string(), // workload_id  
            12345,                       // pid
            "test-worker".to_string(),   // owner_id
            WorkloadType::Compute,       // workload_type
            WorkState::Completed,        // work_state
        );
        
        let time_proof = TimeProof::new(Duration::from_secs(10));
        
        let consensus_proof = ConsensusProof::new(
            stake_proof, 
            space_proof, 
            work_proof, 
            time_proof
        );
        
        // Test basic validation (synchronous)
        assert!(consensus_proof.validate());
    }
    
    #[tokio::test]
    async fn test_asset_manager_creation() {
        let manager = AssetManager::new();
        let stats = manager.get_asset_statistics().await;
        assert_eq!(stats.total_assets, 0);
    }
}