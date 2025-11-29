//! Asset adapter trait and resource management
//!
//! Defines the universal AssetAdapter interface that all asset types
//! must implement for specialized handling while maintaining a unified interface.

use std::collections::HashMap;
use std::time::Duration;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{AssetId, AssetType, AssetResult, ConsensusProof};
use super::status::AssetStatus;
use super::privacy::{PrivacyLevel, AssetAllocation};
use super::proxy::ProxyAddress;

/// Universal asset adapter trait
/// All asset types must implement this for specialized handling
#[async_trait]
pub trait AssetAdapter: Send + Sync {
    /// Get the asset type this adapter handles
    fn asset_type(&self) -> AssetType;
    
    /// Validate consensus proof for this asset type
    /// CRITICAL: Every operation requires PoSpace + PoStake + PoWork + PoTime validation
    async fn validate_consensus_proof(&self, proof: &ConsensusProof) -> AssetResult<bool>;
    
    /// Allocate an asset instance
    async fn allocate_asset(&self, request: &AssetAllocationRequest) -> AssetResult<AssetAllocation>;
    
    /// Deallocate an asset instance
    async fn deallocate_asset(&self, asset_id: &AssetId) -> AssetResult<()>;
    
    /// Get current status of an asset
    async fn get_asset_status(&self, asset_id: &AssetId) -> AssetResult<AssetStatus>;
    
    /// Configure privacy level for asset sharing
    async fn configure_privacy_level(&self, asset_id: &AssetId, privacy: PrivacyLevel) -> AssetResult<()>;
    
    /// Assign remote proxy address (NAT-like system)
    async fn assign_proxy_address(&self, asset_id: &AssetId) -> AssetResult<ProxyAddress>;
    
    /// Resolve proxy address to local asset reference
    async fn resolve_proxy_address(&self, proxy_addr: &ProxyAddress) -> AssetResult<AssetId>;
    
    /// Get real-time resource usage
    async fn get_resource_usage(&self, asset_id: &AssetId) -> AssetResult<ResourceUsage>;
    
    /// Set resource limits
    async fn set_resource_limits(&self, asset_id: &AssetId, limits: ResourceLimits) -> AssetResult<()>;
    
    /// Health check for adapter functionality
    async fn health_check(&self) -> AssetResult<AdapterHealth>;
    
    /// Get adapter capabilities
    fn get_capabilities(&self) -> AdapterCapabilities;
}

/// Asset allocation request with consensus proof
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetAllocationRequest {
    /// Type of asset to allocate
    pub asset_type: AssetType,
    /// Requested resource specifications
    pub requested_resources: ResourceRequirements,
    /// Privacy level configuration
    pub privacy_level: PrivacyLevel,
    /// Consensus proof validation (ALL FOUR PROOFS REQUIRED)
    pub consensus_proof: ConsensusProof,
    /// Certificate fingerprint for authorization
    pub certificate_fingerprint: String,
    /// Optional duration limit for allocation
    pub duration_limit: Option<Duration>,
    /// Optional tags for categorization
    pub tags: HashMap<String, String>,
}

/// Resource requirements specification
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// CPU requirements
    pub cpu: Option<CpuRequirements>,
    /// GPU requirements
    pub gpu: Option<GpuRequirements>,
    /// Memory requirements
    pub memory: Option<MemoryRequirements>,
    /// Storage requirements
    pub storage: Option<StorageRequirements>,
    /// Network requirements
    pub network: Option<NetworkRequirements>,
    /// Container requirements
    pub container: Option<ContainerRequirements>,
    /// Economic requirements
    pub economic: Option<EconomicRequirements>,
}

/// CPU resource requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CpuRequirements {
    /// Number of CPU cores required
    pub cores: u32,
    /// Minimum CPU frequency in MHz
    pub min_frequency_mhz: Option<u32>,
    /// CPU architecture requirement (x86_64, arm64, etc.)
    pub architecture: Option<String>,
    /// CPU features required (AVX, SSE, etc.)
    pub required_features: Vec<String>,
}

/// GPU resource requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GpuRequirements {
    /// Number of GPU units required
    pub units: u32,
    /// Minimum GPU memory in MB
    pub min_memory_mb: Option<u64>,
    /// GPU type requirement (CUDA, OpenCL, etc.)
    pub compute_capability: Option<String>,
    /// Required GPU features
    pub required_features: Vec<String>,
}

