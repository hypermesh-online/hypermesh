// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Asset Management Core
//!
//! Universal asset management system where everything in HyperMesh is an Asset:
//! - Hardware resources (CPU, GPU, Memory, Storage)
//! - Containers and services
//! - Network resources and bandwidth
//! - User-defined assets
//!
//! All assets require State Proof validation (PoSpace + PoStake + PoWork + PoTime)
//! and support user-configurable privacy levels with remote proxy addressing.

#![deny(unsafe_code)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use trustchain::proof_of_state::StateProofOps;

// Submodules
pub mod adapter;
pub mod asset_id;
pub mod authz;
pub mod privacy;
pub mod proxy;
pub mod status;

// Re-exports
pub use adapter::{
    AdapterCapabilities, AdapterHealth, AssetAdapter, AssetAllocationRequest, AssetPriority,
    ContainerRequirements, CpuLimit, CpuRequirements, CpuUsage, EconomicRequirements, GpuLimit,
    GpuRequirements, GpuUsage, MemoryLimit, MemoryRequirements, MemoryUsage, NetworkLimit,
    NetworkRequirements, NetworkUsage, PortMapping, ResourceLimits, ResourceRequirements,
    ResourceUsage, StorageLimit, StorageRequirements, StorageType, StorageUsage, VolumeMount,
};
pub use asset_id::{
    ApplicationDomain, AssetCategory, AssetData, AssetIdError, AssetRegistration, AssetType,
    BaseSystemType, FederationId, NetworkScope, NodeFingerprint, ProofRequirements, ProofScope,
    RegistryId, ScopeBinding, SecurityError,
};
pub use authz::{
    default_authorize, verify_grant, AuthDecision, AuthorizationSet, CapacityDimension,
    CapacityProfile, Grant, GrantScope, GrantSig, Owner,
};
pub use hypermesh_lib::PrivacyMode;
pub use privacy::AssetAllocation;
pub use proxy::{
    GlobalAddress,
    NATTranslator,
    ProxyAddress,
    ProxyAddressResolver,
    ProxyCapabilities,
    ProxyForwarder,
    ProxyNetworkConfig,
    ProxyNodeInfo,
    ProxyRouter,
    ProxyStatistics,
    ProxySystemStats,
    ProxyType,
    QuantumSecurity,
    // CRITICAL Remote Proxy/NAT system exports
    RemoteProxyManager,
    ShardedDataAccess,
    TrustChainIntegration,
};
pub use status::{AssetState, AssetStatus};

/// Result type for asset operations
pub type AssetResult<T> = Result<T, AssetError>;

/// Asset management errors
#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    /// Asset not found
    #[error("Asset not found: {asset_id}")]
    AssetNotFound { asset_id: String },

    /// State proof validation failed
    #[error("State proof validation failed: {reason}")]
    StateProofValidationFailed { reason: String },

    /// Invalid privacy level configuration
    #[error("Invalid privacy level: {level:?}")]
    InvalidPrivacyLevel { level: PrivacyMode },

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

    /// Resource unavailable
    #[error("Resource unavailable: {0}")]
    ResourceUnavailable(String),

    /// Memory mapping failed
    #[error("Memory mapping failed at {address}: {reason}")]
    MemoryMappingFailed { address: String, reason: String },

    /// Memory not mapped
    #[error("Memory not mapped: {address}")]
    MemoryNotMapped { address: String },

    /// Permission denied
    #[error("Permission denied for {operation} on {resource}: {reason}")]
    PermissionDenied {
        operation: String,
        resource: String,
        reason: String,
    },

    /// Memory access failed
    #[error("Memory access failed: {reason}")]
    MemoryAccessFailed { reason: String },

    /// Operation timeout
    #[error("Operation timeout: {operation}")]
    OperationTimeout { operation: String },

    /// Resource not found
    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    /// Serialization error
    #[error("Serialization error: {message}")]
    SerializationError { message: String },

    /// Deserialization error
    #[error("Deserialization error: {message}")]
    DeserializationError { message: String },

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

// Import Proof of State Four-Proof validation system
pub use crate::proof_of_state::proof_of_state_integration::{
    ClientCredentials, StateProof, Proof, SpaceProof, StakeProof, TimeProof, WorkProof,
};

