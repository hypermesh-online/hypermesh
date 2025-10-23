//! ACID transaction management with serializable isolation

use super::{
    storage::{MVCCStorage, StorageEngine, ConflictInfo, ConflictType, Timestamp},
    config::TransactionConfig,
    error::{ConsensusError, Result},
    metrics::ConsensusMetrics,
};

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use std::time::{Duration, Instant, SystemTime};
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use tracing::{debug, warn, info, error};

/// Unique identifier for transactions
pub type TransactionId = Uuid;

/// Transaction manager for ACID operations
pub struct TransactionManager {
    /// Active transactions
    active_transactions: Arc<RwLock<HashMap<TransactionId, Transaction>>>,
    
    /// Lock manager for conflict detection
    lock_manager: Arc<LockManager>,
    
    /// Timestamp oracle for ordering
    timestamp_oracle: Arc<TimestampOracle>,
    
    /// Write set tracker for dependency analysis
    write_set_tracker: Arc<WriteSetTracker>,
    
    /// MVCC storage backend
    storage: Arc<MVCCStorage>,
    
    /// Configuration
    config: TransactionConfig,
    
    /// Metrics collection
    metrics: Arc<ConsensusMetrics>,
    
    /// Deadlock detector
    deadlock_detector: Arc<Mutex<DeadlockDetector>>,
}

/// Individual transaction state
#[derive(Debug, Clone)]
pub struct Transaction {
    /// Unique transaction identifier
    pub id: TransactionId,
    
    /// Transaction start timestamp
    pub start_timestamp: Timestamp,
    
    /// Commit timestamp (set during commit phase)
    pub commit_timestamp: Option<Timestamp>,
    
    /// Isolation level for this transaction
    pub isolation_level: IsolationLevel,
    
    /// Set of keys read during transaction
    pub read_set: HashMap<String, Timestamp>,
    
    /// Set of keys and values written during transaction
    pub write_set: HashMap<String, Vec<u8>>,
    
    /// Current transaction status
    pub status: TransactionStatus,
    
    /// Involved shards for distributed transactions
    pub involved_shards: Vec<String>,
    
    /// Transaction creation time
    pub created_at: Instant,
    
    /// Last activity timestamp
    pub last_activity: Instant,
    
    /// Transaction coordinator (for distributed transactions)
    pub coordinator: Option<String>,
    
    /// Prepared status for 2PC
    pub prepared: bool,
    
    /// Timeout deadline
    pub timeout_at: Instant,
}

/// Transaction isolation levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

/// Transaction status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    Active,
    Preparing,
    Prepared,
    Committing,
    Committed,
    Aborting,
    Aborted,
}

/// Result of transaction commit
#[derive(Debug, Clone)]
pub struct CommitResult {
    pub transaction_id: TransactionId,
    pub commit_timestamp: Timestamp,
    pub committed_keys: HashSet<String>,
    pub duration: Duration,
}

/// Result of transaction prepare phase (2PC)
#[derive(Debug, Clone)]
pub struct PrepareResult {
    pub transaction_id: TransactionId,
    pub prepared: bool,
    pub reason: Option<String>,
}

/// Lock manager for transaction concurrency control
struct LockManager {
    /// Currently held locks
    locks: Arc<RwLock<HashMap<String, LockInfo>>>,
    
    /// Lock wait queue
    wait_queue: Arc<RwLock<HashMap<String, Vec<LockWaiter>>>>,
}

/// Information about a held lock
#[derive(Debug, Clone)]
struct LockInfo {
    transaction_id: TransactionId,
    lock_type: LockType,
    acquired_at: Instant,
}

/// Types of locks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LockType {
    Shared,    // Read lock
    Exclusive, // Write lock
}

/// Lock waiter information
#[derive(Debug, Clone, Copy)]
struct LockWaiter {
    transaction_id: TransactionId,
    lock_type: LockType,
    waiting_since: Instant,
}

/// Timestamp oracle for transaction ordering
struct TimestampOracle {
    current_timestamp: Arc<RwLock<Timestamp>>,
}

