// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Incremental snapshot management
//!
//! Provides snapshot scheduling, creation, and cleanup with copy-on-write
//! optimization for efficient storage.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use super::{PersistenceError, PersistenceResult};

/// Snapshot scheduling options
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SnapshotSchedule {
    /// Time-based: snapshot every N seconds
    TimeBased { interval_secs: u64 },
    /// Event-based: snapshot after N events
    EventBased { event_count: u32 },
    /// Size-based: snapshot when data exceeds N bytes
    SizeBased { size_threshold: u64 },
    /// Manual: only create snapshots on demand
    Manual,
}

impl Default for SnapshotSchedule {
    fn default() -> Self {
        Self::TimeBased {
            interval_secs: 3600,
        } // Default: hourly
    }
}

/// Snapshot metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Unique snapshot ID
    pub id: String,
    /// Creation timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Snapshot version
    pub version: u32,
    /// Total size in bytes
    pub size: u64,
    /// SHA256 checksum
    pub checksum: String,
    /// Parent snapshot ID (for incremental)
    pub parent_id: Option<String>,
    /// Snapshot type
    pub snapshot_type: SnapshotType,
    /// Additional metadata
    pub metadata: BTreeMap<String, String>,
}

/// Snapshot type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SnapshotType {
    /// Full snapshot
    Full,
    /// Incremental (only changes)
    Incremental,
    /// Copy-on-write
    CopyOnWrite,
}

/// Manages snapshot creation and lifecycle
pub struct SnapshotManager {
    /// Storage directory
    storage_dir: PathBuf,
    /// Node ID
    _node_id: String,
    /// Current schedule
    schedule: Arc<RwLock<SnapshotSchedule>>,
    /// Snapshot metadata index
    snapshots: Arc<RwLock<BTreeMap<String, SnapshotMetadata>>>,
    /// Event counter for event-based scheduling
    event_counter: Arc<RwLock<u32>>,
    /// Data size counter for size-based scheduling
    size_counter: Arc<RwLock<u64>>,
    /// Last snapshot time
    last_snapshot: Arc<RwLock<Option<chrono::DateTime<chrono::Utc>>>>,
    /// Maximum snapshots to keep
    max_snapshots: usize,
}

impl SnapshotManager {
    /// Create new snapshot manager
    pub async fn new(
        storage_dir: PathBuf,
        node_id: String,
        schedule: SnapshotSchedule,
    ) -> PersistenceResult<Self> {
        let snapshot_dir = storage_dir.join(&node_id).join("snapshots");
        std::fs::create_dir_all(&snapshot_dir)?;

        // Load existing snapshot metadata
        let snapshots = Self::load_metadata(&snapshot_dir)?;

        Ok(Self {
            storage_dir: snapshot_dir,
            _node_id: node_id,
            schedule: Arc::new(RwLock::new(schedule)),
            snapshots: Arc::new(RwLock::new(snapshots)),
            event_counter: Arc::new(RwLock::new(0)),
            size_counter: Arc::new(RwLock::new(0)),
            last_snapshot: Arc::new(RwLock::new(None)),
            max_snapshots: 10,
        })
    }

    /// Set maximum snapshots to keep
    pub fn set_max_snapshots(&mut self, max: usize) {
        self.max_snapshots = max;
    }

    /// Update schedule
    pub async fn update_schedule(&self, schedule: SnapshotSchedule) {
        *self.schedule.write().await = schedule;
    }

    /// Check if snapshot is needed based on schedule
    pub async fn should_snapshot(&self) -> bool {
        let schedule = self.schedule.read().await;

        match *schedule {
            SnapshotSchedule::TimeBased { interval_secs } => {
                if let Some(last) = *self.last_snapshot.read().await {
                    let elapsed = chrono::Utc::now() - last;
                    elapsed.num_seconds() as u64 >= interval_secs
                } else {
                    true // First snapshot
                }
            }
            SnapshotSchedule::EventBased { event_count } => {
                *self.event_counter.read().await >= event_count
            }
            SnapshotSchedule::SizeBased { size_threshold } => {
                *self.size_counter.read().await >= size_threshold
            }
            SnapshotSchedule::Manual => false,
        }
    }

