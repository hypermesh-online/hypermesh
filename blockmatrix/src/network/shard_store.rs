// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Local shard storage for serving shard requests from peers.
//!
//! Supports content-addressed deduplication with reference counting (R4).

use super::shard_dedup::{DedupPolicy, ShardStoreResult};
use hypermesh_lib::ContentHash;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Internal shard entry with reference counting for dedup.
struct ShardEntry {
    data: Vec<u8>,
    ref_count: u32,
}

/// Local shard storage with optional disk persistence.
///
/// Holds shards that this node is responsible for serving to peers.
/// Thread-safe via `RwLock`. When `shard_dir` is set, shards are
/// persisted to disk and reloaded on startup.
///
/// Supports privacy-scoped deduplication (R4): `DedupPolicy::Full`
/// increments refcounts for duplicate stores, `DedupPolicy::None`
/// always overwrites with refcount 1.
pub struct ShardStore {
    shards: Arc<RwLock<HashMap<ContentHash, ShardEntry>>>,
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
    /// filenames) and loads them into the in-memory map with refcount 1.
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
                            // Load refcount from companion .ref file, default to 1.
                            let ref_count = Self::load_ref_count(&dir, &name_str);
                            map.insert(ContentHash(hash), ShardEntry { data, ref_count });
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

    /// Store a shard locally with default `Full` dedup policy.
    ///
    /// Backward-compatible: existing callers get dedup behavior (idempotent
    /// store with refcount increment).
    pub async fn store(&self, shard_id: ContentHash, data: Vec<u8>) {
        self.store_with_dedup(shard_id, data, DedupPolicy::Full)
            .await;
    }

    /// Store a shard with dedup policy awareness.
    ///
    /// - `Full`: if shard exists, increment refcount and return `Deduplicated`.
    /// - `None`: always overwrite with refcount 1 (no cross-asset correlation).
    pub async fn store_with_dedup(
        &self,
        shard_id: ContentHash,
        data: Vec<u8>,
        policy: DedupPolicy,
    ) -> ShardStoreResult {
        debug!(
            "Storing shard {} (policy={:?})",
            hex::encode(shard_id.0),
            policy
        );

        let mut shards = self.shards.write().await;

        let result = match policy {
            DedupPolicy::Full => {
                if let Some(entry) = shards.get_mut(&shard_id) {
                    entry.ref_count = entry.ref_count.saturating_add(1);
                    let rc = entry.ref_count;
                    self.persist_ref_count(&shard_id, rc);
                    ShardStoreResult::Deduplicated { ref_count: rc }
                } else {
                    self.persist_shard_data(&shard_id, &data);
                    self.persist_ref_count(&shard_id, 1);
                    shards.insert(shard_id, ShardEntry { data, ref_count: 1 });
                    ShardStoreResult::Stored
                }
            }
            DedupPolicy::None => {
                // Anonymous: always store fresh with refcount 1 (no dedup).
                self.persist_shard_data(&shard_id, &data);
                self.persist_ref_count(&shard_id, 1);
                shards.insert(shard_id, ShardEntry { data, ref_count: 1 });
                ShardStoreResult::Stored
            }
        };

        result
    }

    /// Increment reference count for an existing shard.
    ///
    /// Returns new refcount, or `None` if shard doesn't exist.
    pub async fn acquire(&self, shard_id: &ContentHash) -> Option<u32> {
        let mut shards = self.shards.write().await;
        let entry = shards.get_mut(shard_id)?;
        entry.ref_count = entry.ref_count.saturating_add(1);
        let rc = entry.ref_count;
        self.persist_ref_count(shard_id, rc);
        Some(rc)
    }

    /// Decrement reference count. Removes shard data when refcount reaches 0.
    ///
    /// Returns new refcount (0 means removed), or `None` if shard doesn't exist.
    pub async fn release(&self, shard_id: &ContentHash) -> Option<u32> {
        let mut shards = self.shards.write().await;
        let entry = shards.get_mut(shard_id)?;

        if entry.ref_count <= 1 {
            shards.remove(shard_id);
            self.remove_shard_files(shard_id);
            Some(0)
        } else {
            entry.ref_count -= 1;
            let rc = entry.ref_count;
            self.persist_ref_count(shard_id, rc);
            Some(rc)
        }
    }

    /// Get the current reference count for a shard.
    pub async fn ref_count(&self, shard_id: &ContentHash) -> Option<u32> {
        self.shards
            .read()
            .await
            .get(shard_id)
            .map(|e| e.ref_count)
    }

    /// Force-remove a shard from memory and disk (ignores refcount).
    pub async fn remove(&self, shard_id: &ContentHash) {
        self.shards.write().await.remove(shard_id);
        self.remove_shard_files(shard_id);
    }

