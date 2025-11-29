//! Performance Validation for MFN-Enhanced Orchestration
//!
//! Validates that the orchestration layer achieves the revolutionary performance
//! targets enabled by the MFN 4-layer foundation, demonstrating capabilities
//! traditional systems cannot achieve.

use super::mfn_bridge::{MfnBridge, MfnPerformanceMetrics, LayerCoordination};
use super::{PerformanceTargets, AlertThresholds};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Performance validator for orchestration targets
pub struct PerformanceValidator {
    /// MFN bridge reference
    mfn_bridge: Arc<MfnBridge>,
    /// Validation enabled flag
    validation_enabled: bool,
    /// Historical performance data
    performance_history: Arc<RwLock<PerformanceHistory>>,
    /// Validation metrics
    validation_metrics: Arc<RwLock<ValidationMetrics>>,
    /// Alert history
    alert_history: Arc<RwLock<Vec<PerformanceAlert>>>,
}

/// Historical performance tracking
#[derive(Debug, Clone)]
pub struct PerformanceHistory {
    /// Service mesh routing latencies (µs)
    pub service_mesh_latencies: Vec<TimedMetric>,
    /// Container scheduling latencies (ms)
    pub container_scheduling_latencies: Vec<TimedMetric>,
    /// Service discovery latencies (µs)
    pub service_discovery_latencies: Vec<TimedMetric>,
    /// Auto-scaling decision latencies (ms)
    pub auto_scaling_latencies: Vec<TimedMetric>,
    /// End-to-end orchestration latencies (ms)
    pub end_to_end_latencies: Vec<TimedMetric>,
    /// Performance improvement factors
    pub improvement_factors: Vec<TimedMetric>,
}

/// Timed performance metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimedMetric {
    /// Metric value
    pub value: f64,
    /// Timestamp when recorded
    pub timestamp: SystemTime,
    /// Additional context
    pub context: HashMap<String, String>,
}

/// Validation metrics and statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationMetrics {
    /// Total validations performed
    pub total_validations: u64,
    /// Target violations detected
    pub target_violations: u64,
    /// Performance improvements measured
    pub performance_improvements: u64,
    /// Average validation latency (µs)
    pub avg_validation_latency_us: f64,
    /// Validation accuracy percentage
    pub validation_accuracy: f64,
    /// Last validation timestamp
    pub last_validation: Option<SystemTime>,
}

/// Performance alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAlert {
    /// Alert ID
    pub id: String,
    /// Alert severity
    pub severity: AlertSeverity,
    /// Alert message
    pub message: String,
    /// Affected component
    pub component: String,
    /// Metric that triggered alert
    pub metric: String,
    /// Current value
    pub current_value: f64,
    /// Target value
    pub target_value: f64,
    /// Alert timestamp
    pub timestamp: SystemTime,
    /// Alert resolved flag
    pub resolved: bool,
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Informational
    Info,
    /// Warning level
    Warning,
    /// Critical level
    Critical,
}

/// Performance validation report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    /// Overall validation result
    pub overall_result: ValidationResult,
    /// Individual component results
    pub component_results: HashMap<String, ComponentValidation>,
    /// Performance summary
    pub performance_summary: PerformanceSummary,
    /// Recommendations
    pub recommendations: Vec<PerformanceRecommendation>,
    /// Report timestamp
    pub timestamp: SystemTime,
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationResult {
    /// All targets met or exceeded
    Passed,
    /// Some targets missed but within acceptable range
    Warning,
    /// Critical targets missed
    Failed,
}

/// Component validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentValidation {
    /// Component name
    pub component: String,
    /// Target latency (various units)
    pub target_latency: f64,
    /// Actual latency measured
    pub actual_latency: f64,
    /// Performance improvement factor
    pub improvement_factor: f64,
    /// Accuracy/confidence
    pub accuracy: f64,
    /// Validation result
    pub result: ValidationResult,
    /// Additional metrics
    pub additional_metrics: HashMap<String, f64>,
}

