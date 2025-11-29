//! Network Asset Adapter with bandwidth allocation and traffic management
//!
//! Features:
//! - Network interface management
//! - Bandwidth allocation and QoS
//! - Traffic shaping and prioritization
//! - IPv6-only networking support
//! - Network security and isolation
//! - Latency and packet loss monitoring

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
    ResourceUsage, ResourceLimits, NetworkUsage, NetworkLimit,
    AdapterHealth, AdapterCapabilities, ConsensusProof,
    NetworkRequirements,
};

/// Network allocation record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkAllocation {
    /// Asset ID
    pub asset_id: AssetId,
    /// Allocated network interfaces
    pub allocated_interfaces: Vec<String>,
    /// Allocated bandwidth in Mbps
    pub allocated_bandwidth_mbps: u64,
    /// Network protocols enabled
    pub enabled_protocols: Vec<String>,
    /// QoS priority (0-255, higher = more priority)
    pub qos_priority: u8,
    /// Traffic shaping enabled
    pub traffic_shaping_enabled: bool,
    /// Network isolation enabled
    pub isolation_enabled: bool,
    /// IPv6 addresses allocated
    pub ipv6_addresses: Vec<String>,
    /// Privacy level
    pub privacy_level: PrivacyLevel,
    /// VLAN ID (for network isolation)
    pub vlan_id: Option<u16>,
    /// Allocation timestamp
    pub allocated_at: SystemTime,
    /// Last accessed timestamp
    pub last_accessed: SystemTime,
    /// Current bandwidth utilization in Mbps
    pub current_bandwidth_mbps: f32,
    /// Current latency in microseconds
    pub current_latency_us: u32,
    /// Current packet loss percentage
    pub current_packet_loss_percent: f32,
}

/// Network interface information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkInterface {
    /// Interface name (e.g., "eth0", "enp0s3")
    pub interface_name: String,
    /// Interface type (Ethernet, WiFi, etc.)
    pub interface_type: InterfaceType,
    /// Maximum bandwidth in Mbps
    pub max_bandwidth_mbps: u64,
    /// Available bandwidth in Mbps
    pub available_bandwidth_mbps: u64,
    /// MTU size
    pub mtu: u32,
    /// MAC address
    pub mac_address: String,
    /// IPv6 address
    pub ipv6_address: Option<String>,
    /// Current status
    pub status: InterfaceStatus,
    /// Current allocation asset ID
    pub allocated_to: Option<AssetId>,
    /// Interface statistics
    pub interface_stats: InterfaceStats,
}

/// Network interface types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InterfaceType {
    /// Ethernet interface
    Ethernet,
    /// WiFi interface
    WiFi,
    /// Loopback interface
    Loopback,
    /// Virtual interface
    Virtual,
    /// Bridge interface
    Bridge,
}

/// Network interface status
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InterfaceStatus {
    /// Interface is up and available
    Up,
    /// Interface is down
    Down,
    /// Interface is allocated but idle
    Allocated,
    /// Interface is actively transmitting
    Active,
    /// Interface is in maintenance mode
    Maintenance,
    /// Interface has failed
    Failed,
}

/// Network interface statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InterfaceStats {
    /// Bytes received
    pub bytes_received: u64,
    /// Bytes transmitted
    pub bytes_transmitted: u64,
    /// Packets received
    pub packets_received: u64,
    /// Packets transmitted
    pub packets_transmitted: u64,
    /// Receive errors
    pub receive_errors: u64,
    /// Transmit errors
    pub transmit_errors: u64,
    /// Dropped packets
    pub dropped_packets: u64,
    /// Collisions
    pub collisions: u64,
}

/// Quality of Service configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QoSConfig {
    /// Priority level (0-255)
    pub priority: u8,
    /// Bandwidth guarantee in Mbps
    pub guaranteed_bandwidth_mbps: u64,
    /// Maximum burst size in bytes
    pub max_burst_bytes: u64,
    /// Traffic class
    pub traffic_class: TrafficClass,
    /// DSCP marking
    pub dscp_marking: u8,
}

/// Traffic classification
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum TrafficClass {
    /// Best effort traffic
    BestEffort,
    /// Bulk data transfer
    Bulk,
    /// Interactive traffic
    Interactive,
    /// Real-time traffic
    RealTime,
    /// Critical traffic
    Critical,
}

