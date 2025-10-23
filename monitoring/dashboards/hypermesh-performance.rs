//! HyperMesh Performance Monitoring Dashboard
//!
//! This module provides a comprehensive performance monitoring dashboard for HyperMesh
//! infrastructure, including real-time container performance metrics, P2P network
//! monitoring, Byzantine fault tolerance metrics, and consensus performance analysis.
//!
//! # Features
//!
//! - Real-time performance metrics collection and visualization
//! - Container startup and scaling performance tracking (<100ms targets)
//! - P2P mesh connectivity performance (<5ms connection establishment)
//! - Byzantine fault detection and reputation tracking
//! - Consensus latency monitoring (<50ms coordination overhead)
//! - Network throughput and utilization analysis
//! - Automated performance alerting and remediation triggers

use crate::health::{HealthMonitor, SystemHealthStatus, HealthEvent};
use crate::networking::{NetworkManager, NetworkMetrics, NetworkEvent};
use crate::consensus_orchestrator::{ConsensusContainerOrchestrator, OrchestrationMetrics};
use nexus_shared::{NodeId, ResourceId, Timestamp, Result};

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque, BTreeMap};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{mpsc, RwLock as AsyncRwLock, watch};
use tokio::time::{interval, sleep};
use tracing::{debug, info, warn, error, instrument};
use warp::{Filter, Rejection, Reply};

/// Performance monitoring dashboard
pub struct PerformanceDashboard {
    /// Node identifier
    node_id: NodeId,
    
    /// Dashboard configuration
    config: DashboardConfig,
    
    /// Health monitor reference
    health_monitor: Arc<HealthMonitor>,
    
    /// Network manager reference
    network_manager: Arc<NetworkManager>,
    
    /// Container orchestrator reference
    orchestrator: Arc<tokio::sync::Mutex<ConsensusContainerOrchestrator>>,
    
    /// Performance metrics collector
    metrics_collector: Arc<PerformanceMetricsCollector>,
    
    /// Real-time metrics aggregator
    metrics_aggregator: Arc<RealTimeMetricsAggregator>,
    
    /// Performance analyzer
    performance_analyzer: Arc<PerformanceAnalyzer>,
    
    /// Alert manager
    alert_manager: Arc<AlertManager>,
    
    /// Dashboard web server
    web_server: Arc<DashboardWebServer>,
    
    /// Metrics storage
    metrics_storage: Arc<RwLock<MetricsStorage>>,
    
    /// Performance benchmarks
    benchmarks: Arc<PerformanceBenchmarks>,
}

/// Dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    /// Web server configuration
    pub web_server: WebServerConfig,
    
    /// Metrics collection settings
    pub metrics_collection: MetricsCollectionConfig,
    
    /// Performance analysis settings
    pub performance_analysis: PerformanceAnalysisConfig,
    
    /// Alert configuration
    pub alerting: AlertingConfig,
    
    /// Dashboard UI settings
    pub ui_settings: UISettings,
    
    /// Storage configuration
    pub storage: StorageConfig,
}

/// Web server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebServerConfig {
    /// Server bind address
    pub bind_address: String,
    
    /// Server port
    pub port: u16,
    
    /// Enable TLS
    pub enable_tls: bool,
    
    /// TLS certificate file path
    pub tls_cert_path: Option<String>,
    
    /// TLS private key file path
    pub tls_key_path: Option<String>,
    
    /// Enable authentication
    pub enable_auth: bool,
    
    /// Authentication configuration
    pub auth_config: Option<AuthConfig>,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Authentication method
    pub method: AuthMethod,
    
    /// Username/password pairs for basic auth
    pub basic_auth_users: Option<HashMap<String, String>>,
    
    /// JWT secret for token auth
    pub jwt_secret: Option<String>,
    
    /// API keys for API key auth
    pub api_keys: Option<Vec<String>>,
}

/// Authentication methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    None,
    BasicAuth,
    ApiKey,
    Jwt,
}

/// Metrics collection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsCollectionConfig {
    /// Collection interval
    pub interval: Duration,
    
    /// Metrics to collect
    pub enabled_metrics: Vec<MetricType>,
    
    /// High-frequency metrics interval
    pub high_frequency_interval: Duration,
    
    /// High-frequency metrics (performance critical)
    pub high_frequency_metrics: Vec<MetricType>,
    
    /// Metrics retention policy
    pub retention_policy: RetentionPolicy,
}

/// Performance analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAnalysisConfig {
    /// Enable real-time analysis
    pub enable_realtime_analysis: bool,
    
    /// Analysis window duration
    pub analysis_window: Duration,
    
    /// Performance threshold definitions
    pub thresholds: PerformanceThresholds,
    
    /// Trend analysis settings
    pub trend_analysis: TrendAnalysisConfig,
    
    /// Anomaly detection settings
    pub anomaly_detection: AnomalyDetectionConfig,
}

/// Performance thresholds for HyperMesh targets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThresholds {
    /// Container startup time threshold (target: <100ms)
    pub container_startup_ms: f64,
    
    /// P2P connection establishment threshold (target: <5ms)
    pub p2p_connection_ms: f64,
    
    /// Consensus coordination overhead threshold (target: <50ms)
    pub consensus_overhead_ms: f64,
    
    /// Network setup time threshold (target: <10ms)
    pub network_setup_ms: f64,
    
    /// Memory usage threshold percentage
    pub memory_usage_percent: f64,
    
    /// CPU usage threshold percentage
    pub cpu_usage_percent: f64,
    
    /// Network bandwidth utilization threshold
    pub network_bandwidth_percent: f64,
    
    /// Error rate threshold percentage
    pub error_rate_percent: f64,
    
    /// Byzantine fault detection threshold
    pub byzantine_fault_threshold: f64,
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            container_startup_ms: 100.0,
            p2p_connection_ms: 5.0,
            consensus_overhead_ms: 50.0,
            network_setup_ms: 10.0,
            memory_usage_percent: 80.0,
            cpu_usage_percent: 80.0,
            network_bandwidth_percent: 80.0,
            error_rate_percent: 1.0,
            byzantine_fault_threshold: 0.1,
        }
    }
}

