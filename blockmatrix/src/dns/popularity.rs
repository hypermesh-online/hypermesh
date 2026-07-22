// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! DNS popularity tracking for ngauge-driven replication.
//!
//! Popular DNS names are tracked by resolution frequency, analogous to
//! how `SwarmDemandTracker` tracks shard fetch demand. The ngauge
//! intelligence layer can use this data to decide which DNS records
//! should be replicated to more nodes.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Tracks DNS resolution frequency for ngauge-driven replication.
///
/// Popular names get replicated to more nodes (like popular shards).
/// Uses the same pattern as [`crate::network::SwarmDemandTracker`].
pub struct DnsPopularityTracker {
    /// Resolution count per name.
    resolution_counts: Arc<RwLock<HashMap<String, u64>>>,
}

impl Default for DnsPopularityTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl DnsPopularityTracker {
    /// Create a new empty tracker.
    pub fn new() -> Self {
        Self {
            resolution_counts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record a DNS resolution request for a name.
    pub async fn record_resolution(&self, name: &str) {
        let mut counts = self.resolution_counts.write().await;
        *counts.entry(name.to_string()).or_insert(0) += 1;
    }

    /// Get the resolution count for a name.
    pub async fn resolution_count(&self, name: &str) -> u64 {
        self.resolution_counts
            .read()
            .await
            .get(name)
            .copied()
            .unwrap_or(0)
    }

    /// Get the top N most-resolved names with their counts.
    pub async fn top_names(&self, n: usize) -> Vec<(String, u64)> {
        let counts = self.resolution_counts.read().await;
        let mut sorted: Vec<_> = counts.iter().map(|(k, v)| (k.clone(), *v)).collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(n);
        sorted
    }

    /// Get a BLAKE3 content hash for a DNS name (for ngauge demand tracking).
    ///
    /// Uses `dns:` prefix for domain separation from shard content hashes.
    pub fn name_hash(name: &str) -> [u8; 32] {
        *blake3::hash(format!("dns:{}", name).as_bytes()).as_bytes()
    }

    /// Snapshot of all resolution counts for feeding into ngauge.
    pub async fn snapshot(&self) -> HashMap<String, u64> {
        self.resolution_counts.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dns_popularity_tracker_records() {
        let tracker = DnsPopularityTracker::new();
        tracker.record_resolution("alice").await;
        tracker.record_resolution("alice").await;
        tracker.record_resolution("bob").await;
        assert_eq!(tracker.resolution_count("alice").await, 2);
        assert_eq!(tracker.resolution_count("bob").await, 1);
        assert_eq!(tracker.resolution_count("unknown").await, 0);
    }

    #[tokio::test]
    async fn test_dns_popularity_top_names() {
        let tracker = DnsPopularityTracker::new();
        for _ in 0..10 {
            tracker.record_resolution("popular").await;
        }
        for _ in 0..5 {
            tracker.record_resolution("medium").await;
        }
        tracker.record_resolution("rare").await;
        let top = tracker.top_names(2).await;
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, "popular");
        assert_eq!(top[1].0, "medium");
    }

    #[test]
    fn test_dns_name_hash_deterministic() {
        let h1 = DnsPopularityTracker::name_hash("alice");
        let h2 = DnsPopularityTracker::name_hash("alice");
        assert_eq!(h1, h2);
        let h3 = DnsPopularityTracker::name_hash("bob");
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_dns_name_hash_domain_separated() {
        // DNS hash should differ from a raw blake3 hash of the same string
        let dns_hash = DnsPopularityTracker::name_hash("alice");
        let raw_hash = *blake3::hash(b"alice").as_bytes();
        assert_ne!(dns_hash, raw_hash, "dns: prefix should produce different hash");
    }

    #[tokio::test]
    async fn test_dns_popularity_snapshot() {
        let tracker = DnsPopularityTracker::new();
        tracker.record_resolution("a").await;
        tracker.record_resolution("b").await;
        tracker.record_resolution("b").await;
        let snap = tracker.snapshot().await;
        assert_eq!(snap.len(), 2);
        assert_eq!(snap["a"], 1);
        assert_eq!(snap["b"], 2);
    }

    #[tokio::test]
    async fn test_dns_popularity_default() {
        let tracker = DnsPopularityTracker::default();
        assert_eq!(tracker.resolution_count("anything").await, 0);
        let top = tracker.top_names(10).await;
        assert!(top.is_empty());
    }
}
