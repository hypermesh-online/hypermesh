// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Swarm provider system for consumer-becomes-provider (R12).
//!
//! When a node fetches shards to reconstruct an asset, it stores them
//! locally and announces availability. This makes the consumer a provider,
//! creating self-scaling distribution where popularity drives replication.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

use hypermesh_lib::ContentHash;

/// Index mapping shard hashes to the set of node IDs that provide them.
/// Populated from shard announcements received from peers.
pub struct ShardLocationIndex {
    locations: Arc<RwLock<HashMap<ContentHash, HashSet<String>>>>,
}

impl ShardLocationIndex {
    /// Create a new empty shard location index.
    pub fn new() -> Self {
        Self {
            locations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a node as a provider of the given shards.
    pub async fn register_provider(&self, node_id: &str, shard_ids: &[ContentHash]) {
        let mut locs = self.locations.write().await;
        for shard_id in shard_ids {
            locs.entry(*shard_id)
                .or_default()
                .insert(node_id.to_string());
        }
    }

    /// Remove a node from all shard provider sets (e.g., on disconnect).
    pub async fn remove_provider(&self, node_id: &str) {
        let mut locs = self.locations.write().await;
        for providers in locs.values_mut() {
            providers.remove(node_id);
        }
        locs.retain(|_, providers| !providers.is_empty());
    }

    /// Get all known providers for a shard.
    pub async fn get_providers(&self, shard_id: &ContentHash) -> Vec<String> {
        let locs = self.locations.read().await;
        locs.get(shard_id)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get the number of tracked shards.
    pub async fn shard_count(&self) -> usize {
        self.locations.read().await.len()
    }

    /// Get the number of unique providers across all shards.
    pub async fn provider_count(&self) -> usize {
        let locs = self.locations.read().await;
        let mut all_providers = HashSet::new();
        for providers in locs.values() {
            all_providers.extend(providers.iter().cloned());
        }
        all_providers.len()
    }
}

impl Default for ShardLocationIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the wire payload for a TAG_SHARD_ANNOUNCE message.
///
/// Format: tag(1) + count(4 bytes u32 LE) + [shard_hash(32)]...
pub fn build_shard_announce_payload(shard_ids: &[ContentHash]) -> Vec<u8> {
    let count = shard_ids.len() as u32;
    let mut buf = Vec::with_capacity(5 + shard_ids.len() * 32);
    buf.push(0x04); // TAG_SHARD_ANNOUNCE
    buf.extend_from_slice(&count.to_le_bytes());
    for id in shard_ids {
        buf.extend_from_slice(&id.0);
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(seed: u8) -> ContentHash {
        ContentHash([seed; 32])
    }

    #[tokio::test]
    async fn test_swarm_provider_register_and_get() {
        let index = ShardLocationIndex::new();
        let h1 = hash(0xAA);
        let h2 = hash(0xBB);

        index.register_provider("node-a", &[h1, h2]).await;

        let providers = index.get_providers(&h1).await;
        assert_eq!(providers.len(), 1);
        assert!(providers.contains(&"node-a".to_string()));

        let providers = index.get_providers(&h2).await;
        assert_eq!(providers.len(), 1);
    }

    #[tokio::test]
    async fn test_swarm_provider_multiple_providers() {
        let index = ShardLocationIndex::new();
        let h1 = hash(0xCC);

        index.register_provider("node-a", &[h1]).await;
        index.register_provider("node-b", &[h1]).await;

        let providers = index.get_providers(&h1).await;
        assert_eq!(providers.len(), 2);
        assert!(providers.contains(&"node-a".to_string()));
        assert!(providers.contains(&"node-b".to_string()));
    }

    #[tokio::test]
    async fn test_swarm_provider_remove() {
        let index = ShardLocationIndex::new();
        let h1 = hash(0xDD);
        let h2 = hash(0xEE);

        index.register_provider("node-a", &[h1, h2]).await;
        index.register_provider("node-b", &[h1]).await;

        index.remove_provider("node-a").await;

        // h1 still has node-b
        let providers = index.get_providers(&h1).await;
        assert_eq!(providers.len(), 1);
        assert!(providers.contains(&"node-b".to_string()));

        // h2 had only node-a, so it should be removed entirely
        let providers = index.get_providers(&h2).await;
        assert!(providers.is_empty());
        assert_eq!(index.shard_count().await, 1);
    }

    #[tokio::test]
    async fn test_swarm_provider_shard_count() {
        let index = ShardLocationIndex::new();
        assert_eq!(index.shard_count().await, 0);

        index.register_provider("n1", &[hash(1), hash(2), hash(3)]).await;
        assert_eq!(index.shard_count().await, 3);
    }

    #[tokio::test]
    async fn test_swarm_provider_provider_count() {
        let index = ShardLocationIndex::new();
        assert_eq!(index.provider_count().await, 0);

        index.register_provider("n1", &[hash(1)]).await;
        index.register_provider("n2", &[hash(1), hash(2)]).await;
        index.register_provider("n3", &[hash(3)]).await;

        assert_eq!(index.provider_count().await, 3);
    }

    #[tokio::test]
    async fn test_swarm_provider_get_unknown_shard() {
        let index = ShardLocationIndex::new();
        let providers = index.get_providers(&hash(0xFF)).await;
        assert!(providers.is_empty());
    }

    #[tokio::test]
    async fn test_swarm_provider_duplicate_register() {
        let index = ShardLocationIndex::new();
        let h1 = hash(0x11);

        // Registering same node twice should not duplicate
        index.register_provider("node-a", &[h1]).await;
        index.register_provider("node-a", &[h1]).await;

        let providers = index.get_providers(&h1).await;
        assert_eq!(providers.len(), 1);
    }

    #[test]
    fn test_build_shard_announce_payload_empty() {
        let payload = build_shard_announce_payload(&[]);
        assert_eq!(payload.len(), 5);
        assert_eq!(payload[0], 0x04); // TAG_SHARD_ANNOUNCE
        assert_eq!(u32::from_le_bytes([payload[1], payload[2], payload[3], payload[4]]), 0);
    }

    #[test]
    fn test_build_shard_announce_payload_wire_format() {
        let h1 = hash(0xAA);
        let h2 = hash(0xBB);
        let payload = build_shard_announce_payload(&[h1, h2]);

        // tag(1) + count(4) + 2*hash(32) = 69
        assert_eq!(payload.len(), 69);
        assert_eq!(payload[0], 0x04);
        assert_eq!(u32::from_le_bytes([payload[1], payload[2], payload[3], payload[4]]), 2);

        // First hash
        assert_eq!(&payload[5..37], &[0xAA; 32]);
        // Second hash
        assert_eq!(&payload[37..69], &[0xBB; 32]);
    }

    #[test]
    fn test_build_shard_announce_payload_roundtrip() {
        let shards = vec![hash(0x01), hash(0x02), hash(0x03)];
        let payload = build_shard_announce_payload(&shards);

        // Parse it back
        assert_eq!(payload[0], 0x04);
        let count = u32::from_le_bytes([payload[1], payload[2], payload[3], payload[4]]) as usize;
        assert_eq!(count, 3);

        for i in 0..count {
            let offset = 5 + i * 32;
            let mut h = [0u8; 32];
            h.copy_from_slice(&payload[offset..offset + 32]);
            assert_eq!(ContentHash(h), shards[i]);
        }
    }
}
