// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Local shard storage for serving shard requests from peers.

use hypermesh_lib::ContentHash;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Local shard storage with optional disk persistence.
///
/// Holds shards that this node is responsible for serving to peers.
/// Thread-safe via `RwLock`. When `shard_dir` is set, shards are
/// persisted to disk and reloaded on startup.
pub struct ShardStore {
    shards: Arc<RwLock<HashMap<ContentHash, Vec<u8>>>>,
    shard_dir: Option<PathBuf>,
}

impl ShardStore {
    /// Create an in-memory-only shard store (no disk persistence).
    pub fn new() -> Self {
        Self {
            shards: Arc::new(RwLock::new(HashMap::new())),
            shard_dir: None,
        }
    }

    /// Create a shard store backed by `dir` on disk.
    ///
    /// Scans the directory for existing shard files (hex-encoded BLAKE3
    /// filenames) and loads them into the in-memory map.
    pub fn new_with_dir(dir: PathBuf) -> Self {
        if let Err(e) = std::fs::create_dir_all(&dir) {
            warn!("Failed to create shard directory {}: {e}", dir.display());
        }

        let mut map = HashMap::new();
        match std::fs::read_dir(&dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if name_str.len() != 64 {
                        continue;
                    }
                    let Ok(bytes) = hex::decode(name_str.as_ref()) else {
                        continue;
                    };
                    if bytes.len() != 32 {
                        continue;
                    }
                    match std::fs::read(entry.path()) {
                        Ok(data) => {
                            let mut hash = [0u8; 32];
                            hash.copy_from_slice(&bytes);
                            map.insert(ContentHash(hash), data);
                        }
                        Err(e) => {
                            warn!("Failed to read shard file {}: {e}", name_str);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to read shard directory {}: {e}", dir.display());
            }
        }

        info!("Loaded {} shards from {}", map.len(), dir.display());

        Self {
            shards: Arc::new(RwLock::new(map)),
            shard_dir: Some(dir),
        }
    }

    /// Store a shard locally (and persist to disk if configured).
    pub async fn store(&self, shard_id: ContentHash, data: Vec<u8>) {
        debug!("Storing shard {}", hex::encode(shard_id.0));
        if let Some(ref dir) = self.shard_dir {
            let path = dir.join(hex::encode(shard_id.0));
            if let Err(e) = std::fs::write(&path, &data) {
                warn!("Failed to persist shard to {}: {e}", path.display());
            }
        }
        self.shards.write().await.insert(shard_id, data);
    }

    /// Remove a shard from memory and disk.
    pub async fn remove(&self, shard_id: &ContentHash) {
        self.shards.write().await.remove(shard_id);
        if let Some(ref dir) = self.shard_dir {
            let path = dir.join(hex::encode(shard_id.0));
            if let Err(e) = std::fs::remove_file(&path) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    warn!("Failed to remove shard file {}: {e}", path.display());
                }
            }
        }
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

    #[tokio::test]
    async fn test_disk_persistence_roundtrip() {
        let tmp = std::env::temp_dir().join(format!(
            "shard_store_test_roundtrip_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);

        // Store a shard with disk backing
        {
            let store = ShardStore::new_with_dir(tmp.clone());
            let hash = test_hash(42);
            store.store(hash, vec![0xDE, 0xAD]).await;
            assert_eq!(store.count().await, 1);
        }

        // Create a new store from the same directory — shard should reload
        {
            let store = ShardStore::new_with_dir(tmp.clone());
            assert_eq!(store.count().await, 1);
            let fetched = store.get(&test_hash(42)).await;
            assert_eq!(fetched, Some(vec![0xDE, 0xAD]));
        }

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn test_disk_store_and_remove() {
        let tmp = std::env::temp_dir().join(format!(
            "shard_store_test_remove_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);

        let store = ShardStore::new_with_dir(tmp.clone());
        let hash = test_hash(99);
        let hex_name = hex::encode(hash.0);

        // Store and verify file exists on disk
        store.store(hash, vec![1, 2, 3]).await;
        assert!(tmp.join(&hex_name).exists());

        // Remove and verify file is gone
        store.remove(&hash).await;
        assert!(!store.has(&hash).await);
        assert!(!tmp.join(&hex_name).exists());

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
