//! Node reputation management for Byzantine fault detection
//!
//! This module implements comprehensive reputation tracking, consensus-based
//! validation, and time-based decay mechanisms for Byzantine fault detection.

use super::super::error::{ConsensusError, ConsensusResult};
use crate::transport::NodeId;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime, Instant};
use tokio::sync::{RwLock, mpsc};
use serde::{Serialize, Deserialize};
use tracing::{info, warn, error, debug};

/// Configuration for reputation management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationConfig {
    /// Initial reputation score for new nodes
    pub initial_reputation: f64,
    
    /// Maximum reputation score
    pub max_reputation: f64,
    
    /// Minimum reputation score
    pub min_reputation: f64,
    
    /// Reputation decay rate per hour
    pub decay_rate_per_hour: f64,
    
    /// Threshold for quarantine
    pub quarantine_threshold: f64,
    
    /// Threshold for isolation
    pub isolation_threshold: f64,
    
    /// Number of validators required for consensus
    pub consensus_validators: usize,
    
    /// Consensus threshold (percentage of validators that must agree)
    pub consensus_threshold: f64,
    
    /// Time window for reputation events
    pub event_window: Duration,
    
    /// Maximum events to store per node
    pub max_events_per_node: usize,
    
    /// Enable consensus-based validation
    pub enable_consensus_validation: bool,
    
    /// Recovery time for reputation after good behavior
    pub recovery_multiplier: f64,
}

impl Default for ReputationConfig {
    fn default() -> Self {
        Self {
            initial_reputation: 0.8,
            max_reputation: 1.0,
            min_reputation: 0.0,
            decay_rate_per_hour: 0.01,
            quarantine_threshold: 0.3,
            isolation_threshold: 0.1,
            consensus_validators: 5,
            consensus_threshold: 0.66, // 2/3 majority
            event_window: Duration::from_secs(3600), // 1 hour
            max_events_per_node: 1000,
            enable_consensus_validation: true,
            recovery_multiplier: 1.5,
        }
    }
}

/// Types of reputation events (removed Hash derive due to f64 fields)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReputationEvent {
    /// Byzantine behavior detected
    ByzantineBehavior {
        /// Type of Byzantine behavior
        behavior_type: String,
        /// Severity (0.0 to 1.0)
        severity: f64,
    },
    
    /// Good consensus participation
    GoodParticipation {
        /// Quality of participation (0.0 to 1.0)
        quality: f64,
    },
    
    /// Successful block validation
    SuccessfulValidation {
        /// Number of blocks validated
        block_count: u32,
    },
    
    /// Network contribution
    NetworkContribution {
        /// Type of contribution
        contribution_type: String,
        /// Value of contribution (0.0 to 1.0)
        value: f64,
    },
    
    /// Malicious activity
    MaliciousActivity {
        /// Type of malicious activity
        activity_type: String,
        /// Severity (0.0 to 1.0)
        severity: f64,
    },
    
    /// Recovery behavior (showing improvement)
    RecoveryBehavior {
        /// Type of recovery
        recovery_type: String,
        /// Improvement score (0.0 to 1.0)
        improvement: f64,
    },
    
    /// Network disruption
    NetworkDisruption {
        /// Type of disruption
        disruption_type: String,
        /// Impact severity (0.0 to 1.0)
        impact: f64,
    },
    
    /// Consensus violation
    ConsensusViolation {
        /// Type of violation
        violation_type: String,
        /// Severity (0.0 to 1.0)
        severity: f64,
    },
}

/// Reputation event record
#[derive(Debug, Clone)]
pub struct ReputationEventRecord {
    /// Event details
    pub event: ReputationEvent,
    
    /// When the event occurred
    pub timestamp: SystemTime,
    
    /// Node that reported the event
    pub reporter: NodeId,
    
    /// Reputation impact (-1.0 to 1.0)
    pub impact: f64,
    
    /// Confidence in the event (0.0 to 1.0)
    pub confidence: f64,
    
    /// Whether this event has been validated by consensus
    pub consensus_validated: bool,
    
    /// Number of validators that confirmed this event
    pub validator_confirmations: u32,
    
    /// Event source evidence
    pub evidence: Vec<String>,
}

