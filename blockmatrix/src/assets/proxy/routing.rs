//! Proxy Routing System for HyperMesh
//!
//! Implements intelligent routing for proxy traffic with load balancing,
//! trust-based selection, and performance optimization.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

use crate::assets::core::{
    AssetResult, AssetError, ProxyAddress, ProxyNodeInfo, PrivacyLevel
};

/// Type alias for routing table
pub type RouteTable = HashMap<String, Vec<ProxyRoute>>;

/// Proxy routing table with intelligent path selection
pub struct ProxyRouter {
    /// Routing table mapping destinations to routes
    routing_table: Arc<RwLock<HashMap<String, Vec<ProxyRoute>>>>,
    
    /// Node registry for route calculation
    node_registry: Arc<RwLock<HashMap<String, ProxyNodeInfo>>>,
    
    /// Route performance metrics
    route_metrics: Arc<RwLock<HashMap<String, RouteMetrics>>>,
    
    /// Routing configuration
    config: RoutingConfig,
}

/// Individual routing entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProxyRoute {
    /// Destination identifier
    pub destination: String,
    
    /// Next hop proxy node
    pub next_hop: String,
    
    /// Route cost/weight
    pub cost: u32,
    
    /// Route type
    pub route_type: RouteType,
    
    /// Trust level required
    pub min_trust_level: f32,
    
    /// Privacy level compatibility
    pub privacy_level: PrivacyLevel,
    
    /// Route capabilities
    pub capabilities: Vec<String>,
    
    /// Route status
    pub status: RouteStatus,
    
    /// Last updated timestamp
    pub last_updated: SystemTime,
}

/// Types of proxy routes
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RouteType {
    /// Direct connection to destination
    Direct,
    
    /// Single hop through proxy
    Proxy,
    
    /// Multi-hop through proxy chain
    ProxyChain,
    
    /// Load balanced across multiple proxies
    LoadBalanced,
    
    /// Encrypted tunnel
    Tunnel,
    
    /// High availability with failover
    HighAvailability,
}

/// Route status tracking
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RouteStatus {
    /// Route is active and available
    Active,
    
    /// Route is temporarily unavailable
    Unavailable,
    
    /// Route is degraded performance
    Degraded,
    
    /// Route is under maintenance
    Maintenance,
    
    /// Route has failed
    Failed,
}

/// Route performance metrics
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RouteMetrics {
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    
    /// Success rate (0.0 - 1.0)
    pub success_rate: f64,
    
    /// Throughput in Mbps
    pub throughput_mbps: f64,
    
    /// Current load (0.0 - 1.0)
    pub current_load: f64,
    
    /// Total requests routed
    pub total_requests: u64,
    
    /// Failed requests
    pub failed_requests: u64,
    
    /// Last measurement timestamp
    pub last_measured: SystemTime,
}

/// Routing configuration
#[derive(Clone, Debug)]
pub struct RoutingConfig {
    /// Maximum route cost to consider
    pub max_route_cost: u32,
    
    /// Minimum trust score for routing
    pub min_trust_score: f32,
    
    /// Load balancing algorithm
    pub load_balance_algorithm: LoadBalanceAlgorithm,
    
    /// Route refresh interval
    pub route_refresh_interval: Duration,
    
    /// Maximum hops allowed
    pub max_hops: u8,
    
    /// Enable trust-based routing
    pub trust_based_routing: bool,
    
    /// Enable performance-based routing
    pub performance_based_routing: bool,
}

/// Load balancing algorithms
#[derive(Clone, Debug)]
pub enum LoadBalanceAlgorithm {
    /// Round robin distribution
    RoundRobin,
    
    /// Weight-based distribution
    Weighted,
    
    /// Least connections
    LeastConnections,
    
    /// Least latency
    LeastLatency,
    
    /// Trust score based
    TrustBased,
    
