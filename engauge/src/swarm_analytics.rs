// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Swarm analytics for shard popularity tracking, replication triggers,
//! dispersion intelligence, and cascade metrics (R12/R13).
//!
//! # Modules
//!
//! - [`ShardPopularity`] tracks per-shard request frequency and consumer distribution.
//! - [`ReplicationTrigger`] detects when replicas are insufficient for demand.
//! - [`DispersionAdvisor`] recommends optimal placement for new replicas.
//! - [`CascadeMetrics`] measures cascade propagation performance.

use std::collections::{HashMap, HashSet};

use hypermesh_lib::{ContentHash, MatrixPosition, NodeId};

// ---------------------------------------------------------------------------
// ShardPopularity (Item 7.4)
// ---------------------------------------------------------------------------

/// Per-shard popularity data.
#[derive(Debug, Clone)]
pub struct ShardPopularity {
    /// Total request count for this shard.
    pub request_count: u64,
    /// Number of unique consumers that requested this shard.
    pub unique_consumers: usize,
    /// Unix microsecond timestamp of last access.
    pub last_access_us: u64,
    /// Matrix positions of consumers that requested this shard.
    consumer_positions: Vec<MatrixPosition>,
}

impl ShardPopularity {
    fn new() -> Self {
        Self {
            request_count: 0,
            unique_consumers: 0,
            last_access_us: 0,
            consumer_positions: Vec::new(),
        }
    }
}

/// Swarm-level analytics for shard demand tracking.
pub struct SwarmAnalytics {
    /// Per-shard popularity data.
    shard_data: HashMap<ContentHash, ShardPopularity>,
    /// Per-shard set of unique consumer node IDs.
    shard_consumers: HashMap<ContentHash, HashSet<NodeId>>,
    /// Per-shard replica count (set externally by BlockMatrix).
    replica_counts: HashMap<ContentHash, u32>,
}

impl SwarmAnalytics {
    /// Create a new swarm analytics tracker.
    pub fn new() -> Self {
        Self {
            shard_data: HashMap::new(),
            shard_consumers: HashMap::new(),
            replica_counts: HashMap::new(),
        }
    }

    /// Record a request for a shard from a consumer at a matrix position.
    pub fn record_request(
        &mut self,
        shard_id: ContentHash,
        consumer_id: NodeId,
        consumer_pos: MatrixPosition,
        timestamp_us: u64,
    ) {
        let entry = self.shard_data.entry(shard_id).or_insert_with(ShardPopularity::new);
        entry.request_count += 1;
        entry.last_access_us = timestamp_us;
        entry.consumer_positions.push(consumer_pos);

        let consumers = self.shard_consumers.entry(shard_id).or_default();
        consumers.insert(consumer_id);
        entry.unique_consumers = consumers.len();
    }

    /// Get popularity data for a specific shard.
    pub fn get_popularity(&self, shard_id: &ContentHash) -> Option<&ShardPopularity> {
        self.shard_data.get(shard_id)
    }

    /// Get all shards exceeding a request count threshold, sorted by popularity.
    pub fn get_popular_shards(&self, min_request_count: u64) -> Vec<(ContentHash, &ShardPopularity)> {
        let mut popular: Vec<_> = self
            .shard_data
            .iter()
            .filter(|(_, pop)| pop.request_count >= min_request_count)
            .map(|(hash, pop)| (*hash, pop))
            .collect();
        popular.sort_by(|a, b| b.1.request_count.cmp(&a.1.request_count));
        popular
    }

    /// Get the demand map for a shard (consumer matrix positions).
    pub fn get_demand_map(&self, shard_id: &ContentHash) -> Option<&[MatrixPosition]> {
        self.shard_data
            .get(shard_id)
            .map(|pop| pop.consumer_positions.as_slice())
    }

    /// Set the replica count for a shard (called externally by BlockMatrix).
    pub fn set_replica_count(&mut self, shard_id: ContentHash, count: u32) {
        self.replica_counts.insert(shard_id, count);
    }

    /// Get the replica count for a shard.
    pub fn get_replica_count(&self, shard_id: &ContentHash) -> u32 {
        self.replica_counts.get(shard_id).copied().unwrap_or(0)
    }
}

