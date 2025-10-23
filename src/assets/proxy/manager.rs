//! Remote Proxy Manager - Core NAT-like system for HyperMesh
//!
//! CRITICAL IMPLEMENTATION: The main proxy manager that coordinates all NAT-like
//! addressing, routing, forwarding, and security functions.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

use super::{
    ProxyNetworkConfig, ProxySystemStats, ProxySystemError,
    ProxyRouter, ProxyForwarder, TrustChainIntegration,
    QuantumSecurity, ShardedDataAccess, NATTranslator,
    PortRange
};

use crate::assets::core::{
    ProxyAddress, AssetId, AssetResult, AssetError, PrivacyLevel,
    ProxyNodeInfo, ProxyCapabilities
};

/// The main Remote Proxy Manager implementing NAT-like addressing
pub struct RemoteProxyManager {
    /// Network configuration
    config: ProxyNetworkConfig,
    
    /// Router for proxy traffic
    router: Arc<ProxyRouter>,
    
    /// Forwarder for actual traffic handling  
    forwarder: Arc<ProxyForwarder>,
    
    /// Trust integration with TrustChain
    trust_integration: Arc<TrustChainIntegration>,
    
    /// Quantum security handler
    quantum_security: Arc<QuantumSecurity>,
    
    /// Sharded data access handler
    sharded_access: Arc<ShardedDataAccess>,
    
    /// NAT address translator
    nat_translator: Arc<NATTranslator>,
    
    /// Active proxy nodes registry
    proxy_nodes: Arc<RwLock<HashMap<String, ProxyNodeInfo>>>,
    
    /// Proxy address mappings
    address_mappings: Arc<RwLock<HashMap<ProxyAddress, ProxyMapping>>>,
    
    /// System statistics
    stats: Arc<RwLock<ProxySystemStats>>,
    
    /// Port allocation tracking
    port_allocations: Arc<RwLock<HashMap<String, Vec<u16>>>>, // node_id -> allocated_ports
}

/// Internal proxy mapping structure
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ProxyMapping {
    /// Source proxy address
    proxy_address: ProxyAddress,
    
    /// Target asset ID
    target_asset_id: AssetId,
    
    /// Target node information
    target_node_id: String,
    
    /// Local address on target node
    local_address: String,
    
    /// Privacy level for access control
    privacy_level: PrivacyLevel,
    
    /// Forwarding rules
    forwarding_rules: Vec<ForwardingRule>,
    
    /// Access permissions
    access_permissions: AccessPermissions,
    
    /// Quantum security tokens
    quantum_tokens: Vec<u8>,
    
    /// Creation timestamp
    created_at: SystemTime,
    
    /// Expiry timestamp
    expires_at: SystemTime,
    
    /// Usage statistics
    usage_stats: MappingUsageStats,
}

/// Access permissions for proxy mappings
#[derive(Clone, Debug, Serialize, Deserialize)]
struct AccessPermissions {
    /// HTTP proxy access
    http_proxy: bool,
    
    /// SOCKS5 proxy access
    socks5_proxy: bool,
    
    /// TCP forwarding access
    tcp_forwarding: bool,
    
    /// VPN tunnel access
    vpn_tunnel: bool,
    
    /// Direct memory access
    memory_access: bool,
    
    /// Sharded data access
    sharded_access: bool,
}

/// Usage statistics for individual mappings
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct MappingUsageStats {
    /// Total requests forwarded
    total_requests: u64,
    
    /// Total bytes transferred
    total_bytes_transferred: u64,
    
    /// Last access timestamp
    last_accessed: SystemTime,
    
    /// Average response time
    average_response_time_ms: f64,
}

/// Forwarding rule for proxy traffic
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForwardingRule {
    /// Rule type (HTTP, SOCKS5, TCP, etc.)
    rule_type: ForwardingRuleType,
    
    /// Source address pattern
    source_pattern: String,
    
    /// Destination mapping
    destination: String,
    
    /// Port mapping
    port_mapping: Option<(u16, u16)>, // (source_port, dest_port)
    
    /// Protocol specific settings
    protocol_settings: HashMap<String, String>,
}

