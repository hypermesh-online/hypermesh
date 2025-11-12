//! Node isolation and quarantine management
//!
//! This module provides automatic isolation of malicious nodes with graduated
//! response levels, recovery mechanisms, and network protection.

use super::super::error::{ConsensusError, ConsensusResult};
use crate::transport::NodeId;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime, Instant};
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use tracing::{info, warn, error, debug};

/// Configuration for node isolation system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolationConfig {
    /// Reputation threshold below which nodes are isolated
    pub isolation_threshold: f64,
    
    /// Temporary isolation duration
    pub temporary_isolation_duration: Duration,
    
    /// Escalation threshold for permanent isolation
    pub permanent_isolation_threshold: usize,
    
    /// Grace period for new nodes before isolation can occur
    pub new_node_grace_period: Duration,
    
    /// Maximum nodes that can be isolated simultaneously
    pub max_isolated_nodes: usize,
    
    /// Enable automatic network partition prevention
    pub prevent_network_partition: bool,
    
    /// Minimum healthy nodes required to maintain consensus
    pub min_healthy_nodes: usize,
    
    /// Recovery monitoring interval
    pub recovery_monitoring_interval: Duration,
}

impl Default for IsolationConfig {
    fn default() -> Self {
        Self {
            isolation_threshold: 0.3,
            temporary_isolation_duration: Duration::from_secs(3600), // 1 hour
            permanent_isolation_threshold: 5,
            new_node_grace_period: Duration::from_secs(300), // 5 minutes
            max_isolated_nodes: 10,
            prevent_network_partition: true,
            min_healthy_nodes: 3,
            recovery_monitoring_interval: Duration::from_secs(60),
        }
    }
}

/// Reasons for node isolation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IsolationReason {
    /// Low reputation score
    LowReputation(f64),
    
    /// Byzantine behavior detected
    ByzantineBehavior(Vec<u8>),
    
    /// Consensus protocol violations
    ProtocolViolation(String),
    
    /// Network attacks (flooding, etc.)
    NetworkAttack(String),
    
    /// Invalid cryptographic signatures
    InvalidSignatures(usize),
    
    /// Manual isolation by administrator
    Manual(String),
    
    /// Coordinated attack participation
    CoordinatedAttack(Vec<NodeId>),
    
    /// Repeated violations after warnings
    RepeatedViolations(usize),
}

/// Node isolation status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IsolationStatus {
    /// Node is not isolated
    NotIsolated,
    
    /// Temporarily isolated
    TemporaryIsolation {
        /// When isolation started
        started_at: SystemTime,
        /// When isolation expires
        expires_at: SystemTime,
        /// Reason for isolation
        reason: IsolationReason,
    },
    
    /// Permanently isolated
    PermanentIsolation {
        /// When isolation started
        started_at: SystemTime,
        /// Reason for isolation
        reason: IsolationReason,
        /// Number of previous violations
        violation_count: usize,
    },
    
    /// Under review for potential isolation
    UnderReview {
        /// Review started timestamp
        review_started: SystemTime,
        /// Suspected issues
        suspected_issues: Vec<IsolationReason>,
    },
}

/// Isolation level with graduated response
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum IsolationLevel {
    /// Warning level - increased monitoring
    Warning,
    
    /// Limited isolation - restricted participation
    Limited,
    
    /// Temporary isolation - full temporary exclusion
    Temporary,
    
    /// Permanent isolation - permanent exclusion
    Permanent,
}

/// Node isolation record
#[derive(Debug, Clone)]
pub struct IsolationRecord {
    /// Node ID
    pub node_id: NodeId,
    
    /// Current isolation status
    pub status: IsolationStatus,
    
    /// Isolation level
    pub level: IsolationLevel,
    
    /// History of isolation events
    pub isolation_history: Vec<IsolationEvent>,
    
    /// First seen timestamp
    pub first_seen: SystemTime,
    
    /// Last activity timestamp
    pub last_activity: SystemTime,
    
    /// Recovery attempts made
    pub recovery_attempts: usize,
    
    /// Whether node is eligible for recovery
    pub eligible_for_recovery: bool,
}

/// Individual isolation event
#[derive(Debug, Clone)]
pub struct IsolationEvent {
    /// Event timestamp
    pub timestamp: SystemTime,
    
    /// Type of event
    pub event_type: IsolationEventType,
    
    /// Reason for the event
    pub reason: IsolationReason,
    
    /// Duration (for temporary isolations)
    pub duration: Option<Duration>,
    
    /// Additional context
    pub context: HashMap<String, String>,
}

