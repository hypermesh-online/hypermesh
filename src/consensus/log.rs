//! Replicated log implementation for Raft consensus

use super::{
    Term, LogIndex,
    storage::StorageEngine,
    error::{ConsensusError, Result},
};

use serde::{Serialize, Deserialize};
use std::sync::Arc;
use sha2::{Sha256, Digest};
use tracing::{debug, warn};

/// A single log entry in the replicated log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Index of this entry in the log
    pub index: LogIndex,
    
    /// Term when entry was created
    pub term: Term,
    
    /// Entry data payload
    pub data: Vec<u8>,
    
    /// Timestamp when entry was created
    pub timestamp: chrono::DateTime<chrono::Utc>,
    
    /// Checksum for integrity verification
    pub checksum: Vec<u8>,
}

impl LogEntry {
    /// Create a new log entry
    pub fn new(index: LogIndex, term: Term, data: Vec<u8>) -> Self {
        let timestamp = chrono::Utc::now();
        let checksum = Self::compute_checksum(&index, &term, &data, &timestamp);
        
        Self {
            index,
            term,
            data,
            timestamp,
            checksum,
        }
    }
    
    /// Compute checksum for integrity verification
    fn compute_checksum(
        index: &LogIndex,
        term: &Term,
        data: &[u8],
        timestamp: &chrono::DateTime<chrono::Utc>,
    ) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(&index.value().to_le_bytes());
        hasher.update(&term.value().to_le_bytes());
        hasher.update(data);
        hasher.update(&timestamp.timestamp().to_le_bytes());
        hasher.finalize().to_vec()
    }
    
    /// Verify the integrity of this entry
    pub fn verify_integrity(&self) -> bool {
        let expected_checksum = Self::compute_checksum(&self.index, &self.term, &self.data, &self.timestamp);
        self.checksum == expected_checksum
    }
    
    /// Get the size of this entry in bytes
    pub fn size(&self) -> usize {
        std::mem::size_of::<u64>() * 2 + // index + term
        self.data.len() +
        std::mem::size_of::<i64>() + // timestamp
        self.checksum.len()
    }
}

/// Replicated log for Raft consensus
pub struct ReplicatedLog {
    /// All log entries
    entries: Vec<LogEntry>,
    
    /// Index of highest log entry known to be committed
    commit_index: LogIndex,
    
    /// Index of highest log entry applied to state machine
    last_applied: LogIndex,
    
    /// Storage backend for persistence
    storage: Arc<dyn StorageEngine>,
    
    /// Current log size in bytes
    size_bytes: usize,
}

impl ReplicatedLog {
    /// Create a new replicated log
    pub async fn new(storage: Arc<dyn StorageEngine>) -> Result<Self> {
        let mut log = Self {
            entries: Vec::new(),
            commit_index: LogIndex::new(0),
            last_applied: LogIndex::new(0),
            storage,
            size_bytes: 0,
        };
        
        // Load existing log entries from storage
        log.load_from_storage().await?;
        
        Ok(log)
    }
    
    /// Get the number of entries in the log
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    
    /// Check if the log is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    
    /// Get the size of the log in bytes
    pub fn size_bytes(&self) -> usize {
        self.size_bytes
    }
    
    /// Get the commit index
    pub fn commit_index(&self) -> LogIndex {
        self.commit_index
    }
    
    /// Get the last applied index
    pub fn last_applied(&self) -> LogIndex {
        self.last_applied
    }
    
    /// Get information about the last entry
    pub async fn last_entry_info(&self) -> (LogIndex, Term) {
        if let Some(entry) = self.entries.last() {
            (entry.index, entry.term)
        } else {
            (LogIndex::new(0), Term::new(0))
        }
    }
    
    /// Append a new entry to the log
    pub async fn append_entry(&mut self, mut entry: LogEntry) -> Result<LogIndex> {
        // Set the correct index
        let next_index = LogIndex::new(self.entries.len() as u64 + 1);
        entry.index = next_index;
        
        // Recompute checksum with correct index
        entry.checksum = LogEntry::compute_checksum(&entry.index, &entry.term, &entry.data, &entry.timestamp);
        
        // Verify integrity
        if !entry.verify_integrity() {
            return Err(ConsensusError::LogError("Entry failed integrity check".to_string()));
        }
        
        // Update size tracking
        self.size_bytes += entry.size();
        
        // Add to log
        self.entries.push(entry.clone());
        
        // Persist to storage
        self.persist_entry(&entry).await?;
        
        debug!("Appended log entry at index {}", next_index.value());
        Ok(next_index)
    }
    
