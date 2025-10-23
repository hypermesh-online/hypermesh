//! Automatic data sharding and rebalancing system

use super::{
    storage::{StorageEngine, MVCCStorage},
    config::{ShardingConfig, HashFunction, HotShardConfig, MitigationStrategy},
    error::{ConsensusError, Result},
    metrics::ConsensusMetrics,
};

use super::transport::{NodeId, HyperMeshTransportTrait};
use std::collections::{HashMap, BTreeMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use std::time::{Duration, Instant, SystemTime};
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use blake3::Hasher as Blake3Hasher;
use rand::Rng;
use tracing::{debug, warn, info, error};

/// Unique identifier for shards
pub type ShardId = String;

/// Shard manager for automatic data distribution
pub struct ShardManager {
    /// This node's identifier
    node_id: NodeId,
    
    /// Active shards managed by this cluster
    shards: Arc<RwLock<HashMap<ShardId, Shard>>>,
    
    /// Consistent hash ring for data distribution
    hash_ring: Arc<RwLock<ConsistentHashRing>>,
    
    /// Shard rebalancer for automatic rebalancing
    rebalancer: Arc<ShardRebalancer>,
    
    /// Hot shard detector
    hot_shard_detector: Arc<HotShardDetector>,
    
    /// Storage backend
    storage: Arc<MVCCStorage>,
    
    /// Network transport
    transport: Arc<dyn HyperMeshTransportTrait>,
    
    /// Configuration
    config: ShardingConfig,
    
    /// Metrics collection
    metrics: Arc<ConsensusMetrics>,
    
    /// Migration tracker
    migration_tracker: Arc<Mutex<MigrationTracker>>,
}

/// Individual shard information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shard {
    /// Unique shard identifier
    pub id: ShardId,
    
    /// Key range for this shard (start_key, end_key)
    pub key_range: (String, String),
    
    /// Replica nodes for this shard
    pub replicas: HashSet<NodeId>,
    
    /// Primary replica (leader for this shard)
    pub primary: NodeId,
    
    /// Current shard status
    pub status: ShardStatus,
    
    /// Shard statistics
    pub stats: ShardStats,
    
    /// Creation timestamp
    pub created_at: SystemTime,
    
    /// Last modified timestamp
    pub last_modified: SystemTime,
}

/// Status of a shard
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShardStatus {
    /// Shard is active and serving requests
    Active,
    /// Shard is being created/initialized
    Initializing,
    /// Shard is being migrated
    Migrating,
    /// Shard is being split
    Splitting,
    /// Shard is being merged
    Merging,
    /// Shard is temporarily unavailable
    Unavailable,
    /// Shard is being decommissioned
    Decommissioning,
}

/// Statistics for a shard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardStats {
    /// Total number of keys
    pub key_count: u64,
    /// Total size in bytes
    pub size_bytes: u64,
    /// Request rate (requests per second)
    pub request_rate: f64,
    /// CPU utilization percentage
    pub cpu_usage: f64,
    /// Memory utilization percentage
    pub memory_usage: f64,
    /// Average response time in microseconds
    pub avg_response_time_us: u64,
    /// Last updated timestamp
    pub last_updated: SystemTime,
}

impl Default for ShardStats {
    fn default() -> Self {
        Self {
            key_count: 0,
            size_bytes: 0,
            request_rate: 0.0,
            cpu_usage: 0.0,
            memory_usage: 0.0,
            avg_response_time_us: 0,
            last_updated: SystemTime::now(),
        }
    }
}

/// Consistent hash ring for data distribution
pub struct ConsistentHashRing {
    /// Ring mapping hash values to nodes
    ring: BTreeMap<u64, NodeId>,
    /// Virtual nodes per physical node
    virtual_nodes: usize,
    /// Hash function to use
    hash_function: HashFunction,
    /// Total weight of all nodes
    total_weight: f64,
}

/// Shard rebalancer for automatic load balancing
pub struct ShardRebalancer {
    /// Rebalancing state
    state: Arc<RwLock<RebalancingState>>,
    /// Configuration
    config: ShardingConfig,
    /// Metrics
    metrics: Arc<ConsensusMetrics>,
}

/// State of the rebalancing process
#[derive(Debug, Clone, Default)]
struct RebalancingState {
    /// Whether rebalancing is currently active
    active: bool,
    /// Rebalancing start time
    started_at: Option<Instant>,
    /// Shards being rebalanced
    shards_in_progress: HashSet<ShardId>,
    /// Target distribution
    target_distribution: HashMap<NodeId, f64>,
}

/// Hot shard detector for identifying performance bottlenecks
pub struct HotShardDetector {
    /// Recent shard metrics for analysis
    metrics_history: Arc<RwLock<HashMap<ShardId, VecDeque<ShardStats>>>>,
    /// Configuration
    config: HotShardConfig,
    /// Detected hot shards
    hot_shards: Arc<RwLock<HashSet<ShardId>>>,
    /// Detection state
    detection_state: Arc<RwLock<DetectionState>>,
}

