//! Security Monitoring and Consensus Integration
//! 
//! This module implements security monitoring with mandatory Four-Proof consensus validation
//! for all certificate operations, Byzantine fault detection, and real-time security alerts.

use std::sync::Arc;
use std::time::{SystemTime, Duration};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use tokio::sync::{RwLock, Mutex};
use tracing::{info, warn, error, debug};
use dashmap::DashMap;

use crate::consensus::{ConsensusProof, ConsensusResult, FourProofValidator, ConsensusRequirements};
use crate::errors::{TrustChainError, Result as TrustChainResult};

pub mod monitoring;
pub mod byzantine;
pub mod alerts;

pub use monitoring::*;
pub use byzantine::*;
pub use alerts::*;

/// Security monitoring system with consensus integration
pub struct SecurityMonitor {
    /// Four-proof consensus validator (needs mutability for state tracking)
    consensus_validator: Arc<tokio::sync::Mutex<FourProofValidator>>,
    /// Byzantine fault detector
    byzantine_detector: Arc<ByzantineDetector>,
    /// Security alert manager
    alert_manager: Arc<SecurityAlertManager>,
    /// Security metrics collector
    metrics: Arc<SecurityMetrics>,
    /// Security event log
    event_log: Arc<SecurityEventLog>,
    /// Configuration
    config: Arc<SecurityConfig>,
}

/// Security configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Require consensus for all certificate operations
    pub mandatory_consensus: bool,
    /// Byzantine detection threshold (percentage of malicious behavior)
    pub byzantine_threshold: f64,
    /// Security alert severity levels
    pub alert_threshold: SecuritySeverity,
    /// Consensus requirements for certificate operations
    pub consensus_requirements: ConsensusRequirements,
    /// Enable real-time monitoring
    pub real_time_monitoring: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            mandatory_consensus: true,
            byzantine_threshold: 0.33, // 33% Byzantine tolerance
            alert_threshold: SecuritySeverity::Medium,
            consensus_requirements: ConsensusRequirements::production(),
            real_time_monitoring: true,
        }
    }
}

/// Security severity levels
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for SecuritySeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecuritySeverity::Low => write!(f, "Low"),
            SecuritySeverity::Medium => write!(f, "Medium"),
            SecuritySeverity::High => write!(f, "High"),
            SecuritySeverity::Critical => write!(f, "Critical"),
        }
    }
}

/// Security monitoring result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityValidationResult {
    /// Overall security validation result
    pub is_valid: bool,
    /// Consensus validation result
    pub consensus_result: Option<ConsensusResult>,
    /// Byzantine detection result
    pub byzantine_detection: ByzantineDetectionResult,
    /// Security alerts generated
    pub alerts: Vec<SecurityAlert>,
    /// Validation timestamp
    pub validated_at: SystemTime,
    /// Validation metrics
    pub metrics: ValidationMetrics,
}

/// Validation metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationMetrics {
    /// Consensus validation time (ms)
    pub consensus_time_ms: u64,
    /// Byzantine detection time (ms)
    pub byzantine_time_ms: u64,
    /// Total validation time (ms)
    pub total_time_ms: u64,
    /// Security score (0.0 - 1.0)
    pub security_score: f64,
}

/// Security metrics collector
#[derive(Default)]
pub struct SecurityMetrics {
    /// Total security validations performed
    pub validations_total: std::sync::atomic::AtomicU64,
    /// Successful security validations
    pub validations_successful: std::sync::atomic::AtomicU64,
    /// Failed security validations
    pub validations_failed: std::sync::atomic::AtomicU64,
    /// Consensus validations performed
    pub consensus_validations: std::sync::atomic::AtomicU64,
    /// Byzantine detections triggered
    pub byzantine_detections: std::sync::atomic::AtomicU64,
    /// Security alerts generated
    pub alerts_generated: std::sync::atomic::AtomicU64,
    /// Certificate operations requiring consensus
    pub certificate_consensus_required: std::sync::atomic::AtomicU64,
    /// Certificate operations consensus approved
    pub certificate_consensus_approved: std::sync::atomic::AtomicU64,
    /// Average validation time (ms)
    pub average_validation_time_ms: std::sync::atomic::AtomicU64,
}