    /// Insert an entry at a specific position (for replication)
    pub async fn insert_entry(&mut self, entry: LogEntry) -> Result<()> {
        let index = entry.index.value() as usize;
        
        // Verify integrity
        if !entry.verify_integrity() {
            return Err(ConsensusError::LogError("Entry failed integrity check".to_string()));
        }
        
        // If inserting beyond current log, extend with empty entries
        while self.entries.len() < index {
            // This shouldn't happen in normal Raft operation
            return Err(ConsensusError::LogError("Gap in log entries".to_string()));
        }
        
        // If inserting at the end, just append
        if index == self.entries.len() + 1 {
            return self.append_entry(entry).await.map(|_| ());
        }
        
        // If inserting in the middle, we need to truncate and append
        if index <= self.entries.len() {
            self.truncate_from(LogIndex::new(index as u64)).await?;
            return self.append_entry(entry).await.map(|_| ());
        }
        
        Err(ConsensusError::LogError("Invalid entry insertion".to_string()))
    }
    
    /// Get an entry by index
    pub fn get_entry(&self, index: LogIndex) -> Option<&LogEntry> {
        let idx = index.value() as usize;
        if idx > 0 && idx <= self.entries.len() {
            Some(&self.entries[idx - 1])
        } else {
            None
        }
    }
    
    /// Get a range of entries
    pub fn get_entries(&self, start: LogIndex, end: LogIndex) -> Vec<&LogEntry> {
        let start_idx = (start.value() as usize).saturating_sub(1);
        let end_idx = std::cmp::min(end.value() as usize, self.entries.len());
        
        if start_idx < end_idx {
            self.entries[start_idx..end_idx].iter().collect()
        } else {
            Vec::new()
        }
    }
    
    /// Truncate log from a given index onwards
    pub async fn truncate_from(&mut self, index: LogIndex) -> Result<()> {
        let truncate_idx = index.value() as usize;
        
        if truncate_idx <= self.entries.len() {
            // Calculate size reduction
            let removed_size: usize = self.entries[truncate_idx - 1..]
                .iter()
                .map(|entry| entry.size())
                .sum();
            
            // Truncate in-memory log
            self.entries.truncate(truncate_idx - 1);
            self.size_bytes -= removed_size;
            
            // Update commit and applied indices if necessary
            if self.commit_index >= index {
                self.commit_index = LogIndex::new((truncate_idx as u64).saturating_sub(1));
            }
            if self.last_applied >= index {
                self.last_applied = LogIndex::new((truncate_idx as u64).saturating_sub(1));
            }
            
            // Persist truncation to storage
            self.persist_truncation(index).await?;
            
            debug!("Truncated log from index {}", index.value());
        }
        
        Ok(())
    }
    
    /// Update the commit index
    pub async fn update_commit_index(&mut self, new_commit_index: LogIndex) -> Result<()> {
        if new_commit_index > self.commit_index && new_commit_index <= LogIndex::new(self.entries.len() as u64) {
            self.commit_index = new_commit_index;
            
            // Apply committed entries to state machine
            self.apply_committed_entries().await?;
            
            // Persist commit index
            self.persist_commit_index().await?;
            
            debug!("Updated commit index to {}", new_commit_index.value());
        }
        
        Ok(())
    }
    
    /// Check log consistency for a given index and term
    pub async fn check_consistency(&self, index: LogIndex, term: Term) -> bool {
        if index.value() == 0 {
            return true; // Empty log is always consistent
        }
        
        if let Some(entry) = self.get_entry(index) {
            entry.term == term
        } else {
            false // Entry doesn't exist
        }
    }
    
    /// Create a snapshot of the log up to a given index
    pub async fn create_snapshot(&mut self, last_included_index: LogIndex, last_included_term: Term) -> Result<Vec<u8>> {
        debug!("Creating snapshot up to index {}", last_included_index.value());
        
        // Collect all entries up to the snapshot point
        let snapshot_entries: Vec<_> = self.entries
            .iter()
            .filter(|entry| entry.index <= last_included_index)
            .cloned()
            .collect();
        
        // Serialize snapshot
        let snapshot_data = bincode::serialize(&snapshot_entries)
            .map_err(|e| ConsensusError::SerializationError(format!("Snapshot serialization failed: {}", e)))?;
        
        // Remove snapshotted entries from log
        self.entries.retain(|entry| entry.index > last_included_index);
        
        // Update size tracking
        self.size_bytes = self.entries.iter().map(|entry| entry.size()).sum();
        
        // Persist snapshot to storage
        self.persist_snapshot(&snapshot_data, last_included_index, last_included_term).await?;
        
        Ok(snapshot_data)
    }
    
