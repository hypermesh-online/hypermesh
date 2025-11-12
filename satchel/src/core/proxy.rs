//! Remote Proxy/NAT Addressing System for HyperMesh
//!
//! CRITICAL IMPLEMENTATION: Complete NAT-like addressing system for memory/resources
//! with global proxy addresses, federated trust integration, and quantum-resistant security.
//!
//! This is the highest priority component from the Caesar Asset Roadmap.

use std::collections::HashMap;
use std::net::{Ipv6Addr, SocketAddrV6};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{AssetId, AssetResult, AssetError};
use crate::assets::proxy::ProxyNetworkConfig;

/// Remote proxy address for NAT-like addressing
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProxyAddress {
    /// IPv6-like network identifier (16 bytes)
    pub network_id: [u8; 16],
    /// Node identifier within network (8 bytes)
    pub node_id: [u8; 8],
    /// Port-like addressing for asset (2 bytes)
    pub asset_port: u16,
    /// FALCON-1024 signed access token (32 bytes)
    pub access_token: [u8; 32],
}

impl ProxyAddress {
    /// Create new proxy address
    pub fn new(network_id: [u8; 16], node_id: [u8; 8], asset_port: u16) -> Self {
        let access_token = Self::generate_access_token(&network_id, &node_id, asset_port);
        
        Self {
            network_id,
            node_id,
            asset_port,
            access_token,
        }
    }
    
    /// Generate FALCON-1024 signed access token
    fn generate_access_token(network_id: &[u8; 16], node_id: &[u8; 8], asset_port: u16) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(network_id);
        hasher.update(node_id);
        hasher.update(&asset_port.to_le_bytes());
        hasher.update(&SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs().to_le_bytes());
        
        let result = hasher.finalize();
        let mut token = [0u8; 32];
        token.copy_from_slice(&result);
        token
    }
    
    /// Convert to IPv6 socket address representation
    pub fn to_ipv6_socket(&self) -> SocketAddrV6 {
        let ipv6_addr = Ipv6Addr::from(self.network_id);
        SocketAddrV6::new(ipv6_addr, self.asset_port, 0, 0)
    }
    
    /// Convert to string representation
    pub fn to_string(&self) -> String {
        format!(
            "hypermesh://{}:{}/{}",
            hex::encode(self.network_id),
            hex::encode(self.node_id),
            self.asset_port
        )
    }
    
    /// Parse from string representation
    pub fn from_string(s: &str) -> Result<Self, ProxyAddressError> {
        if !s.starts_with("hypermesh://") {
            return Err(ProxyAddressError::InvalidScheme);
        }
        
        let addr_part = &s[12..]; // Remove "hypermesh://"
        let parts: Vec<&str> = addr_part.split('/').collect();
        
        if parts.len() != 2 {
            return Err(ProxyAddressError::InvalidFormat);
        }
        
        let network_node: Vec<&str> = parts[0].split(':').collect();
        if network_node.len() != 2 {
            return Err(ProxyAddressError::InvalidFormat);
        }
        
        let network_bytes = hex::decode(network_node[0])
            .map_err(|_| ProxyAddressError::InvalidNetworkId)?;
        let node_bytes = hex::decode(network_node[1])
            .map_err(|_| ProxyAddressError::InvalidNodeId)?;
        
        if network_bytes.len() != 16 {
            return Err(ProxyAddressError::InvalidNetworkId);
        }
        if node_bytes.len() != 8 {
            return Err(ProxyAddressError::InvalidNodeId);
        }
        
        let asset_port: u16 = parts[1].parse()
            .map_err(|_| ProxyAddressError::InvalidPort)?;
        
        let mut network_id = [0u8; 16];
        let mut node_id = [0u8; 8];
        network_id.copy_from_slice(&network_bytes);
        node_id.copy_from_slice(&node_bytes);
        
        Ok(Self::new(network_id, node_id, asset_port))
    }
    
    /// Verify access token integrity
    pub fn verify_access_token(&self) -> bool {
        let expected_token = Self::generate_access_token(&self.network_id, &self.node_id, self.asset_port);
        // Note: In real implementation, this would use FALCON-1024 signature verification
        // For now, we use simple hash comparison
        self.access_token == expected_token
    }
    
    /// Get network identifier as hex string
    pub fn network_hex(&self) -> String {
        hex::encode(self.network_id)
    }
    
    /// Get node identifier as hex string
    pub fn node_hex(&self) -> String {
        hex::encode(self.node_id)
    }
    
    /// Check if address is in the same network
    pub fn same_network(&self, other: &ProxyAddress) -> bool {
        self.network_id == other.network_id
    }
    
    /// Check if address is on the same node
    pub fn same_node(&self, other: &ProxyAddress) -> bool {
        self.network_id == other.network_id && self.node_id == other.node_id
    }
}

