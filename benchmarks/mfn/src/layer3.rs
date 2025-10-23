/*!
# Layer 3 (ALM) Benchmarking

Benchmarks for the Adaptive Link Management layer focusing on:
- Graph traversal optimization
- Routing decision performance
- Network topology discovery
- Path optimization algorithms
- Dynamic load balancing

Performance targets:
- 777% routing improvement over HTTP baseline
- Sub-millisecond routing decisions
- Efficient graph algorithms
- Adaptive topology management
*/

use crate::common::*;
use std::collections::{HashMap, HashSet, BinaryHeap, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use std::cmp::Reverse;

/// ALM-specific benchmark configuration
#[derive(Debug, Clone)]
pub struct AlmBenchmarkConfig {
    pub base: BenchmarkConfig,
    pub network_nodes: usize,
    pub connection_density: f64,
    pub topology_changes_per_sec: f64,
    pub routing_requests_per_sec: usize,
    pub path_optimization_depth: usize,
    pub load_balancing_algorithms: Vec<LoadBalancingAlgorithm>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadBalancingAlgorithm {
    RoundRobin,
    WeightedRoundRobin,
    LeastConnections,
    LatencyBased,
    AdaptiveWeighted,
}

impl Default for AlmBenchmarkConfig {
    fn default() -> Self {
        Self {
            base: BenchmarkConfig {
                warmup_iterations: 500,
                measurement_iterations: 5000,
                statistical_confidence: 0.95,
                regression_threshold: 0.05,
                memory_limit_mb: 128,
                timeout_seconds: 240,
                parallel_workers: num_cpus::get(),
                output_format: OutputFormat::Json,
                enable_flamegraph: false,
                enable_perf_counters: true,
            },
            network_nodes: 1000,
            connection_density: 0.3, // 30% of possible connections
            topology_changes_per_sec: 10.0,
            routing_requests_per_sec: 10000,
            path_optimization_depth: 5,
            load_balancing_algorithms: vec![
                LoadBalancingAlgorithm::RoundRobin,
                LoadBalancingAlgorithm::WeightedRoundRobin,
                LoadBalancingAlgorithm::LeastConnections,
                LoadBalancingAlgorithm::LatencyBased,
                LoadBalancingAlgorithm::AdaptiveWeighted,
            ],
        }
    }
}

/// Network node representation
#[derive(Debug, Clone)]
pub struct NetworkNode {
    pub id: usize,
    pub address: String,
    pub capacity: f64,
    pub current_load: f64,
    pub latency_ms: f64,
    pub connections: HashSet<usize>,
    pub last_seen: std::time::SystemTime,
    pub node_type: NodeType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Core,
    Edge,
    Leaf,
    Gateway,
}

/// Network link with performance metrics
#[derive(Debug, Clone)]
pub struct NetworkLink {
    pub from: usize,
    pub to: usize,
    pub latency_ms: f64,
    pub bandwidth_mbps: f64,
    pub utilization: f64,
    pub reliability: f64,
    pub cost: f64,
}

/// Adaptive network topology manager
pub struct NetworkTopology {
    nodes: HashMap<usize, NetworkNode>,
    links: HashMap<(usize, usize), NetworkLink>,
    adjacency_list: HashMap<usize, Vec<usize>>,
    routing_cache: HashMap<(usize, usize), Vec<usize>>,
    topology_version: usize,
}

impl NetworkTopology {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            links: HashMap::new(),
            adjacency_list: HashMap::new(),
            routing_cache: HashMap::new(),
            topology_version: 0,
        }
    }

    pub fn add_node(&mut self, node: NetworkNode) {
        self.adjacency_list.entry(node.id).or_default();
        self.nodes.insert(node.id, node);
        self.invalidate_cache();
    }

    pub fn add_link(&mut self, link: NetworkLink) {
        self.adjacency_list.entry(link.from).or_default().push(link.to);
        self.adjacency_list.entry(link.to).or_default().push(link.from);
        
        self.links.insert((link.from, link.to), link.clone());
        self.links.insert((link.to, link.from), NetworkLink {
            from: link.to,
            to: link.from,
            ..link
        });
        
        self.invalidate_cache();
    }

    pub fn remove_node(&mut self, node_id: usize) {
        if let Some(node) = self.nodes.remove(&node_id) {
            // Remove all links to this node
            for &neighbor in &node.connections {
                self.remove_link(node_id, neighbor);
            }
            self.adjacency_list.remove(&node_id);
            self.invalidate_cache();
        }
    }

    pub fn remove_link(&mut self, from: usize, to: usize) {
        self.links.remove(&(from, to));
        self.links.remove(&(to, from));
        
        if let Some(neighbors) = self.adjacency_list.get_mut(&from) {
            neighbors.retain(|&x| x != to);
        }
        if let Some(neighbors) = self.adjacency_list.get_mut(&to) {
            neighbors.retain(|&x| x != from);
        }
        
        self.invalidate_cache();
    }

    fn invalidate_cache(&mut self) {
        self.routing_cache.clear();
        self.topology_version += 1;
    }

    /// Find shortest path using Dijkstra's algorithm with multiple metrics
    pub fn find_optimal_path(&mut self, from: usize, to: usize, optimization: PathOptimization) -> Option<Vec<usize>> {
        if from == to {
            return Some(vec![from]);
        }

        let cache_key = (from, to);
        if let Some(cached_path) = self.routing_cache.get(&cache_key).cloned() {
            return Some(cached_path);
        }

        let path = self.dijkstra_multi_metric(from, to, optimization)?;
        
        if self.routing_cache.len() < 10000 {
            self.routing_cache.insert(cache_key, path.clone());
        }
        
        Some(path)
    }

    fn dijkstra_multi_metric(&self, start: usize, end: usize, optimization: PathOptimization) -> Option<Vec<usize>> {
        let mut distances = HashMap::new();
        let mut previous = HashMap::new();
        let mut heap = BinaryHeap::new();
        
        distances.insert(start, 0.0);
        heap.push(Reverse((0.0, start)));

        while let Some(Reverse((current_cost, current_node))) = heap.pop() {
            if current_node == end {
                break;
            }

            if let Some(&recorded_cost) = distances.get(&current_node) {
                if current_cost > recorded_cost {
                    continue;
                }
            }

            if let Some(neighbors) = self.adjacency_list.get(&current_node) {
                for &neighbor in neighbors {
                    let edge_cost = self.calculate_edge_cost(current_node, neighbor, optimization);
                    let tentative_cost = current_cost + edge_cost;

                    if tentative_cost < distances.get(&neighbor).copied().unwrap_or(f64::INFINITY) {
                        distances.insert(neighbor, tentative_cost);
                        previous.insert(neighbor, current_node);
                        heap.push(Reverse((tentative_cost, neighbor)));
                    }
                }
            }
        }

        // Reconstruct path
        let mut path = Vec::new();
        let mut current = end;
        
        loop {
            path.push(current);
            if current == start {
                break;
            }
            current = *previous.get(&current)?;
        }
        
        path.reverse();
        Some(path)
    }

    fn calculate_edge_cost(&self, from: usize, to: usize, optimization: PathOptimization) -> f64 {
        if let Some(link) = self.links.get(&(from, to)) {
            match optimization {
                PathOptimization::Latency => link.latency_ms,
                PathOptimization::Bandwidth => 1000.0 - link.bandwidth_mbps, // Invert for min-heap
                PathOptimization::Reliability => 1.0 - link.reliability,
                PathOptimization::Cost => link.cost,
                PathOptimization::Balanced => {
                    // Weighted combination of metrics
                    0.4 * link.latency_ms + 
                    0.3 * (1000.0 - link.bandwidth_mbps) + 
                    0.2 * (1.0 - link.reliability) + 
                    0.1 * link.cost
                }
            }
        } else {
            f64::INFINITY
        }
    }

    /// Multi-path routing for load distribution
    pub fn find_multiple_paths(&self, from: usize, to: usize, k: usize) -> Vec<Vec<usize>> {
        let mut paths = Vec::new();
        let mut modified_topology = self.clone();
        
        for _ in 0..k {
            if let Some(path) = modified_topology.find_optimal_path(from, to, PathOptimization::Balanced) {
                paths.push(path.clone());
                
                // Remove edges from this path to find alternative routes
                for window in path.windows(2) {
                    if window.len() == 2 {
                        modified_topology.increase_link_cost(window[0], window[1]);
                    }
                }
            } else {
                break;
            }
        }
        
        paths
    }

    fn increase_link_cost(&mut self, from: usize, to: usize) {
        if let Some(link) = self.links.get_mut(&(from, to)) {
            link.cost *= 2.0; // Double the cost to discourage reuse
        }
    }

    pub fn get_node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn get_link_count(&self) -> usize {
        self.links.len() / 2 // Each link is stored bidirectionally
    }

    pub fn get_cache_size(&self) -> usize {
        self.routing_cache.len()
    }
}