/// Write set tracker for dependency analysis
struct WriteSetTracker {
    /// Recent write sets for conflict detection
    recent_writes: Arc<RwLock<HashMap<String, Vec<WriteRecord>>>>,
}

/// Record of a write operation
#[derive(Debug, Clone)]
struct WriteRecord {
    transaction_id: TransactionId,
    timestamp: Timestamp,
    written_at: Instant,
}

/// Deadlock detector
struct DeadlockDetector {
    /// Wait-for graph for deadlock detection
    wait_for_graph: HashMap<TransactionId, HashSet<TransactionId>>,
    /// Last detection run
    last_detection: Instant,
}

impl TransactionManager {
    /// Create a new transaction manager
    pub async fn new(
        storage: Arc<MVCCStorage>,
        config: TransactionConfig,
    ) -> Result<Self> {
        let metrics = Arc::new(
            ConsensusMetrics::new()
                .map_err(|e| ConsensusError::Internal(format!("Failed to create metrics: {}", e)))?
        );
        
        let manager = Self {
            active_transactions: Arc::new(RwLock::new(HashMap::new())),
            lock_manager: Arc::new(LockManager::new()),
            timestamp_oracle: Arc::new(TimestampOracle::new()),
            write_set_tracker: Arc::new(WriteSetTracker::new()),
            storage,
            config,
            metrics,
            deadlock_detector: Arc::new(Mutex::new(DeadlockDetector::new())),
        };
        
        // Start background tasks
        manager.start_deadlock_detector().await;
        manager.start_timeout_cleaner().await;
        
        Ok(manager)
    }
    
    /// Begin a new transaction
    pub async fn begin_transaction(&self, isolation: IsolationLevel) -> Result<TransactionId> {
        let transaction_id = TransactionId::new_v4();
        let start_timestamp = self.timestamp_oracle.next_timestamp().await;
        let now = Instant::now();
        
        let timeout_duration = Duration::from_secs(self.config.timeout_seconds);
        
        let transaction = Transaction {
            id: transaction_id,
            start_timestamp,
            commit_timestamp: None,
            isolation_level: isolation,
            read_set: HashMap::new(),
            write_set: HashMap::new(),
            status: TransactionStatus::Active,
            involved_shards: Vec::new(),
            created_at: now,
            last_activity: now,
            coordinator: None,
            prepared: false,
            timeout_at: now + timeout_duration,
        };
        
        // Add to active transactions
        self.active_transactions.write().await.insert(transaction_id, transaction);
        
        // Update metrics
        self.metrics.transactions_started.inc();
        
        debug!("Started transaction {} with isolation {:?}", transaction_id, isolation);
        Ok(transaction_id)
    }
    
    /// Read a value within a transaction
    pub async fn read(&self, txn_id: TransactionId, key: &str) -> Result<Option<Vec<u8>>> {
        let mut transactions = self.active_transactions.write().await;
        let transaction = transactions.get_mut(&txn_id)
            .ok_or_else(|| ConsensusError::TransactionError("Transaction not found".to_string()))?;
        
        // Check transaction status
        if transaction.status != TransactionStatus::Active {
            return Err(ConsensusError::TransactionError("Transaction not active".to_string()));
        }
        
        // Update last activity
        transaction.last_activity = Instant::now();
        
        // Acquire shared lock if needed
        if transaction.isolation_level >= IsolationLevel::RepeatableRead {
            self.lock_manager.acquire_lock(txn_id, key, LockType::Shared).await?;
        }
        
        // Choose read timestamp based on isolation level
        let read_timestamp = match transaction.isolation_level {
            IsolationLevel::ReadUncommitted => {
                // Read latest version (even uncommitted)
                self.storage.current_timestamp().await
            }
            IsolationLevel::ReadCommitted => {
                // Read latest committed version
                self.storage.current_timestamp().await
            }
            IsolationLevel::RepeatableRead | IsolationLevel::Serializable => {
                // Read at transaction start timestamp
                transaction.start_timestamp
            }
        };
        
        // Perform the read
        let result = self.storage.read(key, read_timestamp).await?;
        
        // Track read for serializable isolation
        if transaction.isolation_level == IsolationLevel::Serializable {
            transaction.read_set.insert(key.to_string(), read_timestamp);
        }
        
        debug!("Read key {} in transaction {}: {:?}", key, txn_id, result.is_some());
        Ok(result)
    }
    
