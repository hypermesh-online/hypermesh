//! CPE Metrics and Monitoring
//!
//! This module implements comprehensive metrics collection and monitoring
//! for the Context Prediction Engine performance and behavior analysis.

use anyhow::Result;
use prometheus::{
    Counter, Gauge, Histogram, HistogramOpts, Opts, Registry, 
    IntCounter, IntGauge, IntCounterVec, GaugeVec,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, info};

/// CPE metrics collector and exporter
pub struct CpeMetrics {
    /// Prometheus registry for metrics
    registry: Registry,
    
    // Core prediction metrics
    predictions_total: IntCounter,
    predictions_success: IntCounter,
    predictions_failed: IntCounter,
    prediction_latency: Histogram,
    prediction_confidence: Histogram,
    
    // Cache metrics
    cache_hits_total: IntCounter,
    cache_misses_total: IntCounter,
    cache_size_current: IntGauge,
    cache_evictions_total: IntCounter,
    
    // Learning metrics
    learning_adaptations_total: IntCounter,
    learning_rate_current: Gauge,
    model_loss_current: Gauge,
    training_examples_total: IntCounter,
    
    // Integration metrics
    layer2_messages_received: IntCounter,
    layer3_messages_sent: IntCounter,
    hypermesh_events_processed: IntCounter,
    integration_errors_total: IntCounter,
    
    // Model performance metrics
    model_accuracy: Gauge,
    sequence_length_avg: Gauge,
    context_dimension_used: IntGauge,
    
    // System resource metrics
    memory_usage_mb: Gauge,
    cpu_usage_percent: Gauge,
    gpu_usage_percent: Gauge,
    disk_io_bytes: IntCounterVec,
    
    // Custom business metrics
    flow_prediction_accuracy: GaugeVec,
    pattern_recognition_rate: Gauge,
    adaptive_threshold_current: Gauge,
    
    // Performance tracking
    start_time: Instant,
    last_update: Arc<RwLock<Instant>>,
    metric_update_count: Arc<std::sync::atomic::AtomicU64>,
}

/// Detailed metrics snapshot for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: u64,
    pub uptime_seconds: f64,
    
    // Core metrics
    pub predictions_total: u64,
    pub predictions_per_second: f64,
    pub success_rate: f64,
    pub average_latency_ms: f64,
    pub average_confidence: f64,
    
    // Cache performance
    pub cache_hit_rate: f64,
    pub cache_efficiency: f64,
    pub current_cache_size: u64,
    
    // Learning metrics
    pub total_adaptations: u64,
    pub current_learning_rate: f64,
    pub model_loss: f64,
    pub learning_effectiveness: f64,
    
    // Integration health
    pub layer_integration_health: f64,
    pub message_processing_rate: f64,
    pub error_rate: f64,
    
    // Resource utilization
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub gpu_available: bool,
    
    // Performance indicators
    pub p50_latency_ms: f64,
    pub p90_latency_ms: f64,
    pub p99_latency_ms: f64,
}

/// Performance alert levels
#[derive(Debug, Clone, PartialEq)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
    Emergency,
}

/// Performance alert
#[derive(Debug, Clone)]
pub struct PerformanceAlert {
    pub level: AlertLevel,
    pub metric_name: String,
    pub current_value: f64,
    pub threshold: f64,
    pub message: String,
    pub timestamp: Instant,
    pub suggested_action: Option<String>,
}

