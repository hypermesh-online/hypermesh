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
use tracing::{debug, info, warn};

use super::block::Block;
use crate::bootstrap::PrivacyMode;
use crate::matrix::coordinate::MatrixCoordinate;
use crate::matrix::neighbors::{find_k_nearest, find_neighbors};
use crate::network::hash_bucket::SpatialBucketAssigner;
use crate::network::peer_auth::AuthenticatedPeers;
use crate::network::reflector_pool::ReflectorPool;
use crate::network::SwarmDemandTracker;
use hypermesh_lib::BlockchainScope;

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
    /// Torrent model: propagate only to content-interested peers.
    /// Device=none, Private=full replication, Public=reflectors+spatial+consumers, Anonymous=consumers.
    ContentInterested,
}

/// Context for content-interested propagation decisions.
///
/// Provides the network state needed by `ContentInterested` strategy to
/// determine which peers should receive a block based on scope, privacy
/// mode, and active content interest.
pub struct InterestContext {
    /// Which nodes are fetching which shards (consumer interest).
    pub swarm_demand: Arc<SwarmDemandTracker>,
    /// Healthy reflectors by network.
    pub reflector_pool: Arc<tokio::sync::Mutex<ReflectorPool>>,
    /// Authenticated peers with network_id.
    pub authenticated_peers: AuthenticatedPeers,
    /// Our network ID.
    pub network_id: String,
    /// Our blockchain scope.
    pub blockchain_scope: BlockchainScope,
    /// Our privacy mode.
    pub privacy_mode: PrivacyMode,
    /// Spatial bucket assigner (for Public mode send-side filtering).
    pub spatial_assigner: Option<Arc<RwLock<SpatialBucketAssigner>>>,
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

/// A per-node weight modifier for prioritizing block propagation targets.
///
/// Supplied externally by ngauge's `RoutingAdvisor` (when the `intelligence`
/// feature is enabled). Positive factors increase propagation priority;
/// values below 1.0 reduce it; zero means skip the node entirely.
#[derive(Debug, Clone)]
pub struct PropagationWeight {
    /// Matrix coordinate this weight applies to.
    pub coordinate: MatrixCoordinate,
    /// Multiplicative factor (1.0 = neutral, >1 = prefer, <1 = avoid, 0 = skip).
    pub weight: f64,
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
    /// Optional per-node weight modifiers from ngauge routing intelligence.
    /// When set, `propagate_block` sorts targets by weight (highest first)
    /// and skips nodes with weight <= 0.
    propagation_weights: Arc<RwLock<Vec<PropagationWeight>>>,
    /// Optional context for content-interested propagation.
    /// When set and strategy is `ContentInterested`, targets are selected
    /// based on scope, privacy mode, and active content interest.
    interest_context: Option<Arc<InterestContext>>,
}

use std::collections::HashMap;

impl BlockPropagator {
    /// Create a new block propagator with the default simulated transport.
    pub fn new(node_coordinate: MatrixCoordinate, strategy: PropagationStrategy) -> Self {
        Self::with_transport(node_coordinate, strategy, Arc::new(SimulatedTransport))
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
            propagation_weights: Arc::new(RwLock::new(Vec::new())),
            interest_context: None,
        }
    }

    /// Set the interest context for content-interested propagation.
    ///
    /// When the strategy is `ContentInterested`, this context determines
    /// target selection based on blockchain scope, privacy mode, and
    /// active consumer demand.
    pub fn set_interest_context(&mut self, ctx: Arc<InterestContext>) {
        self.interest_context = Some(ctx);
    }

    /// Update the propagation weight modifiers (typically called by ngauge
    /// routing intelligence when new metrics arrive).
    pub async fn set_propagation_weights(&self, weights: Vec<PropagationWeight>) {
        *self.propagation_weights.write().await = weights;
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
        let mut targets = match (&self.strategy, self.interest_context.as_ref()) {
            (PropagationStrategy::ContentInterested, Some(ctx)) => {
                self.select_interested_targets(block, network_nodes, ctx)
                    .await
            }
            // Includes the existing NearestN(6) fallback without context.
            _ => self.select_propagation_targets(network_nodes).await,
        };

        // Apply ngauge routing weights: filter out zero-weight nodes, sort by weight descending.
        targets = self.apply_weights(targets).await;

        self.log_propagation_targets(block, targets.len());

        // Mark this block as seen by this node
        self.mark_block_seen(&block.hash, &self.node_coordinate)
            .await;

        // Propagate to each target via the wired transport
        for target in &targets {
            self.propagate_to_target(block, target, &mut reached_nodes, &mut failed_nodes)
                .await;
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

            PropagationStrategy::ContentInterested => {
                // ContentInterested is handled in propagate_block() before this
                // method is called. If we reach here, fall back to NearestN(6).
                find_k_nearest(&self.node_coordinate, network_nodes, 6)
                    .into_iter()
                    .map(|(coord, _distance)| coord)
                    .collect()
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
            // Find nodes close to the relay (within distance 2)
            for node in network_nodes {
                let dist_to_relay = node.euclidean_distance(&relay);
                if dist_to_relay <= 2.0 && !targets.contains(node) && *node != self.node_coordinate
                {
                    targets.push(*node);
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
    async fn should_propagate_to(&self, block_hash: &str, target: &MatrixCoordinate) -> bool {
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
            .insert(*node);
    }

    /// Apply ngauge routing weights to propagation targets.
    ///
    /// Filters out targets with weight <= 0 and sorts remaining targets
    /// by weight descending (highest priority first). Targets without
    /// a matching weight entry keep the default weight of 1.0.
    async fn apply_weights(&self, targets: Vec<MatrixCoordinate>) -> Vec<MatrixCoordinate> {
        let weights = self.propagation_weights.read().await;
        if weights.is_empty() {
            return targets;
        }

        let mut weighted: Vec<(MatrixCoordinate, f64)> = targets
            .into_iter()
            .map(|coord| {
                let w = weights
                    .iter()
                    .find(|pw| pw.coordinate == coord)
                    .map(|pw| pw.weight)
                    .unwrap_or(1.0);
                (coord, w)
            })
            .filter(|(_, w)| *w > 0.0)
            .collect();

        weighted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        weighted.into_iter().map(|(coord, _)| coord).collect()
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
        queue.push_back((self.node_coordinate, 0));
        reached_nodes.insert(self.node_coordinate);

        while let Some((current, hops)) = queue.pop_front() {
            if hops >= max_hops {
                continue;
            }

            // Find neighbors of current node
            let neighbors = find_neighbors(&current, network_nodes, 1.5);

            for neighbor in neighbors {
                if !reached_nodes.contains(&neighbor) {
                    if self
                        .transport
                        .send_block(block, &neighbor, &self.node_coordinate)
                        .await
                    {
                        reached_nodes.insert(neighbor);
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

mod delivery;
mod interest;
#[cfg(test)]
mod tests;
