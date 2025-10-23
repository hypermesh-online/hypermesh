//! Storage engine abstraction and MVCC implementation

use super::{
    Term, LogIndex,
    config::{StorageConfig, RocksDBConfig, CompressionType},
    error::{ConsensusError, Result},
};

use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::path::Path;
#[cfg(feature = "rocksdb-storage")]
use rocksdb::{DB, Options, WriteBatch, IteratorMode};
use tracing::{debug, warn, info, error};

/// Timestamp for MVCC versioning
pub type Timestamp = u64;

/// Transaction ID for tracking operations
pub type TransactionId = uuid::Uuid;

/// Storage engine trait for pluggable backends
#[async_trait]
pub trait StorageEngine: Send + Sync {
    /// Get a value by key
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    
    /// Put a key-value pair
    async fn put(&self, key: &str, value: Vec<u8>) -> Result<()>;
    
    /// Delete a key
    async fn delete(&self, key: &str) -> Result<()>;
    
    /// Get multiple keys atomically
    async fn multi_get(&self, keys: &[String]) -> Result<Vec<Option<Vec<u8>>>>;
    
    /// Batch write operations
    async fn batch_write(&self, operations: Vec<WriteOperation>) -> Result<()>;
    
    /// Create an iterator over key-value pairs
    async fn scan(&self, start_key: &str, end_key: &str) -> Result<Vec<(String, Vec<u8>)>>;
    
    /// Get storage statistics
    async fn stats(&self) -> Result<StorageStats>;
    
    /// Compact storage to reclaim space
    async fn compact(&self) -> Result<()>;
    
    /// Create a snapshot for consistent reads
    async fn snapshot(&self) -> Result<Box<dyn StorageSnapshot>>;
    
    /// Close the storage engine
    async fn close(&self) -> Result<()>;
}

/// Write operation for batch writes
#[derive(Debug, Clone)]
pub enum WriteOperation {
    Put { key: String, value: Vec<u8> },
    Delete { key: String },
}

/// Storage statistics for monitoring
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_keys: u64,
    pub total_size_bytes: u64,
    pub memory_usage_bytes: u64,
    pub disk_usage_bytes: u64,
    pub read_count: u64,
    pub write_count: u64,
    pub delete_count: u64,
}

/// Storage snapshot for consistent reads
#[async_trait]
pub trait StorageSnapshot: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn scan(&self, start_key: &str, end_key: &str) -> Result<Vec<(String, Vec<u8>)>>;
}

/// Versioned value for MVCC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    /// The actual value
    pub value: Vec<u8>,
    /// Timestamp when this version was created
    pub timestamp: Timestamp,
    /// Transaction that created this version
    pub transaction_id: TransactionId,
    /// Whether this version is deleted
    pub deleted: bool,
    /// Checksum for integrity
    pub checksum: Vec<u8>,
}

impl Version {
    /// Create a new version
    pub fn new(value: Vec<u8>, timestamp: Timestamp, transaction_id: TransactionId) -> Self {
        let checksum = Self::compute_checksum(&value, timestamp);
        Self {
            value,
            timestamp,
            transaction_id,
            deleted: false,
            checksum,
        }
    }
    
    /// Create a deleted version
    pub fn deleted(timestamp: Timestamp, transaction_id: TransactionId) -> Self {
        Self {
            value: Vec::new(),
            timestamp,
            transaction_id,
            deleted: true,
            checksum: Vec::new(),
        }
    }
    
    /// Compute checksum for integrity
    fn compute_checksum(value: &[u8], timestamp: Timestamp) -> Vec<u8> {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(value);
        hasher.update(&timestamp.to_le_bytes());
        hasher.finalize().to_vec()
    }
    
    /// Verify integrity
    pub fn verify_integrity(&self) -> bool {
        if self.deleted {
            return true; // Deleted versions don't need checksum verification
        }
        let expected = Self::compute_checksum(&self.value, self.timestamp);
        self.checksum == expected
    }
    
    /// Get the size of this version
    pub fn size(&self) -> usize {
        self.value.len() + 
        std::mem::size_of::<Timestamp>() +
        std::mem::size_of::<TransactionId>() +
        std::mem::size_of::<bool>() +
        self.checksum.len()
    }
}

