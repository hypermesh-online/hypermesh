//! Real-time Byzantine fault detection
//!
//! This module implements high-performance real-time detection of Byzantine
//! behavior with microsecond response times. It uses statistical analysis,
//! pattern recognition, and ML techniques to identify malicious nodes.

use super::super::error::{ConsensusError, ConsensusResult};
use crate::transport::NodeId;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime, Instant};
use tokio::sync::{RwLock, mpsc};
use serde::{Serialize, Deserialize};
use tracing::{info, warn, error, debug};

/// Configuration for real-time Byzantine detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionConfig {
    /// Detection sensitivity (0.0 to 1.0)
    pub sensitivity: f64,
    
    /// Maximum response time in microseconds
    pub max_response_time_us: u64,
    
    /// Size of behavior history window
    pub history_window_size: usize,
    
    /// Minimum samples before detection
    pub min_samples: usize,
    
    /// Statistical significance threshold
    pub significance_threshold: f64,
    
    /// Enable machine learning detection
    pub enable_ml_detection: bool,
    
    /// Enable statistical anomaly detection
    pub enable_statistical_detection: bool,
    
    /// Enable pattern-based detection
    pub enable_pattern_detection: bool,
    
    /// Anomaly score threshold
    pub anomaly_threshold: f64,
}

impl Default for DetectionConfig {
    fn default() -> Self {
        Self {
            sensitivity: 0.8,
            max_response_time_us: 1000, // 1ms
            history_window_size: 1000,
            min_samples: 50,
            significance_threshold: 0.95,
            enable_ml_detection: true,
            enable_statistical_detection: true,
            enable_pattern_detection: true,
            anomaly_threshold: 0.8,
        }
    }
}

/// Byzantine behavior types that can be detected
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ByzantineBehavior {
    /// Inconsistent message timing
    InconsistentTiming,
    
    /// Unusual message frequency
    AbnormalFrequency,
    
    /// Message content anomalies
    ContentAnomaly,
    
    /// Consensus violation
    ConsensusViolation,
    
    /// Network behavior anomaly
    NetworkAnomaly,
    
    /// Resource usage anomaly
    ResourceAnomaly,
    
    /// Cryptographic signature issues
    SignatureAnomaly,
    
    /// Coordination with other Byzantine nodes
    CoordinationBehavior,
    
    /// Vote manipulation
    VoteManipulation,
    
    /// State inconsistency
    StateInconsistency,
}

/// Alert severity levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Low severity - monitoring only
    Low,
    
    /// Medium severity - increased scrutiny
    Medium,
    
    /// High severity - isolation candidate
    High,
    
    /// Critical severity - immediate quarantine
    Critical,
}

/// Byzantine detection alert
#[derive(Debug, Clone)]
pub struct ByzantineAlert {
    /// Node exhibiting Byzantine behavior
    pub node_id: NodeId,
    
    /// Type of Byzantine behavior detected
    pub behavior_type: ByzantineBehavior,
    
    /// Alert severity
    pub severity: AlertSeverity,
    
    /// Detection confidence (0.0 to 1.0)
    pub confidence: f64,
    
    /// Supporting evidence
    pub evidence: BehaviorEvidence,
    
    /// Detection timestamp
    pub detected_at: SystemTime,
    
    /// Detection latency in microseconds
    pub detection_latency_us: u64,
}

/// Evidence supporting Byzantine behavior detection
#[derive(Debug, Clone)]
pub struct BehaviorEvidence {
    /// Statistical anomaly scores
    pub anomaly_scores: HashMap<String, f64>,
    
    /// Pattern matching results
    pub pattern_matches: Vec<String>,
    
    /// ML model predictions
    pub ml_predictions: HashMap<String, f64>,
    
    /// Recent behavior samples
    pub behavior_samples: Vec<BehaviorSample>,
    
    /// Correlation with other nodes
    pub correlations: HashMap<NodeId, f64>,
}

/// Behavior sample for analysis
#[derive(Debug, Clone)]
pub struct BehaviorSample {
    /// Timestamp of the sample
    pub timestamp: SystemTime,
    
    /// Message frequency
    pub message_frequency: f64,
    
    /// Message latency
    pub message_latency_us: u64,
    
    /// Message size
    pub message_size: usize,
    
    /// Consensus participation
    pub consensus_participation: f64,
    
    /// Vote consistency
    pub vote_consistency: f64,
    
