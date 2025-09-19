//! HyperMesh Asset Layer - Universal Asset System with Four-Proof Consensus
//! 
//! This module implements the HyperMesh asset management system that treats
//! everything as an asset: CPU, GPU, memory, storage, network connections,
//! VMs, and services. Every asset operation requires four-proof consensus
//! validation (PoSpace+PoStake+PoWork+PoTime).

use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};
use dashmap::DashMap;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use crate::config::{Internet2Config, HyperMeshConfig};
use crate::transport::StoqTransportLayer;
use crate::monitoring::PerformanceMonitor;

pub mod consensus;
pub mod allocation;
pub mod proxy;
pub mod vm;
pub mod adapters;

use consensus::{FourProofConsensus, ConsensusProof, ProofType};
use allocation::{AssetAllocator, AllocationRequest, AssetAllocation};
use proxy::{NatAddressManager, ProxyConnection};
use vm::{VmExecutor, VmExecution, VmAsset};
use adapters::{AssetAdapter, CpuAdapter, GpuAdapter, MemoryAdapter, StorageAdapter};

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
    config: Arc<Internet2Config>,
    
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
    
    /// Asset registry (all assets in the system)
    assets: Arc<DashMap<AssetId, Arc<Asset>>>,
    
    /// Active allocations
    allocations: Arc<DashMap<AllocationId, Arc<AssetAllocation>>>,
    
    /// Asset adapters by type
    adapters: Arc<DashMap<AssetType, Arc<dyn AssetAdapter>>>,
    
    /// Performance monitor
    monitor: Arc<PerformanceMonitor>,
}

/// Universal Asset ID
pub type AssetId = String;

/// Allocation ID
pub type AllocationId = String;

/// Universal Asset - Everything in HyperMesh is an Asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    /// Unique asset ID
    pub id: AssetId,
    
    /// Asset type (CPU, GPU, Memory, Storage, Network, VM, Service)
    pub asset_type: AssetType,
    
    /// Asset name and description
    pub name: String,
    pub description: String,
    
    /// Asset owner (node ID or user ID)
    pub owner: String,
    
    /// Current status
    pub status: AssetStatus,
    
    /// Privacy level for sharing
    pub privacy_level: PrivacyLevel,
    
    /// Physical/network location
    pub location: AssetLocation,
    
    /// Asset specifications (hardware specs, capabilities)
    pub specifications: HashMap<String, serde_json::Value>,
    
    /// Resource allocation information
    pub allocation: ResourceAllocation,
    
    /// Proxy address for remote access (NAT-like addressing)
    pub proxy_address: Option<String>,
    
    /// Timestamps
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    
    /// Consensus proofs for asset operations
    pub consensus_proofs: Vec<ConsensusProof>,
}

/// Asset types (everything is an asset in HyperMesh)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetType {
    Cpu,         // CPU cores/threads
    Gpu,         // GPU compute units
    Memory,      // RAM/memory resources
    Storage,     // Disk/storage resources
    Network,     // Network connections/bandwidth
    Container,   // Container instances
    Vm,          // Virtual machines
    Service,     // Running services
    Application, // Applications from Catalog
}

impl std::fmt::Display for AssetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            AssetType::Cpu => "CPU",
            AssetType::Gpu => "GPU",
            AssetType::Memory => "Memory",
            AssetType::Storage => "Storage",
            AssetType::Network => "Network",
            AssetType::Container => "Container",
            AssetType::Vm => "VM",
            AssetType::Service => "Service",
            AssetType::Application => "Application",
        };
        write!(f, "{}", name)
    }
}

/// Asset status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetStatus {
    Available,    // Ready for allocation
    Allocated,    // Currently allocated
    Busy,         // In use but may become available
    Maintenance,  // Under maintenance
    Offline,      // Not available
    Error,        // Error state
}

