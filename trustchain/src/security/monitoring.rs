//! Security Monitoring Dashboard
//! 
//! Real-time security monitoring with consensus validation integration

use std::sync::Arc;
use std::time::{SystemTime, Duration, UNIX_EPOCH};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

use crate::consensus::{ConsensusProof, ConsensusResult};
use crate::errors::{TrustChainError, Result as TrustChainResult};
use super::{SecuritySeverity, SecurityEvent};

/// Real-time security monitoring dashboard
pub struct SecurityMonitoringDashboard {
    /// Security metrics
    metrics: Arc<RwLock<SecurityDashboardData>>,
    /// Active monitoring sessions
    active_sessions: Arc<RwLock<HashMap<String, MonitoringSession>>>,
    /// Configuration
    config: SecurityMonitoringConfig,
}

/// Security dashboard data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityDashboardData {
    /// Real-time security status
    pub security_status: SecurityStatus,
    /// Live certificate operations requiring consensus
    pub live_certificate_operations: Vec<LiveCertificateOperation>,
    /// Active security alerts
    pub active_alerts: Vec<ActiveSecurityAlert>,
    /// Consensus validation metrics
    pub consensus_metrics: LiveConsensusMetrics,
    /// Byzantine detection results
    pub byzantine_metrics: ByzantineMetrics,
    /// Performance metrics
    pub performance_metrics: SecurityPerformanceMetrics,
    /// Last update timestamp
    pub last_update: SystemTime,
}

impl Default for SecurityDashboardData {
    fn default() -> Self {
        Self {
            security_status: SecurityStatus::default(),
            live_certificate_operations: Vec::new(),
            active_alerts: Vec::new(),
            consensus_metrics: LiveConsensusMetrics::default(),
            byzantine_metrics: ByzantineMetrics::default(),
            performance_metrics: SecurityPerformanceMetrics::default(),
            last_update: SystemTime::now(),
        }
    }
}

/// Real-time security status
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SecurityStatus {
    Secure,
    Warning { reason: String },
    Alert { severity: SecuritySeverity, reason: String },
    Critical { reason: String },
}

impl Default for SecurityStatus {
    fn default() -> Self {
        Self::Secure
    }
}

/// Live certificate operation with consensus validation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiveCertificateOperation {
    /// Operation ID
    pub operation_id: String,
    /// Operation type (issue, revoke, validate)
    pub operation_type: String,
    /// Common name for certificate
    pub common_name: String,
    /// Node ID requesting operation
    pub node_id: String,
    /// Consensus proof provided
    pub consensus_proof: ConsensusProof,
    /// Consensus validation status
    pub consensus_status: ConsensusValidationStatus,
    /// Operation start time
    pub started_at: SystemTime,
    /// Operation current state
    pub state: OperationState,
}

/// Consensus validation status for dashboard
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConsensusValidationStatus {
    Pending,
    Validating,
    Approved { validation_time_ms: u64 },
    Rejected { reason: String },
    Failed { error: String },
}

/// Operation state
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OperationState {
    Created,
    ConsensusValidation,
    ByzantineDetection,
    CertificateGeneration,
    CTLogging,
    Completed,
    Failed { reason: String },
}

/// Active security alert with real-time updates
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActiveSecurityAlert {
    /// Alert ID
    pub alert_id: String,
    /// Alert severity
    pub severity: SecuritySeverity,
    /// Alert title
    pub title: String,
    /// Alert description
    pub description: String,
    /// Alert timestamp
    pub timestamp: SystemTime,
    /// Associated operation (if any)
    pub operation_id: Option<String>,
    /// Alert status
    pub status: AlertStatus,
    /// Consensus proof related to alert
    pub consensus_proof: Option<ConsensusProof>,
}

/// Alert status
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AlertStatus {
    Active,
    Acknowledged,
    Resolved,
    Escalated,
}

/// Live consensus metrics
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct LiveConsensusMetrics {
    /// Total consensus validations in progress
    pub validations_in_progress: u64,
    /// Recent validation rate (validations per second)
    pub validation_rate: f64,
    /// Current approval rate (percentage)
    pub approval_rate: f64,
    /// Average validation time (ms)
    pub avg_validation_time_ms: u64,
    /// Proof validation breakdown
    pub proof_validation_breakdown: ProofValidationBreakdown,
}