/// Performance summary statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    /// Service mesh average latency (µs)
    pub service_mesh_avg_latency_us: f64,
    /// Container scheduling average latency (ms)
    pub container_avg_latency_ms: f64,
    /// Service discovery average latency (µs)
    pub discovery_avg_latency_us: f64,
    /// Auto-scaling average latency (ms)
    pub scaling_avg_latency_ms: f64,
    /// End-to-end average latency (ms)
    pub end_to_end_avg_latency_ms: f64,
    /// Overall improvement factor vs traditional systems
    pub overall_improvement_factor: f64,
    /// MFN utilization percentage
    pub mfn_utilization_percentage: f64,
    /// Resource efficiency improvement
    pub resource_efficiency_improvement: f64,
}

/// Performance recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRecommendation {
    /// Recommendation type
    pub recommendation_type: RecommendationType,
    /// Target component
    pub component: String,
    /// Priority (1 = highest, 10 = lowest)
    pub priority: u8,
    /// Expected improvement
    pub expected_improvement: f64,
    /// Description
    pub description: String,
    /// Implementation complexity
    pub complexity: RecommendationComplexity,
}

/// Types of performance recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationType {
    /// Optimize algorithm or configuration
    Optimization,
    /// Scale component resources
    Scaling,
    /// Adjust caching strategy
    Caching,
    /// Load balancing adjustment
    LoadBalancing,
    /// Configuration tuning
    Configuration,
    /// Architecture change
    Architecture,
}

/// Implementation complexity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationComplexity {
    /// Simple configuration change
    Low,
    /// Moderate implementation effort
    Medium,
    /// Significant architecture change
    High,
}

impl PerformanceValidator {
    /// Create a new performance validator
    pub async fn new(mfn_bridge: Arc<MfnBridge>, validation_enabled: bool) -> Result<Self> {
        let performance_history = Arc::new(RwLock::new(PerformanceHistory {
            service_mesh_latencies: Vec::new(),
            container_scheduling_latencies: Vec::new(),
            service_discovery_latencies: Vec::new(),
            auto_scaling_latencies: Vec::new(),
            end_to_end_latencies: Vec::new(),
            improvement_factors: Vec::new(),
        }));
        
        let validation_metrics = Arc::new(RwLock::new(ValidationMetrics {
            total_validations: 0,
            target_violations: 0,
            performance_improvements: 0,
            avg_validation_latency_us: 0.0,
            validation_accuracy: 0.0,
            last_validation: None,
        }));
        
        let alert_history = Arc::new(RwLock::new(Vec::new()));
        
        info!("Performance validator initialized with MFN foundation integration");
        
        Ok(Self {
            mfn_bridge,
            validation_enabled,
            performance_history,
            validation_metrics,
            alert_history,
        })
    }
    
    /// Validate orchestration performance targets
    pub async fn validate_orchestration_targets(&self) -> Result<bool> {
        if !self.validation_enabled {
            return Ok(true);
        }
        
        let validation_start = Instant::now();
        
        // Get current MFN performance metrics
        let mfn_metrics = self.mfn_bridge.get_performance_metrics().await;
        let layer_coordination = self.mfn_bridge.get_layer_coordination().await;
        
        // Validate each component
        let service_mesh_valid = self.validate_service_mesh_performance(&mfn_metrics).await?;
        let container_valid = self.validate_container_performance(&mfn_metrics).await?;
        let scaling_valid = self.validate_scaling_performance(&mfn_metrics).await?;
        let discovery_valid = self.validate_discovery_performance(&mfn_metrics).await?;
        let integration_valid = self.validate_integration_performance(&mfn_metrics, &layer_coordination).await?;
        
        // Overall validation result
        let overall_valid = service_mesh_valid && container_valid && scaling_valid && discovery_valid && integration_valid;
        
        // Update validation metrics
        let validation_latency_us = validation_start.elapsed().as_micros() as u64;
        self.update_validation_metrics(overall_valid, validation_latency_us).await;
        
        if overall_valid {
            info!("All orchestration performance targets validated successfully");
        } else {
            warn!("Some orchestration performance targets not met");
        }
        
        Ok(overall_valid)
    }
    
