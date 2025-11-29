//! State management configuration
//! Emergency stub implementation for Phase 1 stabilization

use serde::{Serialize, Deserialize};

/// State management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateConfig {
    pub node_id: String,
    pub cluster_size: usize,
    pub data_directory: String,
    pub storage: StorageConfig,
    pub consensus: ConsensusConfig,
    pub sharding: ShardingConfig,
    pub transactions: TransactionConfig,
    pub encryption: EncryptionConfig,
    pub replication: ReplicationConfig,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub backend: String,
}

/// Consensus configuration  
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    pub algorithm: String,
}

/// Sharding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardingConfig {
    pub enabled: bool,
}

/// Transaction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionConfig {
    pub isolation_level: String,
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub algorithm: String,
}

/// Replication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationConfig {
    pub factor: usize,
}

impl Default for StateConfig {
    fn default() -> Self {
        Self {
            node_id: "node-1".to_string(),
            cluster_size: 3,
            data_directory: "./data/state".to_string(),
            storage: StorageConfig::default(),
            consensus: ConsensusConfig::default(),
            sharding: ShardingConfig::default(),
            transactions: TransactionConfig::default(),
            encryption: EncryptionConfig::default(),
            replication: ReplicationConfig::default(),
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: "sled".to_string(),
        }
    }
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            algorithm: "raft".to_string(),
        }
    }
}

impl Default for ShardingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
        }
    }
}

impl Default for TransactionConfig {
    fn default() -> Self {
        Self {
            isolation_level: "serializable".to_string(),
        }
    }
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            algorithm: "aes256".to_string(),
        }
    }
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            factor: 3,
        }
    }
}