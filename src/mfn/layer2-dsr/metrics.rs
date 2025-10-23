//! Performance Metrics and Monitoring for DSR
//!
//! Provides comprehensive metrics collection and analysis for the 
//! Dynamic Similarity Reservoir neural network system.

use anyhow::Result;
use prometheus::{Counter, Gauge, Histogram, IntCounter, IntGauge, Registry};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tracing::{debug, info};

/// Comprehensive DSR metrics collector
pub struct DsrMetrics {
    /// Prometheus registry for metrics export
    registry: Arc<Registry>,
    
    /// Neural network metrics
    neural_processing_time: Histogram,
    neural_similarity_scores: Histogram,
    neural_confidence_levels: Histogram,
    neuron_activation_count: IntCounter,
    synaptic_weight_updates: IntCounter,
    spike_events_total: IntCounter,
    
    /// Adaptation metrics
    stdp_potentiation_events: IntCounter,
    stdp_depression_events: IntCounter,
    online_learning_updates: IntCounter,
    adaptation_convergence_time: Histogram,
    learning_rate_adjustments: IntCounter,
    
    /// Routing optimization metrics
    routing_decisions_total: IntCounter,
    routing_decision_time: Histogram,
    routing_confidence_score: Histogram,
    path_optimization_improvements: Histogram,
    cache_hit_rate: Gauge,
    
    /// Service mesh metrics
    load_balance_decisions: IntCounter,
    circuit_breaker_activations: IntCounter,
    service_recommendations_generated: IntCounter,
    endpoint_prediction_accuracy: Histogram,
    
    /// Performance targets tracking
    similarity_detection_latency: Histogram,
    adaptation_rate_performance: Histogram,
    pattern_recognition_accuracy: Histogram,
    memory_usage_bytes: IntGauge,
    
    /// Historical data for trend analysis
    performance_history: Arc<Mutex<PerformanceHistory>>,
    
    /// Custom metrics
    custom_counters: HashMap<String, Counter>,
    custom_gauges: HashMap<String, Gauge>,
    custom_histograms: HashMap<String, Histogram>,
}

/// Historical performance data for trend analysis
#[derive(Debug, Clone)]
struct PerformanceHistory {
    similarity_detection_times: VecDeque<(SystemTime, f64)>,
    adaptation_rates: VecDeque<(SystemTime, f64)>,
    pattern_recognition_scores: VecDeque<(SystemTime, f64)>,
    memory_usage_samples: VecDeque<(SystemTime, f64)>,
    routing_improvements: VecDeque<(SystemTime, f64)>,
    cache_performance: VecDeque<(SystemTime, f64)>,
    max_history_size: usize,
}

/// Metrics summary for reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub neural_network: NeuralNetworkMetrics,
    pub adaptation: AdaptationMetrics,
    pub routing: RoutingMetrics,
    pub service_mesh: ServiceMeshMetrics,
    pub performance_targets: PerformanceTargets,
    pub system_health: SystemHealthMetrics,
}

/// Neural network specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralNetworkMetrics {
    pub total_processing_operations: u64,
    pub average_processing_time_ms: f64,
    pub average_similarity_score: f64,
    pub average_confidence: f64,
    pub total_spike_events: u64,
    pub neuron_activation_rate: f64,
    pub synaptic_weight_changes: u64,
}

/// Adaptation engine metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptationMetrics {
    pub stdp_potentiation_events: u64,
    pub stdp_depression_events: u64,
    pub online_learning_updates: u64,
    pub average_convergence_time_ms: f64,
    pub learning_rate_adjustments: u64,
    pub adaptation_success_rate: f64,
}

/// Routing optimization metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingMetrics {
    pub total_routing_decisions: u64,
    pub average_decision_time_ms: f64,
    pub average_confidence_score: f64,
    pub path_improvements_achieved: f64,
    pub cache_hit_rate_percentage: f64,
    pub routing_accuracy: f64,
}

/// Service mesh intelligence metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMeshMetrics {
    pub load_balance_decisions: u64,
    pub circuit_breaker_activations: u64,
    pub service_recommendations: u64,
    pub endpoint_prediction_accuracy: f64,
    pub service_health_score: f64,
}