impl Default for SwarmAnalytics {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// ReplicationTrigger (Item 7.5)
// ---------------------------------------------------------------------------

/// Signal emitted when a shard needs more replicas.
#[derive(Debug, Clone)]
pub struct ReplicationSignal {
    /// The shard that needs replication.
    pub shard_id: ContentHash,
    /// Urgency level (higher = more urgent).
    pub urgency: f64,
    /// Suggested new replica count.
    pub suggested_count: u32,
    /// Current request rate (requests per check interval).
    pub current_request_rate: u64,
    /// Current replica count.
    pub current_replicas: u32,
}

/// Configuration for replication triggers.
#[derive(Debug, Clone)]
pub struct ReplicationConfig {
    /// Requests per interval that trigger replication per replica.
    pub requests_per_replica_threshold: u64,
    /// Maximum replicas to suggest.
    pub max_replicas: u32,
    /// Minimum replicas below which urgency is high.
    pub min_replicas: u32,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            requests_per_replica_threshold: 100,
            max_replicas: 20,
            min_replicas: 3,
        }
    }
}

/// Monitors shard demand vs replica count and emits replication signals.
pub struct ReplicationTrigger {
    config: ReplicationConfig,
}

impl ReplicationTrigger {
    /// Create a new replication trigger with the given configuration.
    pub fn new(config: ReplicationConfig) -> Self {
        Self { config }
    }

    /// Check all tracked shards and return signals for those needing replication.
    pub fn check(&self, analytics: &SwarmAnalytics) -> Vec<ReplicationSignal> {
        let mut signals = Vec::new();

        for (shard_id, popularity) in &analytics.shard_data {
            let replicas = analytics.get_replica_count(shard_id);
            let rate = popularity.request_count;
            let threshold = self.config.requests_per_replica_threshold;

            // Demand exceeds what current replicas can serve.
            let needed = if threshold > 0 {
                ((rate as f64 / threshold as f64).ceil() as u32).max(self.config.min_replicas)
            } else {
                self.config.min_replicas
            };

            if needed > replicas {
                let suggested = needed.min(self.config.max_replicas);
                let urgency = if replicas < self.config.min_replicas {
                    1.0 // Critical: below minimum
                } else {
                    (needed as f64 - replicas as f64) / needed as f64
                };

                signals.push(ReplicationSignal {
                    shard_id: *shard_id,
                    urgency,
                    suggested_count: suggested,
                    current_request_rate: rate,
                    current_replicas: replicas,
                });
            }
        }

        // Sort by urgency descending.
        signals.sort_by(|a, b| b.urgency.partial_cmp(&a.urgency).unwrap_or(std::cmp::Ordering::Equal));
        signals
    }
}

// ---------------------------------------------------------------------------
// DispersionAdvisor (Item 7.6)
// ---------------------------------------------------------------------------

/// Recommends optimal placement for new shard replicas based on
/// consumer demand distribution in the matrix.
pub struct DispersionAdvisor {
    /// Existing replica positions to avoid clustering.
    existing_replicas: HashMap<ContentHash, Vec<MatrixPosition>>,
}

impl DispersionAdvisor {
    /// Create a new dispersion advisor.
    pub fn new() -> Self {
        Self {
            existing_replicas: HashMap::new(),
        }
    }

    /// Register an existing replica position for a shard.
    pub fn register_replica(&mut self, shard_id: ContentHash, position: MatrixPosition) {
        self.existing_replicas
            .entry(shard_id)
            .or_default()
            .push(position);
    }

    /// Recommend placement positions for new replicas of a shard.
    ///
    /// Uses geographic clustering of consumers to find optimal positions,
    /// then filters out positions too close to existing replicas.
    pub fn recommend_placement(
        &self,
        shard_id: &ContentHash,
        analytics: &SwarmAnalytics,
        count: usize,
    ) -> Vec<MatrixPosition> {
        let demand_map = match analytics.get_demand_map(shard_id) {
            Some(positions) if !positions.is_empty() => positions,
            _ => return Vec::new(),
        };

        let existing = self.existing_replicas.get(shard_id);

        // Compute demand centroid clusters using simple grid-based clustering.
        let clusters = cluster_positions(demand_map, count.max(1));

        // Filter out cluster centroids that are too close to existing replicas.
        let min_distance = 2.0; // Minimum matrix distance between replicas.
        let mut recommended: Vec<MatrixPosition> = Vec::new();

        for centroid in clusters {
            let too_close = existing
                .map(|existing_pos| {
                    existing_pos
                        .iter()
                        .any(|ep| matrix_distance(&centroid, ep) < min_distance)
                })
                .unwrap_or(false);

            if !too_close {
                recommended.push(centroid);
            }
            if recommended.len() >= count {
                break;
            }
        }

        recommended
    }
}

impl Default for DispersionAdvisor {
    fn default() -> Self {
        Self::new()
    }
}