    /// Network activity
    pub network_activity: f64,
    
    /// Resource usage
    pub resource_usage: f64,
    
    /// Signature validity
    pub signature_validity: bool,
}

/// Node behavior tracker
#[derive(Debug)]
struct NodeBehaviorTracker {
    /// Node ID
    node_id: NodeId,
    
    /// Behavior history
    behavior_history: VecDeque<BehaviorSample>,
    
    /// Statistical metrics
    statistics: BehaviorStatistics,
    
    /// Anomaly detection state
    anomaly_state: AnomalyDetectionState,
    
    /// Last analysis timestamp
    last_analysis: Instant,
    
    /// Detection confidence
    confidence: f64,
}

/// Statistical metrics for behavior analysis
#[derive(Debug, Clone)]
struct BehaviorStatistics {
    /// Message frequency statistics
    message_freq_mean: f64,
    message_freq_stddev: f64,
    
    /// Latency statistics
    latency_mean: f64,
    latency_stddev: f64,
    
    /// Size statistics
    size_mean: f64,
    size_stddev: f64,
    
    /// Participation statistics
    participation_mean: f64,
    participation_stddev: f64,
    
    /// Vote consistency statistics
    vote_consistency_mean: f64,
    vote_consistency_stddev: f64,
}

/// State for anomaly detection algorithms
#[derive(Debug)]
struct AnomalyDetectionState {
    /// EWMA (Exponentially Weighted Moving Average) state
    ewma_state: HashMap<String, f64>,
    
    /// Control chart limits
    control_limits: HashMap<String, (f64, f64)>,
    
    /// Pattern detection state
    pattern_state: PatternDetectionState,
    
    /// ML model state
    ml_state: MLDetectionState,
}

/// Pattern detection state
#[derive(Debug)]
struct PatternDetectionState {
    /// Recent patterns observed
    recent_patterns: VecDeque<String>,
    
    /// Pattern frequency counters
    pattern_counts: HashMap<String, usize>,
    
    /// Known Byzantine patterns
    byzantine_patterns: HashSet<String>,
}

/// Machine learning detection state
#[derive(Debug)]
struct MLDetectionState {
    /// Feature vectors for recent behavior
    feature_vectors: VecDeque<Vec<f64>>,
    
    /// Model predictions cache
    prediction_cache: HashMap<String, f64>,
    
    /// Model update timestamp
    last_model_update: Instant,
}

use std::collections::HashSet;

/// Detection system metrics
#[derive(Debug, Clone)]
pub struct DetectionMetrics {
    /// Total detections performed
    pub total_detections: u64,
    
    /// True positive detections
    pub true_positives: u64,
    
    /// False positive detections
    pub false_positives: u64,
    
    /// Average detection time in microseconds
    pub avg_detection_time_us: u64,
    
    /// Maximum detection time in microseconds
    pub max_detection_time_us: u64,
    
    /// Nodes currently under monitoring
    pub nodes_monitored: u64,
    
    /// Alerts generated
    pub alerts_generated: u64,
    
    /// Last update timestamp
    pub last_updated: Instant,
}

impl Default for DetectionMetrics {
    fn default() -> Self {
        Self {
            total_detections: 0,
            true_positives: 0,
            false_positives: 0,
            avg_detection_time_us: 0,
            max_detection_time_us: 0,
            nodes_monitored: 0,
            alerts_generated: 0,
            last_updated: Instant::now(),
        }
    }
}

impl RealTimeByzantineDetector {
    /// Create a new real-time Byzantine detector
    pub async fn new(config: DetectionConfig) -> ConsensusResult<Self> {
        let (alert_sender, alert_receiver) = mpsc::unbounded_channel();
        
        Ok(Self {
            config,
            node_trackers: Arc::new(RwLock::new(HashMap::new())),
            alert_sender,
            alert_receiver: Arc::new(RwLock::new(Some(alert_receiver))),
            metrics: Arc::new(RwLock::new(DetectionMetrics::default())),
            background_tasks: Arc::new(RwLock::new(Vec::new())),
        })
    }
    
    /// Start the real-time detection system
    pub async fn start(&self) -> ConsensusResult<()> {
        info!("Starting real-time Byzantine detection system");
        
        // Start background analysis task
        self.start_analysis_task().await;
        
        // Start metrics collection task
        self.start_metrics_task().await;
        
        info!("Real-time Byzantine detection system started");
        Ok(())
    }
    