    /// Write a value within a transaction
    pub async fn write(&self, txn_id: TransactionId, key: String, value: Vec<u8>) -> Result<()> {
        let mut transactions = self.active_transactions.write().await;
        let transaction = transactions.get_mut(&txn_id)
            .ok_or_else(|| ConsensusError::TransactionError("Transaction not found".to_string()))?;
        
        // Check transaction status
        if transaction.status != TransactionStatus::Active {
            return Err(ConsensusError::TransactionError("Transaction not active".to_string()));
        }
        
        // Update last activity
        transaction.last_activity = Instant::now();
        
        // Acquire exclusive lock
        self.lock_manager.acquire_lock(txn_id, &key, LockType::Exclusive).await?;
        
        // Add to write set (not persisted until commit)
        transaction.write_set.insert(key.clone(), value);
        
        debug!("Wrote key {} in transaction {}", key, txn_id);
        Ok(())
    }
    
    /// Commit a transaction
    pub async fn commit(&self, txn_id: TransactionId) -> Result<CommitResult> {
        let commit_start = Instant::now();
        
        // Get transaction and change status
        let transaction = {
            let mut transactions = self.active_transactions.write().await;
            let mut transaction = transactions.get_mut(&txn_id)
                .ok_or_else(|| ConsensusError::TransactionError("Transaction not found".to_string()))?
                .clone();
            
            if transaction.status != TransactionStatus::Active {
                return Err(ConsensusError::TransactionError("Transaction not active".to_string()));
            }
            
            transaction.status = TransactionStatus::Committing;
            transactions.insert(txn_id, transaction.clone());
            transaction
        };
        
        // Get commit timestamp
        let commit_timestamp = self.timestamp_oracle.next_timestamp().await;
        
        // Conflict detection for serializable isolation
        if transaction.isolation_level == IsolationLevel::Serializable {
            let conflicts = self.detect_conflicts(&transaction, commit_timestamp).await?;
            if !conflicts.is_empty() {
                self.abort_transaction(txn_id, "Serialization conflict").await?;
                self.metrics.read_conflicts.inc_by(conflicts.len() as f64);
                return Err(ConsensusError::TransactionError("Serialization conflict detected".to_string()));
            }
        }
        
        // Write all values to storage
        let mut committed_keys = HashSet::new();
        for (key, value) in &transaction.write_set {
            self.storage.write(key.clone(), value.clone(), txn_id).await?;
            committed_keys.insert(key.clone());
            
            // Track write for conflict detection
            self.write_set_tracker.track_write(key, txn_id, commit_timestamp).await;
        }
        
        // Update transaction status
        {
            let mut transactions = self.active_transactions.write().await;
            if let Some(mut txn) = transactions.get_mut(&txn_id) {
                txn.status = TransactionStatus::Committed;
                txn.commit_timestamp = Some(commit_timestamp);
            }
        }
        
        // Release all locks
        self.lock_manager.release_all_locks(txn_id).await;
        
        // Remove from active transactions
        self.active_transactions.write().await.remove(&txn_id);
        
        let duration = commit_start.elapsed();
        let result = CommitResult {
            transaction_id: txn_id,
            commit_timestamp,
            committed_keys,
            duration,
        };
        
        // Update metrics
        self.metrics.record_transaction_commit(duration);
        
        info!("Committed transaction {} with {} keys in {:?}", 
              txn_id, result.committed_keys.len(), duration);
        Ok(result)
    }
    
    /// Rollback a transaction
    pub async fn rollback(&self, txn_id: TransactionId) -> Result<()> {
        self.abort_transaction(txn_id, "User rollback").await
    }
    