/// Performance target tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTargets {
    /// Target: <1ms neural similarity detection
    pub similarity_detection_target_ms: f64,
    pub similarity_detection_current_ms: f64,
    pub similarity_detection_achievement_rate: f64,
    
    /// Target: <100ms adaptation rate
    pub adaptation_rate_target_ms: f64,
    pub adaptation_rate_current_ms: f64,
    pub adaptation_rate_achievement_rate: f64,
    
    /// Target: >95% pattern recognition accuracy
    pub pattern_recognition_target_accuracy: f64,
    pub pattern_recognition_current_accuracy: f64,
    pub pattern_recognition_achievement_rate: f64,
    
    /// Target: <100MB neural network state
    pub memory_usage_target_mb: f64,
    pub memory_usage_current_mb: f64,
    pub memory_usage_achievement_rate: f64,
    
    /// Target: <5 minutes learning convergence
    pub learning_convergence_target_minutes: f64,
    pub learning_convergence_current_minutes: f64,
    pub learning_convergence_achievement_rate: f64,
}

/// System health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealthMetrics {
    pub overall_health_score: f64,
    pub uptime_seconds: f64,
    pub error_rate: f64,
    pub throughput_operations_per_second: f64,
    pub resource_utilization: f64,
    pub performance_stability: f64,
}

