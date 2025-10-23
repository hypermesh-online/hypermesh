//! Networking types and data structures for HyperMesh P2P networking
//!
//! This module contains all the shared types used across the networking system,
//! including container networks, interfaces, policies, and metrics.

use nexus_shared::{NodeId, ResourceId, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BTreeSet};
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::time::{Duration, Instant};

/// Container network configuration and state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerNetwork {
    /// Container identifier
    pub container_id: ResourceId,
    
    /// Container IPv6 address (HyperMesh uses IPv6-only networking)
    pub ipv6_address: Ipv6Addr,
    
    /// Network namespace ID
    pub namespace_id: String,
    
    /// Network interfaces assigned to container
    pub interfaces: Vec<NetworkInterface>,
    
    /// P2P mesh peers this container can communicate with
    pub authorized_peers: BTreeSet<NodeId>,
    
    /// Network policies applied to this container
    pub policies: Vec<NetworkPolicy>,
    
    /// Creation timestamp
    pub created_at: Timestamp,
    
    /// Current network status
    pub status: NetworkStatus,
    
    /// Bandwidth allocation and limits
    pub bandwidth: BandwidthConfig,
    
    /// Security context for Byzantine protection
    pub security_context: NetworkSecurityContext,
}

/// Network interface configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    /// Interface name (e.g., eth0, mesh0)
    pub name: String,
    
    /// Interface type
    pub interface_type: InterfaceType,
    
    /// MAC address
    pub mac_address: String,
    
    /// MTU size
    pub mtu: u16,
    
    /// Interface flags
    pub flags: Vec<String>,
}

/// Network interface types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterfaceType {
    /// Standard Ethernet interface
    Ethernet,
    
    /// P2P mesh interface for inter-node communication
    MeshP2P,
    
    /// Loopback interface
    Loopback,
}

/// Network policy for traffic control and security
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    /// Policy name
    pub name: String,
    
    /// Policy type
    pub policy_type: PolicyType,
    
    /// Traffic rules
    pub rules: Vec<TrafficRule>,
    
    /// Policy priority (higher number = higher priority)
    pub priority: u32,
    
    /// Policy enabled/disabled
    pub enabled: bool,
}

/// Network policy types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyType {
    /// Ingress traffic policy
    Ingress,
    
    /// Egress traffic policy
    Egress,
    
    /// Byzantine fault protection policy
    Byzantine,
    
    /// Security isolation policy
    Security,
}

/// Traffic rule for network policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficRule {
    /// Rule name
    pub name: String,
    
    /// Source address or network
    pub source: Option<String>,
    
    /// Destination address or network
    pub destination: Option<String>,
    
    /// Protocol specification
    pub protocol: Protocol,
    
    /// Port range
    pub port_range: Option<PortRange>,
    
    /// Traffic action
    pub action: TrafficAction,
}

/// Network protocols
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Protocol {
    TCP,
    UDP,
    QUIC,
    ICMP,
    Any,
}

/// Port range specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortRange {
    pub start: u16,
    pub end: u16,
}

/// Traffic actions for network policies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrafficAction {
    Allow,
    Deny,
    RateLimit,
    Monitor,
}

/// Byzantine network policy for fault protection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByzantineNetworkPolicy {
    /// Maximum message rate per peer
    pub max_message_rate: u64,
    
    /// Reputation threshold for peer communication
    pub reputation_threshold: f64,
    
    /// Quarantine duration for suspicious peers
    pub quarantine_duration: Duration,
    
    /// Enable deep packet inspection
    pub enable_dpi: bool,
}

/// Network status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkStatus {
    /// Network is initializing
    Initializing,
    
    /// Network is active and healthy
    Active,
    
    /// Network is degraded but functional
    Degraded,
    
    /// Network is isolated due to Byzantine fault
    Isolated,
    
    /// Network is being destroyed
    Destroying,
    
    /// Network has been destroyed
    Destroyed,
}

/// Bandwidth configuration and limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthConfig {
    /// Maximum ingress bandwidth (bytes/sec)
    pub max_ingress_bps: u64,
    
    /// Maximum egress bandwidth (bytes/sec)  
    pub max_egress_bps: u64,
    
    /// Burst allowance (bytes)
    pub burst_bytes: u64,
    
    /// Traffic shaping enabled
    pub shaping_enabled: bool,
}