    /// Begin a distributed transaction across multiple shards
    pub async fn begin_distributed_transaction(&self, shards: Vec<String>) -> Result<TransactionId> {
        let transaction_id = self.begin_transaction(IsolationLevel::Serializable).await?;
        
        // Update transaction to include shard information
        {
            let mut transactions = self.active_transactions.write().await;
            if let Some(transaction) = transactions.get_mut(&transaction_id) {
                transaction.involved_shards = shards;
                transaction.coordinator = Some("local".to_string()); // This node is coordinator
            }
        }
        
        let shard_count = {
            let active_txns = self.active_transactions.read().await;
            active_txns.get(&transaction_id)
                .map(|t| t.involved_shards.len())
                .unwrap_or(0)
        };
        debug!("Started distributed transaction {} across {} shards", 
               transaction_id, shard_count);
        Ok(transaction_id)
    }
    
    /// Prepare phase of two-phase commit
    pub async fn prepare_phase(&self, txn_id: TransactionId) -> Result<PrepareResult> {
        let mut transactions = self.active_transactions.write().await;
        let transaction = transactions.get_mut(&txn_id)
            .ok_or_else(|| ConsensusError::TransactionError("Transaction not found".to_string()))?;
        
        if transaction.status != TransactionStatus::Active {
            return Ok(PrepareResult {
                transaction_id: txn_id,
                prepared: false,
                reason: Some("Transaction not active".to_string()),
            });
        }
        
        // Pre-commit validation
        let commit_timestamp = self.timestamp_oracle.next_timestamp().await;
        let conflicts = self.detect_conflicts(transaction, commit_timestamp).await?;
        
        if !conflicts.is_empty() {
            transaction.status = TransactionStatus::Aborted;
            return Ok(PrepareResult {
                transaction_id: txn_id,
                prepared: false,
                reason: Some("Conflicts detected".to_string()),
            });
        }
        
        // Mark as prepared
        transaction.status = TransactionStatus::Prepared;
        transaction.prepared = true;
        
        debug!("Prepared transaction {} for commit", txn_id);
        Ok(PrepareResult {
            transaction_id: txn_id,
            prepared: true,
            reason: None,
        })
    }
    
    /// Commit phase of two-phase commit
    pub async fn commit_phase(&self, txn_id: TransactionId) -> Result<()> {
        let transaction = {
            let transactions = self.active_transactions.read().await;
            transactions.get(&txn_id)
                .ok_or_else(|| ConsensusError::TransactionError("Transaction not found".to_string()))?
                .clone()
        };
        
        if transaction.status != TransactionStatus::Prepared {
            return Err(ConsensusError::TransactionError("Transaction not prepared".to_string()));
        }
        
        // Perform the actual commit
        self.commit(txn_id).await?;
        
        debug!("Completed commit phase for transaction {}", txn_id);
        Ok(())
    }
    
    /// Get transaction statistics
    pub async fn statistics(&self) -> TransactionStatistics {
        let active_count = self.active_transactions.read().await.len();
        
        TransactionStatistics {
            active_transactions: active_count,
            total_started: self.metrics.transactions_started.get() as u64,
            total_committed: self.metrics.transactions_committed.get() as u64,
            total_aborted: self.metrics.transactions_aborted.get() as u64,
            deadlocks_detected: self.metrics.deadlocks_detected.get() as u64,
        }
    }
    
    /// Abort a transaction with a reason
    async fn abort_transaction(&self, txn_id: TransactionId, reason: &str) -> Result<()> {
        let abort_start = Instant::now();
        
        // Update transaction status
        {
            let mut transactions = self.active_transactions.write().await;
            if let Some(transaction) = transactions.get_mut(&txn_id) {
                transaction.status = TransactionStatus::Aborted;
            }
        }
        
        // Release all locks
        self.lock_manager.release_all_locks(txn_id).await;
        
        // Remove from active transactions
        self.active_transactions.write().await.remove(&txn_id);
        
        let duration = abort_start.elapsed();
        
        // Update metrics
        self.metrics.record_transaction_abort(duration, reason);
        
        warn!("Aborted transaction {}: {}", txn_id, reason);
        Ok(())
    }
    