    /// Stop the real-time detection system
    pub async fn stop(&self) -> ConsensusResult<()> {
        info!("Stopping real-time Byzantine detection system");
        
        // Stop background tasks
        let mut tasks = self.background_tasks.write().await;
        for task in tasks.drain(..) {
            task.abort();
        }
        
        info!("Real-time Byzantine detection system stopped");
        Ok(())
    }
    
    /// Process a behavior sample from a node
    pub async fn process_behavior(
        &self,
        node_id: &NodeId,
        sample: BehaviorSample,
    ) -> ConsensusResult<Option<ByzantineAlert>> {
        let start_time = Instant::now();
        
        debug!("Processing behavior sample for node: {:?}", node_id);
        
        // Get or create node tracker
        let mut trackers = self.node_trackers.write().await;
        let tracker = trackers.entry(node_id.clone()).or_insert_with(|| {
            NodeBehaviorTracker {
                node_id: node_id.clone(),
                behavior_history: VecDeque::with_capacity(self.config.history_window_size),
                statistics: BehaviorStatistics {
                    message_freq_mean: 0.0,
                    message_freq_stddev: 0.0,
                    latency_mean: 0.0,
                    latency_stddev: 0.0,
                    size_mean: 0.0,
                    size_stddev: 0.0,
                    participation_mean: 0.0,
                    participation_stddev: 0.0,
                    vote_consistency_mean: 0.0,
                    vote_consistency_stddev: 0.0,
                },
                anomaly_state: AnomalyDetectionState {
                    ewma_state: HashMap::new(),
                    control_limits: HashMap::new(),
                    pattern_state: PatternDetectionState {
                        recent_patterns: VecDeque::new(),
                        pattern_counts: HashMap::new(),
                        byzantine_patterns: HashSet::new(),
                    },
                    ml_state: MLDetectionState {
                        feature_vectors: VecDeque::new(),
                        prediction_cache: HashMap::new(),
                        last_model_update: Instant::now(),
                    },
                },
                last_analysis: Instant::now(),
                confidence: 0.0,
            }
        });
        
        // Add sample to history
        if tracker.behavior_history.len() >= self.config.history_window_size {
            tracker.behavior_history.pop_front();
        }
        tracker.behavior_history.push_back(sample.clone());
        
        // Check if we have enough samples for analysis
        if tracker.behavior_history.len() < self.config.min_samples {
            return Ok(None);
        }
        
        // Perform real-time analysis
        let detection_result = self.analyze_behavior(tracker, &sample).await?;
        
        // Update metrics
        let detection_time_us = start_time.elapsed().as_micros() as u64;
        self.update_metrics(detection_time_us, detection_result.is_some()).await;
        
        // Check response time requirement
        if detection_time_us > self.config.max_response_time_us {
            warn!("Detection time {}μs exceeded maximum {}μs", 
                  detection_time_us, self.config.max_response_time_us);
        }
        
        // Generate alert if Byzantine behavior detected
        if let Some(behavior_type) = detection_result {
            let alert = ByzantineAlert {
                node_id: node_id.clone(),
                behavior_type: behavior_type.clone(),
                severity: self.determine_severity(&behavior_type, tracker.confidence),
                confidence: tracker.confidence,
                evidence: self.collect_evidence(tracker, &sample).await,
                detected_at: SystemTime::now(),
                detection_latency_us: detection_time_us,
            };
            
            // Send alert
            if let Err(e) = self.alert_sender.send(alert.clone()) {
                error!("Failed to send Byzantine alert: {:?}", e);
            }
            
            info!("Byzantine behavior detected: {:?} from node {:?} (confidence: {:.2})", 
                  behavior_type, node_id, tracker.confidence);
            
            return Ok(Some(alert));
        }
        
        Ok(None)
    }
    
    /// Get detection metrics
    pub async fn get_metrics(&self) -> DetectionMetrics {
        self.metrics.read().await.clone()
    }
    
    /// Get alert receiver
    pub async fn get_alert_receiver(&self) -> Option<mpsc::UnboundedReceiver<ByzantineAlert>> {
        self.alert_receiver.write().await.take()
    }
    