impl CpeMetrics {
    /// Create a new metrics collector
    pub fn new() -> Result<Self> {
        let registry = Registry::new();
        
        // Create core prediction metrics
        let predictions_total = IntCounter::new("cpe_predictions_total", "Total number of predictions made")?;
        registry.register(Box::new(predictions_total.clone()))?;
        
        let predictions_success = IntCounter::new("cpe_predictions_success_total", "Number of successful predictions")?;
        registry.register(Box::new(predictions_success.clone()))?;
        
        let predictions_failed = IntCounter::new("cpe_predictions_failed_total", "Number of failed predictions")?;
        registry.register(Box::new(predictions_failed.clone()))?;
        
        let prediction_latency = Histogram::with_opts(
            HistogramOpts::new("cpe_prediction_latency_seconds", "Prediction processing latency")
                .buckets(vec![0.001, 0.002, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0])
        )?;
        registry.register(Box::new(prediction_latency.clone()))?;
        
        let prediction_confidence = Histogram::with_opts(
            HistogramOpts::new("cpe_prediction_confidence", "Prediction confidence scores")
                .buckets(vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0])
        )?;
        registry.register(Box::new(prediction_confidence.clone()))?;
        
        // Create cache metrics
        let cache_hits_total = IntCounter::new("cpe_cache_hits_total", "Total cache hits")?;
        registry.register(Box::new(cache_hits_total.clone()))?;
        
        let cache_misses_total = IntCounter::new("cpe_cache_misses_total", "Total cache misses")?;
        registry.register(Box::new(cache_misses_total.clone()))?;
        
        let cache_size_current = IntGauge::new("cpe_cache_size_current", "Current cache size")?;
        registry.register(Box::new(cache_size_current.clone()))?;
        
        let cache_evictions_total = IntCounter::new("cpe_cache_evictions_total", "Total cache evictions")?;
        registry.register(Box::new(cache_evictions_total.clone()))?;
        
        // Create learning metrics
        let learning_adaptations_total = IntCounter::new("cpe_learning_adaptations_total", "Total learning adaptations")?;
        registry.register(Box::new(learning_adaptations_total.clone()))?;
        
        let learning_rate_current = Gauge::new("cpe_learning_rate_current", "Current learning rate")?;
        registry.register(Box::new(learning_rate_current.clone()))?;
        
        let model_loss_current = Gauge::new("cpe_model_loss_current", "Current model loss")?;
        registry.register(Box::new(model_loss_current.clone()))?;
        
        let training_examples_total = IntCounter::new("cpe_training_examples_total", "Total training examples processed")?;
        registry.register(Box::new(training_examples_total.clone()))?;
        
        // Create integration metrics
        let layer2_messages_received = IntCounter::new("cpe_layer2_messages_received_total", "Messages received from Layer 2")?;
        registry.register(Box::new(layer2_messages_received.clone()))?;
        
        let layer3_messages_sent = IntCounter::new("cpe_layer3_messages_sent_total", "Messages sent to Layer 3")?;
        registry.register(Box::new(layer3_messages_sent.clone()))?;
        
        let hypermesh_events_processed = IntCounter::new("cpe_hypermesh_events_processed_total", "HyperMesh events processed")?;
        registry.register(Box::new(hypermesh_events_processed.clone()))?;
        
        let integration_errors_total = IntCounter::new("cpe_integration_errors_total", "Integration errors")?;
        registry.register(Box::new(integration_errors_total.clone()))?;
        
        // Create model performance metrics
        let model_accuracy = Gauge::new("cpe_model_accuracy", "Current model accuracy")?;
        registry.register(Box::new(model_accuracy.clone()))?;
        
        let sequence_length_avg = Gauge::new("cpe_sequence_length_avg", "Average sequence length")?;
        registry.register(Box::new(sequence_length_avg.clone()))?;
        
        let context_dimension_used = IntGauge::new("cpe_context_dimension", "Context dimension in use")?;
        registry.register(Box::new(context_dimension_used.clone()))?;
        
        // Create resource metrics
        let memory_usage_mb = Gauge::new("cpe_memory_usage_mb", "Memory usage in MB")?;
        registry.register(Box::new(memory_usage_mb.clone()))?;
        
        let cpu_usage_percent = Gauge::new("cpe_cpu_usage_percent", "CPU usage percentage")?;
        registry.register(Box::new(cpu_usage_percent.clone()))?;
        
