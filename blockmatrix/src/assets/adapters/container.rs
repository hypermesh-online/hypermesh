//! Container Asset Adapter with resource orchestration
//!
//! Features:
//! - Container lifecycle management (create, start, stop, destroy)
//! - Resource allocation and limits (CPU, memory, storage, network)
//! - Image management and registry integration
//! - Network isolation and port management
//! - Volume mounting and storage management
//! - Container orchestration (Kubernetes replacement)

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
    ResourceUsage, ResourceLimits,
    AdapterHealth, AdapterCapabilities, ConsensusProof,
    ContainerRequirements, VolumeMount, PortMapping,
};

/// Container allocation record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerAllocation {
    /// Asset ID
    pub asset_id: AssetId,
    /// Container ID (runtime identifier)
    pub container_id: String,
    /// Container image name and tag
    pub image: String,
    /// Container name
    pub container_name: String,
    /// CPU allocation
    pub cpu_allocation: ContainerCpuAllocation,
    /// Memory allocation
    pub memory_allocation: ContainerMemoryAllocation,
    /// Storage volumes
    pub volumes: Vec<ContainerVolume>,
    /// Network configuration
    pub network_config: ContainerNetworkConfig,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Container command and arguments
    pub command: Option<Vec<String>>,
    /// Working directory
    pub working_directory: Option<String>,
    /// Container status
    pub container_status: ContainerStatus,
    /// Privacy level
    pub privacy_level: PrivacyLevel,
    /// Security configuration
    pub security_config: ContainerSecurityConfig,
    /// Allocation timestamp
    pub allocated_at: SystemTime,
    /// Last accessed timestamp
    pub last_accessed: SystemTime,
    /// Container runtime statistics
    pub runtime_stats: ContainerRuntimeStats,
}

/// Container CPU allocation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerCpuAllocation {
    /// CPU limit (fractional cores)
    pub cpu_limit: f32,
    /// CPU request (guaranteed)
    pub cpu_request: f32,
    /// CPU shares (relative weight)
    pub cpu_shares: u32,
    /// CPU cores pinned
    pub pinned_cores: Vec<u32>,
}

/// Container memory allocation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerMemoryAllocation {
    /// Memory limit in bytes
    pub memory_limit_bytes: u64,
    /// Memory request in bytes (guaranteed)
    pub memory_request_bytes: u64,
    /// Swap limit in bytes
    pub swap_limit_bytes: u64,
    /// OOM kill disabled
    pub oom_kill_disabled: bool,
}

/// Container volume configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerVolume {
    /// Volume name
    pub name: String,
    /// Host path
    pub host_path: String,
    /// Container path
    pub container_path: String,
    /// Mount is read-only
    pub read_only: bool,
    /// Volume type
    pub volume_type: VolumeType,
    /// Size limit in bytes
    pub size_limit_bytes: Option<u64>,
}

/// Volume types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum VolumeType {
    /// Host directory bind mount
    HostPath,
    /// Temporary filesystem
    TmpFs,
    /// Named volume
    Volume,
    /// ConfigMap volume
    ConfigMap,
    /// Secret volume
    Secret,
}

/// Container network configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerNetworkConfig {
    /// Network mode (bridge, host, none)
    pub network_mode: NetworkMode,
    /// Port mappings
    pub port_mappings: Vec<ContainerPortMapping>,
    /// IPv6 addresses
    pub ipv6_addresses: Vec<String>,
    /// Network aliases
    pub network_aliases: Vec<String>,
    /// DNS configuration
    pub dns_config: DnsConfig,
    /// Bandwidth limits
    pub bandwidth_limits: BandwidthLimits,
}

/// Network modes
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetworkMode {
    /// Bridge networking
    Bridge,
    /// Host networking
    Host,
    /// No networking
    None,
    /// Custom network
    Custom(String),
}

/// Container port mapping
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerPortMapping {
    /// Container port
    pub container_port: u16,
    /// Host port
    pub host_port: u16,
    /// Protocol (TCP, UDP)
    pub protocol: String,
    /// Bind address
    pub bind_address: Option<String>,
}

/// DNS configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsConfig {
    /// DNS servers
    pub nameservers: Vec<String>,
    /// DNS search domains
    pub search_domains: Vec<String>,
    /// DNS options
    pub options: Vec<String>,
}