/// Types of isolation events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IsolationEventType {
    /// Isolation was imposed
    Isolated,
    
    /// Isolation was lifted
    Released,
    
    /// Isolation was escalated
    Escalated,
    
    /// Recovery attempt was made
    RecoveryAttempt,
    
    /// Manual intervention occurred
    ManualIntervention,
}

/// Network health metrics for isolation decisions
#[derive(Debug, Clone)]
pub struct NetworkHealth {
    /// Total known nodes
    pub total_nodes: usize,
    
    /// Healthy nodes count
    pub healthy_nodes: usize,
    
    /// Temporarily isolated nodes
    pub temporarily_isolated: usize,
    
    /// Permanently isolated nodes
    pub permanently_isolated: usize,
    
    /// Current Byzantine tolerance ratio
    pub byzantine_tolerance_ratio: f64,
    
    /// Network partition risk level
    pub partition_risk: PartitionRisk,
}

/// Risk levels for network partition
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartitionRisk {
    Low,
    Medium,
    High,
    Critical,
}

/// Node isolation and quarantine manager
pub struct NodeIsolationManager {
    /// Configuration
    config: IsolationConfig,
    
    /// Isolated nodes tracking
    isolated_nodes: Arc<RwLock<HashMap<NodeId, IsolationRecord>>>,
    
    /// Network health metrics
    network_health: Arc<RwLock<NetworkHealth>>,
    
    /// Active isolation operations
    active_operations: Arc<RwLock<HashSet<NodeId>>>,
    
    /// Isolation metrics
    metrics: Arc<RwLock<IsolationMetrics>>,
    
    /// Recovery monitoring task handle
    recovery_task_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

/// Isolation system metrics
#[derive(Debug, Clone)]
pub struct IsolationMetrics {
    /// Total isolations performed
    pub total_isolations: u64,
    
    /// Temporary isolations
    pub temporary_isolations: u64,
    
    /// Permanent isolations
    pub permanent_isolations: u64,
    
    /// Successful recoveries
    pub successful_recoveries: u64,
    
    /// Failed recovery attempts
    pub failed_recoveries: u64,
    
    /// Current isolation rate
    pub current_isolation_rate: f64,
    
    /// Average isolation duration
    pub avg_isolation_duration: Duration,
    
    /// Last metrics update
    pub last_updated: Instant,
}

impl Default for IsolationMetrics {
    fn default() -> Self {
        Self {
            total_isolations: 0,
            temporary_isolations: 0,
            permanent_isolations: 0,
            successful_recoveries: 0,
            failed_recoveries: 0,
            current_isolation_rate: 0.0,
            avg_isolation_duration: Duration::ZERO,
            last_updated: Instant::now(),
        }
    }
}

impl NodeIsolationManager {
    /// Create a new node isolation manager
    pub async fn new(isolation_threshold: f64) -> ConsensusResult<Self> {
        let config = IsolationConfig {
            isolation_threshold,
            ..Default::default()
        };
        
        Ok(Self {
            config,
            isolated_nodes: Arc::new(RwLock::new(HashMap::new())),
            network_health: Arc::new(RwLock::new(NetworkHealth {
                total_nodes: 0,
                healthy_nodes: 0,
                temporarily_isolated: 0,
                permanently_isolated: 0,
                byzantine_tolerance_ratio: 0.33,
                partition_risk: PartitionRisk::Low,
            })),
            active_operations: Arc::new(RwLock::new(HashSet::new())),
            metrics: Arc::new(RwLock::new(IsolationMetrics::default())),
            recovery_task_handle: Arc::new(RwLock::new(None)),
        })
    }
    
    /// Start the isolation manager
    pub async fn start(&self) -> ConsensusResult<()> {
        info!("Starting node isolation manager");
        
        // Start recovery monitoring task
        self.start_recovery_monitoring().await?;
        
        info!("Node isolation manager started");
        Ok(())
    }
    
    /// Stop the isolation manager
    pub async fn stop(&self) -> ConsensusResult<()> {
        info!("Stopping node isolation manager");
        
        // Stop recovery monitoring task
        if let Some(handle) = self.recovery_task_handle.write().await.take() {
            handle.abort();
        }
        
        info!("Node isolation manager stopped");
        Ok(())
    }
    
