//! High-performance storage engine for state data

use crate::{Result, StateError};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

/// Storage engine implementation
pub struct StateStore {
    config: StorageConfig,
    backend: StorageBackend,
    stats: Arc<RwLock<StorageStats>>,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Data directory
    pub data_dir: String,
    
    /// Storage backend type
    pub backend: StorageBackendType,
    
    /// Maximum database size in bytes
    pub max_size_bytes: u64,
    
    /// Enable write-ahead logging
    pub enable_wal: bool,
    
    /// Sync mode for writes
    pub sync_mode: SyncMode,
    
    /// Compaction settings
    pub compaction: CompactionConfig,
    
    /// Cache settings
    pub cache: CacheConfig,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: "./data/state".to_string(),
            backend: StorageBackendType::RocksDB,
            max_size_bytes: 100 * 1024 * 1024 * 1024, // 100GB
            enable_wal: true,
            sync_mode: SyncMode::Normal,
            compaction: CompactionConfig::default(),
            cache: CacheConfig::default(),
        }
    }
}

/// Storage backend types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageBackendType {
    RocksDB,
    Sled,
    Memory,
}

/// Write sync modes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SyncMode {
    None,
    Normal, 
    Full,
}

/// Compaction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionConfig {
    /// Enable automatic compaction
    pub auto_compaction: bool,
    
    /// Compaction trigger threshold
    pub trigger_threshold: f64,
    
    /// Maximum compaction threads
    pub max_threads: u32,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            auto_compaction: true,
            trigger_threshold: 0.8,
            max_threads: 4,
        }
    }
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Block cache size in bytes
    pub block_cache_size: usize,
    
    /// Row cache size in bytes  
    pub row_cache_size: usize,
    
    /// Enable compression
    pub enable_compression: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            block_cache_size: 256 * 1024 * 1024, // 256MB
            row_cache_size: 64 * 1024 * 1024,   // 64MB
            enable_compression: true,
        }
    }
}

/// Storage backend abstraction
enum StorageBackend {
    RocksDB(RocksDBBackend),
    Sled(SledBackend),
    Memory(MemoryBackend),
}

/// RocksDB backend implementation
struct RocksDBBackend {
    // db: rocksdb::DB,  // Temporarily using stub for emergency stabilization
    _stub: (),
}

/// Sled backend implementation  
struct SledBackend {
    db: sled::Db,
}

/// In-memory backend implementation
struct MemoryBackend {
    data: Arc<RwLock<std::collections::HashMap<String, Vec<u8>>>>,
}

/// Storage statistics
#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    pub total_keys: u64,
    pub total_size_bytes: u64,
    pub reads: u64,
    pub writes: u64,
    pub deletes: u64,
    pub compactions: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

impl StateStore {
    /// Create a new state store
    pub async fn new(config: &StorageConfig) -> Result<Self> {
        let backend = Self::create_backend(config).await?;
        
        Ok(Self {
            config: config.clone(),
            backend,
            stats: Arc::new(RwLock::new(StorageStats::default())),
        })
    }
    
    /// Start the storage engine
    pub async fn start(&self) -> Result<()> {
        info!("Starting storage engine with backend: {:?}", self.config.backend);
        
        // Perform any necessary initialization
        match &self.backend {
            StorageBackend::RocksDB(_) => {
                debug!("RocksDB backend initialized");
            }
            StorageBackend::Sled(_) => {
                debug!("Sled backend initialized");
            }
            StorageBackend::Memory(_) => {
                debug!("Memory backend initialized");
            }
        }
        
        info!("Storage engine started");
        Ok(())
    }
    
    /// Stop the storage engine
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping storage engine");
        
        // Perform cleanup based on backend
        match &self.backend {
            StorageBackend::RocksDB(_) => {
                // RocksDB cleanup is handled by Drop
            }
            StorageBackend::Sled(backend) => {
                if let Err(e) = backend.db.flush() {
                    warn!("Failed to flush Sled database: {}", e);
                }
            }
            StorageBackend::Memory(_) => {
                // No cleanup needed for memory backend
            }
        }
        
