//! Health monitoring configuration types
//!
//! This module contains all configuration types for the health monitoring system,
//! extracted from the main health.rs for better organization and maintainability.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;


/// Comprehensive health monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    /// Health check interval
    pub check_interval: Duration,
    
    /// Byzantine fault detection thresholds
    pub byzantine_thresholds: ByzantineHealthThresholds,
    
    /// Container health check settings
    pub container_health: ContainerHealthConfig,
    
    /// Network health monitoring settings
    pub network_health: NetworkHealthConfig,
    
    /// Performance degradation detection settings
    pub degradation_detection: DegradationConfig,
    
    /// Automated recovery settings
    pub recovery_config: RecoveryConfig,
    
    /// Health metrics retention settings
    pub metrics_retention: MetricsRetentionConfig,
    
    /// Cluster coordination settings
    pub cluster_coordination: ClusterCoordinationConfig,
}

/// Byzantine fault tolerance health thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByzantineHealthThresholds {
    /// Maximum tolerable reputation score drop before quarantine
    pub reputation_threshold: f64,
    
    /// Maximum consensus timeout rate before node isolation
    pub consensus_timeout_threshold: f64,
    
    /// Maximum network anomaly score before investigation
    pub network_anomaly_threshold: f64,
    
    /// Minimum cluster size to maintain Byzantine fault tolerance
    pub min_cluster_size: usize,
    
    /// Maximum Byzantine faults before cluster degradation alert
    pub max_byzantine_faults: usize,
}

/// Container health monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerHealthConfig {
    /// Enable container health monitoring
    pub enabled: bool,
    
    /// Health check timeout
    pub timeout: Duration,
    
    /// Number of consecutive failures before marking unhealthy
    pub failure_threshold: u32,
    
    /// Number of consecutive successes before marking healthy
    pub success_threshold: u32,
    
    /// Resource usage thresholds
    pub resource_thresholds: ResourceThresholds,
    
    /// Container restart policy on health failure
    pub restart_policy: RestartPolicy,
}

/// Resource usage thresholds for health assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceThresholds {
    /// Maximum CPU usage percentage before alert
    pub max_cpu_percent: f64,
    
    /// Maximum memory usage percentage before alert
    pub max_memory_percent: f64,
    
    /// Maximum disk usage percentage before alert
    pub max_disk_percent: f64,
    
    /// Maximum network bandwidth utilization before alert
    pub max_network_percent: f64,
    
    /// Minimum available file descriptors before alert
    pub min_available_fd: u64,
}

/// Container restart policy on health failures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RestartPolicy {
    /// Never restart on health failure
    Never,
    
    /// Always restart on health failure
    Always,
    
    /// Restart only if exit code indicates failure
    OnFailure,
    
    /// Restart with exponential backoff and maximum attempts
    Backoff { max_attempts: u32, max_delay: Duration },
}

/// Network health monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkHealthConfig {
    /// Enable network health monitoring
    pub enabled: bool,
    
    /// Connection health check interval
    pub check_interval: Duration,
    
    /// Maximum acceptable latency for health checks
    pub max_latency: Duration,
    
    /// Maximum acceptable packet loss percentage
    pub max_packet_loss: f64,
    
    /// Minimum bandwidth for healthy connection
    pub min_bandwidth: u64,
    
    /// Network isolation threshold
    pub isolation_threshold: Duration,
}

/// Performance degradation detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradationConfig {
    /// Enable degradation detection
    pub enabled: bool,
    
    /// Window size for performance analysis
    pub analysis_window: Duration,
    
    /// Minimum degradation percentage to trigger alert
    pub min_degradation_percent: f64,
    
    /// Metrics to monitor for degradation
    pub monitored_metrics: Vec<MonitoredMetric>,
    
    /// Automatic remediation threshold
    pub auto_remediation_threshold: f64,
}

/// Metric to monitor for performance degradation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoredMetric {
    /// Metric name
    pub name: String,
    
    /// Metric type
    pub metric_type: MetricType,
    
    /// Acceptable degradation percentage
    pub degradation_threshold: f64,
    
    /// Weight in overall health score
    pub weight: f64,
}

/// Type of performance metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricType {
    /// Response time metrics (lower is better)
    ResponseTime,
    
    /// Throughput metrics (higher is better)  
    Throughput,
    
    /// Error rate metrics (lower is better)
    ErrorRate,
    
    /// Resource utilization metrics (depends on context)
    ResourceUtilization,
    
    /// Consensus performance metrics
    ConsensusMetrics,
}