// WorkloadType is a DESCRIPTIVE workload label (lib::types), not a proof field.
// Re-exported here for asset/orchestration consumers (e.g. WorkloadOptimized).
pub use hypermesh_lib::WorkloadType;

// All state proof types are now imported from Proof of State integration above

/// Core asset manager coordinating all asset operations
pub struct AssetManager {
    /// Registry of all assets by ID
    assets: Arc<RwLock<HashMap<AssetRegistration, AssetStatus>>>,
    /// Registry of asset adapters by type
    adapters: Arc<RwLock<HashMap<AssetType, Arc<dyn AssetAdapter>>>>,
    /// Proxy address resolver
    proxy_resolver: Arc<ProxyAddressResolver>,
    /// State proof validation requirements
    state_requirements: StateRequirements,
}

/// State proof requirements configuration
///
/// CANONICAL MODEL: proofs answer WHO / WHAT / WHERE / WHEN, never a magnitude.
/// There is NO minimum stake / storage / compute gate — the only quantitative
/// bound is the WHEN proof's freshness (`max_time_offset`).
#[derive(Clone, Debug)]
pub struct StateRequirements {
    /// Require all four proofs (default: true)
    pub require_all_proofs: bool,
    /// Maximum time offset allowed
    pub max_time_offset: Duration,
}

impl Default for StateRequirements {
    fn default() -> Self {
        Self {
            require_all_proofs: true,
            max_time_offset: Duration::from_secs(30),
        }
    }
}

impl Default for AssetManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetManager {
    /// Create new asset manager
    pub fn new() -> Self {
        Self {
            assets: Arc::new(RwLock::new(HashMap::new())),
            adapters: Arc::new(RwLock::new(HashMap::new())),
            proxy_resolver: Arc::new(ProxyAddressResolver::new()),
            state_requirements: StateRequirements::default(),
        }
    }

    /// Convert AssetCategory to AssetType for adapter lookup
    fn category_to_asset_type(category: &AssetCategory) -> AssetResult<AssetType> {
        match category {
            AssetCategory::BaseSystem(sys) => Ok(match sys {
                BaseSystemType::Cpu => AssetType::Cpu,
                BaseSystemType::Gpu => AssetType::Gpu,
                BaseSystemType::Memory => AssetType::Memory,
                BaseSystemType::Storage => AssetType::Storage,
                BaseSystemType::Network => AssetType::Network,
                BaseSystemType::Container => AssetType::Container,
                BaseSystemType::Economic => AssetType::Economic,
                BaseSystemType::Blockchain => AssetType::Blockchain,
                BaseSystemType::Dns => AssetType::Dns,
                BaseSystemType::Transmission => AssetType::Transmission,
                BaseSystemType::Dashboard => AssetType::Dashboard,
                BaseSystemType::Identity => AssetType::Identity,
                BaseSystemType::KeyRotation => AssetType::KeyRotation,
                BaseSystemType::Invitation => AssetType::Invitation,
                BaseSystemType::Message => AssetType::Message,
            }),
            AssetCategory::Application(_) => Err(AssetError::AdapterError {
                message: "Cannot determine asset type for application asset".to_string(),
            }),
        }
    }

    /// Register an asset adapter for a specific asset type
    pub async fn register_adapter(
        &self,
        asset_type: AssetType,
        adapter: Arc<dyn AssetAdapter>,
    ) -> AssetResult<()> {
        let mut adapters = self.adapters.write().await;
        adapters.insert(asset_type.clone(), adapter);
        tracing::info!("Registered adapter for asset type: {:?}", asset_type);
        Ok(())
    }