/// Multi-Version Concurrency Control storage
pub struct MVCCStorage {
    /// Underlying storage engine
    storage: Arc<dyn StorageEngine>,
    
    /// In-memory version cache for performance
    version_cache: Arc<RwLock<BTreeMap<String, Vec<Version>>>>,
    
    /// Garbage collection watermark
    gc_watermark: Arc<RwLock<Timestamp>>,
    
    /// Configuration
    config: StorageConfig,
    
    /// Current timestamp counter
    timestamp_counter: Arc<RwLock<Timestamp>>,
    
    /// Write statistics
    stats: Arc<RwLock<StorageStats>>,
}

impl MVCCStorage {
    /// Create a new MVCC storage
    pub async fn new(config: &StorageConfig) -> Result<Arc<Self>> {
        info!("Initializing MVCC storage at {:?}", config.data_dir);
        
        // Create storage backend - use RocksDB if available, otherwise use in-memory
        let storage: Arc<dyn StorageEngine> = {
            #[cfg(feature = "rocksdb-storage")]
            {
                Arc::new(RocksDBStorage::new(config).await?)
            }
            #[cfg(not(feature = "rocksdb-storage"))]
            {
                warn!("RocksDB feature disabled, using in-memory storage");
                Arc::new(MockStorage::new())
            }
        };
        
        let mvcc = Self {
            storage,
            version_cache: Arc::new(RwLock::new(BTreeMap::new())),
            gc_watermark: Arc::new(RwLock::new(0)),
            config: config.clone(),
            timestamp_counter: Arc::new(RwLock::new(1)),
            stats: Arc::new(RwLock::new(StorageStats {
                total_keys: 0,
                total_size_bytes: 0,
                memory_usage_bytes: 0,
                disk_usage_bytes: 0,
                read_count: 0,
                write_count: 0,
                delete_count: 0,
            })),
        };
        
        // Load existing data into cache
        mvcc.load_version_cache().await?;
        
        // Wrap in Arc and start garbage collection task
        let mvcc_arc = Arc::new(mvcc);
        mvcc_arc.clone().start_gc_task();
        
        Ok(mvcc_arc)
    }
    