/// Types of forwarding rules
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ForwardingRuleType {
    HTTP,
    HTTPS,
    SOCKS5,
    TCP,
    UDP,
    VPN,
    DirectMemory,
    ShardedData,
}

impl RemoteProxyManager {
    /// Create new remote proxy manager
    pub async fn new(config: ProxyNetworkConfig) -> AssetResult<Self> {
        let router = Arc::new(ProxyRouter::new().await?);
        let forwarder = Arc::new(ProxyForwarder::new().await?);
        let trust_integration = Arc::new(TrustChainIntegration::new().await?);
        let quantum_security = Arc::new(QuantumSecurity::new().await?);
        let sharded_access = Arc::new(ShardedDataAccess::new().await?);
        let nat_translator = Arc::new(NATTranslator::new().await?);
        
        Ok(Self {
            config,
            router,
            forwarder,
            trust_integration,
            quantum_security,
            sharded_access,
            nat_translator,
            proxy_nodes: Arc::new(RwLock::new(HashMap::new())),
            address_mappings: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ProxySystemStats {
                active_proxy_nodes: 0,
                total_mappings: 0,
                forwarded_requests: 0,
                nat_translations: 0,
                average_response_time_ms: 0.0,
                quantum_validations: 0,
                trust_validations: 0,
                sharded_requests: 0,
            })),
            port_allocations: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Register a new proxy node
    pub async fn register_proxy_node(&self, node_info: ProxyNodeInfo) -> AssetResult<()> {
        // Validate node with TrustChain
        if !self.trust_integration.validate_node_certificate(&node_info).await? {
            return Err(AssetError::AdapterError {
                message: "Node certificate validation failed".to_string()
            });
        }
        
        // Initialize port allocation tracking for this node
        {
            let mut allocations = self.port_allocations.write().await;
            allocations.insert(node_info.node_id.clone(), Vec::new());
        }
        
        // Register node in router
        self.router.add_proxy_node(&node_info).await?;
        
        // Store node info
        {
            let mut nodes = self.proxy_nodes.write().await;
            nodes.insert(node_info.node_id.clone(), node_info);
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.active_proxy_nodes += 1;
        }
        
        tracing::info!("Registered new proxy node: {}", node_info.node_id);
        Ok(())
    }
    
    /// Allocate a new proxy address for an asset (CRITICAL NAT functionality)
    pub async fn allocate_proxy_address(
        &self,
        asset_id: &AssetId,
        privacy_level: PrivacyLevel,
        capabilities_required: &[String],
    ) -> AssetResult<ProxyAddress> {
        
        // Select best proxy node based on trust, capabilities, and load
        let selected_node = self.select_best_proxy_node(capabilities_required).await?;
        
        // Allocate port for this asset type
        let asset_type_str = format!("{:?}", asset_id.asset_type).to_lowercase();
        let port = self.allocate_port_for_node(&selected_node.node_id, &asset_type_str).await?;
        
        // Generate global proxy address using NAT-like addressing
        let proxy_address = self.nat_translator.generate_global_address(
            &selected_node.node_id,
            asset_id,
            port,
        ).await?;
        
        // Create quantum security tokens
        let quantum_tokens = self.quantum_security.generate_access_tokens(&proxy_address).await?;
        
        // Create forwarding rules based on privacy level
        let forwarding_rules = self.create_forwarding_rules(&privacy_level, &asset_type_str).await?;
        
        // Create access permissions
        let access_permissions = self.create_access_permissions(&privacy_level).await?;
        
        // Create proxy mapping
        let mapping = ProxyMapping {
            proxy_address: proxy_address.clone(),
            target_asset_id: asset_id.clone(),
            target_node_id: selected_node.node_id.clone(),
            local_address: format!("local://{}:{}", selected_node.network_address, port),
            privacy_level,
            forwarding_rules,
            access_permissions,
            quantum_tokens,
            created_at: SystemTime::now(),
            expires_at: SystemTime::now() + Duration::from_secs(3600), // 1 hour default
            usage_stats: MappingUsageStats::default(),
        };
        
        // Install forwarding rules in the forwarder
        for rule in &mapping.forwarding_rules {
            self.forwarder.install_rule(&proxy_address, rule).await?;
        }
        
        // Store mapping
        {
            let mut mappings = self.address_mappings.write().await;
            mappings.insert(proxy_address.clone(), mapping);
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_mappings += 1;
            stats.nat_translations += 1;
        }
        
        tracing::info!(
            "Allocated proxy address {} for asset {} on node {}",
            proxy_address,
            asset_id,
            selected_node.node_id
        );
        
        Ok(proxy_address)
    }
    
    /// Resolve proxy address to local asset information (CRITICAL NAT functionality)
    pub async fn resolve_proxy_address(&self, proxy_addr: &ProxyAddress) -> AssetResult<AssetId> {
        let mappings = self.address_mappings.read().await;
        let mapping = mappings.get(proxy_addr)
            .ok_or_else(|| AssetError::ProxyResolutionFailed {
                address: proxy_addr.clone()
            })?;
        
        // Check if mapping has expired
        if mapping.expires_at < SystemTime::now() {
            return Err(AssetError::AdapterError {
                message: "Proxy address mapping has expired".to_string()
            });
        }
        
        Ok(mapping.target_asset_id.clone())
    }
    
    /// Forward request through proxy system (CRITICAL NAT functionality)
    pub async fn forward_request(
        &self,
        proxy_addr: &ProxyAddress,
        request_data: Vec<u8>,
        request_type: ForwardingRuleType,
    ) -> AssetResult<Vec<u8>> {
        
        // Get mapping
        let mapping = {
            let mappings = self.address_mappings.read().await;
            mappings.get(proxy_addr)
                .ok_or_else(|| AssetError::ProxyResolutionFailed {
                    address: proxy_addr.clone()
                })?
                .clone()
        };
        
        // Validate quantum security
        if self.config.quantum_security_enabled {
            if !self.quantum_security.validate_access_tokens(&mapping.quantum_tokens).await? {
                return Err(AssetError::AdapterError {
                    message: "Quantum security validation failed".to_string()
                });
            }
            
            // Update quantum validation stats
            {
                let mut stats = self.stats.write().await;
                stats.quantum_validations += 1;
            }
        }
        
        // Check privacy level access
        if !self.check_privacy_access(&mapping.privacy_level, &request_type).await? {
            return Err(AssetError::AdapterError {
                message: "Privacy level access denied".to_string()
            });
        }
        
        // Forward the request
        let response = self.forwarder.forward_request(
            proxy_addr,
            &mapping.local_address,
            request_data,
            request_type,
        ).await?;
        
        // Update usage statistics
        self.update_mapping_stats(proxy_addr, response.len() as u64).await?;
        
        // Update global statistics
        {
            let mut stats = self.stats.write().await;
            stats.forwarded_requests += 1;
            stats.total_bytes_transferred += response.len() as u64;
        }
        
        Ok(response)
    }
    
    /// Access sharded data through proxy system
    pub async fn access_sharded_data(
        &self,
        proxy_addr: &ProxyAddress,
        shard_key: &str,
    ) -> AssetResult<Vec<u8>> {
        
        // Get mapping
        let mapping = {
            let mappings = self.address_mappings.read().await;
            mappings.get(proxy_addr)
                .ok_or_else(|| AssetError::ProxyResolutionFailed {
                    address: proxy_addr.clone()
                })?
                .clone()
        };
        
        // Check sharded access permission
        if !mapping.access_permissions.sharded_access {
            return Err(AssetError::AdapterError {
                message: "Sharded data access not permitted".to_string()
            });
        }
        
        // Access sharded data
        let data = self.sharded_access.get_shard_data(
            &mapping.target_asset_id,
            shard_key,
        ).await?;
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.sharded_requests += 1;
        }
        
        Ok(data)
    }
    