impl Clone for SecurityMetrics {
    fn clone(&self) -> Self {
        Self {
            validations_total: std::sync::atomic::AtomicU64::new(
                self.validations_total.load(std::sync::atomic::Ordering::Relaxed)
            ),
            validations_successful: std::sync::atomic::AtomicU64::new(
                self.validations_successful.load(std::sync::atomic::Ordering::Relaxed)
            ),
            validations_failed: std::sync::atomic::AtomicU64::new(
                self.validations_failed.load(std::sync::atomic::Ordering::Relaxed)
            ),
            consensus_validations: std::sync::atomic::AtomicU64::new(
                self.consensus_validations.load(std::sync::atomic::Ordering::Relaxed)
            ),
            byzantine_detections: std::sync::atomic::AtomicU64::new(
                self.byzantine_detections.load(std::sync::atomic::Ordering::Relaxed)
            ),
            alerts_generated: std::sync::atomic::AtomicU64::new(
                self.alerts_generated.load(std::sync::atomic::Ordering::Relaxed)
            ),
            certificate_consensus_required: std::sync::atomic::AtomicU64::new(
                self.certificate_consensus_required.load(std::sync::atomic::Ordering::Relaxed)
            ),
            certificate_consensus_approved: std::sync::atomic::AtomicU64::new(
                self.certificate_consensus_approved.load(std::sync::atomic::Ordering::Relaxed)
            ),
            average_validation_time_ms: std::sync::atomic::AtomicU64::new(
                self.average_validation_time_ms.load(std::sync::atomic::Ordering::Relaxed)
            ),
        }
    }
}

impl std::fmt::Debug for SecurityMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecurityMetrics")
            .field("validations_total", &self.validations_total.load(std::sync::atomic::Ordering::Relaxed))
            .field("validations_successful", &self.validations_successful.load(std::sync::atomic::Ordering::Relaxed))
            .field("validations_failed", &self.validations_failed.load(std::sync::atomic::Ordering::Relaxed))
            .field("consensus_validations", &self.consensus_validations.load(std::sync::atomic::Ordering::Relaxed))
            .field("byzantine_detections", &self.byzantine_detections.load(std::sync::atomic::Ordering::Relaxed))
            .field("alerts_generated", &self.alerts_generated.load(std::sync::atomic::Ordering::Relaxed))
            .field("certificate_consensus_required", &self.certificate_consensus_required.load(std::sync::atomic::Ordering::Relaxed))
            .field("certificate_consensus_approved", &self.certificate_consensus_approved.load(std::sync::atomic::Ordering::Relaxed))
            .field("average_validation_time_ms", &self.average_validation_time_ms.load(std::sync::atomic::Ordering::Relaxed))
            .finish()
    }
}