    /// Retrieve a shard by its content hash.
    pub async fn get(&self, shard_id: &ContentHash) -> Option<Vec<u8>> {
        self.shards
            .read()
            .await
            .get(shard_id)
            .map(|e| e.data.clone())
    }

    /// Check if a shard exists.
    pub async fn has(&self, shard_id: &ContentHash) -> bool {
        self.shards.read().await.contains_key(shard_id)
    }

    /// Number of stored shards.
    pub async fn count(&self) -> usize {
        self.shards.read().await.len()
    }

    // ── Disk persistence helpers ────────────────────────────────────

    /// Persist shard data to disk (if configured).
    fn persist_shard_data(&self, shard_id: &ContentHash, data: &[u8]) {
        if let Some(ref dir) = self.shard_dir {
            let path = dir.join(hex::encode(shard_id.0));
            if let Err(e) = std::fs::write(&path, data) {
                warn!("Failed to persist shard to {}: {e}", path.display());
            }
        }
    }

    /// Persist refcount to a companion `.ref` file.
    fn persist_ref_count(&self, shard_id: &ContentHash, ref_count: u32) {
        if let Some(ref dir) = self.shard_dir {
            let path = dir.join(format!("{}.ref", hex::encode(shard_id.0)));
            if let Err(e) = std::fs::write(&path, ref_count.to_le_bytes()) {
                warn!("Failed to persist refcount to {}: {e}", path.display());
            }
        }
    }

    /// Load refcount from companion `.ref` file, default 1 if missing.
    fn load_ref_count(dir: &PathBuf, hex_hash: &str) -> u32 {
        let path = dir.join(format!("{hex_hash}.ref"));
        match std::fs::read(&path) {
            Ok(bytes) if bytes.len() == 4 => {
                u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
            }
            _ => 1,
        }
    }