    /// Allocate an asset with state proof validation
    pub async fn allocate_asset(
        &self,
        request: AssetAllocationRequest,
    ) -> AssetResult<AssetAllocation> {
        // Validate state proof first
        self.validate_state_proof(&request.state_proof)
            .await?;

        // Get appropriate adapter
        let adapters = self.adapters.read().await;
        let adapter =
            adapters
                .get(&request.asset_type)
                .ok_or_else(|| AssetError::AdapterError {
                    message: format!("No adapter found for asset type: {:?}", request.asset_type),
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
    pub async fn deallocate_asset(&self, asset_id: &AssetRegistration) -> AssetResult<()> {
        // Derive asset type from category
        let asset_type = Self::category_to_asset_type(&asset_id.category)?;

        let adapters = self.adapters.read().await;
        let adapter = adapters
            .get(&asset_type)
            .ok_or_else(|| AssetError::AdapterError {
                message: format!("No adapter found for asset type: {asset_type:?}"),
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
    pub async fn get_asset_status(&self, asset_id: &AssetRegistration) -> AssetResult<AssetStatus> {
        // First check local registry
        {
            let assets = self.assets.read().await;
            if let Some(status) = assets.get(asset_id) {
                return Ok(status.clone());
            }
        }

        // If not in registry, query adapter
        let asset_type = Self::category_to_asset_type(&asset_id.category)?;

        let adapters = self.adapters.read().await;
        let adapter = adapters
            .get(&asset_type)
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string(),
            })?;

        adapter.get_asset_status(asset_id).await
    }

    /// Configure privacy level for an asset
    pub async fn configure_privacy(
        &self,
        asset_id: &AssetRegistration,
        privacy_level: PrivacyMode,
    ) -> AssetResult<()> {
        let asset_type = Self::category_to_asset_type(&asset_id.category)?;

        let adapters = self.adapters.read().await;
        let adapter = adapters
            .get(&asset_type)
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string(),
            })?;

        adapter
            .configure_privacy_level(asset_id, privacy_level)
            .await
    }

    /// Assign proxy address for remote access
    pub async fn assign_proxy_address(
        &self,
        asset_id: &AssetRegistration,
    ) -> AssetResult<ProxyAddress> {
        let asset_type = Self::category_to_asset_type(&asset_id.category)?;

        let adapters = self.adapters.read().await;
        let adapter = adapters
            .get(&asset_type)
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string(),
            })?;

        let proxy_address = adapter.assign_proxy_address(asset_id).await?;

        // Register with proxy resolver
        self.proxy_resolver
            .register_mapping(proxy_address.clone(), asset_id.clone())
            .await;

        Ok(proxy_address)
    }

    /// Resolve proxy address to asset ID
    pub async fn resolve_proxy_address(
        &self,
        proxy_addr: &ProxyAddress,
    ) -> AssetResult<AssetRegistration> {
        self.proxy_resolver
            .resolve(proxy_addr)
            .await
            .ok_or_else(|| AssetError::ProxyResolutionFailed {
                address: proxy_addr.clone(),
            })
    }

    /// List all assets of a specific type
    pub async fn list_assets_by_type(
        &self,
        asset_type: AssetType,
    ) -> AssetResult<Vec<AssetStatus>> {
        let assets = self.assets.read().await;
        let filtered_assets: Vec<AssetStatus> = assets
            .iter()
            .filter(|(id, _)| {
                Self::category_to_asset_type(&id.category).ok() == Some(asset_type.clone())
            })
            .map(|(_, status)| status.clone())
            .collect();

        Ok(filtered_assets)
    }