    /// Read a value at a specific timestamp
    pub async fn read(&self, key: &str, timestamp: Timestamp) -> Result<Option<Vec<u8>>> {
        debug!("Reading key {} at timestamp {}", key, timestamp);
        
        // Update read statistics
        {
            let mut stats = self.stats.write().await;
            stats.read_count += 1;
        }
        
        // First check cache
        {
            let cache = self.version_cache.read().await;
            if let Some(versions) = cache.get(key) {
                // Find the latest version at or before the requested timestamp
                for version in versions.iter().rev() {
                    if version.timestamp <= timestamp {
                        if version.deleted {
                            return Ok(None);
                        }
                        
                        // Verify integrity
                        if !version.verify_integrity() {
                            warn!("Integrity check failed for key {} at timestamp {}", key, timestamp);
                            continue;
                        }
                        
                        return Ok(Some(version.value.clone()));
                    }
                }
            }
        }
        
        // If not in cache, load from storage
        self.load_versions_for_key(key).await?;
        
        // Try cache again
        {
            let cache = self.version_cache.read().await;
            if let Some(versions) = cache.get(key) {
                for version in versions.iter().rev() {
                    if version.timestamp <= timestamp {
                        if version.deleted {
                            return Ok(None);
                        }
                        
                        if !version.verify_integrity() {
                            warn!("Integrity check failed for key {} at timestamp {}", key, timestamp);
                            continue;
                        }
                        
                        return Ok(Some(version.value.clone()));
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// Write a value with a new timestamp
    pub async fn write(&self, key: String, value: Vec<u8>, transaction_id: TransactionId) -> Result<Timestamp> {
        debug!("Writing key {} (length: {})", key, value.len());
        
        // Get next timestamp
        let timestamp = self.next_timestamp().await;
        
        // Create new version
        let version = Version::new(value.clone(), timestamp, transaction_id);
        
        // Add to cache
        {
            let mut cache = self.version_cache.write().await;
            let versions = cache.entry(key.clone()).or_insert_with(Vec::new);
            versions.push(version.clone());
            versions.sort_by_key(|v| v.timestamp);
            
            // Limit versions per key
            if versions.len() > self.config.max_versions_per_key {
                versions.remove(0);
            }
        }
        
        // Persist to storage
        let version_key = format!("{}#{}", key, timestamp);
        let version_data = bincode::serialize(&version)
            .map_err(|e| ConsensusError::SerializationError(format!("Version serialization failed: {}", e)))?;
        
        self.storage.put(&version_key, version_data).await?;
        
        // Update write statistics
        {
            let mut stats = self.stats.write().await;
            stats.write_count += 1;
            stats.total_size_bytes += version.size() as u64;
        }
        
        Ok(timestamp)
    }
    
    /// Delete a key by creating a tombstone version
    pub async fn delete(&self, key: String, transaction_id: TransactionId) -> Result<Timestamp> {
        debug!("Deleting key {}", key);
        
        let timestamp = self.next_timestamp().await;
        let version = Version::deleted(timestamp, transaction_id);
        
        // Add tombstone to cache
        {
            let mut cache = self.version_cache.write().await;
            let versions = cache.entry(key.clone()).or_insert_with(Vec::new);
            versions.push(version.clone());
            versions.sort_by_key(|v| v.timestamp);
        }
        
        // Persist tombstone
        let version_key = format!("{}#{}", key, timestamp);
        let version_data = bincode::serialize(&version)
            .map_err(|e| ConsensusError::SerializationError(format!("Version serialization failed: {}", e)))?;
        
        self.storage.put(&version_key, version_data).await?;
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.delete_count += 1;
        }
        
        Ok(timestamp)
    }
    
    /// Check for write conflicts
    pub async fn check_write_conflicts(
        &self,
        write_set: &HashMap<String, Vec<u8>>,
        start_timestamp: Timestamp,
        commit_timestamp: Timestamp,
    ) -> Result<Vec<ConflictInfo>> {
        let mut conflicts = Vec::new();
        
        for key in write_set.keys() {
            // Check if any version was written between start and commit timestamp
            let cache = self.version_cache.read().await;
            if let Some(versions) = cache.get(key) {
                for version in versions {
                    if version.timestamp > start_timestamp && version.timestamp < commit_timestamp {
                        conflicts.push(ConflictInfo {
                            key: key.clone(),
                            conflict_type: ConflictType::WriteWrite,
                            conflicting_timestamp: version.timestamp,
                            conflicting_transaction: version.transaction_id,
                        });
                        break;
                    }
                }
            }
        }
        
        Ok(conflicts)
    }
    
    /// Check for read conflicts
    pub async fn check_read_conflicts(
        &self,
        read_set: &HashMap<String, Timestamp>,
        start_timestamp: Timestamp,
        commit_timestamp: Timestamp,
    ) -> Result<Vec<ConflictInfo>> {
        let mut conflicts = Vec::new();
        
        for (key, read_timestamp) in read_set {
            // Check if any version was written after the read timestamp but before commit
            let cache = self.version_cache.read().await;
            if let Some(versions) = cache.get(key) {
                for version in versions {
                    if version.timestamp > *read_timestamp && version.timestamp < commit_timestamp {
                        conflicts.push(ConflictInfo {
                            key: key.clone(),
                            conflict_type: ConflictType::ReadWrite,
                            conflicting_timestamp: version.timestamp,
                            conflicting_transaction: version.transaction_id,
                        });
                        break;
                    }
                }
            }
        }
        
        Ok(conflicts)
    }
    
    /// Garbage collect old versions
    pub async fn gc_old_versions(&self, watermark: Timestamp) -> Result<usize> {
        debug!("Starting garbage collection with watermark {}", watermark);
        
        let mut removed_count = 0;
        let mut operations = Vec::new();
        
        // Update watermark
        *self.gc_watermark.write().await = watermark;
        
        // Process each key in cache
        {
            let mut cache = self.version_cache.write().await;
            for (key, versions) in cache.iter_mut() {
                let initial_len = versions.len();
                
                // Keep only versions newer than watermark, plus one older version for reads
                versions.sort_by_key(|v| v.timestamp);
                let mut keep_index = 0;
                
                // Find the first version we need to keep
                for (i, version) in versions.iter().enumerate() {
                    if version.timestamp > watermark {
                        // Keep one version before the watermark if available
                        keep_index = if i > 0 { i - 1 } else { i };
                        break;
                    }
                    
                    // Schedule storage deletion for old versions
                    let version_key = format!("{}#{}", key, version.timestamp);
                    operations.push(WriteOperation::Delete { key: version_key });
                }
                
                // Remove old versions from cache
                if keep_index > 0 {
                    versions.drain(0..keep_index);
                    removed_count += initial_len - versions.len();
                }
            }
        }
        
        // Execute storage deletions
        if !operations.is_empty() {
            self.storage.batch_write(operations).await?;
        }
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_keys = self.version_cache.read().await.len() as u64;
        }
        
        info!("Garbage collection completed, removed {} versions", removed_count);
        Ok(removed_count)
    }
    
    /// Get current timestamp
    pub async fn current_timestamp(&self) -> Timestamp {
        *self.timestamp_counter.read().await
    }
    
    /// Get storage statistics
    pub async fn statistics(&self) -> StorageStats {
        let mut stats = self.stats.read().await.clone();
        
        // Update cache size
        let cache = self.version_cache.read().await;
        stats.memory_usage_bytes = cache.iter()
            .map(|(key, versions)| {
                key.len() + versions.iter().map(|v| v.size()).sum::<usize>()
            })
            .sum::<usize>() as u64;
        
        // Get underlying storage stats
        if let Ok(storage_stats) = self.storage.stats().await {
            stats.disk_usage_bytes = storage_stats.disk_usage_bytes;
        }
        
        stats
    }
    
    /// Get next timestamp (atomic increment)
    async fn next_timestamp(&self) -> Timestamp {
        let mut counter = self.timestamp_counter.write().await;
        *counter += 1;
        *counter
    }
    
    /// Load version cache from storage
    async fn load_version_cache(&self) -> Result<()> {
        debug!("Loading version cache from storage");
        
        // Scan all version entries
        let entries = self.storage.scan("", "~").await?;
        let mut cache = self.version_cache.write().await;
        
        for (version_key, version_data) in entries {
            if let Some(hash_pos) = version_key.rfind('#') {
                let key = &version_key[..hash_pos];
                
                // Deserialize version
                if let Ok(version) = bincode::deserialize::<Version>(&version_data) {
                    let versions = cache.entry(key.to_string()).or_insert_with(Vec::new);
                    versions.push(version);
                }
            }
        }
        
        // Sort all versions by timestamp
        for versions in cache.values_mut() {
            versions.sort_by_key(|v| v.timestamp);
        }
        
        info!("Loaded {} keys into version cache", cache.len());
        Ok(())
    }
    
    /// Load versions for a specific key from storage
    async fn load_versions_for_key(&self, key: &str) -> Result<()> {
        let start_key = format!("{}", key);
        let end_key = format!("{}~", key);
        
        let entries = self.storage.scan(&start_key, &end_key).await?;
        let mut cache = self.version_cache.write().await;
        let versions = cache.entry(key.to_string()).or_insert_with(Vec::new);
        
        for (version_key, version_data) in entries {
            if let Ok(version) = bincode::deserialize::<Version>(&version_data) {
                if !versions.iter().any(|v| v.timestamp == version.timestamp) {
                    versions.push(version);
                }
            }
        }
        
        versions.sort_by_key(|v| v.timestamp);
        Ok(())
    }
    
    /// Start garbage collection background task
    fn start_gc_task(self: Arc<Self>) {
        let storage = Arc::downgrade(&self);
        let gc_interval = Duration::from_secs(self.config.gc_interval_seconds);
        let gc_lag = self.config.gc_watermark_lag_seconds;
        
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(gc_interval).await;
                
                if let Some(storage) = storage.upgrade() {
                    let current_time = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    
                    let watermark = current_time.saturating_sub(gc_lag);
                    
                    if let Err(e) = storage.gc_old_versions(watermark).await {
                        error!("Garbage collection failed: {}", e);
                    }
                } else {
                    break; // Storage has been dropped
                }
            }
        });
    }
}