/// Proof validation breakdown (Four-Proof system)
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ProofValidationBreakdown {
    /// Proof of Space validation success rate
    pub space_proof_success_rate: f64,
    /// Proof of Stake validation success rate
    pub stake_proof_success_rate: f64,
    /// Proof of Work validation success rate
    pub work_proof_success_rate: f64,
    /// Proof of Time validation success rate
    pub time_proof_success_rate: f64,
}

/// Byzantine detection metrics
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ByzantineMetrics {
    /// Active Byzantine detections
    pub active_detections: u64,
    /// Recent detection rate
    pub detection_rate: f64,
    /// Suspected malicious nodes
    pub suspected_nodes: Vec<String>,
    /// Byzantine confidence scores
    pub confidence_scores: HashMap<String, f64>,
}

/// Security performance metrics
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SecurityPerformanceMetrics {
    /// Total operations processed
    pub operations_processed: u64,
    /// Operations per second
    pub operations_per_second: f64,
    /// Average total processing time (ms)
    pub avg_total_time_ms: u64,
    /// Security validation overhead (ms)
    pub security_overhead_ms: u64,
    /// Memory usage (MB)
    pub memory_usage_mb: u64,
}

/// Monitoring session
#[derive(Clone, Debug)]
pub struct MonitoringSession {
    /// Session ID
    pub session_id: String,
    /// Client identifier
    pub client_id: String,
    /// Session start time
    pub started_at: SystemTime,
    /// Last activity
    pub last_activity: SystemTime,
    /// Session configuration
    pub config: SessionConfig,
}

/// Session configuration
#[derive(Clone, Debug, Default)]
pub struct SessionConfig {
    /// Update interval (seconds)
    pub update_interval: u32,
    /// Include detailed metrics
    pub detailed_metrics: bool,
    /// Monitor specific operations
    pub operation_filter: Option<String>,
}

/// Security monitoring configuration
#[derive(Clone, Debug)]
pub struct SecurityMonitoringConfig {
    /// Enable real-time monitoring
    pub enabled: bool,
    /// Update interval for dashboard (seconds)
    pub update_interval: u32,
    /// Maximum monitoring sessions
    pub max_sessions: u32,
    /// Alert retention period (hours)
    pub alert_retention_hours: u32,
}

impl Default for SecurityMonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            update_interval: 5, // 5 seconds
            max_sessions: 100,
            alert_retention_hours: 24,
        }
    }
}

impl SecurityMonitoringDashboard {
    /// Create new security monitoring dashboard
    pub async fn new(config: SecurityMonitoringConfig) -> TrustChainResult<Self> {
        info!("Initializing Security Monitoring Dashboard");

        Ok(Self {
            metrics: Arc::new(RwLock::new(SecurityDashboardData::default())),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            config,
        })
    }

    /// Start monitoring session
    pub async fn start_monitoring_session(&self, client_id: String, session_config: SessionConfig) -> TrustChainResult<String> {
        let session_id = uuid::Uuid::new_v4().to_string();
        
        let session = MonitoringSession {
            session_id: session_id.clone(),
            client_id: client_id.clone(),
            started_at: SystemTime::now(),
            last_activity: SystemTime::now(),
            config: session_config,
        };

        {
            let mut sessions = self.active_sessions.write().await;
            
            // Check session limit
            if sessions.len() >= self.config.max_sessions as usize {
                return Err(TrustChainError::SecurityError {
                    message: "Maximum monitoring sessions reached".to_string(),
                });
            }
            
            sessions.insert(session_id.clone(), session);
        }

        info!("Started monitoring session: {} for client: {}", session_id, client_id);
        Ok(session_id)
    }

