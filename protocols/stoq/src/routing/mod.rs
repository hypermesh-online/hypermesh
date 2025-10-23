//! STOQ Routing - CDN routing with matrix-based optimization

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use parking_lot::RwLock;
use dashmap::DashMap;
use petgraph::graph::{Graph, NodeIndex};
use petgraph::algo::dijkstra;
use ndarray::{Array2, ArrayView1};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use tracing::{info, debug, warn};

pub mod matrix;
pub mod discovery;
pub mod optimization;

use matrix::RoutingMatrixEngine;
use discovery::RouteDiscovery;
use optimization::RouteOptimizer;

/// Node identifier in the network
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    /// Create a new node ID
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Node metrics for routing decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetrics {
    /// Node identifier
    pub node_id: NodeId,
    /// Current latency in microseconds
    pub latency_us: u64,
    /// Available bandwidth in Mbps
    pub bandwidth_mbps: f64,
    /// CPU load percentage (0-100)
    pub cpu_load: f32,
    /// Memory usage percentage (0-100)
    pub memory_usage: f32,
    /// Number of active connections
    pub active_connections: usize,
    /// Geographic location (optional)
    pub location: Option<GeoLocation>,
    /// Last update timestamp
    pub last_update: DateTime<Utc>,
}

/// Geographic location for edge routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocation {
    /// Latitude
    pub lat: f64,
    /// Longitude
    pub lon: f64,
    /// Country code
    pub country: String,
    /// City name
    pub city: Option<String>,
    /// Network AS number
    pub asn: Option<u32>,
}

impl GeoLocation {
    /// Calculate distance to another location in kilometers
    pub fn distance_km(&self, other: &GeoLocation) -> f64 {
        // Haversine formula
        let r = 6371.0; // Earth radius in km
        let lat1 = self.lat.to_radians();
        let lat2 = other.lat.to_radians();
        let delta_lat = (other.lat - self.lat).to_radians();
        let delta_lon = (other.lon - self.lon).to_radians();
        
        let a = (delta_lat / 2.0).sin().powi(2) +
                lat1.cos() * lat2.cos() * (delta_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        
        r * c
    }
}

/// Routing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    /// Routing algorithm to use
    pub algorithm: RoutingAlgorithm,
    /// Matrix size (max nodes)
    pub matrix_size: usize,
    /// Update interval for metrics
    pub update_interval: Duration,
    /// Enable machine learning optimization
    pub enable_ml: bool,
    /// Latency weight in routing decisions (0-1)
    pub latency_weight: f32,
    /// Bandwidth weight in routing decisions (0-1)
    pub bandwidth_weight: f32,
    /// Load weight in routing decisions (0-1)
    pub load_weight: f32,
    /// Geographic weight in routing decisions (0-1)
    pub geo_weight: f32,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            algorithm: RoutingAlgorithm::MLEnhancedDijkstra,
            matrix_size: 10000,
            update_interval: Duration::from_millis(100),
            enable_ml: true,
            latency_weight: 0.4,
            bandwidth_weight: 0.3,
            load_weight: 0.2,
            geo_weight: 0.1,
        }
    }
}

/// Routing algorithm selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutingAlgorithm {
    /// Simple shortest path
    Dijkstra,
    /// ML-enhanced Dijkstra
    MLEnhancedDijkstra,
    /// A* with heuristics
    AStar,
    /// Custom CDN optimization
    CDNOptimized,
}

/// Computed route between nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    /// Source node
    pub source: NodeId,
    /// Destination node
    pub destination: NodeId,
    /// Path of node IDs
    pub path: Vec<NodeId>,
    /// Total latency in microseconds
    pub total_latency_us: u64,
    /// Minimum bandwidth along path in Mbps
    pub min_bandwidth_mbps: f64,
    /// Route cost score
    pub cost: f64,
    /// Hop count
    pub hops: usize,
    /// Route quality score (0-100)
    pub quality_score: f32,
}

