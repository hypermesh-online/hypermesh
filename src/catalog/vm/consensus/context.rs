//! VM Consensus Context - Execution state with consensus awareness
//!
//! This module manages the consensus context for VM operations, tracking
//! the state of ongoing consensus operations and maintaining the temporal
//! and logical consistency required by the four-proof system.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use anyhow::Result;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

use super::{ConsensusOperation, ConsensusExecutionResult};
use crate::catalog::vm::AssetId;

/// VM consensus context managing state across operations
#[derive(Debug)]
pub struct VMConsensusContext {
    /// Context identifier
    id: String,
    /// Current operation being executed
    current_operation: Arc<RwLock<Option<String>>>,
    /// Execution history for temporal consistency
    execution_history: Arc<RwLock<VecDeque<ContextHistoryEntry>>>,
    /// Asset operation tracking
    asset_operations: Arc<RwLock<HashMap<AssetId, Vec<String>>>>,
    /// Temporal state for time proof validation
    temporal_state: Arc<RwLock<TemporalState>>,
    /// Consensus metrics
    metrics: Arc<RwLock<ConsensusMetrics>>,
    /// Context creation timestamp
    created_at: SystemTime,
}

/// Historical entry for context tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextHistoryEntry {
    pub operation_id: String,
    pub operation_type: String,
    pub asset_id: AssetId,
    pub execution_result: ConsensusExecutionResult,
    pub timestamp: SystemTime,
}

/// Temporal state for time proof consistency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalState {
    /// Last logical timestamp processed
    pub last_logical_timestamp: Option<u64>,
    /// Last temporal hash in chain
    pub last_temporal_hash: Option<[u8; 32]>,
    /// Sequence number for this context
    pub sequence_number: u64,
    /// Time drift accumulator
    pub accumulated_time_drift: i64, // microseconds
    /// Clock synchronization state
    pub clock_sync_state: ClockSyncState,
}

/// Clock synchronization state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClockSyncState {
    /// Not synchronized
    NotSynced,
    /// Synchronizing with network time
    Syncing,
    /// Synchronized within acceptable drift
    Synced { drift_micros: i64 },
    /// Drift too large, needs resynchronization
    DriftExceeded { drift_micros: i64 },
}

/// Consensus metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusMetrics {
    /// Total operations processed
    pub total_operations: u64,
    /// Successful operations
    pub successful_operations: u64,
    /// Failed operations
    pub failed_operations: u64,
    /// Average execution time (microseconds)
    pub avg_execution_time_micros: f64,
    /// Proof validation statistics
    pub proof_validation_stats: ProofValidationStats,
    /// Resource utilization metrics
    pub resource_utilization: ResourceUtilizationStats,
}

/// Proof validation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofValidationStats {
    pub space_proof_validations: u64,
    pub space_proof_failures: u64,
    pub stake_proof_validations: u64,
    pub stake_proof_failures: u64,
    pub work_proof_validations: u64,
    pub work_proof_failures: u64,
    pub time_proof_validations: u64,
    pub time_proof_failures: u64,
}

/// Resource utilization statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUtilizationStats {
    pub total_cpu_cycles: u64,
    pub total_memory_bytes: u64,
    pub total_storage_bytes: u64,
    pub total_network_bytes: u64,
    pub peak_concurrent_operations: u32,
}