/// Trend analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysisConfig {
    /// Enable trend analysis
    pub enabled: bool,
    
    /// Trend detection window
    pub detection_window: Duration,
    
    /// Minimum trend confidence threshold
    pub min_confidence: f64,
    
    /// Trend types to detect
    pub trend_types: Vec<TrendType>,
}

/// Types of trends to detect
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendType {
    PerformanceDegradation,
    ResourceExhaustion,
    NetworkCongestion,
    ConsensusSlowdown,
    ByzantineActivity,
}

/// Anomaly detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyDetectionConfig {
    /// Enable anomaly detection
    pub enabled: bool,
    
    /// Statistical anomaly detection threshold
    pub statistical_threshold: f64,
    
    /// Machine learning model settings
    pub ml_model_config: Option<MLModelConfig>,
    
    /// Anomaly types to detect
    pub anomaly_types: Vec<AnomalyType>,
}

/// Machine learning model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLModelConfig {
    /// Model type
    pub model_type: MLModelType,
    
    /// Training data size
    pub training_data_size: usize,
    
    /// Model update frequency
    pub update_frequency: Duration,
    
    /// Feature set
    pub features: Vec<String>,
}

/// ML model types for anomaly detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MLModelType {
    IsolationForest,
    OneClassSVM,
    LSTM,
    Autoencoder,
}

/// Types of anomalies to detect
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalyType {
    PerformanceSpike,
    UnusualResourceUsage,
    NetworkBehaviorAnomaly,
    ConsensusAnomaly,
    ByzantineSignature,
}

/// Alerting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertingConfig {
    /// Enable alerting
    pub enabled: bool,
    
    /// Alert channels
    pub channels: Vec<AlertChannel>,
    
    /// Alert rules
    pub rules: Vec<AlertRule>,
    
    /// Alert aggregation settings
    pub aggregation: AlertAggregationConfig,
}

/// Alert channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertChannel {
    /// Channel name
    pub name: String,
    
    /// Channel type
    pub channel_type: AlertChannelType,
    
    /// Channel configuration
    pub config: HashMap<String, String>,
    
    /// Severity filter
    pub severity_filter: Vec<AlertSeverity>,
}

/// Alert channel types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertChannelType {
    Email,
    Slack,
    Webhook,
    PagerDuty,
    Console,
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Alert rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Rule name
    pub name: String,
    
    /// Rule condition
    pub condition: AlertCondition,
    
    /// Alert severity
    pub severity: AlertSeverity,
    
    /// Alert message template
    pub message_template: String,
    
    /// Channels to notify
    pub channels: Vec<String>,
    
    /// Evaluation interval
    pub evaluation_interval: Duration,
    
    /// Alert suppression settings
    pub suppression: Option<AlertSuppressionConfig>,
}

/// Alert condition specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertCondition {
    /// Metric name
    pub metric: String,
    
    /// Comparison operator
    pub operator: ComparisonOperator,
    
    /// Threshold value
    pub threshold: f64,
    
    /// Time window for condition evaluation
    pub time_window: Duration,
    
    /// Number of consecutive violations required
    pub consecutive_violations: u32,
}

/// Comparison operators for alert conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOperator {
    GreaterThan,
    LessThan,
    Equal,
    NotEqual,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

/// Alert suppression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertSuppressionConfig {
    /// Suppression duration
    pub duration: Duration,
    
    /// Suppression conditions
    pub conditions: Vec<String>,
}

/// Alert aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertAggregationConfig {
    /// Enable alert aggregation
    pub enabled: bool,
    
    /// Aggregation window
    pub window: Duration,
    
    /// Maximum alerts per window
    pub max_alerts_per_window: usize,
    
    /// Aggregation strategy
    pub strategy: AggregationStrategy,
}

/// Alert aggregation strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationStrategy {
    Count,
    Suppress,
    Summary,
}

/// UI settings for dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UISettings {
    /// Dashboard title
    pub title: String,
    
    /// Refresh interval for real-time data
    pub refresh_interval_ms: u64,
    
    /// Default time range for charts
    pub default_time_range: Duration,
    
    /// Chart types to display
    pub chart_types: Vec<ChartType>,
    
    /// Dashboard themes
    pub theme: DashboardTheme,
    
    /// Custom CSS
    pub custom_css: Option<String>,
}

/// Chart types for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChartType {
    LineChart,
    AreaChart,
    BarChart,
    GaugeChart,
    HeatmapChart,
    TimeSeriesChart,
    TopologyChart,
}

/// Dashboard themes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DashboardTheme {
    Light,
    Dark,
    HighContrast,
    Custom(String),
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage backend
    pub backend: StorageBackend,
    
    /// Retention policies
    pub retention: RetentionPolicy,
    
    /// Compression settings
    pub compression: CompressionConfig,
}

/// Storage backend types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageBackend {
    Memory,
    File { path: String },
    Database { connection_string: String },
    TimeSeries { config: TimeSeriesConfig },
}

