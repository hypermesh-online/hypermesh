//! Nexus State - Distributed state management with Byzantine fault tolerance
//! 
//! This module provides a replacement for etcd with the following features:
//! - Raft consensus with Byzantine fault tolerance extensions
//! - Encrypted state replication with forward secrecy
//! - Automatic sharding and rebalancing
//! - ACID transactions with serializable isolation
//! - Real-time subscriptions to state changes

pub mod consensus;
pub mod byzantine;
pub mod storage;
pub mod replication;
pub mod sharding;
pub mod transactions;
pub mod subscriptions;
pub mod encryption;
pub mod config;
pub mod error;

pub use consensus::{ConsensusEngine, ConsensusState, Proposal, ByzantineStatus};
pub use byzantine::{ByzantineCoordinator, ByzantineConfig, OverallByzantineStatus};
pub use storage::{StateStore, StorageEngine, StorageConfig};
pub use replication::{ReplicationManager, ReplicationState};
pub use sharding::{ShardManager, ShardConfig, ShardKey};
pub use transactions::{Transaction, TransactionManager, IsolationLevel};
pub use subscriptions::{SubscriptionManager, StateChange, WatchHandle};
pub use encryption::{EncryptionManager, StateEncryption};
pub use config::StateConfig;
pub use error::{StateError, Result};

use nexus_shared::{NodeId, ResourceId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::{RwLock, broadcast};

/// Distributed state manager
pub struct StateManager {
    config: StateConfig,
    node_id: NodeId,
    
    // Core components
    consensus: Arc<ConsensusEngine>,
    storage: Arc<StateStore>,
    replication: Arc<ReplicationManager>,
    sharding: Arc<ShardManager>,
    transactions: Arc<TransactionManager>,
    subscriptions: Arc<SubscriptionManager>,
    encryption: Arc<EncryptionManager>,
    
    // State
    cluster_members: Arc<RwLock<HashMap<NodeId, ClusterMember>>>,
    leader_node: Arc<RwLock<Option<NodeId>>>,
    
    // Events
    state_change_sender: broadcast::Sender<StateChange>,
}

impl StateManager {
    /// Create a new state manager
    pub async fn new(config: StateConfig, node_id: NodeId) -> Result<Self> {
        // Convert generic config to consensus-specific config
        let consensus_cfg = consensus::ConsensusConfig::default();
        let consensus = Arc::new(ConsensusEngine::new(&consensus_cfg, node_id).await?);
        // Convert generic config to storage-specific config
        let storage_cfg = storage::StorageConfig::default();
        let storage = Arc::new(StateStore::new(&storage_cfg).await?);
        let replication = Arc::new(ReplicationManager::new(&config.replication, node_id)?);
        let sharding = Arc::new(ShardManager::new(&config.sharding)?);
        let transactions = Arc::new(TransactionManager::new(&config.transactions)?);
        let subscriptions = Arc::new(SubscriptionManager::new());
        let encryption = Arc::new(EncryptionManager::from_config(&config.encryption));
        
        let (state_change_sender, _) = broadcast::channel(10000);
        
        Ok(Self {
            config,
            node_id,
            consensus,
            storage,
            replication,
            sharding,
            transactions,
            subscriptions,
            encryption,
            cluster_members: Arc::new(RwLock::new(HashMap::new())),
            leader_node: Arc::new(RwLock::new(None)),
            state_change_sender,
        })
    }
    
    /// Start the state manager
    pub async fn start(&self) -> Result<()> {
        tracing::info!("Starting state manager for node {}", self.node_id);
        
        // Start storage engine
        self.storage.start().await?;
        
        // Start consensus engine
        self.consensus.start().await?;
        
        // Start replication
        self.replication.start().await?;
        
        // Start subscription manager
        self.subscriptions.start().await?;
        
        tracing::info!("State manager started successfully");
        Ok(())
    }
    
    /// Stop the state manager
    pub async fn stop(&self) -> Result<()> {
        tracing::info!("Stopping state manager");
        
        self.subscriptions.stop().await?;
        self.replication.stop().await?;
        self.consensus.stop().await?;
        self.storage.stop().await?;
        
        tracing::info!("State manager stopped");
        Ok(())
    }
    
    /// Join a cluster
    pub async fn join_cluster(&self, bootstrap_nodes: Vec<NodeId>) -> Result<()> {
        tracing::info!("Joining cluster with bootstrap nodes: {:?}", bootstrap_nodes);
        
        // Add bootstrap nodes as cluster members
        let mut members = self.cluster_members.write().await;
        for node_id in bootstrap_nodes {
            members.insert(node_id, ClusterMember {
                node_id,
                status: MemberStatus::Active,
                joined_at: SystemTime::now(),
                last_seen: SystemTime::now(),
            });
        }
        
        // Start consensus with cluster members
        self.consensus.join_cluster(members.keys().cloned().collect()).await?;
        
        Ok(())
    }
    
    /// Get a value from the state store
    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let _shard_key = self.sharding.get_shard_key(key);  // Removed ? since it doesn't return Result
        let encrypted_key = self.encryption.encrypt_key(key).await?;
        
        let encrypted_value = self.storage.get(&encrypted_key).await?;
        
        if let Some(encrypted_data) = encrypted_value {
            let decrypted_value = self.encryption.decrypt_data(&encrypted_data).await?;
            Ok(Some(decrypted_value))
        } else {
            Ok(None)
        }
    }
    
    /// Set a value in the state store
    pub async fn set(&self, key: &str, value: &[u8]) -> Result<()> {
        let encrypted_key = self.encryption.encrypt_key(key).await?;
        let encrypted_value = self.encryption.encrypt_data(value).await?;
        
        // Create proposal for consensus
        let proposal = Proposal::Set {
            key: encrypted_key,
            value: encrypted_value,
        };
        
        // Submit to consensus
        self.consensus.propose(proposal).await?;
        
        Ok(())
    }
    
    /// Delete a value from the state store
    pub async fn delete(&self, key: &str) -> Result<bool> {
        let encrypted_key = self.encryption.encrypt_key(key).await?;
        
        // Create proposal for consensus
        let proposal = Proposal::Delete {
            key: encrypted_key,
        };
        
        // Submit to consensus
        self.consensus.propose(proposal).await?;
        
        Ok(true) // TODO: Return actual result from consensus
    }
    
    /// List keys with prefix
    pub async fn list(&self, prefix: &str, limit: Option<usize>) -> Result<Vec<String>> {
        let encrypted_prefix = self.encryption.encrypt_key(prefix).await?.to_string();
        let encrypted_keys = self.storage.list_keys(&encrypted_prefix, limit).await?;
        
        let mut keys = Vec::new();
        for encrypted_key in encrypted_keys {
            let decrypted_key = self.encryption.decrypt_key(&encrypted_key).await?;
            keys.push(decrypted_key);
        }
        
        Ok(keys)
    }
    
    /// Start a transaction
    pub async fn begin_transaction(&self) -> Result<TransactionHandle> {
        let transaction = self.transactions.begin().await?;
        Ok(TransactionHandle {
            transaction,
            state_manager: self,
        })
    }
    
    /// Watch for changes to a key or prefix
    pub async fn watch(&self, key_prefix: &str) -> Result<WatchHandle> {
        let encrypted_prefix = self.encryption.encrypt_key(key_prefix).await?;
        self.subscriptions.watch(&encrypted_prefix).await
    }
    
    /// Get cluster status
    pub async fn cluster_status(&self) -> ClusterStatus {
        let members = self.cluster_members.read().await;
        let leader = *self.leader_node.read().await;
        
        ClusterStatus {
            node_id: self.node_id,
            leader_node: leader,
            member_count: members.len(),
            members: members.values().cloned().collect(),
            consensus_state: self.consensus.state().await,
        }
    }
    
    /// Get node statistics
    pub async fn stats(&self) -> StateManagerStats {
        StateManagerStats {
            node_id: self.node_id,
            storage_stats: self.storage.stats().await,
            consensus_stats: self.consensus.stats().await,
            replication_stats: self.replication.stats().await,
        }
    }
}

