//! Simplified Certificate Transparency Storage
//! 
//! In-memory storage implementation for CT logs (for initial build success)

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};

use crate::errors::{StorageError, Result as TrustChainResult};
use super::LogEntry;

/// Simplified CT storage using in-memory storage
pub struct SimpleCTStorage {
    /// Storage path (for future persistence)
    storage_path: String,
    /// In-memory entry storage
    entries: Arc<RwLock<HashMap<u64, LogEntry>>>,
    /// Fingerprint index
    fingerprint_index: Arc<RwLock<HashMap<[u8; 32], u64>>>,
    /// Next sequence number
    next_sequence: Arc<RwLock<u64>>,
}

impl SimpleCTStorage {
    /// Create new simplified storage
    pub async fn new(storage_path: &str) -> TrustChainResult<Self> {
        Ok(Self {
            storage_path: storage_path.to_string(),
            entries: Arc::new(RwLock::new(HashMap::new())),
            fingerprint_index: Arc::new(RwLock::new(HashMap::new())),
            next_sequence: Arc::new(RwLock::new(0)),
        })
    }

    /// Store a log entry
    pub async fn store_entry(&self, entry: &LogEntry) -> TrustChainResult<()> {
        let mut entries = self.entries.write().await;
        let mut fingerprint_index = self.fingerprint_index.write().await;

        entries.insert(entry.sequence_number, entry.clone());
        fingerprint_index.insert(entry.fingerprint, entry.sequence_number);

        Ok(())
    }

    /// Get entry by sequence number
    pub async fn get_entry_by_sequence(&self, sequence_number: u64) -> TrustChainResult<Option<LogEntry>> {
        let entries = self.entries.read().await;
        Ok(entries.get(&sequence_number).cloned())
    }

    /// Get entry by fingerprint
    pub async fn get_entry_by_fingerprint(&self, fingerprint: &[u8; 32]) -> TrustChainResult<Option<LogEntry>> {
        let fingerprint_index = self.fingerprint_index.read().await;
        let entries = self.entries.read().await;

        if let Some(&sequence_number) = fingerprint_index.get(fingerprint) {
            Ok(entries.get(&sequence_number).cloned())
        } else {
            Ok(None)
        }
    }

    /// Get entries in range
    pub async fn get_entries_range(&self, start: u64, end: u64) -> TrustChainResult<Vec<LogEntry>> {
        let entries = self.entries.read().await;
        let mut result = Vec::new();

        for seq in start..end {
            if let Some(entry) = entries.get(&seq) {
                result.push(entry.clone());
            }
        }

        Ok(result)
    }

    /// Get next sequence number
    pub async fn get_next_sequence_number(&self) -> TrustChainResult<u64> {
        let next_sequence = self.next_sequence.read().await;
        Ok(*next_sequence)
    }

    /// Reserve sequence number
    pub async fn reserve_sequence_number(&self) -> TrustChainResult<u64> {
        let mut next_sequence = self.next_sequence.write().await;
        let current = *next_sequence;
        *next_sequence += 1;
        Ok(current)
    }

    /// Get entries by common name
    pub async fn get_entries_by_common_name(&self, common_name: &str) -> TrustChainResult<Vec<LogEntry>> {
        let entries = self.entries.read().await;
        let mut result = Vec::new();

        for entry in entries.values() {
            if entry.common_name == common_name {
                result.push(entry.clone());
            }
        }

        Ok(result)
    }

    /// Get entries by CA ID
    pub async fn get_entries_by_ca_id(&self, ca_id: &str) -> TrustChainResult<Vec<LogEntry>> {
        let entries = self.entries.read().await;
        let mut result = Vec::new();

        for entry in entries.values() {
            if entry.issuer_ca_id == ca_id {
                result.push(entry.clone());
            }
        }

        Ok(result)
    }

    /// Get storage statistics
    pub async fn get_stats(&self) -> TrustChainResult<StorageStats> {
        let entries = self.entries.read().await;
        let total_entries = entries.len() as u64;

        // Find last entry timestamp
        let last_entry_timestamp = entries.values()
            .map(|entry| entry.timestamp)
            .max();

        Ok(StorageStats {
            total_entries,
            database_size_bytes: total_entries * 1024, // Rough estimate
            last_entry_timestamp,
            storage_path: self.storage_path.clone(),
            index_count: 2, // fingerprint and sequence indexes
        })
    }

    /// Maintenance (no-op for in-memory)
    pub async fn maintenance(&self) -> TrustChainResult<()> {
        Ok(())
    }

    /// Flush (no-op for in-memory)
    pub async fn flush(&self) -> TrustChainResult<()> {
        Ok(())
    }
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