    /// Remove shard data file and companion `.ref` file from disk.
    fn remove_shard_files(&self, shard_id: &ContentHash) {
        if let Some(ref dir) = self.shard_dir {
            let hex_name = hex::encode(shard_id.0);
            let data_path = dir.join(&hex_name);
            if let Err(e) = std::fs::remove_file(&data_path) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    warn!("Failed to remove shard file {}: {e}", data_path.display());
                }
            }
            let ref_path = dir.join(format!("{hex_name}.ref"));
            if let Err(e) = std::fs::remove_file(&ref_path) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    warn!(
                        "Failed to remove refcount file {}: {e}",
                        ref_path.display()
                    );
                }
            }
        }
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

    // ── Backward-compatible tests (original API) ────────────────────

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

    // ── Dedup-aware tests ───────────────────────────────────────────

    #[tokio::test]
    async fn test_store_with_dedup_full_new_shard() {
        let store = ShardStore::new();
        let hash = test_hash(50);
        let result = store
            .store_with_dedup(hash, vec![0xAA; 64], DedupPolicy::Full)
            .await;
        assert_eq!(result, ShardStoreResult::Stored);
        assert_eq!(store.ref_count(&hash).await, Some(1));
    }

    #[tokio::test]
    async fn test_store_with_dedup_full_duplicate() {
        let store = ShardStore::new();
        let hash = test_hash(51);

        let r1 = store
            .store_with_dedup(hash, vec![0xBB; 64], DedupPolicy::Full)
            .await;
        assert_eq!(r1, ShardStoreResult::Stored);

        let r2 = store
            .store_with_dedup(hash, vec![0xBB; 64], DedupPolicy::Full)
            .await;
        assert_eq!(r2, ShardStoreResult::Deduplicated { ref_count: 2 });
        assert_eq!(store.ref_count(&hash).await, Some(2));

        // Data is still accessible
        assert_eq!(store.get(&hash).await, Some(vec![0xBB; 64]));
    }

    #[tokio::test]
    async fn test_store_with_dedup_none_always_overwrites() {
        let store = ShardStore::new();
        let hash = test_hash(52);

        // First store
        let r1 = store
            .store_with_dedup(hash, vec![0xCC; 32], DedupPolicy::None)
            .await;
        assert_eq!(r1, ShardStoreResult::Stored);
        assert_eq!(store.ref_count(&hash).await, Some(1));

        // Second store with None: overwrites, refcount stays 1
        let r2 = store
            .store_with_dedup(hash, vec![0xCC; 32], DedupPolicy::None)
            .await;
        assert_eq!(r2, ShardStoreResult::Stored);
        assert_eq!(store.ref_count(&hash).await, Some(1));
    }

    #[tokio::test]
    async fn test_acquire_increments_refcount() {
        let store = ShardStore::new();
        let hash = test_hash(53);
        store.store(hash, vec![1]).await;
        assert_eq!(store.ref_count(&hash).await, Some(1));

        let rc = store.acquire(&hash).await;
        assert_eq!(rc, Some(2));
        assert_eq!(store.ref_count(&hash).await, Some(2));

        let rc = store.acquire(&hash).await;
        assert_eq!(rc, Some(3));
    }

    #[tokio::test]
    async fn test_acquire_nonexistent_returns_none() {
        let store = ShardStore::new();
        let hash = test_hash(54);
        assert_eq!(store.acquire(&hash).await, None);
    }

    #[tokio::test]
    async fn test_release_decrements_refcount() {
        let store = ShardStore::new();
        let hash = test_hash(55);
        store.store(hash, vec![1]).await;

        // Bump to refcount 3
        store.acquire(&hash).await;
        store.acquire(&hash).await;
        assert_eq!(store.ref_count(&hash).await, Some(3));

        // Release: 3 -> 2
        let rc = store.release(&hash).await;
        assert_eq!(rc, Some(2));
        assert!(store.has(&hash).await);

        // Release: 2 -> 1
        let rc = store.release(&hash).await;
        assert_eq!(rc, Some(1));
        assert!(store.has(&hash).await);

        // Release: 1 -> 0 (removed)
        let rc = store.release(&hash).await;
        assert_eq!(rc, Some(0));
        assert!(!store.has(&hash).await);
        assert_eq!(store.get(&hash).await, None);
    }

    #[tokio::test]
    async fn test_release_nonexistent_returns_none() {
        let store = ShardStore::new();
        let hash = test_hash(56);
        assert_eq!(store.release(&hash).await, None);
    }

    #[tokio::test]
    async fn test_ref_count_returns_none_for_missing() {
        let store = ShardStore::new();
        let hash = test_hash(57);
        assert_eq!(store.ref_count(&hash).await, None);
    }

    #[tokio::test]
    async fn test_force_remove_ignores_refcount() {
        let store = ShardStore::new();
        let hash = test_hash(58);
        store.store(hash, vec![1]).await;
        store.acquire(&hash).await;
        store.acquire(&hash).await;
        assert_eq!(store.ref_count(&hash).await, Some(3));

        // Force remove: gone regardless of refcount
        store.remove(&hash).await;
        assert!(!store.has(&hash).await);
        assert_eq!(store.ref_count(&hash).await, None);
    }

    #[tokio::test]
    async fn test_backward_compat_store_creates_refcount_1() {
        let store = ShardStore::new();
        let hash = test_hash(59);
        store.store(hash, vec![0xFF]).await;
        assert_eq!(store.ref_count(&hash).await, Some(1));
    }

    #[tokio::test]
    async fn test_backward_compat_store_deduplicates() {
        let store = ShardStore::new();
        let hash = test_hash(60);
        store.store(hash, vec![0xFF]).await;
        store.store(hash, vec![0xFF]).await;
        // Default store uses Full dedup, so refcount increments
        assert_eq!(store.ref_count(&hash).await, Some(2));
    }

    #[tokio::test]
    async fn test_disk_persistence_with_refcount() {
        let tmp = std::env::temp_dir().join(format!(
            "shard_store_test_refcount_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);

        let hash = test_hash(70);

        // Store with refcount 3
        {
            let store = ShardStore::new_with_dir(tmp.clone());
            store.store(hash, vec![0xAA]).await;
            store.acquire(&hash).await;
            store.acquire(&hash).await;
            assert_eq!(store.ref_count(&hash).await, Some(3));
        }

        // Reload from disk — refcount should survive
        {
            let store = ShardStore::new_with_dir(tmp.clone());
            assert_eq!(store.ref_count(&hash).await, Some(3));
            assert_eq!(store.get(&hash).await, Some(vec![0xAA]));
        }

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[tokio::test]
    async fn test_release_removes_disk_files() {
        let tmp = std::env::temp_dir().join(format!(
            "shard_store_test_release_disk_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);

        let store = ShardStore::new_with_dir(tmp.clone());
        let hash = test_hash(71);
        let hex_name = hex::encode(hash.0);

        store.store(hash, vec![1, 2]).await;
        assert!(tmp.join(&hex_name).exists());
        assert!(tmp.join(format!("{hex_name}.ref")).exists());

        // Release to 0 — both files should be removed
        let rc = store.release(&hash).await;
        assert_eq!(rc, Some(0));
        assert!(!tmp.join(&hex_name).exists());
        assert!(!tmp.join(format!("{hex_name}.ref")).exists());

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