impl serde::Serialize for SecurityMetrics {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SecurityMetrics", 9)?;
        state.serialize_field("validations_total", &self.validations_total.load(std::sync::atomic::Ordering::Relaxed))?;
        state.serialize_field("validations_successful", &self.validations_successful.load(std::sync::atomic::Ordering::Relaxed))?;
        state.serialize_field("validations_failed", &self.validations_failed.load(std::sync::atomic::Ordering::Relaxed))?;
        state.serialize_field("consensus_validations", &self.consensus_validations.load(std::sync::atomic::Ordering::Relaxed))?;
        state.serialize_field("byzantine_detections", &self.byzantine_detections.load(std::sync::atomic::Ordering::Relaxed))?;
        state.serialize_field("alerts_generated", &self.alerts_generated.load(std::sync::atomic::Ordering::Relaxed))?;
        state.serialize_field("certificate_consensus_required", &self.certificate_consensus_required.load(std::sync::atomic::Ordering::Relaxed))?;
        state.serialize_field("certificate_consensus_approved", &self.certificate_consensus_approved.load(std::sync::atomic::Ordering::Relaxed))?;
        state.serialize_field("average_validation_time_ms", &self.average_validation_time_ms.load(std::sync::atomic::Ordering::Relaxed))?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for SecurityMetrics {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct SecurityMetricsData {
            validations_total: u64,
            validations_successful: u64,
            validations_failed: u64,
            consensus_validations: u64,
            byzantine_detections: u64,
            alerts_generated: u64,
            certificate_consensus_required: u64,
            certificate_consensus_approved: u64,
            average_validation_time_ms: u64,
        }
        
        let data = SecurityMetricsData::deserialize(deserializer)?;
        Ok(Self {
            validations_total: std::sync::atomic::AtomicU64::new(data.validations_total),
            validations_successful: std::sync::atomic::AtomicU64::new(data.validations_successful),
            validations_failed: std::sync::atomic::AtomicU64::new(data.validations_failed),
            consensus_validations: std::sync::atomic::AtomicU64::new(data.consensus_validations),
            byzantine_detections: std::sync::atomic::AtomicU64::new(data.byzantine_detections),
            alerts_generated: std::sync::atomic::AtomicU64::new(data.alerts_generated),
            certificate_consensus_required: std::sync::atomic::AtomicU64::new(data.certificate_consensus_required),
            certificate_consensus_approved: std::sync::atomic::AtomicU64::new(data.certificate_consensus_approved),
            average_validation_time_ms: std::sync::atomic::AtomicU64::new(data.average_validation_time_ms),
        })
    }
}

/// Security event log
pub struct SecurityEventLog {
    /// Event storage
    events: Arc<DashMap<String, SecurityEvent>>,
    /// Event indices for fast lookup
    indices: Arc<RwLock<SecurityEventIndices>>,
}

/// Security event indices
#[derive(Default)]
pub struct SecurityEventIndices {
    /// Events by timestamp
    pub by_timestamp: Vec<String>,
    /// Events by severity
    pub by_severity: HashMap<SecuritySeverity, Vec<String>>,
    /// Events by type
    pub by_type: HashMap<String, Vec<String>>,
}

/// Security event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityEvent {
    /// Event ID
    pub event_id: String,
    /// Event type
    pub event_type: String,
    /// Event severity
    pub severity: SecuritySeverity,
    /// Event timestamp
    pub timestamp: SystemTime,
    /// Event description
    pub description: String,
    /// Associated consensus proof
    pub consensus_proof: Option<ConsensusProof>,
    /// Event metadata
    pub metadata: HashMap<String, String>,
}

impl SecurityMonitor {
    /// Create new security monitor with consensus integration
    pub async fn new(config: SecurityConfig) -> TrustChainResult<Self> {
        info!("Initializing Security Monitor with consensus integration");

        // Initialize consensus validator with Mutex for thread-safe mutability
        let consensus_validator = Arc::new(tokio::sync::Mutex::new(FourProofValidator::new()));

        // Initialize Byzantine detector
        let byzantine_detector = Arc::new(ByzantineDetector::new(config.byzantine_threshold).await?);

        // Initialize alert manager
        let alert_manager = Arc::new(SecurityAlertManager::new(config.alert_threshold.clone()).await?);

        // Initialize metrics
        let metrics = Arc::new(SecurityMetrics::default());

        // Initialize event log
        let event_log = Arc::new(SecurityEventLog::new().await?);

        let monitor = Self {
            consensus_validator,
            byzantine_detector,
            alert_manager,
            metrics,
            event_log,
            config: Arc::new(config),
        };

        info!("Security Monitor initialized with mandatory consensus validation");
        Ok(monitor)
    }