impl VMConsensusContext {
    /// Create new consensus context
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            current_operation: Arc::new(RwLock::new(None)),
            execution_history: Arc::new(RwLock::new(VecDeque::new())),
            asset_operations: Arc::new(RwLock::new(HashMap::new())),
            temporal_state: Arc::new(RwLock::new(TemporalState::new())),
            metrics: Arc::new(RwLock::new(ConsensusMetrics::new())),
            created_at: SystemTime::now(),
        }
    }
    
    /// Add operation to context
    pub async fn add_operation(&self, operation: &ConsensusOperation) -> Result<()> {
        // Set current operation
        {
            let mut current = self.current_operation.write().unwrap();
            *current = Some(operation.id().to_string());
        }
        
        // Track asset operations
        {
            let mut asset_ops = self.asset_operations.write().unwrap();
            asset_ops.entry(operation.asset_id())
                .or_insert_with(Vec::new)
                .push(operation.id().to_string());
        }
        
        // Update temporal state with time proof
        {
            let mut temporal = self.temporal_state.write().unwrap();
            let time_proof = &operation.consensus_proof().proof_of_time;
            
            // Update logical timestamp
            if temporal.last_logical_timestamp.is_none() || 
               time_proof.logical_timestamp > temporal.last_logical_timestamp.unwrap_or(0) {
                temporal.last_logical_timestamp = Some(time_proof.logical_timestamp);
            }
            
            // Update temporal hash chain
            temporal.last_temporal_hash = Some(time_proof.temporal_hash);
            
            // Increment sequence number
            temporal.sequence_number += 1;
            
            // Update clock synchronization state
            self.update_clock_sync_state(&mut temporal, time_proof).await?;
        }
        
        Ok(())
    }
    
    /// Update context with execution result
    pub async fn update_with_result(&self, result: &ConsensusExecutionResult) -> Result<()> {
        // Add to history
        {
            let mut history = self.execution_history.write().unwrap();
            
            // Find the operation in current operations to get details
            let entry = ContextHistoryEntry {
                operation_id: result.operation_id.clone(),
                operation_type: "unknown".to_string(), // Would be retrieved from operation
                asset_id: Uuid::new_v4(), // Would be retrieved from operation
                execution_result: result.clone(),
                timestamp: SystemTime::now(),
            };
            
            history.push_back(entry);
            
            // Limit history size
            if history.len() > 1000 {
                history.pop_front();
            }
        }
        
        // Update metrics
        {
            let mut metrics = self.metrics.write().unwrap();
            
            metrics.total_operations += 1;
            if result.success {
                metrics.successful_operations += 1;
            } else {
                metrics.failed_operations += 1;
            }
            
            // Update average execution time
            let execution_time = result.resource_usage.execution_duration_micros;
            let total_time = metrics.avg_execution_time_micros * (metrics.total_operations - 1) as f64;
            metrics.avg_execution_time_micros = (total_time + execution_time as f64) / metrics.total_operations as f64;
            
            // Update proof validation stats
            self.update_proof_stats(&mut metrics.proof_validation_stats, &result.proof_validations);
            
            // Update resource utilization
            self.update_resource_stats(&mut metrics.resource_utilization, &result.resource_usage);
        }
        
        // Clear current operation
        {
            let mut current = self.current_operation.write().unwrap();
            *current = None;
        }
        
        Ok(())
    }
    
    /// Get current operation type
    pub fn current_operation_type(&self) -> Option<String> {
        // In a real implementation, this would retrieve the operation type
        // from the current operation. For now, return a default.
        let current = self.current_operation.read().unwrap();
        if current.is_some() {
            Some("generic".to_string())
        } else {
            None
        }
    }
    
    /// Get last logical timestamp
    pub fn last_logical_timestamp(&self) -> Option<u64> {
        let temporal = self.temporal_state.read().unwrap();
        temporal.last_logical_timestamp
    }
    
    /// Get last temporal hash
    pub fn last_temporal_hash(&self) -> Option<[u8; 32]> {
        let temporal = self.temporal_state.read().unwrap();
        temporal.last_temporal_hash
    }
    
    /// Get context ID
    pub fn id(&self) -> &str {
        &self.id
    }
    
    /// Get execution history
    pub fn get_execution_history(&self) -> Vec<ContextHistoryEntry> {
        let history = self.execution_history.read().unwrap();
        history.iter().cloned().collect()
    }
    
    /// Get operations for specific asset
    pub fn get_asset_operations(&self, asset_id: AssetId) -> Vec<String> {
        let asset_ops = self.asset_operations.read().unwrap();
        asset_ops.get(&asset_id).cloned().unwrap_or_default()
    }
    
    /// Get temporal state
    pub fn get_temporal_state(&self) -> TemporalState {
        let temporal = self.temporal_state.read().unwrap();
        temporal.clone()
    }
    
    /// Get consensus metrics
    pub fn get_metrics(&self) -> ConsensusMetrics {
        let metrics = self.metrics.read().unwrap();
        metrics.clone()
    }
    
    /// Update clock synchronization state
    async fn update_clock_sync_state(
        &self,
        temporal: &mut TemporalState,
        time_proof: &crate::consensus::ProofOfTime,
    ) -> Result<()> {
        let current_time = SystemTime::now();
        let proof_time = time_proof.wall_clock;
        
        // Calculate time drift
        let drift_micros = current_time.duration_since(proof_time)
            .map(|d| d.as_micros() as i64)
            .unwrap_or_else(|_| -(proof_time.duration_since(current_time)
                .unwrap_or_default()
                .as_micros() as i64));
        
        // Update accumulated drift
        temporal.accumulated_time_drift = (temporal.accumulated_time_drift + drift_micros) / 2;
        
        // Update sync state
        temporal.clock_sync_state = if drift_micros.abs() > 5_000_000 { // 5 seconds
            ClockSyncState::DriftExceeded { drift_micros }
        } else if drift_micros.abs() > 1_000_000 { // 1 second
            ClockSyncState::Syncing
        } else {
            ClockSyncState::Synced { drift_micros }
        };
        
        Ok(())
    }
    
    /// Update proof validation statistics
    fn update_proof_stats(&self, stats: &mut ProofValidationStats, validations: &super::ProofValidationResults) {
        // Space proof stats
        stats.space_proof_validations += 1;
        if !validations.space_proof_valid {
            stats.space_proof_failures += 1;
        }
        
        // Stake proof stats
        stats.stake_proof_validations += 1;
        if !validations.stake_proof_valid {
            stats.stake_proof_failures += 1;
        }
        
        // Work proof stats
        stats.work_proof_validations += 1;
        if !validations.work_proof_valid {
            stats.work_proof_failures += 1;
        }
        
        // Time proof stats
        stats.time_proof_validations += 1;
        if !validations.time_proof_valid {
            stats.time_proof_failures += 1;
        }
    }
    
    /// Update resource utilization statistics
    fn update_resource_stats(&self, stats: &mut ResourceUtilizationStats, usage: &super::ResourceUsageMetrics) {
        stats.total_cpu_cycles += usage.cpu_cycles;
        stats.total_memory_bytes += usage.memory_bytes;
        stats.total_storage_bytes += usage.storage_bytes;
        stats.total_network_bytes += usage.network_bytes;
        
        // Update peak concurrent operations (simplified)
        let current_ops = self.current_operation.read().unwrap();
        if current_ops.is_some() {
            stats.peak_concurrent_operations = stats.peak_concurrent_operations.max(1);
        }
    }
    
    /// Reset context (for testing or recovery)
    pub fn reset(&self) {
        {
            let mut current = self.current_operation.write().unwrap();
            *current = None;
        }
        
        {
            let mut history = self.execution_history.write().unwrap();
            history.clear();
        }
        
        {
            let mut asset_ops = self.asset_operations.write().unwrap();
            asset_ops.clear();
        }
        
        {
            let mut temporal = self.temporal_state.write().unwrap();
            *temporal = TemporalState::new();
        }
        
        {
            let mut metrics = self.metrics.write().unwrap();
            *metrics = ConsensusMetrics::new();
        }
    }
    
    /// Check if context is healthy (no excessive drift, reasonable success rate)
    pub fn is_healthy(&self) -> bool {
        let temporal = self.temporal_state.read().unwrap();
        let metrics = self.metrics.read().unwrap();
        
        // Check clock sync state
        let clock_healthy = match &temporal.clock_sync_state {
            ClockSyncState::Synced { .. } => true,
            ClockSyncState::Syncing => true,
            _ => false,
        };
        
        // Check success rate
        let success_rate = if metrics.total_operations > 0 {
            metrics.successful_operations as f64 / metrics.total_operations as f64
        } else {
            1.0
        };
        
        clock_healthy && success_rate > 0.8 // 80% success rate threshold
    }
}

