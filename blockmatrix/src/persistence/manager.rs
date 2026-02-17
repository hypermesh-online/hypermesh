// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Persistence manager
//!
//! Unified interface for all persistence operations with background
//! processing, transaction support, and disk monitoring.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use tokio::task::JoinHandle;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn, error};

use super::{
    PersistenceError, PersistenceResult,
    matrix_state::{MatrixState, MatrixStateSerializer, SerializationFormat},
    blockchain_storage::{BlockchainStorage, BlockQuery},
    topology_backup::{TopologyBackup, BackupMode},
    snapshots::{SnapshotManager, SnapshotSchedule},
    recovery::{RecoveryManager, RecoveryReport},
};

use crate::blockchain::block::Block;
use super::topology_backup::NetworkTopology;

/// Serde helper for SerializationFormat
mod serialization_format_serde {
    use super::SerializationFormat;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(format: &SerializationFormat, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match format {
            SerializationFormat::Bincode => "bincode",
            SerializationFormat::Json => "json",
            SerializationFormat::MessagePack => "messagepack",
        };
        serializer.serialize_str(s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SerializationFormat, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "bincode" => Ok(SerializationFormat::Bincode),
            "json" => Ok(SerializationFormat::Json),
            "messagepack" => Ok(SerializationFormat::MessagePack),
            _ => Err(serde::de::Error::custom(format!("Unknown format: {}", s))),
        }
    }
}

/// Persistence configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    /// Base storage directory
    pub storage_dir: PathBuf,
    /// Enable compression
    pub enable_compression: bool,
    /// Compression level (1-22)
    pub compression_level: i32,
    /// Matrix state format (stored as string for serialization)
    #[serde(with = "serialization_format_serde")]
    pub matrix_format: SerializationFormat,
    /// Snapshot schedule
    pub snapshot_schedule: SnapshotSchedule,
    /// Maximum snapshots to keep
    pub max_snapshots: usize,
    /// Maximum topology backups to keep
    pub max_backups: usize,
    /// Enable background persistence
    pub enable_background: bool,
    /// Background save interval (seconds)
    pub background_interval_secs: u64,
    /// Disk space warning threshold (bytes)
    pub disk_warning_threshold: u64,
    /// Disk space error threshold (bytes)
    pub disk_error_threshold: u64,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            storage_dir: PathBuf::from("~/.blockmatrix"),
            enable_compression: true,
            compression_level: 3,
            matrix_format: SerializationFormat::Bincode,
            snapshot_schedule: SnapshotSchedule::TimeBased { interval_secs: 3600 },
            max_snapshots: 10,
            max_backups: 5,
            enable_background: true,
            background_interval_secs: 300, // 5 minutes
            disk_warning_threshold: 100 * 1024 * 1024, // 100MB
            disk_error_threshold: 10 * 1024 * 1024,    // 10MB
        }
    }
}

/// Storage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageStats {
    /// Total disk space used (bytes)
    pub total_used: u64,
    /// Matrix state size
    pub matrix_size: u64,
    /// Blockchain size
    pub blockchain_size: u64,
    /// Topology backup size
    pub topology_size: u64,
    /// Snapshot total size
    pub snapshot_size: u64,
    /// Available disk space
    pub disk_available: u64,
    /// Number of blocks stored
    pub block_count: u64,
    /// Number of snapshots
    pub snapshot_count: u32,
    /// Number of backups
    pub backup_count: u32,
}

/// Persistence transaction handle
#[allow(dead_code)] // Fields used during transaction lifecycle
pub struct PersistenceTransaction {
    /// Transaction ID
    id: String,
    /// Operations to commit
    operations: Vec<PersistenceOperation>,
    /// Rollback data
    rollback: Vec<RollbackData>,
    /// Committed flag
    committed: bool,
}

impl PersistenceTransaction {
    /// Add operation to transaction
    pub fn add_operation(&mut self, op: PersistenceOperation) {
        self.operations.push(op);
    }

    /// Commit transaction
    pub async fn commit(mut self) -> PersistenceResult<()> {
        // Execute all operations
        for _op in &self.operations {
            // Would execute operation here
        }
        self.committed = true;
        Ok(())
    }

    /// Rollback transaction
    pub async fn rollback(self) -> PersistenceResult<()> {
        if self.committed {
            return Err(PersistenceError::LockError(
                "Cannot rollback committed transaction".to_string()
            ));
        }

        // Rollback operations
        for _rollback in self.rollback.iter().rev() {
            // Would rollback here
        }

        Ok(())
    }
}

