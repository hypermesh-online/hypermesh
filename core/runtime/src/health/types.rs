//! Health monitoring types and data structures
//!
//! This module contains all the shared types used across the health monitoring system.

use nexus_shared::{NodeId, ResourceId, Timestamp};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, SystemTime};

/// Overall system health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealthStatus {
    /// Node identifier
    pub node_id: NodeId,
    
    /// Overall health status
    pub overall_status: HealthStatus,
    
    /// Component health status map
    pub components: HashMap<String, ComponentHealth>,
    
    /// Last health update timestamp
    pub last_updated: Timestamp,
    
    /// Active alerts
    pub alerts: Vec<HealthAlert>,
}

impl SystemHealthStatus {
    /// Create new system health status
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            overall_status: HealthStatus::Unknown,
            components: HashMap::new(),
            last_updated: SystemTime::now().into(),
            alerts: Vec::new(),
        }
    }
}

/// Health status for individual components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component health score (0.0 - 1.0)
    pub score: f64,
    
    /// Component status
    pub status: HealthStatus,
    
    /// Detailed metrics for this component
    pub metrics: HashMap<String, f64>,
    
    /// Recent health events
    pub recent_events: Vec<ComponentHealthEvent>,
    
    /// Last health check timestamp
    pub last_check: Timestamp,
}

impl Default for ComponentHealth {
    fn default() -> Self {
        Self {
            score: 1.0,
            status: HealthStatus::Healthy,
            metrics: HashMap::new(),
            recent_events: Vec::new(),
            last_check: SystemTime::now().into(),
        }
    }
}

/// Health status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Component is healthy
    Healthy,
    
    /// Component has warnings but is functional
    Warning,
    
    /// Component has errors affecting functionality
    Degraded,
    
    /// Component is critical or non-functional
    Critical,
    
    /// Component status is unknown
    Unknown,
}

/// Health alert information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthAlert {
    /// Unique alert identifier
    pub id: String,
    
    /// Alert severity level
    pub severity: AlertSeverity,
    
    /// Human-readable alert message
    pub message: String,
    
    /// Component that triggered the alert
    pub component: String,
    
    /// Alert timestamp
    pub timestamp: Timestamp,
    
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Alert severity levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Info => write!(f, "INFO"),
            Self::Warning => write!(f, "WARNING"),
            Self::Error => write!(f, "ERROR"),
            Self::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Health snapshot for historical data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSnapshot {
    /// Snapshot timestamp
    pub timestamp: Timestamp,
    
    /// System health status at the time of snapshot
    pub system_status: SystemHealthStatus,
    
    /// Resource utilization metrics
    pub resource_utilization: ResourceUtilizationMetrics,
}

/// Health trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthTrend {
    /// Trend direction
    pub direction: TrendDirection,
    
    /// Trend confidence (0.0 - 1.0)
    pub confidence: f64,
    
    /// Trend duration
    pub duration: Duration,
    
    /// Trend slope (rate of change)
    pub slope: f64,
}

impl Default for HealthTrend {
    fn default() -> Self {
        Self {
            direction: TrendDirection::Stable,
            confidence: 0.0,
            duration: Duration::default(),
            slope: 0.0,
        }
    }
}

/// Trend direction enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendDirection {
    Improving,
    Stable,
    Degrading,
    Unknown,
}

/// Component health event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealthEvent {
    /// Event timestamp
    pub timestamp: Timestamp,
    
    /// Event type
    pub event_type: HealthEventType,
    
    /// Event description
    pub description: String,
    
    /// Event severity
    pub severity: AlertSeverity,
    
    /// Additional context
    pub context: HashMap<String, String>,
}

/// Health event type enumeration
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthEventType {
    Improvement,
    Degradation,
    Recovery,
    RecoveryFailed,
    Anomaly,
    ThresholdExceeded,
}

