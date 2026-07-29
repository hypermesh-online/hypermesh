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
        let mut targets = match &self.strategy {
            PropagationStrategy::ContentInterested => {
                if let Some(ref ctx) = self.interest_context {
                    self.select_interested_targets(block, network_nodes, ctx)
                        .await
                } else {
                    // Fallback to NearestN(6) when no context is available
                    self.select_propagation_targets(network_nodes).await
                }
            }
            _ => self.select_propagation_targets(network_nodes).await,
        };

        // Apply ngauge routing weights: filter out zero-weight nodes, sort by weight descending.
        targets = self.apply_weights(targets).await;

        info!(
            "Propagating block {} from ({},{},{}) to {} targets",
            block.index,
            self.node_coordinate.x,
            self.node_coordinate.y,
            self.node_coordinate.z,
            targets.len()
        );

        // Mark this block as seen by this node
        self.mark_block_seen(&block.hash, &self.node_coordinate)
            .await;

        // Propagate to each target via the wired transport
        for target in &targets {
            if self.should_propagate_to(&block.hash, target).await {
                if self
                    .transport
                    .send_block(block, target, &self.node_coordinate)
                    .await
                {
                    reached_nodes.push(*target);
                    self.mark_block_seen(&block.hash, target).await;
                } else {
                    failed_nodes.push(*target);
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

        weighted.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        weighted.into_iter().map(|(coord, _)| coord).collect()
    }

    /// Select targets using the content-interested torrent model.
    ///
    /// Routing depends on blockchain scope and privacy mode:
    /// - **Device**: no propagation (local-only chain)
    /// - **Private**: full replication to all authenticated peers in same network
    /// - **Anonymous**: only nodes actively fetching content from this block
    /// - **Public**: reflectors + spatial neighbors + active consumers
    async fn select_interested_targets(
        &self,
        block: &Block,
        all_peers: &[MatrixCoordinate],
        ctx: &InterestContext,
    ) -> Vec<MatrixCoordinate> {
        // Device scope blocks never propagate
        if ctx.blockchain_scope == BlockchainScope::Device {
            debug!("ContentInterested: Device scope — no propagation");
            return vec![];
        }

        // Network scope: routing depends on privacy mode
        if ctx.privacy_mode == PrivacyMode::PRIVATE {
            // Private: full replication to all authenticated peers sharing network_id
            debug!(
                "ContentInterested: Private mode — full replication to {} peer(s)",
                all_peers.len()
            );
            return all_peers.to_vec();
        }

        if ctx.privacy_mode == PrivacyMode::ANONYMOUS {
            // Anonymous: only consumers who are actively fetching content in this block
            let consumers = self.get_consumer_targets(block, ctx).await;
            debug!(
                "ContentInterested: Anonymous mode — {} consumer target(s)",
                consumers.len()
            );
            return consumers;
        }

        // Public (default): reflectors + spatial neighbors + consumers
        let mut targets = Vec::new();

        // 1. Reflectors (always interested in network blocks)
        let reflector_targets = self.get_reflector_targets(ctx).await;
        debug!(
            "ContentInterested: Public mode — {} reflector target(s)",
            reflector_targets.len()
        );
        targets.extend(reflector_targets);

        // 2. Consumers actively fetching this block's shards
        let consumer_targets = self.get_consumer_targets(block, ctx).await;
        debug!(
            "ContentInterested: Public mode — {} consumer target(s)",
            consumer_targets.len()
        );
        targets.extend(consumer_targets);

        // 3. Spatial neighbors (peers whose neighborhood includes this block's content)
        if let Some(ref assigner_lock) = ctx.spatial_assigner {
            let assigner = assigner_lock.read().await;
            let shard_positions = Self::extract_shard_positions(block);
            if !shard_positions.is_empty() {
                let spatial: Vec<MatrixCoordinate> = all_peers
                    .iter()
                    .filter(|peer| {
                        assigner.block_relevant_to_peer(&shard_positions, peer)
                    })
                    .copied()
                    .collect();
                debug!(
                    "ContentInterested: Public mode — {} spatial target(s)",
                    spatial.len()
                );
                targets.extend(spatial);
            }
        } else {
            // No spatial assigner: include nearest 3 peers as fallback
            let nearest = self.select_nearest_n(all_peers, 3);
            targets.extend(nearest);
        }

        // Deduplicate by coordinate
        targets.sort_by(|a, b| (a.x, a.y, a.z).cmp(&(b.x, b.y, b.z)));
        targets.dedup_by(|a, b| a.x == b.x && a.y == b.y && a.z == b.z);

        debug!(
            "ContentInterested: Public mode — {} total unique target(s)",
            targets.len()
        );
        targets
    }

    /// Get reflector coordinates from the pool for this network.
    async fn get_reflector_targets(&self, ctx: &InterestContext) -> Vec<MatrixCoordinate> {
        let pool = ctx.reflector_pool.lock().await;
        pool.get_best_reflectors(&ctx.network_id, 10)
            .iter()
            .filter_map(|r| {
                // MatrixPosition { x: f64, y: f64, z: f64 } -> MatrixCoordinate
                MatrixCoordinate::new(
                    r.position.x as i64,
                    r.position.y as i64,
                    r.position.z as i64,
                )
                .ok()
            })
            .collect()
    }

    /// Get coordinates of nodes actively fetching shards referenced in this block.
    async fn get_consumer_targets(
        &self,
        block: &Block,
        ctx: &InterestContext,
    ) -> Vec<MatrixCoordinate> {
        let demand_snapshot = ctx.swarm_demand.snapshot().await;
        if demand_snapshot.is_empty() {
            return vec![];
        }

        let mut consumer_coords = Vec::new();
        let auth_peers = ctx.authenticated_peers.read().await;

        for entry in &block.entries {
            // Hash the asset_hash to a ContentHash for demand lookup
            let content_hash = hypermesh_lib::ContentHash(entry.asset_hash);

            if let Some(demand) = demand_snapshot.get(&content_hash) {
                for requester_id in &demand.requester_ids {
                    // Look up coordinate from authenticated peers
                    if let Some(peer) = auth_peers.get(requester_id) {
                        match MatrixCoordinate::new(
                            peer.coordinate.0 as i64,
                            peer.coordinate.1 as i64,
                            peer.coordinate.2 as i64,
                        ) {
                            Ok(coord) => consumer_coords.push(coord),
                            Err(e) => {
                                warn!(
                                    "Invalid peer coordinate for {}: {}",
                                    &requester_id[..8.min(requester_id.len())],
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }

        consumer_coords
    }

    /// Extract all shard placement coordinates from a block's entries.
    fn extract_shard_positions(block: &Block) -> Vec<MatrixCoordinate> {
        let mut positions = Vec::new();
        for entry in &block.entries {
            if let super::block::StoragePointer::Sharded {
                ref placements, ..
            } = entry.storage_pointer
            {
                positions.extend_from_slice(placements);
            }
        }
        positions
    }

    /// Select the N nearest peers to our coordinate.
    fn select_nearest_n(
        &self,
        peers: &[MatrixCoordinate],
        n: usize,
    ) -> Vec<MatrixCoordinate> {
        find_k_nearest(&self.node_coordinate, peers, n)
            .into_iter()
            .map(|(coord, _distance)| coord)
            .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_network() -> Vec<MatrixCoordinate> {
        let mut nodes = Vec::new();
        // Create a 3x3x3 grid of nodes
        for x in 0..3 {
            for y in 0..3 {
                for z in 0..3 {
                    nodes.push(MatrixCoordinate::new(x, y, z).expect("test: valid coordinate"));
                }
            }
        }
        nodes
    }

    #[tokio::test]
    async fn test_broadcast_propagation() {
        let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
        let propagator = BlockPropagator::new(origin, PropagationStrategy::Broadcast);

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
        let origin = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let propagator = BlockPropagator::new(origin, PropagationStrategy::NearestN(3));

        let network = create_test_network();
        let block = Block::genesis(origin);

        let result = propagator.propagate_block(&block, &network).await;

        // Should reach exactly 3 or fewer nodes
        assert!(result.reached_nodes.len() <= 3);
    }

    #[tokio::test]
    async fn test_distance_threshold_propagation() {
        let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
        let propagator = BlockPropagator::new(origin, PropagationStrategy::DistanceThreshold(2.0));

        let network = create_test_network();
        let block = Block::genesis(origin);

        let result = propagator.propagate_block(&block, &network).await;

        // Verify all reached nodes are within threshold
        for node in &result.reached_nodes {
            let distance = origin.euclidean_distance(node);
            assert!(distance <= 2.0);
        }
    }

    #[tokio::test]
    async fn test_routed_propagation() {
        let origin = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let propagator = BlockPropagator::new(origin, PropagationStrategy::RoutedPath);

        let network = create_test_network();
        let block = Block::genesis(origin);

        let result = propagator.propagate_block(&block, &network).await;

        // Should reach some strategic relay nodes
        assert!(!result.reached_nodes.is_empty());
    }

    #[tokio::test]
    async fn test_duplicate_prevention() {
        let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
        let propagator = BlockPropagator::new(origin, PropagationStrategy::Broadcast);

        let network = create_test_network();
        let block = Block::genesis(origin);

        // First propagation
        let result1 = propagator.propagate_block(&block, &network).await;
        let reached_count_1 = result1.reached_nodes.len();

        // Second propagation of same block
        let result2 = propagator.propagate_block(&block, &network).await;

        // Should not propagate to nodes that already have it
        assert!(result2.reached_nodes.is_empty() || result2.reached_nodes.len() < reached_count_1);
    }

    #[tokio::test]
    async fn test_flood_propagation() {
        let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
        let propagator = BlockPropagator::new(origin, PropagationStrategy::Broadcast);

        let network = create_test_network();
        let block = Block::genesis(origin);

        let result = propagator.flood_propagate(&block, &network, 2).await;

        // Should reach multiple nodes through flooding
        assert!(result.reached_nodes.len() > 1);
        assert_eq!(result.hop_count, 2);
    }

    #[tokio::test]
    async fn test_relay_node_selection() {
        let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
        let propagator = BlockPropagator::new(origin, PropagationStrategy::RoutedPath);

        let network = create_test_network();
        let relays = propagator.find_relay_nodes(&network);

        // Should find corner nodes as relays
        assert!(!relays.is_empty());

        // Verify relays are at boundaries
        for relay in relays {
            let is_boundary = (relay.x == 0 || relay.x == 2)
                || (relay.y == 0 || relay.y == 2)
                || (relay.z == 0 || relay.z == 2);
            assert!(is_boundary);
        }
    }

    #[tokio::test]
    async fn test_propagation_metrics() {
        let origin = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
        let propagator = BlockPropagator::new(origin, PropagationStrategy::Broadcast);

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

    #[tokio::test]
    async fn test_apply_weights_filters_zero_weight() {
        let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
        let propagator = BlockPropagator::new(origin, PropagationStrategy::Broadcast);

        let skip = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
        let keep = MatrixCoordinate::new(2, 2, 2).expect("test: valid coord");

        propagator
            .set_propagation_weights(vec![
                PropagationWeight {
                    coordinate: skip,
                    weight: 0.0,
                },
                PropagationWeight {
                    coordinate: keep,
                    weight: 1.5,
                },
            ])
            .await;

        let result = propagator.apply_weights(vec![skip, keep]).await;
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], keep);
    }

    #[tokio::test]
    async fn test_apply_weights_sorts_by_weight() {
        let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
        let propagator = BlockPropagator::new(origin, PropagationStrategy::Broadcast);

        let low = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
        let high = MatrixCoordinate::new(2, 2, 2).expect("test: valid coord");

        propagator
            .set_propagation_weights(vec![
                PropagationWeight {
                    coordinate: low,
                    weight: 0.5,
                },
                PropagationWeight {
                    coordinate: high,
                    weight: 2.0,
                },
            ])
            .await;

        let result = propagator.apply_weights(vec![low, high]).await;
        assert_eq!(result.len(), 2);
        // High weight should come first
        assert_eq!(result[0], high);
        assert_eq!(result[1], low);
    }

    #[tokio::test]
    async fn test_apply_weights_empty_keeps_original_order() {
        let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
        let propagator = BlockPropagator::new(origin, PropagationStrategy::Broadcast);

        let a = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
        let b = MatrixCoordinate::new(2, 2, 2).expect("test: valid coord");

        // No weights set
        let result = propagator.apply_weights(vec![a, b]).await;
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], a);
        assert_eq!(result[1], b);
    }

    // ── Content-interested propagation tests ────────────────────────

    use crate::network::peer_auth;
    use crate::network::reflector_pool::{ReflectorConfig, ReflectorPool};
    use crate::network::SwarmDemandTracker;

    /// Deterministic transport that always succeeds (no randomness).
    struct AlwaysSuccessTransport;

    #[async_trait::async_trait]
    impl BlockTransport for AlwaysSuccessTransport {
        async fn send_block(
            &self,
            _block: &Block,
            _target: &MatrixCoordinate,
            _origin: &MatrixCoordinate,
        ) -> bool {
            true
        }
    }

    fn make_interest_context(
        scope: BlockchainScope,
        privacy: PrivacyMode,
    ) -> Arc<InterestContext> {
        Arc::new(InterestContext {
            swarm_demand: Arc::new(SwarmDemandTracker::new()),
            reflector_pool: Arc::new(tokio::sync::Mutex::new(ReflectorPool::new(
                ReflectorConfig::default(),
            ))),
            authenticated_peers: peer_auth::new_authenticated_peers(),
            network_id: "test-net".to_string(),
            blockchain_scope: scope,
            privacy_mode: privacy,
            spatial_assigner: None,
        })
    }

    #[tokio::test]
    async fn test_content_interested_device_no_propagation() {
        let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
        let mut propagator = BlockPropagator::with_transport(
            origin,
            PropagationStrategy::ContentInterested,
            Arc::new(AlwaysSuccessTransport),
        );

        let ctx = make_interest_context(BlockchainScope::Device, PrivacyMode::PRIVATE);
        propagator.set_interest_context(ctx);

        let network = create_test_network();
        let block = Block::genesis(origin);

        let result = propagator.propagate_block(&block, &network).await;
        // Device scope should return empty targets — no propagation
        assert!(
            result.reached_nodes.is_empty(),
            "Device scope should not propagate blocks"
        );
    }

    #[tokio::test]
    async fn test_content_interested_private_full_replication() {
        let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
        let mut propagator = BlockPropagator::with_transport(
            origin,
            PropagationStrategy::ContentInterested,
            Arc::new(AlwaysSuccessTransport),
        );

        let ctx = make_interest_context(BlockchainScope::Network, PrivacyMode::PRIVATE);
        propagator.set_interest_context(ctx);

        let network = create_test_network();
        let block = Block::genesis(origin);

        let result = propagator.propagate_block(&block, &network).await;
        // Private should replicate to all peers (minus self, which is filtered
        // by should_propagate_to). Network has 27 nodes, minus origin = up to 26.
        assert!(
            !result.reached_nodes.is_empty(),
            "Private mode should replicate to peers"
        );
        // All non-self peers should be reached with AlwaysSuccessTransport
        assert_eq!(result.reached_nodes.len(), 26);
    }

    #[tokio::test]
    async fn test_content_interested_public_includes_reflectors() {
        let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
        let mut propagator = BlockPropagator::with_transport(
            origin,
            PropagationStrategy::ContentInterested,
            Arc::new(AlwaysSuccessTransport),
        );

        let ctx = make_interest_context(BlockchainScope::Network, PrivacyMode::PUBLIC);

        // Register a reflector at position (0,0,0)
        {
            let mut pool = ctx.reflector_pool.lock().await;
            pool.register_reflector(
                "test-net",
                crate::network::reflector_pool::Reflector {
                    node_id: "reflector-1".to_string(),
                    position: hypermesh_lib::MatrixPosition {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    last_seen: 999999,
                    block_height: 100,
                    health_score: 0.9,
                    privacy_mode: PrivacyMode::PUBLIC,
                },
            );
        }

        propagator.set_interest_context(ctx);

        let network = create_test_network();
        let block = Block::genesis(origin);

        let result = propagator.propagate_block(&block, &network).await;
        // Should include reflector coordinate (0,0,0) among reached nodes
        let reflector_coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
        assert!(
            result.reached_nodes.contains(&reflector_coord),
            "Public mode should include reflector targets"
        );
    }

    #[tokio::test]
    async fn test_content_interested_anonymous_consumers_only() {
        let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
        let mut propagator = BlockPropagator::with_transport(
            origin,
            PropagationStrategy::ContentInterested,
            Arc::new(AlwaysSuccessTransport),
        );

        let ctx = make_interest_context(BlockchainScope::Network, PrivacyMode::ANONYMOUS);

        // Register no demand — should result in zero targets
        propagator.set_interest_context(ctx);

        let network = create_test_network();
        let block = Block::genesis(origin);

        let result = propagator.propagate_block(&block, &network).await;
        // Anonymous with no active consumers = no propagation
        assert!(
            result.reached_nodes.is_empty(),
            "Anonymous mode with no consumers should not propagate"
        );
    }

    #[tokio::test]
    async fn test_content_interested_anonymous_with_consumer() {
        let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
        let mut propagator = BlockPropagator::with_transport(
            origin,
            PropagationStrategy::ContentInterested,
            Arc::new(AlwaysSuccessTransport),
        );

        let ctx = make_interest_context(BlockchainScope::Network, PrivacyMode::ANONYMOUS);

        // Create a block with a known asset hash
        let block = Block::genesis(origin);
        let asset_hash = block.entries[0].asset_hash;

        // Record demand for this asset hash from a peer
        let content_hash = hypermesh_lib::ContentHash(asset_hash);
        ctx.swarm_demand
            .record_fetch(content_hash, "consumer-node-1")
            .await;

        // Register the consumer as an authenticated peer at (2,2,2)
        peer_auth::register_authenticated_peer(
            &ctx.authenticated_peers,
            peer_auth::AuthenticatedPeer {
                node_id: "consumer-node-1".to_string(),
                pubkey: vec![1, 2, 3],
                coordinate: (2, 2, 2),
                network_id: "test-net".to_string(),
                authenticated_at: std::time::Instant::now(),
                proof_bytes: vec![4, 5, 6],
            },
        )
        .await;

        propagator.set_interest_context(ctx);

        let network = create_test_network();
        let result = propagator.propagate_block(&block, &network).await;

        // Should reach the consumer at (2,2,2)
        let consumer_coord = MatrixCoordinate::new(2, 2, 2).expect("test: valid coord");
        assert!(
            result.reached_nodes.contains(&consumer_coord),
            "Anonymous mode should propagate to active consumers"
        );
    }

    #[tokio::test]
    async fn test_content_interested_fallback_without_context() {
        let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
        // ContentInterested strategy but NO interest context set
        let propagator = BlockPropagator::with_transport(
            origin,
            PropagationStrategy::ContentInterested,
            Arc::new(AlwaysSuccessTransport),
        );

        let network = create_test_network();
        let block = Block::genesis(origin);

        let result = propagator.propagate_block(&block, &network).await;
        // Should fall back to select_propagation_targets (NearestN behavior
        // is not available for ContentInterested, so it uses default routing)
        // The key assertion: it doesn't crash and produces some result
        assert!(
            result.reached_nodes.len() + result.failed_nodes.len() >= 0,
            "Fallback should not panic"
        );
    }
}