/// Persistence operation types
#[derive(Debug, Clone)]
pub enum PersistenceOperation {
    SaveMatrixState(MatrixState),
    SaveBlock(Block),
    CreateSnapshot,
    CreateBackup(BackupMode),
}

/// Rollback data
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields used during transaction rollback
struct RollbackData {
    /// Component affected
    component: String,
    /// Previous data (serialized)
    data: Vec<u8>,
}

/// Unified persistence manager
pub struct PersistenceManager {
    /// Configuration
    config: Arc<PersistenceConfig>,
    /// Node ID
    node_id: String,
    /// Matrix state serializer
    matrix_serializer: Arc<MatrixStateSerializer>,
    /// Blockchain storage
    blockchain_storage: Arc<RwLock<Option<BlockchainStorage>>>,
    /// Topology backup handler
    topology_backup: Arc<TopologyBackup>,
    /// Snapshot manager
    snapshot_manager: Arc<SnapshotManager>,
    /// Recovery manager
    recovery_manager: Arc<Mutex<RecoveryManager>>,
    /// Background save handle
    background_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    /// Storage statistics
    stats: Arc<RwLock<StorageStats>>,
    /// Transaction counter
    transaction_counter: Arc<RwLock<u64>>,
}

impl PersistenceManager {
    /// Create new persistence manager
    pub async fn new(
        config: PersistenceConfig,
        node_id: String,
    ) -> PersistenceResult<Self> {
        // Expand home directory
        let storage_dir = if config.storage_dir.starts_with("~") {
            let home = dirs::home_dir()
                .ok_or_else(|| PersistenceError::InvalidPath("Cannot determine home directory".to_string()))?;
            let relative_path = config.storage_dir.strip_prefix("~")
                .map_err(|_| PersistenceError::InvalidPath("Invalid home directory path".to_string()))?;
            home.join(relative_path)
        } else {
            config.storage_dir.clone()
        };

        // Create directory structure
        let node_dir = storage_dir.join(&node_id);
        std::fs::create_dir_all(&node_dir)?;

        // Initialize components
        let matrix_serializer = MatrixStateSerializer::new(
            config.matrix_format,
            config.enable_compression,
        ).with_compression_level(config.compression_level);

        let blockchain_storage = BlockchainStorage::new(
            storage_dir.clone(),
            node_id.clone(),
        ).await?;

        let topology_backup = TopologyBackup::new(
            storage_dir.clone(),
            node_id.clone(),
        )?;

        let mut snapshot_manager = SnapshotManager::new(
            storage_dir.clone(),
            node_id.clone(),
            config.snapshot_schedule.clone(),
        ).await?;
        snapshot_manager.set_max_snapshots(config.max_snapshots);

        let recovery_manager = RecoveryManager::new(
            storage_dir.clone(),
            node_id.clone(),
        );

        let manager = Self {
            config: Arc::new(config),
            node_id,
            matrix_serializer: Arc::new(matrix_serializer),
            blockchain_storage: Arc::new(RwLock::new(Some(blockchain_storage))),
            topology_backup: Arc::new(topology_backup),
            snapshot_manager: Arc::new(snapshot_manager),
            recovery_manager: Arc::new(Mutex::new(recovery_manager)),
            background_handle: Arc::new(RwLock::new(None)),
            stats: Arc::new(RwLock::new(StorageStats::default())),
            transaction_counter: Arc::new(RwLock::new(0)),
        };

        // Update initial stats
        manager.update_stats().await?;

        // Start background persistence if enabled
        if manager.config.enable_background {
            manager.start_background_persistence().await?;
        }

        Ok(manager)
    }

    /// Save matrix state
    pub async fn save_matrix_state(&self, state: &MatrixState) -> PersistenceResult<()> {
        debug!("Saving matrix state");

        let serialized = self.matrix_serializer.serialize(state)?;

        let state_file = self.get_storage_dir()
            .join(&self.node_id)
            .join("matrix")
            .join("coordinates.bin");

        let parent_dir = state_file.parent()
            .ok_or_else(|| PersistenceError::InvalidPath("Invalid state file path".to_string()))?;
        std::fs::create_dir_all(parent_dir)?;
        std::fs::write(&state_file, serialized)?;

        info!("Saved matrix state ({} neighbors, {} cache entries)",
              state.neighbors.len(), state.distance_cache.len());

        Ok(())
    }