/// Network security context for Byzantine protection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSecurityContext {
    /// Security level
    pub security_level: SecurityLevel,
    
    /// Encryption requirements
    pub encryption_required: bool,
    
    /// Authentication requirements
    pub authentication_required: bool,
    
    /// Certificate validation level
    pub cert_validation_level: CertValidationLevel,
}

/// Security levels for network communication
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityLevel {
    /// Minimal security (testing only)
    Minimal,
    
    /// Standard security
    Standard,
    
    /// High security
    High,
    
    /// Maximum security
    Maximum,
}

/// Certificate validation levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CertValidationLevel {
    /// No certificate validation
    None,
    
    /// Basic certificate validation
    Basic,
    
    /// Full certificate chain validation
    Full,
    
    /// Certificate pinning required
    Pinning,
}

/// Network performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkMetrics {
    /// Total bytes sent
    pub bytes_sent: u64,
    
    /// Total bytes received
    pub bytes_received: u64,
    
    /// Total packets sent
    pub packets_sent: u64,
    
    /// Total packets received
    pub packets_received: u64,
    
    /// Packet loss rate
    pub packet_loss_rate: f64,
    
    /// Average round-trip time
    pub avg_rtt_ms: f64,
    
    /// Current bandwidth utilization
    pub bandwidth_utilization: f64,
    
    /// Active connections count
    pub active_connections: usize,
    
    /// Error count
    pub error_count: u64,
    
    /// eBPF specific metrics
    pub ebpf_metrics: EbpfMetrics,
    
    /// Last metrics update timestamp
    pub last_updated: Timestamp,
}

/// eBPF specific metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EbpfMetrics {
    /// Programs loaded
    pub programs_loaded: usize,
    
    /// Maps created
    pub maps_created: usize,
    
    /// Packets processed by eBPF
    pub packets_processed: u64,
    
    /// eBPF program execution time (microseconds)
    pub execution_time_us: u64,
}

/// Network events for monitoring and coordination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkEvent {
    /// Container network created
    ContainerNetworkCreated {
        container_id: ResourceId,
        ipv6_address: Ipv6Addr,
        timestamp: Timestamp,
    },
    
    /// Container network destroyed
    ContainerNetworkDestroyed {
        container_id: ResourceId,
        timestamp: Timestamp,
    },
    
    /// P2P connection established
    PeerConnected {
        peer_id: NodeId,
        peer_address: SocketAddr,
        timestamp: Timestamp,
    },
    
    /// P2P connection lost
    PeerDisconnected {
        peer_id: NodeId,
        reason: DisconnectionReason,
        timestamp: Timestamp,
    },
    
    /// Byzantine fault detected
    ByzantineFaultDetected {
        peer_id: NodeId,
        fault_type: ByzantineFaultType,
        details: String,
        timestamp: Timestamp,
    },
    
    /// Network policy applied
    PolicyApplied {
        container_id: ResourceId,
        policy_name: String,
        timestamp: Timestamp,
    },
    
    /// Network metrics updated
    MetricsUpdated {
        metrics: NetworkMetrics,
        timestamp: Timestamp,
    },
}

/// Disconnection reasons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DisconnectionReason {
    /// Normal connection close
    Normal,
    
    /// Network timeout
    Timeout,
    
    /// Network error
    NetworkError,
    
    /// Authentication failure
    AuthenticationFailure,
    
    /// Byzantine fault detected
    ByzantineFault,
    
    /// Peer quarantined
    Quarantined,
}

/// Byzantine fault types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ByzantineFaultType {
    /// Message replay attack
    MessageReplay,
    
    /// Invalid message signature
    InvalidSignature,
    
    /// Excessive message rate
    ExcessiveRate,
    
    /// Suspicious traffic pattern
    SuspiciousPattern,
    
    /// Consensus violation
    ConsensusViolation,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Enable P2P mesh networking
    pub enable_mesh: bool,
    
    /// IPv6 subnet for containers
    pub container_subnet: String,
    
    /// Default network policies
    pub default_policies: Vec<NetworkPolicy>,
    
    /// Byzantine protection settings
    pub byzantine_protection: ByzantineNetworkPolicy,
    
    /// eBPF configuration
    pub ebpf_config: EbpfConfig,
    
    /// Connection pool size
    pub connection_pool_size: usize,
    
    /// Metrics collection interval
    pub metrics_interval: Duration,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            enable_mesh: true,
            container_subnet: "fd00:hypermesh::/64".to_string(),
            default_policies: Vec::new(),
            byzantine_protection: ByzantineNetworkPolicy {
                max_message_rate: 1000,
                reputation_threshold: 0.7,
                quarantine_duration: Duration::from_secs(300),
                enable_dpi: true,
            },
            ebpf_config: EbpfConfig::default(),
            connection_pool_size: 100,
            metrics_interval: Duration::from_secs(10),
        }
    }
}

