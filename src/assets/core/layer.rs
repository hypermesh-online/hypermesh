//! HyperMesh Asset Layer Implementation
//!
//! Main implementation of the asset management layer.

use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::Arc;
use dashmap::DashMap;
use tracing::{info, debug, warn, error};

use crate::config::HyperMeshServerConfig;
use crate::transport::StoqTransportLayer;
use crate::monitoring::PerformanceMonitor;

use super::types::*;
use super::management::{AssetManager, AssetSystemStats};

// Import submodules
use crate::assets::consensus::{FourProofConsensus, ConsensusProof, ProofType};
use crate::assets::allocation::{AssetAllocator, AllocationRequest, AssetAllocation};
use crate::assets::proxy::{NatAddressManager, ProxyConnection};
use crate::assets::vm::{VmExecutor, VmExecution, VmAsset};
use crate::assets::adapters::{AssetAdapter, CpuAdapter, GpuAdapter, MemoryAdapter, StorageAdapter};

/// HyperMesh Asset Layer - Universal Asset System
///
/// Manages all resources as assets with consensus validation:
/// - CPU, GPU, Memory, Storage assets
/// - Network connections as assets
/// - VM execution through asset allocation
/// - NAT-like proxy addressing for remote resources
/// - Four-proof consensus for all operations
pub struct HyperMeshAssetLayer {
    /// Configuration
    config: Arc<HyperMeshServerConfig>,

    /// STOQ transport layer for all communications
    stoq_transport: Arc<StoqTransportLayer>,

    /// Four-proof consensus system
    consensus: Arc<FourProofConsensus>,

    /// Asset allocator for resource management
    allocator: Arc<AssetAllocator>,

    /// NAT address manager for remote proxy addressing
    nat_manager: Arc<NatAddressManager>,

    /// VM executor for Catalog integration
    vm_executor: Arc<VmExecutor>,

    /// Asset manager
    asset_manager: Arc<AssetManager>,

    /// Active allocations
    allocations: Arc<DashMap<AllocationId, Arc<AssetAllocation>>>,

    /// Asset adapters by type
    adapters: Arc<DashMap<AssetType, Arc<dyn AssetAdapter>>>,

    /// Performance monitor
    monitor: Arc<PerformanceMonitor>,
}

impl HyperMeshAssetLayer {
    /// Create new asset layer
    pub async fn new(
        config: Arc<HyperMeshServerConfig>,
        stoq_transport: Arc<StoqTransportLayer>,
    ) -> Result<Self> {
        info!("Initializing HyperMesh Asset Layer");

        let consensus = Arc::new(FourProofConsensus::new()?);
        let allocator = Arc::new(AssetAllocator::new());
        let nat_manager = Arc::new(NatAddressManager::new()?);
        let vm_executor = Arc::new(VmExecutor::new());
        let asset_manager = Arc::new(AssetManager::new());
        let monitor = Arc::new(PerformanceMonitor::new());

        let layer = Self {
            config,
            stoq_transport,
            consensus,
            allocator,
            nat_manager,
            vm_executor,
            asset_manager,
            allocations: Arc::new(DashMap::new()),
            adapters: Arc::new(DashMap::new()),
            monitor,
        };

        layer.register_default_adapters().await?;
        layer.discover_local_assets().await?;

        info!("Asset Layer initialized successfully");
        Ok(layer)
    }

    /// Register default asset adapters
    async fn register_default_adapters(&self) -> Result<()> {
        self.register_adapter(AssetType::Cpu, Arc::new(CpuAdapter::new())).await?;
        self.register_adapter(AssetType::Gpu, Arc::new(GpuAdapter::new())).await?;
        self.register_adapter(AssetType::Memory, Arc::new(MemoryAdapter::new())).await?;
        self.register_adapter(AssetType::Storage, Arc::new(StorageAdapter::new())).await?;
        Ok(())
    }

    /// Register an asset adapter
    pub async fn register_adapter(
        &self,
        asset_type: AssetType,
        adapter: Arc<dyn AssetAdapter>,
    ) -> Result<()> {
        self.adapters.insert(asset_type, adapter);
        debug!("Registered adapter for asset type: {}", asset_type);
        Ok(())
    }

    /// Discover and register local assets
    async fn discover_local_assets(&self) -> Result<()> {
        info!("Discovering local assets");

        // Discover CPU assets
        if let Some(cpu_adapter) = self.adapters.get(&AssetType::Cpu) {
            let cpu_assets = cpu_adapter.discover_assets().await?;
            for asset in cpu_assets {
                self.asset_manager.register_asset(asset)?;
            }
        }

        // Discover GPU assets
        if let Some(gpu_adapter) = self.adapters.get(&AssetType::Gpu) {
            let gpu_assets = gpu_adapter.discover_assets().await?;
            for asset in gpu_assets {
                self.asset_manager.register_asset(asset)?;
            }
        }

        // Discover Memory assets
        if let Some(memory_adapter) = self.adapters.get(&AssetType::Memory) {
            let memory_assets = memory_adapter.discover_assets().await?;
            for asset in memory_assets {
                self.asset_manager.register_asset(asset)?;
            }
        }

        // Discover Storage assets
        if let Some(storage_adapter) = self.adapters.get(&AssetType::Storage) {
            let storage_assets = storage_adapter.discover_assets().await?;
            for asset in storage_assets {
                self.asset_manager.register_asset(asset)?;
            }
        }

        Ok(())
    }

