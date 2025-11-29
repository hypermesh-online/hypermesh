//! HyperMesh - Gate 2 Asset System Restoration
//!
//! PHASE 2: Core Foundation - Asset System Implementation
//!
//! Restoring the complete asset management system with:
//! - Universal asset types (CPU, GPU, Memory, Storage)
//! - AssetId blockchain registration system
//! - AssetAdapter pattern for specialized handling
//! - Privacy-aware allocation types
//! - Consensus proof validation (PoSpace + PoStake + PoWork + PoTime)

#![warn(missing_docs)]
#![deny(unsafe_code)]

use anyhow::Result;
use std::sync::Arc;

/// Common types
pub type NodeId = String;
pub type ServiceId = String;

/// Minimal configuration
#[derive(Debug, Clone, Default)]
pub struct HyperMeshConfig {
    /// Placeholder
    pub enabled: bool,
}

/// HyperMesh system with asset management
pub struct HyperMeshSystem {
    /// System configuration
    config: HyperMeshConfig,
    /// Asset manager instance
    asset_manager: Arc<AssetManager>,
    /// Asset adapter registry
    adapter_registry: Arc<AdapterRegistry>,
    /// Extension manager
    extension_manager: Arc<UnifiedExtensionManager>,
}

impl HyperMeshSystem {
    /// Create system with asset management
    pub async fn new(config: HyperMeshConfig) -> Result<Self> {
        // Initialize asset manager
        let asset_manager = Arc::new(AssetManager::new());

        // Initialize adapter registry with all hardware adapters
        let adapter_registry = Arc::new(AdapterRegistry::new().await);

        // Register all adapters with the asset manager
        for (asset_type, adapter) in adapter_registry.get_all_adapters() {
            asset_manager.register_adapter(asset_type, adapter).await?;
        }

        // Initialize extension manager
        let extension_manager = Arc::new(UnifiedExtensionManager::new());

        tracing::info!("HyperMesh Asset System initialized with all adapters");

        Ok(Self {
            config,
            asset_manager,
            adapter_registry,
            extension_manager,
        })
    }

    /// Get asset manager reference
    pub fn asset_manager(&self) -> Arc<AssetManager> {
        Arc::clone(&self.asset_manager)
    }

    /// Get adapter registry reference
    pub fn adapter_registry(&self) -> Arc<AdapterRegistry> {
        Arc::clone(&self.adapter_registry)
    }

    /// Get extension manager reference
    pub fn extension_manager(&self) -> Arc<UnifiedExtensionManager> {
        Arc::clone(&self.extension_manager)
    }

    /// Shutdown system cleanly
    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("HyperMesh shutdown initiated");
        // Future: Add graceful cleanup of allocated assets
        Ok(())
    }
}

/// Initialize HyperMesh with full asset system
pub async fn initialize_hypermesh() -> Result<HyperMeshSystem> {
    tracing::info!("Initializing HyperMesh - Phase 2: Asset System");
    HyperMeshSystem::new(HyperMeshConfig::default()).await
}

/// Service endpoint (minimal)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServiceEndpoint {
    /// Service ID
    pub service_id: ServiceId,
    /// Node ID
    pub node_id: NodeId,
    /// Address
    pub address: std::net::SocketAddr,
}

/// Service mesh config (minimal)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServiceMeshConfig {
    /// Enabled
    pub enabled: bool,
    /// Load balancing
    pub load_balancing: LoadBalancingStrategy,
}

/// Load balancing strategy
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum LoadBalancingStrategy {
    /// Round robin
    RoundRobin,
    /// Random
    Random,
}

impl Default for ServiceMeshConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            load_balancing: LoadBalancingStrategy::RoundRobin,
        }
    }
}

// Core modules - Phase 2: Asset System Enabled
/// API module with STOQ consensus server
pub mod api;

// Real asset module from the codebase
pub mod assets;

// Consensus module (re-exports from TrustChain)
pub mod consensus;

// OS Integration Layer - Sprint 2: Cross-platform abstraction for hardware detection and eBPF
pub mod os_integration;

// Re-export main asset types for easy access
pub use assets::core::{
    AssetManager, AssetId, AssetType, AssetStatus, AssetState,
    AssetAllocation, PrivacyLevel, AssetError, AssetResult,
    ConsensusProof, ConsensusRequirements,
};

pub use assets::adapters::{
    CpuAssetAdapter, GpuAssetAdapter, MemoryAssetAdapter, StorageAssetAdapter,
    NetworkAssetAdapter, ContainerAssetAdapter, AdapterRegistry,
};

pub use extensions::UnifiedExtensionManager;

// Re-export OS integration types for easy access
pub use os_integration::{
    OsAbstraction,
    create_os_abstraction,
    types::{
        CpuInfo, GpuInfo, MemoryInfo, StorageInfo, ResourceUsage,
        EbpfHandle, EbpfAttachType, EbpfMetrics, EbpfMetricType,
        GpuType, StorageType,
    },
};