    /// Select best proxy node based on capabilities and trust
    async fn select_best_proxy_node(
        &self,
        required_capabilities: &[String],
    ) -> AssetResult<ProxyNodeInfo> {
        let nodes = self.proxy_nodes.read().await;
        
        let mut best_node: Option<ProxyNodeInfo> = None;
        let mut best_score = 0.0_f32;
        
        for node in nodes.values() {
            // Check trust score threshold
            if node.trust_score < self.config.min_trust_score {
                continue;
            }
            
            // Check required capabilities
            let has_required_caps = required_capabilities.iter()
                .all(|cap| node.capabilities.protocols.contains(cap));
            
            if !has_required_caps {
                continue;
            }
            
            // Calculate composite score
            let score = self.calculate_node_score(node).await;
            
            if score > best_score {
                best_score = score;
                best_node = Some(node.clone());
            }
        }
        
        best_node.ok_or_else(|| AssetError::AdapterError {
            message: "No suitable proxy node found".to_string()
        })
    }
    
    /// Calculate composite score for proxy node selection
    async fn calculate_node_score(&self, node: &ProxyNodeInfo) -> f32 {
        // Weight factors
        let trust_weight = 0.4;
        let bandwidth_weight = 0.3;
        let connection_weight = 0.2;
        let latency_weight = 0.1;
        
        // Normalize scores to 0-1 range
        let trust_score = node.trust_score;
        let bandwidth_score = (node.capabilities.bandwidth_mbps as f32 / 10000.0).min(1.0);
        let connection_score = (node.capabilities.max_connections as f32 / 100000.0).min(1.0);
        
        // TODO: Implement actual latency measurement
        let latency_score = 0.8; // Placeholder
        
        trust_weight * trust_score +
        bandwidth_weight * bandwidth_score +
        connection_weight * connection_score +
        latency_weight * latency_score
    }
    