    /// Update certificate operation status
    pub async fn update_certificate_operation(
        &self,
        operation_id: String,
        state: OperationState,
        consensus_status: Option<ConsensusValidationStatus>,
    ) -> TrustChainResult<()> {
        let mut metrics = self.metrics.write().await;
        
        // Find and update the operation
        if let Some(operation) = metrics.live_certificate_operations.iter_mut()
            .find(|op| op.operation_id == operation_id) {
            
            operation.state = state;
            if let Some(status) = consensus_status {
                operation.consensus_status = status;
            }
        }

        metrics.last_update = SystemTime::now();
        debug!("Updated certificate operation: {}", operation_id);
        Ok(())
    }

    /// Add new certificate operation to monitoring
    pub async fn add_certificate_operation(&self, operation: LiveCertificateOperation) -> TrustChainResult<()> {
        let mut metrics = self.metrics.write().await;
        metrics.live_certificate_operations.push(operation.clone());
        metrics.last_update = SystemTime::now();
        
        info!("Added certificate operation to monitoring: {} ({})", operation.operation_id, operation.operation_type);
        Ok(())
    }

    /// Add security alert to dashboard
    pub async fn add_security_alert(&self, alert: ActiveSecurityAlert) -> TrustChainResult<()> {
        let mut metrics = self.metrics.write().await;
        
        // Update security status based on alert severity
        match alert.severity {
            SecuritySeverity::Critical => {
                metrics.security_status = SecurityStatus::Critical {
                    reason: alert.title.clone(),
                };
            }
            SecuritySeverity::High => {
                if matches!(metrics.security_status, SecurityStatus::Secure | SecurityStatus::Warning { .. }) {
                    metrics.security_status = SecurityStatus::Alert {
                        severity: SecuritySeverity::High,
                        reason: alert.title.clone(),
                    };
                }
            }
            SecuritySeverity::Medium => {
                if matches!(metrics.security_status, SecurityStatus::Secure) {
                    metrics.security_status = SecurityStatus::Warning {
                        reason: alert.title.clone(),
                    };
                }
            }
            SecuritySeverity::Low => {
                // Don't change status for low severity alerts
            }
        }
        
        metrics.active_alerts.push(alert.clone());
        metrics.last_update = SystemTime::now();
        
        warn!("Security alert added to dashboard: {} ({})", alert.title, alert.severity);
        Ok(())
    }

    /// Update consensus metrics
    pub async fn update_consensus_metrics(&self, metrics_update: LiveConsensusMetrics) -> TrustChainResult<()> {
        let mut metrics = self.metrics.write().await;
        metrics.consensus_metrics = metrics_update;
        metrics.last_update = SystemTime::now();
        
        debug!("Consensus metrics updated");
        Ok(())
    }

    /// Update Byzantine metrics
    pub async fn update_byzantine_metrics(&self, byzantine_update: ByzantineMetrics) -> TrustChainResult<()> {
        let mut metrics = self.metrics.write().await;
        metrics.byzantine_metrics = byzantine_update;
        metrics.last_update = SystemTime::now();
        
        debug!("Byzantine metrics updated");
        Ok(())
    }

    /// Get current dashboard data
    pub async fn get_dashboard_data(&self) -> TrustChainResult<SecurityDashboardData> {
        let metrics = self.metrics.read().await;
        Ok(metrics.clone())
    }

    /// Get real-time updates for session
    pub async fn get_session_updates(&self, session_id: &str) -> TrustChainResult<SecurityDashboardData> {
        // Update session activity
        {
            let mut sessions = self.active_sessions.write().await;
            if let Some(session) = sessions.get_mut(session_id) {
                session.last_activity = SystemTime::now();
            } else {
                return Err(TrustChainError::SecurityError {
                    message: "Invalid session ID".to_string(),
                });
            }
        }

        self.get_dashboard_data().await
    }