/// Euclidean distance between two matrix positions.
fn matrix_distance(a: &MatrixPosition, b: &MatrixPosition) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Simple k-means-style clustering of matrix positions.
///
/// Returns up to `k` centroid positions.
fn cluster_positions(positions: &[MatrixPosition], k: usize) -> Vec<MatrixPosition> {
    if positions.is_empty() {
        return Vec::new();
    }
    if positions.len() <= k {
        return positions.to_vec();
    }

    // Initialize centroids by evenly sampling from positions.
    let step = positions.len() / k;
    let mut centroids: Vec<MatrixPosition> = (0..k)
        .map(|i| positions[i * step])
        .collect();

    // Run a few iterations of k-means.
    for _ in 0..10 {
        // Assign each position to nearest centroid.
        let mut groups: Vec<Vec<&MatrixPosition>> = vec![Vec::new(); k];
        for pos in positions {
            let nearest = centroids
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    matrix_distance(pos, a)
                        .partial_cmp(&matrix_distance(pos, b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(idx, _)| idx)
                .unwrap_or(0);
            groups[nearest].push(pos);
        }

        // Recompute centroids.
        for (i, group) in groups.iter().enumerate() {
            if group.is_empty() {
                continue;
            }
            let count = group.len() as f64;
            let sx: f64 = group.iter().map(|p| p.x).sum();
            let sy: f64 = group.iter().map(|p| p.y).sum();
            let sz: f64 = group.iter().map(|p| p.z).sum();
            centroids[i] = MatrixPosition {
                x: sx / count,
                y: sy / count,
                z: sz / count,
            };
        }
    }

    centroids
}

// ---------------------------------------------------------------------------
// CascadeMetrics (Item 7.7)
// ---------------------------------------------------------------------------

/// A single cascade hop record.
#[derive(Debug, Clone)]
pub struct CascadeHop {
    /// Shard being relayed.
    pub shard_id: ContentHash,
    /// Node that sent the shard.
    pub from_node: NodeId,
    /// Node that received the shard.
    pub to_node: NodeId,
    /// Latency in milliseconds for this hop.
    pub latency_ms: u64,
    /// Unix microsecond timestamp.
    pub timestamp_us: u64,
}

/// Aggregated cascade statistics for a shard.
#[derive(Debug, Clone)]
pub struct CascadeStats {
    /// Number of hops recorded.
    pub hop_count: usize,
    /// Average latency per hop in milliseconds.
    pub avg_latency_ms: f64,
    /// Total propagation time (sum of all hop latencies) in ms.
    pub total_latency_ms: u64,
    /// Node with the highest relay load (most outbound hops).
    pub bottleneck_node: Option<NodeId>,
    /// Maximum latency of any single hop.
    pub max_hop_latency_ms: u64,
}

/// Tracks cascade propagation performance across the swarm.
pub struct CascadeTracker {
    /// Per-shard cascade hops.
    hops: HashMap<ContentHash, Vec<CascadeHop>>,
    /// Maximum hops to retain per shard.
    max_hops_per_shard: usize,
}

impl CascadeTracker {
    /// Create a new cascade tracker.
    pub fn new(max_hops_per_shard: usize) -> Self {
        Self {
            hops: HashMap::new(),
            max_hops_per_shard,
        }
    }

    /// Record a cascade hop.
    pub fn record_cascade_hop(
        &mut self,
        shard_id: ContentHash,
        from_node: NodeId,
        to_node: NodeId,
        latency_ms: u64,
        timestamp_us: u64,
    ) {
        let hop_list = self.hops.entry(shard_id).or_default();
        hop_list.push(CascadeHop {
            shard_id,
            from_node,
            to_node,
            latency_ms,
            timestamp_us,
        });

        // Trim to max.
        while hop_list.len() > self.max_hops_per_shard {
            hop_list.remove(0);
        }
    }

    /// Get cascade statistics for a shard.
    pub fn get_cascade_stats(&self, shard_id: &ContentHash) -> Option<CascadeStats> {
        let hops = self.hops.get(shard_id)?;
        if hops.is_empty() {
            return None;
        }

        let hop_count = hops.len();
        let total_latency_ms: u64 = hops.iter().map(|h| h.latency_ms).sum();
        let avg_latency_ms = total_latency_ms as f64 / hop_count as f64;
        let max_hop_latency_ms = hops.iter().map(|h| h.latency_ms).max().unwrap_or(0);

        // Find bottleneck: node with most outbound relays.
        let mut relay_counts: HashMap<NodeId, usize> = HashMap::new();
        for hop in hops {
            *relay_counts.entry(hop.from_node).or_insert(0) += 1;
        }
        let bottleneck_node = relay_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(node, _)| node);

        Some(CascadeStats {
            hop_count,
            avg_latency_ms,
            total_latency_ms,
            bottleneck_node,
            max_hop_latency_ms,
        })
    }

    /// Get per-node relay load (outbound hop counts).
    pub fn get_relay_loads(&self) -> HashMap<NodeId, usize> {
        let mut loads: HashMap<NodeId, usize> = HashMap::new();
        for hop_list in self.hops.values() {
            for hop in hop_list {
                *loads.entry(hop.from_node).or_insert(0) += 1;
            }
        }
        loads
    }
}