    /// Allocate port for a node and asset type
    async fn allocate_port_for_node(&self, node_id: &str, asset_type: &str) -> AssetResult<u16> {
        let port_range = self.config.port_ranges.get(asset_type)
            .ok_or_else(|| AssetError::AdapterError {
                message: format!("No port range configured for asset type: {}", asset_type)
            })?;
        
        let mut allocations = self.port_allocations.write().await;
        let allocated_ports = allocations.get_mut(node_id)
            .ok_or_else(|| AssetError::AdapterError {
                message: format!("Node not found in port allocations: {}", node_id)
            })?;
        
        // Find first available port in range
        for port in port_range.start..=port_range.end {
            if !allocated_ports.contains(&port) {
                allocated_ports.push(port);
                return Ok(port);
            }
        }
        
        Err(AssetError::AdapterError {
            message: format!("No available ports in range for asset type: {}", asset_type)
        })
    }
    
    /// Create forwarding rules based on privacy level
    async fn create_forwarding_rules(
        &self,
        privacy_level: &PrivacyLevel,
        asset_type: &str,
    ) -> AssetResult<Vec<ForwardingRule>> {
        let mut rules = Vec::new();
        
        match privacy_level {
            PrivacyLevel::Private => {
                // Only direct memory access for private assets
                rules.push(ForwardingRule {
                    rule_type: ForwardingRuleType::DirectMemory,
                    source_pattern: "local".to_string(),
                    destination: "direct".to_string(),
                    port_mapping: None,
                    protocol_settings: HashMap::new(),
                });
            },
            PrivacyLevel::PrivateNetwork => {
                // Limited network access
                rules.push(ForwardingRule {
                    rule_type: ForwardingRuleType::TCP,
                    source_pattern: "private_network".to_string(),
                    destination: "forwarded".to_string(),
                    port_mapping: None,
                    protocol_settings: HashMap::new(),
                });
            },
            PrivacyLevel::P2P | PrivacyLevel::PublicNetwork | PrivacyLevel::FullPublic => {
                // Full proxy capabilities
                rules.push(ForwardingRule {
                    rule_type: ForwardingRuleType::HTTP,
                    source_pattern: "*".to_string(),
                    destination: "forwarded".to_string(),
                    port_mapping: None,
                    protocol_settings: HashMap::new(),
                });
                
                rules.push(ForwardingRule {
                    rule_type: ForwardingRuleType::SOCKS5,
                    source_pattern: "*".to_string(),
                    destination: "forwarded".to_string(),
                    port_mapping: None,
                    protocol_settings: HashMap::new(),
                });
                
                if matches!(privacy_level, PrivacyLevel::FullPublic) {
                    rules.push(ForwardingRule {
                        rule_type: ForwardingRuleType::VPN,
                        source_pattern: "*".to_string(),
                        destination: "forwarded".to_string(),
                        port_mapping: None,
                        protocol_settings: HashMap::new(),
                    });
                }
            },
        }
        
        Ok(rules)
    }
    