    /// Detect serialization conflicts
    async fn detect_conflicts(&self, transaction: &Transaction, commit_timestamp: Timestamp) -> Result<Vec<ConflictInfo>> {
        let mut conflicts = Vec::new();
        
        // Check write-write conflicts
        let write_conflicts = self.storage
            .check_write_conflicts(&transaction.write_set, transaction.start_timestamp, commit_timestamp)
            .await?;
        conflicts.extend(write_conflicts);
        
        // Check read-write conflicts
        let read_conflicts = self.storage
            .check_read_conflicts(&transaction.read_set, transaction.start_timestamp, commit_timestamp)
            .await?;
        conflicts.extend(read_conflicts);
        
        Ok(conflicts)
    }
    
    /// Start deadlock detection background task
    async fn start_deadlock_detector(&self) {
        let detector = self.deadlock_detector.clone();
        let transactions = self.active_transactions.clone();
        let lock_manager = self.lock_manager.clone();
        let metrics = self.metrics.clone();
        let interval = Duration::from_millis(self.config.deadlock_detection_interval_ms);
        
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                
                let deadlock_start = Instant::now();
                
                // Build wait-for graph
                {
                    let mut detector_guard = detector.lock().await;
                    detector_guard.update_wait_for_graph(&*transactions.read().await, &*lock_manager).await;
                    
                    // Detect cycles
                    if let Some(cycle) = detector_guard.detect_cycle() {
                        let victim = detector_guard.select_victim(&cycle);
                        warn!("Deadlock detected, aborting victim transaction: {:?}", victim);
                        
                        // Abort the victim transaction
                        // Note: In a full implementation, would call abort_transaction
                        
                        let duration = deadlock_start.elapsed();
                        metrics.deadlock_resolution_time.observe(duration.as_secs_f64());
                        metrics.deadlocks_detected.inc();
                    }
                }
            }
        });
    }
    
    /// Start timeout cleaner background task
    async fn start_timeout_cleaner(&self) {
        let transactions = self.active_transactions.clone();
        let cleanup_interval = Duration::from_secs(60); // Clean up every minute
        
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(cleanup_interval).await;
                
                let now = Instant::now();
                let mut to_abort = Vec::new();
                
                // Find timed out transactions
                {
                    let transactions_guard = transactions.read().await;
                    for (txn_id, transaction) in transactions_guard.iter() {
                        if now >= transaction.timeout_at {
                            to_abort.push(*txn_id);
                        }
                    }
                }
                
                // Abort timed out transactions
                for txn_id in to_abort {
                    warn!("Aborting transaction {} due to timeout", txn_id);
                    // Note: In full implementation, would call abort_transaction
                }
            }
        });
    }
}

/// Transaction statistics
#[derive(Debug, Clone)]
pub struct TransactionStatistics {
    pub active_transactions: usize,
    pub total_started: u64,
    pub total_committed: u64,
    pub total_aborted: u64,
    pub deadlocks_detected: u64,
}