/// Automated recovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryConfig {
    /// Enable automated recovery
    pub enabled: bool,
    
    /// Maximum concurrent recovery procedures
    pub max_concurrent_recoveries: usize,
    
    /// Timeout for recovery procedures
    pub recovery_timeout: Duration,
    
    /// Retry configuration for recovery attempts
    pub retry_config: RetryConfig,
    
    /// Escalation thresholds
    pub escalation_thresholds: EscalationConfig,
}

/// Recovery retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts
    pub max_retries: u32,
    
    /// Base delay between retries
    pub base_delay: Duration,
    
    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,
    
    /// Maximum delay between retries
    pub max_delay: Duration,
}

/// Recovery escalation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationConfig {
    /// Escalation levels and thresholds
    pub levels: Vec<EscalationLevel>,
    
    /// Automatic escalation timeout
    pub auto_escalation_timeout: Duration,
    
    /// Manual intervention required threshold
    pub manual_intervention_threshold: u32,
}

/// Recovery escalation level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationLevel {
    /// Level name
    pub name: String,
    
    /// Actions to take at this level
    pub actions: Vec<RecoveryAction>,
    
    /// Threshold for this escalation level
    pub threshold: f64,
    
    /// Timeout before escalating to next level
    pub timeout: Duration,
}

/// Automated recovery action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryAction {
    /// Restart container
    RestartContainer { container_id: crate::ResourceId },
    
    /// Restart node service
    RestartNodeService { service_name: String },
    
    /// Quarantine Byzantine node
    QuarantineNode { node_id: nexus_shared::NodeId },
    
    /// Rebalance cluster load
    RebalanceCluster,
    
    /// Reset network connections
    ResetNetworkConnections,
    
    /// Perform garbage collection
    PerformGarbageCollection,
    
    /// Scale resources
    ScaleResources { scaling_factor: f64 },
    
    /// Send alert to operators
    SendAlert { severity: AlertSeverity, message: String },
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Metrics retention configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsRetentionConfig {
    /// Maximum number of health snapshots to retain
    pub max_snapshots: usize,
    
    /// Maximum age of retained metrics
    pub max_age: Duration,
    
    /// Compression settings for historical data
    pub compression: CompressionConfig,
    
    /// Export settings for external monitoring systems
    pub export_config: Option<ExportConfig>,
}

/// Compression configuration for metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Enable compression
    pub enabled: bool,
    
    /// Compression algorithm
    pub algorithm: CompressionAlgorithm,
    
    /// Compression level (1-9)
    pub level: u8,
}

/// Compression algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    Gzip,
    Zstd,
    Lz4,
}

/// Export configuration for native monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    /// Export format
    pub format: ExportFormat,

    /// Export destination (file path or endpoint)
    pub destination: String,

    /// Export interval
    pub interval: Duration,

    /// Authentication configuration (if needed for remote export)
    pub auth: Option<AuthConfig>,
}

/// Export format options for native monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    Native,
    Json,
    Text,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Authentication type
    pub auth_type: AuthType,
    
    /// Credentials or configuration
    pub credentials: HashMap<String, String>,
}

/// Authentication types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthType {
    Basic,
    Bearer,
    ApiKey,
    OAuth2,
    Certificate,
}

/// Cluster coordination configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterCoordinationConfig {
    /// Enable cluster health coordination
    pub enabled: bool,
    
    /// Health synchronization interval
    pub sync_interval: Duration,
    
    /// Cluster health consensus threshold
    pub consensus_threshold: f64,
    
    /// Maximum time to wait for cluster health consensus
    pub consensus_timeout: Duration,
    
    /// Leader election settings for health coordination
    pub leader_election: LeaderElectionConfig,
}