/// State of hot shard detection
#[derive(Debug, Clone)]
struct DetectionState {
    /// Last detection run
    last_detection: Instant,
    /// Detection window start
    window_start: Instant,
}

impl Default for DetectionState {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            last_detection: now,
            window_start: now,
        }
    }
}

/// Migration tracker for shard movements
#[derive(Debug, Default)]
struct MigrationTracker {
    /// Active migrations
    active_migrations: HashMap<ShardId, MigrationInfo>,
    /// Migration history
    migration_history: VecDeque<CompletedMigration>,
}

/// Information about an active migration
#[derive(Debug, Clone)]
struct MigrationInfo {
    shard_id: ShardId,
    source_nodes: HashSet<NodeId>,
    target_nodes: HashSet<NodeId>,
    started_at: Instant,
    progress: f64, // 0.0 to 1.0
    reason: MigrationReason,
}

/// Reason for shard migration
#[derive(Debug, Clone)]
enum MigrationReason {
    HotShardMitigation,
    Rebalancing,
    NodeAddition,
    NodeRemoval,
    ShardSplit,
    ShardMerge,
}

/// Completed migration record
#[derive(Debug, Clone)]
struct CompletedMigration {
    shard_id: ShardId,
    completed_at: Instant,
    duration: Duration,
    reason: MigrationReason,
    success: bool,
}

/// Result of rebalancing operation
#[derive(Debug, Clone)]
pub struct RebalanceResult {
    /// Number of shards moved
    pub shards_moved: usize,
    /// Duration of rebalancing
    pub duration: Duration,
    /// Improvement in load distribution
    pub load_improvement: f64,
}

/// Result of hot shard mitigation
#[derive(Debug, Clone)]
pub struct MitigationResult {
    /// Shard that was mitigated
    pub shard_id: ShardId,
    /// Strategy applied
    pub strategy: MitigationStrategy,
    /// Expected load reduction
    pub load_reduction: f64,
}

/// Routing error types
#[derive(Debug)]
pub enum RoutingError {
    NoShardFound,
    ShardUnavailable,
    InvalidKey,
}

impl ShardManager {
    /// Create a new shard manager
    pub async fn new(
        node_id: NodeId,
        storage: Arc<MVCCStorage>,
        transport: Arc<dyn HyperMeshTransportTrait>,
        config: ShardingConfig,
    ) -> Result<Self> {
        let metrics = Arc::new(
            ConsensusMetrics::new()
                .map_err(|e| ConsensusError::Internal(format!("Failed to create metrics: {}", e)))?
        );
        
        let hash_ring = Arc::new(RwLock::new(
            ConsistentHashRing::new(
                config.virtual_nodes_per_physical_node,
                config.hash_function.clone(),
            )
        ));
        
        let rebalancer = Arc::new(ShardRebalancer::new(config.clone(), metrics.clone()));
        
        let hot_shard_detector = Arc::new(HotShardDetector::new(config.hot_shard_detection.clone()));
        
        let manager = Self {
            node_id,
            shards: Arc::new(RwLock::new(HashMap::new())),
            hash_ring,
            rebalancer,
            hot_shard_detector,
            storage,
            transport,
            config,
            metrics,
            migration_tracker: Arc::new(Mutex::new(MigrationTracker::default())),
        };
        
        // Initialize with default shards
        manager.initialize_default_shards().await?;
        
        Ok(manager)
    }
    
    /// Start the shard manager
    pub async fn start(self: Arc<Self>) -> Result<()> {
        info!("Starting shard manager for node {:?}", self.node_id);
        
        // Start background tasks
        Arc::clone(&self).start_rebalancing_task();
        Arc::clone(&self).start_hot_shard_detection_task();
        self.start_metrics_collection_task().await;
        
        info!("Shard manager started successfully");
        Ok(())
    }
    
    /// Stop the shard manager
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping shard manager");
        
        // Complete any active migrations
        self.complete_pending_migrations().await?;
        
