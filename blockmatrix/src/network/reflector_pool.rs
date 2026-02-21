// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Reflector pool for tracking block availability across the network
//!
//! A reflector is a peer node that holds replicas of blocks for a given
//! Network scope chain. The pool tracks which nodes are available, their
//! health scores, and selects the best reflectors for sync operations.
//!
//! Reflector selection considers:
//! - Health score (uptime, responsiveness)
//! - Block height (how up-to-date the reflector is)
//! - Staleness (when it was last seen)

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::bootstrap::PrivacyMode;
use hypermesh_lib::MatrixPosition;

/// Tracks block replicas across the network for redundancy and availability.
///
/// Each network scope can have multiple reflectors. The pool monitors their
/// health and provides sorted selection for sync operations.
pub struct ReflectorPool {
    /// Available reflectors keyed by network_id, then by node_id
    reflectors: HashMap<String, HashMap<String, Reflector>>,
    /// Configuration
    config: ReflectorConfig,
}

/// A peer node serving as a block reflector for a network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflector {
    /// Unique identifier of the reflector node
    pub node_id: String,
    /// Position in the Block-MATRIX topology
    pub position: MatrixPosition,
    /// Unix timestamp (seconds) when this reflector was last seen
    pub last_seen: u64,
    /// Highest block this reflector has reported
    pub block_height: u64,
    /// Health score from 0.0 (dead) to 1.0 (perfect)
    pub health_score: f64,
    /// Privacy mode of the reflector
    pub privacy_mode: PrivacyMode,
}

/// Configuration for the reflector pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectorConfig {
    /// Minimum number of reflectors needed for a network to be healthy
    pub min_reflectors: usize,
    /// Maximum reflectors to track per network
    pub max_reflectors: usize,
    /// Minimum health score to be eligible for selection
    pub health_threshold: f64,
    /// Milliseconds after which a reflector is considered stale
    pub stale_timeout_ms: u64,
}

impl Default for ReflectorConfig {
    fn default() -> Self {
        Self {
            min_reflectors: 3,
            max_reflectors: 50,
            health_threshold: 0.3,
            stale_timeout_ms: 30_000,
        }
    }
}

impl ReflectorPool {
    /// Create a new reflector pool with the given configuration
    pub fn new(config: ReflectorConfig) -> Self {
        info!(
            min = config.min_reflectors,
            max = config.max_reflectors,
            "ReflectorPool created"
        );

        Self {
            reflectors: HashMap::new(),
            config,
        }
    }

    /// Get the pool configuration
    pub fn config(&self) -> &ReflectorConfig {
        &self.config
    }