/// eBPF configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EbpfConfig {
    /// Enable eBPF traffic control
    pub enabled: bool,
    
    /// eBPF programs to load
    pub programs: Vec<EbpfProgram>,
    
    /// Map size limits
    pub map_size_limit: usize,
}

impl Default for EbpfConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            programs: vec![
                EbpfProgram {
                    name: "traffic_control".to_string(),
                    path: "/opt/hypermesh/ebpf/traffic_control.o".to_string(),
                    attach_point: "tc".to_string(),
                },
                EbpfProgram {
                    name: "security_filter".to_string(),
                    path: "/opt/hypermesh/ebpf/security_filter.o".to_string(),
                    attach_point: "xdp".to_string(),
                },
            ],
            map_size_limit: 1024 * 1024, // 1MB
        }
    }
}

/// eBPF program configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EbpfProgram {
    /// Program name
    pub name: String,
    
    /// Path to compiled eBPF object file
    pub path: String,
    
    /// Kernel attach point (tc, xdp, cgroup, etc.)
    pub attach_point: String,
}

impl Default for ContainerNetwork {
    fn default() -> Self {
        Self {
            container_id: ResourceId::default(),
            ipv6_address: Ipv6Addr::UNSPECIFIED,
            namespace_id: String::new(),
            interfaces: Vec::new(),
            authorized_peers: BTreeSet::new(),
            policies: Vec::new(),
            created_at: std::time::SystemTime::now().into(),
            status: NetworkStatus::Initializing,
            bandwidth: BandwidthConfig {
                max_ingress_bps: 1_000_000_000, // 1 Gbps
                max_egress_bps: 1_000_000_000,  // 1 Gbps
                burst_bytes: 64 * 1024,         // 64 KB
                shaping_enabled: true,
            },
            security_context: NetworkSecurityContext {
                security_level: SecurityLevel::Standard,
                encryption_required: true,
                authentication_required: true,
                cert_validation_level: CertValidationLevel::Full,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn test_container_network_default() {
        let network = ContainerNetwork::default();
        
        assert_eq!(network.status, NetworkStatus::Initializing);
        assert_eq!(network.security_context.security_level, SecurityLevel::Standard);
        assert!(network.security_context.encryption_required);
        assert!(network.security_context.authentication_required);
    }

    #[test]
    fn test_network_config_default() {
        let config = NetworkConfig::default();
        
        assert!(config.enable_mesh);
        assert_eq!(config.container_subnet, "fd00:hypermesh::/64");
        assert_eq!(config.byzantine_protection.max_message_rate, 1000);
        assert!(config.ebpf_config.enabled);
    }

    #[test]
    fn test_interface_types() {
        assert_eq!(InterfaceType::Ethernet, InterfaceType::Ethernet);
        assert_ne!(InterfaceType::Ethernet, InterfaceType::MeshP2P);
    }

    #[test]
    fn test_traffic_rule_creation() {
        let rule = TrafficRule {
            name: "allow_http".to_string(),
            source: Some("0.0.0.0/0".to_string()),
            destination: None,
            protocol: Protocol::TCP,
            port_range: Some(PortRange { start: 80, end: 80 }),
            action: TrafficAction::Allow,
        };
        
        assert_eq!(rule.name, "allow_http");
        assert_eq!(rule.protocol, Protocol::TCP);
        assert_eq!(rule.action, TrafficAction::Allow);
    }

    #[test]
    fn test_network_metrics_default() {
        let metrics = NetworkMetrics::default();
        
        assert_eq!(metrics.bytes_sent, 0);
        assert_eq!(metrics.bytes_received, 0);
        assert_eq!(metrics.active_connections, 0);
        assert_eq!(metrics.ebpf_metrics.programs_loaded, 0);
    }
}