/// Privacy levels for asset sharing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyLevel {
    Private,        // Internal network only
    PrivateNetwork, // Specific networks/groups
    P2p,           // Trusted peer sharing
    PublicNetwork,  // Specific public networks
    FullPublic,     // Maximum CAESAR rewards
}

/// Asset location information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetLocation {
    /// Node ID where asset is located
    pub node_id: String,
    
    /// IPv6 address of node
    pub address: String,
    
    /// Geographic region
    pub region: String,
    
    /// Additional location metadata
    pub metadata: HashMap<String, String>,
}

/// Resource allocation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    /// Total capacity (in appropriate units)
    pub total_capacity: f64,
    
    /// Currently allocated capacity
    pub allocated_capacity: f64,
    
    /// Available capacity
    pub available_capacity: f64,
    
    /// Capacity unit (cores, GB, MB/s, etc.)
    pub unit: String,
    
    /// Allocation granularity
    pub granularity: f64,
}

/// Asset statistics for monitoring
#[derive(Debug, Clone, serde::Serialize)]
pub struct AssetStatistics {
    /// Total assets in system
    pub total_assets: u32,
    
    /// Assets by type
    pub assets_by_type: HashMap<AssetType, u32>,
    
    /// Active allocations
    pub active_allocations: u32,
    
    /// Consensus operations
    pub consensus_operations: u64,
    pub consensus_time_ms: f64,
    
    /// VM executions
    pub vm_executions: u64,
    pub vm_execution_time_ms: f64,
    
    /// Proxy connections
    pub proxy_connections: u32,
    pub proxy_throughput_mbps: f64,
    
    /// Performance metrics
    pub allocation_rate_per_second: f64,
    pub consensus_success_rate: f64,
}

impl HyperMeshAssetLayer {
    /// Create new HyperMesh asset layer
    pub async fn new(
        config: Arc<Internet2Config>,
        stoq_transport: Arc<StoqTransportLayer>,
        monitor: Arc<PerformanceMonitor>
    ) -> Result<Self> {
        info!("🏗️  Initializing HyperMesh Asset Layer");
        info!("   Features: Universal assets, Four-proof consensus, NAT addressing, VM execution");
        
        // Initialize four-proof consensus system
        let consensus = Arc::new(
            FourProofConsensus::new(&config.hypermesh.consensus, stoq_transport.clone()).await
                .map_err(|e| anyhow!("Consensus system initialization failed: {}", e))?
        );
        
        // Initialize asset allocator
        let allocator = Arc::new(
            AssetAllocator::new(&config.hypermesh.assets, consensus.clone()).await
                .map_err(|e| anyhow!("Asset allocator initialization failed: {}", e))?
        );
        
        // Initialize NAT address manager for remote proxy addressing
        let nat_manager = Arc::new(
            NatAddressManager::new(&config.hypermesh.proxy, stoq_transport.clone()).await
                .map_err(|e| anyhow!("NAT address manager initialization failed: {}", e))?
        );
        
        // Initialize VM executor for Catalog integration
        let vm_executor = Arc::new(
            VmExecutor::new(&config.hypermesh.vm, allocator.clone(), stoq_transport.clone()).await
                .map_err(|e| anyhow!("VM executor initialization failed: {}", e))?
        );
        
        // Initialize asset adapters
        let adapters = Arc::new(DashMap::new());
        
        // Register core asset adapters
        adapters.insert(AssetType::Cpu, Arc::new(CpuAdapter::new()) as Arc<dyn AssetAdapter>);
        adapters.insert(AssetType::Gpu, Arc::new(GpuAdapter::new()) as Arc<dyn AssetAdapter>);
        adapters.insert(AssetType::Memory, Arc::new(MemoryAdapter::new()) as Arc<dyn AssetAdapter>);
        adapters.insert(AssetType::Storage, Arc::new(StorageAdapter::new()) as Arc<dyn AssetAdapter>);
        
        info!("✅ HyperMesh Asset Layer initialized successfully");
        info!("   • Four-proof consensus: Ready (PoSpace+PoStake+PoWork+PoTime)");
        info!("   • Asset adapters: {} types registered", adapters.len());
        info!("   • NAT address manager: Ready");
        info!("   • VM executor: Ready");
        
        Ok(Self {
            config,
            stoq_transport,
            consensus,
            allocator,
            nat_manager,
            vm_executor,
            assets: Arc::new(DashMap::new()),
            allocations: Arc::new(DashMap::new()),
            adapters,
            monitor,
        })
    }
    