/// Conflict information for transaction validation
#[derive(Debug, Clone)]
pub struct ConflictInfo {
    pub key: String,
    pub conflict_type: ConflictType,
    pub conflicting_timestamp: Timestamp,
    pub conflicting_transaction: TransactionId,
}

/// Types of conflicts that can occur
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictType {
    ReadWrite,  // Read-Write conflict
    WriteWrite, // Write-Write conflict
}

/// RocksDB storage implementation
#[cfg(feature = "rocksdb-storage")]
struct RocksDBStorage {
    db: Arc<DB>,
    stats: Arc<RwLock<StorageStats>>,
}

#[cfg(feature = "rocksdb-storage")]
impl RocksDBStorage {
    async fn new(config: &StorageConfig) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_max_open_files(config.rocksdb.max_open_files);
        
        // Set compression
        match config.rocksdb.compression_type {
            CompressionType::None => opts.set_compression_type(rocksdb::DBCompressionType::None),
            CompressionType::Snappy => opts.set_compression_type(rocksdb::DBCompressionType::Snappy),
            CompressionType::Zlib => opts.set_compression_type(rocksdb::DBCompressionType::Zlib),
            CompressionType::Bz2 => opts.set_compression_type(rocksdb::DBCompressionType::Bz2),
            CompressionType::Lz4 => opts.set_compression_type(rocksdb::DBCompressionType::Lz4),
            CompressionType::Lz4Hc => opts.set_compression_type(rocksdb::DBCompressionType::Lz4hc),
            CompressionType::Zstd => opts.set_compression_type(rocksdb::DBCompressionType::Zstd),
        }
        