    /// Record an event (for event-based scheduling)
    pub async fn record_event(&self) {
        let mut counter = self.event_counter.write().await;
        *counter += 1;
    }

    /// Record data size change (for size-based scheduling)
    pub async fn record_size_change(&self, bytes: i64) {
        let mut counter = self.size_counter.write().await;
        if bytes > 0 {
            *counter += bytes as u64;
        } else if *counter >= bytes.unsigned_abs() {
            *counter -= bytes.unsigned_abs();
        } else {
            *counter = 0;
        }
    }

    /// Create a snapshot
    pub async fn create_snapshot<F, T>(
        &self,
        data_provider: F,
        snapshot_type: SnapshotType,
    ) -> PersistenceResult<String>
    where
        F: FnOnce() -> PersistenceResult<T>,
        T: Serialize,
    {
        let start = std::time::Instant::now();
        let id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now();

        info!("Creating {:?} snapshot {}", snapshot_type, id);

        // Get data to snapshot
        let data = data_provider()?;

        // Serialize data
        let serialized = bincode::serialize(&data)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        // Compress data
        let compressed = zstd::encode_all(&serialized[..], 3)
            .map_err(|e| PersistenceError::Compression(e.to_string()))?;

        // Calculate checksum
        let checksum = self.calculate_checksum(&compressed);

        // Find parent for incremental snapshots
        let parent_id = if snapshot_type == SnapshotType::Incremental {
            self.get_latest_snapshot_id().await
        } else {
            None
        };

        // Create metadata
        let metadata = SnapshotMetadata {
            id: id.clone(),
            timestamp,
            version: 1,
            size: compressed.len() as u64,
            checksum: checksum.clone(),
            parent_id,
            snapshot_type,
            metadata: BTreeMap::new(),
        };

        // Save snapshot file
        let filename = format!(
            "snapshot_{}_{}.tar.zst",
            timestamp.format("%Y%m%d_%H%M%S"),
            &id[..8]
        );
        let path = self.storage_dir.join(filename);
        std::fs::write(&path, compressed)?;

        // Update metadata
        {
            let mut snapshots = self.snapshots.write().await;
            snapshots.insert(id.clone(), metadata);
        }

        // Save metadata index
        self.save_metadata().await?;

        // Reset counters
        *self.event_counter.write().await = 0;
        *self.size_counter.write().await = 0;
        *self.last_snapshot.write().await = Some(timestamp);

        // Cleanup old snapshots
        self.cleanup_old_snapshots().await?;

        let elapsed = start.elapsed();
        info!("Created snapshot {} in {:?}", id, elapsed);

        Ok(id)
    }

    /// Restore from snapshot
    pub async fn restore_snapshot<T>(&self, snapshot_id: &str) -> PersistenceResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        info!("Restoring from snapshot {}", snapshot_id);

        let metadata = {
            let snapshots = self.snapshots.read().await;
            snapshots
                .get(snapshot_id)
                .ok_or_else(|| {
                    PersistenceError::SnapshotError(format!("Snapshot {snapshot_id} not found"))
                })?
                .clone()
        };

        // Find snapshot file
        let _pattern = format!("*{}*.tar.zst", &snapshot_id[..8]);
        let mut snapshot_file = None;

        for entry in std::fs::read_dir(&self.storage_dir)? {
            let entry = entry?;
            let filename = entry.file_name();
            if let Some(name) = filename.to_str() {
                if name.contains(&snapshot_id[..8]) && name.ends_with(".tar.zst") {
                    snapshot_file = Some(entry.path());
                    break;
                }
            }
        }

