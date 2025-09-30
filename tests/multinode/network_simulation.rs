// Network Simulation Module
// Simulates network conditions, partitions, and connectivity testing

use anyhow::{Result, Context};
use rand::prelude::*;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use tokio::time;
use tracing::{debug, error, info, warn};

use super::{TestNode, PartitionType};

/// Network partition representation
#[derive(Debug)]
pub struct NetworkPartition {
    pub partition_type: PartitionType,
    pub partitions: Vec<HashSet<String>>,
    pub blocked_connections: HashSet<(String, String)>,
    pub asymmetric_rules: HashMap<(String, String), ConnectionRule>,
    pub start_time: Instant,
    pub healed: bool,
}

/// Connection rule for asymmetric partitions
#[derive(Debug, Clone)]
pub enum ConnectionRule {
    Allow,
    Block,
    Delay(Duration),
    PacketLoss(f64),
    Bandwidth(u64), // bytes/sec
}

/// Test connectivity between two nodes
pub async fn test_connectivity(node_a: &TestNode, node_b: &TestNode) -> Result<Duration> {
    let start = Instant::now();

    // Simulate ping/pong
    let latency = measure_latency(&node_a.address, &node_b.address).await?;

    // Test bidirectional communication
    let forward_test = test_connection(&node_a.address, &node_b.address).await?;
    let reverse_test = test_connection(&node_b.address, &node_a.address).await?;

    if !forward_test || !reverse_test {
        return Err(anyhow::anyhow!(
            "Connectivity test failed between {} and {}",
            node_a.id,
            node_b.id
        ));
    }

    debug!(
        "Connectivity test successful: {} <-> {} ({}ms)",
        node_a.id,
        node_b.id,
        latency.as_millis()
    );

    Ok(latency)
}

/// Create a network partition
pub async fn create_partition(
    nodes: &Arc<RwLock<Vec<TestNode>>>,
    partition_type: &PartitionType,
) -> Result<NetworkPartition> {
    info!("Creating {:?} network partition", partition_type);

    let nodes = nodes.read().await;
    let node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();

    let mut partition = NetworkPartition {
        partition_type: partition_type.clone(),
        partitions: Vec::new(),
        blocked_connections: HashSet::new(),
        asymmetric_rules: HashMap::new(),
        start_time: Instant::now(),
        healed: false,
    };

    match partition_type {
        PartitionType::SplitBrain => {
            create_split_brain_partition(&mut partition, &node_ids);
        }
        PartitionType::AsymmetricPartition => {
            create_asymmetric_partition(&mut partition, &node_ids);
        }
        PartitionType::PartialConnectivity => {
            create_partial_connectivity(&mut partition, &node_ids);
        }
        PartitionType::ProgressiveIsolation => {
            create_progressive_isolation(&mut partition, &node_ids);
        }
        PartitionType::RandomPartitions => {
            create_random_partitions(&mut partition, &node_ids);
        }
    }

    // Apply partition rules to network
    apply_partition_rules(&partition, &nodes).await?;

    Ok(partition)
}

/// Create split-brain partition
fn create_split_brain_partition(partition: &mut NetworkPartition, nodes: &[String]) {
    let mid = nodes.len() / 2;
    let group_a: HashSet<String> = nodes[..mid].iter().cloned().collect();
    let group_b: HashSet<String> = nodes[mid..].iter().cloned().collect();

    // Block all connections between groups
    for node_a in &group_a {
        for node_b in &group_b {
            partition.blocked_connections.insert((node_a.clone(), node_b.clone()));
            partition.blocked_connections.insert((node_b.clone(), node_a.clone()));
        }
    }

    partition.partitions.push(group_a);
    partition.partitions.push(group_b);

    info!("Created split-brain partition with {} groups", partition.partitions.len());
}

/// Create asymmetric partition
fn create_asymmetric_partition(partition: &mut NetworkPartition, nodes: &[String]) {
    if nodes.len() < 3 {
        return;
    }

    // Node 0 can send to everyone but only receive from half
    let node_0 = &nodes[0];
    let half = nodes.len() / 2;

    for i in half..nodes.len() {
        // Block incoming connections from second half to node 0
        partition.asymmetric_rules.insert(
            (nodes[i].clone(), node_0.clone()),
            ConnectionRule::Block,
        );
    }

    // Create groups for visualization
    let group_a: HashSet<String> = nodes[..half].iter().cloned().collect();
    let group_b: HashSet<String> = nodes[half..].iter().cloned().collect();
    partition.partitions.push(group_a);
    partition.partitions.push(group_b);

    info!("Created asymmetric partition affecting node {}", node_0);
}