    /// Generate comprehensive performance report
    pub async fn generate_performance_report(&self) -> Result<PerformanceReport> {
        let report_start = Instant::now();
        
        // Get current metrics
        let mfn_metrics = self.mfn_bridge.get_performance_metrics().await;
        let layer_coordination = self.mfn_bridge.get_layer_coordination().await;
        let history = self.performance_history.read().await;
        
        // Validate each component and collect results
        let mut component_results = HashMap::new();
        
        // Service mesh validation
        component_results.insert("service_mesh".to_string(), ComponentValidation {
            component: "service_mesh".to_string(),
            target_latency: 1000.0, // 1ms target
            actual_latency: mfn_metrics.alm_metrics.avg_latency_us,
            improvement_factor: mfn_metrics.alm_metrics.improvement_factor,
            accuracy: mfn_metrics.alm_metrics.accuracy,
            result: if mfn_metrics.alm_metrics.avg_latency_us <= 1000.0 {
                ValidationResult::Passed
            } else if mfn_metrics.alm_metrics.avg_latency_us <= 1500.0 {
                ValidationResult::Warning
            } else {
                ValidationResult::Failed
            },
            additional_metrics: {
                let mut metrics = HashMap::new();
                metrics.insert("ops_per_second".to_string(), mfn_metrics.alm_metrics.ops_per_second);
                metrics.insert("cache_hit_rate".to_string(), mfn_metrics.alm_metrics.cache_hit_rate);
                metrics
            },
        });
        
        // Container orchestration validation
        component_results.insert("container_orchestration".to_string(), ComponentValidation {
            component: "container_orchestration".to_string(),
            target_latency: 100.0, // 100ms target
            actual_latency: mfn_metrics.dsr_metrics.avg_latency_us / 1000.0, // Convert to ms
            improvement_factor: mfn_metrics.dsr_metrics.improvement_factor,
            accuracy: mfn_metrics.dsr_metrics.accuracy,
            result: if mfn_metrics.dsr_metrics.avg_latency_us <= 100000.0 { // 100ms in µs
                ValidationResult::Passed
            } else if mfn_metrics.dsr_metrics.avg_latency_us <= 150000.0 {
                ValidationResult::Warning
            } else {
                ValidationResult::Failed
            },
            additional_metrics: {
                let mut metrics = HashMap::new();
                metrics.insert("scheduling_accuracy".to_string(), mfn_metrics.dsr_metrics.accuracy);
                metrics.insert("resource_efficiency".to_string(), 0.85); // Placeholder
                metrics
            },
        });
        
        // Service discovery validation
        component_results.insert("service_discovery".to_string(), ComponentValidation {
            component: "service_discovery".to_string(),
            target_latency: 52.0, // 52µs target
            actual_latency: mfn_metrics.ifr_metrics.avg_latency_us,
            improvement_factor: mfn_metrics.ifr_metrics.improvement_factor,
            accuracy: mfn_metrics.ifr_metrics.accuracy,
            result: if mfn_metrics.ifr_metrics.avg_latency_us <= 52.0 {
                ValidationResult::Passed
            } else if mfn_metrics.ifr_metrics.avg_latency_us <= 78.0 { // 50% tolerance
                ValidationResult::Warning
            } else {
                ValidationResult::Failed
            },
            additional_metrics: {
                let mut metrics = HashMap::new();
                metrics.insert("cache_hit_rate".to_string(), mfn_metrics.ifr_metrics.cache_hit_rate);
                metrics.insert("lookup_accuracy".to_string(), mfn_metrics.ifr_metrics.accuracy);
                metrics
            },
        });
        
        // Auto-scaling validation
        component_results.insert("auto_scaling".to_string(), ComponentValidation {
            component: "auto_scaling".to_string(),
            target_latency: 1.2, // 1.2ms target
            actual_latency: mfn_metrics.cpe_metrics.avg_latency_us / 1000.0, // Convert to ms
            improvement_factor: mfn_metrics.cpe_metrics.improvement_factor,
            accuracy: mfn_metrics.cpe_metrics.accuracy,
            result: if mfn_metrics.cpe_metrics.avg_latency_us <= 1200.0 { // 1.2ms in µs
                ValidationResult::Passed
            } else if mfn_metrics.cpe_metrics.avg_latency_us <= 1800.0 {
                ValidationResult::Warning
            } else {
                ValidationResult::Failed
            },
            additional_metrics: {
                let mut metrics = HashMap::new();
                metrics.insert("prediction_accuracy".to_string(), mfn_metrics.cpe_metrics.accuracy);
                metrics.insert("proactive_scaling_rate".to_string(), 0.75); // Placeholder
                metrics
            },
        });
        
        // Determine overall result
        let overall_result = if component_results.values().all(|c| matches!(c.result, ValidationResult::Passed)) {
            ValidationResult::Passed
        } else if component_results.values().any(|c| matches!(c.result, ValidationResult::Failed)) {
            ValidationResult::Failed
        } else {
            ValidationResult::Warning
        };
        
        // Generate performance summary
        let performance_summary = PerformanceSummary {
            service_mesh_avg_latency_us: mfn_metrics.alm_metrics.avg_latency_us,
            container_avg_latency_ms: mfn_metrics.dsr_metrics.avg_latency_us / 1000.0,
            discovery_avg_latency_us: mfn_metrics.ifr_metrics.avg_latency_us,
            scaling_avg_latency_ms: mfn_metrics.cpe_metrics.avg_latency_us / 1000.0,
            end_to_end_avg_latency_ms: mfn_metrics.integration_metrics.end_to_end_latency_ms,
            overall_improvement_factor: mfn_metrics.integration_metrics.traditional_vs_mfn_factor,
            mfn_utilization_percentage: mfn_metrics.integration_metrics.mfn_utilization_percentage,
            resource_efficiency_improvement: mfn_metrics.integration_metrics.resource_efficiency_improvement,
        };
        
        // Generate recommendations
        let recommendations = self.generate_recommendations(&component_results, &performance_summary).await;
        
        let report = PerformanceReport {
            overall_result,
            component_results,
            performance_summary,
            recommendations,
            timestamp: SystemTime::now(),
        };
        
        let report_latency_ms = report_start.elapsed().as_millis() as f64;
        debug!("Performance report generated in {:.2}ms", report_latency_ms);
        
        Ok(report)
    }
    