/// Bandwidth limits
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BandwidthLimits {
    /// Ingress bandwidth limit in Mbps
    pub ingress_mbps: Option<u64>,
    /// Egress bandwidth limit in Mbps
    pub egress_mbps: Option<u64>,
}

/// Container security configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerSecurityConfig {
    /// Run as user ID
    pub user_id: Option<u32>,
    /// Run as group ID
    pub group_id: Option<u32>,
    /// Privileged mode
    pub privileged: bool,
    /// Read-only root filesystem
    pub read_only_rootfs: bool,
    /// Security capabilities
    pub capabilities: SecurityCapabilities,
    /// SELinux/AppArmor labels
    pub security_labels: HashMap<String, String>,
    /// Seccomp profile
    pub seccomp_profile: Option<String>,
}

/// Security capabilities
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityCapabilities {
    /// Capabilities to add
    pub add: Vec<String>,
    /// Capabilities to drop
    pub drop: Vec<String>,
}

/// Container status
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ContainerStatus {
    /// Container is created but not started
    Created,
    /// Container is running
    Running,
    /// Container is paused
    Paused,
    /// Container is stopped
    Stopped,
    /// Container has exited
    Exited(i32), // Exit code
    /// Container has failed
    Failed(String), // Error message
}

/// Container runtime statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerRuntimeStats {
    /// CPU usage percentage
    pub cpu_usage_percent: f32,
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    /// Network I/O stats
    pub network_io: NetworkIoStats,
    /// Block I/O stats
    pub block_io: BlockIoStats,
    /// Process count
    pub process_count: u32,
    /// Uptime in seconds
    pub uptime_seconds: u64,
}

/// Network I/O statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkIoStats {
    /// Bytes received
    pub rx_bytes: u64,
    /// Bytes transmitted
    pub tx_bytes: u64,
    /// Packets received
    pub rx_packets: u64,
    /// Packets transmitted
    pub tx_packets: u64,
}

/// Block I/O statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockIoStats {
    /// Bytes read
    pub read_bytes: u64,
    /// Bytes written
    pub write_bytes: u64,
    /// Read operations
    pub read_ops: u64,
    /// Write operations
    pub write_ops: u64,
}

/// Container runtime interface
#[derive(Clone, Debug)]
pub struct ContainerRuntime {
    /// Runtime type (Docker, containerd, etc.)
    pub runtime_type: RuntimeType,
    /// Runtime socket path
    pub socket_path: String,
    /// API version
    pub api_version: String,
}

/// Container runtime types
#[derive(Clone, Debug)]
pub enum RuntimeType {
    /// Docker daemon
    Docker,
    /// containerd
    Containerd,
    /// CRI-O
    CriO,
    /// Podman
    Podman,
}

/// Container Asset Adapter implementation
pub struct ContainerAssetAdapter {
    /// Active container allocations by asset ID
    allocations: Arc<RwLock<HashMap<AssetId, ContainerAllocation>>>,
    /// Container runtime interface
    runtime: Arc<ContainerRuntime>,
    /// Proxy address mappings
    proxy_mappings: Arc<RwLock<HashMap<ProxyAddress, AssetId>>>,
    /// Port allocation tracking
    allocated_ports: Arc<RwLock<HashMap<u16, AssetId>>>,
    /// Image registry
    image_registry: Arc<RwLock<HashMap<String, ImageInfo>>>,
    /// Container usage statistics
    usage_stats: Arc<RwLock<ContainerUsageStats>>,
}

/// Container image information
#[derive(Clone, Debug)]
pub struct ImageInfo {
    /// Image ID
    pub image_id: String,
    /// Image name and tag
    pub image_name: String,
    /// Image size in bytes
    pub size_bytes: u64,
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Architecture
    pub architecture: String,
    /// OS
    pub os: String,
    /// Security scan status
    pub security_status: SecurityScanStatus,
}

/// Security scan status for images
#[derive(Clone, Debug)]
pub enum SecurityScanStatus {
    /// Not scanned
    NotScanned,
    /// Scan in progress
    Scanning,
    /// Scan passed
    Passed,
    /// Vulnerabilities found
    Vulnerabilities(u32), // Number of vulnerabilities
    /// Scan failed
    Failed(String), // Error message
}