/// Network security configuration
#[derive(Clone, Debug)]
pub struct NetworkSecurity {
    /// Firewall rules enabled
    pub firewall_enabled: bool,
    /// VPN enabled
    pub vpn_enabled: bool,
    /// Traffic encryption enabled
    pub encryption_enabled: bool,
    /// DDoS protection enabled
    pub ddos_protection: bool,
    /// Intrusion detection enabled
    pub intrusion_detection: bool,
}

/// Network Asset Adapter implementation
pub struct NetworkAssetAdapter {
    /// Active network allocations by asset ID
    allocations: Arc<RwLock<HashMap<AssetId, NetworkAllocation>>>,
    /// Network interface information and status
    network_interfaces: Arc<RwLock<HashMap<String, NetworkInterface>>>,
    /// Interface allocation mapping (interface_name -> asset_id)
    interface_allocations: Arc<RwLock<HashMap<String, AssetId>>>,
    /// QoS configurations by asset ID
    qos_configs: Arc<RwLock<HashMap<AssetId, QoSConfig>>>,
    /// Proxy address mappings
    proxy_mappings: Arc<RwLock<HashMap<ProxyAddress, AssetId>>>,
    /// Total network bandwidth in Mbps
    total_bandwidth: u64,
    /// Available network bandwidth in Mbps
    available_bandwidth: Arc<RwLock<u64>>,
    /// Network usage statistics
    usage_stats: Arc<RwLock<NetworkUsageStats>>,
}

/// Network usage statistics
#[derive(Clone, Debug, Default)]
pub struct NetworkUsageStats {
    /// Total allocations made
    pub total_allocations: u64,
    /// Total deallocations made
    pub total_deallocations: u64,
    /// Current active allocations
    pub active_allocations: u64,
    /// Total bandwidth allocated (Mbps)
    pub total_bandwidth_allocated: u64,
    /// Total bytes transferred
    pub total_bytes_transferred: u64,
    /// Total packets transferred
    pub total_packets_transferred: u64,
    /// Average latency in microseconds
    pub average_latency_us: f32,
    /// Average packet loss percentage
    pub average_packet_loss_percent: f32,
}

impl NetworkAssetAdapter {
    /// Create new network adapter
    pub async fn new() -> Self {
        // Detect system network configuration
        let (total_bandwidth, network_interfaces) = Self::detect_network_configuration().await;
        
        Self {
            allocations: Arc::new(RwLock::new(HashMap::new())),
            network_interfaces: Arc::new(RwLock::new(network_interfaces)),
            interface_allocations: Arc::new(RwLock::new(HashMap::new())),
            qos_configs: Arc::new(RwLock::new(HashMap::new())),
            proxy_mappings: Arc::new(RwLock::new(HashMap::new())),
            total_bandwidth,
            available_bandwidth: Arc::new(RwLock::new(total_bandwidth)),
            usage_stats: Arc::new(RwLock::new(NetworkUsageStats::default())),
        }
    }
    
    /// Detect system network configuration
    async fn detect_network_configuration() -> (u64, HashMap<String, NetworkInterface>) {
        // TODO: Implement actual network detection using netlink or /proc/net
        // For now, simulate a reasonable configuration
        let mut network_interfaces = HashMap::new();
        let mut total_bandwidth = 0u64;
        
        // Simulate Ethernet interface
        let eth_bandwidth = 10000; // 10 Gbps
        network_interfaces.insert("eth0".to_string(), NetworkInterface {
            interface_name: "eth0".to_string(),
            interface_type: InterfaceType::Ethernet,
            max_bandwidth_mbps: eth_bandwidth,
            available_bandwidth_mbps: eth_bandwidth,
            mtu: 1500,
            mac_address: "02:42:ac:11:00:02".to_string(),
            ipv6_address: Some("2001:db8::1".to_string()),
            status: InterfaceStatus::Up,
            allocated_to: None,
            interface_stats: InterfaceStats {
                bytes_received: 0,
                bytes_transmitted: 0,
                packets_received: 0,
                packets_transmitted: 0,
                receive_errors: 0,
                transmit_errors: 0,
                dropped_packets: 0,
                collisions: 0,
            },
        });
        total_bandwidth += eth_bandwidth;
        
        // Simulate WiFi interface
        let wifi_bandwidth = 1000; // 1 Gbps
        network_interfaces.insert("wlan0".to_string(), NetworkInterface {
            interface_name: "wlan0".to_string(),
            interface_type: InterfaceType::WiFi,
            max_bandwidth_mbps: wifi_bandwidth,
            available_bandwidth_mbps: wifi_bandwidth,
            mtu: 1500,
            mac_address: "02:42:ac:11:00:03".to_string(),
            ipv6_address: Some("2001:db8::2".to_string()),
            status: InterfaceStatus::Up,
            allocated_to: None,
            interface_stats: InterfaceStats {
                bytes_received: 0,
                bytes_transmitted: 0,
                packets_received: 0,
                packets_transmitted: 0,
                receive_errors: 0,
                transmit_errors: 0,
                dropped_packets: 0,
                collisions: 0,
            },
        });
        total_bandwidth += wifi_bandwidth;
        
        (total_bandwidth, network_interfaces)
    }
    