/// Node reputation state
#[derive(Debug, Clone)]
pub struct ReputationState {
    /// Current reputation score (0.0 to 1.0)
    pub current_score: f64,
    
    /// Peak reputation score achieved
    pub peak_score: f64,
    
    /// Lowest reputation score reached
    pub lowest_score: f64,
    
    /// Last reputation update time
    pub last_updated: SystemTime,
    
    /// Number of reputation events
    pub total_events: u64,
    
    /// Number of positive events
    pub positive_events: u64,
    
    /// Number of negative events
    pub negative_events: u64,
    
    /// Recent reputation trend (positive or negative)
    pub trend: ReputationTrend,
    
    /// Whether node is currently quarantined
    pub quarantined: bool,
    
    /// Whether node is currently isolated
    pub isolated: bool,
    
    /// Time since last Byzantine behavior
    pub time_since_last_byzantine: Option<Duration>,
    
    /// Recovery progress (0.0 to 1.0) for isolated nodes
    pub recovery_progress: f64,
}

/// Reputation trend analysis
#[derive(Debug, Clone, PartialEq)]
pub enum ReputationTrend {
    /// Reputation is improving
    Improving,
    
    /// Reputation is declining
    Declining,
    
    /// Reputation is stable
    Stable,
    
    /// Not enough data for trend analysis
    Insufficient,
}

/// Consensus validation for reputation events
#[derive(Debug, Clone)]
pub struct ConsensusValidationPending {
    /// Event being validated
    pub event: ReputationEventRecord,
    
    /// Node being evaluated
    pub target_node: NodeId,
    
    /// Validators assigned to this validation
    pub validators: Vec<NodeId>,
    
    /// Validation responses received
    pub responses: HashMap<NodeId, ValidationResponse>,
    
    /// Validation deadline
    pub deadline: SystemTime,
    
    /// Whether validation is complete
    pub complete: bool,
    
    /// Final consensus result
    pub consensus_result: Option<bool>,
}

/// Validator response to reputation event
#[derive(Debug, Clone)]
pub struct ValidationResponse {
    /// Validator node ID
    pub validator: NodeId,
    
    /// Whether validator agrees with the event
    pub agrees: bool,
    
    /// Validator's confidence in the assessment (0.0 to 1.0)
    pub confidence: f64,
    
    /// Additional evidence provided by validator
    pub evidence: Vec<String>,
    
    /// Response timestamp
    pub timestamp: SystemTime,
    
    /// Validator's own reputation at time of response
    pub validator_reputation: f64,
}

/// Reputation system metrics
#[derive(Debug, Clone)]
pub struct ReputationMetrics {
    /// Total reputation events processed
    pub total_events: u64,
    
    /// Consensus validations completed
    pub consensus_validations: u64,
    
    /// Consensus validation success rate
    pub consensus_success_rate: f64,
    
    /// Nodes currently quarantined
    pub nodes_quarantined: u64,
    
    /// Nodes currently isolated
    pub nodes_isolated: u64,
    
    /// Average reputation score across all nodes
    pub average_reputation: f64,
    
    /// Reputation distribution by score ranges
    pub reputation_distribution: HashMap<String, u64>,
    
    /// Byzantine events detected
    pub byzantine_events: u64,
    
    /// Recovery events recorded
    pub recovery_events: u64,
    
    /// Last update timestamp
    pub last_updated: Instant,
}

impl Default for ReputationMetrics {
    fn default() -> Self {
        Self {
            total_events: 0,
            consensus_validations: 0,
            consensus_success_rate: 0.0,
            nodes_quarantined: 0,
            nodes_isolated: 0,
            average_reputation: 0.0,
            reputation_distribution: HashMap::new(),
            byzantine_events: 0,
            recovery_events: 0,
            last_updated: Instant::now(),
        }
    }
}

/// Main reputation management system
pub struct ReputationManager {
    /// Configuration
    config: ReputationConfig,
    
    /// Node reputation states
    reputation_states: Arc<RwLock<HashMap<NodeId, ReputationState>>>,
    
    /// Reputation event history
    event_history: Arc<RwLock<HashMap<NodeId, VecDeque<ReputationEventRecord>>>>,
    
    /// Pending consensus validations
    pending_validations: Arc<RwLock<HashMap<String, ConsensusValidationPending>>>,
    
    /// System metrics
    metrics: Arc<RwLock<ReputationMetrics>>,
    
