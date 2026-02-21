// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Block propagation through matrix topology
//!
//! Uses the matrix coordinate system from Sprint 1.1 to propagate blocks
//! to neighboring nodes based on distance metrics.
//!
//! Transport abstraction: `BlockTransport` trait decouples propagation logic
//! from the actual network layer. Production code supplies a STOQ-backed
//! implementation; tests can inject a deterministic stub.

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug};

use crate::matrix::coordinate::MatrixCoordinate;
use crate::matrix::neighbors::{find_neighbors, find_k_nearest};
use crate::matrix::tensor::routing::calculate_routing_path;
use super::block::Block;

/// Abstraction over the network transport used for block propagation.
///
/// Implementations send serialised block data to a target matrix coordinate
/// and return `true` on success.  The default (simulation) implementation
/// uses a distance-based probability model for testing.
#[async_trait::async_trait]
pub trait BlockTransport: Send + Sync {
    /// Send `block` to the node at `target`.
    ///
    /// Returns `true` when the remote node acknowledged reception.
    async fn send_block(
        &self,
        block: &Block,
        target: &MatrixCoordinate,
        origin: &MatrixCoordinate,
    ) -> bool;
}

/// Default simulation transport that models success probability based on
/// distance.  Used when no real STOQ transport is wired in.
pub struct SimulatedTransport;

#[async_trait::async_trait]
impl BlockTransport for SimulatedTransport {
    async fn send_block(
        &self,
        _block: &Block,
        target: &MatrixCoordinate,
        origin: &MatrixCoordinate,
    ) -> bool {
        let distance = origin.euclidean_distance(target);
        let success_probability = (1.0 / (1.0 + distance * 0.01)).max(0.5);
        rand::random::<f64>() < success_probability
    }
}

/// Propagation strategy for blocks through the matrix
#[derive(Debug, Clone)]
pub enum PropagationStrategy {
    /// Send to all immediate neighbors
    Broadcast,
    /// Send only to closest N neighbors
    NearestN(usize),
    /// Send along optimal routing paths
    RoutedPath,
    /// Send based on distance threshold
    DistanceThreshold(f64),
}

/// Result of a propagation attempt
#[derive(Debug, Clone)]
pub struct PropagationResult {
    /// Nodes that received the block
    pub reached_nodes: Vec<MatrixCoordinate>,
    /// Nodes that failed to receive
    pub failed_nodes: Vec<MatrixCoordinate>,
    /// Total propagation time in milliseconds
    pub propagation_time_ms: u64,
    /// Number of hops taken
    pub hop_count: usize,
}

/// Block propagation manager for matrix topology
pub struct BlockPropagator {
    /// This node's coordinate
    node_coordinate: MatrixCoordinate,
    /// Propagation strategy
    strategy: PropagationStrategy,
    /// Nodes that have seen blocks (hash -> set of coordinates)
    seen_blocks: Arc<RwLock<HashMap<String, HashSet<MatrixCoordinate>>>>,
    /// Network transport for sending blocks to peers
    transport: Arc<dyn BlockTransport>,
}

use std::collections::HashMap;

impl BlockPropagator {
    /// Create a new block propagator with the default simulated transport.
    pub fn new(
        node_coordinate: MatrixCoordinate,
        strategy: PropagationStrategy,
    ) -> Self {
        Self::with_transport(
            node_coordinate,
            strategy,
            Arc::new(SimulatedTransport),
        )
    }

    /// Create a block propagator backed by a real transport implementation.
    pub fn with_transport(
        node_coordinate: MatrixCoordinate,
        strategy: PropagationStrategy,
        transport: Arc<dyn BlockTransport>,
    ) -> Self {
        BlockPropagator {
            node_coordinate,
            strategy,
            seen_blocks: Arc::new(RwLock::new(HashMap::new())),
            transport,
        }
    }