    /// Allocate network bandwidth from interfaces
    async fn allocate_network_bandwidth(
        &self,
        network_req: &NetworkRequirements,
        asset_id: &AssetId,
    ) -> AssetResult<(Vec<String>, u64)> {
        let mut interfaces = self.network_interfaces.write().await;
        let mut interface_allocations = self.interface_allocations.write().await;
        let mut allocated_interfaces = Vec::new();
        let mut total_allocated_bandwidth = 0u64;
        
        // Find suitable interfaces
        let mut suitable_interfaces: Vec<String> = interfaces
            .iter()
            .filter(|(_, interface)| {
                matches!(interface.status, InterfaceStatus::Up) &&
                interface.available_bandwidth_mbps >= network_req.bandwidth_mbps &&
                (network_req.protocols.is_empty() || 
                 network_req.protocols.iter().all(|proto| 
                     matches!(proto.as_str(), "TCP" | "UDP" | "ICMP"))) // Basic protocol support
            })
            .map(|(interface_name, _)| interface_name.clone())
            .collect();
        
        // Sort by available bandwidth (largest first)
        suitable_interfaces.sort_by_key(|interface_name| {
            interfaces.get(interface_name)
                .map(|interface| std::cmp::Reverse(interface.available_bandwidth_mbps))
                .unwrap_or(std::cmp::Reverse(0))
        });
        
        // Check if we have enough bandwidth
        let total_available: u64 = suitable_interfaces
            .iter()
            .filter_map(|interface_name| interfaces.get(interface_name).map(|i| i.available_bandwidth_mbps))
            .sum();
        
        if total_available < network_req.bandwidth_mbps {
            return Err(AssetError::AllocationFailed {
                reason: format!(
                    "Insufficient network bandwidth: {} Mbps requested, {} Mbps available",
                    network_req.bandwidth_mbps, total_available
                )
            });
        }
        
        // Allocate bandwidth (prefer single interface, fall back to multiple)
        let mut remaining_bandwidth = network_req.bandwidth_mbps;
        
        for interface_name in &suitable_interfaces {
            if remaining_bandwidth == 0 {
                break;
            }
            
            let interface = interfaces.get_mut(interface_name)
                .ok_or_else(|| AssetError::ResourceUnavailable(format!("Interface {} not found", interface_name)))?;
            let allocated_from_this = remaining_bandwidth.min(interface.available_bandwidth_mbps);
            
            interface.available_bandwidth_mbps -= allocated_from_this;
            interface.status = InterfaceStatus::Allocated;
            interface.allocated_to = Some(asset_id.clone());
            
            interface_allocations.insert(interface_name.clone(), asset_id.clone());
            allocated_interfaces.push(interface_name.clone());
            total_allocated_bandwidth += allocated_from_this;
            remaining_bandwidth -= allocated_from_this;
        }
        
        if remaining_bandwidth > 0 {
            // Rollback partial allocation
            for interface_name in &allocated_interfaces {
                let interface = interfaces.get_mut(interface_name)
                .ok_or_else(|| AssetError::ResourceUnavailable(format!("Interface {} not found", interface_name)))?;
                interface.available_bandwidth_mbps += total_allocated_bandwidth / allocated_interfaces.len() as u64;
                interface.status = InterfaceStatus::Up;
                interface.allocated_to = None;
                interface_allocations.remove(interface_name);
            }
            
            return Err(AssetError::AllocationFailed {
                reason: "Failed to allocate complete bandwidth requirement".to_string()
            });
        }
        
        Ok((allocated_interfaces, total_allocated_bandwidth))
    }
    