// Module stubs - some enabled for Gate 2
pub mod transport;
pub mod catalog;
pub mod container;
// consensus module already imported above
pub mod extensions;
pub mod orchestration;
pub mod platform;
pub mod integration;
pub mod security;

// Export container types at root for backwards compatibility
pub use container::{ContainerId, ContainerSpec, ContainerConfig, NetworkConfig};
// Export error module
pub mod error {
    pub use anyhow::{Result, Error};
}

// Integration types
pub type IntegrationResult<T> = anyhow::Result<T>;
pub type IntegrationError = anyhow::Error;
// Export config module
pub mod config {
    pub use super::container::config::ContainerConfig;

    /// Storage configuration
    #[derive(Debug, Clone, Default)]
    pub struct StorageConfig {
        /// Storage path
        pub path: String,
        /// Maximum size in bytes
        pub max_size: u64,
    }
}
// Runtime and monitoring as stubs for now
/// Runtime stub
pub mod runtime {
    /// Placeholder
    pub struct Runtime;
}
/// Monitoring stub
pub mod monitoring {
    /// Placeholder
    pub struct Monitor;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::proof_of_state_integration::{SpaceProof, StakeProof, WorkProof, TimeProof, WorkloadType, WorkState};
    use std::time::Duration;

    #[tokio::test]
    async fn test_gate_2_asset_system_initialization() {
        println!("==== GATE 2 TEST: Asset System ====");

        // Initialize system with asset management
        let system = initialize_hypermesh().await;
        assert!(system.is_ok(), "Gate 2: System initialization failed");

        let system = system.unwrap();

        // Verify asset manager exists
        let asset_manager = system.asset_manager();
        let stats = asset_manager.get_asset_statistics().await;
        assert_eq!(stats.total_assets, 0, "Initial asset count should be 0");

        // Verify adapter registry
        let adapter_registry = system.adapter_registry();
        assert!(adapter_registry.get_adapter(&AssetType::Cpu).is_some());
        assert!(adapter_registry.get_adapter(&AssetType::Gpu).is_some());
        assert!(adapter_registry.get_adapter(&AssetType::Memory).is_some());
        assert!(adapter_registry.get_adapter(&AssetType::Storage).is_some());

        // Clean shutdown
        let shutdown_result = system.shutdown().await;
        assert!(shutdown_result.is_ok(), "Gate 2: System shutdown failed");

        println!("✅ GATE 2 SUCCESS: Asset system initialization passed");
    }

    #[test]
    fn test_asset_types() {
        // Verify all required asset types exist
        let _cpu = AssetType::Cpu;
        let _gpu = AssetType::Gpu;
        let _memory = AssetType::Memory;
        let _storage = AssetType::Storage;
        let _network = AssetType::Network;
        let _container = AssetType::Container;

        println!("✅ All asset types defined");
    }

    #[test]
    fn test_privacy_levels() {
        // Verify privacy allocation types
        let _private = PrivacyLevel::Private;
        let _private_network = PrivacyLevel::PrivateNetwork;
        let _p2p = PrivacyLevel::P2P;
        let _public_network = PrivacyLevel::PublicNetwork;
        let _full_public = PrivacyLevel::FullPublic;

        println!("✅ All privacy levels defined");
    }

    #[test]
    fn test_consensus_proof_creation() {
        // Test Proof of State Four-Proof System integration
        let stake_proof = StakeProof::new(
            "test-holder".to_string(),
            "holder-id".to_string(),
            1000
        );

        let mut space_proof = SpaceProof::new(
            1024 * 1024, // 1MB
            "/test/storage".to_string()
        );
        space_proof.node_id = "test-node".to_string();

        let work_proof = WorkProof::new(
            100,
            "workload-1".to_string(),
            12345,
            "owner-1".to_string(),
            WorkloadType::Compute,
            WorkState::Completed,
        );

        let time_proof = TimeProof::new(Duration::from_secs(10));

        let consensus_proof = ConsensusProof::new(
            stake_proof,
            space_proof,
            work_proof,
            time_proof
        );

        // Basic validation should pass
        assert!(consensus_proof.validate());

        println!("✅ Consensus proof creation successful");
    }

    #[tokio::test]
    async fn test_asset_manager_operations() {
        let manager = AssetManager::new();

        // Get initial statistics
        let stats = manager.get_asset_statistics().await;
        assert_eq!(stats.total_assets, 0);
        assert_eq!(stats.cpu_assets, 0);
        assert_eq!(stats.gpu_assets, 0);
        assert_eq!(stats.memory_assets, 0);
        assert_eq!(stats.storage_assets, 0);

        println!("✅ Asset manager operations tested");
    }

    #[test]
    fn test_basic_types() {
        let _node_id: NodeId = "node1".to_string();
        let _service_id: ServiceId = "service1".to_string();
        let config = HyperMeshConfig::default();
        assert!(!config.enabled);
    }
}