        info!("Storage engine stopped");
        Ok(())
    }
    
    /// Get a value by key
    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let result = match &self.backend {
            StorageBackend::RocksDB(_backend) => {
                // Stub implementation for emergency stabilization
                None  // RocksDB temporarily returns empty results
            }
            StorageBackend::Sled(backend) => {
                backend.db.get(key.as_bytes())
                    .map_err(|e| StateError::Storage { 
                        message: format!("Sled get failed: {}", e) 
                    })?
                    .map(|v| v.to_vec())
            }
            StorageBackend::Memory(backend) => {
                backend.data.read().await.get(key).cloned()
            }
        };
        
        // Update stats
        let mut stats = self.stats.write().await;
        stats.reads += 1;
        if result.is_some() {
            stats.cache_hits += 1;
        } else {
            stats.cache_misses += 1;
        }
        
        Ok(result)
    }
    
    /// Set a key-value pair
    pub async fn set(&self, key: &str, value: &[u8]) -> Result<()> {
        match &self.backend {
            StorageBackend::RocksDB(_backend) => {
                // Stub implementation for emergency stabilization - no-op
            }
            StorageBackend::Sled(backend) => {
                backend.db.insert(key.as_bytes(), value)
                    .map_err(|e| StateError::Storage { 
                        message: format!("Sled insert failed: {}", e) 
                    })?;
            }
            StorageBackend::Memory(backend) => {
                backend.data.write().await.insert(key.to_string(), value.to_vec());
            }
        }
        
        // Update stats
        let mut stats = self.stats.write().await;
        stats.writes += 1;
        stats.total_size_bytes += value.len() as u64;
        
        Ok(())
    }
    
    /// Delete a key
    pub async fn delete(&self, key: &str) -> Result<bool> {
        let existed = match &self.backend {
            StorageBackend::RocksDB(_backend) => {
                // Stub implementation for emergency stabilization
                false  // RocksDB temporarily returns false for delete operations
            }
            StorageBackend::Sled(backend) => {
                backend.db.remove(key.as_bytes())
                    .map_err(|e| StateError::Storage { 
                        message: format!("Sled remove failed: {}", e) 
                    })?
                    .is_some()
            }
            StorageBackend::Memory(backend) => {
                backend.data.write().await.remove(key).is_some()
            }
        };
        
        // Update stats
        if existed {
            let mut stats = self.stats.write().await;
            stats.deletes += 1;
        }
        
        Ok(existed)
    }
    
    /// List keys with prefix
    pub async fn list_keys(&self, prefix: &str, limit: Option<usize>) -> Result<Vec<String>> {
        let mut keys = Vec::new();
        let max_keys = limit.unwrap_or(usize::MAX);
        
        match &self.backend {
            StorageBackend::RocksDB(_backend) => {
                // Stub implementation for emergency stabilization - returns empty list
            }
            StorageBackend::Sled(backend) => {
                for item in backend.db.scan_prefix(prefix.as_bytes()).take(max_keys) {
                    let (key_bytes, _) = item.map_err(|e| StateError::Storage { 
                        message: format!("Sled scan failed: {}", e) 
                    })?;
                    
                    if let Ok(key_str) = String::from_utf8(key_bytes.to_vec()) {
                        keys.push(key_str);
                    }
                }
            }
            StorageBackend::Memory(backend) => {
                let data = backend.data.read().await;
                
                for key in data.keys() {
                    if key.starts_with(prefix) && keys.len() < max_keys {
                        keys.push(key.clone());
                    }
                }
            }
        }
        
        keys.sort();
        Ok(keys)
    }
    
    /// Get storage statistics
    pub async fn stats(&self) -> StorageStats {
        let mut stats = self.stats.read().await.clone();
        
        // Update total keys from backend
        match &self.backend {
            StorageBackend::RocksDB(_) => {
                // RocksDB doesn't have a direct key count method
                // This would require iterating through all keys
            }
            StorageBackend::Sled(backend) => {
                stats.total_keys = backend.db.len() as u64;
            }
            StorageBackend::Memory(backend) => {
                stats.total_keys = backend.data.read().await.len() as u64;
            }
        }
        
        stats
    }
    
    /// Create storage backend based on configuration
    async fn create_backend(config: &StorageConfig) -> Result<StorageBackend> {
        match config.backend {
            StorageBackendType::RocksDB => {
                let backend = Self::create_rocksdb_backend(config).await?;
                Ok(StorageBackend::RocksDB(backend))
            }
            StorageBackendType::Sled => {
                let backend = Self::create_sled_backend(config).await?;
                Ok(StorageBackend::Sled(backend))
            }
            StorageBackendType::Memory => {
                let backend = Self::create_memory_backend(config).await?;
                Ok(StorageBackend::Memory(backend))
            }
        }
    }
    
    /// Create RocksDB backend - stub implementation for emergency stabilization
    async fn create_rocksdb_backend(_config: &StorageConfig) -> Result<RocksDBBackend> {
        // Stub implementation - just return empty backend
        Ok(RocksDBBackend { _stub: () })
    }
    
    /// Create Sled backend
    async fn create_sled_backend(config: &StorageConfig) -> Result<SledBackend> {
        let db_path = Path::new(&config.data_dir).join("sled");
        
        let db = sled::Config::default()
            .path(db_path)
            .cache_capacity(config.cache.block_cache_size as u64)
            .flush_every_ms(if config.sync_mode == SyncMode::None { None } else { Some(1000) })
            .open()
            .map_err(|e| StateError::Storage { 
                message: format!("Failed to open Sled database: {}", e) 
            })?;
        
        Ok(SledBackend { db })
    }
    
    /// Create memory backend
    async fn create_memory_backend(_config: &StorageConfig) -> Result<MemoryBackend> {
        Ok(MemoryBackend {
            data: Arc::new(RwLock::new(std::collections::HashMap::new())),
        })
    }
}