    /// Performance based
    PerformanceBased,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            max_route_cost: 100,
            min_trust_score: 0.5,
            load_balance_algorithm: LoadBalanceAlgorithm::PerformanceBased,
            route_refresh_interval: Duration::from_secs(300), // 5 minutes
            max_hops: 3,
            trust_based_routing: true,
            performance_based_routing: true,
        }
    }
}

/// Route calculation request
#[derive(Clone, Debug)]
pub struct RouteRequest {
    /// Source node/address
    pub source: String,
    
    /// Destination address
    pub destination: String,
    
    /// Required capabilities
    pub required_capabilities: Vec<String>,
    
    /// Privacy level requirements
    pub privacy_level: PrivacyLevel,
    
    /// Performance requirements
    pub performance_requirements: PerformanceRequirements,
    
    /// Trust requirements
    pub trust_requirements: TrustRequirements,
}

/// Performance requirements for routing
#[derive(Clone, Debug)]
pub struct PerformanceRequirements {
    /// Maximum acceptable latency in milliseconds
    pub max_latency_ms: f64,
    
    /// Minimum required throughput in Mbps
    pub min_throughput_mbps: f64,
    
    /// Minimum acceptable success rate
    pub min_success_rate: f64,
    
    /// Maximum acceptable load
    pub max_load: f64,
}

/// Trust requirements for routing
#[derive(Clone, Debug)]
pub struct TrustRequirements {
    /// Minimum trust score
    pub min_trust_score: f32,
    
    /// Required certificate validation
    pub require_certificate_validation: bool,
    
    /// Require quantum security
    pub require_quantum_security: bool,
    
    /// Maximum trust chain length
    pub max_trust_chain_length: u8,
}

impl ProxyRouter {
    /// Create new proxy router
    pub async fn new() -> AssetResult<Self> {
        Ok(Self {
            routing_table: Arc::new(RwLock::new(HashMap::new())),
            node_registry: Arc::new(RwLock::new(HashMap::new())),
            route_metrics: Arc::new(RwLock::new(HashMap::new())),
            config: RoutingConfig::default(),
        })
    }
    
    /// Add proxy node to routing registry
    pub async fn add_proxy_node(&self, node_info: &ProxyNodeInfo) -> AssetResult<()> {
        let mut registry = self.node_registry.write().await;
        registry.insert(node_info.node_id.clone(), node_info.clone());
        
        // Recalculate routes that might involve this node
        self.recalculate_routes().await?;
        
        tracing::info!("Added proxy node to routing registry: {}", node_info.node_id);
        Ok(())
    }
    
    /// Calculate best route for request
    pub async fn calculate_route(&self, request: &RouteRequest) -> AssetResult<ProxyRoute> {
        // Get available routes for destination
        let routes = {
            let table = self.routing_table.read().await;
            table.get(&request.destination)
                .cloned()
                .unwrap_or_default()
        };
        
        if routes.is_empty() {
            return Err(AssetError::AdapterError {
                message: format!("No routes available for destination: {}", request.destination)
            });
        }
        
        // Filter routes based on requirements
        let suitable_routes = self.filter_routes(&routes, request).await?;
        
        if suitable_routes.is_empty() {
            return Err(AssetError::AdapterError {
                message: format!("No suitable routes for destination: {}", request.destination)
            });
        }
        
        // Select best route based on algorithm
        let best_route = self.select_best_route(&suitable_routes, request).await?;
        
        tracing::debug!(
            "Selected route for {}: {} via {} (cost: {})",
            request.destination,
            best_route.route_type,
            best_route.next_hop,
            best_route.cost
        );
        
        Ok(best_route)
    }
    
