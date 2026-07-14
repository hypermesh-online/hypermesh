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
use std::time::{Duration, Instant};

use hypermesh_lib::{ContentHash, MatrixPosition, NodeId};

// ---------------------------------------------------------------------------
// ReplicationRecommendation (windowed fetch-rate API)
// ---------------------------------------------------------------------------

/// Recommendation for a shard's replication based on windowed fetch rate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplicationRecommendation {
    /// No action needed -- fetch rate is below the popularity threshold.
    None,
    /// Shard is popular -- recommend additional replicas.
    Replicate {
        shard_hash: [u8; 32],
        suggested_replicas: u32,
        /// Fetches within the current window.
        fetch_rate: u32,
    },
    /// Shard is extremely popular -- urgent replication needed.
    UrgentReplicate {
        shard_hash: [u8; 32],
        suggested_replicas: u32,
        /// Fetches within the current window.
        fetch_rate: u32,
    },
}

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
///
/// Provides both cumulative request tracking (via [`record_request`]) and
/// time-windowed fetch-rate tracking (via [`record_fetch`]).  The windowed
/// API powers [`ReplicationRecommendation`] generation for R12 swarm scaling.
pub struct SwarmAnalytics {
    /// Per-shard popularity data (cumulative).
    shard_data: HashMap<ContentHash, ShardPopularity>,
    /// Per-shard set of unique consumer node IDs.
    shard_consumers: HashMap<ContentHash, HashSet<NodeId>>,
    /// Per-shard replica count (set externally by BlockMatrix).
    replica_counts: HashMap<ContentHash, u32>,

    // -- Windowed fetch-rate tracking --

    /// shard_hash -> list of fetch timestamps (windowed).
    fetch_history: HashMap<[u8; 32], Vec<Instant>>,
    /// Sliding window for counting fetches.
    window: Duration,
    /// If fetches in window exceed this, shard is "popular".
    popularity_threshold: u32,
    /// Maximum history entries per shard (prevent unbounded growth).
    max_history_per_shard: usize,
}

impl SwarmAnalytics {
    /// Create a new swarm analytics tracker.
    ///
    /// Uses default windowed-fetch settings (5 min window, threshold 10).
    pub fn new() -> Self {
        Self {
            shard_data: HashMap::new(),
            shard_consumers: HashMap::new(),
            replica_counts: HashMap::new(),
            fetch_history: HashMap::new(),
            window: Duration::from_secs(300),
            popularity_threshold: 10,
            max_history_per_shard: 1000,
        }
    }

