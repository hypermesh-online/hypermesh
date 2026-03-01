// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Security types, configuration, and metrics

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;

use crate::consensus::{ConsensusProof, ConsensusRequirements, ConsensusResult};

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
            byzantine_threshold: 0.33,
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
    pub is_valid: bool,
    pub consensus_result: Option<ConsensusResult>,
    pub byzantine_detection: super::ByzantineDetectionResult,
    pub alerts: Vec<super::SecurityAlert>,
    pub validated_at: SystemTime,
    pub metrics: ValidationMetrics,
}

/// Validation metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationMetrics {
    pub consensus_time_ms: u64,
    pub byzantine_time_ms: u64,
    pub total_time_ms: u64,
    pub security_score: f64,
}

/// Security metrics collector
#[derive(Default)]
pub struct SecurityMetrics {
    pub validations_total: std::sync::atomic::AtomicU64,
    pub validations_successful: std::sync::atomic::AtomicU64,
    pub validations_failed: std::sync::atomic::AtomicU64,
    pub consensus_validations: std::sync::atomic::AtomicU64,
    pub byzantine_detections: std::sync::atomic::AtomicU64,
    pub alerts_generated: std::sync::atomic::AtomicU64,
    pub certificate_consensus_required: std::sync::atomic::AtomicU64,
    pub certificate_consensus_approved: std::sync::atomic::AtomicU64,
    pub average_validation_time_ms: std::sync::atomic::AtomicU64,
}

impl Clone for SecurityMetrics {
    fn clone(&self) -> Self {
        use std::sync::atomic::Ordering::Relaxed;
        Self {
            validations_total: std::sync::atomic::AtomicU64::new(
                self.validations_total.load(Relaxed),
            ),
            validations_successful: std::sync::atomic::AtomicU64::new(
                self.validations_successful.load(Relaxed),
            ),
            validations_failed: std::sync::atomic::AtomicU64::new(
                self.validations_failed.load(Relaxed),
            ),
            consensus_validations: std::sync::atomic::AtomicU64::new(
                self.consensus_validations.load(Relaxed),
            ),
            byzantine_detections: std::sync::atomic::AtomicU64::new(
                self.byzantine_detections.load(Relaxed),
            ),
            alerts_generated: std::sync::atomic::AtomicU64::new(
                self.alerts_generated.load(Relaxed),
            ),
            certificate_consensus_required: std::sync::atomic::AtomicU64::new(
                self.certificate_consensus_required.load(Relaxed),
            ),
            certificate_consensus_approved: std::sync::atomic::AtomicU64::new(
                self.certificate_consensus_approved.load(Relaxed),
            ),
            average_validation_time_ms: std::sync::atomic::AtomicU64::new(
                self.average_validation_time_ms.load(Relaxed),
            ),
        }
    }
}

impl std::fmt::Debug for SecurityMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::sync::atomic::Ordering::Relaxed;
        f.debug_struct("SecurityMetrics")
            .field("validations_total", &self.validations_total.load(Relaxed))
            .field(
                "validations_successful",
                &self.validations_successful.load(Relaxed),
            )
            .field("validations_failed", &self.validations_failed.load(Relaxed))
            .field(
                "consensus_validations",
                &self.consensus_validations.load(Relaxed),
            )
            .field(
                "byzantine_detections",
                &self.byzantine_detections.load(Relaxed),
            )
            .field("alerts_generated", &self.alerts_generated.load(Relaxed))
            .field(
                "certificate_consensus_required",
                &self.certificate_consensus_required.load(Relaxed),
            )
            .field(
                "certificate_consensus_approved",
                &self.certificate_consensus_approved.load(Relaxed),
            )
            .field(
                "average_validation_time_ms",
                &self.average_validation_time_ms.load(Relaxed),
            )
            .finish()
    }
}