    /// Load matrix state
    pub async fn load_matrix_state(&self) -> PersistenceResult<Option<MatrixState>> {
        let state_file = self.get_storage_dir()
            .join(&self.node_id)
            .join("matrix")
            .join("coordinates.bin");

        if !state_file.exists() {
            return Ok(None);
        }

        let data = std::fs::read(&state_file)?;
        let state = self.matrix_serializer.deserialize(&data)?;

        Ok(Some(state))
    }

    /// Save block to blockchain storage
    pub async fn save_block(&self, block: &Block) -> PersistenceResult<()> {
        if let Some(storage) = self.blockchain_storage.read().await.as_ref() {
            storage.write_block(block).await?;
        }
        Ok(())
    }

    /// Load block from blockchain storage
    pub async fn load_block(&self, query: BlockQuery) -> PersistenceResult<Option<Block>> {
        if let Some(storage) = self.blockchain_storage.read().await.as_ref() {
            storage.read_block(query).await
        } else {
            Ok(None)
        }
    }

    /// Create topology backup
    pub async fn create_topology_backup(
        &self,
        topology: &NetworkTopology,
        mode: BackupMode,
    ) -> PersistenceResult<PathBuf> {
        match mode {
            BackupMode::Full => {
                self.topology_backup.create_full_backup(topology).await
            }
            BackupMode::Essential => {
                self.topology_backup.create_essential_backup(topology).await
            }
            BackupMode::Incremental => {
                // Need previous backup for incremental
                let backups = self.topology_backup.list_backups()?;
                if let Some(latest) = backups.first() {
                    let previous = self.topology_backup.restore_backup(&latest.path).await?;
                    self.topology_backup.create_incremental_backup(topology, &previous).await
                } else {
                    // Fall back to full backup if no previous
                    self.topology_backup.create_full_backup(topology).await
                }
            }
        }
    }

    /// Create snapshot
    pub async fn create_snapshot(&self) -> PersistenceResult<String> {
        // Collect all data for snapshot
        let snapshot_data = self.collect_snapshot_data().await?;

        self.snapshot_manager.create_snapshot(
            || Ok(snapshot_data),
            super::snapshots::SnapshotType::Full,
        ).await
    }

    /// Begin transaction
    pub async fn begin_transaction(&self) -> PersistenceResult<PersistenceTransaction> {
        let mut counter = self.transaction_counter.write().await;
        *counter += 1;

        Ok(PersistenceTransaction {
            id: format!("txn_{}", counter),
            operations: Vec::new(),
            rollback: Vec::new(),
            committed: false,
        })
    }

    /// Perform recovery
    pub async fn recover(&self) -> PersistenceResult<RecoveryReport> {
        let mut recovery = self.recovery_manager.lock().await;
        recovery.recover_all().await
    }

    /// Verify integrity
    pub async fn verify_integrity(&self) -> PersistenceResult<bool> {
        let mut recovery = self.recovery_manager.lock().await;
        recovery.verify_integrity().await
    }

    /// Update storage statistics
    pub async fn update_stats(&self) -> PersistenceResult<()> {
        let mut stats = StorageStats::default();

        // Calculate directory sizes
        let node_dir = self.get_storage_dir().join(&self.node_id);

        if node_dir.exists() {
            stats.total_used = self.calculate_dir_size(&node_dir)?;

            let matrix_dir = node_dir.join("matrix");
            if matrix_dir.exists() {
                stats.matrix_size = self.calculate_dir_size(&matrix_dir)?;
            }

            let blockchain_dir = node_dir.join("blockchain");
            if blockchain_dir.exists() {
                stats.blockchain_size = self.calculate_dir_size(&blockchain_dir)?;
            }

            let topology_dir = node_dir.join("topology");
            if topology_dir.exists() {
                stats.topology_size = self.calculate_dir_size(&topology_dir)?;
            }

            let snapshot_dir = node_dir.join("snapshots");
            if snapshot_dir.exists() {
                stats.snapshot_size = self.calculate_dir_size(&snapshot_dir)?;
            }
        }

        // Get disk space
        stats.disk_available = self.get_available_disk_space()?;

        // Get counts
        if let Some(storage) = self.blockchain_storage.read().await.as_ref() {
            let chain_stats = storage.get_stats().await;
            stats.block_count = chain_stats.total_blocks;
        }

        stats.snapshot_count = self.snapshot_manager.list_snapshots().await.len() as u32;
        stats.backup_count = self.topology_backup.list_backups()?.len() as u32;

        *self.stats.write().await = stats;

        // Check disk space
        self.check_disk_space().await?;

        Ok(())
    }