    /// Isolate a node
    pub async fn isolate_node(
        &self,
        node_id: NodeId,
        reason: IsolationReason,
    ) -> ConsensusResult<IsolationStatus> {
        info!("Isolating node {:?} for reason: {:?}", node_id, reason);
        
        // Check if operation is already in progress
        {
            let mut active = self.active_operations.write().await;
            if active.contains(&node_id) {
                return Err(ConsensusError::IsolationError(
                    "Isolation operation already in progress".to_string()
                ));
            }
            active.insert(node_id.clone());
        }
        
        let isolation_result = self.perform_isolation(node_id.clone(), reason).await;
        
        // Remove from active operations
        {
            let mut active = self.active_operations.write().await;
            active.remove(&node_id);
        }
        
        isolation_result
    }
    
    /// Release a node from isolation
    pub async fn release_node(&self, node_id: NodeId) -> ConsensusResult<bool> {
        info!("Releasing node {:?} from isolation", node_id);
        
        let mut isolated_nodes = self.isolated_nodes.write().await;
        
        if let Some(record) = isolated_nodes.get_mut(&node_id) {
            match &record.status {
                IsolationStatus::TemporaryIsolation { .. } |
                IsolationStatus::PermanentIsolation { .. } => {
                    // Create release event
                    let release_event = IsolationEvent {
                        timestamp: SystemTime::now(),
                        event_type: IsolationEventType::Released,
                        reason: IsolationReason::Manual("Administrative release".to_string()),
                        duration: None,
                        context: HashMap::new(),
                    };
                    
                    record.isolation_history.push(release_event);
                    record.status = IsolationStatus::NotIsolated;
                    record.eligible_for_recovery = true;
                    
                    // Update metrics
                    self.update_metrics_for_release().await;
                    
                    // Update network health
                    self.update_network_health().await?;
                    
                    info!("Node {:?} released from isolation", node_id);
                    return Ok(true);
                }
                _ => {
                    debug!("Node {:?} is not currently isolated", node_id);
                    return Ok(false);
                }
            }
        }
        
        debug!("Node {:?} not found in isolation records", node_id);
        Ok(false)
    }
    
    /// Check if a node is isolated
    pub async fn is_isolated(&self, node_id: &NodeId) -> ConsensusResult<bool> {
        let isolated_nodes = self.isolated_nodes.read().await;
        
        if let Some(record) = isolated_nodes.get(node_id) {
            match &record.status {
                IsolationStatus::NotIsolated => Ok(false),
                IsolationStatus::TemporaryIsolation { expires_at, .. } => {
                    Ok(SystemTime::now() < *expires_at)
                }
                IsolationStatus::PermanentIsolation { .. } => Ok(true),
                IsolationStatus::UnderReview { .. } => Ok(false), // Under review is not isolated
            }
        } else {
            Ok(false)
        }
    }
    
    /// Get isolation status for a node
    pub async fn get_isolation_status(&self, node_id: &NodeId) -> Option<IsolationStatus> {
        let isolated_nodes = self.isolated_nodes.read().await;
        isolated_nodes.get(node_id).map(|record| record.status.clone())
    }
    
    /// Get list of all isolated nodes
    pub async fn get_isolated_nodes(&self) -> ConsensusResult<Vec<NodeId>> {
        let isolated_nodes = self.isolated_nodes.read().await;
        let mut result = Vec::new();
        
        for (node_id, record) in isolated_nodes.iter() {
            match &record.status {
                IsolationStatus::TemporaryIsolation { expires_at, .. } => {
                    if SystemTime::now() < *expires_at {
                        result.push(node_id.clone());
                    }
                }
                IsolationStatus::PermanentIsolation { .. } => {
                    result.push(node_id.clone());
                }
                _ => {}
            }
        }
        
        Ok(result)
    }
    
    /// Get count of isolated nodes
    pub async fn get_isolated_count(&self) -> ConsensusResult<usize> {
        self.get_isolated_nodes().await.map(|nodes| nodes.len())
    }
    
    /// Check network health
    pub async fn check_network_health(&self) -> NetworkHealth {
        self.network_health.read().await.clone()
    }
    
    /// Get isolation metrics
    pub async fn get_metrics(&self) -> IsolationMetrics {
        self.metrics.read().await.clone()
    }
    