impl DsrMetrics {
    pub fn new() -> Result<Self> {
        let registry = Arc::new(Registry::new());
        
        // Neural network metrics
        let neural_processing_time = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "dsr_neural_processing_time_seconds",
                "Time taken for neural network processing"
            ).buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1])
        )?;
        registry.register(Box::new(neural_processing_time.clone()))?;
        
        let neural_similarity_scores = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "dsr_neural_similarity_scores",
                "Distribution of neural similarity scores"
            ).buckets(vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0])
        )?;
        registry.register(Box::new(neural_similarity_scores.clone()))?;
        
        let neural_confidence_levels = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "dsr_neural_confidence_levels",
                "Distribution of neural confidence levels"
            ).buckets(vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0])
        )?;
        registry.register(Box::new(neural_confidence_levels.clone()))?;
        
        let neuron_activation_count = IntCounter::new(
            "dsr_neuron_activations_total",
            "Total number of neuron activations"
        )?;
        registry.register(Box::new(neuron_activation_count.clone()))?;
        
        let synaptic_weight_updates = IntCounter::new(
            "dsr_synaptic_weight_updates_total",
            "Total number of synaptic weight updates"
        )?;
        registry.register(Box::new(synaptic_weight_updates.clone()))?;
        
        let spike_events_total = IntCounter::new(
            "dsr_spike_events_total",
            "Total number of spike events"
        )?;
        registry.register(Box::new(spike_events_total.clone()))?;
        
        // Adaptation metrics
        let stdp_potentiation_events = IntCounter::new(
            "dsr_stdp_potentiation_events_total",
            "Total STDP potentiation events"
        )?;
        registry.register(Box::new(stdp_potentiation_events.clone()))?;
        
        let stdp_depression_events = IntCounter::new(
            "dsr_stdp_depression_events_total", 
            "Total STDP depression events"
        )?;
        registry.register(Box::new(stdp_depression_events.clone()))?;
        
        let online_learning_updates = IntCounter::new(
            "dsr_online_learning_updates_total",
            "Total online learning updates"
        )?;
        registry.register(Box::new(online_learning_updates.clone()))?;
        
        let adaptation_convergence_time = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "dsr_adaptation_convergence_time_seconds",
                "Time for adaptation convergence"
            ).buckets(vec![1.0, 5.0, 10.0, 30.0, 60.0, 300.0, 600.0])
        )?;
        registry.register(Box::new(adaptation_convergence_time.clone()))?;
        
        let learning_rate_adjustments = IntCounter::new(
            "dsr_learning_rate_adjustments_total",
            "Total learning rate adjustments"
        )?;
        registry.register(Box::new(learning_rate_adjustments.clone()))?;
        
        // Routing metrics
        let routing_decisions_total = IntCounter::new(
            "dsr_routing_decisions_total",
            "Total routing decisions made"
        )?;
        registry.register(Box::new(routing_decisions_total.clone()))?;
        
        let routing_decision_time = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "dsr_routing_decision_time_seconds",
                "Time taken for routing decisions"
            ).buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05])
        )?;
        registry.register(Box::new(routing_decision_time.clone()))?;
        
        let routing_confidence_score = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "dsr_routing_confidence_score",
                "Confidence scores for routing decisions"
            ).buckets(vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0])
        )?;
        registry.register(Box::new(routing_confidence_score.clone()))?;
        
        let path_optimization_improvements = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "dsr_path_optimization_improvements",
                "Path optimization improvement ratios"
            ).buckets(vec![1.0, 2.0, 3.0, 5.0, 7.77, 10.0, 15.0])
        )?;
        registry.register(Box::new(path_optimization_improvements.clone()))?;
        
        let cache_hit_rate = Gauge::new(
            "dsr_cache_hit_rate",
            "Cache hit rate percentage"
        )?;
        registry.register(Box::new(cache_hit_rate.clone()))?;
        
        // Service mesh metrics
        let load_balance_decisions = IntCounter::new(
            "dsr_load_balance_decisions_total",
            "Total load balancing decisions"
        )?;
        registry.register(Box::new(load_balance_decisions.clone()))?;
        
        let circuit_breaker_activations = IntCounter::new(
            "dsr_circuit_breaker_activations_total",
            "Total circuit breaker activations"
        )?;
        registry.register(Box::new(circuit_breaker_activations.clone()))?;
        
        let service_recommendations_generated = IntCounter::new(
            "dsr_service_recommendations_total",
            "Total service recommendations generated"
        )?;
        registry.register(Box::new(service_recommendations_generated.clone()))?;
        
        let endpoint_prediction_accuracy = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "dsr_endpoint_prediction_accuracy",
                "Endpoint prediction accuracy scores"
            ).buckets(vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0])
        )?;
        registry.register(Box::new(endpoint_prediction_accuracy.clone()))?;
        
        // Performance target metrics
        let similarity_detection_latency = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "dsr_similarity_detection_latency_seconds",
                "Latency for similarity detection (target: <1ms)"
            ).buckets(vec![0.0001, 0.0005, 0.001, 0.002, 0.005, 0.01])
        )?;
        registry.register(Box::new(similarity_detection_latency.clone()))?;
        
        let adaptation_rate_performance = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "dsr_adaptation_rate_seconds",
                "Adaptation rate performance (target: <100ms)"
            ).buckets(vec![0.01, 0.05, 0.1, 0.2, 0.5, 1.0])
        )?;
        registry.register(Box::new(adaptation_rate_performance.clone()))?;
        
        let pattern_recognition_accuracy = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "dsr_pattern_recognition_accuracy",
                "Pattern recognition accuracy (target: >95%)"
            ).buckets(vec![0.5, 0.6, 0.7, 0.8, 0.85, 0.9, 0.95, 0.98, 0.99, 1.0])
        )?;
        registry.register(Box::new(pattern_recognition_accuracy.clone()))?;
        
        let memory_usage_bytes = IntGauge::new(
            "dsr_memory_usage_bytes",
            "Neural network memory usage in bytes (target: <100MB)"
        )?;
        registry.register(Box::new(memory_usage_bytes.clone()))?;
        
        let performance_history = Arc::new(Mutex::new(PerformanceHistory::new()));
        
        Ok(Self {
            registry,
            neural_processing_time,
            neural_similarity_scores,
            neural_confidence_levels,
            neuron_activation_count,
            synaptic_weight_updates,
            spike_events_total,
            stdp_potentiation_events,
            stdp_depression_events,
            online_learning_updates,
            adaptation_convergence_time,
            learning_rate_adjustments,
            routing_decisions_total,
            routing_decision_time,
            routing_confidence_score,
            path_optimization_improvements,
            cache_hit_rate,
            load_balance_decisions,
            circuit_breaker_activations,
            service_recommendations_generated,
            endpoint_prediction_accuracy,
            similarity_detection_latency,
            adaptation_rate_performance,
            pattern_recognition_accuracy,
            memory_usage_bytes,
            performance_history,
            custom_counters: HashMap::new(),
            custom_gauges: HashMap::new(),
            custom_histograms: HashMap::new(),
        })
    }
    
    /// Record neural processing time
    pub fn record_processing_time(&self, duration: Duration) {
        let seconds = duration.as_secs_f64();
        self.neural_processing_time.observe(seconds);
        self.similarity_detection_latency.observe(seconds);
        
        // Update performance history
        if let Ok(mut history) = self.performance_history.try_lock() {
            history.add_similarity_detection_time(seconds * 1000.0); // Convert to ms
        }
    }
    
    /// Record similarity score
    pub fn record_similarity_score(&self, score: f64) {
        self.neural_similarity_scores.observe(score);
    }
    
    /// Record confidence level
    pub fn record_confidence(&self, confidence: f64) {
        self.neural_confidence_levels.observe(confidence);
    }
    
    /// Increment neuron activation count
    pub fn increment_neuron_activations(&self, count: u64) {
        self.neuron_activation_count.inc_by(count);
    }
    
    /// Increment spike events
    pub fn increment_spike_events(&self, count: u64) {
        self.spike_events_total.inc_by(count);
    }
    
    /// Increment synaptic weight updates
    pub fn increment_weight_updates(&self, count: u64) {
        self.synaptic_weight_updates.inc_by(count);
    }
    
    /// Record STDP events
    pub fn record_stdp_events(&self, potentiation: u64, depression: u64) {
        self.stdp_potentiation_events.inc_by(potentiation);
        self.stdp_depression_events.inc_by(depression);
    }
    
    /// Record adaptation convergence time
    pub fn record_adaptation_convergence(&self, duration: Duration) {
        let seconds = duration.as_secs_f64();
        self.adaptation_convergence_time.observe(seconds);
        self.adaptation_rate_performance.observe(seconds);
        
        if let Ok(mut history) = self.performance_history.try_lock() {
            history.add_adaptation_rate(seconds * 1000.0); // Convert to ms
        }
    }
    
    /// Record routing decision
    pub fn record_routing_decision(&self, decision_time: Duration, confidence: f64, improvement: f64) {
        self.routing_decisions_total.inc();
        self.routing_decision_time.observe(decision_time.as_secs_f64());
        self.routing_confidence_score.observe(confidence);
        self.path_optimization_improvements.observe(improvement);
        
        if let Ok(mut history) = self.performance_history.try_lock() {
            history.add_routing_improvement(improvement);
        }
    }
    
    /// Update cache hit rate
    pub fn update_cache_hit_rate(&self, hit_rate: f64) {
        self.cache_hit_rate.set(hit_rate);
        
        if let Ok(mut history) = self.performance_history.try_lock() {
            history.add_cache_performance(hit_rate);
        }
    }
    
    /// Record pattern recognition accuracy
    pub fn record_pattern_recognition_accuracy(&self, accuracy: f64) {
        self.pattern_recognition_accuracy.observe(accuracy);
        
        if let Ok(mut history) = self.performance_history.try_lock() {
            history.add_pattern_recognition_score(accuracy);
        }
    }
    
    /// Update memory usage
    pub fn update_memory_usage(&self, bytes: i64) {
        self.memory_usage_bytes.set(bytes);
        
        if let Ok(mut history) = self.performance_history.try_lock() {
            history.add_memory_usage_sample(bytes as f64);
        }
    }
    
    /// Record service mesh events
    pub fn record_service_mesh_events(&self, 
        load_balance_decisions: u64,
        circuit_breaker_activations: u64,
        recommendations: u64,
        endpoint_accuracy: f64
    ) {
        self.load_balance_decisions.inc_by(load_balance_decisions);
        self.circuit_breaker_activations.inc_by(circuit_breaker_activations);
        self.service_recommendations_generated.inc_by(recommendations);
        self.endpoint_prediction_accuracy.observe(endpoint_accuracy);
    }
    
    /// Get comprehensive metrics summary
    pub async fn get_metrics_summary(&self) -> MetricsSummary {
        let history = self.performance_history.lock().await;
        
        // Neural network metrics
        let neural_network = NeuralNetworkMetrics {
            total_processing_operations: self.neural_processing_time.get_sample_count(),
            average_processing_time_ms: self.neural_processing_time.get_sample_sum() * 1000.0 / 
                (self.neural_processing_time.get_sample_count() as f64).max(1.0),
            average_similarity_score: self.neural_similarity_scores.get_sample_sum() / 
                (self.neural_similarity_scores.get_sample_count() as f64).max(1.0),
            average_confidence: self.neural_confidence_levels.get_sample_sum() / 
                (self.neural_confidence_levels.get_sample_count() as f64).max(1.0),
            total_spike_events: self.spike_events_total.get(),
            neuron_activation_rate: self.neuron_activation_count.get() as f64 / 
                (self.neural_processing_time.get_sample_count() as f64).max(1.0),
            synaptic_weight_changes: self.synaptic_weight_updates.get(),
        };
        
        // Adaptation metrics
        let adaptation = AdaptationMetrics {
            stdp_potentiation_events: self.stdp_potentiation_events.get(),
            stdp_depression_events: self.stdp_depression_events.get(),
            online_learning_updates: self.online_learning_updates.get(),
            average_convergence_time_ms: self.adaptation_convergence_time.get_sample_sum() * 1000.0 / 
                (self.adaptation_convergence_time.get_sample_count() as f64).max(1.0),
            learning_rate_adjustments: self.learning_rate_adjustments.get(),
            adaptation_success_rate: self.calculate_adaptation_success_rate(),
        };
        
        // Routing metrics
        let routing = RoutingMetrics {
            total_routing_decisions: self.routing_decisions_total.get(),
            average_decision_time_ms: self.routing_decision_time.get_sample_sum() * 1000.0 / 
                (self.routing_decision_time.get_sample_count() as f64).max(1.0),
            average_confidence_score: self.routing_confidence_score.get_sample_sum() / 
                (self.routing_confidence_score.get_sample_count() as f64).max(1.0),
            path_improvements_achieved: self.path_optimization_improvements.get_sample_sum() / 
                (self.path_optimization_improvements.get_sample_count() as f64).max(1.0),
            cache_hit_rate_percentage: self.cache_hit_rate.get() * 100.0,
            routing_accuracy: self.calculate_routing_accuracy(),
        };
        
        // Service mesh metrics
        let service_mesh = ServiceMeshMetrics {
            load_balance_decisions: self.load_balance_decisions.get(),
            circuit_breaker_activations: self.circuit_breaker_activations.get(),
            service_recommendations: self.service_recommendations_generated.get(),
            endpoint_prediction_accuracy: self.endpoint_prediction_accuracy.get_sample_sum() / 
                (self.endpoint_prediction_accuracy.get_sample_count() as f64).max(1.0),
            service_health_score: self.calculate_service_health_score(),
        };
        
        // Performance targets
        let performance_targets = self.calculate_performance_targets(&history);
        
        // System health
        let system_health = self.calculate_system_health(&history);
        
        MetricsSummary {
            neural_network,
            adaptation,
            routing,
            service_mesh,
            performance_targets,
            system_health,
        }
    }
    
    /// Get all metrics for external monitoring
    pub fn get_all_metrics(&self) -> HashMap<String, f64> {
        let mut metrics = HashMap::new();
        
        // Add key metrics
        metrics.insert("neural_processing_avg_ms".to_string(), 
            self.neural_processing_time.get_sample_sum() * 1000.0 / 
            (self.neural_processing_time.get_sample_count() as f64).max(1.0));
        
        metrics.insert("similarity_score_avg".to_string(),
            self.neural_similarity_scores.get_sample_sum() / 
            (self.neural_similarity_scores.get_sample_count() as f64).max(1.0));
        
        metrics.insert("confidence_avg".to_string(),
            self.neural_confidence_levels.get_sample_sum() / 
            (self.neural_confidence_levels.get_sample_count() as f64).max(1.0));
        
        metrics.insert("routing_decisions_total".to_string(),
            self.routing_decisions_total.get() as f64);
        
        metrics.insert("cache_hit_rate".to_string(),
            self.cache_hit_rate.get());
        
        metrics.insert("memory_usage_mb".to_string(),
            self.memory_usage_bytes.get() as f64 / (1024.0 * 1024.0));
        
        metrics.insert("spike_events_total".to_string(),
            self.spike_events_total.get() as f64);
        
        metrics
    }
    
    /// Get Prometheus registry for metrics export
    pub fn get_registry(&self) -> Arc<Registry> {
        self.registry.clone()
    }
    
    /// Calculate performance against targets
    fn calculate_performance_targets(&self, history: &PerformanceHistory) -> PerformanceTargets {
        let current_similarity_detection_ms = history.get_recent_avg_similarity_detection_time();
        let current_adaptation_rate_ms = history.get_recent_avg_adaptation_rate();
        let current_pattern_accuracy = history.get_recent_avg_pattern_recognition();
        let current_memory_mb = history.get_recent_avg_memory_usage() / (1024.0 * 1024.0);
        let current_convergence_minutes = current_adaptation_rate_ms / (1000.0 * 60.0);
        
        PerformanceTargets {
            // Similarity detection target: <1ms
            similarity_detection_target_ms: 1.0,
            similarity_detection_current_ms: current_similarity_detection_ms,
            similarity_detection_achievement_rate: if current_similarity_detection_ms <= 1.0 { 1.0 } 
                else { 1.0 / current_similarity_detection_ms },
            
            // Adaptation rate target: <100ms
            adaptation_rate_target_ms: 100.0,
            adaptation_rate_current_ms: current_adaptation_rate_ms,
            adaptation_rate_achievement_rate: if current_adaptation_rate_ms <= 100.0 { 1.0 } 
                else { 100.0 / current_adaptation_rate_ms },
            
            // Pattern recognition target: >95%
            pattern_recognition_target_accuracy: 0.95,
            pattern_recognition_current_accuracy: current_pattern_accuracy,
            pattern_recognition_achievement_rate: if current_pattern_accuracy >= 0.95 { 1.0 } 
                else { current_pattern_accuracy / 0.95 },
            
            // Memory usage target: <100MB
            memory_usage_target_mb: 100.0,
            memory_usage_current_mb: current_memory_mb,
            memory_usage_achievement_rate: if current_memory_mb <= 100.0 { 1.0 } 
                else { 100.0 / current_memory_mb },
            
            // Learning convergence target: <5 minutes
            learning_convergence_target_minutes: 5.0,
            learning_convergence_current_minutes: current_convergence_minutes,
            learning_convergence_achievement_rate: if current_convergence_minutes <= 5.0 { 1.0 } 
                else { 5.0 / current_convergence_minutes },
        }
    }
    
    /// Calculate overall system health
    fn calculate_system_health(&self, history: &PerformanceHistory) -> SystemHealthMetrics {
        let uptime = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap_or_default().as_secs_f64();
        
        let error_rate = 0.0; // Would be calculated from error counters
        let throughput = self.neural_processing_time.get_sample_count() as f64 / uptime.max(1.0);
        let resource_utilization = (self.memory_usage_bytes.get() as f64 / (100.0 * 1024.0 * 1024.0)).min(1.0);
        
        let performance_stability = history.calculate_performance_stability();
        
        // Overall health score (0-1)
        let health_components = vec![
            1.0 - error_rate,                    // Error rate (lower is better)
            (throughput / 1000.0).min(1.0),      // Throughput (higher is better)
            1.0 - resource_utilization,          // Resource utilization (lower is better)
            performance_stability,               // Stability (higher is better)
        ];
        
        let overall_health_score = health_components.iter().sum::<f64>() / health_components.len() as f64;
        
        SystemHealthMetrics {
            overall_health_score,
            uptime_seconds: uptime,
            error_rate,
            throughput_operations_per_second: throughput,
            resource_utilization,
            performance_stability,
        }
    }
    
    fn calculate_adaptation_success_rate(&self) -> f64 {
        let total_adaptations = self.stdp_potentiation_events.get() + 
                               self.stdp_depression_events.get() +
                               self.online_learning_updates.get();
        
        if total_adaptations == 0 {
            return 1.0;
        }
        
        // Simplified success rate calculation
        // In practice, this would track successful vs failed adaptations
        let successful_adaptations = total_adaptations; // Assume all succeed for now
        successful_adaptations as f64 / total_adaptations as f64
    }
    
    fn calculate_routing_accuracy(&self) -> f64 {
        // Simplified routing accuracy
        // Would be calculated from actual vs predicted performance
        let avg_confidence = self.routing_confidence_score.get_sample_sum() / 
            (self.routing_confidence_score.get_sample_count() as f64).max(1.0);
        
        avg_confidence // Use confidence as proxy for accuracy
    }
    
    fn calculate_service_health_score(&self) -> f64 {
        let total_decisions = self.load_balance_decisions.get();
        let failed_services = self.circuit_breaker_activations.get();
        
        if total_decisions == 0 {
            return 1.0;
        }
        
        let success_rate = 1.0 - (failed_services as f64 / total_decisions as f64);
        success_rate.max(0.0).min(1.0)
    }
}