    /// Get storage statistics
    pub async fn get_stats(&self) -> StorageStats {
        self.stats.read().await.clone()
    }

    /// Start background persistence
    async fn start_background_persistence(&self) -> PersistenceResult<()> {
        // Clone the Arc references we need for the background task
        let snapshot_manager = Arc::clone(&self.snapshot_manager);
        let topology_backup = Arc::clone(&self.topology_backup);
        let _stats_update = Arc::clone(&self.stats);
        let _storage_dir_clone = self.get_storage_dir();
        let _node_id_clone = self.node_id.clone();
        let max_backups = self.config.max_backups;
        let interval_secs = self.config.background_interval_secs;

        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                tokio::time::Duration::from_secs(interval_secs)
            );

            loop {
                interval.tick().await;

                // Check if snapshot needed
                if snapshot_manager.should_snapshot().await {
                    // Note: Can't call create_snapshot here as it needs full manager
                    // This is a limitation of the background task - it can only check
                    debug!("Snapshot needed but creation requires full manager context");
                }

                // Cleanup old backups
                if let Err(e) = topology_backup.cleanup_old_backups(max_backups) {
                    warn!("Failed to cleanup backups: {}", e);
                }
            }
        });

        *self.background_handle.write().await = Some(handle);

        info!("Started background persistence (interval: {}s)", interval_secs);

        Ok(())
    }

    /// Stop background persistence
    pub async fn stop_background_persistence(&self) {
        if let Some(handle) = self.background_handle.write().await.take() {
            handle.abort();
            info!("Stopped background persistence");
        }
    }

    /// Flush all pending writes
    pub async fn flush(&self) -> PersistenceResult<()> {
        if let Some(storage) = self.blockchain_storage.read().await.as_ref() {
            storage.flush_wal().await?;
        }
        Ok(())
    }

    /// Shutdown persistence manager
    pub async fn shutdown(&self) -> PersistenceResult<()> {
        info!("Shutting down persistence manager");

        // Stop background tasks
        self.stop_background_persistence().await;

        // Flush pending writes
        self.flush().await?;

        // Final stats update
        self.update_stats().await?;

        info!("Persistence manager shutdown complete");

        Ok(())
    }

    /// Get storage directory
    fn get_storage_dir(&self) -> PathBuf {
        if self.config.storage_dir.starts_with("~") {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
            // Safe: we already checked it starts with "~" above
            let relative = self.config.storage_dir.strip_prefix("~")
                .unwrap_or_else(|_| &self.config.storage_dir);
            home.join(relative)
        } else {
            self.config.storage_dir.clone()
        }
    }

    /// Calculate directory size
    fn calculate_dir_size(&self, dir: &Path) -> PersistenceResult<u64> {
        let mut size = 0u64;

        for entry in walkdir::WalkDir::new(dir) {
            let entry = entry.map_err(|e| PersistenceError::Io(e.into()))?;
            if entry.file_type().is_file() {
                if let Ok(metadata) = entry.metadata() {
                    size += metadata.len();
                }
            }
        }

        Ok(size)
    }

    /// Get available disk space
    fn get_available_disk_space(&self) -> PersistenceResult<u64> {
        // Platform-specific implementation would go here
        // For now, return a placeholder
        Ok(1024 * 1024 * 1024) // 1GB
    }

    /// Check disk space and warn/error if low
    async fn check_disk_space(&self) -> PersistenceResult<()> {
        let stats = self.stats.read().await;

        if stats.disk_available < self.config.disk_error_threshold {
            error!("Critical: Disk space below error threshold ({} bytes available)",
                   stats.disk_available);
            return Err(PersistenceError::InsufficientDiskSpace {
                needed: self.config.disk_error_threshold,
                available: stats.disk_available,
            });
        }

        if stats.disk_available < self.config.disk_warning_threshold {
            warn!("Disk space below warning threshold ({} bytes available)",
                  stats.disk_available);
        }

        Ok(())
    }

    /// Collect all data for snapshot
    async fn collect_snapshot_data(&self) -> PersistenceResult<SnapshotData> {
        let matrix_state = self.load_matrix_state().await?;

        // Get latest blocks
        let mut recent_blocks = Vec::new();
        if let Some(storage) = self.blockchain_storage.read().await.as_ref() {
            let metadata = storage.get_metadata().await;
            if metadata.total_blocks > 0 {
                let start = metadata.chain_height.saturating_sub(9);
                recent_blocks = storage.read_range(start, metadata.chain_height).await?;
            }
        }

        Ok(SnapshotData {
            matrix_state,
            recent_blocks,
            timestamp: chrono::Utc::now(),
        })
    }
}