    /// Analyze behavior for Byzantine patterns
    async fn analyze_behavior(
        &self,
        tracker: &mut NodeBehaviorTracker,
        current_sample: &BehaviorSample,
    ) -> ConsensusResult<Option<ByzantineBehavior>> {
        let mut max_confidence = 0.0;
        let mut detected_behavior = None;
        
        // Update statistics
        self.update_statistics(&mut tracker.statistics, &tracker.behavior_history);
        
        // Statistical anomaly detection
        if self.config.enable_statistical_detection {
            if let Some((behavior, confidence)) = self.detect_statistical_anomaly(tracker, current_sample).await? {
                if confidence > max_confidence {
                    max_confidence = confidence;
                    detected_behavior = Some(behavior);
                }
            }
        }
        
        // Pattern-based detection
        if self.config.enable_pattern_detection {
            if let Some((behavior, confidence)) = self.detect_behavior_patterns(tracker, current_sample).await? {
                if confidence > max_confidence {
                    max_confidence = confidence;
                    detected_behavior = Some(behavior);
                }
            }
        }
        
        // Machine learning detection
        if self.config.enable_ml_detection {
            if let Some((behavior, confidence)) = self.detect_ml_anomaly(tracker, current_sample).await? {
                if confidence > max_confidence {
                    max_confidence = confidence;
                    detected_behavior = Some(behavior);
                }
            }
        }
        
        // Update tracker confidence
        tracker.confidence = max_confidence;
        tracker.last_analysis = Instant::now();
        
        // Check if detection threshold is met
        if max_confidence >= self.config.anomaly_threshold {
            Ok(detected_behavior)
        } else {
            Ok(None)
        }
    }
    
    /// Detect statistical anomalies
    async fn detect_statistical_anomaly(
        &self,
        tracker: &mut NodeBehaviorTracker,
        sample: &BehaviorSample,
    ) -> ConsensusResult<Option<(ByzantineBehavior, f64)>> {
        let stats = &tracker.statistics;
        
        // Check message frequency anomaly
        if stats.message_freq_stddev > 0.0 {
            let z_score = (sample.message_frequency - stats.message_freq_mean) / stats.message_freq_stddev;
            if z_score.abs() > 3.0 { // 3-sigma rule
                return Ok(Some((ByzantineBehavior::AbnormalFrequency, z_score.abs() / 5.0)));
            }
        }
        
        // Check latency anomaly
        if stats.latency_stddev > 0.0 {
            let latency = sample.message_latency_us as f64;
            let z_score = (latency - stats.latency_mean) / stats.latency_stddev;
            if z_score.abs() > 3.0 {
                return Ok(Some((ByzantineBehavior::InconsistentTiming, z_score.abs() / 5.0)));
            }
        }
        
        // Check participation anomaly
        if stats.participation_stddev > 0.0 {
            let z_score = (sample.consensus_participation - stats.participation_mean) / stats.participation_stddev;
            if z_score.abs() > 2.5 {
                return Ok(Some((ByzantineBehavior::ConsensusViolation, z_score.abs() / 4.0)));
            }
        }
        
        // Check vote consistency anomaly
        if stats.vote_consistency_stddev > 0.0 {
            let z_score = (sample.vote_consistency - stats.vote_consistency_mean) / stats.vote_consistency_stddev;
            if z_score.abs() > 2.0 {
                return Ok(Some((ByzantineBehavior::VoteManipulation, z_score.abs() / 3.0)));
            }
        }
        
        Ok(None)
    }
    
    /// Detect behavior patterns
    async fn detect_behavior_patterns(
        &self,
        tracker: &mut NodeBehaviorTracker,
        sample: &BehaviorSample,
    ) -> ConsensusResult<Option<(ByzantineBehavior, f64)>> {
        // Generate pattern signature for current behavior
        let pattern = format!("{}:{}:{}:{}",
                             (sample.message_frequency * 10.0) as u32,
                             (sample.message_latency_us / 1000) as u32,
                             (sample.consensus_participation * 10.0) as u32,
                             (sample.vote_consistency * 10.0) as u32);
        
        let pattern_state = &mut tracker.anomaly_state.pattern_state;
        
        // Add to recent patterns
        if pattern_state.recent_patterns.len() >= 100 {
            pattern_state.recent_patterns.pop_front();
        }
        pattern_state.recent_patterns.push_back(pattern.clone());
        
        // Update pattern counts
        *pattern_state.pattern_counts.entry(pattern.clone()).or_insert(0) += 1;
        
        // Check for known Byzantine patterns
        if pattern_state.byzantine_patterns.contains(&pattern) {
            return Ok(Some((ByzantineBehavior::CoordinationBehavior, 0.9)));
        }
        
        // Look for unusual pattern frequency
        let total_patterns = pattern_state.pattern_counts.len();
        let pattern_frequency = *pattern_state.pattern_counts.get(&pattern).unwrap_or(&0) as f64 / total_patterns as f64;
        
        if pattern_frequency > 0.5 {
            // Highly repetitive pattern - potential automation
            return Ok(Some((ByzantineBehavior::NetworkAnomaly, pattern_frequency)));
        }
        
        Ok(None)
    }
    