    /// Filter routes based on requirements
    async fn filter_routes(
        &self,
        routes: &[ProxyRoute],
        request: &RouteRequest,
    ) -> AssetResult<Vec<ProxyRoute>> {
        let metrics = self.route_metrics.read().await;
        let registry = self.node_registry.read().await;
        
        let mut suitable_routes = Vec::new();
        
        for route in routes {
            // Check route status
            if !matches!(route.status, RouteStatus::Active) {
                continue;
            }
            
            // Check trust level
            if route.min_trust_level < request.trust_requirements.min_trust_score {
                continue;
            }
            
            // Check privacy level compatibility
            if !self.privacy_levels_compatible(&route.privacy_level, &request.privacy_level) {
                continue;
            }
            
            // Check capabilities
            let has_required_caps = request.required_capabilities.iter()
                .all(|cap| route.capabilities.contains(cap));
            if !has_required_caps {
                continue;
            }
            
            // Check performance requirements
            if let Some(route_metrics) = metrics.get(&route.destination) {
                if route_metrics.avg_latency_ms > request.performance_requirements.max_latency_ms ||
                   route_metrics.throughput_mbps < request.performance_requirements.min_throughput_mbps ||
                   route_metrics.success_rate < request.performance_requirements.min_success_rate ||
                   route_metrics.current_load > request.performance_requirements.max_load {
                    continue;
                }
            }
            
            // Check node availability
            if let Some(node_info) = registry.get(&route.next_hop) {
                if node_info.trust_score < request.trust_requirements.min_trust_score {
                    continue;
                }
            }
            
            suitable_routes.push(route.clone());
        }
        
        Ok(suitable_routes)
    }
    
    /// Select best route based on configured algorithm
    async fn select_best_route(
        &self,
        routes: &[ProxyRoute],
        request: &RouteRequest,
    ) -> AssetResult<ProxyRoute> {
        let metrics = self.route_metrics.read().await;
        let registry = self.node_registry.read().await;
        
        match self.config.load_balance_algorithm {
            LoadBalanceAlgorithm::PerformanceBased => {
                self.select_performance_based_route(routes, &metrics).await
            },
            LoadBalanceAlgorithm::TrustBased => {
                self.select_trust_based_route(routes, &registry).await
            },
            LoadBalanceAlgorithm::LeastLatency => {
                self.select_least_latency_route(routes, &metrics).await
            },
            LoadBalanceAlgorithm::Weighted => {
                self.select_weighted_route(routes, &metrics, &registry).await
            },
            LoadBalanceAlgorithm::LeastConnections => {
                self.select_least_connections_route(routes, &metrics).await
            },
            LoadBalanceAlgorithm::RoundRobin => {
                self.select_round_robin_route(routes).await
            },
        }
    }
    
    /// Select route based on performance metrics
    async fn select_performance_based_route(
        &self,
        routes: &[ProxyRoute],
        metrics: &HashMap<String, RouteMetrics>,
    ) -> AssetResult<ProxyRoute> {
        let mut best_route = None;
        let mut best_score = 0.0;
        
        for route in routes {
            if let Some(route_metrics) = metrics.get(&route.destination) {
                // Calculate composite performance score
                let latency_score = 1.0 / (1.0 + route_metrics.avg_latency_ms / 100.0);
                let throughput_score = route_metrics.throughput_mbps / 1000.0; // Normalize to Gbps
                let success_score = route_metrics.success_rate;
                let load_score = 1.0 - route_metrics.current_load;
                
                let composite_score = (latency_score * 0.3 + 
                                     throughput_score * 0.3 + 
                                     success_score * 0.25 + 
                                     load_score * 0.15).min(1.0);
                
                if composite_score > best_score {
                    best_score = composite_score;
                    best_route = Some(route.clone());
                }
            }
        }
        
        best_route.ok_or_else(|| AssetError::AdapterError {
            message: "No route found with performance metrics".to_string()
        })
    }
    
    /// Select route based on trust scores
    async fn select_trust_based_route(
        &self,
        routes: &[ProxyRoute],
        registry: &HashMap<String, ProxyNodeInfo>,
    ) -> AssetResult<ProxyRoute> {
        let mut best_route = None;
        let mut best_trust_score = 0.0;
        
        for route in routes {
            if let Some(node_info) = registry.get(&route.next_hop) {
                if node_info.trust_score > best_trust_score {
                    best_trust_score = node_info.trust_score;
                    best_route = Some(route.clone());
                }
            }
        }
        
        best_route.ok_or_else(|| AssetError::AdapterError {
            message: "No route found with trust information".to_string()
        })
    }
    