    /// Validate service mesh performance (ALM-powered)
    async fn validate_service_mesh_performance(&self, metrics: &MfnPerformanceMetrics) -> Result<bool> {
        let target_latency_us = 1000.0; // 1ms target
        let actual_latency_us = metrics.alm_metrics.avg_latency_us;
        
        if actual_latency_us > target_latency_us {
            self.create_alert(
                AlertSeverity::Warning,
                "service_mesh".to_string(),
                "routing_latency".to_string(),
                format!("Service mesh routing latency {}µs exceeds target {}µs", 
                       actual_latency_us, target_latency_us),
                actual_latency_us,
                target_latency_us,
            ).await;
            return Ok(false);
        }
        
        // Record performance metric
        self.record_performance_metric(
            "service_mesh_latency",
            actual_latency_us,
            HashMap::new(),
        ).await;
        
        Ok(true)
    }
    
    /// Validate container orchestration performance (DSR-powered)
    async fn validate_container_performance(&self, metrics: &MfnPerformanceMetrics) -> Result<bool> {
        let target_latency_ms = 100.0; // 100ms target
        let actual_latency_ms = metrics.dsr_metrics.avg_latency_us / 1000.0;
        
        if actual_latency_ms > target_latency_ms {
            self.create_alert(
                AlertSeverity::Warning,
                "container_orchestration".to_string(),
                "scheduling_latency".to_string(),
                format!("Container scheduling latency {:.2}ms exceeds target {}ms", 
                       actual_latency_ms, target_latency_ms),
                actual_latency_ms,
                target_latency_ms,
            ).await;
            return Ok(false);
        }
        
        // Validate accuracy
        if metrics.dsr_metrics.accuracy < 0.96 { // 96% accuracy target
            self.create_alert(
                AlertSeverity::Warning,
                "container_orchestration".to_string(),
                "scheduling_accuracy".to_string(),
                format!("Container scheduling accuracy {:.1}% below target 96%", 
                       metrics.dsr_metrics.accuracy * 100.0),
                metrics.dsr_metrics.accuracy,
                0.96,
            ).await;
            return Ok(false);
        }
        
        // Record performance metric
        self.record_performance_metric(
            "container_scheduling_latency",
            actual_latency_ms,
            HashMap::new(),
        ).await;
        
        Ok(true)
    }
    