impl PerformanceHistory {
    fn new() -> Self {
        Self {
            similarity_detection_times: VecDeque::new(),
            adaptation_rates: VecDeque::new(),
            pattern_recognition_scores: VecDeque::new(),
            memory_usage_samples: VecDeque::new(),
            routing_improvements: VecDeque::new(),
            cache_performance: VecDeque::new(),
            max_history_size: 1000,
        }
    }
    
    fn add_similarity_detection_time(&mut self, time_ms: f64) {
        self.similarity_detection_times.push_back((SystemTime::now(), time_ms));
        if self.similarity_detection_times.len() > self.max_history_size {
            self.similarity_detection_times.pop_front();
        }
    }
    
    fn add_adaptation_rate(&mut self, rate_ms: f64) {
        self.adaptation_rates.push_back((SystemTime::now(), rate_ms));
        if self.adaptation_rates.len() > self.max_history_size {
            self.adaptation_rates.pop_front();
        }
    }
    
    fn add_pattern_recognition_score(&mut self, score: f64) {
        self.pattern_recognition_scores.push_back((SystemTime::now(), score));
        if self.pattern_recognition_scores.len() > self.max_history_size {
            self.pattern_recognition_scores.pop_front();
        }
    }
    
    fn add_memory_usage_sample(&mut self, bytes: f64) {
        self.memory_usage_samples.push_back((SystemTime::now(), bytes));
        if self.memory_usage_samples.len() > self.max_history_size {
            self.memory_usage_samples.pop_front();
        }
    }
    
