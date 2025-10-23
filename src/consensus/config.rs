//! Configuration structures for the consensus system

use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::path::PathBuf;

/// Main consensus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Raft protocol configuration
    pub raft: RaftConfig,
    /// Byzantine fault tolerance configuration
    pub byzantine: ByzantineConfig,
    /// Transaction manager configuration
    pub transaction: TransactionConfig,
    /// Storage engine configuration
    pub storage: StorageConfig,
    /// Sharding configuration
    pub sharding: ShardingConfig,
    /// Performance tuning configuration
    pub performance: PerformanceConfig,
}

/// Raft consensus algorithm configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftConfig {
    /// Election timeout range in milliseconds [min, max]
    pub election_timeout_ms: [u64; 2],
    /// Heartbeat interval in milliseconds
    pub heartbeat_interval_ms: u64,
    /// Maximum log entries per AppendEntries request
    pub max_log_entries_per_request: usize,
    /// Log batch size for batching operations
    pub log_batch_size: usize,
    /// Snapshot threshold - create snapshot after this many log entries
    pub snapshot_threshold: u64,
    /// Enable pre-vote optimization to reduce disruptions
    pub enable_pre_vote: bool,
    /// Maximum time to wait for vote responses
    pub vote_timeout_ms: u64,
    /// Maximum time to wait for append entries responses
    pub append_timeout_ms: u64,
}

/// Byzantine fault tolerance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByzantineConfig {
    /// Enable Byzantine fault tolerance
    pub enabled: bool,
    /// Reputation score threshold below which node is considered Byzantine
    pub detection_threshold: f64,
    /// How long to retain Byzantine evidence (hours)
    pub evidence_retention_hours: u64,
    /// Reputation decay factor (0.0-1.0)
    pub reputation_decay_factor: f64,
    /// Maximum ratio of Byzantine nodes the system can tolerate
    pub max_byzantine_ratio: f64,
    /// Enable automatic node quarantine for Byzantine behavior
    pub enable_quarantine: bool,
    /// Minimum evidence required before quarantining a node
    pub quarantine_evidence_threshold: usize,
}

/// Transaction manager configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionConfig {
    /// Default isolation level
    pub default_isolation_level: IsolationLevel,
    /// Transaction timeout in seconds
    pub timeout_seconds: u64,
    /// Maximum retry attempts for failed transactions
    pub max_retry_attempts: u32,
    /// Deadlock detection interval in milliseconds
    pub deadlock_detection_interval_ms: u64,
    /// Two-phase commit timeout for distributed transactions
    pub two_phase_commit_timeout_seconds: u64,
    /// Coordinator lease duration in seconds
    pub coordinator_lease_seconds: u64,
    /// Participant heartbeat interval for distributed transactions
    pub participant_heartbeat_interval_ms: u64,
    /// Maximum participants per distributed transaction
    pub max_participants_per_transaction: usize,
}

/// MVCC storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Data directory path
    pub data_dir: PathBuf,
    /// Maximum versions per key before garbage collection
    pub max_versions_per_key: usize,
    /// Garbage collection interval in seconds
    pub gc_interval_seconds: u64,
    /// Garbage collection watermark lag in seconds
    pub gc_watermark_lag_seconds: u64,
    /// Enable version compression
    pub version_compression: bool,
    /// RocksDB configuration
    pub rocksdb: RocksDBConfig,
    /// Memory table size limit in MB
    pub memtable_size_mb: usize,
    /// Write buffer size in MB
    pub write_buffer_size_mb: usize,
}

/// RocksDB-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RocksDBConfig {
    /// Maximum number of open files
    pub max_open_files: i32,
    /// Enable statistics
    pub enable_statistics: bool,
    /// Compression algorithm
    pub compression_type: CompressionType,
    /// Block cache size in MB
    pub block_cache_size_mb: usize,
    /// Enable bloom filter
    pub enable_bloom_filter: bool,
    /// Bloom filter bits per key
    pub bloom_filter_bits_per_key: i32,
}