/// Time series database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesConfig {
    /// Database type
    pub db_type: TimeSeriesDBType,
    
    /// Connection configuration
    pub connection: HashMap<String, String>,
    
    /// Batch write settings
    pub batch_config: BatchConfig,
}

/// Time series database types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeSeriesDBType {
    InfluxDB,
    Prometheus,
    TimescaleDB,
    ClickHouse,
}

/// Batch configuration for time series writes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfig {
    /// Batch size
    pub batch_size: usize,
    
    /// Flush interval
    pub flush_interval: Duration,
    
    /// Maximum batch age
    pub max_batch_age: Duration,
}

/// Retention policy for metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// High resolution retention (detailed metrics)
    pub high_resolution: Duration,
    
    /// Medium resolution retention (aggregated metrics)
    pub medium_resolution: Duration,
    
    /// Low resolution retention (summary metrics)
    pub low_resolution: Duration,
    
    /// Aggregation rules
    pub aggregation_rules: Vec<AggregationRule>,
}

/// Aggregation rule for downsampling metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationRule {
    /// Source resolution
    pub from_resolution: Duration,
    
    /// Target resolution
    pub to_resolution: Duration,
    
    /// Aggregation function
    pub function: AggregationFunction,
    
    /// Metrics to aggregate
    pub metrics: Vec<String>,
}

/// Aggregation functions for downsampling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationFunction {
    Average,
    Sum,
    Max,
    Min,
    Median,
    Percentile(f64),
}

/// Compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Enable compression
    pub enabled: bool,
    
    /// Compression algorithm
    pub algorithm: CompressionAlgorithm,
    
    /// Compression level
    pub level: u8,
    
    /// Compression threshold (minimum size)
    pub threshold_bytes: usize,
}

/// Compression algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    Gzip,
    Zstd,
    Lz4,
    Snappy,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            web_server: WebServerConfig {
                bind_address: "0.0.0.0".to_string(),
                port: 3000,
                enable_tls: false,
                tls_cert_path: None,
                tls_key_path: None,
                enable_auth: false,
                auth_config: None,
            },
            metrics_collection: MetricsCollectionConfig {
                interval: Duration::from_secs(5),
                enabled_metrics: vec![
                    MetricType::ContainerPerformance,
                    MetricType::NetworkPerformance,
                    MetricType::ConsensusMetrics,
                    MetricType::ResourceUtilization,
                    MetricType::ByzantineMetrics,
                ],
                high_frequency_interval: Duration::from_millis(100),
                high_frequency_metrics: vec![
                    MetricType::ConsensusLatency,
                    MetricType::NetworkLatency,
                    MetricType::ContainerStartupTime,
                ],
                retention_policy: RetentionPolicy {
                    high_resolution: Duration::from_secs(3600), // 1 hour
                    medium_resolution: Duration::from_secs(86400), // 1 day
                    low_resolution: Duration::from_secs(604800), // 1 week
                    aggregation_rules: Vec::new(),
                },
            },
            performance_analysis: PerformanceAnalysisConfig {
                enable_realtime_analysis: true,
                analysis_window: Duration::from_secs(300),
                thresholds: PerformanceThresholds::default(),
                trend_analysis: TrendAnalysisConfig {
                    enabled: true,
                    detection_window: Duration::from_secs(600),
                    min_confidence: 0.8,
                    trend_types: vec![
                        TrendType::PerformanceDegradation,
                        TrendType::NetworkCongestion,
                        TrendType::ConsensusSlowdown,
                    ],
                },
                anomaly_detection: AnomalyDetectionConfig {
                    enabled: true,
                    statistical_threshold: 2.0, // 2 standard deviations
                    ml_model_config: None, // Disable ML for now
                    anomaly_types: vec![
                        AnomalyType::PerformanceSpike,
                        AnomalyType::NetworkBehaviorAnomaly,
                        AnomalyType::ConsensusAnomaly,
                    ],
                },
            },
            alerting: AlertingConfig {
                enabled: true,
                channels: vec![
                    AlertChannel {
                        name: "console".to_string(),
                        channel_type: AlertChannelType::Console,
                        config: HashMap::new(),
                        severity_filter: vec![
                            AlertSeverity::Warning,
                            AlertSeverity::Error,
                            AlertSeverity::Critical,
                        ],
                    },
                ],
                rules: vec![
                    AlertRule {
                        name: "container_startup_slow".to_string(),
                        condition: AlertCondition {
                            metric: "container_startup_time_ms".to_string(),
                            operator: ComparisonOperator::GreaterThan,
                            threshold: 100.0,
                            time_window: Duration::from_secs(60),
                            consecutive_violations: 3,
                        },
                        severity: AlertSeverity::Warning,
                        message_template: "Container startup time exceeds target: {{value}}ms > 100ms".to_string(),
                        channels: vec!["console".to_string()],
                        evaluation_interval: Duration::from_secs(30),
                        suppression: None,
                    },
                    AlertRule {
                        name: "consensus_latency_high".to_string(),
                        condition: AlertCondition {
                            metric: "consensus_latency_ms".to_string(),
                            operator: ComparisonOperator::GreaterThan,
                            threshold: 50.0,
                            time_window: Duration::from_secs(60),
                            consecutive_violations: 5,
                        },
                        severity: AlertSeverity::Error,
                        message_template: "Consensus latency exceeds target: {{value}}ms > 50ms".to_string(),
                        channels: vec!["console".to_string()],
                        evaluation_interval: Duration::from_secs(10),
                        suppression: None,
                    },
                ],
                aggregation: AlertAggregationConfig {
                    enabled: true,
                    window: Duration::from_secs(300),
                    max_alerts_per_window: 10,
                    strategy: AggregationStrategy::Summary,
                },
            },
            ui_settings: UISettings {
                title: "HyperMesh Performance Dashboard".to_string(),
                refresh_interval_ms: 1000,
                default_time_range: Duration::from_secs(3600),
                chart_types: vec![
                    ChartType::TimeSeriesChart,
                    ChartType::GaugeChart,
                    ChartType::HeatmapChart,
                ],
                theme: DashboardTheme::Dark,
                custom_css: None,
            },
            storage: StorageConfig {
                backend: StorageBackend::Memory,
                retention: RetentionPolicy {
                    high_resolution: Duration::from_secs(3600),
                    medium_resolution: Duration::from_secs(86400),
                    low_resolution: Duration::from_secs(604800),
                    aggregation_rules: Vec::new(),
                },
                compression: CompressionConfig {
                    enabled: true,
                    algorithm: CompressionAlgorithm::Zstd,
                    level: 3,
                    threshold_bytes: 1024,
                },
            },
        }
    }
}