        let gpu_usage_percent = Gauge::new("cpe_gpu_usage_percent", "GPU usage percentage")?;
        registry.register(Box::new(gpu_usage_percent.clone()))?;
        
        let disk_io_bytes = IntCounterVec::new(
            Opts::new("cpe_disk_io_bytes_total", "Disk I/O bytes"),
            &["direction"] // "read" or "write"
        )?;
        registry.register(Box::new(disk_io_bytes.clone()))?;
        
        // Create custom business metrics
        let flow_prediction_accuracy = GaugeVec::new(
            Opts::new("cpe_flow_prediction_accuracy", "Per-flow prediction accuracy"),
            &["flow_type", "pattern_type"]
        )?;
        registry.register(Box::new(flow_prediction_accuracy.clone()))?;
        
        let pattern_recognition_rate = Gauge::new("cpe_pattern_recognition_rate", "Pattern recognition success rate")?;
        registry.register(Box::new(pattern_recognition_rate.clone()))?;
        
        let adaptive_threshold_current = Gauge::new("cpe_adaptive_threshold", "Current adaptive threshold")?;
        registry.register(Box::new(adaptive_threshold_current.clone()))?;
        
        info!("CPE metrics system initialized with {} metrics", registry.gather().len());
        
        Ok(Self {
            registry,
            predictions_total,
            predictions_success,
            predictions_failed,
            prediction_latency,
            prediction_confidence,
            cache_hits_total,
            cache_misses_total,
            cache_size_current,
            cache_evictions_total,
            learning_adaptations_total,
            learning_rate_current,
            model_loss_current,
            training_examples_total,
            layer2_messages_received,
            layer3_messages_sent,
            hypermesh_events_processed,
            integration_errors_total,
            model_accuracy,
            sequence_length_avg,
            context_dimension_used,
            memory_usage_mb,
            cpu_usage_percent,
            gpu_usage_percent,
            disk_io_bytes,
            flow_prediction_accuracy,
            pattern_recognition_rate,
            adaptive_threshold_current,
            start_time: Instant::now(),
            last_update: Arc::new(RwLock::new(Instant::now())),
            metric_update_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        })
    }
    
    /// Record a prediction event
    pub fn record_prediction(&self, latency: Duration, confidence: f32, success: bool) {
        self.predictions_total.inc();
        
        if success {
            self.predictions_success.inc();
        } else {
            self.predictions_failed.inc();
        }
        
        self.prediction_latency.observe(latency.as_secs_f64());
        self.prediction_confidence.observe(confidence as f64);
        
        debug!("Recorded prediction: latency={:?}, confidence={:.3}, success={}", 
               latency, confidence, success);
    }
    
    /// Record cache hit
    pub fn record_cache_hit(&self) {
        self.cache_hits_total.inc();
    }
    
    /// Record cache miss
    pub fn record_cache_miss(&self) {
        self.cache_misses_total.inc();
    }
    
    /// Update cache size
    pub fn update_cache_size(&self, size: usize) {
        self.cache_size_current.set(size as i64);
    }
    
    /// Record cache eviction
    pub fn record_cache_eviction(&self) {
        self.cache_evictions_total.inc();
    }
    
    /// Record learning adaptation
    pub fn record_learning_adaptation(&self, learning_rate: f64, loss: f64) {
        self.learning_adaptations_total.inc();
        self.learning_rate_current.set(learning_rate);
        self.model_loss_current.set(loss);
    }
    
    /// Record training example processed
    pub fn record_training_example(&self) {
        self.training_examples_total.inc();
    }
    
    /// Record Layer 2 message received
    pub fn record_layer2_message(&self) {
        self.layer2_messages_received.inc();
    }
    
    /// Record Layer 3 message sent
    pub fn record_layer3_message(&self) {
        self.layer3_messages_sent.inc();
    }
    
    /// Record HyperMesh event processed
    pub fn record_hypermesh_event(&self) {
        self.hypermesh_events_processed.inc();
    }
    
    /// Record integration error
    pub fn record_integration_error(&self) {
        self.integration_errors_total.inc();
    }
    
    /// Update model accuracy
    pub fn update_model_accuracy(&self, accuracy: f32) {
        self.model_accuracy.set(accuracy as f64);
    }
    
    /// Update sequence length average
    pub fn update_sequence_length_avg(&self, length: f32) {
        self.sequence_length_avg.set(length as f64);
    }
    
    /// Update context dimension
    pub fn update_context_dimension(&self, dimension: usize) {
        self.context_dimension_used.set(dimension as i64);
    }
    
    /// Update resource usage metrics
    pub fn update_resource_usage(&self, memory_mb: f64, cpu_percent: f64, gpu_percent: Option<f64>) {
        self.memory_usage_mb.set(memory_mb);
        self.cpu_usage_percent.set(cpu_percent);
        
        if let Some(gpu) = gpu_percent {
            self.gpu_usage_percent.set(gpu);
        }
    }
    
    /// Record disk I/O
    pub fn record_disk_io(&self, direction: &str, bytes: u64) {
        self.disk_io_bytes.with_label_values(&[direction]).inc_by(bytes);
    }
    
    /// Update flow prediction accuracy
    pub fn update_flow_prediction_accuracy(&self, flow_type: &str, pattern_type: &str, accuracy: f32) {
        self.flow_prediction_accuracy
            .with_label_values(&[flow_type, pattern_type])
            .set(accuracy as f64);
    }
    
    /// Update pattern recognition rate
    pub fn update_pattern_recognition_rate(&self, rate: f32) {
        self.pattern_recognition_rate.set(rate as f64);
    }
    
    /// Update adaptive threshold
    pub fn update_adaptive_threshold(&self, threshold: f32) {
        self.adaptive_threshold_current.set(threshold as f64);
    }
    
    /// Get current metrics snapshot
    pub async fn get_metrics_snapshot(&self) -> MetricsSnapshot {
        let uptime = self.start_time.elapsed();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Calculate derived metrics
        let predictions_total = self.predictions_total.get();
        let predictions_success = self.predictions_success.get();
        let predictions_failed = self.predictions_failed.get();
        
        let success_rate = if predictions_total > 0 {
            predictions_success as f64 / predictions_total as f64
        } else {
            0.0
        };
        
        let predictions_per_second = predictions_total as f64 / uptime.as_secs_f64();
        
        let cache_hits = self.cache_hits_total.get();
        let cache_misses = self.cache_misses_total.get();
        let total_cache_requests = cache_hits + cache_misses;
        
        let cache_hit_rate = if total_cache_requests > 0 {
            cache_hits as f64 / total_cache_requests as f64
        } else {
            0.0
        };
        
        // Calculate latency percentiles (simplified - would need histogram analysis in real implementation)
        let average_latency_ms = self.prediction_latency.get_sample_sum() * 1000.0 / self.prediction_latency.get_sample_count().max(1.0);
        
        let integration_messages = self.layer2_messages_received.get() + 
                                  self.layer3_messages_sent.get() + 
                                  self.hypermesh_events_processed.get();
        let message_processing_rate = integration_messages as f64 / uptime.as_secs_f64();
        
        let total_errors = self.predictions_failed.get() + self.integration_errors_total.get();
        let total_operations = predictions_total + integration_messages;
        let error_rate = if total_operations > 0 {
            total_errors as f64 / total_operations as f64
        } else {
            0.0
        };
        
        *self.last_update.write().await = Instant::now();
        self.metric_update_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        MetricsSnapshot {
            timestamp,
            uptime_seconds: uptime.as_secs_f64(),
            predictions_total,
            predictions_per_second,
            success_rate,
            average_latency_ms,
            average_confidence: self.prediction_confidence.get_sample_sum() / self.prediction_confidence.get_sample_count().max(1.0),
            cache_hit_rate,
            cache_efficiency: cache_hit_rate * success_rate, // Combined efficiency metric
            current_cache_size: self.cache_size_current.get() as u64,
            total_adaptations: self.learning_adaptations_total.get(),
            current_learning_rate: self.learning_rate_current.get(),
            model_loss: self.model_loss_current.get(),
            learning_effectiveness: if self.learning_adaptations_total.get() > 0 { success_rate } else { 0.0 },
            layer_integration_health: 1.0 - error_rate, // Inverted error rate as health indicator
            message_processing_rate,
            error_rate,
            memory_usage_mb: self.memory_usage_mb.get(),
            cpu_usage_percent: self.cpu_usage_percent.get(),
            gpu_available: self.gpu_usage_percent.get() >= 0.0,
            p50_latency_ms: average_latency_ms, // Simplified - would need proper percentile calculation
            p90_latency_ms: average_latency_ms * 1.5,
            p99_latency_ms: average_latency_ms * 2.0,
        }
    }
    
    /// Check for performance alerts
    pub async fn check_performance_alerts(&self) -> Vec<PerformanceAlert> {
        let mut alerts = Vec::new();
        let snapshot = self.get_metrics_snapshot().await;
        
        // Check prediction latency
        if snapshot.average_latency_ms > 2.0 {
            alerts.push(PerformanceAlert {
                level: if snapshot.average_latency_ms > 5.0 { AlertLevel::Critical } else { AlertLevel::Warning },
                metric_name: "prediction_latency".to_string(),
                current_value: snapshot.average_latency_ms,
                threshold: 2.0,
                message: "Prediction latency exceeds target".to_string(),
                timestamp: Instant::now(),
                suggested_action: Some("Consider reducing batch size or optimizing model".to_string()),
            });
        }
        
        // Check cache hit rate
        if snapshot.cache_hit_rate < 0.8 {
            alerts.push(PerformanceAlert {
                level: if snapshot.cache_hit_rate < 0.5 { AlertLevel::Warning } else { AlertLevel::Info },
                metric_name: "cache_hit_rate".to_string(),
                current_value: snapshot.cache_hit_rate,
                threshold: 0.8,
                message: "Cache hit rate below optimal".to_string(),
                timestamp: Instant::now(),
                suggested_action: Some("Increase cache size or optimize cache strategy".to_string()),
            });
        }
        
        // Check success rate
        if snapshot.success_rate < 0.95 {
            alerts.push(PerformanceAlert {
                level: if snapshot.success_rate < 0.8 { AlertLevel::Critical } else { AlertLevel::Warning },
                metric_name: "success_rate".to_string(),
                current_value: snapshot.success_rate,
                threshold: 0.95,
                message: "Prediction success rate below target".to_string(),
                timestamp: Instant::now(),
                suggested_action: Some("Review model training or input data quality".to_string()),
            });
        }
        
        // Check error rate
        if snapshot.error_rate > 0.05 {
            alerts.push(PerformanceAlert {
                level: if snapshot.error_rate > 0.1 { AlertLevel::Critical } else { AlertLevel::Warning },
                metric_name: "error_rate".to_string(),
                current_value: snapshot.error_rate,
                threshold: 0.05,
                message: "Error rate exceeds acceptable threshold".to_string(),
                timestamp: Instant::now(),
                suggested_action: Some("Investigate error sources and improve error handling".to_string()),
            });
        }
        
        // Check memory usage
        if snapshot.memory_usage_mb > 500.0 {
            alerts.push(PerformanceAlert {
                level: if snapshot.memory_usage_mb > 1000.0 { AlertLevel::Critical } else { AlertLevel::Warning },
                metric_name: "memory_usage".to_string(),
                current_value: snapshot.memory_usage_mb,
                threshold: 500.0,
                message: "Memory usage exceeds recommended limit".to_string(),
                timestamp: Instant::now(),
                suggested_action: Some("Optimize memory usage or increase available memory".to_string()),
            });
        }
        
        if !alerts.is_empty() {
            debug!("Generated {} performance alerts", alerts.len());
        }
        
        alerts
    }
    
    /// Export metrics in Prometheus format
    pub fn export_prometheus_metrics(&self) -> String {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder.encode_to_string(&metric_families).unwrap_or_default()
    }
    
    /// Get all metrics as key-value pairs
    pub fn get_all_metrics(&self) -> HashMap<String, f64> {
        let mut metrics = HashMap::new();
        
        metrics.insert("predictions_total".to_string(), self.predictions_total.get() as f64);
        metrics.insert("predictions_success".to_string(), self.predictions_success.get() as f64);
        metrics.insert("predictions_failed".to_string(), self.predictions_failed.get() as f64);
        metrics.insert("cache_hits_total".to_string(), self.cache_hits_total.get() as f64);
        metrics.insert("cache_misses_total".to_string(), self.cache_misses_total.get() as f64);
        metrics.insert("cache_size_current".to_string(), self.cache_size_current.get() as f64);
        metrics.insert("learning_adaptations_total".to_string(), self.learning_adaptations_total.get() as f64);
        metrics.insert("learning_rate_current".to_string(), self.learning_rate_current.get());
        metrics.insert("model_loss_current".to_string(), self.model_loss_current.get());
        metrics.insert("model_accuracy".to_string(), self.model_accuracy.get());
        metrics.insert("memory_usage_mb".to_string(), self.memory_usage_mb.get());
        metrics.insert("cpu_usage_percent".to_string(), self.cpu_usage_percent.get());
        metrics.insert("uptime_seconds".to_string(), self.start_time.elapsed().as_secs_f64());
        
        metrics
    }
    
    /// Reset all counters (for testing purposes)
    pub fn reset_counters(&self) {
        // Note: Prometheus counters cannot be reset in production
        // This would typically be implemented differently
        info!("Metrics reset requested (implementation depends on metric backend)");
    }
}