    /// Register a new asset
    pub async fn register_asset(&self, asset: Asset) -> Result<AssetId> {
        // Validate asset with consensus
        let proof = self.consensus.generate_proof(
            &asset.id,
            ProofType::Space,
            &asset.owner,
        ).await?;

        let mut validated_asset = asset;
        validated_asset.consensus_proofs.insert(
            "registration".to_string(),
            proof.to_bytes(),
        );

        self.asset_manager.register_asset(validated_asset)
    }

    /// Allocate assets for a request
    pub async fn allocate_assets(
        &self,
        request: AllocationRequest,
    ) -> Result<AssetAllocation> {
        info!("Processing allocation request: {}", request.id);

        // Validate request with four-proof consensus
        self.validate_allocation_request(&request).await?;

        // Find available assets
        let available = self.asset_manager.find_available_assets(
            request.asset_type,
            &request.resources,
            request.privacy_level,
        );

        if available.is_empty() {
            return Err(anyhow!("No available assets matching request"));
        }

        // Select best asset
        let asset = self.select_best_asset(&available, &request)?;

        // Create allocation
        let allocation = self.allocator.create_allocation(
            &request,
            &asset,
        ).await?;

        // Update asset allocation
        self.asset_manager.update_allocation(
            &asset.id,
            allocation.allocated_resources.clone(),
        )?;

        // Store allocation
        self.allocations.insert(
            allocation.id.clone(),
            Arc::new(allocation.clone()),
        );

        info!("Allocation successful: {}", allocation.id);
        Ok(allocation)
    }

    /// Validate allocation request with consensus
    async fn validate_allocation_request(&self, request: &AllocationRequest) -> Result<()> {
        // Generate four proofs
        let space_proof = self.consensus.generate_proof(
            &request.id,
            ProofType::Space,
            &request.requestor,
        ).await?;

        let stake_proof = self.consensus.generate_proof(
            &request.id,
            ProofType::Stake,
            &request.requestor,
        ).await?;

        let work_proof = self.consensus.generate_proof(
            &request.id,
            ProofType::Work,
            &request.requestor,
        ).await?;

        let time_proof = self.consensus.generate_proof(
            &request.id,
            ProofType::Time,
            &request.requestor,
        ).await?;

        // Validate all proofs
        self.consensus.validate_proof(&space_proof).await?;
        self.consensus.validate_proof(&stake_proof).await?;
        self.consensus.validate_proof(&work_proof).await?;
        self.consensus.validate_proof(&time_proof).await?;

        Ok(())
    }

    /// Select best asset from available options
    fn select_best_asset(
        &self,
        available: &[Arc<Asset>],
        request: &AllocationRequest,
    ) -> Result<Arc<Asset>> {
        // Simple selection: pick first available
        // TODO: Implement sophisticated selection based on:
        // - Network proximity
        // - Performance metrics
        // - Cost optimization
        // - Load balancing
        available
            .first()
            .cloned()
            .ok_or_else(|| anyhow!("No suitable asset found"))
    }

    /// Release an allocation
    pub async fn release_allocation(&self, allocation_id: &str) -> Result<()> {
        let allocation = self.allocations
            .remove(allocation_id)
            .ok_or_else(|| anyhow!("Allocation not found"))?
            .1;

        // Update asset to release resources
        let asset = self.asset_manager.get_asset(&allocation.asset_id)?;

        // Calculate new allocated resources
        let mut new_allocated = asset.allocated.clone();
        if let Some(cpu) = new_allocated.cpu_units {
            if let Some(released) = allocation.allocated_resources.cpu_units {
                new_allocated.cpu_units = Some((cpu - released).max(0.0));
            }
        }
        // Similar for other resources...

        self.asset_manager.update_allocation(&asset.id, new_allocated)?;

        info!("Released allocation: {}", allocation_id);
        Ok(())
    }

    /// Execute VM through asset allocation
    pub async fn execute_vm(&self, vm_asset: VmAsset) -> Result<VmExecution> {
        // Validate VM asset
        let asset = self.asset_manager.get_asset(&vm_asset.asset_id)?;

        if asset.asset_type != AssetType::Vm {
            return Err(anyhow!("Asset is not a VM"));
        }

        // Execute through VM executor
        self.vm_executor.execute(vm_asset).await
    }

    /// Create remote proxy connection
    pub async fn create_proxy_connection(
        &self,
        asset_id: &str,
        remote_address: &str,
    ) -> Result<ProxyConnection> {
        let asset = self.asset_manager.get_asset(asset_id)?;

        // Create NAT-like proxy address
        let proxy_address = self.nat_manager
            .create_proxy_address(&asset.id, remote_address)
            .await?;

        // Establish connection through STOQ
        let connection = ProxyConnection {
            asset_id: asset.id.clone(),
            proxy_address: proxy_address.clone(),
            remote_address: remote_address.to_string(),
            established: true,
        };

        // Update asset with proxy address
        let mut updated_asset = (*asset).clone();
        updated_asset.proxy_address = Some(proxy_address);
        self.asset_manager.register_asset(updated_asset)?;

        Ok(connection)
    }

    /// Get asset statistics
    pub async fn get_statistics(&self) -> AssetSystemStats {
        self.asset_manager.get_statistics()
    }

    /// Get allocation by ID
    pub async fn get_allocation(&self, allocation_id: &str) -> Result<Arc<AssetAllocation>> {
        self.allocations
            .get(allocation_id)
            .map(|entry| entry.clone())
            .ok_or_else(|| anyhow!("Allocation not found"))
    }

    /// List all active allocations
    pub async fn list_allocations(&self) -> Vec<Arc<AssetAllocation>> {
        self.allocations
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }
}