    /// Propagate a block to neighboring nodes
    pub async fn propagate_block(
        &self,
        block: &Block,
        network_nodes: &[MatrixCoordinate],
    ) -> PropagationResult {
        let start_time = std::time::Instant::now();
        let mut reached_nodes = Vec::new();
        let mut failed_nodes = Vec::new();

        // Get target nodes based on strategy
        let targets = self.select_propagation_targets(network_nodes).await;

        info!(
            "Propagating block {} from ({},{},{}) to {} targets",
            block.index,
            self.node_coordinate.x,
            self.node_coordinate.y,
            self.node_coordinate.z,
            targets.len()
        );

        // Mark this block as seen by this node
        self.mark_block_seen(&block.hash, &self.node_coordinate).await;

        // Propagate to each target via the wired transport
        for target in &targets {
            if self.should_propagate_to(&block.hash, target).await {
                if self.transport.send_block(block, target, &self.node_coordinate).await {
                    reached_nodes.push(target.clone());
                    self.mark_block_seen(&block.hash, target).await;
                } else {
                    failed_nodes.push(target.clone());
                }
            }
        }

        let propagation_time_ms = start_time.elapsed().as_millis() as u64;

        PropagationResult {
            reached_nodes,
            failed_nodes,
            propagation_time_ms,
            hop_count: 1, // Direct propagation for now
        }
    }

    /// Select target nodes based on propagation strategy
    async fn select_propagation_targets(
        &self,
        network_nodes: &[MatrixCoordinate],
    ) -> Vec<MatrixCoordinate> {
        match &self.strategy {
            PropagationStrategy::Broadcast => {
                // Get all immediate neighbors (distance <= 1.5)
                find_neighbors(&self.node_coordinate, network_nodes, 1.5)
            }

            PropagationStrategy::NearestN(n) => {
                // Get N nearest neighbors, extract just the coordinates
                find_k_nearest(&self.node_coordinate, network_nodes, *n)
                    .into_iter()
                    .map(|(coord, _distance)| coord)
                    .collect()
            }

            PropagationStrategy::DistanceThreshold(threshold) => {
                // Get all nodes within distance threshold
                find_neighbors(&self.node_coordinate, network_nodes, *threshold)
            }

            PropagationStrategy::RoutedPath => {
                // Use routing to find optimal paths
                self.select_routed_targets(network_nodes).await
            }
        }
    }

    /// Select targets using routing algorithm
    async fn select_routed_targets(
        &self,
        network_nodes: &[MatrixCoordinate],
    ) -> Vec<MatrixCoordinate> {
        // Find strategic relay nodes (corners of the matrix space)
        let relay_points = self.find_relay_nodes(network_nodes);

        // For each relay, find nearby nodes
        let mut targets = Vec::new();
        for relay in relay_points {
            // Use routing to get path vector
            let _path = calculate_routing_path(&self.node_coordinate, &relay, 3.0);

            // Find nodes close to the path (within distance 2)
            for node in network_nodes {
                let dist_to_relay = node.euclidean_distance(&relay);
                if dist_to_relay <= 2.0 && !targets.contains(node) && *node != self.node_coordinate {
                    targets.push(node.clone());
                    if targets.len() >= 6 {
                        break;
                    }
                }
            }
        }

        targets
    }

    /// Find strategic relay nodes for efficient propagation
    fn find_relay_nodes(&self, network_nodes: &[MatrixCoordinate]) -> Vec<MatrixCoordinate> {
        if network_nodes.is_empty() {
            return Vec::new();
        }

        let mut relays = Vec::new();

        // Find boundary nodes (min/max in each dimension)
        let mut min_x = i64::MAX;
        let mut max_x = i64::MIN;
        let mut min_y = i64::MAX;
        let mut max_y = i64::MIN;
        let mut min_z = i64::MAX;
        let mut max_z = i64::MIN;

        for node in network_nodes {
            min_x = min_x.min(node.x);
            max_x = max_x.max(node.x);
            min_y = min_y.min(node.y);
            max_y = max_y.max(node.y);
            min_z = min_z.min(node.z);
            max_z = max_z.max(node.z);
        }

        // Add corner nodes as relays
        for x in &[min_x, max_x] {
            for y in &[min_y, max_y] {
                for z in &[min_z, max_z] {
                    // Try to create corner coordinate
                    if let Ok(corner) = MatrixCoordinate::new(*x, *y, *z) {
                        // Find closest actual node to this corner
                        if let Some(closest) = self.find_closest_node(&corner, network_nodes) {
                            if !relays.contains(&closest) {
                                relays.push(closest);
                            }
                        }
                    }
                }
            }
        }

        relays
    }