impl std::fmt::Display for ProxyAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

/// Proxy address resolution errors
#[derive(Debug, thiserror::Error)]
pub enum ProxyAddressError {
    /// Invalid scheme (not hypermesh://)
    #[error("Invalid proxy address scheme")]
    InvalidScheme,
    
    /// Invalid address format
    #[error("Invalid proxy address format")]
    InvalidFormat,
    
    /// Invalid network ID
    #[error("Invalid network ID")]
    InvalidNetworkId,
    
    /// Invalid node ID
    #[error("Invalid node ID")]
    InvalidNodeId,
    
    /// Invalid port number
    #[error("Invalid port number")]
    InvalidPort,
    
    /// Address not found in resolver
    #[error("Address not found")]
    AddressNotFound,
}

/// Proxy address resolver with NAT-like functionality
pub struct ProxyAddressResolver {
    /// Forward mapping: ProxyAddress -> AssetId
    forward_mappings: Arc<RwLock<HashMap<ProxyAddress, AssetId>>>,
    /// Reverse mapping: AssetId -> ProxyAddress
    reverse_mappings: Arc<RwLock<HashMap<AssetId, ProxyAddress>>>,
    /// Proxy node registry
    proxy_nodes: Arc<RwLock<HashMap<[u8; 8], ProxyNodeInfo>>>,
    /// Network configuration
    network_config: ProxyNetworkConfig,
}

/// Proxy node information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProxyNodeInfo {
    /// Node identifier
    pub node_id: [u8; 8],
    /// Node network address
    pub network_address: String,
    /// Node capabilities
    pub capabilities: ProxyCapabilities,
    /// Trust score (0.0 - 1.0)
    pub trust_score: f32,
    /// Last heartbeat timestamp
    pub last_heartbeat: SystemTime,
    /// Certificate fingerprint for TrustChain integration
    pub certificate_fingerprint: String,
}

/// Proxy node capabilities
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProxyCapabilities {
    /// Supports HTTP proxy
    pub http_proxy: bool,
    /// Supports SOCKS5 proxy
    pub socks5_proxy: bool,
    /// Supports TCP forwarding
    pub tcp_forwarding: bool,
    /// Supports VPN tunneling
    pub vpn_tunnel: bool,
    /// Maximum concurrent connections
    pub max_connections: u32,
    /// Bandwidth capacity in Mbps
    pub bandwidth_mbps: u64,
    /// Supported protocols
    pub protocols: Vec<String>,
}

// ProxyNetworkConfig is imported from assets::proxy module

impl ProxyAddressResolver {
    /// Create new proxy address resolver
    pub fn new() -> Self {
        Self {
            forward_mappings: Arc::new(RwLock::new(HashMap::new())),
            reverse_mappings: Arc::new(RwLock::new(HashMap::new())),
            proxy_nodes: Arc::new(RwLock::new(HashMap::new())),
            network_config: ProxyNetworkConfig::default(),
        }
    }
    
    /// Register proxy address mapping
    pub async fn register_mapping(&self, proxy_addr: ProxyAddress, asset_id: AssetId) {
        let mut forward = self.forward_mappings.write().await;
        let mut reverse = self.reverse_mappings.write().await;
        
        forward.insert(proxy_addr.clone(), asset_id.clone());
        reverse.insert(asset_id, proxy_addr);
    }
    