    /// Validate certificate operation with MANDATORY consensus
    pub async fn validate_certificate_operation(
        &self,
        operation: &str,
        consensus_proof: &ConsensusProof,
        context: &str,
    ) -> TrustChainResult<SecurityValidationResult> {
        let start_time = std::time::Instant::now();
        
        info!("Security validation for certificate operation: {} (context: {})", operation, context);

        // Increment total validations
        self.metrics.validations_total.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // CRITICAL: Consensus validation is MANDATORY for all certificate operations
        let consensus_start = std::time::Instant::now();
        let consensus_result = if self.config.mandatory_consensus {
            info!("MANDATORY consensus validation required for: {}", operation);
            self.metrics.consensus_validations.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.metrics.certificate_consensus_required.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            
            let mut validator = self.consensus_validator.lock().await;
            let result = validator.validate_consensus(consensus_proof).await?;
            
            if !result.is_valid() {
                error!("CONSENSUS VALIDATION FAILED for {}: {:?}", operation, result);
                
                // Generate critical security alert
                let alert = self.alert_manager.generate_alert(
                    SecuritySeverity::Critical,
                    "Consensus Validation Failed".to_string(),
                    format!("Certificate operation {} failed consensus validation", operation),
                    Some(consensus_proof.clone()),
                ).await?;

                // Log security event
                self.log_security_event(
                    "consensus_validation_failed".to_string(),
                    SecuritySeverity::Critical,
                    format!("Consensus validation failed for operation: {}", operation),
                    Some(consensus_proof.clone()),
                ).await?;

                self.metrics.validations_failed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                
                return Ok(SecurityValidationResult {
                    is_valid: false,
                    consensus_result: Some(result),
                    byzantine_detection: ByzantineDetectionResult::NotDetected,
                    alerts: vec![alert],
                    validated_at: SystemTime::now(),
                    metrics: ValidationMetrics {
                        consensus_time_ms: consensus_start.elapsed().as_millis() as u64,
                        byzantine_time_ms: 0,
                        total_time_ms: start_time.elapsed().as_millis() as u64,
                        security_score: 0.0,
                    },
                });
            } else {
                info!("Consensus validation SUCCESSFUL for: {}", operation);
                self.metrics.certificate_consensus_approved.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            
            Some(result)
        } else {
            warn!("Consensus validation DISABLED - SECURITY RISK for: {}", operation);
            None
        };

        let consensus_time = consensus_start.elapsed().as_millis() as u64;

        // Byzantine fault detection
        let byzantine_start = std::time::Instant::now();
        let byzantine_result = self.byzantine_detector.detect_byzantine_behavior(
            consensus_proof,
            operation,
        ).await?;

        let byzantine_time = byzantine_start.elapsed().as_millis() as u64;

        // Process Byzantine detection results
        let mut alerts = Vec::new();
        if let ByzantineDetectionResult::Detected { .. } = &byzantine_result {
            warn!("Byzantine behavior detected for operation: {}", operation);
            self.metrics.byzantine_detections.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            
            // Generate Byzantine detection alert
            let alert = self.alert_manager.generate_alert(
                SecuritySeverity::High,
                "Byzantine Behavior Detected".to_string(),
                format!("Byzantine fault detected in operation: {}", operation),
                Some(consensus_proof.clone()),
            ).await?;
            
            alerts.push(alert);

            // Log Byzantine event
            self.log_security_event(
                "byzantine_detection".to_string(),
                SecuritySeverity::High,
                format!("Byzantine behavior detected for operation: {}", operation),
                Some(consensus_proof.clone()),
            ).await?;
        }

        // Calculate security score
        let security_score = self.calculate_security_score(&consensus_result, &byzantine_result);

        // Update metrics
        let total_time = start_time.elapsed().as_millis() as u64;
        self.metrics.average_validation_time_ms.store(total_time, std::sync::atomic::Ordering::Relaxed);
        self.metrics.validations_successful.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Log successful validation
        self.log_security_event(
            "security_validation_successful".to_string(),
            SecuritySeverity::Low,
            format!("Security validation successful for operation: {} (score: {:.2})", operation, security_score),
            Some(consensus_proof.clone()),
        ).await?;

        let result = SecurityValidationResult {
            is_valid: security_score >= 0.8, // 80% minimum security score
            consensus_result,
            byzantine_detection: byzantine_result,
            alerts,
            validated_at: SystemTime::now(),
            metrics: ValidationMetrics {
                consensus_time_ms: consensus_time,
                byzantine_time_ms: byzantine_time,
                total_time_ms: total_time,
                security_score,
            },
        };

        info!("Security validation completed for {}: valid={}, score={:.2}, time={}ms", 
              operation, result.is_valid, security_score, total_time);

        Ok(result)
    }

    /// Get security monitoring dashboard data
    pub async fn get_monitoring_dashboard(&self) -> TrustChainResult<SecurityDashboard> {
        debug!("Generating security monitoring dashboard");

        let metrics = SecurityDashboardMetrics {
            validations_total: self.metrics.validations_total.load(std::sync::atomic::Ordering::Relaxed),
            validations_successful: self.metrics.validations_successful.load(std::sync::atomic::Ordering::Relaxed),
            validations_failed: self.metrics.validations_failed.load(std::sync::atomic::Ordering::Relaxed),
            consensus_validations: self.metrics.consensus_validations.load(std::sync::atomic::Ordering::Relaxed),
            byzantine_detections: self.metrics.byzantine_detections.load(std::sync::atomic::Ordering::Relaxed),
            alerts_generated: self.metrics.alerts_generated.load(std::sync::atomic::Ordering::Relaxed),
            certificate_consensus_required: self.metrics.certificate_consensus_required.load(std::sync::atomic::Ordering::Relaxed),
            certificate_consensus_approved: self.metrics.certificate_consensus_approved.load(std::sync::atomic::Ordering::Relaxed),
            average_validation_time_ms: self.metrics.average_validation_time_ms.load(std::sync::atomic::Ordering::Relaxed),
        };

        let recent_alerts = self.alert_manager.get_recent_alerts(10).await?;
        let recent_events = self.event_log.get_recent_events(20).await?;
        let byzantine_summary = self.byzantine_detector.get_detection_summary().await?;

        Ok(SecurityDashboard {
            metrics,
            recent_alerts,
            recent_events,
            byzantine_summary,
            consensus_status: self.get_consensus_status().await?,
            timestamp: SystemTime::now(),
        })
    }

    /// Get consensus validation status
    async fn get_consensus_status(&self) -> TrustChainResult<ConsensusStatus> {
        let total_required = self.metrics.certificate_consensus_required.load(std::sync::atomic::Ordering::Relaxed);
        let total_approved = self.metrics.certificate_consensus_approved.load(std::sync::atomic::Ordering::Relaxed);
        
        let approval_rate = if total_required > 0 {
            (total_approved as f64 / total_required as f64) * 100.0
        } else {
            100.0
        };

        Ok(ConsensusStatus {
            enabled: self.config.mandatory_consensus,
            total_validations: self.metrics.consensus_validations.load(std::sync::atomic::Ordering::Relaxed),
            approval_rate,
            requirements: self.config.consensus_requirements.clone(),
        })
    }

    /// Calculate security score based on validation results
    fn calculate_security_score(&self, consensus_result: &Option<ConsensusResult>, byzantine_result: &ByzantineDetectionResult) -> f64 {
        let mut score = 0.0;

        // Consensus validation score (70% weight)
        if let Some(result) = consensus_result {
            if result.is_valid() {
                score += 0.7;
            }
        } else if !self.config.mandatory_consensus {
            // If consensus is disabled, give partial credit
            score += 0.3;
        }

        // Byzantine detection score (30% weight)
        match byzantine_result {
            ByzantineDetectionResult::NotDetected => score += 0.3,
            ByzantineDetectionResult::Detected { confidence, .. } => {
                // Reduce score based on Byzantine confidence
                score += 0.3 * (1.0 - confidence);
            }
        }

        score.min(1.0).max(0.0)
    }

    /// Log security event
    async fn log_security_event(
        &self,
        event_type: String,
        severity: SecuritySeverity,
        description: String,
        consensus_proof: Option<ConsensusProof>,
    ) -> TrustChainResult<()> {
        let event = SecurityEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type: event_type.clone(),
            severity: severity.clone(),
            timestamp: SystemTime::now(),
            description,
            consensus_proof,
            metadata: HashMap::new(),
        };

        self.event_log.log_event(event).await?;
        debug!("Security event logged: {} ({})", event_type, severity);
        Ok(())
    }