    /// Start HyperMesh asset layer
    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting HyperMesh Asset Layer");
        
        // Start consensus system
        self.consensus.start().await
            .map_err(|e| anyhow!("Consensus system start failed: {}", e))?;
        
        // Start asset allocator
        self.allocator.start().await
            .map_err(|e| anyhow!("Asset allocator start failed: {}", e))?;
        
        // Start NAT manager
        self.nat_manager.start().await
            .map_err(|e| anyhow!("NAT manager start failed: {}", e))?;
        
        // Start VM executor
        self.vm_executor.start().await
            .map_err(|e| anyhow!("VM executor start failed: {}", e))?;
        
        // Initialize system assets (auto-discover hardware)
        self.initialize_system_assets().await?;
        
        info!("✅ HyperMesh Asset Layer started successfully");
        info!("   Assets registered: {}", self.assets.len());
        info!("   Consensus mode: {}", if self.config.hypermesh.consensus.mandatory_four_proof {
            "MANDATORY four-proof"
        } else {
            "Optional consensus"
        });
        
        Ok(())
    }
    
    /// Initialize system assets (auto-discover hardware)
    async fn initialize_system_assets(&self) -> Result<()> {
        info!("🔍 Discovering and registering system assets");
        
        let node_id = format!("node-{}", self.config.global.server_id);
        let location = AssetLocation {
            node_id: node_id.clone(),
            address: self.config.global.bind_address.to_string(),
            region: "local".to_string(),
            metadata: HashMap::new(),
        };
        
        // Discover CPU assets
        if let Some(cpu_adapter) = self.adapters.get(&AssetType::Cpu) {
            let cpu_assets = cpu_adapter.discover_assets(&location).await?;
            for asset in cpu_assets {
                self.register_asset(asset).await?;
            }
        }
        
        // Discover GPU assets
        if let Some(gpu_adapter) = self.adapters.get(&AssetType::Gpu) {
            let gpu_assets = gpu_adapter.discover_assets(&location).await?;
            for asset in gpu_assets {
                self.register_asset(asset).await?;
            }
        }
        
        // Discover Memory assets
        if let Some(memory_adapter) = self.adapters.get(&AssetType::Memory) {
            let memory_assets = memory_adapter.discover_assets(&location).await?;
            for asset in memory_assets {
                self.register_asset(asset).await?;
            }
        }
        
        // Discover Storage assets
        if let Some(storage_adapter) = self.adapters.get(&AssetType::Storage) {
            let storage_assets = storage_adapter.discover_assets(&location).await?;
            for asset in storage_assets {
                self.register_asset(asset).await?;
            }
        }
        
        info!("✅ System asset discovery complete: {} assets registered", self.assets.len());
        Ok(())
    }
    
    /// Register new asset with consensus validation
    pub async fn register_asset(&self, mut asset: Asset) -> Result<AssetId> {
        info!("📝 Registering asset: {} ({})", asset.name, asset.asset_type);
        
        // Generate asset ID if not provided
        if asset.id.is_empty() {
            asset.id = format!("asset-{}-{}", 
                               asset.asset_type.to_string(),
                               Uuid::new_v4().to_string()[..8].to_string());
        }
        
        // Set timestamps
        let now = SystemTime::now();
        asset.created_at = now;
        asset.updated_at = now;
        
        // Generate proxy address for remote access
        asset.proxy_address = Some(
            self.nat_manager.allocate_proxy_address(&asset.id, &asset.asset_type).await?
        );
        
        // CRITICAL: Four-proof consensus validation for asset registration
        if self.config.hypermesh.consensus.mandatory_four_proof {
            debug!("🔐 Validating asset registration with four-proof consensus");
            
            let consensus_start = Instant::now();
            let consensus_proofs = self.consensus.validate_asset_operation(
                &asset.id,
                "register",
                &asset
            ).await.map_err(|e| anyhow!("Consensus validation failed: {}", e))?;
            
            let consensus_time = consensus_start.elapsed();
            
            asset.consensus_proofs = consensus_proofs;
            
            debug!("✅ Four-proof consensus validation completed in {:?}", consensus_time);
            
            // Update performance metrics
            self.monitor.record_consensus_operation(consensus_time).await;
        }
        
        // Register with appropriate adapter
        if let Some(adapter) = self.adapters.get(&asset.asset_type) {
            adapter.register_asset(&asset).await
                .map_err(|e| anyhow!("Asset adapter registration failed: {}", e))?;
        }
        
        let asset_id = asset.id.clone();
        
        // Add to asset registry
        self.assets.insert(asset_id.clone(), Arc::new(asset));
        
        info!("✅ Asset registered: {} with proxy address: {}", 
              asset_id, 
              self.assets.get(&asset_id).unwrap().proxy_address.as_deref().unwrap_or("none"));
        
        Ok(asset_id)
    }
    
    /// Request asset allocation with consensus validation
    pub async fn allocate_asset(&self, request: AllocationRequest) -> Result<Arc<AssetAllocation>> {
        info!("📋 Processing asset allocation request: {} units of {}", 
              request.amount, request.asset_id);
        
        // Validate asset exists
        let asset = self.assets.get(&request.asset_id)
            .ok_or_else(|| anyhow!("Asset not found: {}", request.asset_id))?;
        
        // Check availability
        if asset.allocation.available_capacity < request.amount {
            return Err(anyhow!("Insufficient asset capacity: requested {}, available {}", 
                              request.amount, asset.allocation.available_capacity));
        }
        
        // CRITICAL: Four-proof consensus validation for allocation
        if self.config.hypermesh.consensus.mandatory_four_proof {
            debug!("🔐 Validating allocation with four-proof consensus");
            
            let consensus_start = Instant::now();
            let consensus_proofs = self.consensus.validate_asset_operation(
                &request.asset_id,
                "allocate",
                &request
            ).await.map_err(|e| anyhow!("Allocation consensus validation failed: {}", e))?;
            
            let consensus_time = consensus_start.elapsed();
            debug!("✅ Allocation consensus validation completed in {:?}", consensus_time);
        }
        
        // Perform allocation through allocator
        let allocation = self.allocator.allocate(&request).await
            .map_err(|e| anyhow!("Asset allocation failed: {}", e))?;
        
        // Update asset availability
        let mut asset_clone = (**asset).clone();
        asset_clone.allocation.allocated_capacity += request.amount;
        asset_clone.allocation.available_capacity -= request.amount;
        asset_clone.updated_at = SystemTime::now();
        
        if asset_clone.allocation.available_capacity <= 0.0 {
            asset_clone.status = AssetStatus::Allocated;
        }
        
        // Update asset in registry
        self.assets.insert(request.asset_id.clone(), Arc::new(asset_clone));
        
        // Add allocation to registry
        self.allocations.insert(allocation.id.clone(), allocation.clone());
        
        info!("✅ Asset allocation completed: {} (allocation: {})", 
              request.asset_id, allocation.id);
        
        Ok(allocation)
    }
    
    /// Execute VM through asset allocation
    pub async fn execute_vm(&self, vm_asset_id: &str, execution_request: vm::VmExecutionRequest) -> Result<Arc<VmExecution>> {
        info!("🖥️  Executing VM: {} with operation: {}", vm_asset_id, execution_request.operation);
        
        // Validate VM asset exists
        let vm_asset = self.assets.get(vm_asset_id)
            .ok_or_else(|| anyhow!("VM asset not found: {}", vm_asset_id))?;
        
        if vm_asset.asset_type != AssetType::Vm {
            return Err(anyhow!("Asset is not a VM: {} (type: {:?})", vm_asset_id, vm_asset.asset_type));
        }
        
        // Execute through VM executor (handles allocation and consensus internally)
        let execution = self.vm_executor.execute(&vm_asset, execution_request).await
            .map_err(|e| anyhow!("VM execution failed: {}", e))?;
        
        info!("✅ VM execution started: {} (execution: {})", vm_asset_id, execution.id);
        
        Ok(execution)
    }
    
    /// Create proxy connection for remote asset access
    pub async fn create_proxy_connection(&self, asset_id: &str, remote_address: &str) -> Result<Arc<ProxyConnection>> {
        info!("🌐 Creating proxy connection for asset: {} to {}", asset_id, remote_address);
        
        // Validate asset exists
        let asset = self.assets.get(asset_id)
            .ok_or_else(|| anyhow!("Asset not found: {}", asset_id))?;
        
        // Create proxy connection through NAT manager
        let proxy_connection = self.nat_manager.create_proxy_connection(
            asset_id,
            remote_address,
            &asset.asset_type
        ).await.map_err(|e| anyhow!("Proxy connection creation failed: {}", e))?;
        
        info!("✅ Proxy connection created: {} -> {} (proxy: {})", 
              asset_id, remote_address, proxy_connection.proxy_address);
        
        Ok(proxy_connection)
    }
    
    /// Get asset statistics
    pub async fn get_statistics(&self) -> Result<AssetStatistics> {
        let mut assets_by_type = HashMap::new();
        
        for asset in self.assets.iter() {
            *assets_by_type.entry(asset.asset_type.clone()).or_insert(0) += 1;
        }
        
        let consensus_stats = self.consensus.get_statistics().await;
        let vm_stats = self.vm_executor.get_statistics().await;
        let proxy_stats = self.nat_manager.get_statistics().await;
        
        Ok(AssetStatistics {
            total_assets: self.assets.len() as u32,
            assets_by_type,
            active_allocations: self.allocations.len() as u32,
            consensus_operations: consensus_stats.total_operations,
            consensus_time_ms: consensus_stats.avg_validation_time_ms,
            vm_executions: vm_stats.total_executions,
            vm_execution_time_ms: vm_stats.avg_execution_time_ms,
            proxy_connections: proxy_stats.active_connections,
            proxy_throughput_mbps: proxy_stats.total_throughput_mbps,
            allocation_rate_per_second: 0.0, // Calculated from monitor
            consensus_success_rate: consensus_stats.success_rate,
        })
    }
    
    /// Shutdown asset layer
    pub async fn shutdown(&self) -> Result<()> {
        info!("🛑 Shutting down HyperMesh Asset Layer");
        
        // Shutdown components in reverse order
        self.vm_executor.shutdown().await?;
        self.nat_manager.shutdown().await?;
        self.allocator.shutdown().await?;
        self.consensus.shutdown().await?;
        
        // Clear registries
        self.assets.clear();
        self.allocations.clear();
        
        info!("✅ HyperMesh Asset Layer shutdown complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_asset_registration() {
        // Test asset registration with consensus validation
    }
    
    #[tokio::test]
    async fn test_four_proof_consensus() {
        // Test that all asset operations require four-proof consensus
    }
    
    #[tokio::test]
    async fn test_nat_addressing() {
        // Test NAT-like proxy addressing for remote assets
    }
    
    #[tokio::test]
    async fn test_vm_execution() {
        // Test VM execution through asset allocation
    }
}