/// Compression algorithms supported by RocksDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionType {
    None,
    Snappy,
    Zlib,
    Bz2,
    Lz4,
    Lz4Hc,
    Zstd,
}

/// Sharding system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardingConfig {
    /// Initial number of shards when cluster starts
    pub initial_shard_count: usize,
    /// Maximum number of shards allowed
    pub max_shard_count: usize,
    /// Replication factor for each shard
    pub replication_factor: usize,
    /// Shard size threshold in MB before splitting
    pub split_threshold_mb: u64,
    /// Shard size threshold in MB below which shards are merged
    pub merge_threshold_mb: u64,
    /// Virtual nodes per physical node for consistent hashing
    pub virtual_nodes_per_physical_node: usize,
    /// Hash function for consistent hashing
    pub hash_function: HashFunction,
    /// Load imbalance threshold that triggers rebalancing (0.0-1.0)
    pub rebalance_threshold: f64,
    /// Hot shard detection configuration
    pub hot_shard_detection: HotShardConfig,
}

/// Hash functions for consistent hashing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HashFunction {
    Sha256,
    Blake3,
    Xxhash,
}

/// Hot shard detection and mitigation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotShardConfig {
    /// Enable hot shard detection
    pub enabled: bool,
    /// CPU utilization threshold percentage
    pub cpu_threshold: f64,
    /// Memory utilization threshold percentage
    pub memory_threshold: f64,
    /// Request rate threshold (requests per second)
    pub request_rate_threshold: u64,
    /// Detection window in minutes
    pub detection_window_minutes: u64,
    /// Mitigation strategies to apply
    pub mitigation_strategies: Vec<MitigationStrategy>,
}

/// Hot shard mitigation strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MitigationStrategy {
    Split,
    Replicate,
    Cache,
}

/// Performance tuning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable asynchronous log replication
    pub async_replication: bool,
    /// Pipeline length for batching operations
    pub pipeline_length: usize,
    /// Enable compression for network messages
    pub compression_enabled: bool,
    /// Compression algorithm for network messages
    pub compression_algorithm: NetworkCompressionType,
    /// Network buffer size
    pub network_buffer_size: usize,
    /// Number of worker threads for consensus operations
    pub consensus_worker_threads: usize,
    /// Enable adaptive timeouts based on network conditions
    pub adaptive_timeouts: bool,
}

/// Network compression algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkCompressionType {
    None,
    Gzip,
    Zstd,
    Lz4,
}

/// Transaction isolation levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            raft: RaftConfig::default(),
            byzantine: ByzantineConfig::default(),
            transaction: TransactionConfig::default(),
            storage: StorageConfig::default(),
            sharding: ShardingConfig::default(),
            performance: PerformanceConfig::default(),
        }
    }
}

impl Default for RaftConfig {
    fn default() -> Self {
        Self {
            election_timeout_ms: [1000, 2000],
            heartbeat_interval_ms: 100,
            max_log_entries_per_request: 10000,
            log_batch_size: 1000,
            snapshot_threshold: 100000,
            enable_pre_vote: true,
            vote_timeout_ms: 5000,
            append_timeout_ms: 1000,
        }
    }
}

impl Default for ByzantineConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            detection_threshold: 0.7,
            evidence_retention_hours: 168, // 1 week
            reputation_decay_factor: 0.99,
            max_byzantine_ratio: 0.33,
            enable_quarantine: true,
            quarantine_evidence_threshold: 3,
        }
    }
}

