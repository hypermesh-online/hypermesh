//! Metrics Types for Performance Monitoring
//!
//! Defines all metric structures for the monitoring system

use std::time::{Duration, Instant, SystemTime};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// STOQ Transport layer metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoqMetrics {
    /// Throughput metrics (40 Gbps target)
    pub current_throughput_gbps: f64,
    pub peak_throughput_gbps: f64,
    pub avg_throughput_gbps: f64,
    pub throughput_samples: Vec<(Instant, f64)>,

    /// QUIC performance metrics
    pub quic_connections: u32,
    pub quic_streams: u32,
    pub quic_rtt_ms: f64,
    pub quic_loss_rate: f64,
    pub quic_congestion_events: u64,

    /// Message throughput
    pub messages_per_second: u64,
    pub avg_message_size_bytes: u64,
    pub max_message_size_bytes: u64,

    /// Network metrics
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub packet_loss_rate: f64,

    /// Error tracking
    pub connection_errors: u64,
    pub timeout_errors: u64,
    pub protocol_errors: u64,

    /// Resource utilization
    pub cpu_usage_percent: f32,
    pub memory_usage_mb: u64,
    pub io_operations: u64,
}

/// HyperMesh asset layer metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HyperMeshMetrics {
    /// Consensus validation metrics
    pub consensus_validations_total: u64,
    pub consensus_validations_failed: u64,
    pub avg_consensus_time_ms: f64,
    pub max_consensus_time_ms: f64,

    /// Asset allocation metrics
    pub assets_allocated: u64,
    pub assets_available: u64,
    pub allocation_requests: u64,
    pub allocation_failures: u64,
    pub avg_allocation_time_ms: f64,

    /// Resource tracking
    pub cpu_cores_allocated: u32,
    pub memory_gb_allocated: f32,
    pub storage_gb_allocated: u64,
    pub gpu_count_allocated: u32,
    pub bandwidth_gbps_allocated: f32,

    /// Proxy/NAT metrics
    pub proxy_connections: u64,
    pub proxy_bytes_transferred: u64,
    pub nat_translations: u64,
    pub nat_table_size: u64,

    /// VM execution metrics
    pub vm_executions_total: u64,
    pub vm_executions_active: u32,
    pub vm_executions_failed: u64,
    pub avg_vm_startup_time_ms: f64,
}

/// TrustChain authority layer metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrustChainMetrics {
    /// Certificate operations metrics
    pub certificates_issued: u64,
    pub certificates_verified: u64,
    pub certificates_revoked: u64,
    pub avg_cert_generation_ms: f64,
    pub avg_cert_verification_ms: f64,

    /// Chain validation metrics
    pub chain_validations: u64,
    pub chain_validation_failures: u64,
    pub avg_chain_validation_ms: f64,
    pub max_chain_depth: u32,

    /// Key management metrics
    pub keys_generated: u64,
    pub key_rotations: u64,
    pub quantum_resistant_ops: u64,
    pub avg_quantum_crypto_ms: f64,

    /// DNS integration metrics
    pub dns_queries_resolved: u64,
    pub dns_queries_failed: u64,
    pub avg_dns_resolution_ms: f64,
    pub dnssec_validations: u64,

    /// Storage metrics
    pub certificates_cached: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_size_mb: f64,
}

/// Integration layer metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IntegrationMetrics {
    /// Cross-layer coordination
    pub cross_layer_calls: u64,
    pub avg_coordination_time_ms: f64,
    pub layer_sync_events: u64,

    /// HTTP3 bridge metrics
    pub http3_requests: u64,
    pub http3_responses: u64,
    pub avg_http3_latency_ms: f64,

    /// End-to-end performance
    pub e2e_requests_total: u64,
    pub e2e_requests_successful: u64,
    pub avg_e2e_latency_ms: f64,
    pub p99_e2e_latency_ms: f64,

    /// Protocol conversion metrics
    pub protocol_conversions: u64,
    pub conversion_errors: u64,
    pub avg_conversion_time_ms: f64,
}

