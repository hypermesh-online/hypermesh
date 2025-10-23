/*!
# Layer 3 (ALM) Criterion Benchmarks

Adaptive routing and topology management benchmarks for the Adaptive Logic Manager layer.
Tests graph algorithms, routing optimization, and dynamic topology management.
*/

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use mfn_benchmarks::{common::*, layer3::*};
use std::time::Duration;

fn bench_dijkstra_pathfinding(c: &mut Criterion) {
    let mut group = c.benchmark_group("dijkstra_pathfinding");
    group.measurement_time(Duration::from_secs(15));
    
    // Test different graph sizes and densities
    let node_counts = [100, 500, 1000, 5000];
    let edge_densities = [0.1, 0.3, 0.5]; // Percentage of possible edges
    
    for &node_count in node_counts.iter() {
        for &density in edge_densities.iter() {
            let topology = NetworkTopology::generate_random(node_count, density);
            let pathfinder = DijkstraPathfinder::new(&topology);
            
            // Generate test source-destination pairs
            let test_pairs = generate_node_pairs(100, node_count);
            
            group.throughput(Throughput::Elements(test_pairs.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("shortest_path", format!("{}nodes_{:.0}%dense", node_count, density * 100.0)),
                &(&pathfinder, &test_pairs),
                |b, (finder, pairs)| {
                    b.iter(|| {
                        let mut paths = Vec::new();
                        for (source, dest) in pairs.iter() {
                            let path = finder.find_shortest_path(*source, *dest);
                            paths.push(black_box(path));
                        }
                        black_box(paths)
                    })
                },
            );
        }
    }
    
    group.finish();
}

fn bench_multipath_routing(c: &mut Criterion) {
    let mut group = c.benchmark_group("multipath_routing");
    group.measurement_time(Duration::from_secs(20));
    
    let k_values = [2, 4, 8]; // Number of paths to find
    let topology_sizes = [200, 500, 1000];
    
    for &k in k_values.iter() {
        for &size in topology_sizes.iter() {
            let topology = NetworkTopology::generate_realistic(size, 0.2);
            let multipath_router = MultipathRouter::new(&topology, k);
            
            let test_pairs = generate_node_pairs(50, size);
            
            group.throughput(Throughput::Elements(test_pairs.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("k_shortest_paths", format!("k{}_{nodes}", k, nodes = size)),
                &(&multipath_router, &test_pairs),
                |b, (router, pairs)| {
                    b.iter(|| {
                        let mut path_sets = Vec::new();
                        for (source, dest) in pairs.iter() {
                            let paths = router.find_k_shortest_paths(*source, *dest);
                            path_sets.push(black_box(paths));
                        }
                        black_box(path_sets)
                    })
                },
            );
        }
    }
    
    group.finish();
}

fn bench_load_balancing_algorithms(c: &mut Criterion) {
    let mut group = c.benchmark_group("load_balancing_algorithms");
    
    let server_counts = [5, 10, 20, 50];
    let algorithms = ["round_robin", "least_connections", "weighted_round_robin"];
    
    for &server_count in server_counts.iter() {
        for algorithm in algorithms.iter() {
            let mut load_balancer = LoadBalancer::new(*algorithm, server_count);
            
            // Pre-populate with realistic server states
            for i in 0..server_count {
                load_balancer.add_server(ServerId(i), ServerMetrics {
                    active_connections: fastrand::usize(0..100),
                    cpu_usage: fastrand::f32() * 0.8 + 0.1,
                    memory_usage: fastrand::f32() * 0.7 + 0.2,
                    response_time_ms: fastrand::f32() * 50.0 + 10.0,
                    weight: if i < server_count / 2 { 2.0 } else { 1.0 },
                });
            }
            
            // Generate load balancing requests
            let request_count = 1000;
            
            group.throughput(Throughput::Elements(request_count as u64));
            group.bench_with_input(
                BenchmarkId::new("load_balancing", format!("{}servers_{}", server_count, algorithm)),
                &(&mut load_balancer, request_count),
                |b, (balancer, count)| {
                    b.iter(|| {
                        let mut selections = Vec::new();
                        for _ in 0..*count {
                            let selected_server = balancer.select_server();
                            selections.push(black_box(selected_server));
                        }
                        black_box(selections)
                    })
                },
            );
        }
    }
    
    group.finish();
}

fn bench_topology_adaptation(c: &mut Criterion) {
    let mut group = c.benchmark_group("topology_adaptation");
    group.measurement_time(Duration::from_secs(25));
    
    let initial_sizes = [100, 300, 500];
    let adaptation_rates = [0.05, 0.1, 0.2]; // Fraction of topology changed per adaptation
    
    for &initial_size in initial_sizes.iter() {
        for &rate in adaptation_rates.iter() {
            let mut adaptive_topology = AdaptiveTopology::new(initial_size);
            
            // Generate topology change events
            let changes_per_adaptation = (initial_size as f32 * rate) as usize;
            let topology_changes = generate_topology_changes(changes_per_adaptation * 10, initial_size);
            
            group.throughput(Throughput::Elements(topology_changes.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("topology_adaptation", format!("{}nodes_{:.0}%change", initial_size, rate * 100.0)),
                &(&mut adaptive_topology, &topology_changes),
                |b, (topology, changes)| {
                    b.iter(|| {
                        for change in changes.iter() {
                            let adaptation_result = topology.apply_change(change);
                            black_box(adaptation_result);
                        }
                        
                        // Trigger recomputation of routing tables
                        let recompute_result = topology.recompute_routing_tables();
                        black_box(recompute_result)
                    })
                },
            );
        }
    }
    
    group.finish();
}

fn bench_route_caching(c: &mut Criterion) {
    let mut group = c.benchmark_group("route_caching");
    
    let cache_sizes = [1000, 5000, 10000, 50000];
    let cache_algorithms = ["lru", "lfu", "arc"];
    
    for &cache_size in cache_sizes.iter() {
        for algorithm in cache_algorithms.iter() {
            let mut route_cache = RouteCache::new(*algorithm, cache_size);
            
            // Pre-populate cache with routes
            let initial_routes = generate_cached_routes(cache_size / 2);
            for route in &initial_routes {
                route_cache.insert(route.source, route.destination, route.path.clone());
            }
            
            // Generate cache queries (mix of hits and misses)
            let cache_queries = generate_route_queries(1000, &initial_routes);
            
            group.throughput(Throughput::Elements(cache_queries.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("route_cache", format!("{}entries_{}", cache_size, algorithm)),
                &(&route_cache, &cache_queries),
                |b, (cache, queries)| {
                    b.iter(|| {
                        let mut results = Vec::new();
                        for (source, dest) in queries.iter() {
                            let cached_route = cache.get(*source, *dest);
                            results.push(black_box(cached_route));
                        }
                        black_box(results)
                    })
                },
            );
        }
    }
    
    group.finish();
}

fn bench_dynamic_routing_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("dynamic_routing_updates");
    group.measurement_time(Duration::from_secs(20));
    
    let topology_sizes = [200, 500, 1000];
    let update_frequencies = [10, 50, 100]; // Updates per batch
    
    for &size in topology_sizes.iter() {
        for &frequency in update_frequencies.iter() {
            let mut dynamic_router = DynamicRouter::new(size);
            
            // Generate routing updates
            let routing_updates = generate_routing_updates(frequency * 10, size);
            
            group.throughput(Throughput::Elements(routing_updates.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("routing_updates", format!("{}nodes_{}updates", size, frequency)),
                &(&mut dynamic_router, &routing_updates, frequency),
                |b, (router, updates, freq)| {
                    b.iter(|| {
                        for update_batch in updates.chunks(*freq) {
                            for update in update_batch {
                                router.apply_routing_update(update);
                            }
                            
                            // Trigger routing table convergence
                            let convergence_result = router.converge_routing_tables();
                            black_box(convergence_result);
                        }
                    })
                },
            );
        }
    }
    
    group.finish();
}

fn bench_traffic_engineering(c: &mut Criterion) {
    let mut group = c.benchmark_group("traffic_engineering");
    group.measurement_time(Duration::from_secs(30));
    
    let network_sizes = [100, 300, 500];
    let traffic_loads = [0.3, 0.6, 0.9]; // As fraction of total capacity
    
    for &size in network_sizes.iter() {
        for &load in traffic_loads.iter() {
            let topology = NetworkTopology::generate_realistic(size, 0.25);
            let mut traffic_engineer = TrafficEngineer::new(&topology);
            
            // Generate traffic demand matrix
            let traffic_demands = generate_traffic_demands(size, load);
            
            group.throughput(Throughput::Elements(traffic_demands.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("traffic_optimization", format!("{}nodes_{:.0}%load", size, load * 100.0)),
                &(&mut traffic_engineer, &traffic_demands),
                |b, (engineer, demands)| {
                    b.iter(|| {
                        let optimization_result = engineer.optimize_traffic_flows(demands);
                        black_box(optimization_result)
                    })
                },
            );
        }
    }
    
    group.finish();
}

fn bench_fault_tolerance_mechanisms(c: &mut Criterion) {
    let mut group = c.benchmark_group("fault_tolerance_mechanisms");
    group.measurement_time(Duration::from_secs(20));
    
    let fault_rates = [0.01, 0.05, 0.1]; // Fraction of nodes/links that fail
    let recovery_strategies = ["fast_reroute", "global_recompute", "local_repair"];
    
    for &fault_rate in fault_rates.iter() {
        for strategy in recovery_strategies.iter() {
            let topology_size = 500;
            let mut fault_tolerant_router = FaultTolerantRouter::new(topology_size, *strategy);
            
            // Generate fault scenarios
            let fault_count = (topology_size as f32 * fault_rate) as usize;
            let fault_scenarios = generate_fault_scenarios(fault_count, topology_size);
            
            group.throughput(Throughput::Elements(fault_scenarios.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("fault_recovery", format!("{:.0}%faults_{}", fault_rate * 100.0, strategy)),
                &(&mut fault_tolerant_router, &fault_scenarios),
                |b, (router, scenarios)| {
                    b.iter(|| {
                        for scenario in scenarios.iter() {
                            // Inject faults
                            for fault in &scenario.faults {
                                router.inject_fault(fault);
                            }
                            
                            // Measure recovery time
                            let recovery_result = router.recover_from_faults();
                            black_box(recovery_result);
                            
                            // Restore topology for next iteration
                            router.restore_topology();
                        }
                    })
                },
            );
        }
    }
    
    group.finish();
}

fn bench_integrated_alm_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("integrated_alm_pipeline");
    group.measurement_time(Duration::from_secs(45));
    
    // Benchmark the complete ALM pipeline
    let alm_config = AlmConfig {
        topology_size: 1000,
        cache_size: 10000,
        multipath_k: 4,
        adaptation_threshold: 0.1,
        load_balancing_algorithm: "weighted_round_robin".to_string(),
        fault_tolerance_strategy: "fast_reroute".to_string(),
    };
    
    let mut alm_system = AlmSystem::new(alm_config);
    
    // Generate realistic routing workload
    let routing_requests = generate_routing_workload(1000);
    
    group.throughput(Throughput::Elements(routing_requests.len() as u64));
    group.bench_function("full_pipeline", |b| {
        b.iter(|| {
            let mut results = Vec::new();
            for request in &routing_requests {
                let request = black_box(request);
                
                // Complete ALM processing pipeline
                let routing_result = alm_system.process_routing_request(request);
                results.push(black_box(routing_result));
            }
            black_box(results)
        })
    });
    
    group.finish();
}

// Helper functions for benchmark data generation
fn generate_node_pairs(count: usize, max_node_id: usize) -> Vec<(NodeId, NodeId)> {
    (0..count)
        .map(|i| {
            let source = NodeId(fastrand::usize(0..max_node_id));
            let dest = NodeId(fastrand::usize(0..max_node_id));
            (source, dest)
        })
        .collect()
}

fn generate_topology_changes(count: usize, topology_size: usize) -> Vec<TopologyChange> {
    (0..count)
        .map(|i| {
            match i % 4 {
                0 => TopologyChange::AddNode(NodeId(topology_size + i)),
                1 => TopologyChange::RemoveNode(NodeId(fastrand::usize(0..topology_size))),
                2 => TopologyChange::AddEdge {
                    source: NodeId(fastrand::usize(0..topology_size)),
                    dest: NodeId(fastrand::usize(0..topology_size)),
                    weight: fastrand::f32() * 100.0 + 1.0,
                },
                _ => TopologyChange::UpdateEdgeWeight {
                    source: NodeId(fastrand::usize(0..topology_size)),
                    dest: NodeId(fastrand::usize(0..topology_size)),
                    new_weight: fastrand::f32() * 100.0 + 1.0,
                },
            }
        })
        .collect()
}

fn generate_cached_routes(count: usize) -> Vec<CachedRoute> {
    (0..count)
        .map(|i| {
            let path_length = fastrand::usize(2..8);
            let path: Vec<NodeId> = (0..path_length)
                .map(|j| NodeId((i + j) % 1000))
                .collect();
            
            CachedRoute {
                source: path[0],
                destination: path[path_length - 1],
                path,
                cost: fastrand::f32() * 100.0 + 10.0,
                timestamp: i as u64,
            }
        })
        .collect()
}

fn generate_route_queries(count: usize, existing_routes: &[CachedRoute]) -> Vec<(NodeId, NodeId)> {
    (0..count)
        .map(|i| {
            if i < existing_routes.len() && i % 3 == 0 {
                // Cache hit
                (existing_routes[i].source, existing_routes[i].destination)
            } else {
                // Cache miss
                (NodeId(i + 10000), NodeId(i + 20000))
            }
        })
        .collect()
}

fn generate_routing_updates(count: usize, topology_size: usize) -> Vec<RoutingUpdate> {
    (0..count)
        .map(|i| {
            RoutingUpdate {
                node_id: NodeId(fastrand::usize(0..topology_size)),
                update_type: if i % 2 == 0 { UpdateType::LinkStateChange } else { UpdateType::MetricUpdate },
                data: RoutingUpdateData {
                    neighbor: NodeId(fastrand::usize(0..topology_size)),
                    metric: fastrand::f32() * 100.0 + 1.0,
                    timestamp: i as u64,
                },
            }
        })
        .collect()
}

fn generate_traffic_demands(node_count: usize, load_factor: f32) -> Vec<TrafficDemand> {
    let demand_count = (node_count as f32 * node_count as f32 * load_factor * 0.1) as usize;
    
    (0..demand_count)
        .map(|i| {
            TrafficDemand {
                source: NodeId(fastrand::usize(0..node_count)),
                destination: NodeId(fastrand::usize(0..node_count)),
                bandwidth_mbps: fastrand::f32() * 1000.0 + 100.0,
                priority: fastrand::u8(1..=5),
                duration_seconds: fastrand::u32(60..3600),
            }
        })
        .collect()
}

fn generate_fault_scenarios(fault_count: usize, topology_size: usize) -> Vec<FaultScenario> {
    (0..10) // Generate 10 different fault scenarios
        .map(|scenario_id| {
            let faults: Vec<Fault> = (0..fault_count)
                .map(|i| {
                    if i % 2 == 0 {
                        Fault::NodeFailure(NodeId(fastrand::usize(0..topology_size)))
                    } else {
                        Fault::LinkFailure {
                            source: NodeId(fastrand::usize(0..topology_size)),
                            dest: NodeId(fastrand::usize(0..topology_size)),
                        }
                    }
                })
                .collect();
            
            FaultScenario {
                id: scenario_id,
                faults,
                expected_recovery_time_ms: fastrand::u32(10..1000),
            }
        })
        .collect()
}

fn generate_routing_workload(count: usize) -> Vec<RoutingRequest> {
    (0..count)
        .map(|i| {
            RoutingRequest {
                flow_id: i as u64,
                source: NodeId(fastrand::usize(0..1000)),
                destination: NodeId(fastrand::usize(0..1000)),
                requirements: RoutingRequirements {
                    bandwidth_mbps: fastrand::f32() * 1000.0 + 10.0,
                    max_latency_ms: fastrand::u32(1..100),
                    reliability: fastrand::f32() * 0.5 + 0.5,
                    priority: fastrand::u8(1..=5),
                },
                timestamp: i as u64,
            }
        })
        .collect()
}

criterion_group!(
    alm_benchmarks,
    bench_dijkstra_pathfinding,
    bench_multipath_routing,
    bench_load_balancing_algorithms,
    bench_topology_adaptation,
    bench_route_caching,
    bench_dynamic_routing_updates,
    bench_traffic_engineering,
    bench_fault_tolerance_mechanisms,
    bench_integrated_alm_pipeline
);

criterion_main!(alm_benchmarks);