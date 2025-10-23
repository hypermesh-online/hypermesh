//! Health alerting components for HyperMesh runtime
//!
//! This module contains components for managing health alerts, notifications,
//! and escalation procedures.

use crate::{Result, RuntimeError};
use crate::health::{
    HealthAlert, AlertSeverity, SystemHealthStatus, ComponentHealth,
    EscalationConfig, EscalationLevel
};
use nexus_shared::{NodeId, Timestamp};

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};
use tokio::sync::{mpsc, Mutex, watch};
use tokio::time::{interval, sleep};
use tracing::{debug, info, warn, error, instrument};

/// Alert manager for handling health alerts and notifications
#[derive(Debug, Clone)]
pub struct AlertManager {
    node_id: NodeId,
    active_alerts: Arc<RwLock<HashMap<String, ActiveAlert>>>,
    alert_history: Arc<RwLock<VecDeque<HealthAlert>>>,
    escalation_config: EscalationConfig,
    notification_sender: mpsc::UnboundedSender<NotificationEvent>,
    alert_suppression: Arc<RwLock<HashMap<String, AlertSuppression>>>,
}

/// Active alert with tracking information
#[derive(Debug, Clone)]
struct ActiveAlert {
    alert: HealthAlert,
    first_occurrence: Timestamp,
    last_occurrence: Timestamp,
    occurrence_count: usize,
    escalation_level: usize,
    last_escalation: Option<Timestamp>,
    suppressed: bool,
}

/// Alert suppression rule
#[derive(Debug, Clone)]
struct AlertSuppression {
    alert_pattern: String,
    suppressed_until: Timestamp,
    reason: String,
}

/// Notification event for external systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationEvent {
    NewAlert(HealthAlert),
    AlertResolved(String),
    AlertEscalated { alert_id: String, level: usize },
    AlertSuppressed { alert_id: String, reason: String },
}

/// Alert statistics and metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertStats {
    pub total_alerts: usize,
    pub active_alerts: usize,
    pub resolved_alerts: usize,
    pub suppressed_alerts: usize,
    pub escalated_alerts: usize,
    pub alerts_by_severity: HashMap<AlertSeverity, usize>,
    pub alerts_by_component: HashMap<String, usize>,
    pub average_resolution_time: Duration,
}