    /// Install a snapshot (used by followers)
    pub async fn install_snapshot(
        &mut self,
        snapshot_data: Vec<u8>,
        last_included_index: LogIndex,
        last_included_term: Term,
    ) -> Result<()> {
        debug!("Installing snapshot up to index {}", last_included_index.value());
        
        // Deserialize snapshot
        let snapshot_entries: Vec<LogEntry> = bincode::deserialize(&snapshot_data)
            .map_err(|e| ConsensusError::SerializationError(format!("Snapshot deserialization failed: {}", e)))?;
        
        // Verify snapshot integrity
        for entry in &snapshot_entries {
            if !entry.verify_integrity() {
                return Err(ConsensusError::LogError("Snapshot contains corrupted entries".to_string()));
            }
        }
        
        // Replace log with snapshot + any entries after snapshot
        let remaining_entries: Vec<_> = self.entries
            .iter()
            .filter(|entry| entry.index > last_included_index)
            .cloned()
            .collect();
        
        self.entries = snapshot_entries;
        self.entries.extend(remaining_entries);
        
        // Update indices
        self.commit_index = std::cmp::max(self.commit_index, last_included_index);
        self.last_applied = std::cmp::max(self.last_applied, last_included_index);
        
        // Update size tracking
        self.size_bytes = self.entries.iter().map(|entry| entry.size()).sum();
        
        // Persist to storage
        self.persist_snapshot(&snapshot_data, last_included_index, last_included_term).await?;
        
        Ok(())
    }
    
    /// Apply committed entries to state machine
    async fn apply_committed_entries(&mut self) -> Result<()> {
        while self.last_applied < self.commit_index {
            let next_to_apply = LogIndex::new(self.last_applied.value() + 1);
            
            if let Some(entry) = self.get_entry(next_to_apply) {
                // Apply entry to state machine (simplified)
                debug!("Applied log entry {} to state machine", entry.index.value());
                self.last_applied = entry.index;
            } else {
                break;
            }
        }
        
        // Persist last applied index
        self.persist_last_applied().await?;
        
        Ok(())
    }
    
    /// Load log from storage on startup
    async fn load_from_storage(&mut self) -> Result<()> {
        // In a full implementation, would load persisted log entries
        debug!("Loading log from storage");
        Ok(())
    }
    
    /// Persist a log entry to storage
    async fn persist_entry(&self, entry: &LogEntry) -> Result<()> {
        let key = format!("log_entry_{}", entry.index.value());
        let value = bincode::serialize(entry)
            .map_err(|e| ConsensusError::SerializationError(format!("Entry serialization failed: {}", e)))?;
        
        self.storage.put(&key, value).await
            .map_err(|e| ConsensusError::StorageError(format!("Failed to persist entry: {}", e)))?;
        
        Ok(())
    }
    
    /// Persist log truncation to storage
    async fn persist_truncation(&self, _from_index: LogIndex) -> Result<()> {
        // In a full implementation, would remove truncated entries from storage
        Ok(())
    }
    
    /// Persist commit index to storage
    async fn persist_commit_index(&self) -> Result<()> {
        let key = "commit_index";
        let value = bincode::serialize(&self.commit_index)
            .map_err(|e| ConsensusError::SerializationError(format!("Commit index serialization failed: {}", e)))?;
        
        self.storage.put(key, value).await
            .map_err(|e| ConsensusError::StorageError(format!("Failed to persist commit index: {}", e)))?;
        
        Ok(())
    }
    
    /// Persist last applied index to storage
    async fn persist_last_applied(&self) -> Result<()> {
        let key = "last_applied";
        let value = bincode::serialize(&self.last_applied)
            .map_err(|e| ConsensusError::SerializationError(format!("Last applied serialization failed: {}", e)))?;
        
        self.storage.put(key, value).await
            .map_err(|e| ConsensusError::StorageError(format!("Failed to persist last applied: {}", e)))?;
        
        Ok(())
    }
    