impl TemporalState {
    /// Create new temporal state
    pub fn new() -> Self {
        Self {
            last_logical_timestamp: None,
            last_temporal_hash: None,
            sequence_number: 0,
            accumulated_time_drift: 0,
            clock_sync_state: ClockSyncState::NotSynced,
        }
    }
    
    /// Get next logical timestamp
    pub fn next_logical_timestamp(&self) -> u64 {
        self.last_logical_timestamp.unwrap_or(0) + 1
    }
    
    /// Check if temporal state is consistent
    pub fn is_consistent(&self) -> bool {
        // Basic consistency checks
        self.sequence_number > 0 &&
        self.accumulated_time_drift.abs() < 10_000_000 && // 10 seconds max drift
        matches!(self.clock_sync_state, ClockSyncState::Synced { .. } | ClockSyncState::Syncing)
    }
}

impl ConsensusMetrics {
    /// Create new metrics
    pub fn new() -> Self {
        Self {
            total_operations: 0,
            successful_operations: 0,
            failed_operations: 0,
            avg_execution_time_micros: 0.0,
            proof_validation_stats: ProofValidationStats::new(),
            resource_utilization: ResourceUtilizationStats::new(),
        }
    }
    
    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_operations > 0 {
            self.successful_operations as f64 / self.total_operations as f64
        } else {
            0.0
        }
    }
    
    /// Get average operations per second
    pub fn operations_per_second(&self, duration_seconds: f64) -> f64 {
        if duration_seconds > 0.0 {
            self.total_operations as f64 / duration_seconds
        } else {
            0.0
        }
    }
}