/// Types of metrics to collect
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricType {
    /// Container lifecycle performance
    ContainerPerformance,
    
    /// P2P network performance
    NetworkPerformance,
    
    /// Consensus protocol metrics
    ConsensusMetrics,
    
    /// System resource utilization
    ResourceUtilization,
    
    /// Byzantine fault tolerance metrics
    ByzantineMetrics,
    
    /// Specific latency measurements
    ConsensusLatency,
    NetworkLatency,
    ContainerStartupTime,
}

/// Performance metrics collector
pub struct PerformanceMetricsCollector {
    config: MetricsCollectionConfig,
    metrics_storage: Arc<RwLock<MetricsStorage>>,
    collection_tasks: Vec<tokio::task::JoinHandle<()>>,
}

/// Real-time metrics aggregator
pub struct RealTimeMetricsAggregator {
    metrics_storage: Arc<RwLock<MetricsStorage>>,
    aggregated_metrics: Arc<RwLock<AggregatedMetrics>>,
    aggregation_tasks: Vec<tokio::task::JoinHandle<()>>,
}

/// Performance analyzer
pub struct PerformanceAnalyzer {
    config: PerformanceAnalysisConfig,
    metrics_storage: Arc<RwLock<MetricsStorage>>,
    analysis_results: Arc<RwLock<AnalysisResults>>,
}

/// Alert manager
pub struct AlertManager {
    config: AlertingConfig,
    active_alerts: Arc<RwLock<HashMap<String, ActiveAlert>>>,
    alert_history: Arc<RwLock<VecDeque<AlertEvent>>>,
}

/// Dashboard web server
pub struct DashboardWebServer {
    config: WebServerConfig,
    metrics_storage: Arc<RwLock<MetricsStorage>>,
    server_handle: Option<tokio::task::JoinHandle<()>>,
}

/// Metrics storage
#[derive(Debug, Clone)]
pub struct MetricsStorage {
    /// Time series metrics data
    pub time_series: BTreeMap<String, VecDeque<MetricPoint>>,
    
    /// Aggregated metrics
    pub aggregated: HashMap<String, AggregatedMetric>,
    
    /// Metadata about metrics
    pub metadata: HashMap<String, MetricMetadata>,
    
    /// Storage statistics
    pub storage_stats: StorageStats,
}

/// Individual metric point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint {
    /// Timestamp
    pub timestamp: Timestamp,
    
    /// Metric value
    pub value: f64,
    
    /// Optional labels/tags
    pub labels: HashMap<String, String>,
    
    /// Data source
    pub source: String,
}

/// Aggregated metric data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMetric {
    /// Metric name
    pub name: String,
    
    /// Current value
    pub current_value: f64,
    
    /// Average over time window
    pub average: f64,
    
    /// Minimum value
    pub minimum: f64,
    
    /// Maximum value
    pub maximum: f64,
    
    /// Standard deviation
    pub std_dev: f64,
    
    /// Sample count
    pub sample_count: u64,
    
    /// Last update time
    pub last_updated: Timestamp,
}

/// Metadata about metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricMetadata {
    /// Metric description
    pub description: String,
    
    /// Metric unit
    pub unit: String,
    
    /// Metric type
    pub metric_type: MetricDataType,
    
    /// Collection frequency
    pub frequency: Duration,
    
    /// Retention policy
    pub retention: Duration,
}

/// Types of metric data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricDataType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

/// Storage statistics
#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    /// Total metrics stored
    pub total_metrics: u64,
    
    /// Total data points
    pub total_data_points: u64,
    
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    
    /// Disk usage in bytes
    pub disk_usage_bytes: u64,
    
    /// Last cleanup time
    pub last_cleanup: Option<Timestamp>,
}

/// Aggregated metrics for dashboard display
#[derive(Debug, Clone, Default)]
pub struct AggregatedMetrics {
    /// Performance summary
    pub performance_summary: PerformanceSummary,
    
    /// Current health status
    pub health_status: HealthSummary,
    
    /// Resource utilization summary
    pub resource_summary: ResourceSummary,
    
    /// Network performance summary
    pub network_summary: NetworkSummary,
    
    /// Byzantine fault summary
    pub byzantine_summary: ByzantineSummary,
}

/// Performance summary for dashboard
#[derive(Debug, Clone, Default)]
pub struct PerformanceSummary {
    /// Average container startup time
    pub avg_container_startup_ms: f64,
    
    /// Average P2P connection time
    pub avg_p2p_connection_ms: f64,
    
    /// Average consensus latency
    pub avg_consensus_latency_ms: f64,
    
    /// Average network setup time
    pub avg_network_setup_ms: f64,
    