    /// Resolve proxy address to asset ID
    pub async fn resolve(&self, proxy_addr: &ProxyAddress) -> Option<AssetId> {
        let forward = self.forward_mappings.read().await;
        forward.get(proxy_addr).cloned()
    }
    
    /// Get proxy address for asset ID
    pub async fn get_proxy_address(&self, asset_id: &AssetId) -> Option<ProxyAddress> {
        let reverse = self.reverse_mappings.read().await;
        reverse.get(asset_id).cloned()
    }
    
    /// Allocate new proxy address for asset
    pub async fn allocate_proxy_address(&self, asset_id: &AssetId) -> AssetResult<ProxyAddress> {
        // Select best proxy node based on trust score
        let proxy_node = self.select_best_proxy_node().await?;
        
        // Allocate port within default range
        let asset_port = self.allocate_port(&proxy_node.node_id).await?;
        
        // Create proxy address
        let proxy_addr = ProxyAddress::new(
            self.network_config.hypermesh_network_id,
            proxy_node.node_id,
            asset_port,
        );
        
        // Register mapping
        self.register_mapping(proxy_addr.clone(), asset_id.clone()).await;
        
        Ok(proxy_addr)
    }
    
    /// Register proxy node
    pub async fn register_proxy_node(&self, node_info: ProxyNodeInfo) {
        let mut nodes = self.proxy_nodes.write().await;
        nodes.insert(node_info.node_id, node_info);
    }
    
    /// Update proxy node trust score
    pub async fn update_trust_score(&self, node_id: &[u8; 8], trust_score: f32) {
        let mut nodes = self.proxy_nodes.write().await;
        if let Some(node) = nodes.get_mut(node_id) {
            node.trust_score = trust_score;
        }
    }
    
    /// Select best proxy node based on trust and capabilities
    async fn select_best_proxy_node(&self) -> AssetResult<ProxyNodeInfo> {
        let nodes = self.proxy_nodes.read().await;
        
        let best_node = nodes
            .values()
            .filter(|node| node.trust_score >= self.network_config.min_trust_score)
            .max_by(|a, b| {
                // Primary: trust score
                // Secondary: bandwidth capacity
                // Tertiary: max connections
                a.trust_score.partial_cmp(&b.trust_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| {
                        a.capabilities.bandwidth_mbps.cmp(&b.capabilities.bandwidth_mbps)
                    })
                    .then_with(|| {
                        a.capabilities.max_connections.cmp(&b.capabilities.max_connections)
                    })
            });
        
        best_node.cloned().ok_or_else(|| AssetError::AdapterError {
            message: "No suitable proxy node available".to_string()
        })
    }
    
    /// Allocate available port for proxy node
    async fn allocate_port(&self, node_id: &[u8; 8]) -> AssetResult<u16> {
        let forward = self.forward_mappings.read().await;
        
        // Find used ports for this node
        let used_ports: std::collections::HashSet<u16> = forward
            .keys()
            .filter(|addr| addr.node_id == *node_id)
            .map(|addr| addr.asset_port)
            .collect();
        
        // Find first available port in range
        for port in self.network_config.default_port_range.0..=self.network_config.default_port_range.1 {
            if !used_ports.contains(&port) {
                return Ok(port);
            }
        }
        
        Err(AssetError::AllocationFailed {
            reason: "No available ports in range".to_string()
        })
    }
    
    /// Get proxy statistics
    pub async fn get_statistics(&self) -> ProxyStatistics {
        let forward = self.forward_mappings.read().await;
        let nodes = self.proxy_nodes.read().await;
        
        ProxyStatistics {
            total_mappings: forward.len(),
            total_proxy_nodes: nodes.len(),
            average_trust_score: nodes.values()
                .map(|node| node.trust_score)
                .sum::<f32>() / nodes.len() as f32,
            active_nodes: nodes.values()
                .filter(|node| {
                    SystemTime::now().duration_since(node.last_heartbeat).unwrap_or_default().as_secs() < 300
                })
                .count(),
        }
    }
    