/// Create partial connectivity partition
fn create_partial_connectivity(partition: &mut NetworkPartition, nodes: &[String]) {
    let mut rng = thread_rng();

    // Randomly block 30% of connections
    for i in 0..nodes.len() {
        for j in i + 1..nodes.len() {
            if rng.gen_bool(0.3) {
                partition.blocked_connections.insert((nodes[i].clone(), nodes[j].clone()));
                partition.blocked_connections.insert((nodes[j].clone(), nodes[i].clone()));
            }
        }
    }

    // Create overlapping groups
    let third = nodes.len() / 3;
    let group_a: HashSet<String> = nodes[..2 * third].iter().cloned().collect();
    let group_b: HashSet<String> = nodes[third..].iter().cloned().collect();
    partition.partitions.push(group_a);
    partition.partitions.push(group_b);

    info!("Created partial connectivity partition with {} blocked connections",
          partition.blocked_connections.len());
}

/// Create progressive isolation partition
fn create_progressive_isolation(partition: &mut NetworkPartition, nodes: &[String]) {
    if nodes.is_empty() {
        return;
    }

    // Progressively isolate nodes based on their index
    for i in 0..nodes.len() {
        let isolation_level = i as f64 / nodes.len() as f64;

        for j in 0..nodes.len() {
            if i != j {
                if isolation_level > 0.5 {
                    // High isolation - block connection
                    partition.blocked_connections.insert((nodes[i].clone(), nodes[j].clone()));
                } else if isolation_level > 0.25 {
                    // Medium isolation - add delay
                    partition.asymmetric_rules.insert(
                        (nodes[i].clone(), nodes[j].clone()),
                        ConnectionRule::Delay(Duration::from_millis((isolation_level * 1000.0) as u64)),
                    );
                }
            }
        }

        // Create partition group
        let mut group = HashSet::new();
        group.insert(nodes[i].clone());
        partition.partitions.push(group);
    }

    info!("Created progressive isolation affecting {} nodes", nodes.len());
}

/// Create random partitions
fn create_random_partitions(partition: &mut NetworkPartition, nodes: &[String]) {
    let mut rng = thread_rng();
    let num_partitions = rng.gen_range(2..=4);

    // Randomly assign nodes to partitions
    let mut groups: Vec<HashSet<String>> = vec![HashSet::new(); num_partitions];
    for node in nodes {
        let group_idx = rng.gen_range(0..num_partitions);
        groups[group_idx].insert(node.clone());
    }

    // Block connections between partitions
    for i in 0..num_partitions {
        for j in i + 1..num_partitions {
            for node_a in &groups[i] {
                for node_b in &groups[j] {
                    if rng.gen_bool(0.8) {
                        // 80% chance to block
                        partition.blocked_connections.insert((node_a.clone(), node_b.clone()));
                        partition.blocked_connections.insert((node_b.clone(), node_a.clone()));
                    }
                }
            }
        }
    }

    partition.partitions = groups;

    info!("Created {} random partitions", num_partitions);
}

/// Apply partition rules to the network
async fn apply_partition_rules(partition: &NetworkPartition, nodes: &[TestNode]) -> Result<()> {
    info!("Applying partition rules to network");

    // In a real implementation, this would configure network rules
    // For simulation, we track the rules in the partition struct

    // Apply iptables rules for blocked connections
    for (from, to) in &partition.blocked_connections {
        apply_block_rule(from, to).await?;
    }

    // Apply tc (traffic control) rules for asymmetric conditions
    for ((from, to), rule) in &partition.asymmetric_rules {
        apply_asymmetric_rule(from, to, rule).await?;
    }

    Ok(())
}

/// Apply blocking rule
async fn apply_block_rule(from: &str, to: &str) -> Result<()> {
    // In real implementation: iptables -A INPUT -s $from -j DROP
    debug!("Blocking connection: {} -> {}", from, to);
    Ok(())
}