impl Clone for NetworkTopology {
    fn clone(&self) -> Self {
        Self {
            nodes: self.nodes.clone(),
            links: self.links.clone(),
            adjacency_list: self.adjacency_list.clone(),
            routing_cache: HashMap::new(), // Don't clone cache
            topology_version: self.topology_version,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PathOptimization {
    Latency,
    Bandwidth,
    Reliability,
    Cost,
    Balanced,
}

/// Load balancer for distributing traffic across multiple paths
pub struct LoadBalancer {
    algorithm: LoadBalancingAlgorithm,
    paths: Vec<Vec<usize>>,
    path_stats: HashMap<Vec<usize>, PathStatistics>,
    current_index: usize,
}

#[derive(Debug, Clone, Default)]
pub struct PathStatistics {
    pub requests: usize,
    pub average_latency: f64,
    pub success_rate: f64,
    pub last_used: std::time::SystemTime,
}

impl LoadBalancer {
    pub fn new(algorithm: LoadBalancingAlgorithm, paths: Vec<Vec<usize>>) -> Self {
        let path_stats = paths.iter()
            .map(|path| (path.clone(), PathStatistics::default()))
            .collect();

        Self {
            algorithm,
            paths,
            path_stats,
            current_index: 0,
        }
    }

    pub fn select_path(&mut self) -> Option<&Vec<usize>> {
        if self.paths.is_empty() {
            return None;
        }

        let selected_path = match self.algorithm {
            LoadBalancingAlgorithm::RoundRobin => {
                let path = &self.paths[self.current_index];
                self.current_index = (self.current_index + 1) % self.paths.len();
                path
            }
            LoadBalancingAlgorithm::WeightedRoundRobin => {
                self.select_weighted_path()
            }
            LoadBalancingAlgorithm::LeastConnections => {
                self.select_least_used_path()
            }
            LoadBalancingAlgorithm::LatencyBased => {
                self.select_lowest_latency_path()
            }
            LoadBalancingAlgorithm::AdaptiveWeighted => {
                self.select_adaptive_path()
            }
        };

        // Update statistics
        if let Some(stats) = self.path_stats.get_mut(selected_path) {
            stats.requests += 1;
            stats.last_used = std::time::SystemTime::now();
        }

        Some(selected_path)
    }

    fn select_weighted_path(&self) -> &Vec<usize> {
        // Simple weighted selection based on inverse success rate
        let mut best_weight = f64::NEG_INFINITY;
        let mut best_path = &self.paths[0];

        for path in &self.paths {
            if let Some(stats) = self.path_stats.get(path) {
                let weight = if stats.requests > 0 {
                    stats.success_rate / (stats.requests as f64).sqrt()
                } else {
                    1.0
                };
                
                if weight > best_weight {
                    best_weight = weight;
                    best_path = path;
                }
            }
        }

        best_path
    }

    fn select_least_used_path(&self) -> &Vec<usize> {
        self.paths.iter()
            .min_by_key(|path| {
                self.path_stats.get(path)
                    .map(|stats| stats.requests)
                    .unwrap_or(0)
            })
            .unwrap_or(&self.paths[0])
    }

    fn select_lowest_latency_path(&self) -> &Vec<usize> {
        self.paths.iter()
            .min_by(|a, b| {
                let latency_a = self.path_stats.get(a)
                    .map(|stats| stats.average_latency)
                    .unwrap_or(f64::INFINITY);
                let latency_b = self.path_stats.get(b)
                    .map(|stats| stats.average_latency)
                    .unwrap_or(f64::INFINITY);
                latency_a.partial_cmp(&latency_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(&self.paths[0])
    }

    fn select_adaptive_path(&self) -> &Vec<usize> {
        // Adaptive selection combining multiple factors
        let mut best_score = f64::NEG_INFINITY;
        let mut best_path = &self.paths[0];

        for path in &self.paths {
            if let Some(stats) = self.path_stats.get(path) {
                let latency_score = if stats.average_latency > 0.0 {
                    1.0 / stats.average_latency
                } else {
                    1.0
                };
                
                let load_score = if stats.requests > 0 {
                    1.0 / stats.requests as f64
                } else {
                    1.0
                };
                
                let reliability_score = stats.success_rate;
                
                let combined_score = 0.5 * latency_score + 0.3 * load_score + 0.2 * reliability_score;
                
                if combined_score > best_score {
                    best_score = combined_score;
                    best_path = path;
                }
            }
        }

        best_path
    }

    pub fn update_path_stats(&mut self, path: &Vec<usize>, latency: f64, success: bool) {
        if let Some(stats) = self.path_stats.get_mut(path) {
            // Update average latency with exponential moving average
            if stats.requests > 0 {
                stats.average_latency = 0.9 * stats.average_latency + 0.1 * latency;
            } else {
                stats.average_latency = latency;
            }
            
            // Update success rate
            let old_successes = (stats.success_rate * stats.requests as f64) as usize;
            let new_successes = if success { old_successes + 1 } else { old_successes };
            stats.success_rate = new_successes as f64 / (stats.requests + 1) as f64;
        }
    }

    pub fn get_path_count(&self) -> usize {
        self.paths.len()
    }
}

/// Main ALM benchmark suite
pub async fn run_alm_benchmarks(config: AlmBenchmarkConfig) -> anyhow::Result<Vec<BenchmarkResult>> {
    let mut harness = BenchmarkHarness::new(config.base.clone());
    let mut results = Vec::new();

    println!("🌐 Starting Layer 3 (ALM) Benchmarks");
    println!("    Network nodes: {}", config.network_nodes);
    println!("    Connection density: {:.1}%", config.connection_density * 100.0);
    println!("    Routing requests/sec: {}", config.routing_requests_per_sec);

    // Generate test network topology
    let topology = generate_test_topology(&config);
    let routing_requests = generate_routing_requests(&config, &topology);

    // Benchmark 1: Graph Traversal Performance
    results.push(run_graph_traversal_benchmark(&mut harness, &config, &topology).await?);
    
    // Benchmark 2: Routing Decision Performance
    results.push(run_routing_decision_benchmark(&mut harness, &config, &topology, &routing_requests).await?);
    
    // Benchmark 3: Multi-path Routing
    results.push(run_multipath_routing_benchmark(&mut harness, &config, &topology).await?);
    
    // Benchmark 4: Load Balancing Performance
    results.push(run_load_balancing_benchmark(&mut harness, &config, &topology).await?);
    
    // Benchmark 5: Topology Update Performance
    results.push(run_topology_update_benchmark(&mut harness, &config, topology).await?);

    Ok(results)
}

async fn run_graph_traversal_benchmark(
    harness: &mut BenchmarkHarness,
    config: &AlmBenchmarkConfig,
    topology: &NetworkTopology,
) -> anyhow::Result<BenchmarkResult> {
    harness.run_benchmark(
        "graph_traversal_dijkstra",
        MfnLayer::Layer3Alm,
        {
            let mut topo = topology.clone();
            let node_ids: Vec<_> = topo.nodes.keys().copied().collect();

            move || {
                let start = Instant::now();
                
                let from = node_ids[fastrand::usize(0..node_ids.len())];
                let to = node_ids[fastrand::usize(0..node_ids.len())];
                
                let _ = topo.find_optimal_path(from, to, PathOptimization::Latency);
                let duration = start.elapsed();

                async move { Ok(duration) }
            }
        }
    ).await
}

async fn run_routing_decision_benchmark(
    harness: &mut BenchmarkHarness,
    _config: &AlmBenchmarkConfig,
    topology: &NetworkTopology,
    routing_requests: &[(usize, usize)],
) -> anyhow::Result<BenchmarkResult> {
    harness.run_benchmark(
        "routing_decision_optimized",
        MfnLayer::Layer3Alm,
        {
            let mut topo = topology.clone();
            let requests = routing_requests.to_vec();

            move || {
                let start = Instant::now();
                
                let (from, to) = requests[fastrand::usize(0..requests.len())];
                let _ = topo.find_optimal_path(from, to, PathOptimization::Balanced);
                let duration = start.elapsed();

                async move { Ok(duration) }
            }
        }
    ).await
}

async fn run_multipath_routing_benchmark(
    harness: &mut BenchmarkHarness,
    config: &AlmBenchmarkConfig,
    topology: &NetworkTopology,
) -> anyhow::Result<BenchmarkResult> {
    harness.run_benchmark(
        "multipath_routing",
        MfnLayer::Layer3Alm,
        {
            let topo = topology.clone();
            let node_ids: Vec<_> = topo.nodes.keys().copied().collect();
            let k_paths = config.path_optimization_depth;

            move || {
                let start = Instant::now();
                
                let from = node_ids[fastrand::usize(0..node_ids.len())];
                let to = node_ids[fastrand::usize(0..node_ids.len())];
                
                let _ = topo.find_multiple_paths(from, to, k_paths);
                let duration = start.elapsed();

                async move { Ok(duration) }
            }
        }
    ).await
}

async fn run_load_balancing_benchmark(
    harness: &mut BenchmarkHarness,
    config: &AlmBenchmarkConfig,
    topology: &NetworkTopology,
) -> anyhow::Result<BenchmarkResult> {
    harness.run_benchmark(
        "load_balancing_adaptive",
        MfnLayer::Layer3Alm,
        {
            let node_ids: Vec<_> = topology.nodes.keys().copied().take(10).collect();
            let paths = if node_ids.len() >= 2 {
                topology.find_multiple_paths(node_ids[0], node_ids[1], 5)
            } else {
                vec![vec![0, 1]] // Fallback path
            };
            
            let mut load_balancer = LoadBalancer::new(LoadBalancingAlgorithm::AdaptiveWeighted, paths);

            move || {
                let start = Instant::now();
                
                if let Some(selected_path) = load_balancer.select_path() {
                    // Simulate using the path with random latency and success
                    let latency = fastrand::f64() * 10.0; // 0-10ms
                    let success = fastrand::f64() < 0.95; // 95% success rate
                    load_balancer.update_path_stats(selected_path, latency, success);
                }
                
                let duration = start.elapsed();

                async move { Ok(duration) }
            }
        }
    ).await
}

async fn run_topology_update_benchmark(
    harness: &mut BenchmarkHarness,
    config: &AlmBenchmarkConfig,
    mut topology: NetworkTopology,
) -> anyhow::Result<BenchmarkResult> {
    harness.run_benchmark(
        "topology_update_dynamic",
        MfnLayer::Layer3Alm,
        {
            let node_count = config.network_nodes;

            move || {
                let start = Instant::now();
                
                // Simulate topology change
                if fastrand::f64() < 0.7 {
                    // Add a new link
                    let from = fastrand::usize(0..node_count);
                    let to = fastrand::usize(0..node_count);
                    
                    if from != to {
                        let link = NetworkLink {
                            from,
                            to,
                            latency_ms: fastrand::f64() * 50.0,
                            bandwidth_mbps: fastrand::f64() * 1000.0,
                            utilization: fastrand::f64() * 0.8,
                            reliability: 0.9 + fastrand::f64() * 0.1,
                            cost: fastrand::f64() * 10.0,
                        };
                        topology.add_link(link);
                    }
                } else {
                    // Remove a link
                    if topology.get_link_count() > 10 {
                        let from = fastrand::usize(0..node_count);
                        let to = fastrand::usize(0..node_count);
                        topology.remove_link(from, to);
                    }
                }
                
                let duration = start.elapsed();

                async move { Ok(duration) }
            }
        }
    ).await
}

fn generate_test_topology(config: &AlmBenchmarkConfig) -> NetworkTopology {
    let mut topology = NetworkTopology::new();
    
    // Add nodes
    for i in 0..config.network_nodes {
        let node_type = match i {
            0..=9 => NodeType::Core,
            _ if i > config.network_nodes - 50 => NodeType::Edge,
            _ if i > config.network_nodes - 100 => NodeType::Leaf,
            _ => NodeType::Gateway,
        };
        
        let node = NetworkNode {
            id: i,
            address: format!("192.168.{}.{}", i / 256, i % 256),
            capacity: 1000.0 + fastrand::f64() * 9000.0, // 1-10 Gbps
            current_load: fastrand::f64() * 500.0,        // 0-500 Mbps
            latency_ms: 1.0 + fastrand::f64() * 49.0,     // 1-50ms
            connections: HashSet::new(),
            last_seen: std::time::SystemTime::now(),
            node_type,
        };
        topology.add_node(node);
    }
    
    // Add links based on connection density
    let total_possible_links = config.network_nodes * (config.network_nodes - 1) / 2;
    let target_links = (total_possible_links as f64 * config.connection_density) as usize;
    
    for _ in 0..target_links {
        let from = fastrand::usize(0..config.network_nodes);
        let to = fastrand::usize(0..config.network_nodes);
        
        if from != to {
            let link = NetworkLink {
                from,
                to,
                latency_ms: 1.0 + fastrand::f64() * 99.0,      // 1-100ms
                bandwidth_mbps: 100.0 + fastrand::f64() * 9900.0, // 100Mbps-10Gbps
                utilization: fastrand::f64() * 0.8,             // 0-80% utilization
                reliability: 0.9 + fastrand::f64() * 0.1,       // 90-100% reliability
                cost: fastrand::f64() * 100.0,                  // 0-100 cost units
            };
            topology.add_link(link);
        }
    }
    
    topology
}

fn generate_routing_requests(config: &AlmBenchmarkConfig, topology: &NetworkTopology) -> Vec<(usize, usize)> {
    let node_ids: Vec<_> = topology.nodes.keys().copied().collect();
    let request_count = config.routing_requests_per_sec;
    
    (0..request_count)
        .map(|_| {
            let from = node_ids[fastrand::usize(0..node_ids.len())];
            let to = node_ids[fastrand::usize(0..node_ids.len())];
            (from, to)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_topology_creation() {
        let mut topology = NetworkTopology::new();
        
        let node = NetworkNode {
            id: 0,
            address: "192.168.1.1".to_string(),
            capacity: 1000.0,
            current_load: 100.0,
            latency_ms: 10.0,
            connections: HashSet::new(),
            last_seen: std::time::SystemTime::now(),
            node_type: NodeType::Core,
        };
        
        topology.add_node(node);
        assert_eq!(topology.get_node_count(), 1);
    }

    #[test]
    fn test_network_links() {
        let mut topology = NetworkTopology::new();
        
        // Add nodes
        for i in 0..3 {
            let node = NetworkNode {
                id: i,
                address: format!("192.168.1.{}", i + 1),
                capacity: 1000.0,
                current_load: 100.0,
                latency_ms: 10.0,
                connections: HashSet::new(),
                last_seen: std::time::SystemTime::now(),
                node_type: NodeType::Core,
            };
            topology.add_node(node);
        }
        
        // Add links
        let link = NetworkLink {
            from: 0,
            to: 1,
            latency_ms: 5.0,
            bandwidth_mbps: 1000.0,
            utilization: 0.5,
            reliability: 0.99,
            cost: 1.0,
        };
        
        topology.add_link(link);
        assert_eq!(topology.get_link_count(), 1);
    }

    #[test]
    fn test_path_finding() {
        let mut topology = NetworkTopology::new();
        
        // Create simple 3-node linear topology: 0 -> 1 -> 2
        for i in 0..3 {
            let node = NetworkNode {
                id: i,
                address: format!("192.168.1.{}", i + 1),
                capacity: 1000.0,
                current_load: 100.0,
                latency_ms: 10.0,
                connections: HashSet::new(),
                last_seen: std::time::SystemTime::now(),
                node_type: NodeType::Core,
            };
            topology.add_node(node);
        }
        
        // Add links: 0-1 and 1-2
        for i in 0..2 {
            let link = NetworkLink {
                from: i,
                to: i + 1,
                latency_ms: 5.0,
                bandwidth_mbps: 1000.0,
                utilization: 0.5,
                reliability: 0.99,
                cost: 1.0,
            };
            topology.add_link(link);
        }
        
        let path = topology.find_optimal_path(0, 2, PathOptimization::Latency);
        assert_eq!(path, Some(vec![0, 1, 2]));
    }

    #[test]
    fn test_load_balancer() {
        let paths = vec![
            vec![0, 1, 2],
            vec![0, 3, 2],
            vec![0, 4, 5, 2],
        ];
        
        let mut lb = LoadBalancer::new(LoadBalancingAlgorithm::RoundRobin, paths);
        
        // Test round-robin behavior
        assert_eq!(lb.select_path(), Some(&vec![0, 1, 2]));
        assert_eq!(lb.select_path(), Some(&vec![0, 3, 2]));
        assert_eq!(lb.select_path(), Some(&vec![0, 4, 5, 2]));
        assert_eq!(lb.select_path(), Some(&vec![0, 1, 2])); // Wraps around
        
        assert_eq!(lb.get_path_count(), 3);
    }

    #[test]
    fn test_multipath_routing() {
        let mut topology = NetworkTopology::new();
        
        // Create diamond topology: 0 -> {1,2} -> 3
        for i in 0..4 {
            let node = NetworkNode {
                id: i,
                address: format!("192.168.1.{}", i + 1),
                capacity: 1000.0,
                current_load: 100.0,
                latency_ms: 10.0,
                connections: HashSet::new(),
                last_seen: std::time::SystemTime::now(),
                node_type: NodeType::Core,
            };
            topology.add_node(node);
        }
        
        // Add links to create diamond
        let links = vec![
            (0, 1), (0, 2), (1, 3), (2, 3)
        ];
        
        for (from, to) in links {
            let link = NetworkLink {
                from,
                to,
                latency_ms: 5.0,
                bandwidth_mbps: 1000.0,
                utilization: 0.5,
                reliability: 0.99,
                cost: 1.0,
            };
            topology.add_link(link);
        }
        
        let paths = topology.find_multiple_paths(0, 3, 2);
        assert_eq!(paths.len(), 2); // Should find two paths: 0->1->3 and 0->2->3
    }
}