/// Overall stack metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StackMetrics {
    /// System health
    pub overall_health_score: f64,
    pub uptime_seconds: u64,
    pub system_load_avg: f64,

    /// Performance targets
    pub meeting_throughput_target: bool,
    pub meeting_latency_target: bool,
    pub meeting_consensus_target: bool,
    pub meeting_certificate_target: bool,

    /// Resource usage
    pub total_cpu_usage_percent: f32,
    pub total_memory_usage_gb: f32,
    pub total_network_usage_gbps: f32,
    pub total_storage_usage_tb: f32,

    /// Aggregate statistics
    pub total_requests: u64,
    pub total_errors: u64,
    pub error_rate: f64,
    pub avg_response_time_ms: f64,

    /// Performance scores (0-100)
    pub stoq_performance_score: f64,
    pub hypermesh_performance_score: f64,
    pub trustchain_performance_score: f64,
    pub integration_performance_score: f64,

    /// Trend analysis
    pub throughput_trend: TrendDirection,
    pub latency_trend: TrendDirection,
    pub error_trend: TrendDirection,
    pub resource_trend: TrendDirection,

    /// Last update time
    pub last_update: SystemTime,
}

/// Performance alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAlert {
    /// Alert identifier
    pub id: String,

    /// Alert level
    pub level: AlertLevel,

    /// Alert category
    pub category: AlertCategory,

    /// Alert message
    pub message: String,

    /// Metric that triggered the alert
    pub metric_name: String,
    pub metric_value: f64,
    pub threshold_value: f64,

    /// Alert timestamps
    pub triggered_at: SystemTime,
    pub resolved_at: Option<SystemTime>,

    /// Alert metadata
    pub metadata: HashMap<String, String>,

    /// Action taken
    pub action_taken: Option<String>,
}

/// Alert levels
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

/// Alert categories
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertCategory {
    Throughput,
    Latency,
    Errors,
    Resources,
    Security,
    Consensus,
    Certificates,
}

/// Monitoring state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MonitoringState {
    pub monitoring_enabled: bool,
    pub collection_interval: Duration,
    pub retention_period: Duration,
    pub alert_thresholds: AlertThresholds,
    pub export_enabled: bool,
    pub export_interval: Duration,
}

/// Alert thresholds configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub throughput_min_gbps: f64,
    pub latency_max_ms: f64,
    pub error_rate_max_percent: f64,
    pub cpu_usage_max_percent: f32,
    pub memory_usage_max_percent: f32,
    pub consensus_time_max_ms: f64,
    pub certificate_time_max_ms: f64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            throughput_min_gbps: 30.0,     // Alert if below 30 Gbps
            latency_max_ms: 100.0,          // Alert if above 100ms
            error_rate_max_percent: 1.0,    // Alert if above 1% errors
            cpu_usage_max_percent: 80.0,    // Alert if above 80% CPU
            memory_usage_max_percent: 90.0, // Alert if above 90% memory
            consensus_time_max_ms: 100.0,   // Alert if consensus > 100ms
            certificate_time_max_ms: 35.0,  // Alert if certs > 35ms
        }
    }
}

/// Stack statistics for reporting
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StackStatistics {
    /// Time period
    pub period_start: SystemTime,
    pub period_end: SystemTime,

    /// Performance summary
    pub avg_throughput_gbps: f64,
    pub peak_throughput_gbps: f64,
    pub min_throughput_gbps: f64,

    pub avg_latency_ms: f64,
    pub p50_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,

    /// Reliability
    pub availability_percent: f64,
    pub successful_requests: u64,
    pub failed_requests: u64,

    /// Resource efficiency
    pub avg_cpu_efficiency: f64,
    pub avg_memory_efficiency: f64,
    pub avg_network_efficiency: f64,
}

/// Layer health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerHealth {
    pub layer_name: String,
    pub status: HealthStatus,
    pub health_score: f64,
    pub last_check: SystemTime,
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Trend direction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrendDirection {
    Improving,
    Stable,
    Degrading,
}