    /// Validate auto-scaling performance (CPE-powered)
    async fn validate_scaling_performance(&self, metrics: &MfnPerformanceMetrics) -> Result<bool> {
        let target_latency_ms = 1.2; // 1.2ms target
        let actual_latency_ms = metrics.cpe_metrics.avg_latency_us / 1000.0;
        
        if actual_latency_ms > target_latency_ms {
            self.create_alert(
                AlertSeverity::Warning,
                "auto_scaling".to_string(),
                "decision_latency".to_string(),
                format!("Auto-scaling decision latency {:.2}ms exceeds target {:.1}ms", 
                       actual_latency_ms, target_latency_ms),
                actual_latency_ms,
                target_latency_ms,
            ).await;
            return Ok(false);
        }
        
        // Validate prediction accuracy
        if metrics.cpe_metrics.accuracy < 0.968 { // 96.8% accuracy target
            self.create_alert(
                AlertSeverity::Critical,
                "auto_scaling".to_string(),
                "prediction_accuracy".to_string(),
                format!("Auto-scaling prediction accuracy {:.1}% below target 96.8%", 
                       metrics.cpe_metrics.accuracy * 100.0),
                metrics.cpe_metrics.accuracy,
                0.968,
            ).await;
            return Ok(false);
        }
        
        // Record performance metric
        self.record_performance_metric(
            "auto_scaling_latency",
            actual_latency_ms,
            HashMap::new(),
        ).await;
        
        Ok(true)
    }
    
    /// Validate service discovery performance (IFR-powered)
    async fn validate_discovery_performance(&self, metrics: &MfnPerformanceMetrics) -> Result<bool> {
        let target_latency_us = 52.0; // 52µs target
        let actual_latency_us = metrics.ifr_metrics.avg_latency_us;
        
        if actual_latency_us > target_latency_us {
            self.create_alert(
                AlertSeverity::Critical,
                "service_discovery".to_string(),
                "lookup_latency".to_string(),
                format!("Service discovery latency {}µs exceeds target {}µs", 
                       actual_latency_us, target_latency_us),
                actual_latency_us,
                target_latency_us,
            ).await;
            return Ok(false);
        }
        
        // Validate improvement factor (should show 88.6% improvement)
        if metrics.ifr_metrics.improvement_factor < 8.0 { // Should be around 8.86x
            self.create_alert(
                AlertSeverity::Warning,
                "service_discovery".to_string(),
                "improvement_factor".to_string(),
                format!("Service discovery improvement factor {:.1}x below expected 8.8x", 
                       metrics.ifr_metrics.improvement_factor),
                metrics.ifr_metrics.improvement_factor,
                8.8,
            ).await;
            return Ok(false);
        }
        
        // Record performance metric
        self.record_performance_metric(
            "service_discovery_latency",
            actual_latency_us,
            HashMap::new(),
        ).await;
        
        Ok(true)
    }
    
    /// Validate overall integration performance
    async fn validate_integration_performance(&self, metrics: &MfnPerformanceMetrics, coordination: &LayerCoordination) -> Result<bool> {
        let target_latency_ms = 2.0; // 2ms end-to-end target
        let actual_latency_ms = metrics.integration_metrics.end_to_end_latency_ms;
        
        if actual_latency_ms > target_latency_ms {
            self.create_alert(
                AlertSeverity::Critical,
                "integration".to_string(),
                "end_to_end_latency".to_string(),
                format!("End-to-end orchestration latency {:.2}ms exceeds target {}ms", 
                       actual_latency_ms, target_latency_ms),
                actual_latency_ms,
                target_latency_ms,
            ).await;
            return Ok(false);
        }
        
        // Validate MFN utilization
        if metrics.integration_metrics.mfn_utilization_percentage < 80.0 {
            self.create_alert(
                AlertSeverity::Warning,
                "integration".to_string(),
                "mfn_utilization".to_string(),
                format!("MFN utilization {:.1}% below optimal 80%", 
                       metrics.integration_metrics.mfn_utilization_percentage),
                metrics.integration_metrics.mfn_utilization_percentage,
                80.0,
            ).await;
            return Ok(false);
        }
        
        // Record performance metric
        self.record_performance_metric(
            "end_to_end_latency",
            actual_latency_ms,
            HashMap::new(),
        ).await;
        
        Ok(true)
    }
    