    /// Create a swarm analytics tracker with explicit windowed-fetch settings.
    pub fn with_window(window: Duration, popularity_threshold: u32) -> Self {
        Self {
            shard_data: HashMap::new(),
            shard_consumers: HashMap::new(),
            replica_counts: HashMap::new(),
            fetch_history: HashMap::new(),
            window,
            popularity_threshold,
            max_history_per_shard: 1000,
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

    // -- Windowed fetch-rate API (R12 swarm scaling) --

    /// Record a shard fetch event at a specific instant (test-friendly).
    pub fn record_fetch_at(&mut self, shard_hash: [u8; 32], at: Instant) {
        let history = self.fetch_history.entry(shard_hash).or_default();
        history.push(at);

        // Trim entries outside the window.
        let cutoff = at.checked_sub(self.window).unwrap_or(at);
        history.retain(|t| *t >= cutoff);

        // Cap at max_history_per_shard (drop oldest).
        while history.len() > self.max_history_per_shard {
            history.remove(0);
        }
    }

    /// Record a shard fetch event at the current time.
    pub fn record_fetch(&mut self, shard_hash: [u8; 32]) {
        self.record_fetch_at(shard_hash, Instant::now());
    }

    /// Count of fetches for a shard within the current window.
    pub fn get_fetch_rate(&self, shard_hash: &[u8; 32]) -> u32 {
        let now = Instant::now();
        self.get_fetch_rate_at(shard_hash, now)
    }

    /// Count of fetches within the window relative to a given instant.
    fn get_fetch_rate_at(&self, shard_hash: &[u8; 32], now: Instant) -> u32 {
        let history = match self.fetch_history.get(shard_hash) {
            Some(h) => h,
            None => return 0,
        };
        let cutoff = now.checked_sub(self.window).unwrap_or(now);
        history.iter().filter(|t| **t >= cutoff).count() as u32
    }

    /// Produce a replication recommendation for a single shard.
    ///
    /// - Below `popularity_threshold` -> `None`
    /// - 1x-3x threshold -> `Replicate`
    /// - >3x threshold -> `UrgentReplicate`
    ///
    /// `suggested_replicas` = `fetch_rate / popularity_threshold`, capped at 10.
    pub fn get_recommendation(&self, shard_hash: &[u8; 32]) -> ReplicationRecommendation {
        self.get_recommendation_at(shard_hash, Instant::now())
    }

    /// Recommendation relative to a given instant (test-friendly).
    fn get_recommendation_at(
        &self,
        shard_hash: &[u8; 32],
        now: Instant,
    ) -> ReplicationRecommendation {
        let rate = self.get_fetch_rate_at(shard_hash, now);
        if self.popularity_threshold == 0 || rate < self.popularity_threshold {
            return ReplicationRecommendation::None;
        }
        let suggested = (rate / self.popularity_threshold).min(10);
        if rate > self.popularity_threshold.saturating_mul(3) {
            ReplicationRecommendation::UrgentReplicate {
                shard_hash: *shard_hash,
                suggested_replicas: suggested,
                fetch_rate: rate,
            }
        } else {
            ReplicationRecommendation::Replicate {
                shard_hash: *shard_hash,
                suggested_replicas: suggested,
                fetch_rate: rate,
            }
        }
    }

    /// Return recommendations for ALL shards above the popularity threshold,
    /// sorted by fetch_rate descending.
    pub fn get_popular_shard_recommendations(&self) -> Vec<ReplicationRecommendation> {
        let now = Instant::now();
        let mut recs: Vec<ReplicationRecommendation> = self
            .fetch_history
            .keys()
            .filter_map(|hash| {
                let rec = self.get_recommendation_at(hash, now);
                if rec == ReplicationRecommendation::None {
                    None
                } else {
                    Some(rec)
                }
            })
            .collect();
        recs.sort_by(|a, b| {
            let rate_a = match a {
                ReplicationRecommendation::Replicate { fetch_rate, .. }
                | ReplicationRecommendation::UrgentReplicate { fetch_rate, .. } => *fetch_rate,
                ReplicationRecommendation::None => 0,
            };
            let rate_b = match b {
                ReplicationRecommendation::Replicate { fetch_rate, .. }
                | ReplicationRecommendation::UrgentReplicate { fetch_rate, .. } => *fetch_rate,
                ReplicationRecommendation::None => 0,
            };
            rate_b.cmp(&rate_a)
        });
        recs
    }

    /// Remove shards with zero fetches in the current window (GC for cold shards).
    pub fn cleanup_cold_shards(&mut self) {
        let now = Instant::now();
        let window = self.window;
        self.fetch_history.retain(|_, history| {
            let cutoff = now.checked_sub(window).unwrap_or(now);
            history.retain(|t| *t >= cutoff);
            !history.is_empty()
        });
    }

    /// Number of shards currently tracked in the windowed fetch history.
    pub fn tracked_shard_count(&self) -> usize {
        self.fetch_history.len()
    }

    /// Total fetch events across all shards currently within the window.
    pub fn total_fetches_in_window(&self) -> usize {
        let now = Instant::now();
        let cutoff = now.checked_sub(self.window).unwrap_or(now);
        self.fetch_history
            .values()
            .flat_map(|h| h.iter())
            .filter(|t| **t >= cutoff)
            .count()
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

    // -- Windowed fetch-rate tests (R12 swarm scaling) --

    fn test_shard_bytes(val: u8) -> [u8; 32] {
        [val; 32]
    }

    #[test]
    fn windowed_new_analytics_has_zero_shards() {
        let analytics = SwarmAnalytics::with_window(Duration::from_secs(60), 5);
        assert_eq!(analytics.tracked_shard_count(), 0);
        assert_eq!(analytics.total_fetches_in_window(), 0);
    }

    #[test]
    fn windowed_record_single_fetch_rate_is_one() {
        let mut analytics = SwarmAnalytics::with_window(Duration::from_secs(60), 5);
        let hash = test_shard_bytes(0xAA);
        let now = Instant::now();
        analytics.record_fetch_at(hash, now);

        assert_eq!(analytics.get_fetch_rate_at(&hash, now), 1);
        assert_eq!(analytics.tracked_shard_count(), 1);
    }

    #[test]
    fn windowed_record_multiple_fetches_rate_correct() {
        let mut analytics = SwarmAnalytics::with_window(Duration::from_secs(60), 5);
        let hash = test_shard_bytes(0xBB);
        let now = Instant::now();

        for i in 0..7 {
            analytics.record_fetch_at(hash, now + Duration::from_secs(i));
        }
        // All 7 within 60s window when queried at now + 6s.
        assert_eq!(analytics.get_fetch_rate_at(&hash, now + Duration::from_secs(6)), 7);
    }

    #[test]
    fn windowed_below_threshold_recommendation_none() {
        let mut analytics = SwarmAnalytics::with_window(Duration::from_secs(60), 10);
        let hash = test_shard_bytes(0xCC);
        let now = Instant::now();

        // Only 5 fetches, threshold is 10.
        for i in 0..5 {
            analytics.record_fetch_at(hash, now + Duration::from_millis(i * 100));
        }

        let rec = analytics.get_recommendation_at(&hash, now + Duration::from_secs(1));
        assert_eq!(rec, ReplicationRecommendation::None);
    }

    #[test]
    fn windowed_above_threshold_replicate() {
        let mut analytics = SwarmAnalytics::with_window(Duration::from_secs(60), 10);
        let hash = test_shard_bytes(0xDD);
        let now = Instant::now();

        // 20 fetches, threshold is 10 => 2x threshold => Replicate.
        for i in 0..20 {
            analytics.record_fetch_at(hash, now + Duration::from_millis(i * 50));
        }

        let rec = analytics.get_recommendation_at(&hash, now + Duration::from_secs(1));
        match rec {
            ReplicationRecommendation::Replicate {
                shard_hash,
                suggested_replicas,
                fetch_rate,
            } => {
                assert_eq!(shard_hash, hash);
                assert_eq!(fetch_rate, 20);
                assert_eq!(suggested_replicas, 2); // 20 / 10 = 2
            }
            other => unreachable!("test: expected Replicate, got {:?}", other),
        }
    }

    #[test]
    fn windowed_urgent_replicate_above_3x() {
        let mut analytics = SwarmAnalytics::with_window(Duration::from_secs(60), 10);
        let hash = test_shard_bytes(0xEE);
        let now = Instant::now();

        // 40 fetches, threshold is 10 => 4x threshold => UrgentReplicate.
        for i in 0..40 {
            analytics.record_fetch_at(hash, now + Duration::from_millis(i * 25));
        }

        let rec = analytics.get_recommendation_at(&hash, now + Duration::from_secs(1));
        match rec {
            ReplicationRecommendation::UrgentReplicate {
                shard_hash,
                suggested_replicas,
                fetch_rate,
            } => {
                assert_eq!(shard_hash, hash);
                assert_eq!(fetch_rate, 40);
                assert_eq!(suggested_replicas, 4); // 40 / 10 = 4
            }
            other => unreachable!("test: expected UrgentReplicate, got {:?}", other),
        }
    }

    #[test]
    fn windowed_popular_shard_recommendations_sorted() {
        let mut analytics = SwarmAnalytics::with_window(Duration::from_secs(60), 5);
        let hash_hot = test_shard_bytes(0x01);
        let hash_warm = test_shard_bytes(0x02);
        let hash_cold = test_shard_bytes(0x03);
        let now = Instant::now();

        // Hot: 30 fetches.
        for i in 0..30 {
            analytics.record_fetch_at(hash_hot, now + Duration::from_millis(i * 10));
        }
        // Warm: 10 fetches.
        for i in 0..10 {
            analytics.record_fetch_at(hash_warm, now + Duration::from_millis(i * 10));
        }
        // Cold: 2 fetches (below threshold).
        for i in 0..2 {
            analytics.record_fetch_at(hash_cold, now + Duration::from_millis(i * 10));
        }

        let recs = analytics.get_popular_shard_recommendations();
        // Cold should be excluded (below threshold of 5).
        assert_eq!(recs.len(), 2, "only hot and warm should appear");

        // First should be the hottest.
        match &recs[0] {
            ReplicationRecommendation::UrgentReplicate { fetch_rate, .. } => {
                assert_eq!(*fetch_rate, 30);
            }
            other => unreachable!("test: expected UrgentReplicate for hot shard, got {:?}", other),
        }
        match &recs[1] {
            ReplicationRecommendation::Replicate { fetch_rate, .. } => {
                assert_eq!(*fetch_rate, 10);
            }
            other => unreachable!("test: expected Replicate for warm shard, got {:?}", other),
        }
    }

    #[test]
    fn windowed_cleanup_removes_cold_shards() {
        let mut analytics = SwarmAnalytics::with_window(Duration::from_millis(50), 5);
        let hash = test_shard_bytes(0xFF);

        // Record a fetch that will expire quickly.
        let past = Instant::now() - Duration::from_millis(200);
        analytics.fetch_history.entry(hash).or_default().push(past);

        assert_eq!(analytics.tracked_shard_count(), 1);
        analytics.cleanup_cold_shards();
        assert_eq!(
            analytics.tracked_shard_count(),
            0,
            "cold shard should be removed after cleanup"
        );
    }

    #[test]
    fn windowed_max_history_prevents_unbounded_growth() {
        let mut analytics = SwarmAnalytics::with_window(Duration::from_secs(600), 5);
        analytics.max_history_per_shard = 20;
        let hash = test_shard_bytes(0xAB);
        let now = Instant::now();

        // Record 50 fetches; only 20 should be kept.
        for i in 0..50 {
            analytics.record_fetch_at(hash, now + Duration::from_millis(i * 10));
        }

        let history = analytics
            .fetch_history
            .get(&hash)
            .expect("test: history should exist");
        assert_eq!(history.len(), 20, "history should be capped at max_history_per_shard");
    }

    #[test]
    fn windowed_expiry_drops_old_fetches() {
        let mut analytics = SwarmAnalytics::with_window(Duration::from_millis(100), 3);
        let hash = test_shard_bytes(0xCD);
        let now = Instant::now();

        // Record 5 fetches at `now`.
        for i in 0..5 {
            analytics.record_fetch_at(hash, now + Duration::from_millis(i));
        }

        // At now + 200ms they should all be outside the 100ms window.
        let later = now + Duration::from_millis(200);
        assert_eq!(
            analytics.get_fetch_rate_at(&hash, later),
            0,
            "old fetches should be outside the window"
        );
    }

    #[test]
    fn windowed_suggested_replicas_capped_at_10() {
        let mut analytics = SwarmAnalytics::with_window(Duration::from_secs(60), 1);
        let hash = test_shard_bytes(0xDE);
        let now = Instant::now();

        // 100 fetches with threshold 1 => suggested = 100, but capped at 10.
        for i in 0..100 {
            analytics.record_fetch_at(hash, now + Duration::from_millis(i));
        }

        let rec = analytics.get_recommendation_at(&hash, now + Duration::from_secs(1));
        match rec {
            ReplicationRecommendation::UrgentReplicate {
                suggested_replicas, ..
            } => {
                assert_eq!(suggested_replicas, 10, "suggested_replicas should be capped at 10");
            }
            other => unreachable!("test: expected UrgentReplicate, got {:?}", other),
        }
    }

    // -- P6 convergence tests (the replication loop must STOP) -------------

    /// The reconciler feedback loop MUST converge: given steady demand on a
    /// shard, driving the actuator (fetch a replica → `set_replica_count` to
    /// the new provider count) each cycle must make `ReplicationTrigger::check`
    /// see `needed <= replicas` and STOP emitting a signal. Without the
    /// `set_replica_count` feedback hook (the never-called convergence gap),
    /// the replica count stays at 0 and the trigger fires forever.
    ///
    /// This is the pure-logic mirror of the E.2 actuator loop in
    /// `blockmatrix/src/bin/node/commands/connect.rs`.
    #[test]
    fn replication_loop_converges_then_stops() {
        let mut analytics = SwarmAnalytics::new();
        let shard = test_hash(0x77);
        let node = test_node("hot-consumer");

        // Steady demand: 350 requests. With the default trigger
        // (requests_per_replica_threshold = 100), `needed` = ceil(350/100) = 4.
        for i in 0..350 {
            analytics.record_request(shard, node, test_pos(0.0, 0.0, 0.0), i);
        }

        let trigger = ReplicationTrigger::new(ReplicationConfig::default());

        // Simulate the actuator: each cycle, if a signal fires, "fetch" one
        // more replica (the local node becomes a provider) and feed the new
        // provider count back via set_replica_count. Track the trajectory.
        let mut provider_count: u32 = 0;
        analytics.set_replica_count(shard, provider_count);

        let mut trajectory = vec![provider_count];
        let mut converged_at = None;
        for cycle in 0..20 {
            let signals = trigger.check(&analytics);
            if signals.is_empty() {
                converged_at = Some(cycle);
                break;
            }
            // Actuator fetched one replica this cycle → provider count grows.
            provider_count += 1;
            // THE feedback hook: report the new provider count back.
            analytics.set_replica_count(shard, provider_count);
            trajectory.push(provider_count);
        }

        // 1. The count RISES (monotonic non-decreasing, strictly up while
        //    under-replicated).
        assert!(
            trajectory.windows(2).all(|w| w[1] >= w[0]),
            "replica count must rise monotonically, got {trajectory:?}"
        );
        assert!(
            *trajectory.last().expect("test: trajectory non-empty") > trajectory[0],
            "replica count must actually increase, got {trajectory:?}"
        );

        // 2. It STOPS (converges) — the trigger stops firing once replicas
        //    meet demand. needed = ceil(350/100) = 4 (>= min_replicas 3).
        let cycles = converged_at.expect("loop must converge (trigger must stop firing)");
        assert!(cycles > 0, "should take at least one cycle to converge");
        assert_eq!(
            analytics.get_replica_count(&shard),
            4,
            "converged replica count must equal demand-derived `needed` (ceil(350/100)=4)"
        );

        // 3. Once converged, further checks stay quiet (stable fixed point).
        assert!(
            trigger.check(&analytics).is_empty(),
            "converged loop must not re-fire — stable fixed point"
        );
    }

    /// Convergence must also STOP at `min_replicas` for lightly-loaded shards
    /// (below the per-replica threshold), never over-replicating.
    #[test]
    fn replication_loop_converges_at_min_replicas() {
        let mut analytics = SwarmAnalytics::new();
        let shard = test_hash(0x78);
        let node = test_node("light-consumer");

        // Low demand: 50 requests => needed = max(ceil(50/100), min=3) = 3.
        for i in 0..50 {
            analytics.record_request(shard, node, test_pos(0.0, 0.0, 0.0), i);
        }

        let trigger = ReplicationTrigger::new(ReplicationConfig::default());
        let mut provider_count = 0u32;
        analytics.set_replica_count(shard, provider_count);

        let mut converged = false;
        for _ in 0..10 {
            let signals = trigger.check(&analytics);
            if signals.is_empty() {
                converged = true;
                break;
            }
            provider_count += 1;
            analytics.set_replica_count(shard, provider_count);
        }

        assert!(converged, "light-load loop must converge");
        assert_eq!(
            analytics.get_replica_count(&shard),
            3,
            "must converge at min_replicas (3), not over-replicate"
        );
    }
}