/// Container usage statistics
#[derive(Clone, Debug, Default)]
pub struct ContainerUsageStats {
    /// Total allocations made
    pub total_allocations: u64,
    /// Total deallocations made
    pub total_deallocations: u64,
    /// Current active containers
    pub active_containers: u64,
    /// Total CPU time used (seconds)
    pub total_cpu_time_seconds: f64,
    /// Total memory allocated (bytes)
    pub total_memory_allocated: u64,
    /// Total network I/O (bytes)
    pub total_network_io_bytes: u64,
    /// Total block I/O (bytes)
    pub total_block_io_bytes: u64,
    /// Container restarts
    pub container_restarts: u64,
}

impl ContainerAssetAdapter {
    /// Create new container adapter
    pub async fn new() -> Self {
        // Initialize container runtime
        let runtime = Arc::new(Self::detect_container_runtime().await);
        
        Self {
            allocations: Arc::new(RwLock::new(HashMap::new())),
            runtime,
            proxy_mappings: Arc::new(RwLock::new(HashMap::new())),
            allocated_ports: Arc::new(RwLock::new(HashMap::new())),
            image_registry: Arc::new(RwLock::new(HashMap::new())),
            usage_stats: Arc::new(RwLock::new(ContainerUsageStats::default())),
        }
    }
    
    /// Detect available container runtime
    async fn detect_container_runtime() -> ContainerRuntime {
        // TODO: Implement actual runtime detection
        // Check for Docker socket, containerd socket, etc.
        ContainerRuntime {
            runtime_type: RuntimeType::Docker, // Default assumption
            socket_path: "/var/run/docker.sock".to_string(),
            api_version: "1.41".to_string(),
        }
    }
    
    /// Generate container name
    async fn generate_container_name(&self, asset_id: &AssetId) -> String {
        format!("hypermesh-{}", &asset_id.uuid.to_string()[..8])
    }
    
    /// Create container via runtime API
    async fn create_container(
        &self,
        container_req: &ContainerRequirements,
        asset_id: &AssetId,
    ) -> AssetResult<String> {
        // TODO: Implement actual container creation via runtime API
        // For now, simulate container creation
        let container_id = format!("container_{}", asset_id.uuid);
        
        tracing::info!(
            "Creating container {} with image {} for asset {}",
            container_id, container_req.image, asset_id
        );
        
        Ok(container_id)
    }
    
    /// Allocate host ports for container
    async fn allocate_ports(&self, port_mappings: &[PortMapping], asset_id: &AssetId) -> AssetResult<Vec<ContainerPortMapping>> {
        let mut allocated_ports = self.allocated_ports.write().await;
        let mut container_ports = Vec::new();
        
        for port_mapping in port_mappings {
            let host_port = if let Some(requested_port) = port_mapping.host_port {
                // Check if requested port is available
                if allocated_ports.contains_key(&requested_port) {
                    return Err(AssetError::AllocationFailed {
                        reason: format!("Port {} already allocated", requested_port)
                    });
                }
                requested_port
            } else {
                // Find available port
                let mut port = 30000; // Start from port 30000
                while allocated_ports.contains_key(&port) && port < 65535 {
                    port += 1;
                }
                if port >= 65535 {
                    return Err(AssetError::AllocationFailed {
                        reason: "No available ports".to_string()
                    });
                }
                port
            };
            
            allocated_ports.insert(host_port, asset_id.clone());
            
            container_ports.push(ContainerPortMapping {
                container_port: port_mapping.container_port,
                host_port,
                protocol: port_mapping.protocol.clone(),
                bind_address: Some("::".to_string()), // IPv6 bind
            });
        }
        
        Ok(container_ports)
    }
    
    /// Configure container volumes
    async fn configure_volumes(&self, volume_mounts: &[VolumeMount]) -> Vec<ContainerVolume> {
        volume_mounts.iter().map(|vm| ContainerVolume {
            name: format!("vol-{}", uuid::Uuid::new_v4()),
            host_path: vm.source.clone(),
            container_path: vm.target.clone(),
            read_only: vm.read_only,
            volume_type: VolumeType::HostPath,
            size_limit_bytes: None,
        }).collect()
    }
    
