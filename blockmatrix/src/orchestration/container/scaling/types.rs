// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Type definitions for predictive container scaling.

use crate::ServiceId;
use super::super::ScalingAction;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime};

/// Service scaling policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceScalingPolicy {
    /// Service identifier
    pub service_id: ServiceId,
    /// Minimum replicas
    pub min_replicas: u32,
    /// Maximum replicas
    pub max_replicas: u32,
    /// Scaling thresholds
    pub thresholds: ScalingThresholds,
    /// Scaling behavior
    pub scaling_behavior: ScalingBehavior,
    /// Predictive scaling settings
    pub predictive_settings: PredictiveScalingSettings,
    /// Policy enabled
    pub enabled: bool,
    /// Last updated
    pub last_updated: SystemTime,
}

/// Scaling thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingThresholds {
    /// CPU utilization threshold for scale up (0.0 - 1.0)
    pub cpu_scale_up_threshold: f64,
    /// CPU utilization threshold for scale down (0.0 - 1.0)
    pub cpu_scale_down_threshold: f64,
    /// Memory utilization threshold for scale up (0.0 - 1.0)
    pub memory_scale_up_threshold: f64,
    /// Memory utilization threshold for scale down (0.0 - 1.0)
    pub memory_scale_down_threshold: f64,
    /// Request rate threshold for scale up (requests/second)
    pub request_rate_scale_up: f64,
    /// Request rate threshold for scale down (requests/second)
    pub request_rate_scale_down: f64,
    /// Response time threshold for scale up (ms)
    pub response_time_scale_up: f64,
    /// Custom metric thresholds
    pub custom_thresholds: HashMap<String, CustomThreshold>,
}

/// Custom scaling threshold
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomThreshold {
    /// Metric name
    pub metric_name: String,
    /// Scale up threshold
    pub scale_up_value: f64,
    /// Scale down threshold
    pub scale_down_value: f64,
    /// Threshold type
    pub threshold_type: ThresholdType,
}

/// Threshold comparison types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThresholdType {
    GreaterThan,
    LessThan,
    EqualTo,
    WithinRange { min: f64, max: f64 },
}

/// Scaling behavior configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingBehavior {
    /// Scale up behavior
    pub scale_up: ScalingDirection,
    /// Scale down behavior
    pub scale_down: ScalingDirection,
    /// Stabilization window
    pub stabilization_window: Duration,
    /// Maximum scaling step
    pub max_scaling_step: u32,
}

/// Scaling direction behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingDirection {
    /// Scaling policies for this direction
    pub policies: Vec<ScalingDirectionPolicy>,
    /// Cooldown period
    pub cooldown: Duration,
    /// Select policy mode
    pub select_policy: SelectPolicyMode,
}

/// Scaling direction policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingDirectionPolicy {
    /// Policy type
    pub policy_type: ScalingPolicyType,
    /// Policy value
    pub value: u32,
    /// Policy period
    pub period: Duration,
}

/// Scaling policy types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScalingPolicyType {
    Pods,
    Percent,
}

/// Policy selection modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SelectPolicyMode {
    Max,
    Min,
    Disabled,
}

/// Predictive scaling settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictiveScalingSettings {
    /// Predictive scaling enabled
    pub enabled: bool,
    /// Prediction horizon (seconds)
    pub prediction_horizon: u64,
    /// Prediction confidence threshold
    pub confidence_threshold: f64,
    /// Proactive scaling margin
    pub proactive_margin: f64,
    /// Learning period for predictions
    pub learning_period: Duration,
    /// Maximum proactive scaling
    pub max_proactive_scale: u32,
}

/// Scaling decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingDecision {
    /// Decision ID
    pub decision_id: String,
    /// Service being scaled
    pub service_id: ServiceId,
    /// Scaling action
    pub scaling_action: ScalingAction,
    /// Current replica count
    pub current_replicas: u32,
    /// Target replica count
    pub target_replicas: u32,
    /// Decision trigger
    pub trigger: ScalingTrigger,
    /// Decision confidence
    pub confidence: f64,
    /// CPE prediction used
    pub cpe_enhanced: bool,
    /// Decision latency (ms)
    pub decision_latency_ms: u64,
    /// Decision timestamp
    pub timestamp: SystemTime,
}