    /// Event notification sender
    event_sender: mpsc::UnboundedSender<ReputationEventRecord>,
    
    /// Event notification receiver
    event_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<ReputationEventRecord>>>>,
    
    /// Background task handles
    background_tasks: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,
}

impl ReputationManager {
    /// Create a new reputation manager
    pub async fn new(config: ReputationConfig) -> ConsensusResult<Self> {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        
        Ok(Self {
            config,
            reputation_states: Arc::new(RwLock::new(HashMap::new())),
            event_history: Arc::new(RwLock::new(HashMap::new())),
            pending_validations: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(ReputationMetrics::default())),
            event_sender,
            event_receiver: Arc::new(RwLock::new(Some(event_receiver))),
            background_tasks: Arc::new(RwLock::new(Vec::new())),
        })
    }
    
    /// Start the reputation manager
    pub async fn start(&self) -> ConsensusResult<()> {
        info!("Starting reputation management system");
        
        // Start background tasks
        self.start_event_processing().await;
        self.start_reputation_decay().await;
        self.start_consensus_validation().await;
        
        info!("Reputation management system started");
        Ok(())
    }
    
    /// Stop the reputation manager
    pub async fn stop(&self) -> ConsensusResult<()> {
        info!("Stopping reputation management system");
        
        // Stop background tasks
        let mut tasks = self.background_tasks.write().await;
        for task in tasks.drain(..) {
            task.abort();
        }
        
        info!("Reputation management system stopped");
        Ok(())
    }
    
    /// Perform maintenance operations
    pub async fn perform_maintenance(&self) -> ConsensusResult<()> {
        debug!("Performing reputation system maintenance");
        
        // Clean up old events
        let mut history = self.event_history.write().await;
        let cutoff_time = SystemTime::now() - Duration::from_secs(86400 * 7); // 7 days
        
        for (_node_id, events) in history.iter_mut() {
            events.retain(|event| event.timestamp > cutoff_time);
        }
        
        // Clean up expired validations
        let mut validations = self.pending_validations.write().await;
        let now = SystemTime::now();
        
        validations.retain(|_id, validation| {
            !validation.complete && validation.deadline > now
        });
        
        // Update metrics
        self.update_metrics().await;
        
        debug!("Reputation system maintenance completed");
        Ok(())
    }
    
    /// Record a reputation event for a node
    pub async fn record_event(
        &self,
        node_id: &NodeId,
        event: ReputationEvent,
        reporter: &NodeId,
        confidence: f64,
        evidence: Vec<String>,
    ) -> ConsensusResult<()> {
        debug!("Recording reputation event for node {:?}: {:?}", node_id, event);
        
        // Calculate reputation impact
        let impact = self.calculate_reputation_impact(&event);
        
        let event_record = ReputationEventRecord {
            event: event.clone(),
            timestamp: SystemTime::now(),
            reporter: reporter.clone(),
            impact,
            confidence,
            consensus_validated: false,
            validator_confirmations: 0,
            evidence,
        };
        
        // Send event for processing
        if let Err(e) = self.event_sender.send(event_record) {
            error!("Failed to send reputation event: {:?}", e);
            return Err(ConsensusError::ReputationError("Failed to send event".to_string()));
        }
        
        // If consensus validation is enabled, initiate validation
        if self.config.enable_consensus_validation {
            self.initiate_consensus_validation(node_id, &event, reporter.clone()).await?;
        }
        
        Ok(())
    }
    
    /// Get current reputation for a node
    pub async fn get_reputation(&self, node_id: &NodeId) -> f64 {
        if let Some(state) = self.reputation_states.read().await.get(node_id) {
            state.current_score
        } else {
            self.config.initial_reputation
        }
    }
    
    /// Get detailed reputation state for a node
    pub async fn get_reputation_state(&self, node_id: &NodeId) -> Option<ReputationState> {
        self.reputation_states.read().await.get(node_id).cloned()
    }
    
    /// Get reputation history for a node
    pub async fn get_reputation_history(&self, node_id: &NodeId) -> Vec<ReputationEventRecord> {
        if let Some(history) = self.event_history.read().await.get(node_id) {
            history.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }
    
    /// Check if a node should be quarantined
    pub async fn should_quarantine(&self, node_id: &NodeId) -> bool {
        self.get_reputation(node_id).await < self.config.quarantine_threshold
    }
    
    /// Check if a node should be isolated
    pub async fn should_isolate(&self, node_id: &NodeId) -> bool {
        self.get_reputation(node_id).await < self.config.isolation_threshold
    }
    
    /// Get reputation metrics
    pub async fn get_metrics(&self) -> ReputationMetrics {
        self.metrics.read().await.clone()
    }
    
    /// Get all node reputations above a threshold
    pub async fn get_trusted_nodes(&self, min_reputation: f64) -> Vec<(NodeId, f64)> {
        self.reputation_states.read().await
            .iter()
            .filter(|(_, state)| state.current_score >= min_reputation)
            .map(|(node_id, state)| (node_id.clone(), state.current_score))
            .collect()
    }
    
    /// Calculate reputation impact for an event
    fn calculate_reputation_impact(&self, event: &ReputationEvent) -> f64 {
        match event {
            ReputationEvent::ByzantineBehavior { severity, .. } => -severity * 0.5,
            ReputationEvent::GoodParticipation { quality } => quality * 0.1,
            ReputationEvent::SuccessfulValidation { block_count } => (*block_count as f64 * 0.01).min(0.05),
            ReputationEvent::NetworkContribution { value, .. } => value * 0.05,
            ReputationEvent::MaliciousActivity { severity, .. } => -severity * 0.8,
            ReputationEvent::RecoveryBehavior { improvement, .. } => improvement * 0.3 * self.config.recovery_multiplier,
            ReputationEvent::NetworkDisruption { impact, .. } => -impact * 0.4,
            ReputationEvent::ConsensusViolation { severity, .. } => -severity * 0.6,
        }
    }
    
    /// Process a reputation event and update node state
    async fn process_event(&self, node_id: &NodeId, event_record: ReputationEventRecord) {
        // Get or create reputation state
        let mut states = self.reputation_states.write().await;
        let state = states.entry(node_id.clone()).or_insert_with(|| ReputationState {
            current_score: self.config.initial_reputation,
            peak_score: self.config.initial_reputation,
            lowest_score: self.config.initial_reputation,
            last_updated: SystemTime::now(),
            total_events: 0,
            positive_events: 0,
            negative_events: 0,
            trend: ReputationTrend::Insufficient,
            quarantined: false,
            isolated: false,
            time_since_last_byzantine: None,
            recovery_progress: 0.0,
        });
        
        // Apply reputation decay since last update
        self.apply_reputation_decay(state);
        
        // Apply event impact
        let new_score = (state.current_score + event_record.impact)
            .max(self.config.min_reputation)
            .min(self.config.max_reputation);
        
        state.current_score = new_score;
        state.last_updated = SystemTime::now();
        state.total_events += 1;
        
        if event_record.impact > 0.0 {
            state.positive_events += 1;
        } else if event_record.impact < 0.0 {
            state.negative_events += 1;
        }
        
        // Update peak and lowest scores
        state.peak_score = state.peak_score.max(new_score);
        state.lowest_score = state.lowest_score.min(new_score);
        
        // Update Byzantine behavior tracking
        if matches!(event_record.event, ReputationEvent::ByzantineBehavior { .. } | 
                   ReputationEvent::MaliciousActivity { .. } | 
                   ReputationEvent::ConsensusViolation { .. }) {
            state.time_since_last_byzantine = Some(Duration::ZERO);
        } else if let Some(ref mut duration) = state.time_since_last_byzantine {
            *duration += Duration::from_secs(60); // Approximation
        }
        
        // Update quarantine and isolation status
        state.quarantined = new_score < self.config.quarantine_threshold;
        state.isolated = new_score < self.config.isolation_threshold;
        
        // Calculate trend
        state.trend = self.calculate_reputation_trend(node_id).await;
        
        // Update recovery progress for isolated nodes
        if state.isolated && event_record.impact > 0.0 {
            state.recovery_progress = ((new_score - self.config.isolation_threshold) / 
                                    (self.config.quarantine_threshold - self.config.isolation_threshold))
                                    .max(0.0).min(1.0);
        }
        
        drop(states);
        
        // Store event in history
        let mut history = self.event_history.write().await;
        let node_history = history.entry(node_id.clone())
            .or_insert_with(|| VecDeque::with_capacity(self.config.max_events_per_node));
        
        // Add event to history
        if node_history.len() >= self.config.max_events_per_node {
            node_history.pop_front();
        }
        node_history.push_back(event_record);
        
        drop(history);
        
        // Update metrics
        self.update_metrics().await;
        
        info!("Updated reputation for node {:?}: {:.3}", node_id, new_score);
    }
    
    /// Apply time-based reputation decay
    fn apply_reputation_decay(&self, state: &mut ReputationState) {
        let now = SystemTime::now();
        let time_since_update = now.duration_since(state.last_updated)
            .unwrap_or(Duration::ZERO);
        
        let hours_elapsed = time_since_update.as_secs_f64() / 3600.0;
        let decay_amount = self.config.decay_rate_per_hour * hours_elapsed;
        
        // Apply decay (gradual drift toward initial reputation)
        let target = self.config.initial_reputation;
        let current = state.current_score;
        
        if current > target {
            state.current_score = (current - decay_amount).max(target);
        } else if current < target {
            state.current_score = (current + decay_amount * 0.5).min(target); // Slower recovery through decay
        }
    }
    
    /// Calculate reputation trend for a node
    async fn calculate_reputation_trend(&self, node_id: &NodeId) -> ReputationTrend {
        let history = self.event_history.read().await;
        if let Some(events) = history.get(node_id) {
            if events.len() < 5 {
                return ReputationTrend::Insufficient;
            }
            
            // Analyze recent events for trend
            let recent_events: Vec<_> = events.iter().rev().take(10).collect();
            let positive_count = recent_events.iter()
                .filter(|e| e.impact > 0.0)
                .count();
            let negative_count = recent_events.iter()
                .filter(|e| e.impact < 0.0)
                .count();
            
            if positive_count > negative_count * 2 {
                ReputationTrend::Improving
            } else if negative_count > positive_count * 2 {
                ReputationTrend::Declining
            } else {
                ReputationTrend::Stable
            }
        } else {
            ReputationTrend::Insufficient
        }
    }
    
    /// Initiate consensus validation for a reputation event
    async fn initiate_consensus_validation(
        &self,
        node_id: &NodeId,
        event: &ReputationEvent,
        initial_validator: NodeId,
    ) -> ConsensusResult<()> {
        let validation_id = format!("{:?}:{:?}:{}", node_id, event, SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs());
        
        let pending = ConsensusValidationPending {
            event: ReputationEventRecord {
                event: event.clone(),
                timestamp: SystemTime::now(),
                reporter: initial_validator.clone(),
                impact: self.calculate_reputation_impact(event),
                confidence: 1.0,
                consensus_validated: false,
                validator_confirmations: 0,
                evidence: Vec::new(),
            },
            target_node: node_id.clone(),
            validators: self.select_validators(&initial_validator).await,
            responses: HashMap::new(),
            deadline: SystemTime::now() + Duration::from_secs(300), // 5 minutes
            complete: false,
            consensus_result: None,
        };
        
        self.pending_validations.write().await.insert(validation_id, pending);
        Ok(())
    }
    
    /// Select validators for consensus validation
    async fn select_validators(&self, exclude: &NodeId) -> Vec<NodeId> {
        let states = self.reputation_states.read().await;
        let mut candidates: Vec<_> = states.iter()
            .filter(|(node_id, state)| {
                *node_id != exclude && 
                state.current_score >= self.config.quarantine_threshold &&
                !state.isolated
            })
            .map(|(node_id, state)| (node_id.clone(), state.current_score))
            .collect();
        
        // Sort by reputation and select top validators
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        candidates.into_iter()
            .take(self.config.consensus_validators)
            .map(|(node_id, _)| node_id)
            .collect()
    }
    
    /// Submit validation response
    pub async fn submit_validation_response(
        &self,
        validation_id: &str,
        validator: &NodeId,
        agrees: bool,
        confidence: f64,
        evidence: Vec<String>,
    ) -> ConsensusResult<()> {
        let mut validations = self.pending_validations.write().await;
        if let Some(validation) = validations.get_mut(validation_id) {
            let validator_reputation = self.get_reputation(validator).await;
            
            let response = ValidationResponse {
                validator: validator.clone(),
                agrees,
                confidence,
                evidence,
                timestamp: SystemTime::now(),
                validator_reputation,
            };
            
            validation.responses.insert(validator.clone(), response);
            
            // Check if consensus is reached
            if validation.responses.len() >= self.config.consensus_validators {
                let consensus_reached = self.evaluate_consensus(validation).await;
                if consensus_reached {
                    validation.complete = true;
                    // Process consensus result
                    self.process_consensus_result(validation).await;
                }
            }
        }
        
        Ok(())
    }
    
    /// Evaluate if consensus has been reached
    async fn evaluate_consensus(&self, validation: &mut ConsensusValidationPending) -> bool {
        let total_responses = validation.responses.len();
        if total_responses < self.config.consensus_validators {
            return false;
        }
        
        // Weight votes by validator reputation
        let mut weighted_agree = 0.0;
        let mut total_weight = 0.0;
        
        for response in validation.responses.values() {
            let weight = response.validator_reputation * response.confidence;
            total_weight += weight;
            if response.agrees {
                weighted_agree += weight;
            }
        }
        
        let agreement_ratio = if total_weight > 0.0 {
            weighted_agree / total_weight
        } else {
            0.0
        };
        
        let consensus_reached = agreement_ratio >= self.config.consensus_threshold;
        validation.consensus_result = Some(consensus_reached);
        
        consensus_reached
    }
    
    /// Process consensus validation result
    async fn process_consensus_result(&self, validation: &ConsensusValidationPending) {
        if let Some(consensus_agrees) = validation.consensus_result {
            if consensus_agrees {
                // Consensus agrees with the event - process it
                self.process_event(&validation.target_node, validation.event.clone()).await;
                
                // Update metrics
                let mut metrics = self.metrics.write().await;
                metrics.consensus_validations += 1;
                
                if matches!(validation.event.event, 
                           ReputationEvent::ByzantineBehavior { .. } | 
                           ReputationEvent::MaliciousActivity { .. } | 
                           ReputationEvent::ConsensusViolation { .. }) {
                    metrics.byzantine_events += 1;
                }
                
                if matches!(validation.event.event, ReputationEvent::RecoveryBehavior { .. }) {
                    metrics.recovery_events += 1;
                }
            } else {
                debug!("Consensus rejected reputation event for node {:?}", validation.target_node);
            }
            
            // Update consensus success rate
            let mut metrics = self.metrics.write().await;
            metrics.consensus_success_rate = if metrics.consensus_validations > 0 {
                metrics.byzantine_events as f64 / metrics.consensus_validations as f64
            } else {
                0.0
            };
            
            metrics.last_updated = Instant::now();
        }
    }
    
    /// Update system metrics
    async fn update_metrics(&self) {
        let states = self.reputation_states.read().await;
        let mut metrics = self.metrics.write().await;
        
        let total_nodes = states.len();
        if total_nodes == 0 {
            return;
        }
        
        // Calculate average reputation
        let total_reputation: f64 = states.values().map(|s| s.current_score).sum();
        metrics.average_reputation = total_reputation / total_nodes as f64;
        
        // Count quarantined and isolated nodes
        metrics.nodes_quarantined = states.values().filter(|s| s.quarantined).count() as u64;
        metrics.nodes_isolated = states.values().filter(|s| s.isolated).count() as u64;
        
        // Calculate reputation distribution
        let mut distribution = HashMap::new();
        for state in states.values() {
            let range = if state.current_score >= 0.8 {
                "high"
            } else if state.current_score >= 0.5 {
                "medium"
            } else if state.current_score >= 0.2 {
                "low"
            } else {
                "critical"
            };
            
            *distribution.entry(range.to_string()).or_insert(0) += 1;
        }
        metrics.reputation_distribution = distribution;
        
        metrics.last_updated = Instant::now();
    }
    
    /// Start event processing task
    async fn start_event_processing(&self) {
        if let Some(mut receiver) = self.event_receiver.write().await.take() {
            let _reputation_states = self.reputation_states.clone();
            let _event_history = self.event_history.clone();
            let _config = self.config.clone();
            
            let mut tasks = self.background_tasks.write().await;
            let handle = tokio::spawn(async move {
                while let Some(event_record) = receiver.recv().await {
                    debug!("Processing reputation event: {:?}", event_record);
                    
                    // In a real implementation, would process the event here
                    // For now, just log it
                }
            });
            
            tasks.push(handle);
        }
    }
    
    /// Start reputation decay task
    async fn start_reputation_decay(&self) {
        let reputation_states = self.reputation_states.clone();
        let config = self.config.clone();
        
        let mut tasks = self.background_tasks.write().await;
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Every hour
            
            loop {
                interval.tick().await;
                
                // Apply decay to all nodes
                let mut states = reputation_states.write().await;
                for (node_id, state) in states.iter_mut() {
                    let old_score = state.current_score;
                    
                    // Apply decay
                    let now = SystemTime::now();
                    let time_since_update = now.duration_since(state.last_updated)
                        .unwrap_or(Duration::ZERO);
                    
                    let hours_elapsed = time_since_update.as_secs_f64() / 3600.0;
                    let decay_amount = config.decay_rate_per_hour * hours_elapsed;
                    
                    // Apply decay (gradual drift toward initial reputation)
                    let target = config.initial_reputation;
                    let current = state.current_score;
                    
                    if current > target {
                        state.current_score = (current - decay_amount).max(target);
                    } else if current < target {
                        state.current_score = (current + decay_amount * 0.5).min(target);
                    }
                    
                    state.last_updated = now;
                    
                    if (old_score - state.current_score).abs() > 0.001 {
                        debug!("Applied reputation decay to node {:?}: {:.3} -> {:.3}", 
                               node_id, old_score, state.current_score);
                    }
                }
                
                debug!("Reputation decay cycle completed");
            }
        });
        
        tasks.push(handle);
    }
    
    /// Start consensus validation task
    async fn start_consensus_validation(&self) {
        let pending_validations = self.pending_validations.clone();
        
        let mut tasks = self.background_tasks.write().await;
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                // Check for expired validations
                let now = SystemTime::now();
                let mut validations = pending_validations.write().await;
                
                let expired_validations: Vec<_> = validations.iter()
                    .filter(|(_, v)| now > v.deadline && !v.complete)
                    .map(|(id, _)| id.clone())
                    .collect();
                
                for validation_id in expired_validations {
                    if let Some(mut validation) = validations.remove(&validation_id) {
                        warn!("Consensus validation expired: {}", validation_id);
                        validation.complete = true;
                        validation.consensus_result = Some(false); // Default to rejection on timeout
                    }
                }
                
                debug!("Consensus validation maintenance completed");
            }
        });
        
        tasks.push(handle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_reputation_manager_creation() {
        let config = ReputationConfig::default();
        let manager = ReputationManager::new(config).await;
        assert!(manager.is_ok());
    }
    
    #[tokio::test]
    async fn test_reputation_event_recording() {
        let config = ReputationConfig::default();
        let manager = ReputationManager::new(config).await.unwrap();
        
        let node_id = NodeId::new("test-node".to_string());
        let reporter = NodeId::new("reporter".to_string());
        
        let event = ReputationEvent::ByzantineBehavior {
            behavior_type: "test".to_string(),
            severity: 0.8,
        };
        
        let result = manager.record_event(&node_id, event, &reporter, 1.0, vec!["evidence".to_string()]).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_reputation_calculation() {
        let config = ReputationConfig::default();
        let manager = ReputationManager::new(config.clone()).await.unwrap();
        
        let node_id = NodeId::new("test-node".to_string());
        
        // Initial reputation should be the configured initial value
        let initial_rep = manager.get_reputation(&node_id).await;
        assert_eq!(initial_rep, config.initial_reputation);
    }
    
    #[tokio::test]
    async fn test_quarantine_threshold() {
        let config = ReputationConfig::default();
        let manager = ReputationManager::new(config).await.unwrap();
        
        let node_id = NodeId::new("test-node".to_string());
        
        // Node should not be quarantined initially
        let should_quarantine = manager.should_quarantine(&node_id).await;
        assert!(!should_quarantine);
    }
    
    #[tokio::test]
    async fn test_trusted_nodes() {
        let config = ReputationConfig::default();
        let manager = ReputationManager::new(config).await.unwrap();
        
        let trusted_nodes = manager.get_trusted_nodes(0.5).await;
        assert!(trusted_nodes.is_empty()); // No nodes registered yet
    }
}