    /// Get resource usage for an asset
    pub async fn get_resource_usage(
        &self,
        asset_id: &AssetRegistration,
    ) -> AssetResult<ResourceUsage> {
        let asset_type = Self::category_to_asset_type(&asset_id.category)?;

        let adapters = self.adapters.read().await;
        let adapter = adapters
            .get(&asset_type)
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string(),
            })?;

        adapter.get_resource_usage(asset_id).await
    }

    /// Set resource limits for an asset
    pub async fn set_resource_limits(
        &self,
        asset_id: &AssetRegistration,
        limits: ResourceLimits,
    ) -> AssetResult<()> {
        let asset_type = Self::category_to_asset_type(&asset_id.category)?;

        let adapters = self.adapters.read().await;
        let adapter = adapters
            .get(&asset_type)
            .ok_or_else(|| AssetError::AssetNotFound {
                asset_id: asset_id.to_string(),
            })?;

        adapter.set_resource_limits(asset_id, limits).await
    }

    /// Validate state proof according to requirements using Proof of State Four-Proof System
    async fn validate_state_proof(&self, proof: &StateProof) -> AssetResult<bool> {
        // Use Proof of State comprehensive validation first
        if let Err(e) = proof.validate_comprehensive().await {
            return Err(AssetError::StateProofValidationFailed {
                reason: format!("Proof of State comprehensive validation failed: {e}"),
            });
        }

        // Basic validation check
        if !proof.validate() {
            return Err(AssetError::StateProofValidationFailed {
                reason: "Basic state proof validation failed".to_string(),
            });
        }

        // Check against HyperMesh asset requirements
        if self.state_requirements.require_all_proofs {
            // CANONICAL MODEL: PoStake is authorization (WHO), never a magnitude.
            // Require a bound identity (the FALCON identity binding), NOT a stake
            // amount above a threshold.
            if proof.stake_proof.stake_holder_id.is_empty() {
                return Err(AssetError::StateProofValidationFailed {
                    reason: "PoStake carries no bound identity (unauthorized)".to_string(),
                });
            }

            if proof.time_proof.network_time_offset > self.state_requirements.max_time_offset {
                return Err(AssetError::StateProofValidationFailed {
                    reason: "Time offset too large".to_string(),
                });
            }

            // CANONICAL MODEL: PoWork is the HASH of work done (WHAT), never a
            // resource-capacity magnitude. Require the work was actually hashed
            // (non-zero hash) — capacity is descriptive and never gated here.
            if proof.work_proof.work_hash == [0u8; 32] {
                return Err(AssetError::StateProofValidationFailed {
                    reason: "PoWork carries no work hash".to_string(),
                });
            }

            // PoSpace: CANONICAL MODEL — WHERE (location). Require a bound
            // location; capacity is descriptive and never gates admission.
            if proof.space_proof.node_id.is_empty()
                && proof.space_proof.storage_path.is_empty()
            {
                return Err(AssetError::StateProofValidationFailed {
                    reason: "PoSpace has no bound location (WHERE)".to_string(),
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
            // Derive asset type from category
            match &asset_id.category {
                AssetCategory::BaseSystem(sys) => match sys {
                    BaseSystemType::Cpu => stats.cpu_assets += 1,
                    BaseSystemType::Gpu => stats.gpu_assets += 1,
                    BaseSystemType::Memory => stats.memory_assets += 1,
                    BaseSystemType::Storage => stats.storage_assets += 1,
                    BaseSystemType::Network => stats.network_assets += 1,
                    BaseSystemType::Container => stats.container_assets += 1,
                    BaseSystemType::Economic => stats.economic_assets += 1,
                    BaseSystemType::Blockchain => stats.blockchain_assets += 1,
                    BaseSystemType::Dns => stats.dns_assets += 1,
                    BaseSystemType::Transmission => stats.transmission_assets += 1,
                    BaseSystemType::Dashboard => stats.dashboard_assets += 1,
                    BaseSystemType::Identity => stats.identity_assets += 1,
                    BaseSystemType::KeyRotation => { /* counted in identity_assets */ },
                    BaseSystemType::Invitation => { /* share invitations */ },
                    BaseSystemType::Message => { /* direct messages */ },
                },
                AssetCategory::Application(_) => {
                    // Application assets not tracked separately yet
                }
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
    pub blockchain_assets: usize,
    pub dns_assets: usize,
    pub transmission_assets: usize,
    pub dashboard_assets: usize,
    pub identity_assets: usize,
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
    fn test_state_proof_validation() {
        // Test Proof of State Four-Proof validation system integration
        let stake_proof = StakeProof::new("test-holder".to_string(), "test-holder-id".to_string());

        let space_proof = SpaceProof::new("test-node".to_string(), "/test/path".to_string(), 1024);

        // CANONICAL MODEL: PoWork carries the HASH of work done (WHAT).
        let work_proof = WorkProof::new(
            "test-worker".to_string(),
            "test-workload".to_string(),
            *blake3::hash(b"test-work").as_bytes(),
        );

        let time_proof = TimeProof::new(Duration::from_secs(10));

        // StateProof::new expects: (stake, time, space, work)
        let state_proof = StateProof::new(stake_proof, time_proof, space_proof, work_proof);

        // Test basic validation (synchronous)
        assert!(state_proof.validate());
    }

    #[tokio::test]
    async fn test_asset_manager_creation() {
        let manager = AssetManager::new();
        let stats = manager.get_asset_statistics().await;
        assert_eq!(stats.total_assets, 0);
    }
}