    /// Select route with least latency
    async fn select_least_latency_route(
        &self,
        routes: &[ProxyRoute],
        metrics: &HashMap<String, RouteMetrics>,
    ) -> AssetResult<ProxyRoute> {
        let mut best_route = None;
        let mut best_latency = f64::INFINITY;
        
        for route in routes {
            if let Some(route_metrics) = metrics.get(&route.destination) {
                if route_metrics.avg_latency_ms < best_latency {
                    best_latency = route_metrics.avg_latency_ms;
                    best_route = Some(route.clone());
                }
            }
        }
        
        best_route.ok_or_else(|| AssetError::AdapterError {
            message: "No route found with latency metrics".to_string()
        })
    }
    
    /// Select weighted route based on multiple factors
    async fn select_weighted_route(
        &self,
        routes: &[ProxyRoute],
        metrics: &HashMap<String, RouteMetrics>,
        registry: &HashMap<String, ProxyNodeInfo>,
    ) -> AssetResult<ProxyRoute> {
        let mut best_route = None;
        let mut best_weight = 0.0;
        
        for route in routes {
            let mut weight = 0.0;
            
            // Route cost (lower is better)
            weight += (100.0 - route.cost as f64) / 100.0 * 0.2;
            
            // Trust score
            if let Some(node_info) = registry.get(&route.next_hop) {
                weight += node_info.trust_score as f64 * 0.3;
            }
            
            // Performance metrics
            if let Some(route_metrics) = metrics.get(&route.destination) {
                let latency_factor = 1.0 / (1.0 + route_metrics.avg_latency_ms / 100.0);
                let success_factor = route_metrics.success_rate;
                let load_factor = 1.0 - route_metrics.current_load;
                
                weight += (latency_factor * 0.2 + success_factor * 0.2 + load_factor * 0.1);
            }
            
            if weight > best_weight {
                best_weight = weight;
                best_route = Some(route.clone());
            }
        }
        
        best_route.ok_or_else(|| AssetError::AdapterError {
            message: "No suitable weighted route found".to_string()
        })
    }
    
    /// Select route with least connections
    async fn select_least_connections_route(
        &self,
        routes: &[ProxyRoute],
        metrics: &HashMap<String, RouteMetrics>,
    ) -> AssetResult<ProxyRoute> {
        let mut best_route = None;
        let mut best_load = f64::INFINITY;
        
        for route in routes {
            if let Some(route_metrics) = metrics.get(&route.destination) {
                if route_metrics.current_load < best_load {
                    best_load = route_metrics.current_load;
                    best_route = Some(route.clone());
                }
            }
        }
        
        best_route.ok_or_else(|| AssetError::AdapterError {
            message: "No route found with load metrics".to_string()
        })
    }
    
    /// Select route using round robin algorithm
    async fn select_round_robin_route(&self, routes: &[ProxyRoute]) -> AssetResult<ProxyRoute> {
        // TODO: Implement stateful round robin tracking
        // For now, return first active route
        routes.first()
            .cloned()
            .ok_or_else(|| AssetError::AdapterError {
                message: "No routes available for round robin".to_string()
            })
    }
    
    /// Check if privacy levels are compatible
    fn privacy_levels_compatible(&self, route_privacy: &PrivacyLevel, request_privacy: &PrivacyLevel) -> bool {
        use PrivacyLevel::*;
        
        match (route_privacy, request_privacy) {
            (Private, Private) => true,
            (PrivateNetwork, Private | PrivateNetwork) => true,
            (P2P, Private | PrivateNetwork | P2P) => true,
            (PublicNetwork, Private | PrivateNetwork | P2P | PublicNetwork) => true,
            (FullPublic, _) => true,
            _ => false,
        }
    }
    