    /// Perform the actual isolation
    async fn perform_isolation(
        &self,
        node_id: NodeId,
        reason: IsolationReason,
    ) -> ConsensusResult<IsolationStatus> {
        // Check network health before isolation
        let can_isolate = self.can_safely_isolate(&node_id).await?;
        if !can_isolate {
            warn!("Cannot safely isolate node {:?} - would risk network partition", node_id);
            return Err(ConsensusError::IsolationError(
                "Isolation would risk network partition".to_string()
            ));
        }
        
        let mut isolated_nodes = self.isolated_nodes.write().await;
        
        // Get or create isolation record
        let record = isolated_nodes.entry(node_id.clone())
            .or_insert_with(|| IsolationRecord {
                node_id: node_id.clone(),
                status: IsolationStatus::NotIsolated,
                level: IsolationLevel::Warning,
                isolation_history: Vec::new(),
                first_seen: SystemTime::now(),
                last_activity: SystemTime::now(),
                recovery_attempts: 0,
                eligible_for_recovery: false,
            });
        
        // Determine isolation level based on history and reason
        let isolation_level = self.determine_isolation_level(record, &reason);
        
        // Create isolation event
        let isolation_event = IsolationEvent {
            timestamp: SystemTime::now(),
            event_type: IsolationEventType::Isolated,
            reason: reason.clone(),
            duration: if isolation_level == IsolationLevel::Temporary {
                Some(self.config.temporary_isolation_duration)
            } else {
                None
            },
            context: HashMap::new(),
        };
        
        // Update record
        record.isolation_history.push(isolation_event);
        record.level = isolation_level.clone();
        record.eligible_for_recovery = isolation_level != IsolationLevel::Permanent;
        
        // Set isolation status
        let status = match isolation_level {
            IsolationLevel::Temporary => {
                let expires_at = SystemTime::now() + self.config.temporary_isolation_duration;
                IsolationStatus::TemporaryIsolation {
                    started_at: SystemTime::now(),
                    expires_at,
                    reason,
                }
            }
            IsolationLevel::Permanent => {
                IsolationStatus::PermanentIsolation {
                    started_at: SystemTime::now(),
                    reason,
                    violation_count: record.isolation_history.len(),
                }
            }
            _ => {
                // For Warning and Limited levels, we don't fully isolate
                IsolationStatus::UnderReview {
                    review_started: SystemTime::now(),
                    suspected_issues: vec![reason],
                }
            }
        };
        
        record.status = status.clone();
        
        // Update metrics
        self.update_metrics_for_isolation(&isolation_level).await;
        
        // Update network health
        drop(isolated_nodes); // Release lock before calling update_network_health
        self.update_network_health().await?;
        
        info!("Node {:?} isolated with level {:?}", node_id, isolation_level);
        Ok(status)
    }
    
    /// Determine appropriate isolation level
    fn determine_isolation_level(
        &self,
        record: &IsolationRecord,
        reason: &IsolationReason,
    ) -> IsolationLevel {
        // Count previous isolation events
        let isolation_count = record.isolation_history.iter()
            .filter(|event| event.event_type == IsolationEventType::Isolated)
            .count();
        
        // Determine severity of current reason
        let reason_severity = match reason {
            IsolationReason::LowReputation(score) => {
                if *score < 0.1 { 3 } else if *score < 0.2 { 2 } else { 1 }
            }
            IsolationReason::ByzantineBehavior(_) => 4,
            IsolationReason::CoordinatedAttack(_) => 5,
            IsolationReason::NetworkAttack(_) => 3,
            IsolationReason::InvalidSignatures(count) => {
                if *count > 10 { 4 } else if *count > 5 { 3 } else { 2 }
            }
            IsolationReason::RepeatedViolations(count) => {
                if *count > 5 { 5 } else { 3 }
            }
            _ => 2,
        };
        
        // Determine isolation level
        if isolation_count >= self.config.permanent_isolation_threshold || reason_severity >= 5 {
            IsolationLevel::Permanent
        } else if isolation_count >= 2 || reason_severity >= 3 {
            IsolationLevel::Temporary
        } else if reason_severity >= 2 {
            IsolationLevel::Limited
        } else {
            IsolationLevel::Warning
        }
    }
    
    /// Check if we can safely isolate a node without risking network partition
    async fn can_safely_isolate(&self, node_id: &NodeId) -> ConsensusResult<bool> {
        if !self.config.prevent_network_partition {
            return Ok(true);
        }
        
        let network_health = self.network_health.read().await;
        
        // Calculate what the network would look like after isolation
        let remaining_healthy = network_health.healthy_nodes.saturating_sub(1);
        
        // Check if we'd have enough nodes for consensus
        if remaining_healthy < self.config.min_healthy_nodes {
            return Ok(false);
        }
        
        // Check Byzantine tolerance
        let total_after_isolation = network_health.total_nodes;
        let isolated_after = network_health.temporarily_isolated + network_health.permanently_isolated + 1;
        let byzantine_ratio = isolated_after as f64 / total_after_isolation as f64;
        
        if byzantine_ratio > network_health.byzantine_tolerance_ratio {
            return Ok(false);
        }
        
        Ok(true)
    }
    
