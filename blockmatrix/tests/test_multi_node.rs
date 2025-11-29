// Multi-Node Simulation Tests for Sprint 3
// Tests HyperMesh's ability to manage multiple nodes with OS abstraction

#[cfg(test)]
mod multi_node_tests {
    use blockmatrix::os_integration::{create_os_abstraction, types::*, OsAbstraction};
    use std::sync::{Arc, Mutex, mpsc};
    use std::thread;
    use std::time::{Duration, Instant};
    use std::collections::HashMap;
    use anyhow::{Result, anyhow};

    /// Simulated node with its own OS abstraction instance
    struct SimulatedNode {
        id: String,
        os: Box<dyn OsAbstraction>,
        metrics: Arc<Mutex<NodeMetrics>>,
        is_healthy: Arc<Mutex<bool>>,
    }

    #[derive(Debug, Clone, Default)]
    struct NodeMetrics {
        cpu_usage: f64,
        memory_used_mb: u64,
        network_bytes: u64,
        last_update: Option<Instant>,
        sample_count: u64,
    }

    impl SimulatedNode {
        fn new(id: String) -> Result<Self> {
            let os = create_os_abstraction()?;
            Ok(SimulatedNode {
                id,
                os,
                metrics: Arc::new(Mutex::new(NodeMetrics::default())),
                is_healthy: Arc::new(Mutex::new(true)),
            })
        }

        fn start_monitoring(&self, interval: Duration) -> mpsc::Sender<()> {
            let (tx, rx) = mpsc::channel();
            let metrics = Arc::clone(&self.metrics);
            let is_healthy = Arc::clone(&self.is_healthy);
            let os = create_os_abstraction().expect("Failed to create OS abstraction");
            let node_id = self.id.clone();

            thread::spawn(move || {
                loop {
                    // Check for stop signal
                    if rx.try_recv().is_ok() {
                        println!("Node {} stopping monitoring", node_id);
                        break;
                    }

                    // Collect metrics
                    match os.get_resource_usage() {
                        Ok(usage) => {
                            let mut m = metrics.lock().unwrap();
                            m.cpu_usage = usage.cpu_percent;
                            m.memory_used_mb = usage.memory_used_mb;
                            m.network_bytes = usage.network_bytes_sent + usage.network_bytes_received;
                            m.last_update = Some(Instant::now());
                            m.sample_count += 1;
                        }
                        Err(e) => {
                            println!("Node {} metric collection failed: {}", node_id, e);
                            *is_healthy.lock().unwrap() = false;
                        }
                    }

                    thread::sleep(interval);
                }
            });

            tx
        }

        fn get_metrics(&self) -> NodeMetrics {
            self.metrics.lock().unwrap().clone()
        }

        fn is_healthy(&self) -> bool {
            *self.is_healthy.lock().unwrap()
        }

        fn simulate_failure(&self) {
            *self.is_healthy.lock().unwrap() = false;
        }
    }

    #[test]
    fn test_create_multiple_nodes() {
        let mut nodes = Vec::new();

        // Create 3 simulated nodes
        for i in 0..3 {
            let node = SimulatedNode::new(format!("node-{}", i))
                .expect(&format!("Failed to create node {}", i));

            // Verify node has working OS abstraction
            let cpu_info = node.os.detect_cpu()
                .expect(&format!("Node {} CPU detection failed", i));

            assert!(cpu_info.core_count > 0, "Node {} invalid CPU info", i);

            nodes.push(node);
        }

        println!("Created {} simulated nodes successfully", nodes.len());
        assert_eq!(nodes.len(), 3, "Should have 3 nodes");
    }

    #[test]
    fn test_parallel_metric_collection() {
        let mut nodes = Vec::new();
        let mut stop_channels = Vec::new();

        // Create and start 3 nodes
        for i in 0..3 {
            let node = SimulatedNode::new(format!("node-{}", i))
                .expect(&format!("Failed to create node {}", i));

            let stop_tx = node.start_monitoring(Duration::from_millis(100));
            stop_channels.push(stop_tx);
            nodes.push(node);
        }

        // Let them collect metrics
        thread::sleep(Duration::from_secs(2));

        // Verify all nodes collected metrics
        for node in &nodes {
            let metrics = node.get_metrics();
            assert!(
                metrics.sample_count > 10,
                "Node {} only collected {} samples",
                node.id, metrics.sample_count
            );
            assert!(
                metrics.last_update.is_some(),
                "Node {} has no metric updates",
                node.id
            );

            println!("Node {} collected {} samples", node.id, metrics.sample_count);
        }

        // Stop monitoring
        for tx in stop_channels {
            let _ = tx.send(());
        }
    }