        info!("Shard manager stopped");
        Ok(())
    }
    
    /// Route a request to the appropriate shard
    pub async fn route_request(&self, key: &str) -> Result<ShardId> {
        let hash = self.compute_key_hash(key);
        
        let hash_ring = self.hash_ring.read().await;
        let node = hash_ring.get_node(hash)
            .ok_or_else(|| ConsensusError::ShardError {
                shard_id: "unknown".to_string(),
                message: "No node found in hash ring".to_string(),
            })?;
        
        // Find the shard for this key on the selected node
        let shards = self.shards.read().await;
        for (shard_id, shard) in shards.iter() {
            if shard.primary == node && self.key_in_range(key, &shard.key_range) {
                if shard.status == ShardStatus::Active {
                    return Ok(shard_id.clone());
                } else {
                    return Err(ConsensusError::ShardError {
                        shard_id: shard_id.clone(),
                        message: format!("Shard status: {:?}", shard.status),
                    });
                }
            }
        }
        
        Err(ConsensusError::ShardError {
            shard_id: "unknown".to_string(),
            message: "No suitable shard found".to_string(),
        })
    }
    
    /// Create a new shard with specified replicas
    pub async fn create_shard(&self, shard_id: ShardId, replicas: Vec<NodeId>) -> Result<()> {
        info!("Creating shard {} with {} replicas", shard_id, replicas.len());
        
        if replicas.is_empty() {
            return Err(ConsensusError::ShardError {
                shard_id: shard_id.clone(),
                message: "At least one replica required".to_string(),
            });
        }
        
        let primary = replicas[0].clone();
        let replica_set: HashSet<_> = replicas.into_iter().collect();
        
        // Determine key range based on existing shards
        let key_range = self.calculate_key_range_for_new_shard().await;
        
        let shard = Shard {
            id: shard_id.clone(),
            key_range,
            replicas: replica_set,
            primary,
            status: ShardStatus::Initializing,
            stats: ShardStats::default(),
            created_at: SystemTime::now(),
            last_modified: SystemTime::now(),
        };
        
        // Add to shards collection
        self.shards.write().await.insert(shard_id.clone(), shard);
        
        // Initialize shard on replicas
        self.initialize_shard_replicas(&shard_id).await?;
        
        // Mark as active
        {
            let mut shards = self.shards.write().await;
            if let Some(shard) = shards.get_mut(&shard_id) {
                shard.status = ShardStatus::Active;
                shard.last_modified = SystemTime::now();
            }
        }
        
        // Update metrics
        self.metrics.active_shards.inc();
        
        info!("Created shard {} successfully", shard_id);
        Ok(())
    }
    
    /// Split a shard at the specified key
    pub async fn split_shard(&self, shard_id: ShardId, split_key: &str) -> Result<(ShardId, ShardId)> {
        let split_start = Instant::now();
        info!("Splitting shard {} at key '{}'", shard_id, split_key);
        
        // Get original shard
        let original_shard = {
            let shards = self.shards.read().await;
            shards.get(&shard_id)
                .ok_or_else(|| ConsensusError::ShardError {
                    shard_id: shard_id.clone(),
                    message: "Shard not found".to_string(),
                })?
                .clone()
        };
        
        // Validate split key is within shard range
        if !self.key_in_range(split_key, &original_shard.key_range) {
            return Err(ConsensusError::ShardError {
                shard_id: shard_id.clone(),
                message: "Split key not in shard range".to_string(),
            });
        }
        
        // Mark original shard as splitting
        {
            let mut shards = self.shards.write().await;
            if let Some(shard) = shards.get_mut(&shard_id) {
                shard.status = ShardStatus::Splitting;
            }
        }
        
        // Create two new shard IDs
        let shard_id_1 = format!("{}_1", shard_id);
        let shard_id_2 = format!("{}_2", shard_id);
        
        // Create first shard (covers start to split key)
        let shard_1 = Shard {
            id: shard_id_1.clone(),
            key_range: (original_shard.key_range.0.clone(), split_key.to_string()),
            replicas: original_shard.replicas.clone(),
            primary: original_shard.primary.clone(),
            status: ShardStatus::Initializing,
            stats: ShardStats::default(),
            created_at: SystemTime::now(),
            last_modified: SystemTime::now(),
        };
        
        // Create second shard (covers split key to end)
        let shard_2 = Shard {
            id: shard_id_2.clone(),
            key_range: (split_key.to_string(), original_shard.key_range.1.clone()),
            replicas: original_shard.replicas.clone(),
            primary: original_shard.primary.clone(),
            status: ShardStatus::Initializing,
            stats: ShardStats::default(),
            created_at: SystemTime::now(),
            last_modified: SystemTime::now(),
        };
        
        // Migrate data to new shards
        self.migrate_shard_data(&shard_id, &shard_id_1, &shard_id_2, split_key).await?;
        
        // Add new shards and remove original
        {
            let mut shards = self.shards.write().await;
            shards.insert(shard_id_1.clone(), shard_1);
            shards.insert(shard_id_2.clone(), shard_2);
            shards.remove(&shard_id);
        }
        
        // Activate new shards
        {
            let mut shards = self.shards.write().await;
            if let Some(shard) = shards.get_mut(&shard_id_1) {
                shard.status = ShardStatus::Active;
            }
            if let Some(shard) = shards.get_mut(&shard_id_2) {
                shard.status = ShardStatus::Active;
            }
        }
        
        let duration = split_start.elapsed();
        
        // Update metrics
        self.metrics.record_shard_split(&shard_id, duration);
        
        info!("Split shard {} into {} and {} in {:?}", 
              shard_id, shard_id_1, shard_id_2, duration);
        
        Ok((shard_id_1, shard_id_2))
    }
    
    /// Merge two adjacent shards
    pub async fn merge_shards(&self, shard1: ShardId, shard2: ShardId) -> Result<ShardId> {
        let merge_start = Instant::now();
        info!("Merging shards {} and {}", shard1, shard2);
        
        // Get both shards
        let (shard_1, shard_2) = {
            let shards = self.shards.read().await;
            let s1 = shards.get(&shard1)
                .ok_or_else(|| ConsensusError::ShardError {
                    shard_id: shard1.clone(),
                    message: "Shard not found".to_string(),
                })?
                .clone();
            let s2 = shards.get(&shard2)
                .ok_or_else(|| ConsensusError::ShardError {
                    shard_id: shard2.clone(),
                    message: "Shard not found".to_string(),
                })?
                .clone();
            (s1, s2)
        };
        
        // Validate shards are adjacent and compatible
        if !self.shards_are_adjacent(&shard_1, &shard_2) {
            return Err(ConsensusError::ShardError {
                shard_id: "merge".to_string(),
                message: "Shards are not adjacent".to_string(),
            });
        }
        
        // Mark shards as merging
        {
            let mut shards = self.shards.write().await;
            if let Some(shard) = shards.get_mut(&shard1) {
                shard.status = ShardStatus::Merging;
            }
            if let Some(shard) = shards.get_mut(&shard2) {
                shard.status = ShardStatus::Merging;
            }
        }
        
        // Create merged shard
        let merged_id = format!("{}_{}_merged", shard1, shard2);
        let merged_range = (
            shard_1.key_range.0.clone(),
            shard_2.key_range.1.clone(),
        );
        
        let merged_shard = Shard {
            id: merged_id.clone(),
            key_range: merged_range,
            replicas: shard_1.replicas.union(&shard_2.replicas).cloned().collect(),
            primary: shard_1.primary.clone(), // Use first shard's primary
            status: ShardStatus::Initializing,
            stats: ShardStats::default(),
            created_at: SystemTime::now(),
            last_modified: SystemTime::now(),
        };
        
        // Migrate data from both shards to merged shard
        self.merge_shard_data(&shard1, &shard2, &merged_id).await?;
        
        // Update shards collection
        {
            let mut shards = self.shards.write().await;
            shards.insert(merged_id.clone(), merged_shard);
            shards.remove(&shard1);
            shards.remove(&shard2);
        }
        
        // Activate merged shard
        {
            let mut shards = self.shards.write().await;
            if let Some(shard) = shards.get_mut(&merged_id) {
                shard.status = ShardStatus::Active;
            }
        }
        
        let duration = merge_start.elapsed();
        
        // Update metrics
        self.metrics.shard_merges.inc();
        
        info!("Merged shards {} and {} into {} in {:?}", 
              shard1, shard2, merged_id, duration);
        
        Ok(merged_id)
    }
    
    /// Rebalance shards across the cluster
    pub async fn rebalance_shards(&self) -> Result<RebalanceResult> {
        let rebalance_start = Instant::now();
        info!("Starting shard rebalancing");
        
        // Check if rebalancing is already active
        {
            let state = self.rebalancer.state.read().await;
            if state.active {
                return Err(ConsensusError::ShardError {
                    shard_id: "rebalance".to_string(),
                    message: "Rebalancing already active".to_string(),
                });
            }
        }
        
        // Mark rebalancing as active
        {
            let mut state = self.rebalancer.state.write().await;
            state.active = true;
            state.started_at = Some(rebalance_start);
            state.shards_in_progress.clear();
        }
        
        let mut shards_moved = 0;
        
        // Analyze current distribution
        let load_distribution = self.analyze_load_distribution().await;
        let imbalance = self.calculate_load_imbalance(&load_distribution);
        
        if imbalance > self.config.rebalance_threshold {
            // Identify shards to move
            let movements = self.plan_rebalancing_movements(&load_distribution).await;
            
            // Execute movements
            for (shard_id, target_node) in movements {
                self.migrate_shard(&shard_id, target_node).await?;
                shards_moved += 1;
                
                // Add to rebalancing state
                {
                    let mut state = self.rebalancer.state.write().await;
                    state.shards_in_progress.insert(shard_id);
                }
            }
        }
        
        // Calculate improvement
        let new_distribution = self.analyze_load_distribution().await;
        let new_imbalance = self.calculate_load_imbalance(&new_distribution);
        let improvement = imbalance - new_imbalance;
        
        // Mark rebalancing as complete
        {
            let mut state = self.rebalancer.state.write().await;
            state.active = false;
            state.started_at = None;
            state.shards_in_progress.clear();
        }
        
        let duration = rebalance_start.elapsed();
        let result = RebalanceResult {
            shards_moved,
            duration,
            load_improvement: improvement,
        };
        
        // Update metrics
        self.metrics.shard_rebalances.inc();
        
        info!("Rebalancing completed: moved {} shards, improvement: {:.2}%, duration: {:?}",
              shards_moved, improvement * 100.0, duration);
        
        Ok(result)
    }
    
    /// Detect and mitigate hot shards
    pub async fn detect_hot_shards(&self) -> Result<Vec<ShardId>> {
        self.hot_shard_detector.detect_hot_shards().await
    }
    
    /// Mitigate a hot shard using configured strategies
    pub async fn mitigate_hot_shard(&self, shard_id: ShardId) -> Result<MitigationResult> {
        let mitigation_start = Instant::now();
        info!("Mitigating hot shard: {}", shard_id);
        
        // Get shard information
        let shard = {
            let shards = self.shards.read().await;
            shards.get(&shard_id)
                .ok_or_else(|| ConsensusError::ShardError {
                    shard_id: shard_id.clone(),
                    message: "Shard not found".to_string(),
                })?
                .clone()
        };
        
        // Try mitigation strategies in order
        for strategy in &self.config.hot_shard_detection.mitigation_strategies {
            match strategy {
                MitigationStrategy::Split => {
                    // Find optimal split point
                    let split_key = self.find_optimal_split_key(&shard_id).await?;
                    self.split_shard(shard_id.clone(), &split_key).await?;
                    
                    let duration = mitigation_start.elapsed();
                    self.metrics.record_hot_shard_detection(&shard_id, "split", 0.5);
                    
                    return Ok(MitigationResult {
                        shard_id,
                        strategy: strategy.clone(),
                        load_reduction: 0.5, // Approximate 50% load reduction from split
                    });
                }
                MitigationStrategy::Replicate => {
                    // Add more replicas to distribute read load
                    self.add_read_replicas(&shard_id, 2).await?;
                    
                    let duration = mitigation_start.elapsed();
                    self.metrics.record_hot_shard_detection(&shard_id, "replicate", 0.3);
                    
                    return Ok(MitigationResult {
                        shard_id,
                        strategy: strategy.clone(),
                        load_reduction: 0.3, // Approximate 30% load reduction from replication
                    });
                }
                MitigationStrategy::Cache => {
                    // Implement caching for frequently accessed data
                    self.enable_shard_caching(&shard_id).await?;
                    
                    let duration = mitigation_start.elapsed();
                    self.metrics.record_hot_shard_detection(&shard_id, "cache", 0.4);
                    
                    return Ok(MitigationResult {
                        shard_id,
                        strategy: strategy.clone(),
                        load_reduction: 0.4, // Approximate 40% load reduction from caching
                    });
                }
            }
        }
        
        Err(ConsensusError::ShardError {
            shard_id,
            message: "No mitigation strategy succeeded".to_string(),
        })
    }
    
    /// Get shard statistics
    pub async fn get_shard_stats(&self, shard_id: &ShardId) -> Option<ShardStats> {
        let shards = self.shards.read().await;
        shards.get(shard_id).map(|shard| shard.stats.clone())
    }
    
    /// Get all active shards
    pub async fn get_all_shards(&self) -> HashMap<ShardId, Shard> {
        self.shards.read().await.clone()
    }
    
    /// Private helper methods
    
    /// Initialize default shards for a new cluster
    async fn initialize_default_shards(&self) -> Result<()> {
        info!("Initializing {} default shards", self.config.initial_shard_count);
        
        // Add this node to hash ring
        self.hash_ring.write().await.add_node(self.node_id.clone(), 1.0)?;
        
        for i in 0..self.config.initial_shard_count {
            let shard_id = format!("shard_{:04}", i);
            let replicas = vec![self.node_id.clone()]; // Single replica initially
            self.create_shard(shard_id, replicas).await?;
        }
        
        Ok(())
    }
    
    /// Compute hash for a key
    fn compute_key_hash(&self, key: &str) -> u64 {
        match self.config.hash_function {
            HashFunction::Sha256 => {
                let mut hasher = Sha256::new();
                hasher.update(key.as_bytes());
                let result = hasher.finalize();
                u64::from_be_bytes(result[0..8].try_into().unwrap())
            }
            HashFunction::Blake3 => {
                let mut hasher = Blake3Hasher::new();
                hasher.update(key.as_bytes());
                let result = hasher.finalize();
                u64::from_be_bytes(result.as_bytes()[0..8].try_into().unwrap())
            }
            HashFunction::Xxhash => {
                // Simplified - would use actual xxhash crate
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                key.hash(&mut hasher);
                hasher.finish()
            }
        }
    }
    
    /// Check if key is in the specified range
    fn key_in_range(&self, key: &str, range: &(String, String)) -> bool {
        key >= range.0.as_str() && key < range.1.as_str()
    }
    
    /// Calculate key range for a new shard
    async fn calculate_key_range_for_new_shard(&self) -> (String, String) {
        // Find the largest existing shard and split it
        let shards = self.shards.read().await;
        
        if let Some((_, largest_shard)) = shards.iter().max_by_key(|(_, shard)| shard.stats.key_count) {
            // Split the largest shard's range
            let mid_point = self.find_range_midpoint(&largest_shard.key_range);
            (largest_shard.key_range.0.clone(), mid_point)
        } else {
            // First shard covers entire key space
            (String::new(), String::from("~")) // ~ is greater than all other characters
        }
    }
    
    /// Find midpoint of a key range
    fn find_range_midpoint(&self, range: &(String, String)) -> String {
        // Simple midpoint calculation - in practice would be more sophisticated
        if range.0.is_empty() {
            "m".to_string() // Midpoint of empty to ~
        } else {
            format!("{}m", range.0) // Append 'm' to start
        }
    }
    
    /// Initialize shard replicas
    async fn initialize_shard_replicas(&self, _shard_id: &ShardId) -> Result<()> {
        // In a full implementation, would coordinate with replica nodes
        debug!("Initializing shard replicas (simplified)");
        Ok(())
    }
    
    /// Migrate data during shard split
    async fn migrate_shard_data(
        &self,
        _original_shard: &ShardId,
        _shard_1: &ShardId,
        _shard_2: &ShardId,
        _split_key: &str,
    ) -> Result<()> {
        // In a full implementation, would migrate actual data
        debug!("Migrating shard data (simplified)");
        Ok(())
    }
    
    /// Check if two shards are adjacent
    fn shards_are_adjacent(&self, shard_1: &Shard, shard_2: &Shard) -> bool {
        shard_1.key_range.1 == shard_2.key_range.0 || shard_2.key_range.1 == shard_1.key_range.0
    }
    
    /// Merge data from two shards
    async fn merge_shard_data(&self, _shard1: &ShardId, _shard2: &ShardId, _merged_id: &ShardId) -> Result<()> {
        // In a full implementation, would merge actual data
        debug!("Merging shard data (simplified)");
        Ok(())
    }
    
    /// Analyze current load distribution
    async fn analyze_load_distribution(&self) -> HashMap<NodeId, f64> {
        let mut distribution = HashMap::new();
        let shards = self.shards.read().await;
        
        for shard in shards.values() {
            let load = shard.stats.cpu_usage + shard.stats.memory_usage;
            *distribution.entry(shard.primary.clone()).or_insert(0.0) += load;
        }
        
        distribution
    }
    
    /// Calculate load imbalance
    fn calculate_load_imbalance(&self, distribution: &HashMap<NodeId, f64>) -> f64 {
        if distribution.is_empty() {
            return 0.0;
        }
        
        let total_load: f64 = distribution.values().sum();
        let avg_load = total_load / distribution.len() as f64;
        
        let max_load = distribution.values().fold(0.0f64, |a, b| a.max(*b));
        let min_load = distribution.values().fold(f64::MAX, |a, b| a.min(*b));
        
        if avg_load > 0.0 {
            (max_load - min_load) / avg_load
        } else {
            0.0
        }
    }
    
    /// Plan rebalancing movements
    async fn plan_rebalancing_movements(&self, _distribution: &HashMap<NodeId, f64>) -> Vec<(ShardId, NodeId)> {
        // Simplified planning - would use more sophisticated algorithm
        Vec::new()
    }
    
    /// Migrate a shard to a target node
    async fn migrate_shard(&self, _shard_id: &ShardId, _target_node: NodeId) -> Result<()> {
        // In a full implementation, would migrate shard
        debug!("Migrating shard (simplified)");
        Ok(())
    }
    
    /// Find optimal split key for a shard
    async fn find_optimal_split_key(&self, _shard_id: &ShardId) -> Result<String> {
        // In a full implementation, would analyze access patterns
        Ok("optimal_split_key".to_string())
    }
    
    /// Add read replicas for a shard
    async fn add_read_replicas(&self, _shard_id: &ShardId, _count: usize) -> Result<()> {
        // In a full implementation, would add replicas
        debug!("Adding read replicas (simplified)");
        Ok(())
    }
    
    /// Enable caching for a shard
    async fn enable_shard_caching(&self, _shard_id: &ShardId) -> Result<()> {
        // In a full implementation, would enable caching
        debug!("Enabling shard caching (simplified)");
        Ok(())
    }
    
    /// Complete any pending migrations
    async fn complete_pending_migrations(&self) -> Result<()> {
        let mut tracker = self.migration_tracker.lock().await;
        
        // Wait for active migrations to complete
        while !tracker.active_migrations.is_empty() {
            debug!("Waiting for {} migrations to complete", tracker.active_migrations.len());
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        
        info!("All migrations completed");
        Ok(())
    }
    
    /// Start background rebalancing task  
    fn start_rebalancing_task(self: Arc<Self>) {
        let shard_manager = Arc::clone(&self);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // Every 5 minutes
            
            loop {
                interval.tick().await;
                
                // Check if rebalancing is needed
                if let Err(e) = shard_manager.rebalance_shards().await {
                    debug!("Rebalancing check failed: {}", e);
                }
            }
        });
    }
    
    /// Start hot shard detection task
    fn start_hot_shard_detection_task(self: Arc<Self>) {
        let detector = self.hot_shard_detector.clone();
        let shard_manager = Arc::clone(&self);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60)); // Every minute
            
            loop {
                interval.tick().await;
                
                // Detect hot shards
                if let Ok(hot_shards) = detector.detect_hot_shards().await {
                    for shard_id in hot_shards {
                        if let Err(e) = shard_manager.mitigate_hot_shard(shard_id).await {
                            warn!("Hot shard mitigation failed: {}", e);
                        }
                    }
                }
            }
        });
    }
    
    /// Start metrics collection task
    async fn start_metrics_collection_task(&self) {
        let shards = self.shards.clone();
        let metrics = self.metrics.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30)); // Every 30 seconds
            
            loop {
                interval.tick().await;
                
                let shard_count = shards.read().await.len();
                metrics.active_shards.set(shard_count as f64);
            }
        });
    }
}

