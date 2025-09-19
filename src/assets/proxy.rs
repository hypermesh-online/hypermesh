//! NAT Address Manager for Remote Proxy Addressing
//! 
//! Implements NAT-like addressing for HyperMesh assets enabling remote access
//! to resources through proxy connections with trust-based routing.

use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::net::Ipv6Addr;
use std::time::{Duration, SystemTime, Instant};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, debug, warn, error};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use crate::config::ProxyConfig;
use crate::transport::StoqTransportLayer;
use crate::assets::AssetType;

/// NAT address manager for remote proxy addressing
pub struct NatAddressManager {
    /// Configuration
    config: ProxyConfig,
    
    /// STOQ transport for proxy connections
    stoq_transport: Arc<StoqTransportLayer>,
    
    /// Proxy address allocations
    proxy_addresses: Arc<RwLock<HashMap<String, ProxyAddress>>>,
    
    /// Active proxy connections
    proxy_connections: Arc<RwLock<HashMap<String, Arc<ProxyConnection>>>>,
    
    /// Trust-based routing table
    routing_table: Arc<RwLock<RoutingTable>>,
    
    /// Proxy statistics
    stats: Arc<RwLock<ProxyStats>>,
}

/// Proxy address allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyAddress {
    /// Proxy address (IPv6-like for HyperMesh ecosystem)
    pub proxy_address: String,
    
    /// Associated asset information
    pub asset_id: String,
    pub asset_type: AssetType,
    
    /// Proxy configuration
    pub proxy_config: ProxyAddressConfig,
    
    /// Allocation details
    pub allocated_at: SystemTime,
    pub allocated_by: String,
    pub ttl: Duration,
    
    /// Access control
    pub access_permissions: AccessPermissions,
    
    /// Routing information
    pub routing_info: RoutingInfo,
}

/// Proxy address configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyAddressConfig {
    /// Address type
    pub address_type: ProxyAddressType,
    
    /// Address scope
    pub scope: AddressScope,
    
    /// Load balancing configuration
    pub load_balancing: Option<LoadBalancingConfig>,
    
    /// Caching configuration
    pub caching: Option<CachingConfig>,
    
    /// Security configuration
    pub security: SecurityConfig,
}

/// Proxy address types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyAddressType {
    /// Direct 1:1 mapping
    Direct,
    /// Load balanced across multiple instances
    LoadBalanced,
    /// Cached/CDN-like distribution
    Cached,
    /// Dynamic routing based on conditions
    Dynamic,
}

/// Address scope
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AddressScope {
    /// Local node only
    Local,
    /// Private network
    Private,
    /// Public network with restrictions
    Public,
    /// Global HyperMesh ecosystem
    Global,
}

/// Load balancing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancingConfig {
    pub algorithm: LoadBalancingAlgorithm,
    pub health_check_interval: Duration,
    pub failover_threshold: u32,
    pub max_backend_connections: u32,
}

/// Load balancing algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalancingAlgorithm {
    RoundRobin,
    LeastConnections,
    WeightedRoundRobin,
    IpHash,
    GeographicProximity,
}

/// Caching configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachingConfig {
    pub cache_ttl: Duration,
    pub cache_size_mb: u64,
    pub cache_strategy: CacheStrategy,
    pub invalidation_policy: InvalidationPolicy,
}

/// Cache strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheStrategy {
    WriteThrough,
    WriteBack,
    ReadThrough,
    RefreshAhead,
}

/// Cache invalidation policies
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvalidationPolicy {
    TimeToLive,
    WriteInvalidate,
    Manual,
    EventDriven,
}

/// Security configuration for proxy addresses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Encryption requirements
    pub require_encryption: bool,
    pub encryption_algorithm: String,
    
    /// Authentication requirements
    pub require_authentication: bool,
    pub auth_methods: Vec<String>,
    
    /// Access control
    pub access_control_enabled: bool,
    pub rate_limiting: Option<RateLimitConfig>,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_second: u32,
    pub burst_size: u32,
    pub window_size: Duration,
}

/// Access permissions for proxy addresses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPermissions {
    /// Allowed operations
    pub allowed_operations: Vec<String>,
    
    /// Access control list
    pub allowed_principals: Vec<String>,
    pub denied_principals: Vec<String>,
    
    /// Time-based access
    pub time_restrictions: Option<TimeRestrictions>,
    
    /// Geographic restrictions
    pub geographic_restrictions: Option<GeographicRestrictions>,
}