    #[test]
    fn test_aggregate_metrics_across_nodes() {
        let mut nodes = Vec::new();

        // Create 3 nodes and collect initial metrics
        for i in 0..3 {
            let node = SimulatedNode::new(format!("node-{}", i))
                .expect(&format!("Failed to create node {}", i));

            // Get initial metrics
            let usage = node.os.get_resource_usage()
                .expect("Failed to get resource usage");

            let mut metrics = node.metrics.lock().unwrap();
            metrics.cpu_usage = usage.cpu_percent;
            metrics.memory_used_mb = usage.memory_used_mb;
            metrics.network_bytes = usage.network_bytes_sent + usage.network_bytes_received;
            drop(metrics);

            nodes.push(node);
        }

        // Aggregate metrics
        let mut total_cpu = 0.0;
        let mut total_memory = 0;
        let mut total_network = 0;

        for node in &nodes {
            let metrics = node.get_metrics();
            total_cpu += metrics.cpu_usage;
            total_memory += metrics.memory_used_mb;
            total_network += metrics.network_bytes;
        }

        let avg_cpu = total_cpu / nodes.len() as f64;

        println!("Cluster aggregate metrics:");
        println!("  Average CPU: {:.2}%", avg_cpu);
        println!("  Total Memory: {} MB", total_memory);
        println!("  Total Network: {} bytes", total_network);

        // Verify reasonable values
        assert!(avg_cpu >= 0.0 && avg_cpu <= 100.0, "Invalid average CPU");
        assert!(total_memory > 0, "No memory usage detected");
    }

    #[test]
    fn test_node_failure_handling() {
        let mut nodes = Vec::new();
        let mut stop_channels = Vec::new();

        // Create 3 nodes
        for i in 0..3 {
            let node = SimulatedNode::new(format!("node-{}", i))
                .expect(&format!("Failed to create node {}", i));

            let stop_tx = node.start_monitoring(Duration::from_millis(100));
            stop_channels.push(stop_tx);
            nodes.push(node);
        }

        thread::sleep(Duration::from_millis(500));

        // Simulate node 1 failure
        nodes[1].simulate_failure();

        thread::sleep(Duration::from_millis(500));

        // Check health status
        let healthy_count = nodes.iter().filter(|n| n.is_healthy()).count();
        assert_eq!(healthy_count, 2, "Expected 2 healthy nodes after 1 failure");

        // Verify we can still collect from healthy nodes
        let mut collected = 0;
        for node in &nodes {
            if node.is_healthy() {
                let metrics = node.get_metrics();
                if metrics.sample_count > 0 {
                    collected += 1;
                }
            }
        }

        assert_eq!(collected, 2, "Should collect metrics from 2 healthy nodes");

        println!("Node failure handled: 2/3 nodes remain operational");

        // Cleanup
        for tx in stop_channels {
            let _ = tx.send(());
        }
    }

    #[test]
    fn test_node_resource_heterogeneity() {
        // Simulate nodes with different capabilities
        let node_configs = vec![
            ("high-cpu", 32, 64000),  // 32 cores, 64GB
            ("high-mem", 8, 256000),   // 8 cores, 256GB
            ("balanced", 16, 32000),   // 16 cores, 32GB
        ];

        for (name, expected_cores, expected_mem) in node_configs {
            let node = SimulatedNode::new(name.to_string())
                .expect(&format!("Failed to create node {}", name));

            let cpu = node.os.detect_cpu().expect("CPU detection failed");
            let mem = node.os.detect_memory().expect("Memory detection failed");

            println!("Node {} capabilities:", name);
            println!("  CPU: {} cores @ {} MHz", cpu.core_count, cpu.frequency_mhz);
            println!("  Memory: {} MB total", mem.total_mb);

            // In reality, all nodes have same hardware in test
            // But verify detection works consistently
            assert!(cpu.core_count > 0, "Invalid CPU count");
            assert!(mem.total_mb > 0, "Invalid memory size");
        }
    }

    #[test]
    fn test_concurrent_node_operations() {
        let nodes: Vec<_> = (0..5)
            .map(|i| {
                Arc::new(
                    SimulatedNode::new(format!("node-{}", i))
                        .expect("Failed to create node")
                )
            })
            .collect();

        let mut handles = Vec::new();

        // Each node performs operations concurrently
        for node in nodes.clone() {
            let handle = thread::spawn(move || {
                let start = Instant::now();
                let mut results = Vec::new();

                for _ in 0..10 {
                    // Detect hardware
                    let cpu = node.os.detect_cpu();
                    let mem = node.os.detect_memory();

                    // Collect metrics
                    let usage = node.os.get_resource_usage();

                    results.push((cpu.is_ok(), mem.is_ok(), usage.is_ok()));
                }

                (start.elapsed(), results)
            });
            handles.push(handle);
        }

        // Wait for all nodes
        let mut total_time = Duration::ZERO;
        let mut total_success = 0;

        for handle in handles {
            let (duration, results) = handle.join().expect("Thread panicked");
            total_time += duration;

            for (cpu_ok, mem_ok, usage_ok) in results {
                if cpu_ok && mem_ok && usage_ok {
                    total_success += 1;
                }
            }
        }

        let avg_time = total_time / 5;
        println!("Concurrent operations across 5 nodes:");
        println!("  Average time per node: {:?}", avg_time);
        println!("  Success rate: {}/50", total_success);

        assert_eq!(total_success, 50, "Some operations failed");
        assert!(avg_time < Duration::from_secs(2), "Operations too slow");
    }