    /// Register or update a reflector for a network.
    ///
    /// If the reflector already exists, its fields are updated.
    /// If the network has reached max_reflectors, the lowest-health
    /// reflector is evicted to make room (only if the new one is better).
    pub fn register_reflector(
        &mut self,
        network_id: &str,
        reflector: Reflector,
    ) {
        let network_pool = self
            .reflectors
            .entry(network_id.to_string())
            .or_default();

        // If already registered, just update
        if network_pool.contains_key(&reflector.node_id) {
            debug!(
                network = %network_id,
                node = %reflector.node_id,
                "Updated existing reflector"
            );
            network_pool.insert(reflector.node_id.clone(), reflector);
            return;
        }

        // Check capacity
        if network_pool.len() >= self.config.max_reflectors {
            // Find the worst reflector
            let worst = network_pool
                .values()
                .min_by(|a, b| {
                    a.health_score
                        .partial_cmp(&b.health_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|r| (r.node_id.clone(), r.health_score));

            if let Some((worst_id, worst_score)) = worst {
                if reflector.health_score > worst_score {
                    debug!(
                        network = %network_id,
                        evicted = %worst_id,
                        new = %reflector.node_id,
                        "Evicted low-health reflector for better one"
                    );
                    network_pool.remove(&worst_id);
                } else {
                    debug!(
                        network = %network_id,
                        node = %reflector.node_id,
                        "Rejected reflector: pool full, not better than worst"
                    );
                    return;
                }
            }
        }

        info!(
            network = %network_id,
            node = %reflector.node_id,
            health = reflector.health_score,
            "Registered new reflector"
        );

        network_pool.insert(reflector.node_id.clone(), reflector);
    }

    /// Remove a reflector from a network
    ///
    /// Returns true if the reflector was found and removed.
    pub fn remove_reflector(
        &mut self,
        network_id: &str,
        node_id: &str,
    ) -> bool {
        if let Some(network_pool) = self.reflectors.get_mut(network_id) {
            let removed = network_pool.remove(node_id).is_some();
            if removed {
                debug!(
                    network = %network_id,
                    node = %node_id,
                    "Removed reflector"
                );
            }
            removed
        } else {
            false
        }
    }

    /// Get the best reflectors for a network, sorted by health score
    /// (highest first), limited to `count`.
    pub fn get_best_reflectors(
        &self,
        network_id: &str,
        count: usize,
    ) -> Vec<&Reflector> {
        let Some(network_pool) = self.reflectors.get(network_id) else {
            return Vec::new();
        };

        let mut eligible: Vec<&Reflector> = network_pool
            .values()
            .filter(|r| r.health_score >= self.config.health_threshold)
            .collect();

        // Sort by health_score descending, then block_height descending
        eligible.sort_by(|a, b| {
            b.health_score
                .partial_cmp(&a.health_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.block_height.cmp(&a.block_height))
        });

        eligible.truncate(count);
        eligible
    }

    /// Check if a network has enough healthy reflectors
    pub fn is_healthy(&self, network_id: &str) -> bool {
        let healthy_count = self.healthy_count(network_id);
        healthy_count >= self.config.min_reflectors
    }

    /// Count reflectors above the health threshold for a network
    pub fn healthy_count(&self, network_id: &str) -> usize {
        let Some(network_pool) = self.reflectors.get(network_id) else {
            return 0;
        };

        network_pool
            .values()
            .filter(|r| r.health_score >= self.config.health_threshold)
            .count()
    }

    /// Total reflector count for a network (regardless of health)
    pub fn total_count(&self, network_id: &str) -> usize {
        self.reflectors
            .get(network_id)
            .map_or(0, |pool| pool.len())
    }

    /// Update the health score for a specific reflector node.
    ///
    /// The node_id is searched across all networks.
    pub fn update_health(&mut self, node_id: &str, health_score: f64) {
        let clamped = health_score.clamp(0.0, 1.0);

        for pool in self.reflectors.values_mut() {
            if let Some(reflector) = pool.get_mut(node_id) {
                reflector.health_score = clamped;
            }
        }
    }

    /// Remove all reflectors that have not been seen since
    /// `now_ms - stale_timeout_ms`. `now_ms` is in milliseconds.
    ///
    /// Returns the number of reflectors pruned.
    pub fn prune_stale(&mut self, now_ms: u64) -> usize {
        let cutoff_secs = now_ms.saturating_sub(self.config.stale_timeout_ms) / 1000;
        let mut pruned = 0;

        for (network_id, pool) in &mut self.reflectors {
            let before = pool.len();
            pool.retain(|_, r| r.last_seen >= cutoff_secs);
            let removed = before - pool.len();
            if removed > 0 {
                warn!(
                    network = %network_id,
                    pruned = removed,
                    "Pruned stale reflectors"
                );
            }
            pruned += removed;
        }

        pruned
    }

    /// Get all tracked network IDs
    pub fn tracked_networks(&self) -> Vec<&str> {
        self.reflectors.keys().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hypermesh_lib::MatrixPosition;

    fn make_reflector(
        node_id: &str,
        health: f64,
        height: u64,
        last_seen: u64,
    ) -> Reflector {
        Reflector {
            node_id: node_id.to_string(),
            position: MatrixPosition {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            last_seen,
            block_height: height,
            health_score: health,
            privacy_mode: PrivacyMode::PUBLIC,
        }
    }

    fn test_config() -> ReflectorConfig {
        ReflectorConfig {
            min_reflectors: 2,
            max_reflectors: 5,
            health_threshold: 0.3,
            stale_timeout_ms: 10_000,
        }
    }

    #[test]
    fn test_create_pool() {
        let pool = ReflectorPool::new(test_config());
        assert!(pool.tracked_networks().is_empty());
        assert_eq!(pool.total_count("net"), 0);
    }

    #[test]
    fn test_register_and_remove_reflector() {
        let mut pool = ReflectorPool::new(test_config());

        let r = make_reflector("node-1", 0.9, 100, 5000);
        pool.register_reflector("net-alpha", r);

        assert_eq!(pool.total_count("net-alpha"), 1);

        let removed = pool.remove_reflector("net-alpha", "node-1");
        assert!(removed);
        assert_eq!(pool.total_count("net-alpha"), 0);

        // Removing again returns false
        assert!(!pool.remove_reflector("net-alpha", "node-1"));
    }

    #[test]
    fn test_get_best_reflectors_sorted_by_health() {
        let mut pool = ReflectorPool::new(test_config());

        pool.register_reflector("net", make_reflector("low", 0.4, 50, 100));
        pool.register_reflector("net", make_reflector("high", 0.95, 50, 100));
        pool.register_reflector("net", make_reflector("mid", 0.7, 50, 100));

        let best = pool.get_best_reflectors("net", 2);
        assert_eq!(best.len(), 2);
        assert_eq!(best[0].node_id, "high");
        assert_eq!(best[1].node_id, "mid");
    }

    #[test]
    fn test_health_threshold_filtering() {
        let mut pool = ReflectorPool::new(test_config());

        // Below threshold (0.3)
        pool.register_reflector("net", make_reflector("bad", 0.1, 50, 100));
        // Above threshold
        pool.register_reflector("net", make_reflector("good", 0.5, 50, 100));

        let best = pool.get_best_reflectors("net", 10);
        assert_eq!(best.len(), 1);
        assert_eq!(best[0].node_id, "good");
    }

    #[test]
    fn test_is_healthy() {
        let mut pool = ReflectorPool::new(test_config());

        // Need min_reflectors=2 healthy ones
        pool.register_reflector("net", make_reflector("n1", 0.8, 10, 100));
        assert!(!pool.is_healthy("net")); // Only 1

        pool.register_reflector("net", make_reflector("n2", 0.9, 10, 100));
        assert!(pool.is_healthy("net")); // Now 2

        // Unknown network
        assert!(!pool.is_healthy("nonexistent"));
    }

    #[test]
    fn test_update_health() {
        let mut pool = ReflectorPool::new(test_config());

        pool.register_reflector("net", make_reflector("n1", 0.5, 10, 100));

        pool.update_health("n1", 0.9);

        let best = pool.get_best_reflectors("net", 1);
        assert_eq!(best.len(), 1);
        assert!((best[0].health_score - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn test_update_health_clamped() {
        let mut pool = ReflectorPool::new(test_config());

        pool.register_reflector("net", make_reflector("n1", 0.5, 10, 100));

        pool.update_health("n1", 1.5); // Should clamp to 1.0
        let best = pool.get_best_reflectors("net", 1);
        assert!((best[0].health_score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_prune_stale() {
        let mut pool = ReflectorPool::new(test_config());

        // last_seen is in seconds; stale_timeout_ms = 10_000 (10 seconds)
        // Reflector seen at second 5
        pool.register_reflector("net", make_reflector("old", 0.8, 10, 5));
        // Reflector seen at second 20
        pool.register_reflector("net", make_reflector("new", 0.9, 10, 20));

        assert_eq!(pool.total_count("net"), 2);

        // now_ms = 25_000 (second 25). Cutoff = (25000 - 10000) / 1000 = 15.
        // "old" last_seen=5 < 15 -> pruned
        // "new" last_seen=20 >= 15 -> kept
        let pruned = pool.prune_stale(25_000);
        assert_eq!(pruned, 1);
        assert_eq!(pool.total_count("net"), 1);

        let best = pool.get_best_reflectors("net", 10);
        assert_eq!(best[0].node_id, "new");
    }

    #[test]
    fn test_max_reflectors_eviction() {
        let config = ReflectorConfig {
            max_reflectors: 3,
            ..test_config()
        };
        let mut pool = ReflectorPool::new(config);

        pool.register_reflector("net", make_reflector("a", 0.5, 10, 100));
        pool.register_reflector("net", make_reflector("b", 0.6, 10, 100));
        pool.register_reflector("net", make_reflector("c", 0.7, 10, 100));

        // Pool is full (3). Add a better one -> should evict "a" (lowest health)
        pool.register_reflector("net", make_reflector("d", 0.8, 10, 100));

        assert_eq!(pool.total_count("net"), 3);

        let best = pool.get_best_reflectors("net", 10);
        let ids: Vec<&str> = best.iter().map(|r| r.node_id.as_str()).collect();
        assert!(!ids.contains(&"a")); // "a" was evicted
        assert!(ids.contains(&"d")); // "d" was added
    }

    #[test]
    fn test_register_update_existing() {
        let mut pool = ReflectorPool::new(test_config());

        pool.register_reflector("net", make_reflector("n1", 0.5, 10, 100));
        // Update same node with new height and health
        pool.register_reflector("net", make_reflector("n1", 0.9, 50, 200));

        assert_eq!(pool.total_count("net"), 1);

        let best = pool.get_best_reflectors("net", 1);
        assert!((best[0].health_score - 0.9).abs() < f64::EPSILON);
        assert_eq!(best[0].block_height, 50);
    }

    #[test]
    fn test_tracked_networks() {
        let mut pool = ReflectorPool::new(test_config());

        pool.register_reflector("alpha", make_reflector("n1", 0.8, 10, 100));
        pool.register_reflector("beta", make_reflector("n2", 0.8, 10, 100));

        let networks = pool.tracked_networks();
        assert_eq!(networks.len(), 2);
        assert!(networks.contains(&"alpha"));
        assert!(networks.contains(&"beta"));
    }
}