    /// Cleanup completed operations and old alerts
    pub async fn cleanup_dashboard(&self) -> TrustChainResult<()> {
        let mut metrics = self.metrics.write().await;
        
        // Remove completed operations older than 1 hour
        let cutoff_time = SystemTime::now() - Duration::from_secs(3600);
        metrics.live_certificate_operations.retain(|op| {
            match op.state {
                OperationState::Completed | OperationState::Failed { .. } => {
                    op.started_at > cutoff_time
                }
                _ => true, // Keep ongoing operations
            }
        });

        // Remove old resolved alerts
        let alert_cutoff = SystemTime::now() - Duration::from_secs(self.config.alert_retention_hours as u64 * 3600);
        metrics.active_alerts.retain(|alert| {
            match alert.status {
                AlertStatus::Resolved => alert.timestamp > alert_cutoff,
                _ => true, // Keep active alerts
            }
        });

        // Update security status if no active critical/high alerts
        if !metrics.active_alerts.iter().any(|a| {
            matches!(a.severity, SecuritySeverity::Critical | SecuritySeverity::High) &&
            matches!(a.status, AlertStatus::Active)
        }) {
            metrics.security_status = SecurityStatus::Secure;
        }

        debug!("Dashboard cleanup completed");
        Ok(())
    }

    /// Stop monitoring session
    pub async fn stop_monitoring_session(&self, session_id: &str) -> TrustChainResult<()> {
        let mut sessions = self.active_sessions.write().await;
        
        if sessions.remove(session_id).is_some() {
            info!("Stopped monitoring session: {}", session_id);
            Ok(())
        } else {
            Err(TrustChainError::SecurityError {
                message: "Session not found".to_string(),
            })
        }
    }

    /// Get active session count
    pub async fn get_active_session_count(&self) -> usize {
        self.active_sessions.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::ConsensusProof;

    #[tokio::test]
    async fn test_dashboard_creation() {
        let config = SecurityMonitoringConfig::default();
        let dashboard = SecurityMonitoringDashboard::new(config).await.unwrap();
        
        let data = dashboard.get_dashboard_data().await.unwrap();
        assert!(matches!(data.security_status, SecurityStatus::Secure));
    }

    #[tokio::test]
    async fn test_monitoring_session() {
        let config = SecurityMonitoringConfig::default();
        let dashboard = SecurityMonitoringDashboard::new(config).await.unwrap();
        
        let session_config = SessionConfig::default();
        let session_id = dashboard.start_monitoring_session(
            "test_client".to_string(),
            session_config
        ).await.unwrap();
        
        assert_eq!(dashboard.get_active_session_count().await, 1);
        
        dashboard.stop_monitoring_session(&session_id).await.unwrap();
        assert_eq!(dashboard.get_active_session_count().await, 0);
    }

    #[tokio::test]
    async fn test_certificate_operation_tracking() {
        let config = SecurityMonitoringConfig::default();
        let dashboard = SecurityMonitoringDashboard::new(config).await.unwrap();
        
        let operation = LiveCertificateOperation {
            operation_id: "test_op_001".to_string(),
            operation_type: "issue_certificate".to_string(),
            common_name: "test.example.com".to_string(),
            node_id: "test_node_001".to_string(),
            consensus_proof: ConsensusProof::default_for_testing(),
            consensus_status: ConsensusValidationStatus::Pending,
            started_at: SystemTime::now(),
            state: OperationState::Created,
        };
        
        dashboard.add_certificate_operation(operation).await.unwrap();
        
        let data = dashboard.get_dashboard_data().await.unwrap();
        assert_eq!(data.live_certificate_operations.len(), 1);
        assert_eq!(data.live_certificate_operations[0].operation_id, "test_op_001");
    }

    #[tokio::test]
    async fn test_security_alert_handling() {
        let config = SecurityMonitoringConfig::default();
        let dashboard = SecurityMonitoringDashboard::new(config).await.unwrap();
        
        let alert = ActiveSecurityAlert {
            alert_id: "alert_001".to_string(),
            severity: SecuritySeverity::Critical,
            title: "Consensus Validation Failed".to_string(),
            description: "Critical consensus validation failure detected".to_string(),
            timestamp: SystemTime::now(),
            operation_id: Some("test_op_001".to_string()),
            status: AlertStatus::Active,
            consensus_proof: None,
        };
        
        dashboard.add_security_alert(alert).await.unwrap();
        
        let data = dashboard.get_dashboard_data().await.unwrap();
        assert_eq!(data.active_alerts.len(), 1);
        assert!(matches!(data.security_status, SecurityStatus::Critical { .. }));
    }
}