/// Memory resource requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryRequirements {
    /// Required memory size in bytes
    pub size_bytes: u64,
    /// Memory type requirement (DDR4, DDR5, etc.)
    pub memory_type: Option<String>,
    /// ECC memory requirement
    pub ecc_required: bool,
    /// NUMA node preference
    pub numa_node: Option<u32>,
}

/// Storage resource requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageRequirements {
    /// Required storage size in bytes
    pub size_bytes: u64,
    /// Storage type (SSD, NVMe, HDD)
    pub storage_type: StorageType,
    /// IOPS requirement
    pub min_iops: Option<u32>,
    /// Bandwidth requirement in MB/s
    pub min_bandwidth_mbps: Option<u32>,
    /// Durability requirement (number of replicas)
    pub durability_replicas: u32,
}

/// Network resource requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkRequirements {
    /// Required bandwidth in Mbps
    pub bandwidth_mbps: u64,
    /// Latency requirement in microseconds
    pub max_latency_us: Option<u32>,
    /// Packet loss tolerance (percentage)
    pub max_packet_loss_percent: Option<f32>,
    /// Network protocols required
    pub protocols: Vec<String>,
}

/// Container resource requirements
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerRequirements {
    /// Container image specification
    pub image: String,
    /// CPU limit
    pub cpu_limit: f32,
    /// Memory limit in bytes
    pub memory_limit_bytes: u64,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Volume mounts
    pub volumes: Vec<VolumeMount>,
    /// Network ports
    pub ports: Vec<PortMapping>,
}

/// Storage type enumeration
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageType {
    /// Solid State Drive
    Ssd,
    /// NVMe storage
    Nvme,
    /// Hard Disk Drive
    Hdd,
    /// Memory-mapped storage
    Memory,
    /// Distributed storage
    Distributed,
    /// Network storage
    Network,
}

/// Container volume mount
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VolumeMount {
    /// Source path on host
    pub source: String,
    /// Target path in container
    pub target: String,
    /// Mount is read-only
    pub read_only: bool,
}

/// Container port mapping
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortMapping {
    /// Container port
    pub container_port: u16,
    /// Host port (optional, auto-assigned if None)
    pub host_port: Option<u16>,
    /// Protocol (TCP, UDP)
    pub protocol: String,
}

/// Current resource usage metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU usage metrics
    pub cpu_usage: Option<CpuUsage>,
    /// GPU usage metrics
    pub gpu_usage: Option<GpuUsage>,
    /// Memory usage metrics
    pub memory_usage: Option<MemoryUsage>,
    /// Storage usage metrics
    pub storage_usage: Option<StorageUsage>,
    /// Network usage metrics
    pub network_usage: Option<NetworkUsage>,
    /// Timestamp of measurement
    pub measurement_timestamp: std::time::SystemTime,
}

/// CPU usage metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CpuUsage {
    /// CPU utilization percentage (0.0 - 100.0)
    pub utilization_percent: f32,
    /// CPU frequency in MHz
    pub frequency_mhz: u32,
    /// Temperature in Celsius
    pub temperature_celsius: Option<f32>,
    /// Number of active cores
    pub active_cores: u32,
}

/// GPU usage metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GpuUsage {
    /// GPU utilization percentage (0.0 - 100.0)
    pub utilization_percent: f32,
    /// Memory utilization percentage (0.0 - 100.0)
    pub memory_utilization_percent: f32,
    /// Temperature in Celsius
    pub temperature_celsius: Option<f32>,
    /// Power consumption in watts
    pub power_watts: Option<f32>,
}

/// Memory usage metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryUsage {
    /// Used memory in bytes
    pub used_bytes: u64,
    /// Total memory in bytes
    pub total_bytes: u64,
    /// Cache usage in bytes
    pub cached_bytes: u64,
    /// Swap usage in bytes
    pub swap_used_bytes: u64,
}

/// Storage usage metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageUsage {
    /// Used storage in bytes
    pub used_bytes: u64,
    /// Total storage in bytes
    pub total_bytes: u64,
    /// Read IOPS
    pub read_iops: u32,
    /// Write IOPS
    pub write_iops: u32,
    /// Read throughput in MB/s
    pub read_mbps: f32,
    /// Write throughput in MB/s
    pub write_mbps: f32,
}

/// Network usage metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkUsage {
    /// Bytes received
    pub bytes_received: u64,
    /// Bytes transmitted
    pub bytes_transmitted: u64,
    /// Packets received
    pub packets_received: u64,
    /// Packets transmitted
    pub packets_transmitted: u64,
    /// Current latency in microseconds
    pub latency_us: Option<u32>,
}