    /// Persist snapshot to storage
    async fn persist_snapshot(
        &self,
        snapshot_data: &[u8],
        last_included_index: LogIndex,
        last_included_term: Term,
    ) -> Result<()> {
        // Persist snapshot metadata
        let metadata = SnapshotMetadata {
            last_included_index,
            last_included_term,
            size: snapshot_data.len(),
            checksum: sha2::Sha256::digest(snapshot_data).to_vec(),
        };
        
        let metadata_key = "snapshot_metadata";
        let metadata_value = bincode::serialize(&metadata)
            .map_err(|e| ConsensusError::SerializationError(format!("Snapshot metadata serialization failed: {}", e)))?;
        
        self.storage.put(metadata_key, metadata_value).await
            .map_err(|e| ConsensusError::StorageError(format!("Failed to persist snapshot metadata: {}", e)))?;
        
        // Persist snapshot data
        let data_key = "snapshot_data";
        self.storage.put(data_key, snapshot_data.to_vec()).await
            .map_err(|e| ConsensusError::StorageError(format!("Failed to persist snapshot data: {}", e)))?;
        
        Ok(())
    }
    
    /// Get log statistics
    pub fn statistics(&self) -> LogStatistics {
        LogStatistics {
            total_entries: self.entries.len(),
            size_bytes: self.size_bytes,
            commit_index: self.commit_index.value(),
            last_applied: self.last_applied.value(),
            earliest_index: self.entries.first().map(|e| e.index.value()).unwrap_or(0),
            latest_index: self.entries.last().map(|e| e.index.value()).unwrap_or(0),
        }
    }
}

/// Snapshot metadata for persistence
#[derive(Debug, Serialize, Deserialize)]
struct SnapshotMetadata {
    last_included_index: LogIndex,
    last_included_term: Term,
    size: usize,
    checksum: Vec<u8>,
}

/// Log statistics for monitoring
#[derive(Debug, Clone)]
pub struct LogStatistics {
    pub total_entries: usize,
    pub size_bytes: usize,
    pub commit_index: u64,
    pub last_applied: u64,
    pub earliest_index: u64,
    pub latest_index: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::storage::MockStorage;
    
    #[tokio::test]
    async fn test_log_creation() {
        let storage = Arc::new(MockStorage::new());
        let log = ReplicatedLog::new(storage).await.unwrap();
        
        assert_eq!(log.len(), 0);
        assert!(log.is_empty());
        assert_eq!(log.commit_index().value(), 0);
        assert_eq!(log.last_applied().value(), 0);
    }
    
    #[tokio::test]
    async fn test_log_entry_append() {
        let storage = Arc::new(MockStorage::new());
        let mut log = ReplicatedLog::new(storage).await.unwrap();
        
        let entry = LogEntry::new(LogIndex::new(1), Term::new(1), vec![1, 2, 3]);
        let index = log.append_entry(entry).await.unwrap();
        
        assert_eq!(index.value(), 1);
        assert_eq!(log.len(), 1);
        assert!(!log.is_empty());
    }
    
    #[tokio::test]
    async fn test_log_entry_integrity() {
        let entry = LogEntry::new(LogIndex::new(1), Term::new(1), vec![1, 2, 3]);
        assert!(entry.verify_integrity());
        
        // Corrupt the entry
        let mut corrupted = entry.clone();
        corrupted.data[0] = 99;
        assert!(!corrupted.verify_integrity());
    }
    
    #[tokio::test]
    async fn test_log_truncation() {
        let storage = Arc::new(MockStorage::new());
        let mut log = ReplicatedLog::new(storage).await.unwrap();
        
        // Add several entries
        for i in 1..=5 {
            let entry = LogEntry::new(LogIndex::new(i), Term::new(1), vec![i as u8]);
            log.append_entry(entry).await.unwrap();
        }
        
        assert_eq!(log.len(), 5);
        
        // Truncate from index 3
        log.truncate_from(LogIndex::new(3)).await.unwrap();
        
        assert_eq!(log.len(), 2);
        assert!(log.get_entry(LogIndex::new(3)).is_none());
        assert!(log.get_entry(LogIndex::new(2)).is_some());
    }
    
    #[tokio::test]
    async fn test_commit_index_update() {
        let storage = Arc::new(MockStorage::new());
        let mut log = ReplicatedLog::new(storage).await.unwrap();
        
        // Add entries
        for i in 1..=3 {
            let entry = LogEntry::new(LogIndex::new(i), Term::new(1), vec![i as u8]);
            log.append_entry(entry).await.unwrap();
        }
        
        // Update commit index
        log.update_commit_index(LogIndex::new(2)).await.unwrap();
        
        assert_eq!(log.commit_index().value(), 2);
        assert_eq!(log.last_applied().value(), 2);
    }
}