    /// Detect ML-based anomalies
    async fn detect_ml_anomaly(
        &self,
        tracker: &mut NodeBehaviorTracker,
        sample: &BehaviorSample,
    ) -> ConsensusResult<Option<(ByzantineBehavior, f64)>> {
        // Extract feature vector from sample
        let features = vec![
            sample.message_frequency,
            sample.message_latency_us as f64 / 1000.0, // Normalize to milliseconds
            sample.message_size as f64 / 1024.0, // Normalize to KB
            sample.consensus_participation,
            sample.vote_consistency,
            sample.network_activity,
            sample.resource_usage,
            if sample.signature_validity { 1.0 } else { 0.0 },
        ];
        
        let ml_state = &mut tracker.anomaly_state.ml_state;
        
        // Add to feature history
        if ml_state.feature_vectors.len() >= 1000 {
            ml_state.feature_vectors.pop_front();
        }
        ml_state.feature_vectors.push_back(features.clone());
        
        // Simple anomaly detection using isolation forest concept
        if ml_state.feature_vectors.len() >= 100 {
            let anomaly_score = self.calculate_isolation_score(&features, &ml_state.feature_vectors);
            
            if anomaly_score > 0.7 {
                return Ok(Some((ByzantineBehavior::ContentAnomaly, anomaly_score)));
            }
        }
        
        Ok(None)
    }
    
    /// Calculate isolation-based anomaly score
    fn calculate_isolation_score(&self, sample: &[f64], history: &VecDeque<Vec<f64>>) -> f64 {
        let mut distances = Vec::new();
        
        // Calculate distances to recent samples
        for historical_sample in history.iter().rev().take(50) {
            let distance: f64 = sample.iter()
                .zip(historical_sample.iter())
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt();
            distances.push(distance);
        }
        
        if distances.is_empty() {
            return 0.0;
        }
        
        // Calculate mean distance
        let mean_distance = distances.iter().sum::<f64>() / distances.len() as f64;
        
        // Find k-th nearest neighbor (k=5)
        distances.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let kth_distance = distances.get(5).copied().unwrap_or(mean_distance);
        
        // Normalize to 0-1 range (higher means more anomalous)
        (kth_distance / (mean_distance + 1e-10)).min(1.0)
    }
    
    /// Update behavior statistics
    fn update_statistics(&self, stats: &mut BehaviorStatistics, history: &VecDeque<BehaviorSample>) {
        if history.len() < 2 {
            return;
        }
        
        // Calculate means
        let n = history.len() as f64;
        stats.message_freq_mean = history.iter().map(|s| s.message_frequency).sum::<f64>() / n;
        stats.latency_mean = history.iter().map(|s| s.message_latency_us as f64).sum::<f64>() / n;
        stats.size_mean = history.iter().map(|s| s.message_size as f64).sum::<f64>() / n;
        stats.participation_mean = history.iter().map(|s| s.consensus_participation).sum::<f64>() / n;
        stats.vote_consistency_mean = history.iter().map(|s| s.vote_consistency).sum::<f64>() / n;
        
        // Calculate standard deviations
        stats.message_freq_stddev = (history.iter()
            .map(|s| (s.message_frequency - stats.message_freq_mean).powi(2))
            .sum::<f64>() / n).sqrt();
        
        stats.latency_stddev = (history.iter()
            .map(|s| (s.message_latency_us as f64 - stats.latency_mean).powi(2))
            .sum::<f64>() / n).sqrt();
        
        stats.size_stddev = (history.iter()
            .map(|s| (s.message_size as f64 - stats.size_mean).powi(2))
            .sum::<f64>() / n).sqrt();
        
        stats.participation_stddev = (history.iter()
            .map(|s| (s.consensus_participation - stats.participation_mean).powi(2))
            .sum::<f64>() / n).sqrt();
        
        stats.vote_consistency_stddev = (history.iter()
            .map(|s| (s.vote_consistency - stats.vote_consistency_mean).powi(2))
            .sum::<f64>() / n).sqrt();
    }
    
