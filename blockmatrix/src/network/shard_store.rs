// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Local shard storage for serving shard requests from peers.

use hypermesh_lib::ContentHash;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

/// Local shard storage.
///
/// Holds shards that this node is responsible for serving to peers.
/// Thread-safe via `RwLock`.
pub struct ShardStore {
    shards: Arc<RwLock<HashMap<ContentHash, Vec<u8>>>>,
}

impl ShardStore {
    pub fn new() -> Self {
        Self {
            shards: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store a shard locally.
    pub async fn store(&self, shard_id: ContentHash, data: Vec<u8>) {
        debug!("Storing shard {}", hex::encode(shard_id.0));
        self.shards.write().await.insert(shard_id, data);
    }

    /// Retrieve a shard by its content hash.
    pub async fn get(&self, shard_id: &ContentHash) -> Option<Vec<u8>> {
        self.shards.read().await.get(shard_id).cloned()
    }

    /// Check if a shard exists.
    pub async fn has(&self, shard_id: &ContentHash) -> bool {
        self.shards.read().await.contains_key(shard_id)
    }

    /// Number of stored shards.
    pub async fn count(&self) -> usize {
        self.shards.read().await.len()
    }
}

impl Default for ShardStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_hash(seed: u8) -> ContentHash {
        ContentHash([seed; 32])
    }

    #[tokio::test]
    async fn test_store_and_get() {
        let store = ShardStore::new();
        let hash = test_hash(1);
        let data = vec![0xAB; 256];

        store.store(hash, data.clone()).await;

        let fetched = store.get(&hash).await;
        assert_eq!(fetched, Some(data));
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let store = ShardStore::new();
        let hash = test_hash(2);
        assert_eq!(store.get(&hash).await, None);
    }

    #[tokio::test]
    async fn test_has() {
        let store = ShardStore::new();
        let hash = test_hash(3);
        assert!(!store.has(&hash).await);

        store.store(hash, vec![1, 2, 3]).await;
        assert!(store.has(&hash).await);
    }

    #[tokio::test]
    async fn test_count() {
        let store = ShardStore::new();
        assert_eq!(store.count().await, 0);

        store.store(test_hash(10), vec![1]).await;
        store.store(test_hash(11), vec![2]).await;
        assert_eq!(store.count().await, 2);
    }

    #[tokio::test]
    async fn test_default() {
        let store = ShardStore::default();
        assert_eq!(store.count().await, 0);
    }
}