impl ConsistentHashRing {
    fn new(virtual_nodes: usize, hash_function: HashFunction) -> Self {
        Self {
            ring: BTreeMap::new(),
            virtual_nodes,
            hash_function,
            total_weight: 0.0,
        }
    }
    
    fn add_node(&mut self, node: NodeId, weight: f64) -> Result<()> {
        for i in 0..self.virtual_nodes {
            let virtual_key = format!("{}:{}", node.as_str(), i);
            let hash = self.compute_hash(&virtual_key);
            self.ring.insert(hash, node.clone());
        }
        
        self.total_weight += weight;
        debug!("Added node {:?} with weight {} to hash ring", node, weight);
        Ok(())
    }
    
    fn remove_node(&mut self, node: &NodeId) -> Result<()> {
        let mut to_remove = Vec::new();
        
        for (&hash, ring_node) in &self.ring {
            if ring_node == node {
                to_remove.push(hash);
            }
        }
        
        for hash in to_remove {
            self.ring.remove(&hash);
        }
        
        debug!("Removed node {:?} from hash ring", node);
        Ok(())
    }
    
    fn get_node(&self, hash: u64) -> Option<NodeId> {
        if self.ring.is_empty() {
            return None;
        }
        
        // Find the first node with hash >= target hash
        for (&ring_hash, node) in self.ring.range(hash..) {
            return Some(node.clone());
        }
        
        // Wrap around to the first node
        self.ring.iter().next().map(|(_, node)| node.clone())
    }
    