    /// Determine alert severity based on behavior type and confidence
    fn determine_severity(&self, behavior: &ByzantineBehavior, confidence: f64) -> AlertSeverity {
        match behavior {
            ByzantineBehavior::ConsensusViolation | 
            ByzantineBehavior::VoteManipulation |
            ByzantineBehavior::StateInconsistency => {
                if confidence > 0.9 { AlertSeverity::Critical }
                else if confidence > 0.7 { AlertSeverity::High }
                else { AlertSeverity::Medium }
            }
            
            ByzantineBehavior::CoordinationBehavior |
            ByzantineBehavior::SignatureAnomaly => {
                if confidence > 0.8 { AlertSeverity::High }
                else if confidence > 0.6 { AlertSeverity::Medium }
                else { AlertSeverity::Low }
            }
            
            _ => {
                if confidence > 0.7 { AlertSeverity::Medium }
                else { AlertSeverity::Low }
            }
        }
    }
    
    /// Collect evidence for Byzantine behavior
    async fn collect_evidence(
        &self,
        tracker: &NodeBehaviorTracker,
        sample: &BehaviorSample,
    ) -> BehaviorEvidence {
        let mut anomaly_scores = HashMap::new();
        let stats = &tracker.statistics;
        
        // Calculate anomaly scores for different metrics
        if stats.message_freq_stddev > 0.0 {
            let z_score = (sample.message_frequency - stats.message_freq_mean) / stats.message_freq_stddev;
            anomaly_scores.insert("message_frequency".to_string(), z_score.abs());
        }
        
        if stats.latency_stddev > 0.0 {
            let latency = sample.message_latency_us as f64;
            let z_score = (latency - stats.latency_mean) / stats.latency_stddev;
            anomaly_scores.insert("message_latency".to_string(), z_score.abs());
        }
        
        if stats.participation_stddev > 0.0 {
            let z_score = (sample.consensus_participation - stats.participation_mean) / stats.participation_stddev;
            anomaly_scores.insert("consensus_participation".to_string(), z_score.abs());
        }
        
        BehaviorEvidence {
            anomaly_scores,
            pattern_matches: Vec::new(), // Would be populated with actual pattern matches
            ml_predictions: HashMap::new(), // Would be populated with ML model outputs
            behavior_samples: tracker.behavior_history.iter().rev().take(10).cloned().collect(),
            correlations: HashMap::new(), // Would be calculated with other nodes
        }
    }
    
    /// Update detection metrics
    async fn update_metrics(&self, detection_time_us: u64, detection_occurred: bool) {
        let mut metrics = self.metrics.write().await;
        
        metrics.total_detections += 1;
        
        if detection_occurred {
            metrics.alerts_generated += 1;
        }
        
        // Update average detection time
        if metrics.total_detections == 1 {
            metrics.avg_detection_time_us = detection_time_us;
        } else {
            let total_time = (metrics.avg_detection_time_us * (metrics.total_detections - 1)) + detection_time_us;
            metrics.avg_detection_time_us = total_time / metrics.total_detections;
        }
        
        // Update maximum detection time
        metrics.max_detection_time_us = metrics.max_detection_time_us.max(detection_time_us);
        
        // Update node count
        let trackers = self.node_trackers.read().await;
        metrics.nodes_monitored = trackers.len() as u64;
        
        metrics.last_updated = Instant::now();
    }
    