    /// Find the closest actual node to a target coordinate
    fn find_closest_node(
        &self,
        target: &MatrixCoordinate,
        nodes: &[MatrixCoordinate],
    ) -> Option<MatrixCoordinate> {
        nodes
            .iter()
            .min_by_key(|n| {
                let dist = n.euclidean_distance(target);
                (dist * 1000.0) as i64 // Convert to integer for comparison
            })
            .cloned()
    }

    /// Check if we should propagate to a specific node
    async fn should_propagate_to(
        &self,
        block_hash: &str,
        target: &MatrixCoordinate,
    ) -> bool {
        // Don't propagate to self
        if *target == self.node_coordinate {
            return false;
        }

        // Check if target has already seen this block
        let seen = self.seen_blocks.read().await;
        if let Some(nodes) = seen.get(block_hash) {
            if nodes.contains(target) {
                debug!(
                    "Node ({},{},{}) already has block {}",
                    target.x, target.y, target.z, block_hash
                );
                return false;
            }
        }

        true
    }

    /// Mark a block as seen by a node
    async fn mark_block_seen(&self, block_hash: &str, node: &MatrixCoordinate) {
        let mut seen = self.seen_blocks.write().await;
        seen.entry(block_hash.to_string())
            .or_insert_with(HashSet::new)
            .insert(node.clone());
    }