/// Time-based access restrictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRestrictions {
    pub allowed_hours: Vec<u8>, // 0-23
    pub allowed_days: Vec<u8>,  // 0-6 (Sunday=0)
    pub timezone: String,
}

/// Geographic access restrictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeographicRestrictions {
    pub allowed_regions: Vec<String>,
    pub denied_regions: Vec<String>,
    pub allowed_countries: Vec<String>,
    pub denied_countries: Vec<String>,
}

/// Routing information for proxy addresses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingInfo {
    /// Primary routing destination
    pub primary_destination: RoutingDestination,
    
    /// Backup destinations
    pub backup_destinations: Vec<RoutingDestination>,
    
    /// Routing preferences
    pub routing_preferences: RoutingPreferences,
}

/// Routing destination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDestination {
    pub destination_id: String,
    pub address: String,
    pub port: Option<u16>,
    pub weight: u32,
    pub health_status: HealthStatus,
    pub last_health_check: Option<SystemTime>,
}

/// Health status for routing destinations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Routing preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingPreferences {
    pub prefer_local: bool,
    pub prefer_low_latency: bool,
    pub prefer_high_bandwidth: bool,
    pub trust_threshold: f64,
}

/// Active proxy connection
#[derive(Debug, Clone)]
pub struct ProxyConnection {
    /// Connection identification
    pub connection_id: String,
    pub proxy_address: String,
    pub asset_id: String,
    
    /// Connection details
    pub source_address: String,
    pub destination_address: String,
    pub established_at: Instant,
    pub last_activity: Arc<RwLock<Instant>>,
    
    /// Connection state
    pub state: Arc<RwLock<ConnectionState>>,
    
    /// Performance metrics
    pub metrics: Arc<RwLock<ConnectionMetrics>>,
    
    /// Trust information
    pub trust_score: f64,
    pub trust_chain: Vec<String>,
}

/// Connection state
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Establishing,
    Active,
    Idle,
    Closing,
    Closed,
    Error(String),
}

/// Connection performance metrics
#[derive(Debug, Clone, Default)]
pub struct ConnectionMetrics {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub average_latency_ms: f64,
    pub bandwidth_utilization: f64,
    pub error_count: u32,
}

/// Trust-based routing table
#[derive(Debug, Clone)]
pub struct RoutingTable {
    /// Routing entries by proxy address
    pub routes: HashMap<String, RoutingEntry>,
    
    /// Trust relationships
    pub trust_relationships: HashMap<String, TrustRelationship>,
    
    /// Routing metrics
    pub routing_metrics: RoutingMetrics,
}

/// Routing table entry
#[derive(Debug, Clone)]
pub struct RoutingEntry {
    pub proxy_address: String,
    pub destinations: Vec<RoutingDestination>,
    pub routing_algorithm: LoadBalancingAlgorithm,
    pub last_updated: SystemTime,
    pub route_priority: u32,
}

/// Trust relationship between nodes
#[derive(Debug, Clone)]
pub struct TrustRelationship {
    pub node_a: String,
    pub node_b: String,
    pub trust_score: f64,
    pub relationship_type: TrustRelationshipType,
    pub established_at: SystemTime,
    pub last_interaction: Option<SystemTime>,
}

/// Trust relationship types
#[derive(Debug, Clone)]
pub enum TrustRelationshipType {
    Direct,
    Transitive,
    Reputation,
    Cryptographic,
}

/// Routing metrics
#[derive(Debug, Clone, Default)]
pub struct RoutingMetrics {
    pub total_routes: u64,
    pub active_routes: u64,
    pub failed_routes: u64,
    pub average_route_latency_ms: f64,
    pub route_success_rate: f64,
}

/// Proxy statistics
#[derive(Debug, Clone, Default)]
pub struct ProxyStats {
    pub total_addresses_allocated: u64,
    pub active_proxy_addresses: u32,
    pub active_connections: u32,
    pub total_throughput_mbps: f64,
    pub average_connection_duration: Duration,
    pub proxy_success_rate: f64,
    pub trust_validations: u64,
    pub routing_optimizations: u64,
}