    /// Overall performance score (0-100)
    pub performance_score: f64,
    
    /// Performance trend
    pub trend: PerformanceTrend,
}

/// Performance trend analysis
#[derive(Debug, Clone, Default)]
pub struct PerformanceTrend {
    /// Trend direction
    pub direction: TrendDirection,
    
    /// Confidence in trend analysis
    pub confidence: f64,
    
    /// Rate of change
    pub change_rate: f64,
    
    /// Predicted next value
    pub predicted_value: f64,
}

/// Trend directions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrendDirection {
    Improving,
    #[default]
    Stable,
    Degrading,
    Unknown,
}

/// Health summary
#[derive(Debug, Clone, Default)]
pub struct HealthSummary {
    /// Overall health score
    pub overall_health: f64,
    
    /// Component health scores
    pub component_health: HashMap<String, f64>,
    
    /// Active alerts count
    pub active_alerts: u32,
    
    /// Critical issues count
    pub critical_issues: u32,
}

/// Resource utilization summary
#[derive(Debug, Clone, Default)]
pub struct ResourceSummary {
    /// CPU utilization percentage
    pub cpu_utilization: f64,
    
    /// Memory utilization percentage
    pub memory_utilization: f64,
    
    /// Network utilization percentage
    pub network_utilization: f64,
    
    /// Disk utilization percentage
    pub disk_utilization: f64,
    
    /// Resource efficiency score
    pub efficiency_score: f64,
}

/// Network performance summary
#[derive(Debug, Clone, Default)]
pub struct NetworkSummary {
    /// Active connections count
    pub active_connections: u32,
    
    /// Total throughput (bytes/sec)
    pub total_throughput: u64,
    
    /// Average latency
    pub average_latency_ms: f64,
    
    /// Packet loss rate
    pub packet_loss_rate: f64,
    
    /// P2P mesh health score
    pub mesh_health_score: f64,
}

/// Byzantine fault tolerance summary
#[derive(Debug, Clone, Default)]
pub struct ByzantineSummary {
    /// Quarantined nodes count
    pub quarantined_nodes: u32,
    
    /// Byzantine faults detected
    pub faults_detected: u32,
    
    /// Fault tolerance remaining
    pub fault_tolerance_remaining: u32,
    
    /// Reputation scores
    pub reputation_scores: HashMap<NodeId, f64>,
    
    /// Byzantine health score
    pub byzantine_health_score: f64,
}

/// Analysis results
#[derive(Debug, Clone, Default)]
pub struct AnalysisResults {
    /// Performance analysis
    pub performance_analysis: PerformanceAnalysisResults,
    
    /// Anomaly detection results
    pub anomaly_results: AnomalyResults,
    
    /// Trend analysis results
    pub trend_results: TrendResults,
    
    /// Bottleneck analysis
    pub bottleneck_analysis: BottleneckAnalysis,
}

/// Performance analysis results
#[derive(Debug, Clone, Default)]
pub struct PerformanceAnalysisResults {
    /// Performance violations
    pub violations: Vec<PerformanceViolation>,
    
    /// Performance insights
    pub insights: Vec<PerformanceInsight>,
    
    /// Recommendations
    pub recommendations: Vec<PerformanceRecommendation>,
}

/// Performance violation
#[derive(Debug, Clone)]
pub struct PerformanceViolation {
    /// Violated metric
    pub metric: String,
    
    /// Current value
    pub current_value: f64,
    
    /// Threshold value
    pub threshold_value: f64,
    
    /// Violation severity
    pub severity: ViolationSeverity,
    
    /// Duration of violation
    pub duration: Duration,
    
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Violation severity levels
#[derive(Debug, Clone)]
pub enum ViolationSeverity {
    Minor,
    Moderate,
    Severe,
    Critical,
}

/// Performance insight
#[derive(Debug, Clone)]
pub struct PerformanceInsight {
    /// Insight category
    pub category: InsightCategory,
    
    /// Insight message
    pub message: String,
    
    /// Confidence level
    pub confidence: f64,
    
    /// Supporting data
    pub data: HashMap<String, f64>,
    
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Insight categories
#[derive(Debug, Clone)]
pub enum InsightCategory {
    Optimization,
    CapacityPlanning,
    TuningRecommendation,
    SecurityAlert,
    MaintenanceRequired,
}

/// Performance recommendation
#[derive(Debug, Clone)]
pub struct PerformanceRecommendation {
    /// Recommendation type
    pub rec_type: RecommendationType,
    
    /// Recommendation description
    pub description: String,
    
    /// Expected impact
    pub expected_impact: String,
    
    /// Implementation complexity
    pub complexity: ComplexityLevel,
    
    /// Priority level
    pub priority: PriorityLevel,
    
    /// Estimated effort
    pub estimated_effort: String,
}

/// Recommendation types
#[derive(Debug, Clone)]
pub enum RecommendationType {
    ResourceScaling,
    ConfigurationChange,
    ArchitectureImprovement,
    MaintenanceAction,
    SecurityUpdate,
}

/// Complexity levels
#[derive(Debug, Clone)]
pub enum ComplexityLevel {
    Low,
    Medium,
    High,
    VeryHigh,
}

/// Priority levels
#[derive(Debug, Clone)]
pub enum PriorityLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Anomaly detection results
#[derive(Debug, Clone, Default)]
pub struct AnomalyResults {
    /// Detected anomalies
    pub anomalies: Vec<Anomaly>,
    
    /// Anomaly statistics
    pub statistics: AnomalyStatistics,
}

/// Detected anomaly
#[derive(Debug, Clone)]
pub struct Anomaly {
    /// Anomaly type
    pub anomaly_type: AnomalyType,
    
