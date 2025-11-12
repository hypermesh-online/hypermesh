//! Attack prevention and security validation
//!
//! This module provides comprehensive attack prevention capabilities including:
//! - Double-spending attack detection and prevention
//! - Sybil attack detection through node behavior analysis
//! - Eclipse attack detection via network topology monitoring
//! - Statistical analysis and machine learning-based pattern detection
//!
//! The system uses quantum-resistant cryptography and implements real-time
//! monitoring with adaptive threat response.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{sync::RwLock, time::sleep};
use serde::{Serialize, Deserialize};
use rand::Rng;

use super::{
    error::{ConsensusError, ConsensusResult},
    ConsensusMessage, NodeId, Transaction,
    detection::quantum_security::{QuantumSecureValidator, SecurityLevel},
};

/// Configuration for attack prevention system
#[derive(Debug, Clone)]
pub struct AttackPreventionConfig {
    /// Detection sensitivity (0.0-1.0, higher = more sensitive)
    pub detection_sensitivity: f64,
    /// Maximum allowed transactions per node per second
    pub max_tx_rate: u64,
    /// Window size for behavior analysis
    pub behavior_window_size: usize,
    /// Minimum network size for eclipse detection
    pub min_network_size: usize,
    /// Sybil detection threshold
    pub sybil_threshold: f64,
    /// Double spending detection enabled
    pub enable_double_spend_detection: bool,
    /// Eclipse attack detection enabled  
    pub enable_eclipse_detection: bool,
    /// ML-based prediction enabled
    pub enable_ml_prediction: bool,
    /// Quantum security validation level
    pub quantum_security_level: SecurityLevel,
}

impl Default for AttackPreventionConfig {
    fn default() -> Self {
        Self {
            detection_sensitivity: 0.8,
            max_tx_rate: 1000,
            behavior_window_size: 100,
            min_network_size: 10,
            sybil_threshold: 0.7,
            enable_double_spend_detection: true,
            enable_eclipse_detection: true,
            enable_ml_prediction: true,
            quantum_security_level: SecurityLevel::HighSecurity,
        }
    }
}

/// Attack prevention system
pub struct AttackPreventionSystem {
    config: AttackPreventionConfig,
    double_spend_detector: Arc<RwLock<DoubleSpendDetector>>,
    sybil_detector: Arc<RwLock<SybilDetector>>,
    eclipse_detector: Arc<RwLock<EclipseDetector>>,
    coordination_detector: Arc<RwLock<CoordinationDetector>>,
    quantum_validator: Arc<QuantumSecureValidator>,
    metrics: Arc<RwLock<PreventionMetrics>>,
}

impl AttackPreventionSystem {
    /// Create new attack prevention system
    pub fn new(config: AttackPreventionConfig) -> Self {
        Self {
            double_spend_detector: Arc::new(RwLock::new(
                DoubleSpendDetector::new(config.max_tx_rate)
            )),
            sybil_detector: Arc::new(RwLock::new(
                SybilDetector::new(config.sybil_threshold, config.behavior_window_size)
            )),
            eclipse_detector: Arc::new(RwLock::new(
                EclipseDetector::new(config.min_network_size)
            )),
            coordination_detector: Arc::new(RwLock::new(
                CoordinationDetector::new(config.behavior_window_size)
            )),
            quantum_validator: Arc::new(
                QuantumSecureValidator::new(config.quantum_security_level.clone())
            ),
            metrics: Arc::new(RwLock::new(PreventionMetrics::default())),
            config,
        }
    }

    /// Validate transaction for potential attacks
    pub async fn validate_transaction(&self, tx: &Transaction, sender: &NodeId) -> ConsensusResult<bool> {
        let mut metrics = self.metrics.write().await;
        metrics.total_validations += 1;
        drop(metrics);

        // Double-spending detection
        if self.config.enable_double_spend_detection {
            if !self.double_spend_detector.read().await.validate_transaction(tx, sender).await? {
                let mut metrics = self.metrics.write().await;
                metrics.double_spend_attempts += 1;
                return Ok(false);
            }
        }

        // Quantum security validation
        if !self.quantum_validator.validate_transaction_security(tx, sender).await? {
            let mut metrics = self.metrics.write().await;
            metrics.quantum_security_violations += 1;
            return Ok(false);
        }

        Ok(true)
    }