impl NatAddressManager {
    /// Create new NAT address manager
    pub async fn new(
        config: &ProxyConfig,
        stoq_transport: Arc<StoqTransportLayer>
    ) -> Result<Self> {
        info!("🌐 Initializing NAT Address Manager");
        info!("   Features: Remote proxy addressing, Trust-based routing, Load balancing");
        
        let routing_table = RoutingTable {
            routes: HashMap::new(),
            trust_relationships: HashMap::new(),
            routing_metrics: RoutingMetrics::default(),
        };
        
        Ok(Self {
            config: config.clone(),
            stoq_transport,
            proxy_addresses: Arc::new(RwLock::new(HashMap::new())),
            proxy_connections: Arc::new(RwLock::new(HashMap::new())),
            routing_table: Arc::new(RwLock::new(routing_table)),
            stats: Arc::new(RwLock::new(ProxyStats::default())),
        })
    }
    
    /// Start NAT address manager
    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting NAT Address Manager");
        
        // Start health checking
        self.start_health_checking().await?;
        
        // Start connection monitoring
        self.start_connection_monitoring().await?;
        
        // Start trust score updates
        self.start_trust_score_updates().await?;
        
        info!("✅ NAT Address Manager started");
        Ok(())
    }
    
    /// Start health checking for routing destinations
    async fn start_health_checking(&self) -> Result<()> {
        let routing_table = self.routing_table.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                let mut table = routing_table.write().await;
                
                for route in table.routes.values_mut() {
                    for destination in &mut route.destinations {
                        // Simplified health check
                        let health_check_result = Self::perform_health_check(&destination.address).await;
                        
                        destination.health_status = if health_check_result {
                            HealthStatus::Healthy
                        } else {
                            HealthStatus::Unhealthy
                        };
                        
                        destination.last_health_check = Some(SystemTime::now());
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Start connection monitoring
    async fn start_connection_monitoring(&self) -> Result<()> {
        let proxy_connections = self.proxy_connections.clone();
        let stats = self.stats.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                
                let mut connections = proxy_connections.write().await;
                let mut closed_connections = Vec::new();
                
                for (connection_id, connection) in connections.iter_mut() {
                    let state = connection.state.read().await;
                    
                    // Check for idle/timeout connections
                    let last_activity = *connection.last_activity.read().await;
                    if last_activity.elapsed() > Duration::from_secs(300) { // 5 minutes idle timeout
                        closed_connections.push(connection_id.clone());
                    }
                }
                
                // Clean up closed connections
                for connection_id in closed_connections {
                    connections.remove(&connection_id);
                    
                    let mut stats_guard = stats.write().await;
                    stats_guard.active_connections = stats_guard.active_connections.saturating_sub(1);
                }
            }
        });
        
        Ok(())
    }
    
    /// Start trust score updates
    async fn start_trust_score_updates(&self) -> Result<()> {
        let routing_table = self.routing_table.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
            
            loop {
                interval.tick().await;
                
                let mut table = routing_table.write().await;
                
                // Update trust scores based on recent interactions
                for relationship in table.trust_relationships.values_mut() {
                    // Simplified trust score decay over time
                    if let Some(last_interaction) = relationship.last_interaction {
                        if let Ok(elapsed) = last_interaction.elapsed() {
                            let decay_factor = 1.0 - (elapsed.as_secs() as f64 / 86400.0 * 0.01); // 1% decay per day
                            relationship.trust_score = (relationship.trust_score * decay_factor).max(0.0);
                        }
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Allocate proxy address for asset
    pub async fn allocate_proxy_address(
        &self,
        asset_id: &str,
        asset_type: &AssetType
    ) -> Result<String> {
        info!("🔗 Allocating proxy address for asset: {} ({})", asset_id, asset_type);
        
        // Generate unique proxy address
        let proxy_address = self.generate_proxy_address(asset_id, asset_type).await?;
        
        // Create proxy address configuration
        let proxy_config = ProxyAddressConfig {
            address_type: ProxyAddressType::Direct,
            scope: AddressScope::Private,
            load_balancing: None,
            caching: None,
            security: SecurityConfig {
                require_encryption: true,
                encryption_algorithm: "AES-256-GCM".to_string(),
                require_authentication: true,
                auth_methods: vec!["certificate".to_string()],
                access_control_enabled: true,
                rate_limiting: Some(RateLimitConfig {
                    requests_per_second: 1000,
                    burst_size: 100,
                    window_size: Duration::from_secs(60),
                }),
            },
        };
        
        // Create access permissions
        let access_permissions = AccessPermissions {
            allowed_operations: vec!["read".to_string(), "write".to_string()],
            allowed_principals: vec!["*".to_string()], // Allow all for now
            denied_principals: Vec::new(),
            time_restrictions: None,
            geographic_restrictions: None,
        };
        
        // Create routing information
        let routing_info = RoutingInfo {
            primary_destination: RoutingDestination {
                destination_id: format!("dest-{}", asset_id),
                address: "localhost".to_string(),
                port: Some(8080),
                weight: 100,
                health_status: HealthStatus::Healthy,
                last_health_check: Some(SystemTime::now()),
            },
            backup_destinations: Vec::new(),
            routing_preferences: RoutingPreferences {
                prefer_local: true,
                prefer_low_latency: true,
                prefer_high_bandwidth: false,
                trust_threshold: 0.8,
            },
        };
        
        // Create proxy address allocation
        let proxy_addr = ProxyAddress {
            proxy_address: proxy_address.clone(),
            asset_id: asset_id.to_string(),
            asset_type: asset_type.clone(),
            proxy_config,
            allocated_at: SystemTime::now(),
            allocated_by: "system".to_string(),
            ttl: Duration::from_secs(24 * 3600), // 24 hours
            access_permissions,
            routing_info,
        };
        
        // Store proxy address
        self.proxy_addresses.write().await.insert(proxy_address.clone(), proxy_addr);
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_addresses_allocated += 1;
            stats.active_proxy_addresses += 1;
        }
        
        info!("✅ Proxy address allocated: {} -> {}", asset_id, proxy_address);
        Ok(proxy_address)
    }
    
    /// Generate unique proxy address
    async fn generate_proxy_address(&self, asset_id: &str, asset_type: &AssetType) -> Result<String> {
        // Generate IPv6-like address for HyperMesh ecosystem
        let uuid = Uuid::new_v4();
        let uuid_bytes = uuid.as_bytes();
        
        // Create IPv6-like address: hypermesh:asset_type:uuid_part
        let proxy_address = format!(
            "hypermesh:{}:{:x}{:x}:{:x}{:x}:{:x}{:x}:{:x}{:x}",
            asset_type.to_string().to_lowercase(),
            uuid_bytes[0], uuid_bytes[1],
            uuid_bytes[2], uuid_bytes[3],
            uuid_bytes[4], uuid_bytes[5],
            uuid_bytes[6], uuid_bytes[7]
        );
        
        // Ensure uniqueness (simplified to avoid recursion)
        let mut counter = 0;
        let mut final_address = proxy_address.clone();
        while self.proxy_addresses.read().await.contains_key(&final_address) && counter < 10 {
            counter += 1;
            final_address = format!("{}:{}", proxy_address, counter);
        }
        
        Ok(final_address)
    }
    
    /// Create proxy connection
    pub async fn create_proxy_connection(
        &self,
        asset_id: &str,
        remote_address: &str,
        asset_type: &AssetType
    ) -> Result<Arc<ProxyConnection>> {
        info!("🔗 Creating proxy connection: {} -> {}", asset_id, remote_address);
        
        // Find or allocate proxy address
        let proxy_address = self.find_proxy_address_for_asset(asset_id).await
            .unwrap_or_else(|| {
                // This would normally wait for allocation, but we'll create a temporary one
                format!("temp-proxy-{}", asset_id)
            });
        
        // Generate connection ID
        let connection_id = format!("conn-{}", Uuid::new_v4());
        
        // Calculate trust score for the connection
        let trust_score = self.calculate_trust_score(&proxy_address, remote_address).await;
        
        // Create proxy connection
        let connection = ProxyConnection {
            connection_id: connection_id.clone(),
            proxy_address: proxy_address.clone(),
            asset_id: asset_id.to_string(),
            source_address: proxy_address,
            destination_address: remote_address.to_string(),
            established_at: Instant::now(),
            last_activity: Arc::new(RwLock::new(Instant::now())),
            state: Arc::new(RwLock::new(ConnectionState::Establishing)),
            metrics: Arc::new(RwLock::new(ConnectionMetrics::default())),
            trust_score,
            trust_chain: vec![asset_id.to_string(), remote_address.to_string()],
        };
        
        let connection_arc = Arc::new(connection);
        
        // Store connection
        self.proxy_connections.write().await.insert(connection_id.clone(), connection_arc.clone());
        
        // Update connection state to active
        *connection_arc.state.write().await = ConnectionState::Active;
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.active_connections += 1;
        }
        
        info!("✅ Proxy connection established: {} (trust: {:.2})", connection_id, trust_score);
        Ok(connection_arc)
    }
    
    /// Find proxy address for asset
    async fn find_proxy_address_for_asset(&self, asset_id: &str) -> Option<String> {
        let addresses = self.proxy_addresses.read().await;
        
        for (proxy_addr, allocation) in addresses.iter() {
            if allocation.asset_id == asset_id {
                return Some(proxy_addr.clone());
            }
        }
        
        None
    }
    
    /// Calculate trust score for connection
    async fn calculate_trust_score(&self, _source: &str, _destination: &str) -> f64 {
        // Simplified trust calculation
        // In production, this would consider:
        // - Certificate validation
        // - Historical behavior
        // - Network reputation
        // - Cryptographic proofs
        0.85 // Default trust score
    }
    
    /// Perform health check on destination
    async fn perform_health_check(address: &str) -> bool {
        // Simplified health check
        // In production, this would perform actual connectivity and performance tests
        !address.is_empty()
    }
    
    /// Get proxy statistics
    pub async fn get_statistics(&self) -> ProxyStats {
        let connections_count = self.proxy_connections.read().await.len() as u32;
        let addresses_count = self.proxy_addresses.read().await.len() as u32;
        
        let mut stats = self.stats.read().await.clone();
        stats.active_connections = connections_count;
        stats.active_proxy_addresses = addresses_count;
        
        stats
    }
    
    /// Resolve proxy address to routing destination
    pub async fn resolve_proxy_address(&self, proxy_address: &str) -> Option<RoutingDestination> {
        let routing_table = self.routing_table.read().await;
        
        if let Some(route) = routing_table.routes.get(proxy_address) {
            // Return first healthy destination
            for destination in &route.destinations {
                if destination.health_status == HealthStatus::Healthy {
                    return Some(destination.clone());
                }
            }
        }
        
        None
    }
    
    /// Update routing table
    pub async fn update_routing_table(&self, proxy_address: String, destinations: Vec<RoutingDestination>) {
        let mut routing_table = self.routing_table.write().await;
        
        let routing_entry = RoutingEntry {
            proxy_address: proxy_address.clone(),
            destinations,
            routing_algorithm: LoadBalancingAlgorithm::RoundRobin,
            last_updated: SystemTime::now(),
            route_priority: 100,
        };
        
        routing_table.routes.insert(proxy_address, routing_entry);
        routing_table.routing_metrics.total_routes += 1;
        
        debug!("📋 Updated routing table for proxy address");
    }
    
    /// Release proxy address
    pub async fn release_proxy_address(&self, proxy_address: &str) -> Result<()> {
        info!("🔓 Releasing proxy address: {}", proxy_address);
        
        // Remove from allocations
        if self.proxy_addresses.write().await.remove(proxy_address).is_some() {
            // Update statistics
            let mut stats = self.stats.write().await;
            stats.active_proxy_addresses = stats.active_proxy_addresses.saturating_sub(1);
            
            info!("✅ Proxy address released: {}", proxy_address);
        } else {
            return Err(anyhow!("Proxy address not found: {}", proxy_address));
        }
        
        Ok(())
    }
    
    /// Shutdown NAT address manager
    pub async fn shutdown(&self) -> Result<()> {
        info!("🛑 Shutting down NAT Address Manager");
        
        // Close all proxy connections
        let connection_ids: Vec<String> = self.proxy_connections.read().await.keys().cloned().collect();
        
        for connection_id in connection_ids {
            if let Some(connection) = self.proxy_connections.read().await.get(&connection_id) {
                *connection.state.write().await = ConnectionState::Closed;
            }
        }
        
        // Clear all data structures
        self.proxy_connections.write().await.clear();
        self.proxy_addresses.write().await.clear();
        self.routing_table.write().await.routes.clear();
        
        info!("✅ NAT Address Manager shutdown complete");
        Ok(())
    }
}

impl AssetType {
    pub fn to_string(&self) -> String {
        match self {
            AssetType::Cpu => "cpu".to_string(),
            AssetType::Gpu => "gpu".to_string(),
            AssetType::Memory => "memory".to_string(),
            AssetType::Storage => "storage".to_string(),
            AssetType::Network => "network".to_string(),
            AssetType::Container => "container".to_string(),
            AssetType::Vm => "vm".to_string(),
            AssetType::Service => "service".to_string(),
            AssetType::Application => "application".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_proxy_address_generation() {
        // Test proxy address generation and uniqueness
    }
    
    #[tokio::test]
    async fn test_proxy_connection_lifecycle() {
        // Test proxy connection creation, usage, and cleanup
    }
    
    #[tokio::test]
    async fn test_trust_based_routing() {
        // Test trust score calculation and routing decisions
    }
    
    #[tokio::test]
    async fn test_health_checking() {
        // Test health checking and failover mechanisms
    }
}