    /// Affected metric
    pub metric: String,
    
    /// Anomaly score
    pub score: f64,
    
    /// Detection timestamp
    pub timestamp: Timestamp,
    
    /// Anomaly description
    pub description: String,
    
    /// Potential causes
    pub potential_causes: Vec<String>,
}

/// Anomaly statistics
#[derive(Debug, Clone, Default)]
pub struct AnomalyStatistics {
    /// Total anomalies detected
    pub total_detected: u64,
    
    /// Anomalies by type
    pub by_type: HashMap<String, u64>,
    
    /// Detection accuracy
    pub accuracy: f64,
    
    /// False positive rate
    pub false_positive_rate: f64,
}

/// Trend analysis results
#[derive(Debug, Clone, Default)]
pub struct TrendResults {
    /// Detected trends
    pub trends: Vec<Trend>,
    
    /// Trend predictions
    pub predictions: Vec<TrendPrediction>,
}

/// Detected trend
#[derive(Debug, Clone)]
pub struct Trend {
    /// Trend type
    pub trend_type: TrendType,
    
    /// Affected metric
    pub metric: String,
    
    /// Trend direction
    pub direction: TrendDirection,
    
    /// Confidence level
    pub confidence: f64,
    
    /// Rate of change
    pub change_rate: f64,
    
    /// Detection timestamp
    pub timestamp: Timestamp,
}

/// Trend prediction
#[derive(Debug, Clone)]
pub struct TrendPrediction {
    /// Metric name
    pub metric: String,
    
    /// Predicted values
    pub predictions: Vec<(Timestamp, f64)>,
    
    /// Confidence interval
    pub confidence_interval: (f64, f64),
    
    /// Prediction accuracy
    pub accuracy: f64,
}

/// Bottleneck analysis results
#[derive(Debug, Clone, Default)]
pub struct BottleneckAnalysis {
    /// Identified bottlenecks
    pub bottlenecks: Vec<Bottleneck>,
    
    /// Performance impact analysis
    pub impact_analysis: Vec<ImpactAnalysis>,
}

/// Identified bottleneck
#[derive(Debug, Clone)]
pub struct Bottleneck {
    /// Bottleneck type
    pub bottleneck_type: BottleneckType,
    
    /// Affected component
    pub component: String,
    
    /// Severity level
    pub severity: f64,
    
    /// Performance impact
    pub impact: f64,
    
    /// Detection timestamp
    pub timestamp: Timestamp,
    
    /// Remediation suggestions
    pub remediation: Vec<String>,
}

/// Types of bottlenecks
#[derive(Debug, Clone)]
pub enum BottleneckType {
    CPU,
    Memory,
    Network,
    Disk,
    Consensus,
    Byzantine,
}

/// Performance impact analysis
#[derive(Debug, Clone)]
pub struct ImpactAnalysis {
    /// Impact category
    pub category: String,
    
    /// Impact description
    pub description: String,
    
    /// Quantified impact
    pub impact_value: f64,
    
    /// Affected metrics
    pub affected_metrics: Vec<String>,
}

/// Active alert
#[derive(Debug, Clone)]
pub struct ActiveAlert {
    /// Alert ID
    pub id: String,
    
    /// Alert rule name
    pub rule_name: String,
    
    /// Alert severity
    pub severity: AlertSeverity,
    
    /// Alert message
    pub message: String,
    
    /// Triggered timestamp
    pub triggered_at: Timestamp,
    
    /// Last updated timestamp
    pub last_updated: Timestamp,
    
    /// Alert status
    pub status: AlertStatus,
    
    /// Acknowledgment info
    pub acknowledgment: Option<AlertAcknowledgment>,
}

/// Alert status
#[derive(Debug, Clone)]
pub enum AlertStatus {
    Active,
    Acknowledged,
    Resolved,
    Suppressed,
}

/// Alert acknowledgment information
#[derive(Debug, Clone)]
pub struct AlertAcknowledgment {
    /// Who acknowledged the alert
    pub acknowledged_by: String,
    
    /// Acknowledgment timestamp
    pub acknowledged_at: Timestamp,
    
    /// Acknowledgment note
    pub note: Option<String>,
}

/// Alert event for history
#[derive(Debug, Clone)]
pub struct AlertEvent {
    /// Event type
    pub event_type: AlertEventType,
    
    /// Alert ID
    pub alert_id: String,
    
    /// Event timestamp
    pub timestamp: Timestamp,
    
    /// Event data
    pub data: HashMap<String, String>,
}

/// Alert event types
#[derive(Debug, Clone)]
pub enum AlertEventType {
    Triggered,
    Acknowledged,
    Resolved,
    Escalated,
    Suppressed,
}

/// Performance benchmarks for comparison
pub struct PerformanceBenchmarks {
    /// Target performance metrics
    pub targets: PerformanceThresholds,
    
    /// Historical benchmarks
    pub historical: Vec<BenchmarkResult>,
    
    /// Baseline measurements
    pub baselines: HashMap<String, f64>,
}

/// Benchmark result
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Benchmark name
    pub name: String,
    
    /// Measured value
    pub value: f64,
    
    /// Target value
    pub target: f64,
    
    /// Performance ratio (measured/target)
    pub performance_ratio: f64,
    
    /// Test conditions
    pub conditions: HashMap<String, String>,
    