/// Scaling triggers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScalingTrigger {
    CpuUtilization { current: f64, threshold: f64 },
    MemoryUtilization { current: f64, threshold: f64 },
    RequestRate { current: f64, threshold: f64 },
    ResponseTime { current: f64, threshold: f64 },
    PredictiveTrigger {
        predicted_metric: String,
        predicted_value: f64,
        confidence: f64,
    },
    CustomMetric {
        metric_name: String,
        current: f64,
        threshold: f64,
    },
    Manual { reason: String },
}

/// Workload prediction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadPrediction {
    /// Service identifier
    pub service_id: ServiceId,
    /// Prediction timestamp
    pub prediction_timestamp: SystemTime,
    /// Prediction horizon
    pub horizon_seconds: u64,
    /// Predicted metrics
    pub predicted_metrics: PredictedMetrics,
    /// Prediction confidence
    pub confidence: f64,
    /// Recommended scaling action
    pub recommended_action: ScalingAction,
    /// Prediction reasoning
    pub reasoning: PredictionReasoning,
}

/// Predicted metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictedMetrics {
    /// Predicted CPU utilization
    pub cpu_utilization: f64,
    /// Predicted memory utilization
    pub memory_utilization: f64,
    /// Predicted request rate
    pub request_rate: f64,
    /// Predicted response time
    pub response_time: f64,
    /// Predicted resource demand
    pub resource_demand: f64,
    /// Custom predicted metrics
    pub custom_metrics: HashMap<String, f64>,
}

/// Prediction reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionReasoning {
    /// Primary prediction factors
    pub primary_factors: Vec<String>,
    /// Historical patterns identified
    pub patterns_identified: Vec<String>,
    /// Confidence factors
    pub confidence_factors: Vec<String>,
    /// Risk assessment
    pub risks: Vec<String>,
}

/// Cached prediction result
#[derive(Debug, Clone)]
pub struct CachedPrediction {
    /// Prediction result
    pub prediction: WorkloadPrediction,
    /// Cache timestamp
    pub cached_at: Instant,
    /// Cache TTL
    pub ttl: Duration,
    /// Access count
    pub access_count: u32,
}

/// Scaling record for learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingRecord {
    /// Record timestamp
    pub timestamp: SystemTime,
    /// Scaling decision
    pub decision: ScalingDecision,
    /// Workload context at time of scaling
    pub workload_context: WorkloadContext,
    /// Scaling outcome
    pub outcome: ScalingOutcome,
    /// Performance impact
    pub performance_impact: Option<PerformanceImpact>,
}

/// Workload context at scaling time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadContext {
    /// Current resource utilization
    pub resource_utilization: AggregateResourceUsage,
    /// Request patterns
    pub request_patterns: RequestPatterns,
    /// Time-based context
    pub temporal_context: TemporalContext,
    /// Service health
    pub service_health: ServiceHealth,
}

/// Aggregate resource usage across service instances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateResourceUsage {
    pub avg_cpu_utilization: f64,
    pub peak_cpu_utilization: f64,
    pub avg_memory_utilization: f64,
    pub peak_memory_utilization: f64,
    pub total_network_io: u64,
    pub total_disk_io: u64,
}

/// Request patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestPatterns {
    pub current_rps: f64,
    pub peak_rps_1h: f64,
    pub avg_response_time: f64,
    pub p95_response_time: f64,
    pub error_rate: f64,
}

/// Temporal context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalContext {
    pub hour_of_day: u8,
    pub day_of_week: u8,
    pub day_of_month: u8,
    pub is_weekend: bool,
    pub is_business_hours: bool,
    pub special_events: Vec<String>,
}

/// Service health indicators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub status: HealthStatus,
    pub healthy_instances: u32,
    pub total_instances: u32,
    pub recent_failures: u32,
    pub availability: f64,
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    PartiallyUnavailable,
    Unavailable,
}

/// Scaling outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScalingOutcome {
    Success { completion_time_ms: u64, final_replicas: u32 },
    PartialSuccess { achieved_replicas: u32, reasons: Vec<String> },
    Failure { reason: String, failure_time_ms: u64 },
    Cancelled { reason: String },
}

/// Performance impact of scaling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceImpact {
    pub response_time_delta: f64,
    pub throughput_delta: f64,
    pub efficiency_delta: f64,
    pub cost_delta: f64,
    pub overall_impact: f64,
}

/// Scaling metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingMetrics {
    pub total_decisions: u64,
    pub cpe_enhanced_decisions: u64,
    pub successful_scalings: u64,
    pub failed_scalings: u64,
    pub avg_decision_latency_ms: f64,
    pub proactive_scaling_accuracy: f64,
    pub resource_efficiency_improvement: f64,
    pub cost_optimization: f64,
}