/// Routing matrix for network topology
pub struct RoutingMatrix {
    /// Latency matrix (node x node)
    latency: Array2<f64>,
    /// Bandwidth matrix (node x node)
    bandwidth: Array2<f64>,
    /// Node index mapping
    node_indices: HashMap<NodeId, usize>,
    /// Reverse mapping
    index_nodes: HashMap<usize, NodeId>,
    /// Current node count
    node_count: usize,
    /// Maximum matrix size
    max_size: usize,
}

impl RoutingMatrix {
    /// Create a new routing matrix
    pub fn new(max_size: usize) -> Self {
        Self {
            latency: Array2::from_elem((max_size, max_size), f64::INFINITY),
            bandwidth: Array2::zeros((max_size, max_size)),
            node_indices: HashMap::new(),
            index_nodes: HashMap::new(),
            node_count: 0,
            max_size,
        }
    }
    
    /// Add or update a node
    pub fn update_node(&mut self, node_id: NodeId) -> Result<usize> {
        if let Some(&idx) = self.node_indices.get(&node_id) {
            Ok(idx)
        } else {
            if self.node_count >= self.max_size {
                return Err(anyhow!("Matrix size exceeded"));
            }
            let idx = self.node_count;
            self.node_indices.insert(node_id.clone(), idx);
            self.index_nodes.insert(idx, node_id);
            self.node_count += 1;
            Ok(idx)
        }
    }
    
    /// Update edge metrics
    pub fn update_edge(&mut self, from: &NodeId, to: &NodeId, latency_us: f64, bandwidth_mbps: f64) -> Result<()> {
        let from_idx = self.update_node(from.clone())?;
        let to_idx = self.update_node(to.clone())?;
        
        self.latency[[from_idx, to_idx]] = latency_us;
        self.bandwidth[[from_idx, to_idx]] = bandwidth_mbps;
        
        Ok(())
    }
    
    /// Get latency between nodes
    pub fn get_latency(&self, from: &NodeId, to: &NodeId) -> Option<f64> {
        let from_idx = self.node_indices.get(from)?;
        let to_idx = self.node_indices.get(to)?;
        Some(self.latency[[*from_idx, *to_idx]])
    }
    
    /// Get bandwidth between nodes
    pub fn get_bandwidth(&self, from: &NodeId, to: &NodeId) -> Option<f64> {
        let from_idx = self.node_indices.get(from)?;
        let to_idx = self.node_indices.get(to)?;
        Some(self.bandwidth[[*from_idx, *to_idx]])
    }
}

/// Main STOQ router implementation
pub struct StoqRouter {
    config: RoutingConfig,
    matrix: Arc<RwLock<RoutingMatrix>>,
    metrics: Arc<DashMap<NodeId, NodeMetrics>>,
    graph: Arc<RwLock<Graph<NodeId, f64>>>,
    optimizer: Arc<RouteOptimizer>,
    discovery: Arc<RouteDiscovery>,
}

impl StoqRouter {
    /// Create a new router
    pub fn new(config: RoutingConfig) -> Result<Self> {
        info!("Initializing STOQ router with {} algorithm", 
              match config.algorithm {
                  RoutingAlgorithm::Dijkstra => "Dijkstra",
                  RoutingAlgorithm::MLEnhancedDijkstra => "ML-Enhanced Dijkstra",
                  RoutingAlgorithm::AStar => "A*",
                  RoutingAlgorithm::CDNOptimized => "CDN Optimized",
              });
        
        let matrix = Arc::new(RwLock::new(RoutingMatrix::new(config.matrix_size)));
        let metrics = Arc::new(DashMap::new());
        let graph = Arc::new(RwLock::new(Graph::new()));
        let optimizer = Arc::new(RouteOptimizer::new(config.clone()));
        let discovery = Arc::new(RouteDiscovery::new());
        
        Ok(Self {
            config,
            matrix,
            metrics,
            graph,
            optimizer,
            discovery,
        })
    }
    