impl AlertManager {
    /// Create a new alert manager
    pub fn new(node_id: NodeId, escalation_config: EscalationConfig) -> (Self, mpsc::UnboundedReceiver<NotificationEvent>) {
        let (notification_sender, notification_receiver) = mpsc::unbounded_channel();
        
        (Self {
            node_id,
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(VecDeque::new())),
            escalation_config,
            notification_sender,
            alert_suppression: Arc::new(RwLock::new(HashMap::new())),
        }, notification_receiver)
    }

    /// Start the alert manager background processing
    #[instrument(skip(self))]
    pub async fn start(&self) -> Result<()> {
        let active_alerts = Arc::clone(&self.active_alerts);
        let escalation_config = self.escalation_config.clone();
        let notification_sender = self.notification_sender.clone();
        let suppression = Arc::clone(&self.alert_suppression);

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                if let Err(e) = Self::process_escalations(&active_alerts, &escalation_config, &notification_sender).await {
                    error!("Error processing alert escalations: {}", e);
                }
                
                if let Err(e) = Self::cleanup_suppressed_alerts(&suppression).await {
                    error!("Error cleaning up suppressed alerts: {}", e);
                }
            }
        });

        info!(node_id = %self.node_id, "Alert manager started");
        Ok(())
    }

    /// Process a new health alert
    #[instrument(skip(self, alert))]
    pub async fn process_alert(&self, alert: HealthAlert) -> Result<()> {
        // Check if alert should be suppressed
        if self.should_suppress_alert(&alert).await? {
            debug!(alert_id = %alert.id, "Alert suppressed");
            return Ok(());
        }

        let alert_id = alert.id.clone();
        let now = SystemTime::now().into();
        
        // Check if this is a duplicate alert
        let mut active_alerts = self.active_alerts.write()
            .map_err(|e| RuntimeError::LockPoisoned(format!("Active alerts: {}", e)))?;
        
        if let Some(existing_alert) = active_alerts.get_mut(&alert_id) {
            // Update existing alert
            existing_alert.last_occurrence = now;
            existing_alert.occurrence_count += 1;
            
            debug!(
                alert_id = %alert_id,
                occurrence_count = existing_alert.occurrence_count,
                "Updated existing alert"
            );
        } else {
            // Create new active alert
            let active_alert = ActiveAlert {
                alert: alert.clone(),
                first_occurrence: now,
                last_occurrence: now,
                occurrence_count: 1,
                escalation_level: 0,
                last_escalation: None,
                suppressed: false,
            };
            
            active_alerts.insert(alert_id.clone(), active_alert);
            
            // Send notification for new alert
            if let Err(e) = self.notification_sender.send(NotificationEvent::NewAlert(alert.clone())) {
                error!(alert_id = %alert_id, "Failed to send new alert notification: {}", e);
            }
            
            info!(
                alert_id = %alert_id,
                severity = ?alert.severity,
                component = %alert.component,
                "New alert registered"
            );
        }

        // Add to history
        {
            let mut history = self.alert_history.write()
                .map_err(|e| RuntimeError::LockPoisoned(format!("Alert history: {}", e)))?;
            history.push_back(alert);
            
            // Maintain history size limit
            while history.len() > self.escalation_config.history_retention_size {
                history.pop_front();
            }
        }

        Ok(())
    }

    /// Resolve an active alert
    #[instrument(skip(self))]
    pub async fn resolve_alert(&self, alert_id: &str) -> Result<bool> {
        let mut active_alerts = self.active_alerts.write()
            .map_err(|e| RuntimeError::LockPoisoned(format!("Active alerts: {}", e)))?;
        
        if let Some(resolved_alert) = active_alerts.remove(alert_id) {
            // Send resolution notification
            if let Err(e) = self.notification_sender.send(NotificationEvent::AlertResolved(alert_id.to_string())) {
                error!(alert_id = %alert_id, "Failed to send alert resolution notification: {}", e);
            }
            
            info!(
                alert_id = %alert_id,
                duration_active = ?SystemTime::now().duration_since(SystemTime::from(resolved_alert.first_occurrence)).unwrap_or_default(),
                occurrence_count = resolved_alert.occurrence_count,
                "Alert resolved"
            );
            
            Ok(true)
        } else {
            warn!(alert_id = %alert_id, "Attempted to resolve non-existent alert");
            Ok(false)
        }
    }

    /// Suppress alerts matching a pattern
    #[instrument(skip(self))]
    pub async fn suppress_alerts(&self, pattern: &str, duration: Duration, reason: &str) -> Result<usize> {
        let suppression = AlertSuppression {
            alert_pattern: pattern.to_string(),
            suppressed_until: (SystemTime::now() + duration).into(),
            reason: reason.to_string(),
        };

        let mut suppression_rules = self.alert_suppression.write()
            .map_err(|e| RuntimeError::LockPoisoned(format!("Alert suppression: {}", e)))?;
        suppression_rules.insert(pattern.to_string(), suppression);

        // Count how many active alerts this affects
        let active_alerts = self.active_alerts.read()
            .map_err(|e| RuntimeError::LockPoisoned(format!("Active alerts: {}", e)))?;
        
        let affected_count = active_alerts.values()
            .filter(|alert| self.alert_matches_pattern(&alert.alert, pattern))
            .count();

        info!(
            pattern = %pattern,
            duration = ?duration,
            affected_count = affected_count,
            reason = %reason,
            "Alert suppression rule added"
        );

        Ok(affected_count)
    }

    /// Check if alert should be suppressed
    async fn should_suppress_alert(&self, alert: &HealthAlert) -> Result<bool> {
        let suppression_rules = self.alert_suppression.read()
            .map_err(|e| RuntimeError::LockPoisoned(format!("Alert suppression: {}", e)))?;
        
        let now = SystemTime::now().into();
        
        for suppression in suppression_rules.values() {
            if suppression.suppressed_until > now && self.alert_matches_pattern(alert, &suppression.alert_pattern) {
                return Ok(true);
            }
        }
        
        Ok(false)
    }

    /// Check if alert matches suppression pattern
    fn alert_matches_pattern(&self, alert: &HealthAlert, pattern: &str) -> bool {
        // Simple pattern matching - could be extended with regex
        alert.id.contains(pattern) || 
        alert.component.contains(pattern) ||
        alert.message.contains(pattern)
    }

    /// Process alert escalations
    async fn process_escalations(
        active_alerts: &Arc<RwLock<HashMap<String, ActiveAlert>>>,
        escalation_config: &EscalationConfig,
        notification_sender: &mpsc::UnboundedSender<NotificationEvent>,
    ) -> Result<()> {
        let now = SystemTime::now();
        let mut alerts_to_escalate = Vec::new();

        // Find alerts that need escalation
        {
            let active_guard = active_alerts.read()
                .map_err(|e| RuntimeError::LockPoisoned(format!("Active alerts: {}", e)))?;
            
            for (alert_id, active_alert) in active_guard.iter() {
                if active_alert.suppressed {
                    continue;
                }
                
                let time_since_last_escalation = if let Some(last_escalation) = active_alert.last_escalation {
                    now.duration_since(SystemTime::from(last_escalation)).unwrap_or_default()
                } else {
                    now.duration_since(SystemTime::from(active_alert.first_occurrence)).unwrap_or_default()
                };

                // Check if escalation is needed
                if let Some(level) = escalation_config.levels.get(active_alert.escalation_level) {
                    if time_since_last_escalation >= level.timeout {
                        alerts_to_escalate.push(alert_id.clone());
                    }
                }
            }
        }

        // Perform escalations
        for alert_id in alerts_to_escalate {
            if let Err(e) = Self::escalate_alert(&alert_id, active_alerts, escalation_config, notification_sender).await {
                error!(alert_id = %alert_id, "Failed to escalate alert: {}", e);
            }
        }

        Ok(())
    }

    /// Escalate a specific alert
    async fn escalate_alert(
        alert_id: &str,
        active_alerts: &Arc<RwLock<HashMap<String, ActiveAlert>>>,
        escalation_config: &EscalationConfig,
        notification_sender: &mpsc::UnboundedSender<NotificationEvent>,
    ) -> Result<()> {
        let mut active_guard = active_alerts.write()
            .map_err(|e| RuntimeError::LockPoisoned(format!("Active alerts: {}", e)))?;
        
        if let Some(active_alert) = active_guard.get_mut(alert_id) {
            let next_level = active_alert.escalation_level + 1;
            
            if next_level < escalation_config.levels.len() {
                active_alert.escalation_level = next_level;
                active_alert.last_escalation = Some(SystemTime::now().into());
                
                // Send escalation notification
                if let Err(e) = notification_sender.send(NotificationEvent::AlertEscalated {
                    alert_id: alert_id.to_string(),
                    level: next_level,
                }) {
                    error!(alert_id = %alert_id, "Failed to send escalation notification: {}", e);
                }
                
                warn!(
                    alert_id = %alert_id,
                    escalation_level = next_level,
                    "Alert escalated"
                );
            }
        }

        Ok(())
    }

    /// Clean up expired suppression rules
    async fn cleanup_suppressed_alerts(
        suppression: &Arc<RwLock<HashMap<String, AlertSuppression>>>,
    ) -> Result<()> {
        let now = SystemTime::now().into();
        let mut expired_patterns = Vec::new();

        // Find expired suppressions
        {
            let suppression_guard = suppression.read()
                .map_err(|e| RuntimeError::LockPoisoned(format!("Alert suppression: {}", e)))?;
            
            for (pattern, rule) in suppression_guard.iter() {
                if rule.suppressed_until <= now {
                    expired_patterns.push(pattern.clone());
                }
            }
        }

        // Remove expired suppressions
        if !expired_patterns.is_empty() {
            let mut suppression_guard = suppression.write()
                .map_err(|e| RuntimeError::LockPoisoned(format!("Alert suppression: {}", e)))?;
            
            for pattern in expired_patterns {
                suppression_guard.remove(&pattern);
                debug!(pattern = %pattern, "Removed expired alert suppression");
            }
        }

        Ok(())
    }

    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Result<Vec<HealthAlert>> {
        let active_alerts = self.active_alerts.read()
            .map_err(|e| RuntimeError::LockPoisoned(format!("Active alerts: {}", e)))?;
        
        Ok(active_alerts.values()
            .map(|active| active.alert.clone())
            .collect())
    }

    /// Get alert statistics
    pub async fn get_alert_stats(&self) -> Result<AlertStats> {
        let active_alerts = self.active_alerts.read()
            .map_err(|e| RuntimeError::LockPoisoned(format!("Active alerts: {}", e)))?;
        
        let history = self.alert_history.read()
            .map_err(|e| RuntimeError::LockPoisoned(format!("Alert history: {}", e)))?;

        let mut alerts_by_severity = HashMap::new();
        let mut alerts_by_component = HashMap::new();
        let mut total_resolution_time = Duration::default();
        let mut resolved_count = 0;

        // Process active alerts
        for active_alert in active_alerts.values() {
            *alerts_by_severity.entry(active_alert.alert.severity.clone()).or_insert(0) += 1;
            *alerts_by_component.entry(active_alert.alert.component.clone()).or_insert(0) += 1;
        }

        // Process historical alerts for resolution time calculation
        for alert in history.iter() {
            *alerts_by_severity.entry(alert.severity.clone()).or_insert(0) += 1;
            *alerts_by_component.entry(alert.component.clone()).or_insert(0) += 1;
        }

        let average_resolution_time = if resolved_count > 0 {
            total_resolution_time / resolved_count as u32
        } else {
            Duration::default()
        };

        Ok(AlertStats {
            total_alerts: active_alerts.len() + history.len(),
            active_alerts: active_alerts.len(),
            resolved_alerts: resolved_count,
            suppressed_alerts: 0, // TODO: Calculate suppressed alerts
            escalated_alerts: active_alerts.values()
                .filter(|a| a.escalation_level > 0)
                .count(),
            alerts_by_severity,
            alerts_by_component,
            average_resolution_time,
        })
    }

    /// Get alerts by severity level
    pub async fn get_alerts_by_severity(&self, severity: AlertSeverity) -> Result<Vec<HealthAlert>> {
        let active_alerts = self.active_alerts.read()
            .map_err(|e| RuntimeError::LockPoisoned(format!("Active alerts: {}", e)))?;
        
        Ok(active_alerts.values()
            .filter(|active| active.alert.severity == severity)
            .map(|active| active.alert.clone())
            .collect())
    }

    /// Get alerts for specific component
    pub async fn get_alerts_by_component(&self, component: &str) -> Result<Vec<HealthAlert>> {
        let active_alerts = self.active_alerts.read()
            .map_err(|e| RuntimeError::LockPoisoned(format!("Active alerts: {}", e)))?;
        
        Ok(active_alerts.values()
            .filter(|active| active.alert.component == component)
            .map(|active| active.alert.clone())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_alert_manager_creation() {
        let node_id = "test-node".into();
        let escalation_config = EscalationConfig {
            levels: vec![],
            history_retention_size: 1000,
        };
        
        let (alert_manager, _receiver) = AlertManager::new(node_id, escalation_config);
        
        let stats = alert_manager.get_alert_stats().await.unwrap();
        assert_eq!(stats.active_alerts, 0);
        assert_eq!(stats.total_alerts, 0);
    }

    #[tokio::test]
    async fn test_alert_processing() {
        let node_id = "test-node".into();
        let escalation_config = EscalationConfig {
            levels: vec![],
            history_retention_size: 1000,
        };
        
        let (alert_manager, _receiver) = AlertManager::new(node_id, escalation_config);
        
        let alert = HealthAlert {
            id: "test-alert".to_string(),
            severity: AlertSeverity::Warning,
            message: "Test alert message".to_string(),
            component: "test-component".to_string(),
            timestamp: SystemTime::now().into(),
            metadata: HashMap::new(),
        };
        
        alert_manager.process_alert(alert.clone()).await.unwrap();
        
        let active_alerts = alert_manager.get_active_alerts().await.unwrap();
        assert_eq!(active_alerts.len(), 1);
        assert_eq!(active_alerts[0].id, "test-alert");
    }

    #[tokio::test]
    async fn test_alert_resolution() {
        let node_id = "test-node".into();
        let escalation_config = EscalationConfig {
            levels: vec![],
            history_retention_size: 1000,
        };
        
        let (alert_manager, _receiver) = AlertManager::new(node_id, escalation_config);
        
        let alert = HealthAlert {
            id: "test-alert".to_string(),
            severity: AlertSeverity::Warning,
            message: "Test alert message".to_string(),
            component: "test-component".to_string(),
            timestamp: SystemTime::now().into(),
            metadata: HashMap::new(),
        };
        
        alert_manager.process_alert(alert).await.unwrap();
        
        let resolved = alert_manager.resolve_alert("test-alert").await.unwrap();
        assert!(resolved);
        
        let active_alerts = alert_manager.get_active_alerts().await.unwrap();
        assert_eq!(active_alerts.len(), 0);
    }

    #[tokio::test]
    async fn test_alert_suppression() {
        let node_id = "test-node".into();
        let escalation_config = EscalationConfig {
            levels: vec![],
            history_retention_size: 1000,
        };
        
        let (alert_manager, _receiver) = AlertManager::new(node_id, escalation_config);
        
        // Add suppression rule
        let affected = alert_manager.suppress_alerts(
            "test-component",
            Duration::from_secs(3600),
            "Testing suppression"
        ).await.unwrap();
        
        // Should be 0 since no alerts are active yet
        assert_eq!(affected, 0);
    }
}