    /// Create access permissions based on privacy level
    async fn create_access_permissions(&self, privacy_level: &PrivacyLevel) -> AssetResult<AccessPermissions> {
        Ok(match privacy_level {
            PrivacyLevel::Private => AccessPermissions {
                http_proxy: false,
                socks5_proxy: false,
                tcp_forwarding: false,
                vpn_tunnel: false,
                memory_access: true,
                sharded_access: false,
            },
            PrivacyLevel::PrivateNetwork => AccessPermissions {
                http_proxy: false,
                socks5_proxy: false,
                tcp_forwarding: true,
                vpn_tunnel: false,
                memory_access: true,
                sharded_access: true,
            },
            PrivacyLevel::P2P => AccessPermissions {
                http_proxy: true,
                socks5_proxy: true,
                tcp_forwarding: true,
                vpn_tunnel: false,
                memory_access: true,
                sharded_access: true,
            },
            PrivacyLevel::PublicNetwork => AccessPermissions {
                http_proxy: true,
                socks5_proxy: true,
                tcp_forwarding: true,
                vpn_tunnel: true,
                memory_access: true,
                sharded_access: true,
            },
            PrivacyLevel::FullPublic => AccessPermissions {
                http_proxy: true,
                socks5_proxy: true,
                tcp_forwarding: true,
                vpn_tunnel: true,
                memory_access: true,
                sharded_access: true,
            },
        })
    }
    
    /// Check privacy level access for request type
    async fn check_privacy_access(
        &self,
        privacy_level: &PrivacyLevel,
        request_type: &ForwardingRuleType,
    ) -> AssetResult<bool> {
        let permissions = self.create_access_permissions(privacy_level).await?;
        
        Ok(match request_type {
            ForwardingRuleType::HTTP | ForwardingRuleType::HTTPS => permissions.http_proxy,
            ForwardingRuleType::SOCKS5 => permissions.socks5_proxy,
            ForwardingRuleType::TCP | ForwardingRuleType::UDP => permissions.tcp_forwarding,
            ForwardingRuleType::VPN => permissions.vpn_tunnel,
            ForwardingRuleType::DirectMemory => permissions.memory_access,
            ForwardingRuleType::ShardedData => permissions.sharded_access,
        })
    }
    
    /// Update mapping usage statistics
    async fn update_mapping_stats(&self, proxy_addr: &ProxyAddress, bytes_transferred: u64) -> AssetResult<()> {
        let mut mappings = self.address_mappings.write().await;
        if let Some(mapping) = mappings.get_mut(proxy_addr) {
            mapping.usage_stats.total_requests += 1;
            mapping.usage_stats.total_bytes_transferred += bytes_transferred;
            mapping.usage_stats.last_accessed = SystemTime::now();
        }
        Ok(())
    }
    
    /// Get system statistics
    pub async fn get_system_stats(&self) -> AssetResult<ProxySystemStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
    
    /// Cleanup expired mappings
    pub async fn cleanup_expired_mappings(&self) -> AssetResult<u64> {
        let mut mappings = self.address_mappings.write().await;
        let now = SystemTime::now();
        
        let initial_count = mappings.len();
        mappings.retain(|_, mapping| mapping.expires_at > now);
        let final_count = mappings.len();
        
        let removed_count = initial_count - final_count;
        
        // Update statistics
        if removed_count > 0 {
            let mut stats = self.stats.write().await;
            stats.total_mappings = stats.total_mappings.saturating_sub(removed_count as u64);
        }
        
        Ok(removed_count as u64)
    }
    
    /// Shutdown proxy manager
    pub async fn shutdown(&self) -> AssetResult<()> {
        // Cleanup all mappings
        self.cleanup_expired_mappings().await?;
        
        // Clear proxy nodes
        {
            let mut nodes = self.proxy_nodes.write().await;
            nodes.clear();
        }
        
        // Clear port allocations
        {
            let mut allocations = self.port_allocations.write().await;
            allocations.clear();
        }
        
        tracing::info!("Remote Proxy Manager shutdown completed");
        Ok(())
    }
}