impl Default for CascadeTracker {
    fn default() -> Self {
        Self::new(1000)
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_hash(val: u8) -> ContentHash {
        ContentHash::from_bytes([val; 32])
    }

    fn test_node(name: &str) -> NodeId {
        NodeId::from_public_key(name.as_bytes())
    }

    fn test_pos(x: f64, y: f64, z: f64) -> MatrixPosition {
        MatrixPosition { x, y, z }
    }

    // -- ShardPopularity / SwarmAnalytics tests (7.4) --

    #[test]
    fn record_and_retrieve_shard_popularity() {
        let mut analytics = SwarmAnalytics::new();
        let shard = test_hash(1);
        let consumer_a = test_node("consumer-a");
        let consumer_b = test_node("consumer-b");

        analytics.record_request(shard, consumer_a, test_pos(1.0, 0.0, 0.0), 1000);
        analytics.record_request(shard, consumer_a, test_pos(1.0, 0.0, 0.0), 2000);
        analytics.record_request(shard, consumer_b, test_pos(5.0, 0.0, 0.0), 3000);

        let pop = analytics.get_popularity(&shard).expect("test: popularity should exist");
        assert_eq!(pop.request_count, 3);
        assert_eq!(pop.unique_consumers, 2);
        assert_eq!(pop.last_access_us, 3000);
    }

    #[test]
    fn get_popular_shards_with_threshold() {
        let mut analytics = SwarmAnalytics::new();
        let hot = test_hash(1);
        let cold = test_hash(2);
        let node = test_node("consumer");

        for i in 0..10 {
            analytics.record_request(hot, node, test_pos(0.0, 0.0, 0.0), i);
        }
        analytics.record_request(cold, node, test_pos(0.0, 0.0, 0.0), 0);

        let popular = analytics.get_popular_shards(5);
        assert_eq!(popular.len(), 1, "only hot shard should exceed threshold");
        assert_eq!(popular[0].0, hot);
    }

    #[test]
    fn demand_map_tracks_consumer_positions() {
        let mut analytics = SwarmAnalytics::new();
        let shard = test_hash(3);
        let node = test_node("node-x");

        analytics.record_request(shard, node, test_pos(1.0, 2.0, 3.0), 100);
        analytics.record_request(shard, node, test_pos(4.0, 5.0, 6.0), 200);

        let map = analytics.get_demand_map(&shard).expect("test: demand map");
        assert_eq!(map.len(), 2);
        assert!((map[0].x - 1.0).abs() < 1e-9);
        assert!((map[1].x - 4.0).abs() < 1e-9);
    }

    // -- ReplicationTrigger tests (7.5) --

    #[test]
    fn replication_trigger_detects_under_replicated_shards() {
        let mut analytics = SwarmAnalytics::new();
        let shard = test_hash(10);
        let node = test_node("requester");

        // 500 requests, only 2 replicas.
        for i in 0..500 {
            analytics.record_request(shard, node, test_pos(0.0, 0.0, 0.0), i);
        }
        analytics.set_replica_count(shard, 2);

        let trigger = ReplicationTrigger::new(ReplicationConfig {
            requests_per_replica_threshold: 100,
            max_replicas: 20,
            min_replicas: 3,
        });

        let signals = trigger.check(&analytics);
        assert!(!signals.is_empty(), "should detect under-replication");
        assert_eq!(signals[0].shard_id, shard);
        assert!(signals[0].suggested_count > 2);
    }

    #[test]
    fn replication_trigger_no_signal_when_sufficient() {
        let mut analytics = SwarmAnalytics::new();
        let shard = test_hash(11);
        let node = test_node("light-user");

        // Only 50 requests, 3 replicas -- below threshold.
        for i in 0..50 {
            analytics.record_request(shard, node, test_pos(0.0, 0.0, 0.0), i);
        }
        analytics.set_replica_count(shard, 3);

        let trigger = ReplicationTrigger::new(ReplicationConfig {
            requests_per_replica_threshold: 100,
            max_replicas: 20,
            min_replicas: 3,
        });

        let signals = trigger.check(&analytics);
        assert!(
            signals.is_empty(),
            "should not trigger when replicas are sufficient"
        );
    }

    // -- DispersionAdvisor tests (7.6) --

    #[test]
    fn dispersion_recommends_positions_near_demand() {
        let mut analytics = SwarmAnalytics::new();
        let shard = test_hash(20);
        let node = test_node("consumer");

        // Cluster of consumers around (10, 10, 0).
        for i in 0..20 {
            let x = 9.0 + (i as f64 * 0.1);
            analytics.record_request(shard, node, test_pos(x, 10.0, 0.0), i);
        }

        let advisor = DispersionAdvisor::new();
        let recommendations = advisor.recommend_placement(&shard, &analytics, 2);

        assert!(
            !recommendations.is_empty(),
            "should recommend at least one placement"
        );
        // Recommended position should be near the demand cluster.
        for pos in &recommendations {
            let dist = matrix_distance(pos, &test_pos(10.0, 10.0, 0.0));
            assert!(
                dist < 5.0,
                "recommended position should be near demand cluster, got distance {dist}"
            );
        }
    }

    #[test]
    fn dispersion_avoids_existing_replicas() {
        let mut analytics = SwarmAnalytics::new();
        let shard = test_hash(21);
        let node = test_node("consumer");

        // Demand at (5, 5, 0).
        for i in 0..10 {
            analytics.record_request(shard, node, test_pos(5.0, 5.0, 0.0), i);
        }

        let mut advisor = DispersionAdvisor::new();
        // Place existing replica at (5, 5, 0).
        advisor.register_replica(shard, test_pos(5.0, 5.0, 0.0));

        let recommendations = advisor.recommend_placement(&shard, &analytics, 1);
        // With only one demand cluster matching the existing replica,
        // the advisor should filter it out.
        for pos in &recommendations {
            let dist = matrix_distance(pos, &test_pos(5.0, 5.0, 0.0));
            assert!(
                dist >= 2.0,
                "should not recommend position too close to existing replica"
            );
        }
    }

    // -- CascadeMetrics tests (7.7) --

    #[test]
    fn cascade_metrics_track_hops() {
        let mut tracker = CascadeTracker::new(100);
        let shard = test_hash(30);
        let node_a = test_node("node-a");
        let node_b = test_node("node-b");
        let node_c = test_node("node-c");

        tracker.record_cascade_hop(shard, node_a, node_b, 10, 1000);
        tracker.record_cascade_hop(shard, node_b, node_c, 20, 2000);

        let stats = tracker
            .get_cascade_stats(&shard)
            .expect("test: stats should exist");
        assert_eq!(stats.hop_count, 2);
        assert!((stats.avg_latency_ms - 15.0).abs() < 1e-9);
        assert_eq!(stats.total_latency_ms, 30);
        assert_eq!(stats.max_hop_latency_ms, 20);
    }

    #[test]
    fn cascade_metrics_identifies_bottleneck() {
        let mut tracker = CascadeTracker::new(100);
        let shard = test_hash(31);
        let hub = test_node("hub-node");
        let leaf_a = test_node("leaf-a");
        let leaf_b = test_node("leaf-b");
        let leaf_c = test_node("leaf-c");

        // Hub relays to three leaves.
        tracker.record_cascade_hop(shard, hub, leaf_a, 5, 1000);
        tracker.record_cascade_hop(shard, hub, leaf_b, 8, 2000);
        tracker.record_cascade_hop(shard, hub, leaf_c, 6, 3000);

        let stats = tracker
            .get_cascade_stats(&shard)
            .expect("test: stats should exist");
        assert_eq!(
            stats.bottleneck_node,
            Some(hub),
            "hub with 3 outbound hops should be bottleneck"
        );
    }

    #[test]
    fn relay_loads_aggregate_across_shards() {
        let mut tracker = CascadeTracker::new(100);
        let shard_a = test_hash(40);
        let shard_b = test_hash(41);
        let hub = test_node("relay");
        let target = test_node("target");

        tracker.record_cascade_hop(shard_a, hub, target, 5, 1000);
        tracker.record_cascade_hop(shard_b, hub, target, 5, 2000);

        let loads = tracker.get_relay_loads();
        assert_eq!(loads.get(&hub).copied().unwrap_or(0), 2);
    }
}