impl Serialize for SecurityMetrics {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        use std::sync::atomic::Ordering::Relaxed;
        let mut state = serializer.serialize_struct("SecurityMetrics", 9)?;
        state.serialize_field("validations_total", &self.validations_total.load(Relaxed))?;
        state.serialize_field(
            "validations_successful",
            &self.validations_successful.load(Relaxed),
        )?;
        state.serialize_field("validations_failed", &self.validations_failed.load(Relaxed))?;
        state.serialize_field(
            "consensus_validations",
            &self.consensus_validations.load(Relaxed),
        )?;
        state.serialize_field(
            "byzantine_detections",
            &self.byzantine_detections.load(Relaxed),
        )?;
        state.serialize_field("alerts_generated", &self.alerts_generated.load(Relaxed))?;
        state.serialize_field(
            "certificate_consensus_required",
            &self.certificate_consensus_required.load(Relaxed),
        )?;
        state.serialize_field(
            "certificate_consensus_approved",
            &self.certificate_consensus_approved.load(Relaxed),
        )?;
        state.serialize_field(
            "average_validation_time_ms",
            &self.average_validation_time_ms.load(Relaxed),
        )?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for SecurityMetrics {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Data {
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
        let data = Data::deserialize(deserializer)?;
        Ok(Self {
            validations_total: std::sync::atomic::AtomicU64::new(data.validations_total),
            validations_successful: std::sync::atomic::AtomicU64::new(data.validations_successful),
            validations_failed: std::sync::atomic::AtomicU64::new(data.validations_failed),
            consensus_validations: std::sync::atomic::AtomicU64::new(data.consensus_validations),
            byzantine_detections: std::sync::atomic::AtomicU64::new(data.byzantine_detections),
            alerts_generated: std::sync::atomic::AtomicU64::new(data.alerts_generated),
            certificate_consensus_required: std::sync::atomic::AtomicU64::new(
                data.certificate_consensus_required,
            ),
            certificate_consensus_approved: std::sync::atomic::AtomicU64::new(
                data.certificate_consensus_approved,
            ),
            average_validation_time_ms: std::sync::atomic::AtomicU64::new(
                data.average_validation_time_ms,
            ),
        })
    }
}

/// Security event log
pub struct SecurityEventLog {
    events: Arc<DashMap<String, SecurityEvent>>,
    indices: Arc<RwLock<SecurityEventIndices>>,
}

/// Security event indices
#[derive(Default)]
pub struct SecurityEventIndices {
    pub by_timestamp: Vec<String>,
    pub by_severity: HashMap<SecuritySeverity, Vec<String>>,
    pub by_type: HashMap<String, Vec<String>>,
}

/// Security event
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub event_id: String,
    pub event_type: String,
    pub severity: SecuritySeverity,
    pub timestamp: SystemTime,
    pub description: String,
    pub consensus_proof: Option<ConsensusProof>,
    pub metadata: HashMap<String, String>,
}

/// Security dashboard data structure
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityDashboard {
    pub metrics: SecurityDashboardMetrics,
    pub recent_alerts: Vec<super::SecurityAlert>,
    pub recent_events: Vec<SecurityEvent>,
    pub byzantine_summary: super::ByzantineDetectionSummary,
    pub consensus_status: ConsensusStatus,
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
    pub enabled: bool,
    pub total_validations: u64,
    pub approval_rate: f64,
    pub requirements: ConsensusRequirements,
}

impl SecurityEventLog {
    /// Create new security event log
    pub async fn new() -> crate::errors::Result<Self> {
        Ok(Self {
            events: Arc::new(DashMap::new()),
            indices: Arc::new(RwLock::new(SecurityEventIndices::default())),
        })
    }

    /// Log security event
    pub async fn log_event(&self, event: SecurityEvent) -> crate::errors::Result<()> {
        let event_id = event.event_id.clone();
        self.events.insert(event_id.clone(), event.clone());
        {
            let mut indices = self.indices.write().await;
            indices.by_timestamp.push(event_id.clone());
            indices.by_timestamp.sort_by(|a, b| {
                let event_a = self.events.get(a).map(|e| e.timestamp);
                let event_b = self.events.get(b).map(|e| e.timestamp);
                event_b.cmp(&event_a)
            });
            indices
                .by_severity
                .entry(event.severity.clone())
                .or_insert_with(Vec::new)
                .push(event_id.clone());
            indices
                .by_type
                .entry(event.event_type.clone())
                .or_insert_with(Vec::new)
                .push(event_id.clone());
        }
        Ok(())
    }

    /// Get recent events
    pub async fn get_recent_events(
        &self,
        limit: usize,
    ) -> crate::errors::Result<Vec<SecurityEvent>> {
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