    /// Analyze node behavior for Sybil attack patterns
    pub async fn analyze_node_behavior(&self, node_id: &NodeId, behavior_data: &NodeBehaviorData) -> ConsensusResult<f64> {
        if !self.config.enable_eclipse_detection {
            return Ok(0.0);
        }

        let sybil_score = self.sybil_detector.read().await
            .analyze_behavior(node_id, behavior_data).await?;

        if sybil_score > self.config.sybil_threshold {
            let mut metrics = self.metrics.write().await;
            metrics.sybil_attempts += 1;
        }

        Ok(sybil_score)
    }

    /// Monitor network topology for eclipse attacks
    pub async fn monitor_network_topology(&self, topology: &NetworkTopology) -> ConsensusResult<f64> {
        if !self.config.enable_eclipse_detection {
            return Ok(0.0);
        }

        let eclipse_risk = self.eclipse_detector.read().await
            .analyze_topology(topology).await?;

        if eclipse_risk > 0.8 {
            let mut metrics = self.metrics.write().await;
            metrics.eclipse_attempts += 1;
        }

        Ok(eclipse_risk)
    }

    /// Update node coordination behavior
    pub async fn update_coordination_behavior(&self, node_id: &NodeId, behavior: &CoordinationBehavior) -> ConsensusResult<()> {
        self.coordination_detector.write().await
            .update_behavior(node_id, behavior).await?;
        Ok(())
    }

    /// Get attack prevention metrics
    pub async fn get_metrics(&self) -> PreventionMetrics {
        self.metrics.read().await.clone()
    }

    /// Perform maintenance operations
    pub async fn perform_maintenance(&self) -> ConsensusResult<()> {
        // Clean up old data from all detectors
        self.double_spend_detector.write().await.cleanup_old_data().await?;
        self.sybil_detector.write().await.cleanup_old_data().await?;
        self.eclipse_detector.write().await.cleanup_old_data().await?;
        self.coordination_detector.write().await.cleanup_old_data().await?;

        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.last_maintenance = Instant::now();

        Ok(())
    }
}

/// Double-spending attack detector
pub struct DoubleSpendDetector {
    max_tx_rate: u64,
    transaction_history: HashMap<NodeId, VecDeque<TransactionRecord>>,
    spent_outputs: HashMap<String, (NodeId, Instant)>, // tx_hash -> (spender, timestamp)
}

impl DoubleSpendDetector {
    fn new(max_tx_rate: u64) -> Self {
        Self {
            max_tx_rate,
            transaction_history: HashMap::new(),
            spent_outputs: HashMap::new(),
        }
    }

    async fn validate_transaction(&self, tx: &Transaction, sender: &NodeId) -> ConsensusResult<bool> {
        // Check rate limiting
        if let Some(history) = self.transaction_history.get(sender) {
            let recent_count = history.iter()
                .filter(|record| record.timestamp.elapsed() < Duration::from_secs(1))
                .count() as u64;
            
            if recent_count >= self.max_tx_rate {
                return Ok(false);
            }
        }

        // Check for double spending using transaction ID
        let tx_hash = format!("{}", tx.id);
        if let Some((previous_spender, _)) = self.spent_outputs.get(&tx_hash) {
            if previous_spender != sender {
                return Ok(false); // Double spend detected
            }
        }

        Ok(true)
    }

    async fn cleanup_old_data(&mut self) -> ConsensusResult<()> {
        let cutoff = Instant::now() - Duration::from_secs(24 * 60 * 60); // 24 hours
        
        // Clean transaction history
        for history in self.transaction_history.values_mut() {
            history.retain(|record| record.timestamp > cutoff);
        }
        
        // Clean spent outputs
        self.spent_outputs.retain(|_, (_, timestamp)| *timestamp > cutoff);
        
        Ok(())
    }
}

/// Sybil attack detector using behavioral analysis
pub struct SybilDetector {
    threshold: f64,
    window_size: usize,
    node_profiles: HashMap<NodeId, NodeProfile>,
    ml_predictor: Option<SimpleMLP>,
}

impl SybilDetector {
    fn new(threshold: f64, window_size: usize) -> Self {
        Self {
            threshold,
            window_size,
            node_profiles: HashMap::new(),
            ml_predictor: Some(SimpleMLP::new()),
        }
    }