impl fmt::Debug for HealthEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Improvement => write!(f, "Improvement"),
            Self::Degradation => write!(f, "Degradation"),
            Self::Recovery => write!(f, "Recovery"),
            Self::RecoveryFailed => write!(f, "RecoveryFailed"),
            Self::Anomaly => write!(f, "Anomaly"),
            Self::ThresholdExceeded => write!(f, "ThresholdExceeded"),
        }
    }
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
    
    /// Performance degradation threshold percentage
    pub degradation_threshold_percent: f64,
    
    /// History window size for analysis
    pub history_window_size: usize,
    
    /// Baseline window size for comparison
    pub baseline_window_size: usize,
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
    pub timeout: Duration,
    
    /// Retry configuration for recovery attempts
    pub retry: RetryConfig,
    
    /// Escalation thresholds
    pub escalation_thresholds: EscalationConfig,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_concurrent_recoveries: 3,
            timeout: Duration::from_secs(300),
            retry: RetryConfig::default(),
            escalation_thresholds: EscalationConfig::default(),
        }
    }
}

/// Recovery retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts
    pub max_attempts: u32,
    
    /// Base delay between retries
    pub base_delay: Duration,
    
    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,
    
    /// Maximum delay between retries
    pub max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_secs(5),
            backoff_multiplier: 2.0,
            max_delay: Duration::from_secs(60),
        }
    }
}

/// Recovery escalation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationConfig {
    /// Escalation levels and thresholds
    pub levels: Vec<EscalationLevel>,
    
    /// History retention size
    pub history_retention_size: usize,
}

impl Default for EscalationConfig {
    fn default() -> Self {
        Self {
            levels: vec![
                EscalationLevel {
                    name: "Level1".to_string(),
                    actions: vec![RecoveryAction::RestartContainer],
                    threshold: 0.2,
                    timeout: Duration::from_secs(120),
                },
                EscalationLevel {
                    name: "Level2".to_string(),
                    actions: vec![RecoveryAction::ScaleUp],
                    threshold: 0.5,
                    timeout: Duration::from_secs(300),
                },
            ],
            history_retention_size: 1000,
        }
    }
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
    /// Stop container
    StopContainer,
    
    /// Start container
    StartContainer,
    
    /// Restart container
    RestartContainer,
    
    /// Stop service
    StopService,
    
    /// Start service
    StartService,
    
    /// Scale up resources
    ScaleUp,
    
    /// Scale down resources
    ScaleDown,
    
    /// Drain node
    DrainNode,
    
    /// Migrate workloads
    MigrateWorkloads,
    
    /// Isolate node
    IsolateNode,
    
    /// Clear caches
    ClearCaches,
    
    /// Reload configuration
    ReloadConfiguration,
    
    /// Rollback to last version
    RollbackToLastVersion,
    
    /// Verify health
    VerifyHealth,
    
    /// Send alert
    SendAlert,
    
    /// Escalate to next level
    Escalate,
}

/// Recovery type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryType {
    ContainerRestart,
    ServiceRestart,
    NodeFailover,
    AutoScale,
    SelfHealing,
    RollbackDeployment,
}

/// Recovery state enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryState {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Metrics retention configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsRetentionConfig {
    /// Maximum number of health snapshots to retain
    pub max_snapshots: usize,
    
    /// Maximum retention duration
    pub retention_duration: Duration,
    
    /// Compression settings for historical data
    pub compression: Option<CompressionConfig>,
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

/// Container health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerHealthMetrics {
    /// Container CPU usage
    pub cpu_usage: f64,
    
    /// Container memory usage
    pub memory_usage: f64,
    
    /// Disk I/O metrics
    pub disk_io: DiskIOMetrics,
    
    /// Network I/O metrics
    pub network_io: NetworkIOMetrics,
    
    /// Process count
    pub process_count: u32,
    
    /// File descriptor count
    pub fd_count: u32,
}

impl Default for ContainerHealthMetrics {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_usage: 0.0,
            disk_io: DiskIOMetrics::default(),
            network_io: NetworkIOMetrics::default(),
            process_count: 0,
            fd_count: 0,
        }
    }
}

/// Disk I/O metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskIOMetrics {
    /// Bytes read per second
    pub read_bytes_per_sec: u64,
    
    /// Bytes written per second
    pub write_bytes_per_sec: u64,
    
    /// Read operations per second
    pub read_ops_per_sec: u32,
    
    /// Write operations per second
    pub write_ops_per_sec: u32,
}

