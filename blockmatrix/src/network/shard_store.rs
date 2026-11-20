// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Local shard storage for serving shard requests from peers.
//!
//! Supports content-addressed deduplication with reference counting (R4).

use super::shard_dedup::{DedupPolicy, ShardStoreResult};
use hypermesh_lib::ContentHash;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};

/// Internal shard entry with reference counting for dedup.
struct ShardEntry {
    data: Vec<u8>,
    ref_count: u32,
    mirror_refs: u32,
}

/// Local shard storage with optional disk persistence.
///
/// Holds shards that this node is responsible for serving to peers.
/// Thread-safe via `RwLock`. When `shard_dir` is set, shards are
/// persisted to disk and reloaded on startup.
///
/// Supports privacy-scoped deduplication (R4): `DedupPolicy::Full`
/// increments refcounts for duplicate stores, `DedupPolicy::HashOnly`
/// detects dupes but skips refcount tracking, `DedupPolicy::None`
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
                            map.insert(
                                ContentHash(hash),
                                ShardEntry {
                                    data,
                                    ref_count,
                                    mirror_refs: 0,
                                },
                            );
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

    /// Store with the requested owner dedup policy. Mirror claims are separate.
    pub async fn store_with_dedup(
        &self,
        shard_id: ContentHash,
        data: Vec<u8>,
        policy: DedupPolicy,
    ) -> ShardStoreResult {
        match policy {
            DedupPolicy::Full => self.store_full(shard_id, data),
            DedupPolicy::HashOnly => self.store_hash_only(shard_id, data),
            DedupPolicy::None => self.store_none(shard_id, data),
        }
    }

    fn store_full(&self, shard_id: ContentHash, data: Vec<u8>) -> ShardStoreResult {
        let mut shards = self.shards.write();
        if let Some(entry) = shards.get_mut(&shard_id) {
            if entry.ref_count == 0 {
                self.persist_shard_data(&shard_id, &entry.data);
            }
            entry.ref_count = entry.ref_count.saturating_add(1);
            self.persist_ref_count(&shard_id, entry.ref_count);
            return ShardStoreResult::Deduplicated {
                ref_count: entry.ref_count.saturating_add(entry.mirror_refs),
            };
        }
        self.persist_shard_data(&shard_id, &data);
        self.persist_ref_count(&shard_id, 1);
        shards.insert(
            shard_id,
            ShardEntry {
                data,
                ref_count: 1,
                mirror_refs: 0,
            },
        );
        ShardStoreResult::Stored
    }

    fn store_hash_only(&self, shard_id: ContentHash, data: Vec<u8>) -> ShardStoreResult {
        let mut shards = self.shards.write();
        if let Some(entry) = shards.get_mut(&shard_id) {
            if entry.ref_count > 0 {
                return ShardStoreResult::Deduplicated { ref_count: 1 };
            }
            self.persist_shard_data(&shard_id, &entry.data);
            self.persist_ref_count(&shard_id, 1);
            entry.ref_count = 1;
            return ShardStoreResult::Stored;
        }
        self.persist_shard_data(&shard_id, &data);
        self.persist_ref_count(&shard_id, 1);
        shards.insert(
            shard_id,
            ShardEntry {
                data,
                ref_count: 1,
                mirror_refs: 0,
            },
        );
        ShardStoreResult::Stored
    }

    fn store_none(&self, shard_id: ContentHash, data: Vec<u8>) -> ShardStoreResult {
        let mut shards = self.shards.write();
        if let Some(entry) = shards.get_mut(&shard_id) {
            if entry.mirror_refs > 0 && entry.data != data {
                return ShardStoreResult::Deduplicated {
                    ref_count: entry.ref_count.saturating_add(entry.mirror_refs),
                };
            }
            entry.data = data;
            entry.ref_count = 1;
            self.persist_shard_data(&shard_id, &entry.data);
            self.persist_ref_count(&shard_id, 1);
            return ShardStoreResult::Stored;
        }
        self.persist_shard_data(&shard_id, &data);
        self.persist_ref_count(&shard_id, 1);
        shards.insert(
            shard_id,
            ShardEntry {
                data,
                ref_count: 1,
                mirror_refs: 0,
            },
        );
        ShardStoreResult::Stored
    }

    /// Claim verified mirror bytes under the store lock. Caller holds the index
    /// write lock first; neither lock crosses an await. Mirror-only bytes stay
    /// in RAM and are freed when the final provider claim is removed or expires.
    pub(crate) fn claim_mirror(&self, shard_id: ContentHash, data: &[u8]) -> bool {
        let mut shards = self.shards.write();
        if let Some(entry) = shards.get_mut(&shard_id) {
            if entry.data != data {
                return false;
            }
            let Some(next) = entry.mirror_refs.checked_add(1) else {
                return false;
            };
            entry.mirror_refs = next;
            return true;
        }
        shards.insert(
            shard_id,
            ShardEntry {
                data: data.to_vec(),
                ref_count: 0,
                mirror_refs: 1,
            },
        );
        true
    }

    /// Confirm an existing provider claim still points to these serve bytes.
    pub(crate) fn mirror_bytes_match(&self, shard_id: &ContentHash, data: &[u8]) -> bool {
        self.shards
            .read()
            .get(shard_id)
            .is_some_and(|entry| entry.data == data)
    }

    /// Release one provider-owned mirror hold; remove mirror-only bytes at zero.
    pub(crate) fn release_mirror(&self, shard_id: &ContentHash) {
        let mut shards = self.shards.write();
        let Some(entry) = shards.get_mut(shard_id) else {
            return;
        };
        entry.mirror_refs = entry.mirror_refs.saturating_sub(1);
        if entry.mirror_refs == 0 && entry.ref_count == 0 {
            shards.remove(shard_id);
            self.remove_shard_files(shard_id);
        }
    }

    /// Increment an owner reference without changing any mirror claim.
    pub async fn acquire(&self, shard_id: &ContentHash) -> Option<u32> {
        let mut shards = self.shards.write();
        let entry = shards.get_mut(shard_id)?;
        if entry.ref_count == 0 {
            self.persist_shard_data(shard_id, &entry.data);
        }
        entry.ref_count = entry.ref_count.saturating_add(1);
        self.persist_ref_count(shard_id, entry.ref_count);
        Some(entry.ref_count.saturating_add(entry.mirror_refs))
    }

    /// Release one owner reference; active mirror claims keep bytes servable.
    pub async fn release(&self, shard_id: &ContentHash) -> Option<u32> {
        self.release_owner_now(shard_id)
    }

    pub(crate) fn release_owner_now(&self, shard_id: &ContentHash) -> Option<u32> {
        let mut shards = self.shards.write();
        let entry = shards.get_mut(shard_id)?;
        if entry.ref_count == 0 {
            return None;
        }
        entry.ref_count -= 1;
        let total = entry.ref_count.saturating_add(entry.mirror_refs);
        if entry.ref_count == 0 {
            self.remove_shard_files(shard_id);
            if entry.mirror_refs == 0 {
                shards.remove(shard_id);
            }
        } else {
            self.persist_ref_count(shard_id, entry.ref_count);
        }
        Some(total)
    }

    /// Total owner plus active mirror references for this shard.
    pub async fn ref_count(&self, shard_id: &ContentHash) -> Option<u32> {
        self.shards
            .read()
            .get(shard_id)
            .map(|e| e.ref_count.saturating_add(e.mirror_refs))
    }

    /// Remove owner references; an active mirror claim defers byte removal.
    pub async fn remove(&self, shard_id: &ContentHash) {
        self.remove_owner_now(shard_id);
    }

    pub(crate) fn remove_owner_now(&self, shard_id: &ContentHash) {
        let mut shards = self.shards.write();
        if let Some(entry) = shards.get_mut(shard_id) {
            if entry.mirror_refs > 0 {
                entry.ref_count = 0;
                self.remove_shard_files(shard_id);
                return;
            }
        }
        shards.remove(shard_id);
        self.remove_shard_files(shard_id);
    }

    /// Retrieve bytes currently held by an owner or active mirror.
    pub async fn get(&self, shard_id: &ContentHash) -> Option<Vec<u8>> {
        self.shards.read().get(shard_id).map(|e| e.data.clone())
    }

    /// Check if a shard is currently held.
    pub async fn has(&self, shard_id: &ContentHash) -> bool {
        self.shards.read().contains_key(shard_id)
    }

    /// Number of currently held shards.
    pub async fn count(&self) -> usize {
        self.shards.read().len()
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
                    warn!("Failed to remove refcount file {}: {e}", ref_path.display());
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
#[path = "tests/mirror_store.rs"]
mod tests;