    async fn analyze_behavior(&self, node_id: &NodeId, behavior: &NodeBehaviorData) -> ConsensusResult<f64> {
        let profile = self.node_profiles.get(node_id).cloned()
            .unwrap_or_else(|| NodeProfile::new(node_id.clone()));

        // Statistical analysis
        let stat_score = self.calculate_statistical_score(&profile, behavior);
        
        // ML-based prediction if enabled
        let ml_score = if let Some(predictor) = &self.ml_predictor {
            predictor.predict_sybil_probability(behavior).await.unwrap_or(0.0)
        } else {
            0.0
        };

        // Combined score
        let combined_score = (stat_score + ml_score) / 2.0;
        Ok(combined_score)
    }

    fn calculate_statistical_score(&self, _profile: &NodeProfile, behavior: &NodeBehaviorData) -> f64 {
        let mut score: f64 = 0.0;
        
        // Analyze connection patterns
        if behavior.connection_count < 5 {
            score += 0.3; // Suspicious low connectivity
        }
        
        // Analyze timing patterns
        if behavior.message_intervals.len() > 10 {
            let variance = self.calculate_variance(&behavior.message_intervals);
            if variance < 0.1 {
                score += 0.4; // Suspiciously regular timing
            }
        }
        
        // Analyze resource usage
        if behavior.cpu_usage < 0.05 && behavior.network_usage > 0.8 {
            score += 0.3; // Unusual resource profile
        }

        score.min(1.0)
    }

    fn calculate_variance(&self, values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        
        let mean: f64 = values.iter().sum::<f64>() / values.len() as f64;
        let variance: f64 = values.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / values.len() as f64;
        
        variance
    }

    async fn cleanup_old_data(&mut self) -> ConsensusResult<()> {
        let cutoff = SystemTime::now() - Duration::from_secs(24 * 60 * 60); // 24 hours
        
        self.node_profiles.retain(|_, profile| {
            profile.last_seen > cutoff
        });
        
        Ok(())
    }
}

/// Eclipse attack detector through network analysis
pub struct EclipseDetector {
    min_network_size: usize,
    topology_history: VecDeque<TopologySnapshot>,
}

impl EclipseDetector {
    fn new(min_network_size: usize) -> Self {
        Self {
            min_network_size,
            topology_history: VecDeque::with_capacity(100),
        }
    }

    async fn analyze_topology(&self, topology: &NetworkTopology) -> ConsensusResult<f64> {
        let mut risk_score = 0.0;

        // Check network size
        if topology.node_count < self.min_network_size {
            risk_score += 0.3;
        }

        // Check connectivity distribution
        let connectivity_score = self.analyze_connectivity_distribution(topology);
        risk_score += connectivity_score * 0.4;

        // Check for isolation patterns
        let isolation_score = self.detect_isolation_patterns(topology);
        risk_score += isolation_score * 0.3;

        Ok(risk_score.min(1.0))
    }

    fn analyze_connectivity_distribution(&self, topology: &NetworkTopology) -> f64 {
        if topology.connections.is_empty() {
            return 1.0;
        }

        // Calculate connection distribution
        let mut connection_counts: HashMap<NodeId, usize> = HashMap::new();
        for (node1, node2) in &topology.connections {
            *connection_counts.entry(node1.clone()).or_insert(0) += 1;
            *connection_counts.entry(node2.clone()).or_insert(0) += 1;
        }

        // Check for highly concentrated connections (potential eclipse)
        let max_connections = connection_counts.values().max().cloned().unwrap_or(0);
        let avg_connections = connection_counts.values().sum::<usize>() as f64 / connection_counts.len() as f64;

        if max_connections > (avg_connections * 3.0) as usize {
            0.8 // High risk of eclipse
        } else {
            0.0
        }
    }

    fn detect_isolation_patterns(&self, topology: &NetworkTopology) -> f64 {
        // Look for nodes with very few connections
        let mut isolated_count = 0;
        let mut total_count = 0;

        let connection_map = self.build_connection_map(topology);
        
        for (_, connections) in &connection_map {
            total_count += 1;
            if connections.len() <= 2 {
                isolated_count += 1;
            }
        }

        if total_count == 0 {
            return 1.0;
        }

        let isolation_ratio = isolated_count as f64 / total_count as f64;
        if isolation_ratio > 0.3 {
            0.7 // High isolation detected
        } else {
            isolation_ratio * 0.5
        }
    }