impl Default for DiskIOMetrics {
    fn default() -> Self {
        Self {
            read_bytes_per_sec: 0,
            write_bytes_per_sec: 0,
            read_ops_per_sec: 0,
            write_ops_per_sec: 0,
        }
    }
}

/// Network I/O metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkIOMetrics {
    /// Bytes received per second
    pub rx_bytes_per_sec: u64,
    
    /// Bytes transmitted per second
    pub tx_bytes_per_sec: u64,
    
    /// Packets received per second
    pub rx_packets_per_sec: u32,
    
    /// Packets transmitted per second
    pub tx_packets_per_sec: u32,
}

impl Default for NetworkIOMetrics {
    fn default() -> Self {
        Self {
            rx_bytes_per_sec: 0,
            tx_bytes_per_sec: 0,
            rx_packets_per_sec: 0,
            tx_packets_per_sec: 0,
        }
    }
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Check status
    pub status: HealthCheckStatus,
    
    /// Check duration
    pub duration: Duration,
    
    /// Check timestamp
    pub timestamp: Timestamp,
    
    /// Check details
    pub details: String,
    
    /// Additional metrics
    pub metrics: HashMap<String, f64>,
}

/// Health check status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthCheckStatus {
    Success,
    Failure,
    Timeout,
    Unknown,
}

/// Resource utilization metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUtilizationMetrics {
    /// CPU utilization percentage
    pub cpu_utilization: f64,
    
    /// Memory utilization percentage
    pub memory_utilization: f64,
    
    /// Disk utilization percentage
    pub disk_utilization: f64,
    
    /// Network utilization percentage
    pub network_utilization: f64,
    
    /// Available file descriptors
    pub available_file_descriptors: u64,
}

impl Default for ResourceUtilizationMetrics {
    fn default() -> Self {
        Self {
            cpu_utilization: 0.0,
            memory_utilization: 0.0,
            disk_utilization: 0.0,
            network_utilization: 0.0,
            available_file_descriptors: 1024,
        }
    }
}

/// Health event enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthEvent {
    ComponentHealthy {
        component: String,
        metrics: HashMap<String, f64>,
    },
    ComponentDegraded {
        component: String,
        severity: AlertSeverity,
        details: String,
    },
    ComponentRecovered {
        component: String,
        previous_status: HealthStatus,
        recovery_time: Duration,
    },
    ThresholdExceeded {
        metric_name: String,
        threshold: f64,
        current_value: f64,
        component: String,
    },
    RecoveryStarted {
        procedure_id: String,
        recovery_type: RecoveryType,
        target: String,
    },
    RecoveryCompleted {
        procedure_id: String,
        success: bool,
        duration: Duration,
    },
    AlertTriggered {
        alert: HealthAlert,
    },
    AlertResolved {
        alert_id: String,
        resolution_time: Duration,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_health_status_creation() {
        let node_id: nexus_shared::NodeId = "test-node".into();
        let status = SystemHealthStatus::new(node_id.clone());
        
        assert_eq!(status.node_id, node_id);
        assert_eq!(status.overall_status, HealthStatus::Unknown);
        assert!(status.components.is_empty());
        assert!(status.alerts.is_empty());
    }

    #[test]
    fn test_component_health_default() {
        let health = ComponentHealth::default();
        
        assert_eq!(health.score, 1.0);
        assert_eq!(health.status, HealthStatus::Healthy);
        assert!(health.metrics.is_empty());
        assert!(health.recent_events.is_empty());
    }

    #[test]
    fn test_alert_severity_display() {
        assert_eq!(AlertSeverity::Info.to_string(), "INFO");
        assert_eq!(AlertSeverity::Warning.to_string(), "WARNING");
        assert_eq!(AlertSeverity::Error.to_string(), "ERROR");
        assert_eq!(AlertSeverity::Critical.to_string(), "CRITICAL");
    }

    #[test]
    fn test_recovery_config_default() {
        let config = RecoveryConfig::default();
        
        assert!(config.enabled);
        assert_eq!(config.max_concurrent_recoveries, 3);
        assert_eq!(config.timeout, Duration::from_secs(300));
    }
}