    /// Get security metrics
    pub async fn get_metrics(&self) -> SecurityMetrics {
        SecurityMetrics {
            validations_total: std::sync::atomic::AtomicU64::new(
                self.metrics.validations_total.load(std::sync::atomic::Ordering::Relaxed)
            ),
            validations_successful: std::sync::atomic::AtomicU64::new(
                self.metrics.validations_successful.load(std::sync::atomic::Ordering::Relaxed)
            ),
            validations_failed: std::sync::atomic::AtomicU64::new(
                self.metrics.validations_failed.load(std::sync::atomic::Ordering::Relaxed)
            ),
            consensus_validations: std::sync::atomic::AtomicU64::new(
                self.metrics.consensus_validations.load(std::sync::atomic::Ordering::Relaxed)
            ),
            byzantine_detections: std::sync::atomic::AtomicU64::new(
                self.metrics.byzantine_detections.load(std::sync::atomic::Ordering::Relaxed)
            ),
            alerts_generated: std::sync::atomic::AtomicU64::new(
                self.metrics.alerts_generated.load(std::sync::atomic::Ordering::Relaxed)
            ),
            certificate_consensus_required: std::sync::atomic::AtomicU64::new(
                self.metrics.certificate_consensus_required.load(std::sync::atomic::Ordering::Relaxed)
            ),
            certificate_consensus_approved: std::sync::atomic::AtomicU64::new(
                self.metrics.certificate_consensus_approved.load(std::sync::atomic::Ordering::Relaxed)
            ),
            average_validation_time_ms: std::sync::atomic::AtomicU64::new(
                self.metrics.average_validation_time_ms.load(std::sync::atomic::Ordering::Relaxed)
            ),
        }
    }
}

