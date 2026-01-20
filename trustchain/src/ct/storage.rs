//! Certificate Transparency Storage Backend
//!
//! Persistent storage for CT log entries using BlockMatrix asset storage,
//! efficient indexing, and data integrity verification.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
use anyhow::{Result, anyhow};
use tracing::{debug, info, warn, error};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::fs;

use crate::errors::{StorageError, Result as TrustChainResult};
use super::LogEntry;

/// CT storage backend using file-based blockchain storage
pub struct CTStorage {
    /// Storage directory path
    storage_path: PathBuf,
    /// In-memory index for fast lookups
    entries: Arc<RwLock<BTreeMap<u64, LogEntry>>>,
    /// Next sequence number
    next_sequence: Arc<std::sync::atomic::AtomicU64>,
}

/// Storage statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_entries: u64,
    pub database_size_bytes: u64,
    pub last_entry_timestamp: Option<SystemTime>,
    pub storage_path: String,
    pub index_count: u32,
}

impl CTStorage {
    /// Create new CT storage
    pub async fn new(storage_path: &str) -> TrustChainResult<Self> {
        info!("Initializing CT storage: {}", storage_path);

        let path = PathBuf::from(storage_path);

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| StorageError::FileSystem {
                    path: parent.to_string_lossy().to_string(),
                    reason: e.to_string(),
                })?;
        }

        // Load existing entries
        let entries = Arc::new(RwLock::new(BTreeMap::new()));
        let next_sequence = Arc::new(std::sync::atomic::AtomicU64::new(0));

        // Load index file if it exists
        let index_file = path.join("ct_index.json");
        if index_file.exists() {
            let data = tokio::fs::read_to_string(&index_file).await
                .map_err(|e| StorageError::FileSystem {
                    path: index_file.to_string_lossy().to_string(),
                    reason: e.to_string(),
                })?;

            if let Ok(loaded_entries) = serde_json::from_str::<BTreeMap<u64, LogEntry>>(&data) {
                let max_seq = loaded_entries.keys().max().copied().unwrap_or(0);
                next_sequence.store(max_seq + 1, std::sync::atomic::Ordering::SeqCst);
                *entries.write().await = loaded_entries;
            }
        }

        Ok(Self {
            storage_path: path,
            entries,
            next_sequence,
        })
    }

    /// Store a new log entry
    pub async fn store_entry(&self, entry: LogEntry) -> TrustChainResult<u64> {
        let sequence = self.next_sequence.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        // Store in memory
        self.entries.write().await.insert(sequence, entry.clone());

        // Persist to disk
        self.persist_index().await?;

        debug!("Stored CT log entry: sequence={}", sequence);
        Ok(sequence)
    }

    /// Get entry by sequence number
    pub async fn get_entry(&self, sequence: u64) -> TrustChainResult<Option<LogEntry>> {
        Ok(self.entries.read().await.get(&sequence).cloned())
    }

    /// Get entries in range
    pub async fn get_entries_range(&self, start: u64, end: u64) -> TrustChainResult<Vec<LogEntry>> {
        let entries = self.entries.read().await;
        let range: Vec<LogEntry> = entries
            .range(start..=end)
            .map(|(_, entry)| entry.clone())
            .collect();
        Ok(range)
    }

    /// Get latest entries
    pub async fn get_latest_entries(&self, count: u64) -> TrustChainResult<Vec<LogEntry>> {
        let entries = self.entries.read().await;
        let latest: Vec<LogEntry> = entries
            .iter()
            .rev()
            .take(count as usize)
            .map(|(_, entry)| entry.clone())
            .collect();
        Ok(latest)
    }

    /// Search entries by certificate hash
    pub async fn search_by_cert_hash(&self, cert_hash: &[u8]) -> TrustChainResult<Vec<LogEntry>> {
        let entries = self.entries.read().await;
        let matching: Vec<LogEntry> = entries
            .values()
            .filter(|entry| {
                // Check if certificate hash matches
                // This would need to be implemented based on LogEntry structure
                true // Placeholder
            })
            .cloned()
            .collect();
        Ok(matching)
    }

    /// Get storage statistics
    pub async fn get_stats(&self) -> TrustChainResult<StorageStats> {
        let entries = self.entries.read().await;
        let total_entries = entries.len() as u64;

        // Calculate approximate size
        let database_size_bytes = total_entries * 1024; // Rough estimate

        let last_entry_timestamp = entries
            .iter()
            .last()
            .map(|(_, _)| SystemTime::now()); // Would need actual timestamp from entry

        Ok(StorageStats {
            total_entries,
            database_size_bytes,
            last_entry_timestamp,
            storage_path: self.storage_path.to_string_lossy().to_string(),
            index_count: 1,
        })
    }

    /// Clear all entries (dangerous!)
    pub async fn clear(&self) -> TrustChainResult<()> {
        warn!("Clearing all CT log entries!");
        self.entries.write().await.clear();
        self.next_sequence.store(0, std::sync::atomic::Ordering::SeqCst);
        self.persist_index().await?;
        Ok(())
    }

    /// Persist index to disk
    async fn persist_index(&self) -> TrustChainResult<()> {
        let index_file = self.storage_path.join("ct_index.json");
        let entries = self.entries.read().await;

        let data = serde_json::to_string_pretty(&*entries)
            .map_err(|e| StorageError::Serialization {
                reason: e.to_string(),
            })?;

        tokio::fs::write(&index_file, data).await
            .map_err(|e| StorageError::FileSystem {
                path: index_file.to_string_lossy().to_string(),
                reason: e.to_string(),
            })?;

        Ok(())
    }

    /// Verify data integrity
    pub async fn verify_integrity(&self) -> TrustChainResult<bool> {
        // In a real implementation, this would verify merkle tree hashes
        // For now, just check that entries are sequential
        let entries = self.entries.read().await;
        let mut expected_seq = 0u64;

        for &seq in entries.keys() {
            if seq != expected_seq {
                error!("Integrity check failed: missing sequence {}", expected_seq);
                return Ok(false);
            }
            expected_seq += 1;
        }

        info!("Integrity check passed for {} entries", entries.len());
        Ok(true)
    }

    /// Compact and optimize storage
    pub async fn compact(&self) -> TrustChainResult<()> {
        info!("Compacting CT storage");
        // Re-persist to optimize file size
        self.persist_index().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_storage_basic() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join("ct_storage");

        let storage = CTStorage::new(storage_path.to_str().unwrap()).await.unwrap();

        // Create a test log entry
        let entry = LogEntry::default(); // Would need actual LogEntry creation

        // Store entry
        let seq = storage.store_entry(entry.clone()).await.unwrap();
        assert_eq!(seq, 0);

        // Retrieve entry
        let retrieved = storage.get_entry(seq).await.unwrap();
        assert!(retrieved.is_some());

        // Get stats
        let stats = storage.get_stats().await.unwrap();
        assert_eq!(stats.total_entries, 1);
    }

    #[tokio::test]
    async fn test_storage_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join("ct_storage");

        // Create and populate storage
        {
            let storage = CTStorage::new(storage_path.to_str().unwrap()).await.unwrap();
            let entry = LogEntry::default();
            storage.store_entry(entry).await.unwrap();
        }

        // Reload storage
        {
            let storage = CTStorage::new(storage_path.to_str().unwrap()).await.unwrap();
            let stats = storage.get_stats().await.unwrap();
            assert_eq!(stats.total_entries, 1);
        }
    }
}