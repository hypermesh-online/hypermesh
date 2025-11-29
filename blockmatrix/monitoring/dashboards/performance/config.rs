//! Dashboard configuration types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    pub web_server: WebServerConfig,
    pub metrics_collection: MetricsCollectionConfig,
    pub performance_analysis: PerformanceAnalysisConfig,
    pub alerting: AlertingConfig,
    pub ui_settings: UISettings,
    pub storage: StorageConfig,
}

/// Web server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebServerConfig {
    pub bind_address: String,
    pub port: u16,
    pub enable_tls: bool,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
    pub enable_auth: bool,
    pub auth_config: Option<AuthConfig>,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub method: AuthMethod,
    pub basic_auth_users: Option<HashMap<String, String>>,
    pub jwt_secret: Option<String>,
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
    pub interval: Duration,
    pub enabled_metrics: Vec<MetricType>,
    pub high_frequency_interval: Duration,
    pub high_frequency_metrics: Vec<MetricType>,
    pub retention_policy: RetentionPolicy,
}

/// Performance analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAnalysisConfig {
    pub enable_realtime_analysis: bool,
    pub analysis_window: Duration,
    pub thresholds: PerformanceThresholds,
    pub trend_analysis: TrendAnalysisConfig,
    pub anomaly_detection: AnomalyDetectionConfig,
}

/// Performance thresholds for HyperMesh targets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThresholds {
    pub container_startup_ms: f64,
    pub p2p_connection_ms: f64,
    pub consensus_overhead_ms: f64,
    pub network_setup_ms: f64,
    pub memory_usage_percent: f64,
    pub cpu_usage_percent: f64,
    pub network_bandwidth_percent: f64,
    pub error_rate_percent: f64,
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
    pub enabled: bool,
    pub detection_window: Duration,
    pub min_confidence: f64,
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
    pub enabled: bool,
    pub statistical_threshold: f64,
    pub ml_model_config: Option<MLModelConfig>,
    pub anomaly_types: Vec<AnomalyType>,
}

/// Machine learning model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLModelConfig {
    pub model_type: MLModelType,
    pub training_data_size: usize,
    pub update_frequency: Duration,
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
    pub enabled: bool,
    pub channels: Vec<AlertChannel>,
    pub rules: Vec<AlertRule>,
    pub aggregation: AlertAggregationConfig,
}

/// Alert channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertChannel {
    pub name: String,
    pub channel_type: AlertChannelType,
    pub config: HashMap<String, String>,
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
    pub name: String,
    pub condition: AlertCondition,
    pub severity: AlertSeverity,
    pub message_template: String,
    pub channels: Vec<String>,
    pub evaluation_interval: Duration,
    pub suppression: Option<AlertSuppressionConfig>,
}

/// Alert condition specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertCondition {
    pub metric: String,
    pub operator: ComparisonOperator,
    pub threshold: f64,
    pub time_window: Duration,
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
    pub duration: Duration,
    pub conditions: Vec<String>,
}

/// Alert aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertAggregationConfig {
    pub enabled: bool,
    pub window: Duration,
    pub max_alerts_per_window: usize,
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
    pub title: String,
    pub refresh_interval_ms: u64,
    pub default_time_range: Duration,
    pub chart_types: Vec<ChartType>,
    pub theme: DashboardTheme,
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
    pub backend: StorageBackend,
    pub retention: RetentionPolicy,
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
    pub db_type: TimeSeriesDBType,
    pub connection: HashMap<String, String>,
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
    pub batch_size: usize,
    pub flush_interval: Duration,
    pub max_batch_age: Duration,
}

/// Retention policy for metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub high_resolution: Duration,
    pub medium_resolution: Duration,
    pub low_resolution: Duration,
    pub aggregation_rules: Vec<AggregationRule>,
}

/// Aggregation rule for downsampling metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationRule {
    pub from_resolution: Duration,
    pub to_resolution: Duration,
    pub function: AggregationFunction,
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
    pub enabled: bool,
    pub algorithm: CompressionAlgorithm,
    pub level: u8,
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

/// Types of metrics to collect
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricType {
    ContainerPerformance,
    NetworkPerformance,
    ConsensusMetrics,
    ResourceUtilization,
    ByzantineMetrics,
    ConsensusLatency,
    NetworkLatency,
    ContainerStartupTime,
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
                    high_resolution: Duration::from_secs(3600),
                    medium_resolution: Duration::from_secs(86400),
                    low_resolution: Duration::from_secs(604800),
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
                    statistical_threshold: 2.0,
                    ml_model_config: None,
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