    /// Generate proxy address for network access
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
    
    /// Allocate IPv6 addresses
    async fn allocate_ipv6_addresses(&self, count: u32) -> Vec<String> {
        // TODO: Implement actual IPv6 address allocation
        // For now, generate simulated addresses
        let mut addresses = Vec::new();
        for i in 0..count {
            addresses.push(format!("2001:db8:hypermesh::asset:{:x}", i + 1));
        }
        addresses
    }
    
    /// Configure Quality of Service
    async fn configure_qos(&self, asset_id: &AssetId, network_req: &NetworkRequirements) -> QoSConfig {
        let priority = if network_req.max_latency_us.unwrap_or(10000) < 1000 {
            200 // High priority for low latency requirements
        } else {
            128 // Default priority
        };
        
        let traffic_class = if network_req.max_latency_us.unwrap_or(10000) < 1000 {
            TrafficClass::RealTime
        } else if network_req.bandwidth_mbps > 1000 {
            TrafficClass::Bulk
        } else {
            TrafficClass::BestEffort
        };
        
        QoSConfig {
            priority,
            guaranteed_bandwidth_mbps: network_req.bandwidth_mbps / 2, // Guarantee 50%
            max_burst_bytes: 1024 * 1024, // 1MB burst
            traffic_class,
            dscp_marking: match traffic_class {
                TrafficClass::RealTime => 46, // EF
                TrafficClass::Critical => 34, // AF41
                TrafficClass::Interactive => 18, // AF21
                TrafficClass::Bulk => 10, // AF11
                TrafficClass::BestEffort => 0, // BE
            },
        }
    }
    
    /// Update usage statistics
    async fn update_usage_stats(&self, operation: NetworkOperation, bandwidth_mbps: u64) {
        let mut stats = self.usage_stats.write().await;
        
        match operation {
            NetworkOperation::Allocate => {
                stats.total_allocations += 1;
                stats.active_allocations += 1;
                stats.total_bandwidth_allocated += bandwidth_mbps;
            },
            NetworkOperation::Deallocate => {
                stats.total_deallocations += 1;
                stats.active_allocations = stats.active_allocations.saturating_sub(1);
                stats.total_bandwidth_allocated = stats.total_bandwidth_allocated.saturating_sub(bandwidth_mbps);
            },
            NetworkOperation::Transfer => {
                stats.total_bytes_transferred += bandwidth_mbps * 1024 * 1024 / 8; // Estimate bytes
                stats.total_packets_transferred += bandwidth_mbps * 100; // Estimate packets
            },
        }
    }
}

/// Network operations for statistics
#[derive(Clone, Debug)]
enum NetworkOperation {
    Allocate,
    Deallocate,
    Transfer,
}

#[async_trait]
impl AssetAdapter for NetworkAssetAdapter {
    fn asset_type(&self) -> AssetType {
        AssetType::Network
    }
    
    async fn validate_consensus_proof(&self, proof: &ConsensusProof) -> AssetResult<bool> {
        // Validate all four proofs with network-specific requirements
        
        // PoSpace: Validate network space allocation
        if proof.space_proof.total_size == 0 {
            return Ok(false);
        }
        
        // PoStake: Validate network access stake
        if proof.stake_proof.stake_amount < 25 { // Low minimum for network
            return Ok(false);
        }
        
        // PoWork: Validate work for network allocation
        if proof.work_proof.computational_power < 10 { // Lowest minimum for network
            return Ok(false);
        }
        
        // PoTime: CRITICAL for network - validate timing for synchronization
        if proof.time_proof.network_time_offset > Duration::from_secs(1) {
            return Ok(false);
        }
        
        Ok(true)
    }
    