    #[test]
    fn test_node_coordination_protocol() {
        // Simulate coordinator selecting best node for workload
        let mut nodes = Vec::new();

        for i in 0..3 {
            let node = SimulatedNode::new(format!("node-{}", i))
                .expect("Failed to create node");

            // Collect current state
            let usage = node.os.get_resource_usage()
                .expect("Failed to get usage");

            let mut metrics = node.metrics.lock().unwrap();
            metrics.cpu_usage = usage.cpu_percent;
            metrics.memory_used_mb = usage.memory_used_mb;
            drop(metrics);

            nodes.push(node);
        }

        // Find node with lowest CPU usage
        let mut best_node_idx = 0;
        let mut lowest_cpu = f64::MAX;

        for (i, node) in nodes.iter().enumerate() {
            let metrics = node.get_metrics();
            if metrics.cpu_usage < lowest_cpu {
                lowest_cpu = metrics.cpu_usage;
                best_node_idx = i;
            }
        }

        println!("Selected node-{} with {:.2}% CPU for workload",
            best_node_idx, lowest_cpu);

        // Verify selection is reasonable
        assert!(lowest_cpu >= 0.0 && lowest_cpu <= 100.0);
    }

    #[test]
    fn test_cluster_wide_resource_limits() {
        let nodes: Vec<_> = (0..3)
            .map(|i| SimulatedNode::new(format!("node-{}", i)).unwrap())
            .collect();

        // Set cluster-wide resource limits
        let max_cluster_cpu_percent = 80.0 * nodes.len() as f64; // 80% per node
        let max_cluster_memory_mb = 100000; // 100GB total

        // Collect current usage
        let mut total_cpu = 0.0;
        let mut total_memory = 0;

        for node in &nodes {
            let usage = node.os.get_resource_usage()
                .expect("Failed to get usage");

            total_cpu += usage.cpu_percent;
            total_memory += usage.memory_used_mb;
        }

        println!("Cluster resource usage:");
        println!("  Total CPU: {:.2}% / {:.2}%", total_cpu, max_cluster_cpu_percent);
        println!("  Total Memory: {} MB / {} MB", total_memory, max_cluster_memory_mb);

        // Check if within limits
        let cpu_headroom = max_cluster_cpu_percent - total_cpu;
        let mem_headroom = max_cluster_memory_mb - total_memory as u64;

        assert!(cpu_headroom > 0.0, "Cluster CPU over limit");
        assert!(mem_headroom > 0, "Cluster memory over limit");

        println!("Cluster has {:.2}% CPU and {} MB memory headroom",
            cpu_headroom, mem_headroom);
    }

    #[test]
    fn test_rolling_node_updates() {
        let mut nodes = Vec::new();
        let mut stop_channels = Vec::new();

        // Create and start nodes
        for i in 0..3 {
            let node = SimulatedNode::new(format!("node-{}", i))
                .expect("Failed to create node");
            let stop_tx = node.start_monitoring(Duration::from_millis(100));
            stop_channels.push(stop_tx);
            nodes.push(node);
        }

        thread::sleep(Duration::from_millis(500));

        // Simulate rolling update
        for i in 0..3 {
            println!("Updating node-{}...", i);

            // Stop monitoring (simulate node going down)
            let _ = stop_channels[i].send(());

            thread::sleep(Duration::from_millis(200));

            // "Restart" with new monitoring
            let new_stop = nodes[i].start_monitoring(Duration::from_millis(100));
            stop_channels[i] = new_stop;

            // Verify other nodes still healthy
            let healthy: Vec<_> = nodes.iter()
                .enumerate()
                .filter(|(idx, _)| *idx != i)
                .filter(|(_, n)| n.is_healthy())
                .collect();

            assert_eq!(healthy.len(), 2,
                "Expected 2 healthy nodes during update of node {}", i);
        }

        println!("Rolling update completed successfully");

        // Cleanup
        for tx in stop_channels {
            let _ = tx.send(());
        }
    }
}