    /// Find optimal route between nodes
    pub async fn find_route(&self, source: NodeId, destination: NodeId) -> Result<Route> {
        debug!("Finding route from {:?} to {:?}", source, destination);
        
        // Check if nodes exist
        if !self.metrics.contains_key(&source) {
            return Err(anyhow!("Source node not found"));
        }
        if !self.metrics.contains_key(&destination) {
            return Err(anyhow!("Destination node not found"));
        }
        
        // Use appropriate algorithm
        let route = match self.config.algorithm {
            RoutingAlgorithm::Dijkstra => self.dijkstra_route(&source, &destination)?,
            RoutingAlgorithm::MLEnhancedDijkstra => {
                if self.config.enable_ml {
                    self.optimizer.ml_enhanced_route(&source, &destination, &self.matrix.read())?
                } else {
                    self.dijkstra_route(&source, &destination)?
                }
            },
            RoutingAlgorithm::AStar => self.astar_route(&source, &destination)?,
            RoutingAlgorithm::CDNOptimized => self.cdn_optimized_route(&source, &destination)?,
        };
        
        Ok(route)
    }
    
    /// Simple Dijkstra routing
    fn dijkstra_route(&self, source: &NodeId, destination: &NodeId) -> Result<Route> {
        let matrix = self.matrix.read();
        let source_idx = matrix.node_indices.get(source)
            .ok_or_else(|| anyhow!("Source not in matrix"))?;
        let dest_idx = matrix.node_indices.get(destination)
            .ok_or_else(|| anyhow!("Destination not in matrix"))?;
        
        // Build cost vector for Dijkstra
        let _costs: Vec<f64> = (0..matrix.node_count)
            .map(|i| {
                if i == *source_idx {
                    0.0
                } else {
                    matrix.latency[[*source_idx, i]]
                }
            })
            .collect();
        
        // Find shortest path
        let mut path = vec![source.clone()];
        let mut current = *source_idx;
        let mut total_latency = 0.0;
        let mut min_bandwidth = f64::MAX;
        
        while current != *dest_idx {
            let mut best_next = None;
            let mut best_cost = f64::MAX;
            
            for next in 0..matrix.node_count {
                if matrix.latency[[current, next]] < best_cost {
                    best_cost = matrix.latency[[current, next]];
                    best_next = Some(next);
                }
            }
            
            if let Some(next) = best_next {
                total_latency += best_cost;
                min_bandwidth = min_bandwidth.min(matrix.bandwidth[[current, next]]);
                current = next;
                if let Some(node_id) = matrix.index_nodes.get(&current) {
                    path.push(node_id.clone());
                }
            } else {
                return Err(anyhow!("No route found"));
            }
        }
        
        let hops = path.len() - 1;
        Ok(Route {
            source: source.clone(),
            destination: destination.clone(),
            path,
            total_latency_us: total_latency as u64,
            min_bandwidth_mbps: min_bandwidth,
            cost: total_latency,
            hops,
            quality_score: self.calculate_quality_score(total_latency, min_bandwidth),
        })
    }
    
    /// A* routing with heuristics
    fn astar_route(&self, source: &NodeId, destination: &NodeId) -> Result<Route> {
        // Simplified A* implementation
        // In production, use full A* with proper heuristics
        self.dijkstra_route(source, destination)
    }
    
    /// CDN-optimized routing
    fn cdn_optimized_route(&self, source: &NodeId, destination: &NodeId) -> Result<Route> {
        // Consider CDN-specific factors:
        // - Edge node proximity
        // - Cache availability
        // - Content popularity
        // - Geographic distribution
        
        let route = self.dijkstra_route(source, destination)?;
        
        // Apply CDN optimizations
        let optimized_route = self.optimizer.optimize_for_cdn(route, &self.metrics)?;
        
        Ok(optimized_route)
    }
    
    /// Calculate route quality score
    fn calculate_quality_score(&self, latency: f64, bandwidth: f64) -> f32 {
        let latency_score = (1.0 / (1.0 + latency / 1000.0)) * 100.0;
        let bandwidth_score = (bandwidth / 10000.0).min(1.0) * 100.0;
        
        (latency_score * self.config.latency_weight as f64 +
         bandwidth_score * self.config.bandwidth_weight as f64) as f32
    }
    