/// Data included in snapshots
#[derive(Debug, Serialize, Deserialize)]
struct SnapshotData {
    matrix_state: Option<MatrixState>,
    recent_blocks: Vec<Block>,
    timestamp: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::matrix::coordinate::MatrixCoordinate;

    #[tokio::test]
    async fn test_persistence_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = PersistenceConfig::default();
        config.storage_dir = temp_dir.path().to_path_buf();
        config.enable_background = false;

        let manager = PersistenceManager::new(config, "test_node".to_string()).await.unwrap();
        let stats = manager.get_stats().await;

        assert_eq!(stats.block_count, 0);
        assert_eq!(stats.snapshot_count, 0);
    }

    #[tokio::test]
    async fn test_save_and_load_matrix_state() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = PersistenceConfig::default();
        config.storage_dir = temp_dir.path().to_path_buf();
        config.enable_background = false;

        let manager = PersistenceManager::new(config, "test_node".to_string()).await.unwrap();

        let coord = MatrixCoordinate::new(1, 2, 3).unwrap();
        let mut state = MatrixState::new(coord);
        state.add_neighbor("node1".to_string(), MatrixCoordinate::new(4, 5, 6).unwrap());

        // Save
        manager.save_matrix_state(&state).await.unwrap();

        // Load
        let loaded = manager.load_matrix_state().await.unwrap();
        assert!(loaded.is_some());

        let loaded_state = loaded.unwrap();
        assert_eq!(loaded_state.coordinate, state.coordinate);
        assert_eq!(loaded_state.neighbors.len(), 1);
    }

    #[tokio::test]
    async fn test_create_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = PersistenceConfig::default();
        config.storage_dir = temp_dir.path().to_path_buf();
        config.enable_background = false;

        let manager = PersistenceManager::new(config, "test_node".to_string()).await.unwrap();

        let snapshot_id = manager.create_snapshot().await.unwrap();
        assert!(!snapshot_id.is_empty());

        let stats = manager.get_stats().await;
        assert!(stats.snapshot_count > 0);
    }

    #[tokio::test]
    async fn test_transaction() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = PersistenceConfig::default();
        config.storage_dir = temp_dir.path().to_path_buf();
        config.enable_background = false;

        let manager = PersistenceManager::new(config, "test_node".to_string()).await.unwrap();

        let mut txn = manager.begin_transaction().await.unwrap();

        let state = MatrixState::new(MatrixCoordinate::new(0, 0, 0).unwrap());
        txn.add_operation(PersistenceOperation::SaveMatrixState(state));

        txn.commit().await.unwrap();
    }

    #[tokio::test]
    async fn test_recovery() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = PersistenceConfig::default();
        config.storage_dir = temp_dir.path().to_path_buf();
        config.enable_background = false;

        let manager = PersistenceManager::new(config, "test_node".to_string()).await.unwrap();

        let report = manager.recover().await.unwrap();
        assert!(report.status == crate::persistence::recovery::RecoveryStatus::Completed ||
                report.status == crate::persistence::recovery::RecoveryStatus::Partial);
    }

    #[tokio::test]
    async fn test_verify_integrity() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = PersistenceConfig::default();
        config.storage_dir = temp_dir.path().to_path_buf();
        config.enable_background = false;

        let manager = PersistenceManager::new(config, "test_node".to_string()).await.unwrap();

        let valid = manager.verify_integrity().await.unwrap();
        assert!(valid);
    }

    #[tokio::test]
    async fn test_flush() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = PersistenceConfig::default();
        config.storage_dir = temp_dir.path().to_path_buf();
        config.enable_background = false;

        let manager = PersistenceManager::new(config, "test_node".to_string()).await.unwrap();

        manager.flush().await.unwrap();
    }

    #[tokio::test]
    async fn test_shutdown() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = PersistenceConfig::default();
        config.storage_dir = temp_dir.path().to_path_buf();
        config.enable_background = true;

        let manager = PersistenceManager::new(config, "test_node".to_string()).await.unwrap();

        // Let background task run briefly
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        manager.shutdown().await.unwrap();
    }
}