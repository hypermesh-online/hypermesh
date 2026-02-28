// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Security Monitoring and Consensus Integration
//!
//! This module implements security monitoring with mandatory Four-Proof consensus validation
//! for all certificate operations, Byzantine fault detection, and real-time security alerts.

use std::sync::Arc;
use std::time::SystemTime;
use std::collections::HashMap;
use tracing::{info, warn, error, debug};

use crate::consensus::{ConsensusProof, ConsensusResult, FourProofValidator};
use crate::errors::Result as TrustChainResult;

pub mod monitoring;
pub mod byzantine;
pub mod alerts;
pub mod types;
pub mod trust_scoring;

#[allow(ambiguous_glob_reexports)]
pub use monitoring::*;
pub use byzantine::*;
#[allow(ambiguous_glob_reexports)]
pub use alerts::*;
pub use types::*;
pub use trust_scoring::*;

/// Security monitoring system with consensus integration
pub struct SecurityMonitor {
    consensus_validator: Arc<tokio::sync::Mutex<FourProofValidator>>,
    byzantine_detector: Arc<ByzantineDetector>,
    alert_manager: Arc<SecurityAlertManager>,
    metrics: Arc<SecurityMetrics>,
    event_log: Arc<SecurityEventLog>,
    config: Arc<SecurityConfig>,
}

impl SecurityMonitor {
    /// Create new security monitor with consensus integration
    pub async fn new(config: SecurityConfig) -> TrustChainResult<Self> {
        info!("Initializing Security Monitor with consensus integration");
        let consensus_validator = Arc::new(tokio::sync::Mutex::new(FourProofValidator::new()));
        let byzantine_detector = Arc::new(ByzantineDetector::new(config.byzantine_threshold).await?);
        let alert_manager = Arc::new(SecurityAlertManager::new(config.alert_threshold.clone()).await?);
        let metrics = Arc::new(SecurityMetrics::default());
        let event_log = Arc::new(SecurityEventLog::new().await?);
        let monitor = Self {
            consensus_validator, byzantine_detector, alert_manager,
            metrics, event_log, config: Arc::new(config),
        };
        info!("Security Monitor initialized with mandatory consensus validation");
        Ok(monitor)
    }

