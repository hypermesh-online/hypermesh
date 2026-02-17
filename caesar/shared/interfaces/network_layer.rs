// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// Network Layer Interface - Team 1 Implementation Contract
// All teams must implement against this interface to ensure compatibility

use std::net::{IpAddr, SocketAddr};
use std::result::Result;

/// Unique identifier for peers in the network
pub type PeerId = [u8; 32];

/// Network address that can be IPv4 or IPv6 with NAT handling
#[derive(Debug, Clone)]
pub struct NetworkAddress {
    pub ip: IpAddr,
    pub port: u16,
    pub nat_type: NATType,
    pub reachability: ReachabilityType,
}

/// NAT traversal types supported by the network layer
#[derive(Debug, Clone)]
pub enum NATType {
    None,              // Direct connection
    FullCone,          // Basic NAT
    RestrictedCone,    // Port-restricted NAT
    PortRestricted,    // Address and port-restricted NAT
    Symmetric,         // Symmetric NAT (requires TURN)
}

/// Network reachability classification
#[derive(Debug, Clone)]
pub enum ReachabilityType {
    Public,            // Directly reachable
    NATTraversal,      // Reachable via NAT traversal
    RelayRequired,     // Requires TURN relay
    Unreachable,       // Cannot establish connection
}

/// Secure communication channel between peers
pub struct SecureChannel {
    pub peer_id: PeerId,
    pub local_addr: NetworkAddress,
    pub remote_addr: NetworkAddress,
    pub encryption_keys: ChannelKeys,
    pub performance_metrics: ChannelMetrics,
}

/// Encryption keys for secure channel
pub struct ChannelKeys {
    pub send_key: [u8; 32],
    pub receive_key: [u8; 32],
    pub nonce: [u8; 12],
}

/// Performance metrics for channel monitoring
pub struct ChannelMetrics {
    pub throughput_mbps: f64,
    pub latency_ms: u32,
    pub packet_loss_rate: f32,
    pub connection_stability: f32,
}

/// Raw 32-byte asset hash. Distinct from hypermesh_lib::AssetId (string identifier).
pub type RawAssetId = [u8; 32];

/// Errors that can occur in network operations
#[derive(Debug)]
pub enum NetworkError {
    ConnectionFailed,
    NATTraversalFailed,
    DNSResolutionFailed,
    FirewallBlocked,
    PerformanceThreshold,
    SecurityValidationFailed,
    IPv6Unavailable,
    IPv4Fallback,
}

/// Main network layer interface - CRITICAL FOR ALL TEAMS
pub trait NetworkLayer {
    /// Establish secure channel with peer (Team 1 → Teams 2,3)
    fn establish_secure_channel(&self, peer: PeerId) -> Result<SecureChannel, NetworkError>;
    
    /// Resolve asset to network address using NAT-like system (Team 1 → Team 2)
    fn resolve_asset_address(&self, asset_id: RawAssetId) -> Result<NetworkAddress, NetworkError>;
    
    /// Handle NAT traversal for local address (Team 1 core functionality)
    fn handle_nat_traversal(&self, local_addr: SocketAddr) -> Result<NetworkAddress, NetworkError>;
    
    /// Provide IPv4/IPv6 dual-stack connectivity (Team 1 critical requirement)
    fn get_connectivity_status(&self) -> ConnectivityStatus;
    
    /// DNS resolution with traditional fallback (Team 1 → All teams)
    fn resolve_hypermesh_address(&self, name: &str) -> Result<NetworkAddress, NetworkError>;
    
    /// Performance monitoring for STOQ optimization (Team 1 responsibility)
    fn get_transport_performance(&self) -> TransportPerformance;
    
    /// Security channel validation (Team 1 → Team 3)
    fn validate_channel_security(&self, channel: &SecureChannel) -> Result<SecurityLevel, NetworkError>;
}

/// Network connectivity status for dual-stack operation
#[derive(Debug)]
pub struct ConnectivityStatus {
    pub ipv4_available: bool,
    pub ipv6_available: bool,
    pub nat_type: NATType,
    pub firewall_compatible: bool,
    pub internet_reachability: f32, // Percentage of internet users reachable
}

/// Transport layer performance metrics
#[derive(Debug)]
pub struct TransportPerformance {
    pub current_throughput_gbps: f64,
    pub target_throughput_gbps: f64,
    pub connection_count: u32,
    pub average_latency_ms: u32,
    pub successful_nat_traversal_rate: f32,
}

/// Security level of network channel
#[derive(Debug)]
pub enum SecurityLevel {
    Insecure,          // No encryption
    Basic,             // Basic encryption
    QuantumResistant,  // FALCON-1024/Kyber protected
    FullValidation,    // Complete certificate chain validation
}

/// Implementation requirements for Team 1
pub trait NetworkInfrastructureTeam: NetworkLayer {
    /// IPv4/IPv6 dual-stack implementation (CRITICAL PATH)
    fn enable_dual_stack(&mut self) -> Result<ConnectivityStatus, NetworkError>;
    
    /// NAT traversal using STUN/TURN (CRITICAL PATH)
    fn configure_nat_traversal(&mut self, stun_servers: Vec<String>, turn_servers: Vec<String>) -> Result<(), NetworkError>;
    
    /// Traditional DNS fallback system (CRITICAL PATH)  
    fn configure_dns_fallback(&mut self, fallback_servers: Vec<String>) -> Result<(), NetworkError>;
    
    /// STOQ transport optimization (PERFORMANCE CRITICAL)
    fn optimize_stoq_transport(&mut self) -> Result<TransportPerformance, NetworkError>;
    
    /// Enterprise firewall compatibility (USER ADOPTION CRITICAL)
    fn test_firewall_compatibility(&self, firewall_rules: &[String]) -> Result<bool, NetworkError>;
}

/// Critical performance targets Team 1 must achieve
pub const TARGET_THROUGHPUT_GBPS: f64 = 10.0;
pub const TARGET_INTERNET_REACHABILITY: f32 = 0.75; // 75% of internet users
pub const TARGET_NAT_TRAVERSAL_SUCCESS: f32 = 0.90; // 90% success rate
pub const TARGET_LATENCY_MS: u32 = 100; // Maximum acceptable latency

/// Interface validation for cross-team integration
pub trait NetworkIntegrationValidator {
    /// Validate Team 2 can use network for consensus
    fn validate_consensus_network_requirements(&self) -> Result<bool, NetworkError>;
    
    /// Validate Team 3 can use network for security
    fn validate_security_network_requirements(&self) -> Result<bool, NetworkError>;
    
    /// Validate enterprise entity connectivity
    fn validate_enterprise_connectivity(&self, entity_type: &str) -> Result<bool, NetworkError>;
}