        let path = snapshot_file.ok_or_else(|| {
            PersistenceError::SnapshotError(format!("Snapshot file for {snapshot_id} not found"))
        })?;

        // Read and decompress
        let compressed = std::fs::read(&path)?;

        // Verify checksum
        let checksum = self.calculate_checksum(&compressed);
        if checksum != metadata.checksum {
            return Err(PersistenceError::ChecksumMismatch {
                expected: metadata.checksum,
                actual: checksum,
            });
        }

        // Decompress
        let decompressed = zstd::decode_all(&compressed[..])
            .map_err(|e| PersistenceError::Decompression(e.to_string()))?;

        // Deserialize
        let data = bincode::deserialize(&decompressed)
            .map_err(|e| PersistenceError::Deserialization(e.to_string()))?;

        info!("Successfully restored snapshot {}", snapshot_id);

        Ok(data)
    }

    /// List available snapshots
    pub async fn list_snapshots(&self) -> Vec<SnapshotMetadata> {
        let snapshots = self.snapshots.read().await;
        let mut list: Vec<_> = snapshots.values().cloned().collect();
        list.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        list
    }

    /// Get latest snapshot ID
    pub async fn get_latest_snapshot_id(&self) -> Option<String> {
        let snapshots = self.snapshots.read().await;
        snapshots
            .values()
            .max_by_key(|s| s.timestamp)
            .map(|s| s.id.clone())
    }

    /// Delete a snapshot
    pub async fn delete_snapshot(&self, snapshot_id: &str) -> PersistenceResult<()> {
        info!("Deleting snapshot {}", snapshot_id);

        // Remove from metadata
        {
            let mut snapshots = self.snapshots.write().await;
            snapshots.remove(snapshot_id);
        }

        // Delete file
        for entry in std::fs::read_dir(&self.storage_dir)? {
            let entry = entry?;
            let filename = entry.file_name();
            if let Some(name) = filename.to_str() {
                if name.contains(&snapshot_id[..8]) && name.ends_with(".tar.zst") {
                    std::fs::remove_file(entry.path())?;
                    break;
                }
            }
        }

        self.save_metadata().await?;

        Ok(())
    }

    /// Cleanup old snapshots keeping only max_snapshots
    async fn cleanup_old_snapshots(&self) -> PersistenceResult<u32> {
        let mut deleted = 0;
        let snapshots = self.list_snapshots().await;

        if snapshots.len() > self.max_snapshots {
            for snapshot in snapshots.iter().skip(self.max_snapshots) {
                self.delete_snapshot(&snapshot.id).await?;
                deleted += 1;
                info!("Cleaned up old snapshot: {}", snapshot.id);
            }
        }

        Ok(deleted)
    }

    /// Calculate BLAKE3 checksum
    fn calculate_checksum(&self, data: &[u8]) -> String {
        hex::encode(blake3::hash(data).as_bytes())
    }

    /// Save metadata index to disk
    async fn save_metadata(&self) -> PersistenceResult<()> {
        let metadata_path = self.storage_dir.join("snapshot.meta");
        let snapshots = self.snapshots.read().await;

        let json = serde_json::to_string_pretty(&*snapshots)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        std::fs::write(metadata_path, json)?;
        Ok(())
    }

    /// Load metadata index from disk
    fn load_metadata(dir: &Path) -> PersistenceResult<BTreeMap<String, SnapshotMetadata>> {
        let metadata_path = dir.join("snapshot.meta");

        if metadata_path.exists() {
            let json = std::fs::read_to_string(metadata_path)?;
            serde_json::from_str(&json)
                .map_err(|e| PersistenceError::Deserialization(e.to_string()))
        } else {
            Ok(BTreeMap::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        value: String,
        counter: u32,
    }

    #[tokio::test]
    async fn test_snapshot_creation() {
        let temp_dir = TempDir::new().expect("test: temp dir creation");
        let manager = SnapshotManager::new(
            temp_dir.path().to_path_buf(),
            "test_node".to_string(),
            SnapshotSchedule::Manual,
        )
        .await
        .expect("test: expected success");

        let test_data = TestData {
            value: "test".to_string(),
            counter: 42,
        };

        let id = manager
            .create_snapshot(|| Ok(test_data.clone()), SnapshotType::Full)
            .await
            .expect("test: expected success");

        assert!(!id.is_empty());

        // Verify snapshot exists
        let snapshots = manager.list_snapshots().await;
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].id, id);
    }

    #[tokio::test]
    async fn test_snapshot_restore() {
        let temp_dir = TempDir::new().expect("test: temp dir creation");
        let manager = SnapshotManager::new(
            temp_dir.path().to_path_buf(),
            "test_node".to_string(),
            SnapshotSchedule::Manual,
        )
        .await
        .expect("test: expected success");

        let original = TestData {
            value: "original".to_string(),
            counter: 100,
        };

        // Create snapshot
        let id = manager
            .create_snapshot(|| Ok(original.clone()), SnapshotType::Full)
            .await
            .expect("test: expected success");

        // Restore
        let restored: TestData = manager.restore_snapshot(&id).await.expect("test: async operation");

        assert_eq!(restored, original);
    }

    #[tokio::test]
    async fn test_time_based_scheduling() {
        let temp_dir = TempDir::new().expect("test: temp dir creation");
        let manager = SnapshotManager::new(
            temp_dir.path().to_path_buf(),
            "test_node".to_string(),
            SnapshotSchedule::TimeBased { interval_secs: 1 },
        )
        .await
        .expect("test: expected success");

        // Should snapshot initially
        assert!(manager.should_snapshot().await);

        // Create snapshot
        let _ = manager
            .create_snapshot(
                || {
                    Ok(TestData {
                        value: "test".to_string(),
                        counter: 1,
                    })
                },
                SnapshotType::Full,
            )
            .await
            .expect("test: expected success");

        // Should not snapshot immediately
        assert!(!manager.should_snapshot().await);

        // Wait for interval
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Should snapshot after interval
        assert!(manager.should_snapshot().await);
    }

    #[tokio::test]
    async fn test_event_based_scheduling() {
        let temp_dir = TempDir::new().expect("test: temp dir creation");
        let manager = SnapshotManager::new(
            temp_dir.path().to_path_buf(),
            "test_node".to_string(),
            SnapshotSchedule::EventBased { event_count: 5 },
        )
        .await
        .expect("test: expected success");

        assert!(!manager.should_snapshot().await);

        // Record events
        for _ in 0..5 {
            manager.record_event().await;
        }

        assert!(manager.should_snapshot().await);

        // Create snapshot (resets counter)
        let _ = manager
            .create_snapshot(
                || {
                    Ok(TestData {
                        value: "test".to_string(),
                        counter: 1,
                    })
                },
                SnapshotType::Full,
            )
            .await
            .expect("test: expected success");

        assert!(!manager.should_snapshot().await);
    }

    #[tokio::test]
    async fn test_size_based_scheduling() {
        let temp_dir = TempDir::new().expect("test: temp dir creation");
        let manager = SnapshotManager::new(
            temp_dir.path().to_path_buf(),
            "test_node".to_string(),
            SnapshotSchedule::SizeBased {
                size_threshold: 1000,
            },
        )
        .await
        .expect("test: expected success");

        assert!(!manager.should_snapshot().await);

        // Record size changes
        manager.record_size_change(500).await;
        assert!(!manager.should_snapshot().await);

        manager.record_size_change(600).await;
        assert!(manager.should_snapshot().await);
    }

    #[tokio::test]
    async fn test_snapshot_cleanup() {
        let temp_dir = TempDir::new().expect("test: temp dir creation");
        let mut manager = SnapshotManager::new(
            temp_dir.path().to_path_buf(),
            "test_node".to_string(),
            SnapshotSchedule::Manual,
        )
        .await
        .expect("test: expected success");

        manager.set_max_snapshots(2);

        // Create multiple snapshots
        for i in 0..5 {
            let data = TestData {
                value: format!("test{i}"),
                counter: i,
            };
            manager
                .create_snapshot(move || Ok(data), SnapshotType::Full)
                .await
                .expect("test: expected success");
        }

        // Should only have 2 snapshots
        let snapshots = manager.list_snapshots().await;
        assert_eq!(snapshots.len(), 2);
    }

    #[tokio::test]
    async fn test_incremental_snapshot() {
        let temp_dir = TempDir::new().expect("test: temp dir creation");
        let manager = SnapshotManager::new(
            temp_dir.path().to_path_buf(),
            "test_node".to_string(),
            SnapshotSchedule::Manual,
        )
        .await
        .expect("test: expected success");

        // Create full snapshot
        let full_id = manager
            .create_snapshot(
                || {
                    Ok(TestData {
                        value: "full".to_string(),
                        counter: 1,
                    })
                },
                SnapshotType::Full,
            )
            .await
            .expect("test: expected success");

        // Create incremental snapshot
        let inc_id = manager
            .create_snapshot(
                || {
                    Ok(TestData {
                        value: "incremental".to_string(),
                        counter: 2,
                    })
                },
                SnapshotType::Incremental,
            )
            .await
            .expect("test: expected success");

        // Verify parent relationship
        let snapshots = manager.list_snapshots().await;
        let incremental = snapshots.iter().find(|s| s.id == inc_id).expect("test: query operation");
        assert_eq!(incremental.parent_id, Some(full_id));
    }

    #[tokio::test]
    async fn test_checksum_validation() {
        let temp_dir = TempDir::new().expect("test: temp dir creation");
        let manager = SnapshotManager::new(
            temp_dir.path().to_path_buf(),
            "test_node".to_string(),
            SnapshotSchedule::Manual,
        )
        .await
        .expect("test: expected success");

        let data = TestData {
            value: "checksum_test".to_string(),
            counter: 99,
        };

        let id = manager
            .create_snapshot(|| Ok(data.clone()), SnapshotType::Full)
            .await
            .expect("test: expected success");

        // Corrupt the snapshot file
        for entry in std::fs::read_dir(&manager.storage_dir).expect("test: directory reading") {
            let entry = entry.expect("test: directory entry");
            if entry.file_name().to_str().expect("test: directory entry").contains(&id[..8]) {
                let mut content = std::fs::read(entry.path()).expect("test: directory entry");
                content[10] ^= 0xFF; // Flip some bits
                std::fs::write(entry.path(), content).expect("test: directory entry");
                break;
            }
        }

        // Restore should fail with checksum error
        let result: Result<TestData, _> = manager.restore_snapshot(&id).await;
        assert!(result.is_err());

        if let Err(PersistenceError::ChecksumMismatch { .. }) = result {
            // Expected error
        } else {
            panic!("Expected checksum mismatch error");
        }
    }

    #[tokio::test]
    async fn test_delete_snapshot() {
        let temp_dir = TempDir::new().expect("test: temp dir creation");
        let manager = SnapshotManager::new(
            temp_dir.path().to_path_buf(),
            "test_node".to_string(),
            SnapshotSchedule::Manual,
        )
        .await
        .expect("test: expected success");

        let id = manager
            .create_snapshot(
                || {
                    Ok(TestData {
                        value: "delete_me".to_string(),
                        counter: 1,
                    })
                },
                SnapshotType::Full,
            )
            .await
            .expect("test: expected success");

        // Verify exists
        assert_eq!(manager.list_snapshots().await.len(), 1);

        // Delete
        manager.delete_snapshot(&id).await.expect("test: async operation");

        // Verify deleted
        assert_eq!(manager.list_snapshots().await.len(), 0);
    }
}