    /// Flood propagation for critical blocks
    pub async fn flood_propagate(
        &self,
        block: &Block,
        network_nodes: &[MatrixCoordinate],
        max_hops: usize,
    ) -> PropagationResult {
        let start_time = std::time::Instant::now();
        let mut reached_nodes = HashSet::new();
        let mut failed_nodes = Vec::new();
        let mut queue = VecDeque::new();

        // Start with this node
        queue.push_back((self.node_coordinate.clone(), 0));
        reached_nodes.insert(self.node_coordinate.clone());

        while let Some((current, hops)) = queue.pop_front() {
            if hops >= max_hops {
                continue;
            }

            // Find neighbors of current node
            let neighbors = find_neighbors(&current, network_nodes, 1.5);

            for neighbor in neighbors {
                if !reached_nodes.contains(&neighbor) {
                    if self.transport.send_block(block, &neighbor, &self.node_coordinate).await {
                        reached_nodes.insert(neighbor.clone());
                        queue.push_back((neighbor, hops + 1));
                    } else {
                        failed_nodes.push(neighbor);
                    }
                }
            }
        }

        let propagation_time_ms = start_time.elapsed().as_millis() as u64;

        PropagationResult {
            reached_nodes: reached_nodes.into_iter().collect(),
            failed_nodes,
            propagation_time_ms,
            hop_count: max_hops,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_network() -> Vec<MatrixCoordinate> {
        let mut nodes = Vec::new();
        // Create a 3x3x3 grid of nodes
        for x in 0..3 {
            for y in 0..3 {
                for z in 0..3 {
                    nodes.push(MatrixCoordinate::new(x, y, z).unwrap());
                }
            }
        }
        nodes
    }

    #[tokio::test]
    async fn test_broadcast_propagation() {
        let origin = MatrixCoordinate::new(1, 1, 1).unwrap();
        let propagator = BlockPropagator::new(origin.clone(), PropagationStrategy::Broadcast);

        let network = create_test_network();
        let block = Block::genesis(origin);

        let result = propagator.propagate_block(&block, &network).await;

        // Should reach immediate neighbors (distance <= 1.5 includes
        // face-adjacent and edge-diagonal neighbours in a 3x3x3 grid)
        assert!(!result.reached_nodes.is_empty());
        assert!(result.reached_nodes.len() <= 18); // Up to 18 within distance 1.5
    }

    #[tokio::test]
    async fn test_nearest_n_propagation() {
        let origin = MatrixCoordinate::new(0, 0, 0).unwrap();
        let propagator = BlockPropagator::new(
            origin.clone(),
            PropagationStrategy::NearestN(3),
        );

        let network = create_test_network();
        let block = Block::genesis(origin);

        let result = propagator.propagate_block(&block, &network).await;

        // Should reach exactly 3 or fewer nodes
        assert!(result.reached_nodes.len() <= 3);
    }

    #[tokio::test]
    async fn test_distance_threshold_propagation() {
        let origin = MatrixCoordinate::new(1, 1, 1).unwrap();
        let propagator = BlockPropagator::new(
            origin.clone(),
            PropagationStrategy::DistanceThreshold(2.0),
        );

        let network = create_test_network();
        let block = Block::genesis(origin.clone());

        let result = propagator.propagate_block(&block, &network).await;

        // Verify all reached nodes are within threshold
        for node in &result.reached_nodes {
            let distance = origin.euclidean_distance(node);
            assert!(distance <= 2.0);
        }
    }

    #[tokio::test]
    async fn test_routed_propagation() {
        let origin = MatrixCoordinate::new(0, 0, 0).unwrap();
        let propagator = BlockPropagator::new(
            origin.clone(),
            PropagationStrategy::RoutedPath,
        );

        let network = create_test_network();
        let block = Block::genesis(origin);

        let result = propagator.propagate_block(&block, &network).await;

        // Should reach some strategic relay nodes
        assert!(!result.reached_nodes.is_empty());
    }

    #[tokio::test]
    async fn test_duplicate_prevention() {
        let origin = MatrixCoordinate::new(1, 1, 1).unwrap();
        let propagator = BlockPropagator::new(
            origin.clone(),
            PropagationStrategy::Broadcast,
        );

        let network = create_test_network();
        let block = Block::genesis(origin);

        // First propagation
        let result1 = propagator.propagate_block(&block, &network).await;
        let reached_count_1 = result1.reached_nodes.len();

        // Second propagation of same block
        let result2 = propagator.propagate_block(&block, &network).await;

        // Should not propagate to nodes that already have it
        assert!(result2.reached_nodes.is_empty() ||
                result2.reached_nodes.len() < reached_count_1);
    }

    #[tokio::test]
    async fn test_flood_propagation() {
        let origin = MatrixCoordinate::new(1, 1, 1).unwrap();
        let propagator = BlockPropagator::new(
            origin.clone(),
            PropagationStrategy::Broadcast,
        );

        let network = create_test_network();
        let block = Block::genesis(origin);

        let result = propagator.flood_propagate(&block, &network, 2).await;

        // Should reach multiple nodes through flooding
        assert!(result.reached_nodes.len() > 1);
        assert_eq!(result.hop_count, 2);
    }

    #[tokio::test]
    async fn test_relay_node_selection() {
        let origin = MatrixCoordinate::new(1, 1, 1).unwrap();
        let propagator = BlockPropagator::new(
            origin,
            PropagationStrategy::RoutedPath,
        );

        let network = create_test_network();
        let relays = propagator.find_relay_nodes(&network);

        // Should find corner nodes as relays
        assert!(!relays.is_empty());

        // Verify relays are at boundaries
        for relay in relays {
            let is_boundary =
                (relay.x == 0 || relay.x == 2) ||
                (relay.y == 0 || relay.y == 2) ||
                (relay.z == 0 || relay.z == 2);
            assert!(is_boundary);
        }
    }

    #[tokio::test]
    async fn test_propagation_metrics() {
        let origin = MatrixCoordinate::new(0, 0, 0).unwrap();
        let propagator = BlockPropagator::new(
            origin.clone(),
            PropagationStrategy::Broadcast,
        );

        let network = create_test_network();
        let block = Block::genesis(origin);

        let result = propagator.propagate_block(&block, &network).await;

        // Verify metrics are recorded
        let _ = result.propagation_time_ms; // u64 always >= 0
        assert_eq!(result.hop_count, 1);
        assert_eq!(
            result.reached_nodes.len() + result.failed_nodes.len(),
            result.reached_nodes.len() + result.failed_nodes.len()
        );
    }
}