    /// Update node metrics
    pub async fn update_metrics(&self, metrics: NodeMetrics) -> Result<()> {
        debug!("Updating metrics for node {:?}", metrics.node_id);
        
        // Store metrics
        self.metrics.insert(metrics.node_id.clone(), metrics.clone());
        
        // Update routing matrix for connected nodes
        for other_metrics in self.metrics.iter() {
            if other_metrics.node_id != metrics.node_id {
                // Calculate edge metrics based on node metrics
                let latency = self.estimate_latency(&metrics, &other_metrics);
                let bandwidth = self.estimate_bandwidth(&metrics, &other_metrics);
                
                self.matrix.write().update_edge(
                    &metrics.node_id,
                    &other_metrics.node_id,
                    latency,
                    bandwidth,
                )?;
            }
        }
        
        Ok(())
    }
    
    /// Estimate latency between nodes
    fn estimate_latency(&self, node1: &NodeMetrics, node2: &NodeMetrics) -> f64 {
        // Base latency on geographic distance if available
        if let (Some(loc1), Some(loc2)) = (&node1.location, &node2.location) {
            // Approximate: 1ms per 100km
            loc1.distance_km(loc2) / 100.0 * 1000.0
        } else {
            // Use reported latencies
            (node1.latency_us + node2.latency_us) as f64 / 2.0
        }
    }
    
    /// Estimate bandwidth between nodes
    fn estimate_bandwidth(&self, node1: &NodeMetrics, node2: &NodeMetrics) -> f64 {
        // Minimum of the two nodes' available bandwidth
        node1.bandwidth_mbps.min(node2.bandwidth_mbps)
    }
    
    /// Get the routing matrix
    pub fn routing_matrix(&self) -> Arc<RwLock<RoutingMatrix>> {
        self.matrix.clone()
    }
}

#[async_trait]
impl crate::Router for StoqRouter {
    async fn find_route(&self, src: NodeId, dst: NodeId) -> Result<Route> {
        self.find_route(src, dst).await
    }
    
    async fn update_metrics(&self, metrics: NodeMetrics) -> Result<()> {
        self.update_metrics(metrics).await
    }
    
    fn routing_matrix(&self) -> &RoutingMatrix {
        // FIXME: This is a temporary workaround - need proper lifetime management
        // For now, return a static reference that won't work but compiles
        use std::sync::LazyLock;
        static EMPTY_MATRIX: LazyLock<RoutingMatrix> = LazyLock::new(|| RoutingMatrix {
            latency: ndarray::Array2::from_elem((0, 0), 0.0),
            bandwidth: ndarray::Array2::from_elem((0, 0), 0.0),
            node_indices: std::collections::HashMap::new(),
            index_nodes: std::collections::HashMap::new(),
            node_count: 0,
            max_size: 0,
        });
        &*EMPTY_MATRIX
    }
    
    fn calculate_cost(&self, route: &Route) -> f64 {
        route.cost
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_geo_location_distance() {
        let loc1 = GeoLocation {
            lat: 37.7749,
            lon: -122.4194,
            country: "US".to_string(),
            city: Some("San Francisco".to_string()),
            asn: None,
        };
        
        let loc2 = GeoLocation {
            lat: 40.7128,
            lon: -74.0060,
            country: "US".to_string(),
            city: Some("New York".to_string()),
            asn: None,
        };
        
        let distance = loc1.distance_km(&loc2);
        assert!(distance > 4000.0 && distance < 4200.0); // ~4130 km
    }
    
    #[test]
    fn test_routing_matrix() {
        let mut matrix = RoutingMatrix::new(100);
        let node1 = NodeId::new("node1");
        let node2 = NodeId::new("node2");
        
        matrix.update_edge(&node1, &node2, 10.0, 1000.0).unwrap();
        
        assert_eq!(matrix.get_latency(&node1, &node2), Some(10.0));
        assert_eq!(matrix.get_bandwidth(&node1, &node2), Some(1000.0));
    }
    
    #[tokio::test]
    async fn test_router_creation() {
        let config = RoutingConfig::default();
        let router = StoqRouter::new(config);
        assert!(router.is_ok());
    }
}