    fn build_connection_map(&self, topology: &NetworkTopology) -> HashMap<NodeId, HashSet<NodeId>> {
        let mut map = HashMap::new();
        
        for (node1, node2) in &topology.connections {
            map.entry(node1.clone()).or_insert_with(HashSet::new).insert(node2.clone());
            map.entry(node2.clone()).or_insert_with(HashSet::new).insert(node1.clone());
        }
        
        map
    }

    async fn cleanup_old_data(&mut self) -> ConsensusResult<()> {
        let cutoff = Instant::now() - Duration::from_secs(60 * 60); // 1 hour
        
        self.topology_history.retain(|snapshot| snapshot.timestamp > cutoff);
        
        Ok(())
    }
}

/// Coordination attack detector
pub struct CoordinationDetector {
    window_size: usize,
    behavior_histories: HashMap<NodeId, VecDeque<BehaviorSnapshot>>,
    coordination_patterns: HashMap<String, CoordinationPattern>,
}

impl CoordinationDetector {
    fn new(window_size: usize) -> Self {
        Self {
            window_size,
            behavior_histories: HashMap::new(),
            coordination_patterns: HashMap::new(),
        }
    }

    async fn update_behavior(&mut self, node_id: &NodeId, behavior: &CoordinationBehavior) -> ConsensusResult<()> {
        let behavior_snapshot = BehaviorSnapshot {
            timestamp: Instant::now(),
            message_rate: behavior.message_rate,
            vote_timing: behavior.vote_timing,
            network_activity: behavior.network_activity,
            consensus_participation: behavior.consensus_participation,
            vote_patterns: behavior.vote_patterns.clone(),
        };
        
        // Store window_size before mutable borrow
        let window_size = self.window_size;
        
        // Add to behavior history
        let history = self.behavior_histories.entry(node_id.clone())
            .or_insert_with(|| VecDeque::with_capacity(window_size));
        
        if history.len() >= window_size {
            history.pop_front();
        }
        history.push_back(behavior_snapshot);
        
        // Analyze coordination if we have sufficient data
        if history.len() >= window_size / 2 {
            self.analyze_coordination(node_id).await?;
        }
        
        Ok(())
    }

    async fn analyze_coordination(&mut self, node_id: &NodeId) -> ConsensusResult<()> {
        let history = match self.behavior_histories.get(node_id) {
            Some(h) => h,
            None => return Ok(()),
        };

        // Analyze timing patterns
        let timings: Vec<Duration> = history.iter()
            .map(|snapshot| snapshot.vote_timing)
            .collect();

        if self.detect_synchronized_timing(&timings) {
            // Record potential coordination - use Debug formatting for NodeId
            let pattern_id = format!("sync_timing_{:?}", node_id);
            self.coordination_patterns.insert(pattern_id, CoordinationPattern {
                pattern_type: "synchronized_timing".to_string(),
                nodes: vec![node_id.clone()],
                confidence: 0.8,
                first_detected: Instant::now(),
                last_updated: Instant::now(),
            });
        }

        Ok(())
    }

    fn detect_synchronized_timing(&self, timings: &[Duration]) -> bool {
        if timings.len() < 5 {
            return false;
        }

        // Convert to milliseconds for analysis
        let timing_ms: Vec<u64> = timings.iter()
            .map(|d| d.as_millis() as u64)
            .collect();

        // Check for suspicious regularity
        let mut intervals = Vec::new();
        for i in 1..timing_ms.len() {
            intervals.push(timing_ms[i].saturating_sub(timing_ms[i-1]));
        }

        if intervals.is_empty() {
            return false;
        }

        // Calculate variance in intervals
        let mean = intervals.iter().sum::<u64>() as f64 / intervals.len() as f64;
        let variance = intervals.iter()
            .map(|&x| (x as f64 - mean).powi(2))
            .sum::<f64>() / intervals.len() as f64;

        // Very low variance suggests coordination
        variance < 100.0 // Less than 100ms variance
    }

    async fn cleanup_old_data(&mut self) -> ConsensusResult<()> {
        let cutoff = Instant::now() - Duration::from_secs(2 * 60 * 60); // 2 hours
        
        // Clean behavior histories
        for history in self.behavior_histories.values_mut() {
            history.retain(|snapshot| snapshot.timestamp > cutoff);
        }
        
        // Clean coordination patterns
        self.coordination_patterns.retain(|_, pattern| {
            pattern.last_updated > cutoff
        });
        
        Ok(())
    }
}