    fn add_routing_improvement(&mut self, improvement: f64) {
        self.routing_improvements.push_back((SystemTime::now(), improvement));
        if self.routing_improvements.len() > self.max_history_size {
            self.routing_improvements.pop_front();
        }
    }
    
    fn add_cache_performance(&mut self, hit_rate: f64) {
        self.cache_performance.push_back((SystemTime::now(), hit_rate));
        if self.cache_performance.len() > self.max_history_size {
            self.cache_performance.pop_front();
        }
    }
    
    fn get_recent_avg_similarity_detection_time(&self) -> f64 {
        if self.similarity_detection_times.is_empty() {
            return 0.5; // Default value
        }
        
        let recent_count = 100.min(self.similarity_detection_times.len());
        let recent: Vec<f64> = self.similarity_detection_times.iter()
            .rev()
            .take(recent_count)
            .map(|(_, value)| *value)
            .collect();
        
        recent.iter().sum::<f64>() / recent.len() as f64
    }
    
    fn get_recent_avg_adaptation_rate(&self) -> f64 {
        if self.adaptation_rates.is_empty() {
            return 50.0; // Default 50ms
        }
        
        let recent_count = 100.min(self.adaptation_rates.len());
        let recent: Vec<f64> = self.adaptation_rates.iter()
            .rev()
            .take(recent_count)
            .map(|(_, value)| *value)
            .collect();
        
        recent.iter().sum::<f64>() / recent.len() as f64
    }
    