impl SecurityEventLog {
    /// Create new security event log
    pub async fn new() -> TrustChainResult<Self> {
        Ok(Self {
            events: Arc::new(DashMap::new()),
            indices: Arc::new(RwLock::new(SecurityEventIndices::default())),
        })
    }

    /// Log security event
    pub async fn log_event(&self, event: SecurityEvent) -> TrustChainResult<()> {
        let event_id = event.event_id.clone();
        
        // Store event
        self.events.insert(event_id.clone(), event.clone());

        // Update indices
        {
            let mut indices = self.indices.write().await;
            
            // Add to timestamp index
            indices.by_timestamp.push(event_id.clone());
            indices.by_timestamp.sort_by(|a, b| {
                let event_a = self.events.get(a).map(|e| e.timestamp);
                let event_b = self.events.get(b).map(|e| e.timestamp);
                event_b.cmp(&event_a) // Reverse order (newest first)
            });

            // Add to severity index
            indices.by_severity.entry(event.severity.clone())
                .or_insert_with(Vec::new)
                .push(event_id.clone());

            // Add to type index
            indices.by_type.entry(event.event_type.clone())
                .or_insert_with(Vec::new)
                .push(event_id.clone());
        }

        Ok(())
    }

    /// Get recent events
    pub async fn get_recent_events(&self, limit: usize) -> TrustChainResult<Vec<SecurityEvent>> {
        let indices = self.indices.read().await;
        let event_ids = indices.by_timestamp.iter().take(limit);
        
        let mut events = Vec::new();
        for event_id in event_ids {
            if let Some(event) = self.events.get(event_id) {
                events.push(event.clone());
            }
        }
        
        Ok(events)
    }
}