impl ProofValidationStats {
    /// Create new proof validation stats
    pub fn new() -> Self {
        Self {
            space_proof_validations: 0,
            space_proof_failures: 0,
            stake_proof_validations: 0,
            stake_proof_failures: 0,
            work_proof_validations: 0,
            work_proof_failures: 0,
            time_proof_validations: 0,
            time_proof_failures: 0,
        }
    }
    
    /// Get space proof success rate
    pub fn space_proof_success_rate(&self) -> f64 {
        if self.space_proof_validations > 0 {
            (self.space_proof_validations - self.space_proof_failures) as f64 / 
                self.space_proof_validations as f64
        } else {
            0.0
        }
    }
    
    /// Get overall proof validation success rate
    pub fn overall_success_rate(&self) -> f64 {
        let total_validations = self.space_proof_validations + 
                               self.stake_proof_validations + 
                               self.work_proof_validations + 
                               self.time_proof_validations;
        
        let total_failures = self.space_proof_failures + 
                            self.stake_proof_failures + 
                            self.work_proof_failures + 
                            self.time_proof_failures;
        
        if total_validations > 0 {
            (total_validations - total_failures) as f64 / total_validations as f64
        } else {
            0.0
        }
    }
}

impl ResourceUtilizationStats {
    /// Create new resource utilization stats
    pub fn new() -> Self {
        Self {
            total_cpu_cycles: 0,
            total_memory_bytes: 0,
            total_storage_bytes: 0,
            total_network_bytes: 0,
            peak_concurrent_operations: 0,
        }
    }
    
    /// Get total resource usage (simplified metric)
    pub fn total_resource_units(&self) -> u64 {
        self.total_cpu_cycles + 
        self.total_memory_bytes / 1024 + // Convert to KB for normalization
        self.total_storage_bytes / (1024 * 1024) + // Convert to MB
        self.total_network_bytes / 1024 // Convert to KB
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::proof::{ProofOfSpace, ProofOfStake, ProofOfWork, ProofOfTime, ConsensusProof};
    
    #[tokio::test]
    async fn test_context_creation() {
        let context = VMConsensusContext::new();
        assert!(!context.id().is_empty());
        assert!(context.current_operation_type().is_none());
    }
    
    #[tokio::test]
    async fn test_operation_tracking() {
        let context = VMConsensusContext::new();
        let asset_id = Uuid::new_v4();
        
        // Create mock consensus operation
        let consensus_proof = ConsensusProof::new(
            ProofOfSpace::default(),
            ProofOfStake::default(),
            ProofOfWork::default(),
            ProofOfTime::default(),
        );
        
        let operation = super::ConsensusOperation::new(
            "test".to_string(),
            asset_id,
            consensus_proof,
            Arc::new(context.clone()),
        );
        
        // Add operation to context
        let result = context.add_operation(&operation).await;
        assert!(result.is_ok());
        
        // Check asset operations tracking
        let asset_ops = context.get_asset_operations(asset_id);
        assert_eq!(asset_ops.len(), 1);
    }
    
    #[test]
    fn test_temporal_state() {
        let mut state = TemporalState::new();
        assert_eq!(state.next_logical_timestamp(), 1);
        
        state.last_logical_timestamp = Some(10);
        assert_eq!(state.next_logical_timestamp(), 11);
    }
    
    #[test]
    fn test_metrics() {
        let metrics = ConsensusMetrics::new();
        assert_eq!(metrics.success_rate(), 0.0);
        assert_eq!(metrics.operations_per_second(0.0), 0.0);
    }
}