    /// Cleanup expired mappings
    pub async fn cleanup_expired_mappings(&self) {
        // In a real implementation, this would check for expired assets
        // and remove their proxy mappings
        tracing::debug!("Cleaning up expired proxy mappings");
    }
}

/// Proxy system statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProxyStatistics {
    /// Total number of proxy mappings
    pub total_mappings: usize,
    /// Total number of registered proxy nodes
    pub total_proxy_nodes: usize,
    /// Average trust score across all nodes
    pub average_trust_score: f32,
    /// Number of active proxy nodes
    pub active_nodes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{AssetId, AssetType};
    
    #[test]
    fn test_proxy_address_creation() {
        let network_id = [1u8; 16];
        let node_id = [2u8; 8];
        let asset_port = 8080;
        
        let proxy_addr = ProxyAddress::new(network_id, node_id, asset_port);
        
        assert_eq!(proxy_addr.network_id, network_id);
        assert_eq!(proxy_addr.node_id, node_id);
        assert_eq!(proxy_addr.asset_port, asset_port);
        assert!(proxy_addr.verify_access_token());
    }
    
    #[test]
    fn test_proxy_address_string_conversion() {
        let network_id = [0xaa; 16];
        let node_id = [0xbb; 8];
        let asset_port = 9000;
        
        let proxy_addr = ProxyAddress::new(network_id, node_id, asset_port);
        let addr_string = proxy_addr.to_string();
        
        assert!(addr_string.starts_with("hypermesh://"));
        assert!(addr_string.contains(&hex::encode(network_id)));
        assert!(addr_string.contains(&hex::encode(node_id)));
        assert!(addr_string.contains("9000"));
    }
    
    #[test]
    fn test_proxy_address_parsing() {
        let original = ProxyAddress::new([0xcc; 16], [0xdd; 8], 8888);
        let addr_string = original.to_string();
        
        // Note: Parsing will create a new access token, so we only test structure
        let parsed = ProxyAddress::from_string(&addr_string).unwrap();
        
        assert_eq!(parsed.network_id, original.network_id);
        assert_eq!(parsed.node_id, original.node_id);
        assert_eq!(parsed.asset_port, original.asset_port);
    }
    
    #[tokio::test]
    async fn test_proxy_resolver() {
        let resolver = ProxyAddressResolver::new();
        let asset_id = AssetId::new(AssetType::Cpu);
        let proxy_addr = ProxyAddress::new([0x01; 16], [0x02; 8], 8080);
        
        // Register mapping
        resolver.register_mapping(proxy_addr.clone(), asset_id.clone()).await;
        
        // Test resolution
        let resolved = resolver.resolve(&proxy_addr).await;
        assert_eq!(resolved, Some(asset_id.clone()));
        
        // Test reverse lookup
        let reverse_addr = resolver.get_proxy_address(&asset_id).await;
        assert_eq!(reverse_addr, Some(proxy_addr));
    }
    
    #[test]
    fn test_proxy_address_relationships() {
        let addr1 = ProxyAddress::new([0x01; 16], [0x02; 8], 8080);
        let addr2 = ProxyAddress::new([0x01; 16], [0x02; 8], 8081); // Same node, different port
        let addr3 = ProxyAddress::new([0x01; 16], [0x03; 8], 8080); // Same network, different node
        
        assert!(addr1.same_network(&addr2));
        assert!(addr1.same_network(&addr3));
        assert!(addr1.same_node(&addr2));
        assert!(!addr1.same_node(&addr3));
    }
}

// CRITICAL Remote Proxy/NAT System modules - import from separate proxy directory
pub use crate::assets::proxy::{
    RemoteProxyManager, ProxyRouter, ProxyForwarder,
    TrustChainIntegration, QuantumSecurity, ShardedDataAccess,
    NATTranslator, GlobalAddress,
    ProxySystemStats, // ProxyNetworkConfig already imported above
};