    /// Validate certificate operation with MANDATORY consensus
    pub async fn validate_certificate_operation(
        &self, operation: &str, consensus_proof: &ConsensusProof, context: &str,
    ) -> TrustChainResult<SecurityValidationResult> {
        let start_time = std::time::Instant::now();
        info!("Security validation for certificate operation: {} (context: {})", operation, context);
        self.metrics.validations_total.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let consensus_start = std::time::Instant::now();
        let consensus_result = if self.config.mandatory_consensus {
            info!("MANDATORY consensus validation required for: {}", operation);
            self.metrics.consensus_validations.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.metrics.certificate_consensus_required.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let mut validator = self.consensus_validator.lock().await;
            let result = validator.validate_consensus(consensus_proof).await?;
            if !result.is_valid() {
                error!("CONSENSUS VALIDATION FAILED for {}: {:?}", operation, result);
                let alert = self.alert_manager.generate_alert(
                    SecuritySeverity::Critical, "Consensus Validation Failed".to_string(),
                    format!("Certificate operation {} failed consensus validation", operation),
                    Some(consensus_proof.clone()),
                ).await?;
                self.log_security_event("consensus_validation_failed".to_string(),
                    SecuritySeverity::Critical,
                    format!("Consensus validation failed for operation: {}", operation),
                    Some(consensus_proof.clone()),
                ).await?;
                self.metrics.validations_failed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return Ok(SecurityValidationResult {
                    is_valid: false, consensus_result: Some(result),
                    byzantine_detection: ByzantineDetectionResult::NotDetected,
                    alerts: vec![alert], validated_at: SystemTime::now(),
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

        let byzantine_start = std::time::Instant::now();
        let byzantine_result = self.byzantine_detector.detect_byzantine_behavior(
            consensus_proof, operation,
        ).await?;
        let byzantine_time = byzantine_start.elapsed().as_millis() as u64;

        let mut alerts = Vec::new();
        if let ByzantineDetectionResult::Detected { .. } = &byzantine_result {
            warn!("Byzantine behavior detected for operation: {}", operation);
            self.metrics.byzantine_detections.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let alert = self.alert_manager.generate_alert(
                SecuritySeverity::High, "Byzantine Behavior Detected".to_string(),
                format!("Byzantine fault detected in operation: {}", operation),
                Some(consensus_proof.clone()),
            ).await?;
            alerts.push(alert);
            self.log_security_event("byzantine_detection".to_string(),
                SecuritySeverity::High,
                format!("Byzantine behavior detected for operation: {}", operation),
                Some(consensus_proof.clone()),
            ).await?;
        }

        let security_score = self.calculate_security_score(&consensus_result, &byzantine_result);
        let total_time = start_time.elapsed().as_millis() as u64;
        self.metrics.average_validation_time_ms.store(total_time, std::sync::atomic::Ordering::Relaxed);
        self.metrics.validations_successful.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        self.log_security_event("security_validation_successful".to_string(),
            SecuritySeverity::Low,
            format!("Security validation successful for operation: {} (score: {:.2})", operation, security_score),
            Some(consensus_proof.clone()),
        ).await?;

        let result = SecurityValidationResult {
            is_valid: security_score >= 0.8,
            consensus_result, byzantine_detection: byzantine_result,
            alerts, validated_at: SystemTime::now(),
            metrics: ValidationMetrics {
                consensus_time_ms: consensus_time, byzantine_time_ms: byzantine_time,
                total_time_ms: total_time, security_score,
            },
        };
        info!("Security validation completed for {}: valid={}, score={:.2}, time={}ms",
              operation, result.is_valid, security_score, total_time);
        Ok(result)
    }

    /// Get security monitoring dashboard data
    pub async fn get_monitoring_dashboard(&self) -> TrustChainResult<SecurityDashboard> {
        debug!("Generating security monitoring dashboard");
        use std::sync::atomic::Ordering::Relaxed;
        let metrics = SecurityDashboardMetrics {
            validations_total: self.metrics.validations_total.load(Relaxed),
            validations_successful: self.metrics.validations_successful.load(Relaxed),
            validations_failed: self.metrics.validations_failed.load(Relaxed),
            consensus_validations: self.metrics.consensus_validations.load(Relaxed),
            byzantine_detections: self.metrics.byzantine_detections.load(Relaxed),
            alerts_generated: self.metrics.alerts_generated.load(Relaxed),
            certificate_consensus_required: self.metrics.certificate_consensus_required.load(Relaxed),
            certificate_consensus_approved: self.metrics.certificate_consensus_approved.load(Relaxed),
            average_validation_time_ms: self.metrics.average_validation_time_ms.load(Relaxed),
        };
        let recent_alerts = self.alert_manager.get_recent_alerts(10).await?;
        let recent_events = self.event_log.get_recent_events(20).await?;
        let byzantine_summary = self.byzantine_detector.get_detection_summary().await?;
        Ok(SecurityDashboard {
            metrics, recent_alerts, recent_events, byzantine_summary,
            consensus_status: self.get_consensus_status().await?,
            timestamp: SystemTime::now(),
        })
    }

    async fn get_consensus_status(&self) -> TrustChainResult<ConsensusStatus> {
        use std::sync::atomic::Ordering::Relaxed;
        let total_required = self.metrics.certificate_consensus_required.load(Relaxed);
        let total_approved = self.metrics.certificate_consensus_approved.load(Relaxed);
        let approval_rate = if total_required > 0 {
            (total_approved as f64 / total_required as f64) * 100.0
        } else { 100.0 };
        Ok(ConsensusStatus {
            enabled: self.config.mandatory_consensus,
            total_validations: self.metrics.consensus_validations.load(Relaxed),
            approval_rate,
            requirements: self.config.consensus_requirements.clone(),
        })
    }

    fn calculate_security_score(&self, consensus_result: &Option<ConsensusResult>, byzantine_result: &ByzantineDetectionResult) -> f64 {
        let mut score = 0.0;
        if let Some(result) = consensus_result {
            if result.is_valid() { score += 0.7; }
        } else if !self.config.mandatory_consensus { score += 0.3; }
        match byzantine_result {
            ByzantineDetectionResult::NotDetected => score += 0.3,
            ByzantineDetectionResult::Detected { confidence, .. } => {
                score += 0.3 * (1.0 - confidence);
            }
        }
        score.min(1.0).max(0.0)
    }

    async fn log_security_event(
        &self, event_type: String, severity: SecuritySeverity,
        description: String, consensus_proof: Option<ConsensusProof>,
    ) -> TrustChainResult<()> {
        let event = SecurityEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type: event_type.clone(), severity: severity.clone(),
            timestamp: SystemTime::now(), description,
            consensus_proof, metadata: HashMap::new(),
        };
        self.event_log.log_event(event).await?;
        debug!("Security event logged: {} ({})", event_type, severity);
        Ok(())
    }

    /// Get security metrics
    pub async fn get_metrics(&self) -> SecurityMetrics {
        self.metrics.as_ref().clone()
    }
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
            "issue_certificate", &consensus_proof, "test_validation",
        ).await.unwrap();
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