/// Metrics reporter for periodic metric updates
pub struct MetricsReporter {
    metrics: Arc<CpeMetrics>,
    report_interval: Duration,
    system_monitor: SystemMonitor,
}

/// System resource monitor
struct SystemMonitor;

impl SystemMonitor {
    fn get_memory_usage_mb(&self) -> f64 {
        // Placeholder - would use actual system monitoring
        250.0
    }
    
    fn get_cpu_usage_percent(&self) -> f64 {
        // Placeholder - would use actual system monitoring
        15.0
    }
    
    fn get_gpu_usage_percent(&self) -> Option<f64> {
        // Placeholder - would use actual GPU monitoring
        Some(5.0)
    }
}

impl MetricsReporter {
    pub fn new(metrics: Arc<CpeMetrics>, report_interval: Duration) -> Self {
        Self {
            metrics,
            report_interval,
            system_monitor: SystemMonitor,
        }
    }
    
    /// Start periodic metrics reporting
    pub async fn start_reporting(&self) {
        let metrics = self.metrics.clone();
        let interval = self.report_interval;
        let monitor = SystemMonitor;
        
        tokio::spawn(async move {
            let mut timer = tokio::time::interval(interval);
            
            loop {
                timer.tick().await;
                
                // Update system metrics
                let memory_usage = monitor.get_memory_usage_mb();
                let cpu_usage = monitor.get_cpu_usage_percent();
                let gpu_usage = monitor.get_gpu_usage_percent();
                
                metrics.update_resource_usage(memory_usage, cpu_usage, gpu_usage);
                
                // Check for alerts
                let alerts = metrics.check_performance_alerts().await;
                
                for alert in alerts {
                    match alert.level {
                        AlertLevel::Critical | AlertLevel::Emergency => {
                            tracing::error!("CPE Alert [{}]: {} = {:.3} (threshold: {:.3}) - {}", 
                                          format!("{:?}", alert.level).to_uppercase(),
                                          alert.metric_name, 
                                          alert.current_value, 
                                          alert.threshold, 
                                          alert.message);
                        }
                        AlertLevel::Warning => {
                            tracing::warn!("CPE Alert [WARNING]: {} = {:.3} (threshold: {:.3}) - {}", 
                                         alert.metric_name, 
                                         alert.current_value, 
                                         alert.threshold, 
                                         alert.message);
                        }
                        AlertLevel::Info => {
                            tracing::info!("CPE Alert [INFO]: {} = {:.3} (threshold: {:.3}) - {}", 
                                         alert.metric_name, 
                                         alert.current_value, 
                                         alert.threshold, 
                                         alert.message);
                        }
                    }
                }
                
                debug!("Metrics reporting cycle completed");
            }
        });
        
        info!("Metrics reporter started with interval: {:?}", self.report_interval);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_metrics_creation() {
        let metrics = CpeMetrics::new();
        assert!(metrics.is_ok());
    }
    
    #[tokio::test]
    async fn test_prediction_recording() {
        let metrics = CpeMetrics::new().unwrap();
        
        metrics.record_prediction(Duration::from_millis(1), 0.8, true);
        metrics.record_prediction(Duration::from_millis(2), 0.6, false);
        
        assert_eq!(metrics.predictions_total.get(), 2);
        assert_eq!(metrics.predictions_success.get(), 1);
        assert_eq!(metrics.predictions_failed.get(), 1);
    }
    
    #[tokio::test]
    async fn test_cache_metrics() {
        let metrics = CpeMetrics::new().unwrap();
        
        metrics.record_cache_hit();
        metrics.record_cache_hit();
        metrics.record_cache_miss();
        metrics.update_cache_size(1000);
        
        assert_eq!(metrics.cache_hits_total.get(), 2);
        assert_eq!(metrics.cache_misses_total.get(), 1);
        assert_eq!(metrics.cache_size_current.get(), 1000);
    }
    
    #[tokio::test]
    async fn test_metrics_snapshot() {
        let metrics = CpeMetrics::new().unwrap();
        
        // Record some metrics
        metrics.record_prediction(Duration::from_millis(1), 0.9, true);
        metrics.record_cache_hit();
        metrics.update_model_accuracy(0.95);
        
        let snapshot = metrics.get_metrics_snapshot().await;
        
        assert_eq!(snapshot.predictions_total, 1);
        assert_eq!(snapshot.success_rate, 1.0);
        assert!(snapshot.uptime_seconds > 0.0);
    }
    
    #[tokio::test]
    async fn test_performance_alerts() {
        let metrics = CpeMetrics::new().unwrap();
        
        // Trigger some alert conditions
        metrics.record_prediction(Duration::from_millis(10), 0.5, false); // High latency, low confidence, failure
        
        let alerts = metrics.check_performance_alerts().await;
        
        // Should have alerts for latency and success rate
        assert!(!alerts.is_empty());
        assert!(alerts.iter().any(|a| a.metric_name == "prediction_latency"));
        assert!(alerts.iter().any(|a| a.metric_name == "success_rate"));
    }
    
    #[test]
    fn test_prometheus_export() {
        let metrics = CpeMetrics::new().unwrap();
        
        metrics.record_prediction(Duration::from_millis(1), 0.8, true);
        
        let prometheus_output = metrics.export_prometheus_metrics();
        assert!(!prometheus_output.is_empty());
        assert!(prometheus_output.contains("cpe_predictions_total"));
    }
    
    #[test]
    fn test_all_metrics_export() {
        let metrics = CpeMetrics::new().unwrap();
        
        metrics.record_prediction(Duration::from_millis(1), 0.8, true);
        metrics.update_model_accuracy(0.92);
        
        let all_metrics = metrics.get_all_metrics();
        
        assert!(all_metrics.contains_key("predictions_total"));
        assert!(all_metrics.contains_key("model_accuracy"));
        assert_eq!(all_metrics.get("predictions_total"), Some(&1.0));
        assert_eq!(all_metrics.get("model_accuracy"), Some(&0.92));
    }
}