    /// Update network health metrics
    async fn update_network_health(&self) -> ConsensusResult<()> {
        let isolated_nodes = self.isolated_nodes.read().await;
        let mut health = self.network_health.write().await;
        
        let mut temp_isolated = 0;
        let mut perm_isolated = 0;
        
        for record in isolated_nodes.values() {
            match &record.status {
                IsolationStatus::TemporaryIsolation { expires_at, .. } => {
                    if SystemTime::now() < *expires_at {
                        temp_isolated += 1;
                    }
                }
                IsolationStatus::PermanentIsolation { .. } => {
                    perm_isolated += 1;
                }
                _ => {}
            }
        }
        
        health.temporarily_isolated = temp_isolated;
        health.permanently_isolated = perm_isolated;
        health.healthy_nodes = health.total_nodes.saturating_sub(temp_isolated + perm_isolated);
        
        // Calculate partition risk
        let isolation_ratio = (temp_isolated + perm_isolated) as f64 / health.total_nodes as f64;
        health.partition_risk = if isolation_ratio > 0.4 {
            PartitionRisk::Critical
        } else if isolation_ratio > 0.3 {
            PartitionRisk::High
        } else if isolation_ratio > 0.2 {
            PartitionRisk::Medium
        } else {
            PartitionRisk::Low
        };
        
        Ok(())
    }
    
    /// Update metrics for isolation
    async fn update_metrics_for_isolation(&self, level: &IsolationLevel) {
        let mut metrics = self.metrics.write().await;
        
        metrics.total_isolations += 1;
        
        match level {
            IsolationLevel::Temporary => metrics.temporary_isolations += 1,
            IsolationLevel::Permanent => metrics.permanent_isolations += 1,
            _ => {}
        }
        
        metrics.last_updated = Instant::now();
    }
    
    /// Update metrics for release
    async fn update_metrics_for_release(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.successful_recoveries += 1;
        metrics.last_updated = Instant::now();
    }
    
    /// Start recovery monitoring task
    async fn start_recovery_monitoring(&self) -> ConsensusResult<()> {
        let isolated_nodes = self.isolated_nodes.clone();
        let config = self.config.clone();
        let network_health = self.network_health.clone();
        
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.recovery_monitoring_interval);
            
            loop {
                interval.tick().await;
                
                // Check for expired temporary isolations
                let mut expired_nodes = Vec::new();
                {
                    let mut nodes = isolated_nodes.write().await;
                    let now = SystemTime::now();
                    
                    for (node_id, record) in nodes.iter_mut() {
                        if let IsolationStatus::TemporaryIsolation { expires_at, .. } = &record.status {
                            if now >= *expires_at {
                                expired_nodes.push(node_id.clone());
                                record.status = IsolationStatus::NotIsolated;
                                record.eligible_for_recovery = true;
                                
                                let release_event = IsolationEvent {
                                    timestamp: now,
                                    event_type: IsolationEventType::Released,
                                    reason: IsolationReason::Manual("Timeout expired".to_string()),
                                    duration: None,
                                    context: HashMap::new(),
                                };
                                record.isolation_history.push(release_event);
                            }
                        }
                    }
                }
                
                // Log released nodes
                for node_id in expired_nodes {
                    info!("Temporary isolation expired for node {:?}", node_id);
                }
            }
        });
        
        *self.recovery_task_handle.write().await = Some(handle);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_isolation_manager_creation() {
        let manager = NodeIsolationManager::new(0.3).await;
        assert!(manager.is_ok());
    }
    
    #[tokio::test]
    async fn test_node_isolation() {
        let manager = NodeIsolationManager::new(0.3).await.unwrap();
        let node_id = NodeId::new("test-node".to_string());
        
        let status = manager.isolate_node(
            node_id.clone(),
            IsolationReason::LowReputation(0.1),
        ).await.unwrap();
        
        assert!(matches!(status, IsolationStatus::TemporaryIsolation { .. }));
        assert!(manager.is_isolated(&node_id).await.unwrap());
    }
    
    #[tokio::test]
    async fn test_node_release() {
        let manager = NodeIsolationManager::new(0.3).await.unwrap();
        let node_id = NodeId::new("test-node".to_string());
        
        // First isolate
        manager.isolate_node(
            node_id.clone(),
            IsolationReason::LowReputation(0.1),
        ).await.unwrap();
        
        // Then release
        let released = manager.release_node(node_id.clone()).await.unwrap();
        assert!(released);
        assert!(!manager.is_isolated(&node_id).await.unwrap());
    }
}