/// Leader election configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderElectionConfig {
    /// Enable leader election for health coordination
    pub enabled: bool,
    
    /// Lease duration for health leader
    pub lease_duration: Duration,
    
    /// Renewal interval for leader lease
    pub renewal_interval: Duration,
    
    /// Retry timeout for leader election
    pub retry_timeout: Duration,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(30),
            byzantine_thresholds: ByzantineHealthThresholds {
                reputation_threshold: 0.7,
                consensus_timeout_threshold: 0.1,
                network_anomaly_threshold: 0.8,
                min_cluster_size: 4,
                max_byzantine_faults: 1,
            },
            container_health: ContainerHealthConfig {
                enabled: true,
                timeout: Duration::from_secs(10),
                failure_threshold: 3,
                success_threshold: 2,
                resource_thresholds: ResourceThresholds {
                    max_cpu_percent: 90.0,
                    max_memory_percent: 90.0,
                    max_disk_percent: 85.0,
                    max_network_percent: 80.0,
                    min_available_fd: 100,
                },
                restart_policy: RestartPolicy::Backoff {
                    max_attempts: 5,
                    max_delay: Duration::from_secs(300),
                },
            },
            network_health: NetworkHealthConfig {
                enabled: true,
                check_interval: Duration::from_secs(15),
                max_latency: Duration::from_millis(100),
                max_packet_loss: 0.05,
                min_bandwidth: 1_000_000, // 1 MB/s
                isolation_threshold: Duration::from_secs(60),
            },
            degradation_detection: DegradationConfig {
                enabled: true,
                analysis_window: Duration::from_secs(300),
                min_degradation_percent: 15.0,
                monitored_metrics: vec![
                    MonitoredMetric {
                        name: "consensus_latency".to_string(),
                        metric_type: MetricType::ResponseTime,
                        degradation_threshold: 20.0,
                        weight: 0.3,
                    },
                    MonitoredMetric {
                        name: "container_startup_time".to_string(),
                        metric_type: MetricType::ResponseTime,
                        degradation_threshold: 25.0,
                        weight: 0.2,
                    },
                    MonitoredMetric {
                        name: "network_throughput".to_string(),
                        metric_type: MetricType::Throughput,
                        degradation_threshold: 30.0,
                        weight: 0.25,
                    },
                    MonitoredMetric {
                        name: "error_rate".to_string(),
                        metric_type: MetricType::ErrorRate,
                        degradation_threshold: 10.0,
                        weight: 0.25,
                    },
                ],
                auto_remediation_threshold: 30.0,
            },
            recovery_config: RecoveryConfig {
                enabled: true,
                max_concurrent_recoveries: 3,
                recovery_timeout: Duration::from_secs(300),
                retry_config: RetryConfig {
                    max_retries: 3,
                    base_delay: Duration::from_secs(5),
                    backoff_multiplier: 2.0,
                    max_delay: Duration::from_secs(60),
                },
                escalation_thresholds: EscalationConfig {
                    levels: vec![
                        EscalationLevel {
                            name: "Level1".to_string(),
                            actions: vec![RecoveryAction::RestartNodeService { 
                                service_name: "container-runtime".to_string() 
                            }],
                            threshold: 0.2,
                            timeout: Duration::from_secs(120),
                        },
                        EscalationLevel {
                            name: "Level2".to_string(),
                            actions: vec![RecoveryAction::RebalanceCluster],
                            threshold: 0.5,
                            timeout: Duration::from_secs(300),
                        },
                        EscalationLevel {
                            name: "Level3".to_string(),
                            actions: vec![RecoveryAction::SendAlert { 
                                severity: AlertSeverity::Critical,
                                message: "Manual intervention required".to_string(),
                            }],
                            threshold: 0.8,
                            timeout: Duration::from_secs(600),
                        },
                    ],
                    auto_escalation_timeout: Duration::from_secs(600),
                    manual_intervention_threshold: 3,
                },
            },
            metrics_retention: MetricsRetentionConfig {
                max_snapshots: 1000,
                max_age: Duration::from_secs(86400 * 7), // 1 week
                compression: CompressionConfig {
                    enabled: true,
                    algorithm: CompressionAlgorithm::Zstd,
                    level: 3,
                },
                export_config: None,
            },
            cluster_coordination: ClusterCoordinationConfig {
                enabled: true,
                sync_interval: Duration::from_secs(60),
                consensus_threshold: 0.66,
                consensus_timeout: Duration::from_secs(30),
                leader_election: LeaderElectionConfig {
                    enabled: true,
                    lease_duration: Duration::from_secs(30),
                    renewal_interval: Duration::from_secs(10),
                    retry_timeout: Duration::from_secs(5),
                },
            },
        }
    }
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertSeverity::Info => write!(f, "INFO"),
            AlertSeverity::Warning => write!(f, "WARNING"),
            AlertSeverity::Error => write!(f, "ERROR"),
            AlertSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}