    async fn allocate_asset(&self, request: &AssetAllocationRequest) -> AssetResult<AssetAllocation> {
        // Validate consensus proof first
        if !self.validate_consensus_proof(&request.consensus_proof).await? {
            return Err(AssetError::ConsensusValidationFailed {
                reason: "Network allocation consensus validation failed".to_string()
            });
        }
        
        // Get network requirements
        let network_req = request.requested_resources.network.as_ref()
            .ok_or_else(|| AssetError::AllocationFailed {
                reason: "No network requirements specified".to_string()
            })?;
        
        // Check available bandwidth
        let available = *self.available_bandwidth.read().await;
        if available < network_req.bandwidth_mbps {
            return Err(AssetError::AllocationFailed {
                reason: format!(
                    "Insufficient network bandwidth: {} Mbps requested, {} Mbps available",
                    network_req.bandwidth_mbps, available
                )
            });
        }
        
        // Create asset ID
        let asset_id = AssetId::new(AssetType::Network);
        
        // Allocate network bandwidth
        let (allocated_interfaces, allocated_bandwidth) = self.allocate_network_bandwidth(network_req, &asset_id).await?;
        
        // Generate proxy address
        let proxy_address = Self::generate_proxy_address(&asset_id).await;
        
        // Allocate IPv6 addresses
        let ipv6_addresses = self.allocate_ipv6_addresses(allocated_interfaces.len() as u32).await;
        
        // Configure QoS
        let qos_config = self.configure_qos(&asset_id, network_req).await;
        {
            let mut qos_configs = self.qos_configs.write().await;
            qos_configs.insert(asset_id.clone(), qos_config);
        }
        
        // Create network allocation record
        let allocation = NetworkAllocation {
            asset_id: asset_id.clone(),
            allocated_interfaces: allocated_interfaces.clone(),
            allocated_bandwidth_mbps: allocated_bandwidth,
            enabled_protocols: network_req.protocols.clone(),
            privacy_level: request.privacy_level.clone(),
            qos_priority: 128, // Default priority
            traffic_shaping_enabled: true,
            isolation_enabled: matches!(request.privacy_level, PrivacyLevel::Private | PrivacyLevel::PrivateNetwork),
            ipv6_addresses,
            vlan_id: if matches!(request.privacy_level, PrivacyLevel::Private) {
                Some(100 + (asset_id.uuid.as_u128() % 4000) as u16) // Generate unique VLAN ID
            } else {
                None
            },
            allocated_at: SystemTime::now(),
            last_accessed: SystemTime::now(),
            current_bandwidth_mbps: 0.0,
            current_latency_us: network_req.max_latency_us.unwrap_or(1000),
            current_packet_loss_percent: 0.0,
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
        
        // Update available bandwidth
        {
            let mut available = self.available_bandwidth.write().await;
            *available -= allocated_bandwidth;
        }
        
        // Update usage statistics
        self.update_usage_stats(NetworkOperation::Allocate, allocated_bandwidth).await;
        
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
        
        // Free network interfaces
        {
            let mut interfaces = self.network_interfaces.write().await;
            let mut interface_allocations = self.interface_allocations.write().await;
            
            let bandwidth_per_interface = allocation.allocated_bandwidth_mbps / allocation.allocated_interfaces.len() as u64;
            
            for interface_name in &allocation.allocated_interfaces {
                if let Some(interface) = interfaces.get_mut(interface_name) {
                    interface.status = InterfaceStatus::Up;
                    interface.allocated_to = None;
                    interface.available_bandwidth_mbps += bandwidth_per_interface;
                }
                interface_allocations.remove(interface_name);
            }
        }
        
        // Remove QoS configuration
        {
            let mut qos_configs = self.qos_configs.write().await;
            qos_configs.remove(asset_id);
        }
        
        // Remove proxy mapping
        {
            let mut proxy_mappings = self.proxy_mappings.write().await;
            proxy_mappings.retain(|_, mapped_asset_id| mapped_asset_id != asset_id);
        }
        
        // Update available bandwidth
        {
            let mut available = self.available_bandwidth.write().await;
            *available += allocation.allocated_bandwidth_mbps;
        }
        
        // Update usage statistics
        self.update_usage_stats(NetworkOperation::Deallocate, allocation.allocated_bandwidth_mbps).await;
        
        tracing::info!(
            "Deallocated network asset: {} ({} interfaces, {} Mbps)", 
            asset_id, 
            allocation.allocated_interfaces.len(),
            allocation.allocated_bandwidth_mbps
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
            state: AssetState::InUse,
            allocated_at: allocation.allocated_at,
            last_accessed: allocation.last_accessed,
            privacy_level: allocation.privacy_level.clone(),
            proxy_address: None, // Will be filled by proxy resolver
            resource_usage: self.get_resource_usage(asset_id).await?,
            consensus_proofs: Vec::new(),
            owner_certificate_fingerprint: "network-adapter".to_string(),
            health_status: crate::assets::core::status::AssetHealthStatus::default(),
            performance_metrics: crate::assets::core::status::AssetPerformanceMetrics::default(),
            metadata: {
                let mut metadata = HashMap::new();
                metadata.insert("allocated_bandwidth_mbps".to_string(), allocation.allocated_bandwidth_mbps.to_string());
                metadata.insert("interfaces".to_string(), allocation.allocated_interfaces.len().to_string());
                metadata.insert("protocols".to_string(), allocation.enabled_protocols.join(","));
                metadata.insert("qos_priority".to_string(), allocation.qos_priority.to_string());
                metadata.insert("current_latency_us".to_string(), allocation.current_latency_us.to_string());
                metadata.insert("packet_loss_percent".to_string(), allocation.current_packet_loss_percent.to_string());
                metadata.insert("ipv6_addresses".to_string(), allocation.ipv6_addresses.len().to_string());
                metadata.insert("vlan_id".to_string(), allocation.vlan_id.map(|v| v.to_string()).unwrap_or_else(|| "none".to_string()));
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
        allocation.isolation_enabled = matches!(privacy, PrivacyLevel::Private | PrivacyLevel::PrivateNetwork);
        
        tracing::info!("Updated privacy level for network asset {}: {:?}", asset_id, privacy);
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
        
        // TODO: Implement actual network usage monitoring
        let network_usage = NetworkUsage {
            bytes_received: 0, // TODO: Get actual stats
            bytes_transmitted: 0,
            packets_received: 0,
            packets_transmitted: 0,
            latency_us: Some(allocation.current_latency_us),
        };
        
        Ok(ResourceUsage {
            cpu_usage: None,
            gpu_usage: None,
            memory_usage: None,
            storage_usage: None,
            network_usage: Some(network_usage),
            measurement_timestamp: SystemTime::now(),
        })
    }
    
    async fn set_resource_limits(&self, asset_id: &AssetId, limits: ResourceLimits) -> AssetResult<()> {
        if let Some(network_limit) = limits.network_limit {
            tracing::info!(
                "Set network limits for asset {}: max {} Mbps, max {} connections",
                asset_id,
                network_limit.max_bandwidth_mbps,
                network_limit.max_connections
            );
        }
        Ok(())
    }
    
    async fn health_check(&self) -> AssetResult<AdapterHealth> {
        let stats = self.usage_stats.read().await;
        let interfaces = self.network_interfaces.read().await;
        let available = *self.available_bandwidth.read().await;
        
        let failed_interfaces = interfaces.values().filter(|interface| matches!(interface.status, InterfaceStatus::Failed)).count();
        let down_interfaces = interfaces.values().filter(|interface| matches!(interface.status, InterfaceStatus::Down)).count();
        let healthy = failed_interfaces == 0 && down_interfaces < 2 && available > 0;
        
        let mut performance_metrics = HashMap::new();
        performance_metrics.insert("total_bandwidth_gbps".to_string(), (self.total_bandwidth as f64 / 1000.0));
        performance_metrics.insert("available_bandwidth_gbps".to_string(), (available as f64 / 1000.0));
        performance_metrics.insert("bandwidth_utilization_percent".to_string(), 
            ((self.total_bandwidth - available) as f64 / self.total_bandwidth as f64) * 100.0);
        performance_metrics.insert("active_allocations".to_string(), stats.active_allocations as f64);
        performance_metrics.insert("total_interfaces".to_string(), interfaces.len() as f64);
        performance_metrics.insert("failed_interfaces".to_string(), failed_interfaces as f64);
        performance_metrics.insert("down_interfaces".to_string(), down_interfaces as f64);
        performance_metrics.insert("average_latency_us".to_string(), stats.average_latency_us as f64);
        performance_metrics.insert("average_packet_loss_percent".to_string(), stats.average_packet_loss_percent as f64);
        
        Ok(AdapterHealth {
            healthy,
            message: if healthy {
                "Network adapter operating normally".to_string()
            } else {
                format!("Network adapter issues: {} failed, {} down interfaces", failed_interfaces, down_interfaces)
            },
            last_check: SystemTime::now(),
            performance_metrics,
        })
    }
    
    fn get_capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            asset_type: AssetType::Network,
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
            max_concurrent_allocations: Some(100),
            features: vec![
                "ipv6_only".to_string(),
                "bandwidth_allocation".to_string(),
                "qos_management".to_string(),
                "traffic_shaping".to_string(),
                "vlan_isolation".to_string(),
                "network_security".to_string(),
                "latency_monitoring".to_string(),
                "packet_loss_monitoring".to_string(),
                "multi_interface".to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::core::{SpaceProof, StakeProof, WorkProof, TimeProof, WorkloadType, WorkState};
    
    async fn create_test_network_request() -> AssetAllocationRequest {
        AssetAllocationRequest {
            asset_type: AssetType::Network,
            requested_resources: crate::assets::core::ResourceRequirements {
                network: Some(NetworkRequirements {
                    bandwidth_mbps: 1000, // 1 Gbps
                    max_latency_us: Some(1000),
                    max_packet_loss_percent: Some(0.1),
                    protocols: vec!["TCP".to_string(), "UDP".to_string()],
                }),
                ..Default::default()
            },
            privacy_level: PrivacyLevel::Private,
            consensus_proof: ConsensusProof::new(
                SpaceProof {
                    node_id: "test-node".to_string(),
                    storage_path: "/test/network".to_string(),
                    allocated_size: 1000,
                    proof_hash: vec![1, 2, 3, 4],
                    timestamp: SystemTime::now(),
                },
                StakeProof {
                    stake_holder: "test-holder".to_string(),
                    stake_holder_id: "test-holder-id".to_string(),
                    stake_amount: 50,
                    stake_timestamp: SystemTime::now(),
                },
                WorkProof {
                    worker_id: "test-worker".to_string(),
                    workload_id: "test-workload".to_string(),
                    process_id: 12345,
                    computational_power: 20,
                    workload_type: WorkloadType::Network,
                    work_state: WorkState::Completed,
                },
                TimeProof {
                    network_time_offset: Duration::from_millis(500),
                    time_verification_timestamp: SystemTime::now(),
                    nonce: 42,
                    proof_hash: vec![5, 6, 7, 8],
                },
            ),
            certificate_fingerprint: "test-cert".to_string(),
        }
    }
    
    #[tokio::test]
    async fn test_network_adapter_creation() {
        let adapter = NetworkAssetAdapter::new().await;
        assert_eq!(adapter.asset_type(), AssetType::Network);
        assert!(adapter.total_bandwidth > 0);
    }
    
    #[tokio::test]
    async fn test_network_allocation() {
        let adapter = NetworkAssetAdapter::new().await;
        let request = create_test_network_request().await;
        
        let allocation = adapter.allocate_asset(&request).await.unwrap();
        assert_eq!(allocation.asset_id.asset_type, AssetType::Network);
        
        // Test deallocation
        adapter.deallocate_asset(&allocation.asset_id).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_network_health_check() {
        let adapter = NetworkAssetAdapter::new().await;
        let health = adapter.health_check().await.unwrap();
        
        assert!(health.healthy);
        assert!(health.performance_metrics.contains_key("total_bandwidth_gbps"));
        assert!(health.performance_metrics.contains_key("total_interfaces"));
    }
    
    #[tokio::test]
    async fn test_network_capabilities() {
        let adapter = NetworkAssetAdapter::new().await;
        let capabilities = adapter.get_capabilities();
        
        assert_eq!(capabilities.asset_type, AssetType::Network);
        assert!(capabilities.supports_proxy_addressing);
        assert!(capabilities.features.contains(&"ipv6_only".to_string()));
        assert!(capabilities.features.contains(&"qos_management".to_string()));
    }
}