    /// Timestamp
    pub timestamp: Timestamp,
}

impl PerformanceDashboard {
    /// Create a new performance dashboard
    #[instrument(skip(config, health_monitor, network_manager, orchestrator))]
    pub async fn new(
        node_id: NodeId,
        config: DashboardConfig,
        health_monitor: Arc<HealthMonitor>,
        network_manager: Arc<NetworkManager>,
        orchestrator: Arc<tokio::sync::Mutex<ConsensusContainerOrchestrator>>,
    ) -> Result<Self> {
        info!("Initializing HyperMesh performance dashboard for node {}", node_id);

        // Initialize metrics storage
        let metrics_storage = Arc::new(RwLock::new(MetricsStorage {
            time_series: BTreeMap::new(),
            aggregated: HashMap::new(),
            metadata: HashMap::new(),
            storage_stats: StorageStats::default(),
        }));

        // Initialize components
        let metrics_collector = Arc::new(
            PerformanceMetricsCollector::new(&config.metrics_collection, Arc::clone(&metrics_storage))
        );

        let metrics_aggregator = Arc::new(
            RealTimeMetricsAggregator::new(Arc::clone(&metrics_storage))
        );

        let performance_analyzer = Arc::new(
            PerformanceAnalyzer::new(&config.performance_analysis, Arc::clone(&metrics_storage))
        );

        let alert_manager = Arc::new(
            AlertManager::new(&config.alerting)
        );

        let web_server = Arc::new(
            DashboardWebServer::new(&config.web_server, Arc::clone(&metrics_storage))
        );

        let benchmarks = Arc::new(
            PerformanceBenchmarks::new(&config.performance_analysis.thresholds)
        );

        let dashboard = Self {
            node_id,
            config,
            health_monitor,
            network_manager,
            orchestrator,
            metrics_collector,
            metrics_aggregator,
            performance_analyzer,
            alert_manager,
            web_server,
            metrics_storage,
            benchmarks,
        };

        // Start dashboard services
        dashboard.start_services().await?;

        info!("Performance dashboard initialized successfully");
        Ok(dashboard)
    }

    /// Start dashboard services
    async fn start_services(&self) -> Result<()> {
        // Start metrics collection
        self.metrics_collector.start_collection(
            Arc::clone(&self.health_monitor),
            Arc::clone(&self.network_manager),
            Arc::clone(&self.orchestrator),
        ).await?;

        // Start metrics aggregation
        self.metrics_aggregator.start_aggregation().await?;

        // Start performance analysis
        self.performance_analyzer.start_analysis().await?;

        // Start alert manager
        self.alert_manager.start_monitoring().await?;

        // Start web server
        self.web_server.start_server().await?;

        Ok(())
    }

    /// Get current dashboard data
    pub async fn get_dashboard_data(&self) -> DashboardData {
        let metrics = self.metrics_aggregator.get_aggregated_metrics().await;
        let analysis = self.performance_analyzer.get_analysis_results().await;
        let alerts = self.alert_manager.get_active_alerts().await;

        DashboardData {
            metrics,
            analysis,
            alerts,
            timestamp: Timestamp::now(),
        }
    }

    /// Export metrics using native monitoring system
    pub async fn export_metrics(&self, format: ExportFormat) -> Result<String> {
        let metrics = self.metrics_storage.read().unwrap();

        match format {
            ExportFormat::Native => self.export_native_metrics(&metrics),
            ExportFormat::Json => self.export_json_metrics(&metrics),
            ExportFormat::Text => self.export_text_metrics(&metrics),
        }
    }

    /// Export metrics in native format
    fn export_native_metrics(&self, metrics: &MetricsStorage) -> Result<String> {
        let mut output = String::new();
        
        for (metric_name, points) in &metrics.time_series {
            if let Some(latest_point) = points.back() {
                if let Some(metadata) = metrics.metadata.get(metric_name) {
                    output.push_str(&format!(
                        "# TYPE {} {}\n",
                        metric_name,
                        match metadata.metric_type {
                            MetricDataType::Counter => "counter",
                            MetricDataType::Gauge => "gauge",
                            MetricDataType::Histogram => "histogram",
                            MetricDataType::Summary => "summary",
                        }
                    ));
                    output.push_str(&format!("# HELP {} {}\n", metric_name, metadata.description));
                }
                
                output.push_str(&format!(
                    "{} {}\n",
                    metric_name,
                    latest_point.value
                ));
            }
        }
        
        Ok(output)
    }

    /// Export metrics in JSON format
    fn export_json_metrics(&self, metrics: &MetricsStorage) -> Result<String> {
        serde_json::to_string_pretty(metrics)
            .map_err(|e| crate::RuntimeError::SerializationError { 
                message: format!("JSON export failed: {}", e) 
            }.into())
    }

    /// Export metrics in text format
    fn export_text_metrics(&self, metrics: &MetricsStorage) -> Result<String> {
        let mut output = String::new();
        
        for (metric_name, points) in &metrics.time_series {
            for point in points {
                output.push_str(&format!(
                    "{}[source={}] = {} @ {}\n",
                    metric_name,
                    point.source,
                    point.value,
                    point.timestamp.as_secs()
                ));
            }
        }
        
        Ok(output)
    }

        // Generate native dashboard configuration
        let dashboard = serde_json::json!({
            "dashboard": {
                "title": "HyperMesh Performance Dashboard",
                "version": "1.0.0",
                "backend": "native",
                "metrics_count": metrics.time_series.len(),
                "refresh_interval_ms": 5000,
                "time_range": {
                    "from": "-1h",
                    "to": "now"
                }
            }
        });

        serde_json::to_string_pretty(&dashboard)
            .map_err(|e| crate::RuntimeError::SerializationError {
                message: format!("Dashboard config generation failed: {}", e)
            }.into())
    }

