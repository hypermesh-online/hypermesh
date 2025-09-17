//! Certificate Transparency Storage Backend
//! 
//! Persistent storage for CT log entries with SQLite backend,
//! efficient indexing, and data integrity verification.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
use sqlx::{SqlitePool, Row, sqlite::SqliteConnectOptions, ConnectOptions};
use anyhow::{Result, anyhow};
use tracing::{debug, info, warn, error};

use crate::errors::{StorageError, Result as TrustChainResult};
use super::LogEntry;

/// CT storage backend using SQLite
pub struct CTStorage {
    /// Database connection pool
    pool: SqlitePool,
    /// Storage file path
    storage_path: String,
    /// Next sequence number cache
    next_sequence: std::sync::atomic::AtomicU64,
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

        // Ensure directory exists
        if let Some(parent) = Path::new(storage_path).parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| StorageError::FileSystem {
                    path: parent.to_string_lossy().to_string(),
                    reason: e.to_string(),
                })?;
        }

        // Create database connection
        let database_url = format!("sqlite:{}/ct_logs.db", storage_path);
        let options = SqliteConnectOptions::new()
            .filename(&format!("{}/ct_logs.db", storage_path))
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
            .disable_statement_logging();

        let pool = SqlitePool::connect_with(options).await
            .map_err(|e| StorageError::Database {
                operation: "connect".to_string(),
                reason: e.to_string(),
            })?;

        let storage = Self {
            pool,
            storage_path: storage_path.to_string(),
            next_sequence: std::sync::atomic::AtomicU64::new(0),
        };

        // Initialize database schema
        storage.initialize_schema().await?;

        // Load next sequence number
        storage.load_next_sequence().await?;

        info!("CT storage initialized successfully");
        Ok(storage)
    }

    /// Store a log entry
    pub async fn store_entry(&self, entry: &LogEntry) -> TrustChainResult<()> {
        debug!("Storing CT log entry: {}", entry.sequence_number);

        let timestamp_secs = entry.timestamp
            .duration_since(UNIX_EPOCH)
            .map_err(|e| StorageError::Database {
                operation: "timestamp_conversion".to_string(),
                reason: e.to_string(),
            })?
            .as_secs() as i64;

        // Serialize consensus proof
        let consensus_proof_bytes = entry.consensus_proof.to_bytes()
            .map_err(|e| StorageError::Database {
                operation: "serialize_consensus_proof".to_string(),
                reason: e.to_string(),
            })?;

        sqlx::query(
            r#"
            INSERT INTO ct_entries (
                sequence_number, certificate_der, fingerprint, timestamp,
                common_name, issuer_ca_id, consensus_proof, entry_id, leaf_hash
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(entry.sequence_number as i64)
        .bind(&entry.certificate_der)
        .bind(entry.fingerprint.as_slice())
        .bind(timestamp_secs)
        .bind(&entry.common_name)
        .bind(&entry.issuer_ca_id)
        .bind(&consensus_proof_bytes)
        .bind(entry.entry_id.as_slice())
        .bind(entry.leaf_hash.as_slice())
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database {
            operation: "insert_entry".to_string(),
            reason: e.to_string(),
        })?;

        // Update indexes
        self.update_indexes(entry).await?;

        debug!("Stored CT log entry successfully");
        Ok(())
    }

    /// Get log entry by sequence number
    pub async fn get_entry_by_sequence(&self, sequence_number: u64) -> TrustChainResult<Option<LogEntry>> {
        debug!("Retrieving CT log entry: {}", sequence_number);

        let row = sqlx::query("SELECT * FROM ct_entries WHERE sequence_number = ?")
            .bind(sequence_number as i64)
            .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database {
            operation: "select_by_sequence".to_string(),
            reason: e.to_string(),
        })?;

        if let Some(row) = row {
            let entry = self.row_to_log_entry(row).await?;
            Ok(Some(entry))
        } else {
            Ok(None)
        }
    }

    /// Get log entry by fingerprint
    pub async fn get_entry_by_fingerprint(&self, fingerprint: &[u8; 32]) -> TrustChainResult<Option<LogEntry>> {
        debug!("Retrieving CT log entry by fingerprint: {}", hex::encode(fingerprint));

        let row = sqlx::query!(
            "SELECT * FROM ct_entries WHERE fingerprint = ?",
            fingerprint.as_slice()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database {
            operation: "select_by_fingerprint".to_string(),
            reason: e.to_string(),
        })?;

        if let Some(row) = row {
            let entry = self.row_to_log_entry(row).await?;
            Ok(Some(entry))
        } else {
            Ok(None)
        }
    }

    /// Get log entries in range
    pub async fn get_entries_range(&self, start: u64, end: u64) -> TrustChainResult<Vec<LogEntry>> {
        debug!("Retrieving CT log entries: {} to {}", start, end);

        if end <= start {
            return Err(anyhow!("Invalid range: end must be greater than start").into());
        }

        let rows = sqlx::query!(
            "SELECT * FROM ct_entries WHERE sequence_number >= ? AND sequence_number < ? ORDER BY sequence_number",
            start as i64,
            end as i64
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database {
            operation: "select_range".to_string(),
            reason: e.to_string(),
        })?;

        let mut entries = Vec::new();
        for row in rows {
            let entry = self.row_to_log_entry(row).await?;
            entries.push(entry);
        }

        Ok(entries)
    }

    /// Get entries by common name
    pub async fn get_entries_by_common_name(&self, common_name: &str) -> TrustChainResult<Vec<LogEntry>> {
        debug!("Retrieving CT log entries by common name: {}", common_name);

        let rows = sqlx::query!(
            "SELECT * FROM ct_entries WHERE common_name = ? ORDER BY sequence_number",
            common_name
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database {
            operation: "select_by_common_name".to_string(),
            reason: e.to_string(),
        })?;

        let mut entries = Vec::new();
        for row in rows {
            let entry = self.row_to_log_entry(row).await?;
            entries.push(entry);
        }

        Ok(entries)
    }

    /// Get entries by CA ID
    pub async fn get_entries_by_ca_id(&self, ca_id: &str) -> TrustChainResult<Vec<LogEntry>> {
        debug!("Retrieving CT log entries by CA ID: {}", ca_id);

        let rows = sqlx::query!(
            "SELECT * FROM ct_entries WHERE issuer_ca_id = ? ORDER BY sequence_number",
            ca_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database {
            operation: "select_by_ca_id".to_string(),
            reason: e.to_string(),
        })?;

        let mut entries = Vec::new();
        for row in rows {
            let entry = self.row_to_log_entry(row).await?;
            entries.push(entry);
        }

        Ok(entries)
    }

    /// Get next sequence number
    pub async fn get_next_sequence_number(&self) -> TrustChainResult<u64> {
        let next = self.next_sequence.load(std::sync::atomic::Ordering::Acquire);
        Ok(next)
    }

    /// Reserve next sequence number
    pub async fn reserve_sequence_number(&self) -> TrustChainResult<u64> {
        let next = self.next_sequence.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        Ok(next)
    }

    /// Get storage statistics
    pub async fn get_stats(&self) -> TrustChainResult<StorageStats> {
        debug!("Retrieving storage statistics");

        // Get total entries
        let total_entries_row = sqlx::query!("SELECT COUNT(*) as count FROM ct_entries")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| StorageError::Database {
                operation: "count_entries".to_string(),
                reason: e.to_string(),
            })?;
        let total_entries = total_entries_row.count as u64;

        // Get last entry timestamp
        let last_entry_row = sqlx::query!(
            "SELECT timestamp FROM ct_entries ORDER BY sequence_number DESC LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database {
            operation: "get_last_timestamp".to_string(),
            reason: e.to_string(),
        })?;

        let last_entry_timestamp = last_entry_row.map(|row| {
            UNIX_EPOCH + std::time::Duration::from_secs(row.timestamp as u64)
        });

        // Get database file size
        let db_path = format!("{}/ct_logs.db", self.storage_path);
        let database_size_bytes = tokio::fs::metadata(&db_path).await
            .map(|metadata| metadata.len())
            .unwrap_or(0);

        Ok(StorageStats {
            total_entries,
            database_size_bytes,
            last_entry_timestamp,
            storage_path: self.storage_path.clone(),
            index_count: 4, // fingerprint, common_name, issuer_ca_id, timestamp indexes
        })
    }

    /// Perform maintenance operations
    pub async fn maintenance(&self) -> TrustChainResult<()> {
        info!("Performing CT storage maintenance");

        // Analyze query plans and update statistics
        sqlx::query("ANALYZE")
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Database {
                operation: "analyze".to_string(),
                reason: e.to_string(),
            })?;

        // Vacuum database (reclaim space)
        sqlx::query("VACUUM")
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Database {
                operation: "vacuum".to_string(),
                reason: e.to_string(),
            })?;

        // Check integrity
        let integrity_row = sqlx::query!("PRAGMA integrity_check")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| StorageError::Database {
                operation: "integrity_check".to_string(),
                reason: e.to_string(),
            })?;

        if integrity_row.integrity_check != Some("ok".to_string()) {
            error!("Database integrity check failed: {:?}", integrity_row.integrity_check);
            return Err(StorageError::DataCorruption {
                location: self.storage_path.clone(),
            }.into());
        }

        info!("CT storage maintenance completed successfully");
        Ok(())
    }

    /// Flush pending operations
    pub async fn flush(&self) -> TrustChainResult<()> {
        debug!("Flushing CT storage");

        // Force WAL checkpoint
        sqlx::query("PRAGMA wal_checkpoint(FULL)")
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Database {
                operation: "wal_checkpoint".to_string(),
                reason: e.to_string(),
            })?;

        debug!("CT storage flushed successfully");
        Ok(())
    }

    // Internal helper methods

    async fn initialize_schema(&self) -> TrustChainResult<()> {
        debug!("Initializing CT storage schema");

        // Create main entries table
        sqlx::query!(
            r#"
            CREATE TABLE IF NOT EXISTS ct_entries (
                sequence_number INTEGER PRIMARY KEY,
                certificate_der BLOB NOT NULL,
                fingerprint BLOB NOT NULL UNIQUE,
                timestamp INTEGER NOT NULL,
                common_name TEXT NOT NULL,
                issuer_ca_id TEXT NOT NULL,
                consensus_proof BLOB NOT NULL,
                entry_id BLOB NOT NULL UNIQUE,
                leaf_hash BLOB NOT NULL,
                created_at INTEGER DEFAULT (strftime('%s', 'now'))
            )
            "#
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database {
            operation: "create_table_entries".to_string(),
            reason: e.to_string(),
        })?;

        // Create indexes for efficient queries
        let indexes = [
            ("idx_ct_entries_fingerprint", "CREATE INDEX IF NOT EXISTS idx_ct_entries_fingerprint ON ct_entries(fingerprint)"),
            ("idx_ct_entries_common_name", "CREATE INDEX IF NOT EXISTS idx_ct_entries_common_name ON ct_entries(common_name)"),
            ("idx_ct_entries_issuer_ca_id", "CREATE INDEX IF NOT EXISTS idx_ct_entries_issuer_ca_id ON ct_entries(issuer_ca_id)"),
            ("idx_ct_entries_timestamp", "CREATE INDEX IF NOT EXISTS idx_ct_entries_timestamp ON ct_entries(timestamp)"),
            ("idx_ct_entries_entry_id", "CREATE INDEX IF NOT EXISTS idx_ct_entries_entry_id ON ct_entries(entry_id)"),
        ];

        for (name, sql) in &indexes {
            sqlx::query(sql)
                .execute(&self.pool)
                .await
                .map_err(|e| StorageError::Database {
                    operation: format!("create_index_{}", name),
                    reason: e.to_string(),
                })?;
        }

        // Create metadata table for storage info
        sqlx::query!(
            r#"
            CREATE TABLE IF NOT EXISTS ct_metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at INTEGER DEFAULT (strftime('%s', 'now'))
            )
            "#
        )
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database {
            operation: "create_table_metadata".to_string(),
            reason: e.to_string(),
        })?;

        debug!("CT storage schema initialized");
        Ok(())
    }

    async fn load_next_sequence(&self) -> TrustChainResult<()> {
        // Get the highest sequence number + 1
        let row = sqlx::query!(
            "SELECT COALESCE(MAX(sequence_number), -1) + 1 as next_seq FROM ct_entries"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| StorageError::Database {
            operation: "load_next_sequence".to_string(),
            reason: e.to_string(),
        })?;

        let next_seq = row.next_seq.unwrap_or(0) as u64;
        self.next_sequence.store(next_seq, std::sync::atomic::Ordering::Release);

        debug!("Loaded next sequence number: {}", next_seq);
        Ok(())
    }

    async fn update_indexes(&self, _entry: &LogEntry) -> TrustChainResult<()> {
        // Indexes are automatically updated by SQLite
        // This method could be extended for additional custom indexing
        Ok(())
    }

    async fn row_to_log_entry(&self, row: sqlx::sqlite::SqliteRow) -> TrustChainResult<LogEntry> {
        use crate::consensus::ConsensusProof;
        
        let sequence_number = row.get::<i64, _>("sequence_number") as u64;
        let certificate_der: Vec<u8> = row.get("certificate_der");
        let fingerprint_bytes: Vec<u8> = row.get("fingerprint");
        let timestamp_secs = row.get::<i64, _>("timestamp") as u64;
        let common_name: String = row.get("common_name");
        let issuer_ca_id: String = row.get("issuer_ca_id");
        let consensus_proof_bytes: Vec<u8> = row.get("consensus_proof");
        let entry_id_bytes: Vec<u8> = row.get("entry_id");
        let leaf_hash_bytes: Vec<u8> = row.get("leaf_hash");

        // Convert bytes to fixed-size arrays
        let mut fingerprint = [0u8; 32];
        fingerprint.copy_from_slice(&fingerprint_bytes);

        let mut entry_id = [0u8; 32];
        entry_id.copy_from_slice(&entry_id_bytes);

        let mut leaf_hash = [0u8; 32];
        leaf_hash.copy_from_slice(&leaf_hash_bytes);

        // Deserialize consensus proof
        let consensus_proof = ConsensusProof::from_bytes(&consensus_proof_bytes)
            .map_err(|e| StorageError::Database {
                operation: "deserialize_consensus_proof".to_string(),
                reason: e.to_string(),
            })?;

        let timestamp = UNIX_EPOCH + std::time::Duration::from_secs(timestamp_secs);

        Ok(LogEntry {
            sequence_number,
            certificate_der,
            fingerprint,
            timestamp,
            common_name,
            issuer_ca_id,
            consensus_proof,
            entry_id,
            leaf_hash,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use crate::consensus::ConsensusProof;

    async fn create_test_storage() -> (CTStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = CTStorage::new(temp_dir.path().to_str().unwrap()).await.unwrap();
        (storage, temp_dir)
    }

    fn create_test_entry(seq_num: u64) -> LogEntry {
        LogEntry {
            sequence_number: seq_num,
            certificate_der: format!("cert_{}", seq_num).into_bytes(),
            fingerprint: [seq_num as u8; 32],
            timestamp: SystemTime::now(),
            common_name: format!("test{}.example.com", seq_num),
            issuer_ca_id: "test-ca".to_string(),
            consensus_proof: ConsensusProof::default_for_testing(),
            entry_id: [seq_num as u8; 32],
            leaf_hash: [(seq_num + 100) as u8; 32],
        }
    }

    #[tokio::test]
    async fn test_storage_creation() {
        let (_storage, _temp_dir) = create_test_storage().await;
        // Test passes if storage creation doesn't panic
    }

    #[tokio::test]
    async fn test_entry_storage_and_retrieval() {
        let (storage, _temp_dir) = create_test_storage().await;

        let entry = create_test_entry(0);
        storage.store_entry(&entry).await.unwrap();

        let retrieved = storage.get_entry_by_sequence(0).await.unwrap().unwrap();
        assert_eq!(retrieved.sequence_number, entry.sequence_number);
        assert_eq!(retrieved.common_name, entry.common_name);
        assert_eq!(retrieved.fingerprint, entry.fingerprint);
    }

    #[tokio::test]
    async fn test_fingerprint_lookup() {
        let (storage, _temp_dir) = create_test_storage().await;

        let entry = create_test_entry(1);
        storage.store_entry(&entry).await.unwrap();

        let retrieved = storage.get_entry_by_fingerprint(&entry.fingerprint).await.unwrap().unwrap();
        assert_eq!(retrieved.sequence_number, entry.sequence_number);
    }

    #[tokio::test]
    async fn test_range_retrieval() {
        let (storage, _temp_dir) = create_test_storage().await;

        // Store multiple entries
        for i in 0..5 {
            let entry = create_test_entry(i);
            storage.store_entry(&entry).await.unwrap();
        }

        let entries = storage.get_entries_range(1, 4).await.unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].sequence_number, 1);
        assert_eq!(entries[2].sequence_number, 3);
    }

    #[tokio::test]
    async fn test_common_name_lookup() {
        let (storage, _temp_dir) = create_test_storage().await;

        let mut entry1 = create_test_entry(0);
        entry1.common_name = "example.com".to_string();
        
        let mut entry2 = create_test_entry(1);
        entry2.common_name = "example.com".to_string();
        
        let mut entry3 = create_test_entry(2);
        entry3.common_name = "different.com".to_string();

        storage.store_entry(&entry1).await.unwrap();
        storage.store_entry(&entry2).await.unwrap();
        storage.store_entry(&entry3).await.unwrap();

        let entries = storage.get_entries_by_common_name("example.com").await.unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[tokio::test]
    async fn test_ca_id_lookup() {
        let (storage, _temp_dir) = create_test_storage().await;

        let mut entry1 = create_test_entry(0);
        entry1.issuer_ca_id = "ca-1".to_string();
        
        let mut entry2 = create_test_entry(1);
        entry2.issuer_ca_id = "ca-1".to_string();

        storage.store_entry(&entry1).await.unwrap();
        storage.store_entry(&entry2).await.unwrap();

        let entries = storage.get_entries_by_ca_id("ca-1").await.unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[tokio::test]
    async fn test_sequence_number_management() {
        let (storage, _temp_dir) = create_test_storage().await;

        // Initial sequence number should be 0
        let seq1 = storage.get_next_sequence_number().await.unwrap();
        assert_eq!(seq1, 0);

        // Reserve sequence numbers
        let reserved1 = storage.reserve_sequence_number().await.unwrap();
        let reserved2 = storage.reserve_sequence_number().await.unwrap();
        
        assert_eq!(reserved1, 0);
        assert_eq!(reserved2, 1);

        let next = storage.get_next_sequence_number().await.unwrap();
        assert_eq!(next, 2);
    }

    #[tokio::test]
    async fn test_storage_stats() {
        let (storage, _temp_dir) = create_test_storage().await;

        // Store some entries
        for i in 0..3 {
            let entry = create_test_entry(i);
            storage.store_entry(&entry).await.unwrap();
        }

        let stats = storage.get_stats().await.unwrap();
        assert_eq!(stats.total_entries, 3);
        assert!(stats.database_size_bytes > 0);
        assert!(stats.last_entry_timestamp.is_some());
    }

    #[tokio::test]
    async fn test_maintenance_operations() {
        let (storage, _temp_dir) = create_test_storage().await;

        // Store some entries
        for i in 0..5 {
            let entry = create_test_entry(i);
            storage.store_entry(&entry).await.unwrap();
        }

        // Run maintenance - should not error
        storage.maintenance().await.unwrap();
    }

    #[tokio::test]
    async fn test_flush_operations() {
        let (storage, _temp_dir) = create_test_storage().await;

        let entry = create_test_entry(0);
        storage.store_entry(&entry).await.unwrap();

        // Flush should not error
        storage.flush().await.unwrap();
    }

    #[tokio::test]
    async fn test_duplicate_fingerprint_rejection() {
        let (storage, _temp_dir) = create_test_storage().await;

        let entry1 = create_test_entry(0);
        storage.store_entry(&entry1).await.unwrap();

        // Try to store entry with same fingerprint but different sequence
        let mut entry2 = create_test_entry(1);
        entry2.fingerprint = entry1.fingerprint; // Same fingerprint

        let result = storage.store_entry(&entry2).await;
        assert!(result.is_err()); // Should fail due to unique constraint
    }
}