/// Resource limits configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// CPU limits
    pub cpu_limit: Option<CpuLimit>,
    /// GPU limits
    pub gpu_limit: Option<GpuLimit>,
    /// Memory limits
    pub memory_limit: Option<MemoryLimit>,
    /// Storage limits
    pub storage_limit: Option<StorageLimit>,
    /// Network limits
    pub network_limit: Option<NetworkLimit>,
}

/// CPU resource limits
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CpuLimit {
    /// Maximum CPU cores
    pub max_cores: u32,
    /// Maximum CPU usage percentage
    pub max_utilization_percent: f32,
    /// Maximum CPU frequency in MHz
    pub max_frequency_mhz: Option<u32>,
}

/// GPU resource limits
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GpuLimit {
    /// Maximum GPU units
    pub max_units: u32,
    /// Maximum GPU memory in bytes
    pub max_memory_bytes: u64,
    /// Maximum GPU utilization percentage
    pub max_utilization_percent: f32,
}

/// Memory resource limits
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryLimit {
    /// Maximum memory in bytes
    pub max_bytes: u64,
    /// Maximum swap usage in bytes
    pub max_swap_bytes: u64,
}

/// Storage resource limits
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageLimit {
    /// Maximum storage in bytes
    pub max_bytes: u64,
    /// Maximum IOPS
    pub max_iops: u32,
    /// Maximum bandwidth in MB/s
    pub max_bandwidth_mbps: u32,
}

/// Network resource limits
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkLimit {
    /// Maximum bandwidth in Mbps
    pub max_bandwidth_mbps: u64,
    /// Maximum concurrent connections
    pub max_connections: u32,
}

/// Adapter health status
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdapterHealth {
    /// Adapter is healthy and operational
    pub healthy: bool,
    /// Health check message
    pub message: String,
    /// Last health check timestamp
    pub last_check: std::time::SystemTime,
    /// Performance metrics
    pub performance_metrics: HashMap<String, f64>,
}

/// Adapter capabilities
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdapterCapabilities {
    /// Asset type handled
    pub asset_type: AssetType,
    /// Supported privacy levels
    pub supported_privacy_levels: Vec<PrivacyLevel>,
    /// Supports remote proxy addressing
    pub supports_proxy_addressing: bool,
    /// Supports resource monitoring
    pub supports_resource_monitoring: bool,
    /// Supports dynamic resource limits
    pub supports_dynamic_limits: bool,
    /// Maximum concurrent allocations
    pub max_concurrent_allocations: Option<u32>,
    /// Additional features
    pub features: Vec<String>,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            cpu: None,
            gpu: None,
            memory: None,
            storage: None,
            network: None,
            container: None,
        }
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpu_limit: None,
            gpu_limit: None,
            memory_limit: None,
            storage_limit: None,
            network_limit: None,
        }
    }
}

/// Economic requirements for asset allocation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EconomicRequirements {
    /// Minimum stake required
    pub min_stake: Option<u64>,
    /// Maximum cost per hour
    pub max_cost_per_hour: Option<u64>,
    /// Preferred payment method
    pub payment_method: Option<String>,
    /// Budget limit
    pub budget_limit: Option<u64>,
}

/// Asset priority levels
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssetPriority {
    /// Lowest priority
    Low,
    /// Normal priority
    Normal,
    /// High priority
    High,
    /// Critical priority
    Critical,
    /// Emergency priority
    Emergency,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_resource_requirements_creation() {
        let requirements = ResourceRequirements {
            cpu: Some(CpuRequirements {
                cores: 4,
                min_frequency_mhz: Some(2400),
                architecture: Some("x86_64".to_string()),
                required_features: vec!["AVX2".to_string()],
            }),
            memory: Some(MemoryRequirements {
                size_bytes: 8 * 1024 * 1024 * 1024, // 8GB
                memory_type: Some("DDR4".to_string()),
                ecc_required: false,
                numa_node: None,
            }),
            ..Default::default()
        };
        
        assert!(requirements.cpu.is_some());
        assert!(requirements.memory.is_some());
        assert!(requirements.gpu.is_none());
    }
    
    #[test]
    fn test_storage_type_serialization() {
        let storage_type = StorageType::Nvme;
        let serialized = serde_json::to_string(&storage_type).unwrap();
        let deserialized: StorageType = serde_json::from_str(&serialized).unwrap();
        
        assert!(matches!(deserialized, StorageType::Nvme));
    }
}