/// Apply asymmetric rule
async fn apply_asymmetric_rule(from: &str, to: &str, rule: &ConnectionRule) -> Result<()> {
    match rule {
        ConnectionRule::Block => {
            apply_block_rule(from, to).await?;
        }
        ConnectionRule::Delay(delay) => {
            // tc qdisc add dev eth0 root netem delay $delay
            debug!("Adding {}ms delay: {} -> {}", delay.as_millis(), from, to);
        }
        ConnectionRule::PacketLoss(loss) => {
            // tc qdisc add dev eth0 root netem loss $loss%
            debug!("Adding {:.1}% packet loss: {} -> {}", loss * 100.0, from, to);
        }
        ConnectionRule::Bandwidth(bw) => {
            // tc qdisc add dev eth0 root tbf rate $bw
            debug!("Limiting bandwidth to {} bytes/sec: {} -> {}", bw, from, to);
        }
        ConnectionRule::Allow => {
            // No action needed
        }
    }
    Ok(())
}

/// Check system availability during partition
pub async fn check_availability(partition: &NetworkPartition) -> Result<f64> {
    // Check what percentage of the system is still available

    let total_nodes = partition.partitions.iter()
        .map(|p| p.len())
        .sum::<usize>();

    if total_nodes == 0 {
        return Ok(0.0);
    }

    // Find largest partition (assumes it maintains availability)
    let largest_partition = partition.partitions.iter()
        .map(|p| p.len())
        .max()
        .unwrap_or(0);

    let availability = largest_partition as f64 / total_nodes as f64;

    debug!("System availability during partition: {:.2}%", availability * 100.0);

    Ok(availability)
}

/// Check data consistency during partition
pub async fn check_consistency(partition: &NetworkPartition) -> Result<bool> {
    // Check if data remains consistent across partitions

    // Simulate checking data state in each partition
    let mut partition_states = Vec::new();

    for group in &partition.partitions {
        if !group.is_empty() {
            let state = get_partition_state(group).await?;
            partition_states.push(state);
        }
    }

    // Check if all partitions that can communicate have consistent state
    let consistent = check_state_consistency(&partition_states);

    debug!("Data consistency check: {}", if consistent { "PASS" } else { "FAIL" });

    Ok(consistent)
}

/// Heal a network partition
pub async fn heal_partition(partition: &NetworkPartition) -> Result<()> {
    info!("Healing network partition");

    // Remove all blocking rules
    for (from, to) in &partition.blocked_connections {
        remove_block_rule(from, to).await?;
    }

    // Remove all asymmetric rules
    for ((from, to), _rule) in &partition.asymmetric_rules {
        remove_asymmetric_rules(from, to).await?;
    }

    info!("Network partition healed after {:?}", partition.start_time.elapsed());

    Ok(())
}

/// Measure recovery time after partition healing
pub async fn measure_recovery_time(partition: &NetworkPartition) -> Result<Duration> {
    let start = Instant::now();

    // Wait for network to stabilize
    time::sleep(Duration::from_millis(100)).await;

    // Check connectivity restoration
    loop {
        if check_full_connectivity().await? {
            break;
        }

        if start.elapsed() > Duration::from_secs(60) {
            return Err(anyhow::anyhow!("Recovery timeout exceeded"));
        }

        time::sleep(Duration::from_millis(100)).await;
    }

    // Check consensus restoration
    loop {
        if check_consensus_restored().await? {
            break;
        }

        if start.elapsed() > Duration::from_secs(120) {
            return Err(anyhow::anyhow!("Consensus recovery timeout exceeded"));
        }

        time::sleep(Duration::from_millis(100)).await;
    }

    let recovery_time = start.elapsed();
    info!("System recovered in {:?}", recovery_time);

    Ok(recovery_time)
}

/// Validate final consistency after partition healing
pub async fn validate_final_consistency(nodes: &Arc<RwLock<Vec<TestNode>>>) -> Result<bool> {
    info!("Validating final consistency across all nodes");

    let nodes = nodes.read().await;
    let mut node_states = HashMap::new();

    // Collect state from each node
    for node in nodes.iter() {
        let state = get_node_state(node).await?;
        node_states.insert(node.id.clone(), state);
    }

    // Verify all nodes have converged to same state
    let states: HashSet<_> = node_states.values().cloned().collect();

    if states.len() == 1 {
        info!("All nodes have consistent state");
        Ok(true)
    } else {
        error!("Inconsistent state detected across {} different values", states.len());
        Ok(false)
    }
}