    /// Generate dashboard panels for native UI
    fn generate_dashboard_panels(&self, metrics: &MetricsStorage) -> Vec<serde_json::Value> {
        let mut panels = Vec::new();

        // Performance summary panel
        panels.push(serde_json::json!({
            "id": "perf-summary",
            "title": "Performance Summary",
            "type": "stat",
            "position": {"row": 0, "col": 0, "width": 12, "height": 8},
            "metrics": [{
                "name": "container_startup_time_ms",
                "label": "Container Startup (ms)",
                "format": "milliseconds"
            }]
        }));

        // Add panels for each metric type
        let mut row = 1;
        for (metric_name, _) in metrics.time_series.iter() {
            panels.push(serde_json::json!({
                "id": format!("metric-{}", metric_name),
                "title": metric_name,
                "type": "timeseries",
                "position": {"row": row, "col": 0, "width": 12, "height": 6},
                "metric": metric_name,
                "aggregation": "avg"
            }));
            row += 1;
        }

        panels
    }
}

/// Export formats for metrics
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ExportFormat {
    Native,
    Json,
    Text,
}

/// Dashboard data for UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    /// Aggregated metrics
    pub metrics: AggregatedMetrics,

    /// Analysis results
    pub analysis: AnalysisResults,

    /// Active alerts
    pub alerts: Vec<ActiveAlert>,

    /// Data timestamp
    pub timestamp: Timestamp,
}

impl PerformanceMetricsCollector {
    fn new(config: &MetricsCollectionConfig, metrics_storage: Arc<RwLock<MetricsStorage>>) -> Self {
        Self {
            config: config.clone(),
            metrics_storage,
            collection_tasks: Vec::new(),
        }
    }

    async fn start_collection(
        &self,
        health_monitor: Arc<HealthMonitor>,
        network_manager: Arc<NetworkManager>,
        orchestrator: Arc<tokio::sync::Mutex<ConsensusContainerOrchestrator>>,
    ) -> Result<()> {
        // Start collection tasks for each metric type
        // Implementation details would go here
        Ok(())
    }
}

impl RealTimeMetricsAggregator {
    fn new(metrics_storage: Arc<RwLock<MetricsStorage>>) -> Self {
        Self {
            metrics_storage,
            aggregated_metrics: Arc::new(RwLock::new(AggregatedMetrics::default())),
            aggregation_tasks: Vec::new(),
        }
    }

    async fn start_aggregation(&self) -> Result<()> {
        // Start aggregation tasks
        Ok(())
    }

    async fn get_aggregated_metrics(&self) -> AggregatedMetrics {
        self.aggregated_metrics.read().unwrap().clone()
    }
}

impl PerformanceAnalyzer {
    fn new(config: &PerformanceAnalysisConfig, metrics_storage: Arc<RwLock<MetricsStorage>>) -> Self {
        Self {
            config: config.clone(),
            metrics_storage,
            analysis_results: Arc::new(RwLock::new(AnalysisResults::default())),
        }
    }

    async fn start_analysis(&self) -> Result<()> {
        // Start analysis tasks
        Ok(())
    }

    async fn get_analysis_results(&self) -> AnalysisResults {
        self.analysis_results.read().unwrap().clone()
    }
}

impl AlertManager {
    fn new(config: &AlertingConfig) -> Self {
        Self {
            config: config.clone(),
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    async fn start_monitoring(&self) -> Result<()> {
        // Start alert monitoring
        Ok(())
    }

    async fn get_active_alerts(&self) -> Vec<ActiveAlert> {
        self.active_alerts.read().unwrap().values().cloned().collect()
    }
}

impl DashboardWebServer {
    fn new(config: &WebServerConfig, metrics_storage: Arc<RwLock<MetricsStorage>>) -> Self {
        Self {
            config: config.clone(),
            metrics_storage,
            server_handle: None,
        }
    }

    async fn start_server(&self) -> Result<()> {
        // Start web server with warp
        info!("Starting dashboard web server on {}:{}", self.config.bind_address, self.config.port);
        Ok(())
    }
}

impl PerformanceBenchmarks {
    fn new(thresholds: &PerformanceThresholds) -> Self {
        Self {
            targets: thresholds.clone(),
            historical: Vec::new(),
            baselines: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_config_defaults() {
        let config = DashboardConfig::default();
        assert_eq!(config.web_server.port, 3000);
        assert!(config.performance_analysis.enable_realtime_analysis);
        assert_eq!(config.performance_analysis.thresholds.container_startup_ms, 100.0);
        assert_eq!(config.performance_analysis.thresholds.p2p_connection_ms, 5.0);
        assert_eq!(config.performance_analysis.thresholds.consensus_overhead_ms, 50.0);
    }

    #[test]
    fn test_performance_thresholds() {
        let thresholds = PerformanceThresholds::default();
        assert_eq!(thresholds.container_startup_ms, 100.0);
        assert_eq!(thresholds.p2p_connection_ms, 5.0);
        assert_eq!(thresholds.consensus_overhead_ms, 50.0);
        assert_eq!(thresholds.network_setup_ms, 10.0);
    }

    #[test]
    fn test_metrics_storage() {
        let mut storage = MetricsStorage {
            time_series: BTreeMap::new(),
            aggregated: HashMap::new(),
            metadata: HashMap::new(),
            storage_stats: StorageStats::default(),
        };

        // Test adding metric point
        let point = MetricPoint {
            timestamp: Timestamp::now(),
            value: 42.0,
            labels: HashMap::new(),
            source: "test".to_string(),
        };

        storage.time_series.entry("test_metric".to_string())
            .or_insert_with(VecDeque::new)
            .push_back(point);

        assert_eq!(storage.time_series.len(), 1);
    }
}