    fn get_recent_avg_pattern_recognition(&self) -> f64 {
        if self.pattern_recognition_scores.is_empty() {
            return 0.95; // Default high accuracy
        }
        
        let recent_count = 100.min(self.pattern_recognition_scores.len());
        let recent: Vec<f64> = self.pattern_recognition_scores.iter()
            .rev()
            .take(recent_count)
            .map(|(_, value)| *value)
            .collect();
        
        recent.iter().sum::<f64>() / recent.len() as f64
    }
    
    fn get_recent_avg_memory_usage(&self) -> f64 {
        if self.memory_usage_samples.is_empty() {
            return 50.0 * 1024.0 * 1024.0; // Default 50MB
        }
        
        let recent_count = 100.min(self.memory_usage_samples.len());
        let recent: Vec<f64> = self.memory_usage_samples.iter()
            .rev()
            .take(recent_count)
            .map(|(_, value)| *value)
            .collect();
        
        recent.iter().sum::<f64>() / recent.len() as f64
    }
    
    fn calculate_performance_stability(&self) -> f64 {
        if self.similarity_detection_times.len() < 10 {
            return 0.8; // Default stability score
        }
        
        let recent_times: Vec<f64> = self.similarity_detection_times.iter()
            .rev()
            .take(100)
            .map(|(_, value)| *value)
            .collect();
        
        let mean = recent_times.iter().sum::<f64>() / recent_times.len() as f64;
        let variance = recent_times.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / recent_times.len() as f64;
        
        let coefficient_of_variation = if mean > 0.0 {
            variance.sqrt() / mean
        } else {
            0.0
        };
        
        // Stability = 1 - normalized coefficient of variation
        (1.0 - coefficient_of_variation.min(1.0)).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_metrics_creation() {
        let metrics = DsrMetrics::new();
        assert!(metrics.is_ok());
    }
    
    #[tokio::test]
    async fn test_metrics_recording() {
        let metrics = DsrMetrics::new().unwrap();
        
        // Record some metrics
        metrics.record_processing_time(Duration::from_millis(5));
        metrics.record_similarity_score(0.85);
        metrics.record_confidence(0.9);
        metrics.increment_spike_events(100);
        
        let summary = metrics.get_metrics_summary().await;
        
        assert!(summary.neural_network.total_processing_operations > 0);
        assert!(summary.neural_network.average_processing_time_ms > 0.0);
        assert!(summary.neural_network.total_spike_events > 0);
    }
    
    #[tokio::test]
    async fn test_performance_targets() {
        let metrics = DsrMetrics::new().unwrap();
        
        // Record performance within targets
        metrics.record_processing_time(Duration::from_micros(500)); // 0.5ms - within 1ms target
        metrics.record_adaptation_convergence(Duration::from_millis(50)); // 50ms - within 100ms target
        metrics.record_pattern_recognition_accuracy(0.97); // 97% - above 95% target
        metrics.update_memory_usage(50 * 1024 * 1024); // 50MB - within 100MB target
        
        let summary = metrics.get_metrics_summary().await;
        
        // Check that we're meeting performance targets
        assert!(summary.performance_targets.similarity_detection_achievement_rate >= 0.5);
        assert!(summary.performance_targets.adaptation_rate_achievement_rate >= 0.5);
        assert!(summary.performance_targets.pattern_recognition_achievement_rate >= 0.95);
        assert!(summary.performance_targets.memory_usage_achievement_rate >= 0.5);
    }
    
    #[test]
    fn test_performance_history() {
        let mut history = PerformanceHistory::new();
        
        // Add some sample data
        for i in 0..50 {
            history.add_similarity_detection_time(0.5 + (i as f64 * 0.01));
            history.add_adaptation_rate(40.0 + (i as f64));
            history.add_pattern_recognition_score(0.95 + (i as f64 * 0.001));
        }
        
        let avg_detection_time = history.get_recent_avg_similarity_detection_time();
        let avg_adaptation_rate = history.get_recent_avg_adaptation_rate();
        let avg_pattern_score = history.get_recent_avg_pattern_recognition();
        
        assert!(avg_detection_time > 0.0);
        assert!(avg_adaptation_rate > 0.0);
        assert!(avg_pattern_score > 0.9);
    }
    
    #[test]
    fn test_metrics_export() {
        let metrics = DsrMetrics::new().unwrap();
        
        // Record some data
        metrics.record_processing_time(Duration::from_millis(2));
        metrics.increment_spike_events(50);
        metrics.update_cache_hit_rate(0.75);
        
        let all_metrics = metrics.get_all_metrics();
        
        assert!(all_metrics.contains_key("neural_processing_avg_ms"));
        assert!(all_metrics.contains_key("cache_hit_rate"));
        assert!(all_metrics.contains_key("spike_events_total"));
        
        assert_eq!(all_metrics["cache_hit_rate"], 0.75);
        assert_eq!(all_metrics["spike_events_total"], 50.0);
    }
}