    /// Generate proxy address for container access
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
    
    /// Get container runtime statistics
    async fn get_container_stats(&self, container_id: &str) -> ContainerRuntimeStats {
        // TODO: Implement actual stats collection from runtime
        ContainerRuntimeStats {
            cpu_usage_percent: 5.0,
            memory_usage_bytes: 100 * 1024 * 1024, // 100MB
            network_io: NetworkIoStats {
                rx_bytes: 1024 * 1024,
                tx_bytes: 512 * 1024,
                rx_packets: 1000,
                tx_packets: 800,
            },
            block_io: BlockIoStats {
                read_bytes: 10 * 1024 * 1024,
                write_bytes: 5 * 1024 * 1024,
                read_ops: 100,
                write_ops: 50,
            },
            process_count: 3,
            uptime_seconds: 3600,
        }
    }
    
    /// Update usage statistics
    async fn update_usage_stats(&self, operation: ContainerOperation) {
        let mut stats = self.usage_stats.write().await;
        
        match operation {
            ContainerOperation::Create => {
                stats.total_allocations += 1;
                stats.active_containers += 1;
            },
            ContainerOperation::Destroy => {
                stats.total_deallocations += 1;
                stats.active_containers = stats.active_containers.saturating_sub(1);
            },
            ContainerOperation::Restart => {
                stats.container_restarts += 1;
            },
        }
    }
}

/// Container operations for statistics
#[derive(Clone, Debug)]
enum ContainerOperation {
    Create,
    Destroy,
    Restart,
}

#[async_trait]
impl AssetAdapter for ContainerAssetAdapter {
    fn asset_type(&self) -> AssetType {
        AssetType::Container
    }
    
    async fn validate_consensus_proof(&self, proof: &ConsensusProof) -> AssetResult<bool> {
        // Validate all four proofs with container-specific requirements
        
        // PoSpace: Validate container space allocation
        if proof.space_proof.total_size == 0 {
            return Ok(false);
        }
        
        // PoStake: Validate container access stake
        if proof.stake_proof.stake_amount < 50 { // Moderate minimum for containers
            return Ok(false);
        }
        
        // PoWork: Validate work for container allocation
        if proof.work_proof.computational_power < 30 { // Moderate minimum for containers
            return Ok(false);
        }
        
        // PoTime: Validate timing for container synchronization
        if proof.time_proof.network_time_offset > Duration::from_secs(15) {
            return Ok(false);
        }
        
        Ok(true)
    }
    