// Helper functions

async fn measure_latency(addr_a: &SocketAddr, addr_b: &SocketAddr) -> Result<Duration> {
    // Simulate measuring network latency
    let base_latency = Duration::from_millis(5);
    let jitter = Duration::from_millis(thread_rng().gen_range(0..3));
    Ok(base_latency + jitter)
}

async fn test_connection(from: &SocketAddr, to: &SocketAddr) -> Result<bool> {
    // Simulate testing TCP connection
    Ok(true)
}

async fn remove_block_rule(from: &str, to: &str) -> Result<()> {
    debug!("Removing block rule: {} -> {}", from, to);
    Ok(())
}

async fn remove_asymmetric_rules(from: &str, to: &str) -> Result<()> {
    debug!("Removing asymmetric rules: {} -> {}", from, to);
    Ok(())
}

async fn check_full_connectivity() -> Result<bool> {
    // Check if all nodes can communicate
    Ok(true)
}

async fn check_consensus_restored() -> Result<bool> {
    // Check if consensus mechanism is working
    Ok(true)
}

async fn get_partition_state(nodes: &HashSet<String>) -> Result<String> {
    // Get aggregated state from partition
    Ok(format!("state_{}", nodes.len()))
}

fn check_state_consistency(states: &[String]) -> bool {
    // For simulation, assume consistent if same length
    states.windows(2).all(|w| w[0].len() == w[1].len())
}

async fn get_node_state(node: &TestNode) -> Result<String> {
    // Get current state from node
    Ok(format!("state_{}", node.id))
}

/// Network health monitoring
pub struct NetworkMonitor {
    pub latency_samples: Arc<RwLock<Vec<Duration>>>,
    pub packet_loss: Arc<RwLock<f64>>,
    pub bandwidth_usage: Arc<RwLock<u64>>,
    pub connection_count: Arc<RwLock<usize>>,
}

impl NetworkMonitor {
    pub fn new() -> Self {
        Self {
            latency_samples: Arc::new(RwLock::new(Vec::new())),
            packet_loss: Arc::new(RwLock::new(0.0)),
            bandwidth_usage: Arc::new(RwLock::new(0)),
            connection_count: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn record_latency(&self, latency: Duration) {
        let mut samples = self.latency_samples.write().await;
        samples.push(latency);

        // Keep only last 1000 samples
        if samples.len() > 1000 {
            samples.remove(0);
        }
    }

    pub async fn get_latency_stats(&self) -> LatencyStats {
        let samples = self.latency_samples.read().await;

        if samples.is_empty() {
            return LatencyStats::default();
        }

        let mut sorted = samples.clone();
        sorted.sort();

        LatencyStats {
            min: sorted[0],
            max: sorted[sorted.len() - 1],
            avg: Duration::from_nanos(
                sorted.iter().map(|d| d.as_nanos()).sum::<u128>() as u64 / sorted.len() as u64
            ),
            p50: sorted[sorted.len() / 2],
            p99: sorted[sorted.len() * 99 / 100],
        }
    }
}

#[derive(Debug, Default)]
pub struct LatencyStats {
    pub min: Duration,
    pub max: Duration,
    pub avg: Duration,
    pub p50: Duration,
    pub p99: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_split_brain_creation() {
        let mut partition = NetworkPartition {
            partition_type: PartitionType::SplitBrain,
            partitions: Vec::new(),
            blocked_connections: HashSet::new(),
            asymmetric_rules: HashMap::new(),
            start_time: Instant::now(),
            healed: false,
        };

        let nodes = vec!["node1".to_string(), "node2".to_string(),
                        "node3".to_string(), "node4".to_string()];

        create_split_brain_partition(&mut partition, &nodes);

        assert_eq!(partition.partitions.len(), 2);
        assert_eq!(partition.blocked_connections.len(), 8); // 2*2*2 bidirectional
    }

    #[tokio::test]
    async fn test_network_monitor() {
        let monitor = NetworkMonitor::new();

        monitor.record_latency(Duration::from_millis(10)).await;
        monitor.record_latency(Duration::from_millis(20)).await;
        monitor.record_latency(Duration::from_millis(15)).await;

        let stats = monitor.get_latency_stats().await;
        assert_eq!(stats.min, Duration::from_millis(10));
        assert_eq!(stats.max, Duration::from_millis(20));
    }
}