    fn compute_hash(&self, key: &str) -> u64 {
        match self.hash_function {
            HashFunction::Sha256 => {
                let mut hasher = Sha256::new();
                hasher.update(key.as_bytes());
                let result = hasher.finalize();
                u64::from_be_bytes(result[0..8].try_into().unwrap())
            }
            HashFunction::Blake3 => {
                let mut hasher = Blake3Hasher::new();
                hasher.update(key.as_bytes());
                let result = hasher.finalize();
                u64::from_be_bytes(result.as_bytes()[0..8].try_into().unwrap())
            }
            HashFunction::Xxhash => {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                key.hash(&mut hasher);
                hasher.finish()
            }
        }
    }
}

impl ShardRebalancer {
    fn new(config: ShardingConfig, metrics: Arc<ConsensusMetrics>) -> Self {
        Self {
            state: Arc::new(RwLock::new(RebalancingState::default())),
            config,
            metrics,
        }
    }
}

impl HotShardDetector {
    fn new(config: HotShardConfig) -> Self {
        Self {
            metrics_history: Arc::new(RwLock::new(HashMap::new())),
            config,
            hot_shards: Arc::new(RwLock::new(HashSet::new())),
            detection_state: Arc::new(RwLock::new(DetectionState::default())),
        }
    }
    