impl Default for TransactionConfig {
    fn default() -> Self {
        Self {
            default_isolation_level: IsolationLevel::Serializable,
            timeout_seconds: 30,
            max_retry_attempts: 3,
            deadlock_detection_interval_ms: 100,
            two_phase_commit_timeout_seconds: 60,
            coordinator_lease_seconds: 300,
            participant_heartbeat_interval_ms: 1000,
            max_participants_per_transaction: 1000,
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./hypermesh-data"),
            max_versions_per_key: 100,
            gc_interval_seconds: 300,
            gc_watermark_lag_seconds: 3600,
            version_compression: true,
            rocksdb: RocksDBConfig::default(),
            memtable_size_mb: 64,
            write_buffer_size_mb: 32,
        }
    }
}

impl Default for RocksDBConfig {
    fn default() -> Self {
        Self {
            max_open_files: 1000,
            enable_statistics: true,
            compression_type: CompressionType::Zstd,
            block_cache_size_mb: 256,
            enable_bloom_filter: true,
            bloom_filter_bits_per_key: 10,
        }
    }
}

impl Default for ShardingConfig {
    fn default() -> Self {
        Self {
            initial_shard_count: 16,
            max_shard_count: 100000,
            replication_factor: 3,
            split_threshold_mb: 100,
            merge_threshold_mb: 10,
            virtual_nodes_per_physical_node: 150,
            hash_function: HashFunction::Blake3,
            rebalance_threshold: 0.1,
            hot_shard_detection: HotShardConfig::default(),
        }
    }
}

impl Default for HotShardConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cpu_threshold: 80.0,
            memory_threshold: 85.0,
            request_rate_threshold: 10000,
            detection_window_minutes: 5,
            mitigation_strategies: vec![
                MitigationStrategy::Split,
                MitigationStrategy::Replicate,
                MitigationStrategy::Cache,
            ],
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            async_replication: true,
            pipeline_length: 10,
            compression_enabled: true,
            compression_algorithm: NetworkCompressionType::Zstd,
            network_buffer_size: 65536, // 64KB
            consensus_worker_threads: 4,
            adaptive_timeouts: true,
        }
    }
}

impl ConsensusConfig {
    /// Load configuration from a file
    pub fn from_file(path: &std::path::Path) -> Result<Self, config::ConfigError> {
        let settings = config::Config::builder()
            .add_source(config::File::from(path))
            .build()?;
        settings.try_deserialize()
    }
    
    /// Save configuration to a file
    pub fn to_file(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let yaml = serde_yaml::to_string(self)?;
        std::fs::write(path, yaml)?;
        Ok(())
    }
    
    /// Validate the configuration for consistency
    pub fn validate(&self) -> Result<(), String> {
        // Validate election timeout range
        if self.raft.election_timeout_ms[0] >= self.raft.election_timeout_ms[1] {
            return Err("Election timeout minimum must be less than maximum".to_string());
        }
        
        // Validate heartbeat interval
        if self.raft.heartbeat_interval_ms >= self.raft.election_timeout_ms[0] {
            return Err("Heartbeat interval must be less than election timeout minimum".to_string());
        }
        
        // Validate Byzantine ratio
        if self.byzantine.max_byzantine_ratio >= 0.5 {
            return Err("Byzantine ratio must be less than 0.5 for safety".to_string());
        }
        
        // Validate replication factor
        if self.sharding.replication_factor < 1 {
            return Err("Replication factor must be at least 1".to_string());
        }
        
        // Validate shard thresholds
        if self.sharding.split_threshold_mb <= self.sharding.merge_threshold_mb {
            return Err("Split threshold must be greater than merge threshold".to_string());
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config_validation() {
        let config = ConsensusConfig::default();
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_invalid_election_timeout() {
        let mut config = ConsensusConfig::default();
        config.raft.election_timeout_ms = [2000, 1000]; // Invalid: min > max
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_invalid_byzantine_ratio() {
        let mut config = ConsensusConfig::default();
        config.byzantine.max_byzantine_ratio = 0.6; // Invalid: > 0.5
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_config_serialization() {
        let config = ConsensusConfig::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let deserialized: ConsensusConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(config.raft.heartbeat_interval_ms, deserialized.raft.heartbeat_interval_ms);
    }
}