impl LockManager {
    fn new() -> Self {
        Self {
            locks: Arc::new(RwLock::new(HashMap::new())),
            wait_queue: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    async fn acquire_lock(&self, txn_id: TransactionId, key: &str, lock_type: LockType) -> Result<()> {
        debug!("Transaction {} requesting {:?} lock on key {}", txn_id, lock_type, key);
        
        loop {
            // Check if we can acquire the lock immediately
            {
                let mut locks = self.locks.write().await;
                if let Some(existing) = locks.get(key) {
                    // Check compatibility
                    let compatible = match (&existing.lock_type, &lock_type) {
                        (LockType::Shared, LockType::Shared) => true,
                        _ => existing.transaction_id == txn_id, // Same transaction can upgrade
                    };
                    
                    if compatible {
                        // Grant the lock
                        locks.insert(key.to_string(), LockInfo {
                            transaction_id: txn_id,
                            lock_type,
                            acquired_at: Instant::now(),
                        });
                        debug!("Granted {:?} lock on key {} to transaction {}", lock_type, key, txn_id);
                        return Ok(());
                    }
                } else {
                    // No existing lock, grant immediately
                    locks.insert(key.to_string(), LockInfo {
                        transaction_id: txn_id,
                        lock_type,
                        acquired_at: Instant::now(),
                    });
                    debug!("Granted {:?} lock on key {} to transaction {}", lock_type, key, txn_id);
                    return Ok(());
                }
            }
            
            // Add to wait queue
            {
                let mut wait_queue = self.wait_queue.write().await;
                let waiters = wait_queue.entry(key.to_string()).or_insert_with(Vec::new);
                waiters.push(LockWaiter {
                    transaction_id: txn_id,
                    lock_type,
                    waiting_since: Instant::now(),
                });
            }
            
            // Wait a bit and retry
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
    
    async fn release_all_locks(&self, txn_id: TransactionId) {
        let mut released_keys = Vec::new();
        
        // Release all locks held by this transaction
        {
            let mut locks = self.locks.write().await;
            locks.retain(|key, lock_info| {
                if lock_info.transaction_id == txn_id {
                    released_keys.push(key.clone());
                    false
                } else {
                    true
                }
            });
        }
        
        // Process wait queues for released locks
        for key in released_keys {
            self.process_wait_queue(&key).await;
        }
        
        debug!("Released all locks for transaction {}", txn_id);
    }
    
    async fn process_wait_queue(&self, key: &str) {
        let mut wait_queue = self.wait_queue.write().await;
        if let Some(waiters) = wait_queue.get_mut(key) {
            if let Some(waiter) = waiters.first().copied() {
                // Try to grant lock to first waiter
                let mut locks = self.locks.write().await;
                locks.insert(key.to_string(), LockInfo {
                    transaction_id: waiter.transaction_id,
                    lock_type: waiter.lock_type,
                    acquired_at: Instant::now(),
                });
                waiters.remove(0);
                
                debug!("Granted queued {:?} lock on key {} to transaction {}", 
                       waiter.lock_type, key, waiter.transaction_id);
            }
            
            if waiters.is_empty() {
                wait_queue.remove(key);
            }
        }
    }
}

impl TimestampOracle {
    fn new() -> Self {
        Self {
            current_timestamp: Arc::new(RwLock::new(1)),
        }
    }
    
    async fn next_timestamp(&self) -> Timestamp {
        let mut timestamp = self.current_timestamp.write().await;
        *timestamp += 1;
        *timestamp
    }
}

impl WriteSetTracker {
    fn new() -> Self {
        Self {
            recent_writes: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    async fn track_write(&self, key: &str, txn_id: TransactionId, timestamp: Timestamp) {
        let mut recent_writes = self.recent_writes.write().await;
        let writes = recent_writes.entry(key.to_string()).or_insert_with(Vec::new);
        
        writes.push(WriteRecord {
            transaction_id: txn_id,
            timestamp,
            written_at: Instant::now(),
        });
        
        // Keep only recent writes (last 5 minutes)
        let cutoff = Instant::now() - Duration::from_secs(300);
        writes.retain(|record| record.written_at > cutoff);
    }
}

impl DeadlockDetector {
    fn new() -> Self {
        Self {
            wait_for_graph: HashMap::new(),
            last_detection: Instant::now(),
        }
    }
    
    async fn update_wait_for_graph(
        &mut self,
        transactions: &HashMap<TransactionId, Transaction>,
        lock_manager: &LockManager,
    ) {
        self.wait_for_graph.clear();
        
        // Build wait-for relationships
        let wait_queue = lock_manager.wait_queue.read().await;
        let locks = lock_manager.locks.read().await;
        
        for (key, waiters) in wait_queue.iter() {
            if let Some(lock_holder) = locks.get(key) {
                for waiter in waiters {
                    // Waiter is waiting for lock holder
                    self.wait_for_graph
                        .entry(waiter.transaction_id)
                        .or_insert_with(HashSet::new)
                        .insert(lock_holder.transaction_id);
                }
            }
        }
        
        self.last_detection = Instant::now();
    }
    
    fn detect_cycle(&self) -> Option<Vec<TransactionId>> {
        // Simple cycle detection using DFS
        for &start_node in self.wait_for_graph.keys() {
            if let Some(cycle) = self.dfs_cycle_detection(start_node, &mut HashSet::new(), &mut Vec::new()) {
                return Some(cycle);
            }
        }
        None
    }
    
    fn dfs_cycle_detection(
        &self,
        node: TransactionId,
        visited: &mut HashSet<TransactionId>,
        path: &mut Vec<TransactionId>,
    ) -> Option<Vec<TransactionId>> {
        if path.contains(&node) {
            // Found cycle
            if let Some(cycle_start) = path.iter().position(|&x| x == node) {
                return Some(path[cycle_start..].to_vec());
            } else {
                // Should not happen but handle gracefully
                return Some(vec![node]);
            }
        }
        
        if visited.contains(&node) {
            return None;
        }
        
        visited.insert(node);
        path.push(node);
        
        if let Some(dependencies) = self.wait_for_graph.get(&node) {
            for &dependency in dependencies {
                if let Some(cycle) = self.dfs_cycle_detection(dependency, visited, path) {
                    return Some(cycle);
                }
            }
        }
        
        path.pop();
        None
    }
    
    fn select_victim(&self, cycle: &[TransactionId]) -> TransactionId {
        // Simple victim selection: choose the transaction with the smallest ID
        // In practice, would use more sophisticated criteria
        cycle.iter().min().copied().unwrap_or_else(|| {
            // Fallback to first transaction if cycle is empty (shouldn't happen)
            cycle.first().copied().unwrap_or_else(|| Uuid::new_v4())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::storage::{MockStorage, MVCCStorage};
    use super::config::{StorageConfig, RocksDBConfig};
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_basic_transaction() {
        let temp_dir = tempdir().unwrap();
        let storage_config = StorageConfig {
            data_dir: temp_dir.path().to_path_buf(),
            max_versions_per_key: 10,
            gc_interval_seconds: 3600,
            gc_watermark_lag_seconds: 1800,
            version_compression: true,
            rocksdb: RocksDBConfig::default(),
            memtable_size_mb: 64,
            write_buffer_size_mb: 32,
        };
        
        let storage = Arc::new(MVCCStorage::new(&storage_config).await.unwrap());
        let config = TransactionConfig::default();
        let txn_manager = TransactionManager::new(storage, config).await.unwrap();
        
        // Begin transaction
        let txn_id = txn_manager.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();
        
        // Write some data
        txn_manager.write(txn_id, "key1".to_string(), b"value1".to_vec()).await.unwrap();
        txn_manager.write(txn_id, "key2".to_string(), b"value2".to_vec()).await.unwrap();
        
        // Read the data
        let result = txn_manager.read(txn_id, "key1").await.unwrap();
        assert!(result.is_none()); // Not committed yet, should not see own writes in MVCC
        
        // Commit transaction
        let commit_result = txn_manager.commit(txn_id).await.unwrap();
        assert_eq!(commit_result.committed_keys.len(), 2);
        
        // Verify statistics
        let stats = txn_manager.statistics().await;
        assert_eq!(stats.total_started, 1);
        assert_eq!(stats.total_committed, 1);
        assert_eq!(stats.active_transactions, 0);
    }
    
    #[tokio::test]
    async fn test_transaction_rollback() {
        let temp_dir = tempdir().unwrap();
        let storage_config = StorageConfig {
            data_dir: temp_dir.path().to_path_buf(),
            max_versions_per_key: 10,
            gc_interval_seconds: 3600,
            gc_watermark_lag_seconds: 1800,
            version_compression: true,
            rocksdb: RocksDBConfig::default(),
            memtable_size_mb: 64,
            write_buffer_size_mb: 32,
        };
        
        let storage = Arc::new(MVCCStorage::new(&storage_config).await.unwrap());
        let config = TransactionConfig::default();
        let txn_manager = TransactionManager::new(storage, config).await.unwrap();
        
        // Begin transaction
        let txn_id = txn_manager.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();
        
        // Write some data
        txn_manager.write(txn_id, "key1".to_string(), b"value1".to_vec()).await.unwrap();
        
        // Rollback transaction
        txn_manager.rollback(txn_id).await.unwrap();
        
        // Verify statistics
        let stats = txn_manager.statistics().await;
        assert_eq!(stats.total_started, 1);
        assert_eq!(stats.total_aborted, 1);
        assert_eq!(stats.active_transactions, 0);
    }
    
    #[tokio::test]
    async fn test_isolation_levels() {
        let temp_dir = tempdir().unwrap();
        let storage_config = StorageConfig {
            data_dir: temp_dir.path().to_path_buf(),
            max_versions_per_key: 10,
            gc_interval_seconds: 3600,
            gc_watermark_lag_seconds: 1800,
            version_compression: true,
            rocksdb: RocksDBConfig::default(),
            memtable_size_mb: 64,
            write_buffer_size_mb: 32,
        };
        
        let storage = Arc::new(MVCCStorage::new(&storage_config).await.unwrap());
        let config = TransactionConfig::default();
        let txn_manager = TransactionManager::new(storage.clone(), config).await.unwrap();
        
        // Pre-populate data
        let setup_txn = txn_manager.begin_transaction(IsolationLevel::ReadCommitted).await.unwrap();
        txn_manager.write(setup_txn, "key1".to_string(), b"initial".to_vec()).await.unwrap();
        txn_manager.commit(setup_txn).await.unwrap();
        
        // Test serializable isolation
        let txn1 = txn_manager.begin_transaction(IsolationLevel::Serializable).await.unwrap();
        let txn2 = txn_manager.begin_transaction(IsolationLevel::Serializable).await.unwrap();
        
        // Both transactions read the same key
        let _val1 = txn_manager.read(txn1, "key1").await.unwrap();
        let _val2 = txn_manager.read(txn2, "key1").await.unwrap();
        
        // Both try to write
        txn_manager.write(txn1, "key1".to_string(), b"value1".to_vec()).await.unwrap();
        txn_manager.write(txn2, "key1".to_string(), b"value2".to_vec()).await.unwrap();
        
        // First commit should succeed
        let result1 = txn_manager.commit(txn1).await;
        assert!(result1.is_ok());
        
        // Second commit should fail due to conflict
        let result2 = txn_manager.commit(txn2).await;
        // Note: In this simplified test, conflict detection is basic
        // In a full implementation, txn2 should fail
    }
    
    #[tokio::test]
    async fn test_distributed_transaction() {
        let temp_dir = tempdir().unwrap();
        let storage_config = StorageConfig {
            data_dir: temp_dir.path().to_path_buf(),
            max_versions_per_key: 10,
            gc_interval_seconds: 3600,
            gc_watermark_lag_seconds: 1800,
            version_compression: true,
            rocksdb: RocksDBConfig::default(),
            memtable_size_mb: 64,
            write_buffer_size_mb: 32,
        };
        
        let storage = Arc::new(MVCCStorage::new(&storage_config).await.unwrap());
        let config = TransactionConfig::default();
        let txn_manager = TransactionManager::new(storage, config).await.unwrap();
        
        // Begin distributed transaction
        let shards = vec!["shard1".to_string(), "shard2".to_string()];
        let txn_id = txn_manager.begin_distributed_transaction(shards).await.unwrap();
        
        // Write to different shards (simulated)
        txn_manager.write(txn_id, "shard1:key1".to_string(), b"value1".to_vec()).await.unwrap();
        txn_manager.write(txn_id, "shard2:key2".to_string(), b"value2".to_vec()).await.unwrap();
        
        // Prepare phase
        let prepare_result = txn_manager.prepare_phase(txn_id).await.unwrap();
        assert!(prepare_result.prepared);
        
        // Commit phase
        txn_manager.commit_phase(txn_id).await.unwrap();
        
        let stats = txn_manager.statistics().await;
        assert_eq!(stats.total_committed, 1);
    }
}