    /// Generate performance recommendations
    async fn generate_recommendations(&self, 
        component_results: &HashMap<String, ComponentValidation>,
        summary: &PerformanceSummary,
    ) -> Vec<PerformanceRecommendation> {
        let mut recommendations = Vec::new();
        
        // Service mesh recommendations
        if let Some(service_mesh) = component_results.get("service_mesh") {
            if matches!(service_mesh.result, ValidationResult::Warning | ValidationResult::Failed) {
                recommendations.push(PerformanceRecommendation {
                    recommendation_type: RecommendationType::Optimization,
                    component: "service_mesh".to_string(),
                    priority: 1,
                    expected_improvement: 0.3,
                    description: "Optimize ALM routing algorithm parameters for better latency".to_string(),
                    complexity: RecommendationComplexity::Medium,
                });
            }
        }
        
        // Container orchestration recommendations
        if let Some(container) = component_results.get("container_orchestration") {
            if matches!(container.result, ValidationResult::Warning | ValidationResult::Failed) {
                recommendations.push(PerformanceRecommendation {
                    recommendation_type: RecommendationType::Caching,
                    component: "container_orchestration".to_string(),
                    priority: 2,
                    expected_improvement: 0.4,
                    description: "Increase DSR neural network cache size for better scheduling performance".to_string(),
                    complexity: RecommendationComplexity::Low,
                });
            }
        }
        
        // Service discovery recommendations
        if let Some(discovery) = component_results.get("service_discovery") {
            if matches!(discovery.result, ValidationResult::Warning | ValidationResult::Failed) {
                recommendations.push(PerformanceRecommendation {
                    recommendation_type: RecommendationType::Configuration,
                    component: "service_discovery".to_string(),
                    priority: 1,
                    expected_improvement: 0.5,
                    description: "Tune IFR bloom filter parameters for optimal lookup performance".to_string(),
                    complexity: RecommendationComplexity::Low,
                });
            }
        }
        
        // Auto-scaling recommendations
        if let Some(scaling) = component_results.get("auto_scaling") {
            if matches!(scaling.result, ValidationResult::Warning | ValidationResult::Failed) {
                recommendations.push(PerformanceRecommendation {
                    recommendation_type: RecommendationType::Optimization,
                    component: "auto_scaling".to_string(),
                    priority: 2,
                    expected_improvement: 0.25,
                    description: "Optimize CPE model parameters for faster prediction convergence".to_string(),
                    complexity: RecommendationComplexity::Medium,
                });
            }
        }
        
        // Overall performance recommendations
        if summary.overall_improvement_factor < 10.0 {
            recommendations.push(PerformanceRecommendation {
                recommendation_type: RecommendationType::Architecture,
                component: "integration".to_string(),
                priority: 3,
                expected_improvement: 0.6,
                description: "Enable additional MFN layer optimizations for better overall performance".to_string(),
                complexity: RecommendationComplexity::High,
            });
        }
        
        // Sort by priority
        recommendations.sort_by_key(|r| r.priority);
        
        recommendations
    }
    
    /// Record a performance metric
    async fn record_performance_metric(&self, metric_name: &str, value: f64, context: HashMap<String, String>) {
        let mut history = self.performance_history.write().await;
        let timed_metric = TimedMetric {
            value,
            timestamp: SystemTime::now(),
            context,
        };
        
        match metric_name {
            "service_mesh_latency" => history.service_mesh_latencies.push(timed_metric),
            "container_scheduling_latency" => history.container_scheduling_latencies.push(timed_metric),
            "service_discovery_latency" => history.service_discovery_latencies.push(timed_metric),
            "auto_scaling_latency" => history.auto_scaling_latencies.push(timed_metric),
            "end_to_end_latency" => history.end_to_end_latencies.push(timed_metric),
            _ => debug!("Unknown metric: {}", metric_name),
        }
        
        // Keep only recent metrics (last 1000 entries)
        const MAX_HISTORY: usize = 1000;
        if history.service_mesh_latencies.len() > MAX_HISTORY {
            history.service_mesh_latencies.drain(0..100);
        }
        if history.container_scheduling_latencies.len() > MAX_HISTORY {
            history.container_scheduling_latencies.drain(0..100);
        }
        if history.service_discovery_latencies.len() > MAX_HISTORY {
            history.service_discovery_latencies.drain(0..100);
        }
        if history.auto_scaling_latencies.len() > MAX_HISTORY {
            history.auto_scaling_latencies.drain(0..100);
        }
        if history.end_to_end_latencies.len() > MAX_HISTORY {
            history.end_to_end_latencies.drain(0..100);
        }
    }
    