        // Memory settings
        opts.set_write_buffer_size(config.write_buffer_size_mb * 1024 * 1024);
        
        // Bloom filter
        if config.rocksdb.enable_bloom_filter {
            opts.set_bloom_filter(config.rocksdb.bloom_filter_bits_per_key as f64, false);
        }
        
        let db = DB::open(&opts, &config.data_dir)
            .map_err(|e| ConsensusError::StorageError(format!("Failed to open RocksDB: {}", e)))?;
        
        Ok(Self {
            db: Arc::new(db),
            stats: Arc::new(RwLock::new(StorageStats {
                total_keys: 0,
                total_size_bytes: 0,
                memory_usage_bytes: 0,
                disk_usage_bytes: 0,
                read_count: 0,
                write_count: 0,
                delete_count: 0,
            })),
        })
    }
}

#[cfg(feature = "rocksdb-storage")]
#[async_trait]
impl StorageEngine for RocksDBStorage {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let result = self.db.get(key)
            .map_err(|e| ConsensusError::StorageError(format!("RocksDB get failed: {}", e)))?;
        
        {
            let mut stats = self.stats.write().await;
            stats.read_count += 1;
        }
        
        Ok(result)
    }
    
    async fn put(&self, key: &str, value: Vec<u8>) -> Result<()> {
        self.db.put(key, &value)
            .map_err(|e| ConsensusError::StorageError(format!("RocksDB put failed: {}", e)))?;
        
        {
            let mut stats = self.stats.write().await;
            stats.write_count += 1;
            stats.total_size_bytes += value.len() as u64;
        }
        
        Ok(())
    }
    
    async fn delete(&self, key: &str) -> Result<()> {
        self.db.delete(key)
            .map_err(|e| ConsensusError::StorageError(format!("RocksDB delete failed: {}", e)))?;
        
        {
            let mut stats = self.stats.write().await;
            stats.delete_count += 1;
        }
        
        Ok(())
    }
    
    async fn multi_get(&self, keys: &[String]) -> Result<Vec<Option<Vec<u8>>>> {
        let key_refs: Vec<&str> = keys.iter().map(|k| k.as_str()).collect();
        let results = self.db.multi_get(key_refs);
        
        let mut output = Vec::new();
        for result in results {
            match result {
                Ok(value) => output.push(value),
                Err(e) => return Err(ConsensusError::StorageError(format!("RocksDB multi_get failed: {}", e))),
            }
        }
        
        {
            let mut stats = self.stats.write().await;
            stats.read_count += keys.len() as u64;
        }
        
        Ok(output)
    }
    
    async fn batch_write(&self, operations: Vec<WriteOperation>) -> Result<()> {
        let mut batch = WriteBatch::default();
        
        for op in &operations {
            match op {
                WriteOperation::Put { key, value } => {
                    batch.put(key, value);
                }
                WriteOperation::Delete { key } => {
                    batch.delete(key);
                }
            }
        }
        
        self.db.write(batch)
            .map_err(|e| ConsensusError::StorageError(format!("RocksDB batch write failed: {}", e)))?;
        
        {
            let mut stats = self.stats.write().await;
            for op in operations {
                match op {
                    WriteOperation::Put { value, .. } => {
                        stats.write_count += 1;
                        stats.total_size_bytes += value.len() as u64;
                    }
                    WriteOperation::Delete { .. } => {
                        stats.delete_count += 1;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    async fn scan(&self, start_key: &str, end_key: &str) -> Result<Vec<(String, Vec<u8>)>> {
        let mut results = Vec::new();
        
        let iter = self.db.iterator(IteratorMode::From(start_key.as_bytes(), rocksdb::Direction::Forward));
        
        for item in iter {
            let (key, value) = item
                .map_err(|e| ConsensusError::StorageError(format!("RocksDB scan failed: {}", e)))?;
            
            let key_str = String::from_utf8_lossy(&key).to_string();
            
            if key_str >= end_key {
                break;
            }
            
            results.push((key_str, value.to_vec()));
        }
        
        Ok(results)
    }
    
    async fn stats(&self) -> Result<StorageStats> {
        Ok(self.stats.read().await.clone())
    }
    
    async fn compact(&self) -> Result<()> {
        self.db.compact_range::<&str, &str>(None, None);
        Ok(())
    }
    
    async fn snapshot(&self) -> Result<Box<dyn StorageSnapshot>> {
        let snapshot = self.db.snapshot();
        Ok(Box::new(RocksDBSnapshot { snapshot }))
    }
    
    async fn close(&self) -> Result<()> {
        // RocksDB closes automatically when dropped
        Ok(())
    }
}

/// RocksDB snapshot implementation
#[cfg(feature = "rocksdb-storage")]
struct RocksDBSnapshot {
    snapshot: rocksdb::Snapshot<'static>,
}

#[cfg(feature = "rocksdb-storage")]
#[async_trait]
impl StorageSnapshot for RocksDBSnapshot {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let result = self.snapshot.get(key)
            .map_err(|e| ConsensusError::StorageError(format!("Snapshot get failed: {}", e)))?;
        Ok(result)
    }
    
    async fn scan(&self, start_key: &str, end_key: &str) -> Result<Vec<(String, Vec<u8>)>> {
        let mut results = Vec::new();
        
        let iter = self.snapshot.iterator(IteratorMode::From(start_key.as_bytes(), rocksdb::Direction::Forward));
        
        for item in iter {
            let (key, value) = item
                .map_err(|e| ConsensusError::StorageError(format!("Snapshot scan failed: {}", e)))?;
            
            let key_str = String::from_utf8_lossy(&key).to_string();
            
            if key_str >= end_key {
                break;
            }
            
            results.push((key_str, value.to_vec()));
        }
        
        Ok(results)
    }
}

/// Mock storage for testing and non-RocksDB builds
pub struct MockStorage {
    data: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl MockStorage {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl StorageEngine for MockStorage {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        Ok(self.data.read().await.get(key).cloned())
    }
    
    async fn put(&self, key: &str, value: Vec<u8>) -> Result<()> {
        self.data.write().await.insert(key.to_string(), value);
        Ok(())
    }
    
    async fn delete(&self, key: &str) -> Result<()> {
        self.data.write().await.remove(key);
        Ok(())
    }
    
    async fn multi_get(&self, keys: &[String]) -> Result<Vec<Option<Vec<u8>>>> {
        let data = self.data.read().await;
        Ok(keys.iter().map(|k| data.get(k).cloned()).collect())
    }
    
    async fn batch_write(&self, operations: Vec<WriteOperation>) -> Result<()> {
        let mut data = self.data.write().await;
        for op in operations {
            match op {
                WriteOperation::Put { key, value } => {
                    data.insert(key, value);
                }
                WriteOperation::Delete { key } => {
                    data.remove(&key);
                }
            }
        }
        Ok(())
    }
    
    async fn scan(&self, start_key: &str, end_key: &str) -> Result<Vec<(String, Vec<u8>)>> {
        let data = self.data.read().await;
        let mut results: Vec<_> = data.iter()
            .filter(|(k, _)| k.as_str() >= start_key && k.as_str() < end_key)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        results.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(results)
    }
    
    async fn stats(&self) -> Result<StorageStats> {
        let data = self.data.read().await;
        let total_size = data.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>() as u64;
        
        Ok(StorageStats {
            total_keys: data.len() as u64,
            total_size_bytes: total_size,
            memory_usage_bytes: total_size,
            disk_usage_bytes: 0,
            read_count: 0,
            write_count: 0,
            delete_count: 0,
        })
    }
    
    async fn compact(&self) -> Result<()> {
        Ok(())
    }
    
    async fn snapshot(&self) -> Result<Box<dyn StorageSnapshot>> {
        unimplemented!("MockStorage snapshot not implemented")
    }
    
    async fn close(&self) -> Result<()> {
        Ok(())
    }
}

/// Implement StorageEngine trait for MVCCStorage
#[async_trait]
impl StorageEngine for MVCCStorage {
    async fn put(&self, key: &str, value: Vec<u8>) -> Result<()> {
        // Create a synthetic transaction ID for non-transactional operations
        let txn_id = TransactionId::new_v4();
        self.write(key.to_string(), value, txn_id).await?;
        Ok(())
    }
    
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let read_ts = self.next_timestamp().await;
        Ok(self.read(key, read_ts).await?)
    }
    
    async fn delete(&self, key: &str) -> Result<()> {
        // Create a synthetic transaction ID for non-transactional operations  
        let txn_id = TransactionId::new_v4();
        self.delete(key.to_string(), txn_id).await?;
        Ok(())
    }
    
    async fn multi_get(&self, keys: &[String]) -> Result<Vec<Option<Vec<u8>>>> {
        let read_ts = self.next_timestamp().await;
        let mut results = Vec::with_capacity(keys.len());
        
        for key in keys {
            let value = self.read(key, read_ts).await?;
            results.push(value);
        }
        
        Ok(results)
    }
    
    async fn batch_write(&self, operations: Vec<WriteOperation>) -> Result<()> {
        let txn_id = TransactionId::new_v4();
        
        for operation in operations {
            match operation {
                WriteOperation::Put { key, value } => {
                    self.write(key, value, txn_id).await?;
                }
                WriteOperation::Delete { key } => {
                    self.delete(key, txn_id).await?;
                }
            }
        }
        
        Ok(())
    }
    
    async fn scan(&self, start_key: &str, end_key: &str) -> Result<Vec<(String, Vec<u8>)>> {
        // For MVCC, we need to scan at a specific timestamp
        let read_ts = self.next_timestamp().await;
        
        // Get all keys from the underlying storage with version prefixes
        let all_data = self.storage.scan(&format!("{}#", start_key), &format!("{}#", end_key)).await?;
        let mut results = Vec::new();
        
        // Group by key and find latest version for each key
        let mut key_versions: BTreeMap<String, Vec<(Timestamp, Vec<u8>)>> = BTreeMap::new();
        
        for (versioned_key, data) in all_data {
            if let Some(hash_pos) = versioned_key.rfind('#') {
                let key = versioned_key[..hash_pos].to_string();
                if let Ok(timestamp_str) = versioned_key[hash_pos + 1..].parse::<u64>() {
                    if timestamp_str <= read_ts {
                        key_versions.entry(key).or_insert_with(Vec::new).push((timestamp_str, data));
                    }
                }
            }
        }
        
        // Get latest version for each key
        for (key, mut versions) in key_versions {
            if key.as_str() >= start_key && key.as_str() < end_key {
                versions.sort_by_key(|(ts, _)| *ts);
                if let Some((_, data)) = versions.last() {
                    // Deserialize to check if it's a deletion
                    if let Ok(version) = bincode::deserialize::<Version>(data) {
                        if !version.deleted {
                            results.push((key, version.value));
                        }
                    }
                }
            }
        }
        
        Ok(results)
    }
    
    async fn stats(&self) -> Result<StorageStats> {
        // Delegate to underlying storage and enhance with MVCC stats
        let base_stats = self.storage.stats().await?;
        let mvcc_stats = self.stats.read().await.clone();
        
        Ok(StorageStats {
            total_keys: mvcc_stats.total_keys,
            total_size_bytes: mvcc_stats.total_size_bytes,
            memory_usage_bytes: base_stats.memory_usage_bytes + mvcc_stats.memory_usage_bytes,
            disk_usage_bytes: base_stats.disk_usage_bytes,
            read_count: mvcc_stats.read_count,
            write_count: mvcc_stats.write_count,
            delete_count: mvcc_stats.delete_count,
        })
    }
    
    async fn compact(&self) -> Result<()> {
        // Trigger garbage collection and delegate to underlying storage
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let gc_lag = self.config.gc_watermark_lag_seconds;
        let watermark = current_time.saturating_sub(gc_lag);
        
        self.gc_old_versions(watermark).await?;
        self.storage.compact().await
    }
    
    async fn snapshot(&self) -> Result<Box<dyn StorageSnapshot>> {
        self.storage.snapshot().await
    }
    
    async fn close(&self) -> Result<()> {
        self.storage.close().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_mock_storage() {
        let storage = MockStorage::new();
        
        // Test basic operations
        storage.put("key1", b"value1".to_vec()).await.unwrap();
        let result = storage.get("key1").await.unwrap();
        assert_eq!(result, Some(b"value1".to_vec()));
        
        // Test deletion
        storage.delete("key1").await.unwrap();
        let result = storage.get("key1").await.unwrap();
        assert_eq!(result, None);
    }
    
    #[tokio::test]
    async fn test_mvcc_storage() {
        let temp_dir = tempdir().unwrap();
        let config = StorageConfig {
            data_dir: temp_dir.path().to_path_buf(),
            max_versions_per_key: 10,
            gc_interval_seconds: 3600,
            gc_watermark_lag_seconds: 1800,
            version_compression: true,
            rocksdb: RocksDBConfig::default(),
            memtable_size_mb: 64,
            write_buffer_size_mb: 32,
        };
        
        let storage = MVCCStorage::new(&config).await.unwrap();
        let txn_id = TransactionId::new_v4();
        
        // Write a value
        let ts1 = storage.write("test_key".to_string(), b"value1".to_vec(), txn_id).await.unwrap();
        
        // Read at that timestamp
        let result = storage.read("test_key", ts1).await.unwrap();
        assert_eq!(result, Some(b"value1".to_vec()));
        
        // Write another version
        let ts2 = storage.write("test_key".to_string(), b"value2".to_vec(), txn_id).await.unwrap();
        
        // Read at old timestamp should return old value
        let result = storage.read("test_key", ts1).await.unwrap();
        assert_eq!(result, Some(b"value1".to_vec()));
        
        // Read at new timestamp should return new value
        let result = storage.read("test_key", ts2).await.unwrap();
        assert_eq!(result, Some(b"value2".to_vec()));
        
        // Delete the key
        let ts3 = storage.delete("test_key".to_string(), txn_id).await.unwrap();
        
        // Read at delete timestamp should return None
        let result = storage.read("test_key", ts3).await.unwrap();
        assert_eq!(result, None);
        
        // But reading at earlier timestamp should still work
        let result = storage.read("test_key", ts2).await.unwrap();
        assert_eq!(result, Some(b"value2".to_vec()));
    }
    
    #[tokio::test]
    async fn test_conflict_detection() {
        let temp_dir = tempdir().unwrap();
        let config = StorageConfig {
            data_dir: temp_dir.path().to_path_buf(),
            max_versions_per_key: 10,
            gc_interval_seconds: 3600,
            gc_watermark_lag_seconds: 1800,
            version_compression: true,
            rocksdb: RocksDBConfig::default(),
            memtable_size_mb: 64,
            write_buffer_size_mb: 32,
        };
        
        let storage = MVCCStorage::new(&config).await.unwrap();
        let txn1 = TransactionId::new_v4();
        let txn2 = TransactionId::new_v4();
        
        // Transaction 1 writes
        let ts1 = storage.write("key1".to_string(), b"value1".to_vec(), txn1).await.unwrap();
        
        // Transaction 2 tries to write same key
        let ts2 = storage.write("key1".to_string(), b"value2".to_vec(), txn2).await.unwrap();
        
        // Check for conflicts
        let mut write_set = HashMap::new();
        write_set.insert("key1".to_string(), b"value3".to_vec());
        
        let conflicts = storage.check_write_conflicts(&write_set, ts1, ts2).await.unwrap();
        assert!(!conflicts.is_empty());
        assert_eq!(conflicts[0].conflict_type, ConflictType::WriteWrite);
    }
}