    async fn detect_hot_shards(&self) -> Result<Vec<ShardId>> {
        if !self.config.enabled {
            return Ok(Vec::new());
        }
        
        let mut hot_shards = Vec::new();
        let metrics_history = self.metrics_history.read().await;
        
        for (shard_id, history) in metrics_history.iter() {
            if let Some(latest_stats) = history.back() {
                // Check if shard exceeds thresholds
                if latest_stats.cpu_usage > self.config.cpu_threshold ||
                   latest_stats.memory_usage > self.config.memory_threshold ||
                   latest_stats.request_rate > self.config.request_rate_threshold as f64 {
                    hot_shards.push(shard_id.clone());
                }
            }
        }
        
        // Update hot shards set
        {
            let mut hot_set = self.hot_shards.write().await;
            *hot_set = hot_shards.iter().cloned().collect();
        }
        
        Ok(hot_shards)
    }
    
    async fn update_shard_metrics(&self, shard_id: &ShardId, stats: ShardStats) {
        let mut history = self.metrics_history.write().await;
        let shard_history = history.entry(shard_id.clone()).or_insert_with(VecDeque::new);
        
        shard_history.push_back(stats);
        
        // Keep only recent metrics (last 10 minutes)
        while shard_history.len() > 20 { // 20 samples at 30-second intervals
            shard_history.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::storage::MockStorage;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_shard_manager_creation() {
        let temp_dir = tempdir().unwrap();
        let storage_config = crate::config::StorageConfig {
            data_dir: temp_dir.path().to_path_buf(),
            max_versions_per_key: 10,
            gc_interval_seconds: 3600,
            gc_watermark_lag_seconds: 1800,
            version_compression: true,
            rocksdb: crate::config::RocksDBConfig::default(),
            memtable_size_mb: 64,
            write_buffer_size_mb: 32,
        };
        
        let storage = Arc::new(MVCCStorage::new(&storage_config).await.unwrap());
        
        // Mock transport
        struct MockTransport;
        #[async_trait::async_trait]
        impl HyperMeshTransportTrait for MockTransport {
            async fn connect_node(&self, _node_id: NodeId, _endpoint: &hypermesh_transport::Endpoint) 
                -> hypermesh_transport::Result<Arc<hypermesh_transport::Connection>> {
                unimplemented!()
            }
            async fn accept_node(&self) -> hypermesh_transport::Result<Arc<hypermesh_transport::Connection>> {
                unimplemented!()
            }
            async fn send_to(&self, _node_id: &NodeId, _data: &[u8]) -> hypermesh_transport::Result<()> {
                Ok(())
            }
            async fn receive_from(&self, _connection: &hypermesh_transport::Connection) 
                -> hypermesh_transport::Result<bytes::Bytes> {
                Ok(bytes::Bytes::new())
            }
            async fn metrics(&self) -> hypermesh_transport::TransportStats {
                hypermesh_transport::TransportStats {
                    bytes_sent: 0,
                    bytes_received: 0,
                    active_connections: 0,
                    total_connections: 0,
                    throughput_gbps: 0.0,
                    avg_latency_us: 0,
                }
            }
            async fn maintain(&self) -> hypermesh_transport::Result<()> {
                Ok(())
            }
        }
        
        let transport = Arc::new(MockTransport);
        let node_id = NodeId::new("test-node".to_string());
        let config = ShardingConfig::default();
        
        let shard_manager = ShardManager::new(node_id, storage, transport, config).await.unwrap();
        
        // Should have created initial shards
        let shards = shard_manager.get_all_shards().await;
        assert!(!shards.is_empty());
    }
    
    #[tokio::test]
    async fn test_consistent_hash_ring() {
        let mut ring = ConsistentHashRing::new(10, HashFunction::Blake3);
        
        let node1 = NodeId::new("node1".to_string());
        let node2 = NodeId::new("node2".to_string());
        
        ring.add_node(node1.clone(), 1.0).unwrap();
        ring.add_node(node2.clone(), 1.0).unwrap();
        
        // Test key routing
        let key_hash = ring.compute_hash("test_key");
        let selected_node = ring.get_node(key_hash);
        assert!(selected_node.is_some());
        
        // Test node removal
        ring.remove_node(&node1).unwrap();
        let selected_node_after_removal = ring.get_node(key_hash);
        assert!(selected_node_after_removal.is_some());
    }
    
    #[tokio::test]
    async fn test_hot_shard_detection() {
        let config = HotShardConfig {
            enabled: true,
            cpu_threshold: 80.0,
            memory_threshold: 85.0,
            request_rate_threshold: 1000,
            detection_window_minutes: 5,
            mitigation_strategies: vec![MitigationStrategy::Split],
        };
        
        let detector = HotShardDetector::new(config);
        
        // Add metrics for a hot shard
        let shard_id = "hot_shard".to_string();
        let hot_stats = ShardStats {
            key_count: 1000,
            size_bytes: 1024 * 1024,
            request_rate: 1500.0, // Above threshold
            cpu_usage: 90.0, // Above threshold
            memory_usage: 80.0,
            avg_response_time_us: 1000,
            last_updated: SystemTime::now(),
        };
        
        detector.update_shard_metrics(&shard_id, hot_stats).await;
        
        // Detect hot shards
        let hot_shards = detector.detect_hot_shards().await.unwrap();
        assert!(hot_shards.contains(&shard_id));
    }
}