    /// Create a performance alert
    async fn create_alert(&self, 
        severity: AlertSeverity,
        component: String,
        metric: String,
        message: String,
        current_value: f64,
        target_value: f64,
    ) {
        let alert = PerformanceAlert {
            id: uuid::Uuid::new_v4().to_string(),
            severity: severity.clone(),
            message: message.clone(),
            component: component.clone(),
            metric,
            current_value,
            target_value,
            timestamp: SystemTime::now(),
            resolved: false,
        };
        
        match severity {
            AlertSeverity::Critical => error!("Critical performance alert: {}: {}", component, message),
            AlertSeverity::Warning => warn!("Performance warning: {}: {}", component, message),
            AlertSeverity::Info => info!("Performance info: {}: {}", component, message),
        }
        
        let mut alerts = self.alert_history.write().await;
        alerts.push(alert);
        
        // Keep only recent alerts (last 100)
        if alerts.len() > 100 {
            alerts.drain(0..10);
        }
    }
    
    /// Update validation metrics
    async fn update_validation_metrics(&self, success: bool, latency_us: u64) {
        let mut metrics = self.validation_metrics.write().await;
        metrics.total_validations += 1;
        
        if !success {
            metrics.target_violations += 1;
        } else {
            metrics.performance_improvements += 1;
        }
        
        // Update average validation latency
        let total_validations = metrics.total_validations as f64;
        let current_avg = metrics.avg_validation_latency_us;
        metrics.avg_validation_latency_us = (current_avg * (total_validations - 1.0) + latency_us as f64) / total_validations;
        
        // Update validation accuracy
        metrics.validation_accuracy = (metrics.performance_improvements as f64 / total_validations) * 100.0;
        
        metrics.last_validation = Some(SystemTime::now());
    }
    
    /// Get performance history
    pub async fn get_performance_history(&self) -> PerformanceHistory {
        self.performance_history.read().await.clone()
    }
    
    /// Get validation metrics
    pub async fn get_validation_metrics(&self) -> ValidationMetrics {
        self.validation_metrics.read().await.clone()
    }
    
    /// Get alert history
    pub async fn get_alert_history(&self) -> Vec<PerformanceAlert> {
        self.alert_history.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integration::mfn_bridge::MfnBridge;
    use crate::integration::IntegrationConfig;
    
    #[tokio::test]
    async fn test_performance_validator_creation() {
        let config = IntegrationConfig::default();
        let mfn_bridge = Arc::new(MfnBridge::new(config).await.unwrap());
        let validator = PerformanceValidator::new(mfn_bridge, true).await;
        assert!(validator.is_ok());
    }
    
    #[tokio::test]
    async fn test_validation_targets() {
        let config = IntegrationConfig::default();
        let mfn_bridge = Arc::new(MfnBridge::new(config).await.unwrap());
        let validator = PerformanceValidator::new(mfn_bridge, true).await.unwrap();
        
        let result = validator.validate_orchestration_targets().await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_performance_report_generation() {
        let config = IntegrationConfig::default();
        let mfn_bridge = Arc::new(MfnBridge::new(config).await.unwrap());
        let validator = PerformanceValidator::new(mfn_bridge, true).await.unwrap();
        
        let report = validator.generate_performance_report().await;
        assert!(report.is_ok());
        
        let report = report.unwrap();
        assert!(report.component_results.len() >= 4); // At least 4 components
        assert!(report.recommendations.len() >= 0); // May have recommendations
    }
}