/// Storage engine interface
pub trait StorageEngine: Send + Sync {
    /// Get a value by key
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;
    
    /// Set a key-value pair
    fn set(&self, key: &[u8], value: &[u8]) -> Result<()>;
    
    /// Delete a key
    fn delete(&self, key: &[u8]) -> Result<bool>;
    
    /// List keys with prefix
    fn list_keys(&self, prefix: &[u8], limit: Option<usize>) -> Result<Vec<Vec<u8>>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_memory_backend() {
        let mut config = StorageConfig::default();
        config.backend = StorageBackendType::Memory;
        
        let store = StateStore::new(&config).await.unwrap();
        store.start().await.unwrap();
        
        // Test basic operations
        assert!(store.get("key1").await.unwrap().is_none());
        
        store.set("key1", b"value1").await.unwrap();
        assert_eq!(store.get("key1").await.unwrap(), Some(b"value1".to_vec()));
        
        assert!(store.delete("key1").await.unwrap());
        assert!(store.get("key1").await.unwrap().is_none());
        
        store.stop().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_sled_backend() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = StorageConfig::default();
        config.backend = StorageBackendType::Sled;
        config.data_dir = temp_dir.path().to_string_lossy().to_string();
        
        let store = StateStore::new(&config).await.unwrap();
        store.start().await.unwrap();
        
        // Test basic operations
        store.set("key1", b"value1").await.unwrap();
        assert_eq!(store.get("key1").await.unwrap(), Some(b"value1".to_vec()));
        
        let keys = store.list_keys("key", Some(10)).await.unwrap();
        assert_eq!(keys, vec!["key1".to_string()]);
        
        store.stop().await.unwrap();
    }
    
    #[test]
    fn test_storage_config_serialization() {
        let config = StorageConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: StorageConfig = serde_json::from_str(&json).unwrap();
        
        assert_eq!(config.max_size_bytes, parsed.max_size_bytes);
    }
}