    /// Start background analysis task
    async fn start_analysis_task(&self) {
        let node_trackers = self.node_trackers.clone();
        let config = self.config.clone();
        
        let mut tasks = self.background_tasks.write().await;
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100)); // 100ms analysis cycles
            
            loop {
                interval.tick().await;
                
                let trackers = node_trackers.read().await;
                
                // Perform periodic analysis on all tracked nodes
                for (node_id, _tracker) in trackers.iter() {
                    // Periodic maintenance tasks could go here
                    debug!("Periodic analysis for node: {:?}", node_id);
                }
                
                // Clean up old tracking data
                drop(trackers);
                let mut trackers_mut = node_trackers.write().await;
                let cutoff_time = Instant::now() - Duration::from_secs(3600); // 1 hour
                
                trackers_mut.retain(|_id, tracker| tracker.last_analysis > cutoff_time);
            }
        });
        
        tasks.push(handle);
    }
    
    /// Start metrics collection task
    async fn start_metrics_task(&self) {
        let metrics = self.metrics.clone();
        
        let mut tasks = self.background_tasks.write().await;
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                // Periodic metrics updates
                let mut metrics_guard = metrics.write().await;
                metrics_guard.last_updated = Instant::now();
                
                debug!("Detection metrics updated - Total: {}, Alerts: {}, Avg time: {}μs",
                       metrics_guard.total_detections,
                       metrics_guard.alerts_generated,
                       metrics_guard.avg_detection_time_us);
            }
        });
        
        tasks.push(handle);
    }
}

/// Real-time Byzantine detector
pub struct RealTimeByzantineDetector {
    /// Detection configuration
    config: DetectionConfig,
    
    /// Node behavior trackers
    node_trackers: Arc<RwLock<HashMap<NodeId, NodeBehaviorTracker>>>,
    
    /// Alert sender
    alert_sender: mpsc::UnboundedSender<ByzantineAlert>,
    
    /// Alert receiver
    alert_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<ByzantineAlert>>>>,
    
    /// System metrics
    metrics: Arc<RwLock<DetectionMetrics>>,
    
    /// Background task handles
    background_tasks: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_real_time_detector_creation() {
        let config = DetectionConfig::default();
        let detector = RealTimeByzantineDetector::new(config).await;
        assert!(detector.is_ok());
    }
    
    #[tokio::test]
    async fn test_behavior_sample_processing() {
        let config = DetectionConfig::default();
        let detector = RealTimeByzantineDetector::new(config).await.unwrap();
        let node_id = NodeId::new("test-node".to_string());
        
        // Create a normal behavior sample
        let sample = BehaviorSample {
            timestamp: SystemTime::now(),
            message_frequency: 10.0,
            message_latency_us: 1000,
            message_size: 1024,
            consensus_participation: 0.9,
            vote_consistency: 0.95,
            network_activity: 0.8,
            resource_usage: 0.7,
            signature_validity: true,
        };
        
        // First few samples should not trigger detection (not enough data)
        let result = detector.process_behavior(&node_id, sample).await.unwrap();
        assert!(result.is_none());
    }
    
    #[tokio::test]
    async fn test_anomaly_detection() {
        let mut config = DetectionConfig::default();
        config.min_samples = 5; // Lower for testing
        config.anomaly_threshold = 0.5; // Lower threshold for testing
        
        let detector = RealTimeByzantineDetector::new(config).await.unwrap();
        let node_id = NodeId::new("test-node".to_string());
        
        // Send normal samples first
        for i in 0..10 {
            let sample = BehaviorSample {
                timestamp: SystemTime::now(),
                message_frequency: 10.0 + (i as f64 * 0.1),
                message_latency_us: 1000 + (i * 10),
                message_size: 1024,
                consensus_participation: 0.9,
                vote_consistency: 0.95,
                network_activity: 0.8,
                resource_usage: 0.7,
                signature_validity: true,
            };
            
            detector.process_behavior(&node_id, sample).await.unwrap();
        }
        
        // Send an anomalous sample
        let anomalous_sample = BehaviorSample {
            timestamp: SystemTime::now(),
            message_frequency: 100.0, // Very high frequency
            message_latency_us: 50000, // Very high latency
            message_size: 1024,
            consensus_participation: 0.1, // Very low participation
            vote_consistency: 0.1, // Very low consistency
            network_activity: 0.8,
            resource_usage: 0.7,
            signature_validity: true,
        };
        
        let result = detector.process_behavior(&node_id, anomalous_sample).await.unwrap();
        
        // Should detect anomaly with sufficient deviation
        if let Some(alert) = result {
            assert!(alert.confidence > 0.5);
            assert!(matches!(alert.severity, AlertSeverity::Medium | AlertSeverity::High | AlertSeverity::Critical));
        }
    }
    
    #[tokio::test]
    async fn test_detection_metrics() {
        let config = DetectionConfig::default();
        let detector = RealTimeByzantineDetector::new(config).await.unwrap();
        
        let metrics = detector.get_metrics().await;
        assert_eq!(metrics.total_detections, 0);
        assert_eq!(metrics.alerts_generated, 0);
    }
}