/// Security dashboard data structure
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityDashboard {
    /// Security metrics
    pub metrics: SecurityDashboardMetrics,
    /// Recent security alerts
    pub recent_alerts: Vec<SecurityAlert>,
    /// Recent security events
    pub recent_events: Vec<SecurityEvent>,
    /// Byzantine detection summary
    pub byzantine_summary: ByzantineDetectionSummary,
    /// Consensus status
    pub consensus_status: ConsensusStatus,
    /// Dashboard timestamp
    pub timestamp: SystemTime,
}

/// Security dashboard metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityDashboardMetrics {
    pub validations_total: u64,
    pub validations_successful: u64,
    pub validations_failed: u64,
    pub consensus_validations: u64,
    pub byzantine_detections: u64,
    pub alerts_generated: u64,
    pub certificate_consensus_required: u64,
    pub certificate_consensus_approved: u64,
    pub average_validation_time_ms: u64,
}

/// Consensus status
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsensusStatus {
    /// Whether consensus validation is enabled
    pub enabled: bool,
    /// Total consensus validations performed
    pub total_validations: u64,
    /// Consensus approval rate (percentage)
    pub approval_rate: f64,
    /// Consensus requirements
    pub requirements: ConsensusRequirements,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::ConsensusProof;

    #[tokio::test]
    async fn test_security_monitor_creation() {
        let config = SecurityConfig::default();
        let monitor = SecurityMonitor::new(config).await.unwrap();
        
        let metrics = monitor.get_metrics().await;
        assert_eq!(metrics.validations_total.load(std::sync::atomic::Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn test_mandatory_consensus_validation() {
        let mut config = SecurityConfig::default();
        config.mandatory_consensus = true;
        
        let monitor = SecurityMonitor::new(config).await.unwrap();
        
        let consensus_proof = ConsensusProof::default_for_testing();
        let result = monitor.validate_certificate_operation(
            "issue_certificate",
            &consensus_proof,
            "test_validation",
        ).await.unwrap();
        
        // Should have consensus result since mandatory consensus is enabled
        assert!(result.consensus_result.is_some());
        
        let metrics = monitor.get_metrics().await;
        assert_eq!(metrics.consensus_validations.load(std::sync::atomic::Ordering::Relaxed), 1);
        assert_eq!(metrics.certificate_consensus_required.load(std::sync::atomic::Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_security_dashboard() {
        let config = SecurityConfig::default();
        let monitor = SecurityMonitor::new(config).await.unwrap();
        
        let dashboard = monitor.get_monitoring_dashboard().await.unwrap();
        
        assert_eq!(dashboard.metrics.validations_total, 0);
        assert!(dashboard.consensus_status.enabled);
    }

    #[tokio::test]
    async fn test_security_event_logging() {
        let event_log = SecurityEventLog::new().await.unwrap();
        
        let event = SecurityEvent {
            event_id: "test_event_001".to_string(),
            event_type: "test_event".to_string(),
            severity: SecuritySeverity::Medium,
            timestamp: SystemTime::now(),
            description: "Test security event".to_string(),
            consensus_proof: None,
            metadata: HashMap::new(),
        };
        
        event_log.log_event(event).await.unwrap();
        
        let recent_events = event_log.get_recent_events(10).await.unwrap();
        assert_eq!(recent_events.len(), 1);
        assert_eq!(recent_events[0].event_type, "test_event");
    }
}