/// Cluster member information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMember {
    pub node_id: NodeId,
    pub status: MemberStatus,
    pub joined_at: SystemTime,
    pub last_seen: SystemTime,
}

/// Member status in cluster
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MemberStatus {
    Active,
    Inactive,
    Suspect,
    Failed,
}

/// Cluster status information
#[derive(Debug, Clone)]
pub struct ClusterStatus {
    pub node_id: NodeId,
    pub leader_node: Option<NodeId>,
    pub member_count: usize,
    pub members: Vec<ClusterMember>,
    pub consensus_state: ConsensusState,
}

/// Transaction handle for managing transactions
pub struct TransactionHandle<'a> {
    transaction: Transaction,
    state_manager: &'a StateManager,
}

impl<'a> TransactionHandle<'a> {
    /// Get value within transaction
    pub async fn get(&mut self, key: &str) -> Result<Option<Vec<u8>>> {
        let encrypted_key = self.state_manager.encryption.encrypt_key(key).await?;
        let encrypted_value = self.transaction.get(&encrypted_key).await?;
        
        if let Some(encrypted_data) = encrypted_value {
            let decrypted_value = self.state_manager.encryption.decrypt_data(&encrypted_data).await?;
            Ok(Some(decrypted_value))
        } else {
            Ok(None)
        }
    }
    
    /// Set value within transaction
    pub async fn set(&mut self, key: &str, value: &[u8]) -> Result<()> {
        let encrypted_key = self.state_manager.encryption.encrypt_key(key).await?;
        let encrypted_value = self.state_manager.encryption.encrypt_data(value).await?;
        
        self.transaction.set(encrypted_key, encrypted_value).await
    }
    
    /// Delete value within transaction
    pub async fn delete(&mut self, key: &str) -> Result<bool> {
        let encrypted_key = self.state_manager.encryption.encrypt_key(key).await?;
        self.transaction.delete(&encrypted_key).await
    }
    
    /// Commit transaction
    pub async fn commit(self) -> Result<()> {
        self.state_manager.transactions.commit(self.transaction).await
    }
    
    /// Rollback transaction
    pub async fn rollback(self) -> Result<()> {
        self.state_manager.transactions.rollback(self.transaction).await
    }
}

/// State manager statistics
#[derive(Debug, Clone)]
pub struct StateManagerStats {
    pub node_id: NodeId,
    pub storage_stats: storage::StorageStats,
    pub consensus_stats: consensus::ConsensusStats,
    pub replication_stats: replication::ReplicationStats,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_state_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = StateConfig::default();
        config.storage.data_dir = temp_dir.path().to_string_lossy().to_string();
        
        let node_id = NodeId::random();
        let state_manager = StateManager::new(config, node_id).await;
        assert!(state_manager.is_ok());
    }
    
    #[tokio::test]
    async fn test_cluster_status() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = StateConfig::default();
        config.storage.data_dir = temp_dir.path().to_string_lossy().to_string();
        
        let node_id = NodeId::random();
        let state_manager = StateManager::new(config, node_id).await.unwrap();
        
        let status = state_manager.cluster_status().await;
        assert_eq!(status.node_id, node_id);
        assert_eq!(status.member_count, 0);
        assert!(status.leader_node.is_none());
    }
}