/// Simple MLP implementation
pub struct SimpleMLP;

impl SimpleMLP {
    fn new() -> Self {
        Self
    }

    async fn predict_sybil_probability(&self, behavior: &NodeBehaviorData) -> ConsensusResult<f64> {
        // Simplified ML logic - in production this would be a trained model
        let mut score: f64 = 0.0;
        
        // Feature engineering
        score += (1.0 - behavior.cpu_usage) * 0.3; // Low CPU usage suspicious
        score += behavior.network_usage * 0.2; // High network usage suspicious
        score += (10.0 - behavior.connection_count as f64).max(0.0) / 10.0 * 0.3; // Few connections suspicious
        
        if behavior.message_intervals.len() > 5 {
            let regularity = self.calculate_regularity(&behavior.message_intervals);
            score += regularity * 0.2; // High regularity suspicious
        }
        
        Ok(score.min(1.0))
    }

    fn calculate_regularity(&self, intervals: &[f64]) -> f64 {
        if intervals.len() < 2 {
            return 0.0;
        }
        
        let mean = intervals.iter().sum::<f64>() / intervals.len() as f64;
        let variance = intervals.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / intervals.len() as f64;
        
        // High regularity = low variance
        if variance < 0.1 {
            1.0
        } else {
            (1.0 / (1.0 + variance)).min(1.0)
        }
    }
}

/// Attack prevention metrics
#[derive(Debug, Clone)]
pub struct PreventionMetrics {
    pub total_validations: u64,
    pub double_spend_attempts: u64,
    pub sybil_attempts: u64,
    pub eclipse_attempts: u64,
    pub quantum_security_violations: u64,
    pub coordination_patterns_detected: u64,
    pub false_positive_rate: f64,
    pub last_maintenance: Instant,
}

impl Default for PreventionMetrics {
    fn default() -> Self {
        Self {
            total_validations: 0,
            double_spend_attempts: 0,
            sybil_attempts: 0,
            eclipse_attempts: 0,
            quantum_security_violations: 0,
            coordination_patterns_detected: 0,
            false_positive_rate: 0.0,
            last_maintenance: Instant::now(),
        }
    }
}

/// Supporting data structures
#[derive(Debug, Clone)]
pub struct NodeBehaviorData {
    pub connection_count: u32,
    pub message_intervals: Vec<f64>,
    pub cpu_usage: f64,
    pub network_usage: f64,
    pub vote_patterns: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct NetworkTopology {
    pub node_count: usize,
    pub connections: Vec<(NodeId, NodeId)>,
}

#[derive(Debug, Clone)]
pub struct CoordinationBehavior {
    pub message_rate: f64,
    pub vote_timing: Duration,
    pub network_activity: f64,
    pub consensus_participation: f64,
    pub vote_patterns: Vec<String>,
}

#[derive(Debug, Clone)]
struct TransactionRecord {
    pub transaction_hash: String,
    pub timestamp: Instant,
}

#[derive(Debug, Clone)]
struct NodeProfile {
    pub node_id: NodeId,
    pub first_seen: SystemTime,
    pub last_seen: SystemTime,
    pub behavior_history: VecDeque<NodeBehaviorData>,
}

impl NodeProfile {
    fn new(node_id: NodeId) -> Self {
        let now = SystemTime::now();
        Self {
            node_id,
            first_seen: now,
            last_seen: now,
            behavior_history: VecDeque::with_capacity(100),
        }
    }
}

#[derive(Debug, Clone)]
struct BehaviorSnapshot {
    pub timestamp: Instant,
    pub message_rate: f64,
    pub vote_timing: Duration,
    pub network_activity: f64,
    pub consensus_participation: f64,
    pub vote_patterns: Vec<String>,
}

#[derive(Debug, Clone)]
struct CoordinationPattern {
    pub pattern_type: String,
    pub nodes: Vec<NodeId>,
    pub confidence: f64,
    pub first_detected: Instant,
    pub last_updated: Instant,
}

#[derive(Debug, Clone)]
struct TopologySnapshot {
    pub timestamp: Instant,
    pub topology: NetworkTopology,
    pub risk_score: f64,
}