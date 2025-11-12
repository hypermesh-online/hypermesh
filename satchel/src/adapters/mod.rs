//! Hardware asset adapters with consensus proof validation
//!
//! Implements specialized adapters for different hardware types:
//! - Memory: NAT-like addressing and remote proxy system (CRITICAL)
//! - CPU: Core management and scheduling
//! - GPU: Compute and memory management
//! - Storage: Distributed sharding with encryption
//! - Network: Bandwidth allocation
//! - Container: Resource orchestration
//!
//! All adapters implement the universal AssetAdapter trait and require
//! Consensus Proof validation (PoSpace + PoStake + PoWork + PoTime).

mod adapter_helpers;

pub mod memory;
pub mod cpu;
pub mod gpu;
pub mod storage;
pub mod network;
pub mod container;
pub mod economic;

// Re-exports
pub use memory::MemoryAssetAdapter;
pub use cpu::CpuAssetAdapter;
pub use gpu::GpuAssetAdapter;
pub use storage::StorageAssetAdapter;
pub use network::NetworkAssetAdapter;
pub use container::ContainerAssetAdapter;
pub use economic::EconomicAssetAdapter;

use crate::assets::core::{AssetAdapter, AssetType};
use std::sync::Arc;

/// Registry of all available asset adapters
pub struct AdapterRegistry {
    memory: Arc<MemoryAssetAdapter>,
    cpu: Arc<CpuAssetAdapter>,
    gpu: Arc<GpuAssetAdapter>,
    storage: Arc<StorageAssetAdapter>,
    network: Arc<NetworkAssetAdapter>,
    container: Arc<ContainerAssetAdapter>,
    economic: Arc<EconomicAssetAdapter>,
}

impl AdapterRegistry {
    /// Create new adapter registry with all asset adapters
    pub async fn new() -> Self {
        Self {
            memory: Arc::new(MemoryAssetAdapter::new().await),
            cpu: Arc::new(CpuAssetAdapter::new().await),
            gpu: Arc::new(GpuAssetAdapter::new().await),
            storage: Arc::new(StorageAssetAdapter::new().await),
            network: Arc::new(NetworkAssetAdapter::new().await),
            container: Arc::new(ContainerAssetAdapter::new().await),
            economic: Arc::new(EconomicAssetAdapter::new()),
        }
    }
    
    /// Get adapter for specific asset type
    pub fn get_adapter(&self, asset_type: &AssetType) -> Option<Arc<dyn AssetAdapter>> {
        match asset_type {
            AssetType::Memory => Some(self.memory.clone() as Arc<dyn AssetAdapter>),
            AssetType::Cpu => Some(self.cpu.clone() as Arc<dyn AssetAdapter>),
            AssetType::Gpu => Some(self.gpu.clone() as Arc<dyn AssetAdapter>),
            AssetType::Storage => Some(self.storage.clone() as Arc<dyn AssetAdapter>),
            AssetType::Network => Some(self.network.clone() as Arc<dyn AssetAdapter>),
            AssetType::Container => Some(self.container.clone() as Arc<dyn AssetAdapter>),
            AssetType::Economic => Some(self.economic.clone() as Arc<dyn AssetAdapter>),
        }
    }
    
    /// Get all available adapters as vector
    pub fn get_all_adapters(&self) -> Vec<(AssetType, Arc<dyn AssetAdapter>)> {
        vec![
            (AssetType::Memory, self.memory.clone() as Arc<dyn AssetAdapter>),
            (AssetType::Cpu, self.cpu.clone() as Arc<dyn AssetAdapter>),
            (AssetType::Gpu, self.gpu.clone() as Arc<dyn AssetAdapter>),
            (AssetType::Storage, self.storage.clone() as Arc<dyn AssetAdapter>),
            (AssetType::Network, self.network.clone() as Arc<dyn AssetAdapter>),
            (AssetType::Container, self.container.clone() as Arc<dyn AssetAdapter>),
            (AssetType::Economic, self.economic.clone() as Arc<dyn AssetAdapter>),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_adapter_registry_creation() {
        let registry = AdapterRegistry::new().await;
        
        // Test that all adapters are available
        assert!(registry.get_adapter(&AssetType::Memory).is_some());
        assert!(registry.get_adapter(&AssetType::Cpu).is_some());
        assert!(registry.get_adapter(&AssetType::Gpu).is_some());
        assert!(registry.get_adapter(&AssetType::Storage).is_some());
        assert!(registry.get_adapter(&AssetType::Network).is_some());
        assert!(registry.get_adapter(&AssetType::Container).is_some());
    }
    
    #[tokio::test]
    async fn test_get_all_adapters() {
        let registry = AdapterRegistry::new().await;
        let adapters = registry.get_all_adapters();
        
        assert_eq!(adapters.len(), 6);
        
        // Verify all asset types are represented
        let asset_types: Vec<AssetType> = adapters.iter().map(|(t, _)| t.clone()).collect();
        assert!(asset_types.contains(&AssetType::Memory));
        assert!(asset_types.contains(&AssetType::Cpu));
        assert!(asset_types.contains(&AssetType::Gpu));
        assert!(asset_types.contains(&AssetType::Storage));
        assert!(asset_types.contains(&AssetType::Network));
        assert!(asset_types.contains(&AssetType::Container));
    }
}