    /// Recalculate all routes (called when network topology changes)
    async fn recalculate_routes(&self) -> AssetResult<()> {
        // TODO: Implement intelligent route recalculation
        // This would analyze network topology and update routing table
        tracing::debug!("Recalculating proxy routes");
        Ok(())
    }
    
    /// Update route metrics
    pub async fn update_route_metrics(&self, destination: &str, metrics: RouteMetrics) -> AssetResult<()> {
        let mut route_metrics = self.route_metrics.write().await;
        route_metrics.insert(destination.to_string(), metrics);
        Ok(())
    }
    
    /// Get route statistics
    pub async fn get_route_stats(&self) -> AssetResult<HashMap<String, RouteMetrics>> {
        let metrics = self.route_metrics.read().await;
        Ok(metrics.clone())
    }
}

impl std::fmt::Display for RouteType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RouteType::Direct => write!(f, "Direct"),
            RouteType::Proxy => write!(f, "Proxy"),
            RouteType::ProxyChain => write!(f, "ProxyChain"),
            RouteType::LoadBalanced => write!(f, "LoadBalanced"),
            RouteType::Tunnel => write!(f, "Tunnel"),
            RouteType::HighAvailability => write!(f, "HighAvailability"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::core::{ProxyCapabilities};
    
    async fn create_test_router() -> ProxyRouter {
        ProxyRouter::new().await.unwrap()
    }
    
    fn create_test_node_info(node_id: &str, trust_score: f32) -> ProxyNodeInfo {
        ProxyNodeInfo {
            node_id: node_id.to_string(),
            network_address: format!("192.168.1.{}", 100),
            capabilities: ProxyCapabilities {
                http_proxy: true,
                socks5_proxy: true,
                tcp_forwarding: true,
                vpn_tunnel: false,
                max_connections: 1000,
                bandwidth_mbps: 1000,
                protocols: vec!["HTTP".to_string(), "SOCKS5".to_string()],
            },
            trust_score,
            last_heartbeat: SystemTime::now(),
            certificate_fingerprint: format!("{}-cert", node_id),
        }
    }
    
    #[tokio::test]
    async fn test_router_creation() {
        let router = create_test_router().await;
        assert!(router.routing_table.read().await.is_empty());
    }
    
    #[tokio::test]
    async fn test_add_proxy_node() {
        let router = create_test_router().await;
        let node_info = create_test_node_info("test-node-1", 0.8);
        
        router.add_proxy_node(&node_info).await.unwrap();
        
        let registry = router.node_registry.read().await;
        assert!(registry.contains_key("test-node-1"));
        assert_eq!(registry["test-node-1"].trust_score, 0.8);
    }
    
    #[tokio::test]
    async fn test_route_privacy_compatibility() {
        let router = create_test_router().await;
        
        // FullPublic should be compatible with everything
        assert!(router.privacy_levels_compatible(&PrivacyLevel::FullPublic, &PrivacyLevel::Private));
        assert!(router.privacy_levels_compatible(&PrivacyLevel::FullPublic, &PrivacyLevel::FullPublic));
        
        // Private should only be compatible with Private
        assert!(router.privacy_levels_compatible(&PrivacyLevel::Private, &PrivacyLevel::Private));
        assert!(!router.privacy_levels_compatible(&PrivacyLevel::Private, &PrivacyLevel::P2P));
        
        // P2P should be compatible with Private, PrivateNetwork, P2P
        assert!(router.privacy_levels_compatible(&PrivacyLevel::P2P, &PrivacyLevel::Private));
        assert!(router.privacy_levels_compatible(&PrivacyLevel::P2P, &PrivacyLevel::P2P));
        assert!(!router.privacy_levels_compatible(&PrivacyLevel::P2P, &PrivacyLevel::PublicNetwork));
    }
}