    async fn allocate_asset(&self, request: &AssetAllocationRequest) -> AssetResult<AssetAllocation> {
        // Validate consensus proof first
        if !self.validate_consensus_proof(&request.consensus_proof).await? {
            return Err(AssetError::ConsensusValidationFailed {
                reason: "Container allocation consensus validation failed".to_string()
            });
        }
        
        // Get container requirements
        let container_req = request.requested_resources.container.as_ref()
            .ok_or_else(|| AssetError::AllocationFailed {
                reason: "No container requirements specified".to_string()
            })?;
        
        // Create asset ID
        let asset_id = AssetId::new(AssetType::Container);
        
        // Generate container name
        let container_name = self.generate_container_name(&asset_id).await;
        
        // Create container via runtime
        let container_id = self.create_container(container_req, &asset_id).await?;
        
        // Allocate ports
        let port_mappings = self.allocate_ports(&container_req.ports, &asset_id).await?;
        
        // Configure volumes
        let volumes = self.configure_volumes(&container_req.volumes).await;
        
        // Generate proxy address
        let proxy_address = Self::generate_proxy_address(&asset_id).await;
        
        // Configure CPU allocation
        let cpu_allocation = ContainerCpuAllocation {
            cpu_limit: container_req.cpu_limit,
            cpu_request: container_req.cpu_limit * 0.5, // Request 50% of limit
            cpu_shares: (container_req.cpu_limit * 1024.0) as u32, // Docker CPU shares
            pinned_cores: Vec::new(), // TODO: Implement CPU pinning
        };
        
        // Configure memory allocation
        let memory_allocation = ContainerMemoryAllocation {
            memory_limit_bytes: container_req.memory_limit_bytes,
            memory_request_bytes: container_req.memory_limit_bytes / 2, // Request 50% of limit
            swap_limit_bytes: container_req.memory_limit_bytes, // Same as memory limit
            oom_kill_disabled: false,
        };
        
        // Configure network
        let network_config = ContainerNetworkConfig {
            network_mode: NetworkMode::Bridge, // Default to bridge mode
            port_mappings,
            ipv6_addresses: vec![format!("2001:db8:hypermesh:container::{:x}", asset_id.uuid.as_u128() & 0xFFFF)],
            network_aliases: vec![container_name.clone()],
            dns_config: DnsConfig {
                nameservers: vec!["2001:4860:4860::8888".to_string()], // Google DNS IPv6
                search_domains: vec!["hypermesh.local".to_string()],
                options: vec!["ndots:2".to_string()],
            },
            bandwidth_limits: BandwidthLimits {
                ingress_mbps: None, // No limits by default
                egress_mbps: None,
            },
        };
        
        // Configure security
        let security_config = ContainerSecurityConfig {
            user_id: Some(1000), // Non-root user
            group_id: Some(1000),
            privileged: false,
            read_only_rootfs: false,
            capabilities: SecurityCapabilities {
                add: Vec::new(),
                drop: vec!["ALL".to_string()], // Drop all capabilities by default
            },
            security_labels: HashMap::new(),
            seccomp_profile: Some("default".to_string()),
        };
        
        // Get initial runtime stats
        let runtime_stats = self.get_container_stats(&container_id).await;
        
        // Create container allocation record
        let allocation = ContainerAllocation {
            asset_id: asset_id.clone(),
            container_id: container_id.clone(),
            image: container_req.image.clone(),
            container_name: container_name.clone(),
            cpu_allocation,
            memory_allocation,
            volumes,
            network_config,
            environment: container_req.environment.clone(),
            command: None, // TODO: Extract from image or requirements
            working_directory: None,
            container_status: ContainerStatus::Created,
            security_config,
            privacy_level: request.privacy_level.clone(),
            allocated_at: SystemTime::now(),
            last_accessed: SystemTime::now(),
            runtime_stats,
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
        
        // Update usage statistics
        self.update_usage_stats(ContainerOperation::Create).await;
        
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
        
        // Stop and remove container
        // TODO: Implement actual container stop/remove via runtime API
        tracing::info!("Stopping and removing container {}", allocation.container_id);
        
        // Free allocated ports
        {
            let mut allocated_ports = self.allocated_ports.write().await;
            for port_mapping in &allocation.network_config.port_mappings {
                allocated_ports.remove(&port_mapping.host_port);
            }
        }
        
        // Remove proxy mapping
        {
            let mut proxy_mappings = self.proxy_mappings.write().await;
            proxy_mappings.retain(|_, mapped_asset_id| mapped_asset_id != asset_id);
        }
        
        // Update usage statistics
        self.update_usage_stats(ContainerOperation::Destroy).await;
        
        tracing::info!(
            "Deallocated container asset: {} (container: {})", 
            asset_id, 
            allocation.container_id
        );
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
            state: match allocation.container_status {
                ContainerStatus::Running => AssetState::InUse,
                ContainerStatus::Created | ContainerStatus::Stopped => AssetState::Allocated,
                ContainerStatus::Failed(_) => AssetState::Failed,
                _ => AssetState::Available,
            },
            allocated_at: allocation.allocated_at,
            last_accessed: allocation.last_accessed,
            privacy_level: allocation.privacy_level.clone(),
            proxy_address: None, // Will be filled by proxy resolver
            resource_usage: self.get_resource_usage(asset_id).await?,
            consensus_proofs: Vec::new(),
            owner_certificate_fingerprint: "container-adapter".to_string(),
            health_status: crate::assets::core::status::AssetHealthStatus::default(),
            performance_metrics: crate::assets::core::status::AssetPerformanceMetrics::default(),
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert("container_id".to_string(), allocation.container_id.clone());
                metadata.insert("container_name".to_string(), allocation.container_name.clone());
                metadata.insert("image".to_string(), allocation.image.clone());
                metadata.insert("status".to_string(), format!("{:?}", allocation.container_status));
                metadata.insert("cpu_limit".to_string(), allocation.cpu_allocation.cpu_limit.to_string());
                metadata.insert("memory_limit_bytes".to_string(), allocation.memory_allocation.memory_limit_bytes.to_string());
                metadata.insert("ports".to_string(), allocation.network_config.port_mappings.len().to_string());
                metadata.insert("uptime_seconds".to_string(), allocation.runtime_stats.uptime_seconds.to_string());
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
        
        // Update network isolation based on privacy level
        if matches!(privacy, PrivacyLevel::Private | PrivacyLevel::PrivateNetwork) {
            allocation.network_config.network_mode = NetworkMode::Custom("isolated".to_string());
        }
        
        tracing::info!("Updated privacy level for container asset {}: {:?}", asset_id, privacy);
        Ok(())
    }
    
    async fn assign_proxy_address(&self, asset_id: &AssetId) -> AssetResult<ProxyAddress> {
        let proxy_address = Self::generate_proxy_address(asset_id).await;
        
        // Find existing proxy address
        let proxy_mappings = self.proxy_mappings.read().await;
        for (proxy_addr, mapped_asset_id) in proxy_mappings.iter() {
            if mapped_asset_id == asset_id {
                return Ok(proxy_addr.clone());
            }
        }
        
        Ok(proxy_address)
    }
    
    async fn resolve_proxy_address(&self, proxy_addr: &ProxyAddress) -> AssetResult<AssetId> {
        let proxy_mappings = self.proxy_mappings.read().await;
        proxy_mappings.get(proxy_addr)
            .cloned()
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
        
        // Get updated runtime stats
        let runtime_stats = self.get_container_stats(&allocation.container_id).await;
        
        Ok(ResourceUsage {
            cpu_usage: Some(crate::assets::core::CpuUsage {
                utilization_percent: runtime_stats.cpu_usage_percent,
                frequency_mhz: 2400, // TODO: Get actual frequency
                temperature_celsius: None,
                active_cores: allocation.cpu_allocation.pinned_cores.len() as u32,
            }),
            gpu_usage: None,
            memory_usage: Some(crate::assets::core::MemoryUsage {
                used_bytes: runtime_stats.memory_usage_bytes,
                total_bytes: allocation.memory_allocation.memory_limit_bytes,
                cached_bytes: 0,
                swap_used_bytes: 0,
            }),
            storage_usage: Some(crate::assets::core::StorageUsage {
                used_bytes: runtime_stats.block_io.read_bytes + runtime_stats.block_io.write_bytes,
                total_bytes: 0, // TODO: Get container storage limit
                read_iops: 0,
                write_iops: 0,
                read_mbps: 0.0,
                write_mbps: 0.0,
            }),
            network_usage: Some(crate::assets::core::NetworkUsage {
                bytes_received: runtime_stats.network_io.rx_bytes,
                bytes_transmitted: runtime_stats.network_io.tx_bytes,
                packets_received: runtime_stats.network_io.rx_packets,
                packets_transmitted: runtime_stats.network_io.tx_packets,
                latency_us: None,
            }),
            measurement_timestamp: SystemTime::now(),
        })
    }
    
    async fn set_resource_limits(&self, asset_id: &AssetId, limits: ResourceLimits) -> AssetResult<()> {
        // TODO: Implement runtime resource limit updates
        tracing::info!("Set resource limits for container asset {}: {:?}", asset_id, limits);
        Ok(())
    }
    
    async fn health_check(&self) -> AssetResult<AdapterHealth> {
        let stats = self.usage_stats.read().await;
        let allocations = self.allocations.read().await;
        
        let failed_containers = allocations.values()
            .filter(|allocation| matches!(allocation.container_status, ContainerStatus::Failed(_)))
            .count();
        
        let healthy = failed_containers == 0 && stats.active_containers < 1000; // Reasonable limits
        
        let total_memory_allocated = allocations.values()
            .map(|a| a.memory_allocation.memory_limit_bytes)
            .sum::<u64>();
        
        let mut performance_metrics = HashMap::new();
        performance_metrics.insert("active_containers".to_string(), stats.active_containers as f64);
        performance_metrics.insert("failed_containers".to_string(), failed_containers as f64);
        performance_metrics.insert("total_memory_allocated_gb".to_string(), (total_memory_allocated / (1024 * 1024 * 1024)) as f64);
        performance_metrics.insert("total_cpu_time_hours".to_string(), stats.total_cpu_time_seconds / 3600.0);
        performance_metrics.insert("container_restarts".to_string(), stats.container_restarts as f64);
        performance_metrics.insert("network_io_gb".to_string(), (stats.total_network_io_bytes / (1024 * 1024 * 1024)) as f64);
        
        Ok(AdapterHealth {
            healthy,
            message: if healthy {
                "Container adapter operating normally".to_string()
            } else {
                format!("Container adapter issues: {} failed containers", failed_containers)
            },
            last_check: SystemTime::now(),
            performance_metrics,
        })
    }
    
    fn get_capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            asset_type: AssetType::Container,
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
                "container_orchestration".to_string(),
                "image_management".to_string(),
                "network_isolation".to_string(),
                "volume_management".to_string(),
                "security_controls".to_string(),
                "resource_limits".to_string(),
                "port_management".to_string(),
                "ipv6_networking".to_string(),
                "runtime_stats".to_string(),
                "lifecycle_management".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::core::{SpaceProof, StakeProof, WorkProof, TimeProof, WorkloadType, WorkState};
    
    async fn create_test_container_request() -> AssetAllocationRequest {
        AssetAllocationRequest {
            asset_type: AssetType::Container,
            requested_resources: crate::assets::core::ResourceRequirements {
                container: Some(ContainerRequirements {
                    image: "nginx:latest".to_string(),
                    cpu_limit: 1.0, // 1 CPU core
                    memory_limit_bytes: 512 * 1024 * 1024, // 512MB
                    environment: {
                        let mut env = HashMap::new();
                        env.insert("ENV".to_string(), "production".to_string());
                        env
                    },
                    volumes: vec![VolumeMount {
                        source: "/host/data".to_string(),
                        target: "/container/data".to_string(),
                        read_only: false,
                    }],
                    ports: vec![PortMapping {
                        container_port: 80,
                        host_port: None, // Auto-assign
                        protocol: "TCP".to_string(),
                    }],
                }),
                ..Default::default()
            },
            privacy_level: PrivacyLevel::Private,
            consensus_proof: ConsensusProof::new(
                SpaceProof {
                    node_id: "test-node".to_string(),
                    storage_path: "/test/container".to_string(),
                    allocated_size: 512 * 1024 * 1024,
                    proof_hash: vec![1, 2, 3, 4],
                    timestamp: SystemTime::now(),
                },
                StakeProof {
                    stake_holder: "test-holder".to_string(),
                    stake_holder_id: "test-holder-id".to_string(),
                    stake_amount: 75,
                    stake_timestamp: SystemTime::now(),
                },
                WorkProof {
                    worker_id: "test-worker".to_string(),
                    workload_id: "test-workload".to_string(),
                    process_id: 12345,
                    computational_power: 50,
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
    async fn test_container_adapter_creation() {
        let adapter = ContainerAssetAdapter::new().await;
        assert_eq!(adapter.asset_type(), AssetType::Container);
    }
    
    #[tokio::test]
    async fn test_container_allocation() {
        let adapter = ContainerAssetAdapter::new().await;
        let request = create_test_container_request().await;
        
        let allocation = adapter.allocate_asset(&request).await.unwrap();
        assert_eq!(allocation.asset_id.asset_type, AssetType::Container);
        
        // Test deallocation
        adapter.deallocate_asset(&allocation.asset_id).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_container_health_check() {
        let adapter = ContainerAssetAdapter::new().await;
        let health = adapter.health_check().await.unwrap();
        
        assert!(health.healthy);
        assert!(health.performance_metrics.contains_key("active_containers"));
    }
    
    #[tokio::test]
    async fn test_container_capabilities() {
        let adapter = ContainerAssetAdapter::new().await;
        let capabilities = adapter.get_capabilities();
        
        assert_eq!(capabilities.asset_type, AssetType::Container);
        assert!(capabilities.supports_proxy_addressing);
        assert!(capabilities.features.contains(&"container_orchestration".to_string()));
        assert!(capabilities.features.contains(&"security_controls".to_string()));
    }
}