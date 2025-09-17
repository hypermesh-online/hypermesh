//! Security Alert Management System
//! 
//! Real-time security alerts with consensus validation integration

use std::sync::Arc;
use std::time::{SystemTime, Duration};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

use crate::consensus::ConsensusProof;
use crate::errors::{TrustChainError, Result as TrustChainResult};
use super::SecuritySeverity;

/// Security alert manager
pub struct SecurityAlertManager {
    /// Alert threshold for triggering
    threshold: SecuritySeverity,
    /// Active alerts storage
    active_alerts: Arc<RwLock<HashMap<String, SecurityAlert>>>,
    /// Alert history for analysis
    alert_history: Arc<RwLock<Vec<SecurityAlert>>>,
    /// Alert statistics
    stats: Arc<RwLock<AlertStatistics>>,
    /// Configuration
    config: AlertConfig,
}

/// Security alert
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityAlert {
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
    /// Alert category
    pub category: AlertCategory,
    /// Alert status
    pub status: AlertStatus,
    /// Associated consensus proof (if any)
    pub consensus_proof: Option<ConsensusProof>,
    /// Associated operation ID (if any)
    pub operation_id: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    /// Auto-resolve timestamp (if applicable)
    pub auto_resolve_at: Option<SystemTime>,
}

/// Alert category
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AlertCategory {
    /// Consensus validation failures
    ConsensusValidation,
    /// Byzantine behavior detection
    ByzantineBehavior,
    /// Certificate operations
    CertificateOperations,
    /// System security
    SystemSecurity,
    /// Performance anomalies
    Performance,
    /// Configuration issues
    Configuration,
}

/// Alert status
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AlertStatus {
    /// Alert is active and requires attention
    Active,
    /// Alert has been acknowledged by operator
    Acknowledged,
    /// Alert has been resolved
    Resolved,
    /// Alert has been escalated
    Escalated,
    /// Alert auto-resolved
    AutoResolved,
}

/// Alert statistics
#[derive(Clone, Debug, Default)]
pub struct AlertStatistics {
    /// Total alerts generated
    pub total_alerts: u64,
    /// Alerts by severity
    pub by_severity: HashMap<String, u64>,
    /// Alerts by category
    pub by_category: HashMap<String, u64>,
    /// Average resolution time (seconds)
    pub avg_resolution_time: f64,
    /// False positive rate
    pub false_positive_rate: f64,
}

/// Alert configuration
#[derive(Clone, Debug)]
pub struct AlertConfig {
    /// Maximum active alerts
    pub max_active_alerts: usize,
    /// Alert history retention (hours)
    pub history_retention_hours: u32,
    /// Auto-resolve timeout for certain alerts (seconds)
    pub auto_resolve_timeout: u32,
    /// Enable alert aggregation
    pub enable_aggregation: bool,
    /// Alert rate limiting (alerts per minute)
    pub rate_limit: u32,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            max_active_alerts: 1000,
            history_retention_hours: 72, // 3 days
            auto_resolve_timeout: 3600, // 1 hour
            enable_aggregation: true,
            rate_limit: 60, // 60 alerts per minute max
        }
    }
}

/// Alert escalation rule
#[derive(Clone, Debug)]
pub struct EscalationRule {
    /// Severity threshold for escalation
    pub severity_threshold: SecuritySeverity,
    /// Time before escalation (minutes)
    pub escalation_time_minutes: u32,
    /// Maximum escalations
    pub max_escalations: u32,
}

impl SecurityAlertManager {
    /// Create new security alert manager
    pub async fn new(threshold: SecuritySeverity) -> TrustChainResult<Self> {
        info!("Initializing Security Alert Manager with threshold: {:?}", threshold);

        Ok(Self {
            threshold,
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(AlertStatistics::default())),
            config: AlertConfig::default(),
        })
    }

    /// Generate security alert
    pub async fn generate_alert(
        &self,
        severity: SecuritySeverity,
        title: String,
        description: String,
        consensus_proof: Option<ConsensusProof>,
    ) -> TrustChainResult<SecurityAlert> {
        // Check if alert meets threshold
        if severity < self.threshold {
            debug!("Alert below threshold, ignoring: {} ({:?})", title, severity);
            return Err(TrustChainError::SecurityError {
                message: "Alert below threshold".to_string(),
            });
        }

        // Check rate limiting
        if !self.check_rate_limit().await? {
            warn!("Alert rate limit exceeded, dropping alert: {}", title);
            return Err(TrustChainError::SecurityError {
                message: "Rate limit exceeded".to_string(),
            });
        }

        // Generate alert ID
        let alert_id = uuid::Uuid::new_v4().to_string();

        // Determine alert category based on content
        let category = self.categorize_alert(&title, &description, &consensus_proof);

        // Calculate auto-resolve time for certain alert types
        let auto_resolve_at = if matches!(category, AlertCategory::Performance) {
            Some(SystemTime::now() + Duration::from_secs(self.config.auto_resolve_timeout as u64))
        } else {
            None
        };

        let alert = SecurityAlert {
            alert_id: alert_id.clone(),
            severity: severity.clone(),
            title: title.clone(),
            description,
            timestamp: SystemTime::now(),
            category,
            status: AlertStatus::Active,
            consensus_proof,
            operation_id: None, // Can be set later
            metadata: HashMap::new(),
            auto_resolve_at,
        };

        // Store alert
        {
            let mut active_alerts = self.active_alerts.write().await;
            
            // Check max active alerts limit
            if active_alerts.len() >= self.config.max_active_alerts {
                warn!("Maximum active alerts reached, removing oldest");
                self.cleanup_oldest_alerts().await?;
            }
            
            active_alerts.insert(alert_id.clone(), alert.clone());
        }

        // Add to history
        {
            let mut history = self.alert_history.write().await;
            history.push(alert.clone());
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_alerts += 1;
            
            let severity_str = format!("{:?}", severity);
            *stats.by_severity.entry(severity_str).or_insert(0) += 1;
            
            let category_str = format!("{:?}", alert.category);
            *stats.by_category.entry(category_str).or_insert(0) += 1;
        }

        // Log alert based on severity
        match severity {
            SecuritySeverity::Critical => {
                error!("CRITICAL SECURITY ALERT: {} - {}", title, alert.description);
            }
            SecuritySeverity::High => {
                error!("HIGH SECURITY ALERT: {} - {}", title, alert.description);
            }
            SecuritySeverity::Medium => {
                warn!("MEDIUM SECURITY ALERT: {} - {}", title, alert.description);
            }
            SecuritySeverity::Low => {
                info!("LOW SECURITY ALERT: {} - {}", title, alert.description);
            }
        }

        // Handle escalation if needed
        if matches!(severity, SecuritySeverity::Critical | SecuritySeverity::High) {
            self.handle_escalation(&alert).await?;
        }

        info!("Security alert generated: {} ({})", alert_id, severity);
        Ok(alert)
    }

    /// Acknowledge alert
    pub async fn acknowledge_alert(&self, alert_id: &str, operator: &str) -> TrustChainResult<()> {
        let mut active_alerts = self.active_alerts.write().await;
        
        if let Some(alert) = active_alerts.get_mut(alert_id) {
            alert.status = AlertStatus::Acknowledged;
            alert.metadata.insert("acknowledged_by".to_string(), operator.to_string());
            alert.metadata.insert("acknowledged_at".to_string(), SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs().to_string());
            
            info!("Alert acknowledged: {} by {}", alert_id, operator);
            Ok(())
        } else {
            Err(TrustChainError::SecurityError {
                message: format!("Alert not found: {}", alert_id),
            })
        }
    }

    /// Resolve alert
    pub async fn resolve_alert(&self, alert_id: &str, resolution_note: &str) -> TrustChainResult<()> {
        let mut active_alerts = self.active_alerts.write().await;
        
        if let Some(alert) = active_alerts.get_mut(alert_id) {
            let resolution_time = alert.timestamp.elapsed().unwrap_or(Duration::from_secs(0));
            
            alert.status = AlertStatus::Resolved;
            alert.metadata.insert("resolution_note".to_string(), resolution_note.to_string());
            alert.metadata.insert("resolved_at".to_string(), SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs().to_string());
            alert.metadata.insert("resolution_time_seconds".to_string(), resolution_time.as_secs().to_string());
            
            // Update resolution time statistics
            {
                let mut stats = self.stats.write().await;
                let current_avg = stats.avg_resolution_time;
                let new_time = resolution_time.as_secs_f64();
                stats.avg_resolution_time = if current_avg == 0.0 {
                    new_time
                } else {
                    (current_avg + new_time) / 2.0
                };
            }
            
            info!("Alert resolved: {} in {:.2}s", alert_id, resolution_time.as_secs_f64());
            Ok(())
        } else {
            Err(TrustChainError::SecurityError {
                message: format!("Alert not found: {}", alert_id),
            })
        }
    }

    /// Get recent alerts
    pub async fn get_recent_alerts(&self, limit: usize) -> TrustChainResult<Vec<SecurityAlert>> {
        let history = self.alert_history.read().await;
        
        let mut recent_alerts: Vec<_> = history.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect();
        
        // Sort by timestamp (newest first)
        recent_alerts.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        Ok(recent_alerts)
    }

    /// Get active alerts
    pub async fn get_active_alerts(&self) -> TrustChainResult<Vec<SecurityAlert>> {
        let active_alerts = self.active_alerts.read().await;
        let mut alerts: Vec<_> = active_alerts.values().cloned().collect();
        
        // Sort by severity (critical first) then by timestamp
        alerts.sort_by(|a, b| {
            match b.severity.partial_cmp(&a.severity) {
                Some(std::cmp::Ordering::Equal) => b.timestamp.cmp(&a.timestamp),
                Some(ordering) => ordering,
                None => std::cmp::Ordering::Equal,
            }
        });
        
        Ok(alerts)
    }

    /// Get alert statistics
    pub async fn get_statistics(&self) -> AlertStatistics {
        self.stats.read().await.clone()
    }

    /// Process auto-resolve alerts
    pub async fn process_auto_resolve(&self) -> TrustChainResult<()> {
        let mut active_alerts = self.active_alerts.write().await;
        let now = SystemTime::now();
        
        let mut alerts_to_resolve = Vec::new();
        
        for (alert_id, alert) in active_alerts.iter() {
            if let Some(auto_resolve_at) = alert.auto_resolve_at {
                if now >= auto_resolve_at && matches!(alert.status, AlertStatus::Active) {
                    alerts_to_resolve.push(alert_id.clone());
                }
            }
        }
        
        for alert_id in alerts_to_resolve {
            if let Some(alert) = active_alerts.get_mut(&alert_id) {
                alert.status = AlertStatus::AutoResolved;
                alert.metadata.insert("auto_resolved_at".to_string(), now.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs().to_string());
                debug!("Auto-resolved alert: {}", alert_id);
            }
        }
        
        Ok(())
    }

    /// Cleanup old alerts from history
    pub async fn cleanup_old_alerts(&self) -> TrustChainResult<()> {
        let cutoff_time = SystemTime::now() - Duration::from_secs(self.config.history_retention_hours as u64 * 3600);
        
        {
            let mut history = self.alert_history.write().await;
            let original_len = history.len();
            history.retain(|alert| alert.timestamp > cutoff_time);
            let removed = original_len - history.len();
            
            if removed > 0 {
                debug!("Cleaned up {} old alerts from history", removed);
            }
        }
        
        // Also cleanup resolved active alerts older than cutoff
        {
            let mut active_alerts = self.active_alerts.write().await;
            let original_len = active_alerts.len();
            active_alerts.retain(|_, alert| {
                match alert.status {
                    AlertStatus::Resolved | AlertStatus::AutoResolved => alert.timestamp > cutoff_time,
                    _ => true, // Keep active, acknowledged, escalated alerts
                }
            });
            let removed = original_len - active_alerts.len();
            
            if removed > 0 {
                debug!("Cleaned up {} old active alerts", removed);
            }
        }
        
        Ok(())
    }

    // Internal helper methods

    /// Check rate limiting
    async fn check_rate_limit(&self) -> TrustChainResult<bool> {
        // Simple rate limiting based on recent alerts
        let recent_cutoff = SystemTime::now() - Duration::from_secs(60); // Last minute
        
        let history = self.alert_history.read().await;
        let recent_count = history.iter()
            .rev()
            .take_while(|alert| alert.timestamp > recent_cutoff)
            .count();
        
        Ok(recent_count < self.config.rate_limit as usize)
    }

    /// Categorize alert based on content
    fn categorize_alert(&self, title: &str, description: &str, consensus_proof: &Option<ConsensusProof>) -> AlertCategory {
        let content = format!("{} {}", title.to_lowercase(), description.to_lowercase());
        
        if content.contains("consensus") || content.contains("proof") || consensus_proof.is_some() {
            AlertCategory::ConsensusValidation
        } else if content.contains("byzantine") || content.contains("malicious") {
            AlertCategory::ByzantineBehavior
        } else if content.contains("certificate") || content.contains("ca") || content.contains("ct") {
            AlertCategory::CertificateOperations
        } else if content.contains("performance") || content.contains("slow") || content.contains("timeout") {
            AlertCategory::Performance
        } else if content.contains("config") || content.contains("setting") {
            AlertCategory::Configuration
        } else {
            AlertCategory::SystemSecurity
        }
    }

    /// Handle alert escalation
    async fn handle_escalation(&self, alert: &SecurityAlert) -> TrustChainResult<()> {
        // In production, this would integrate with external alerting systems
        // For now, we just log the escalation
        
        match alert.severity {
            SecuritySeverity::Critical => {
                error!("ESCALATING CRITICAL ALERT: {} - Immediate attention required!", alert.title);
                // Would send to pager, email, Slack, etc.
            }
            SecuritySeverity::High => {
                warn!("ESCALATING HIGH SEVERITY ALERT: {} - Requires prompt attention", alert.title);
                // Would send email notification
            }
            _ => {
                // No escalation for medium/low severity
            }
        }
        
        Ok(())
    }

    /// Cleanup oldest alerts when limit is reached
    async fn cleanup_oldest_alerts(&self) -> TrustChainResult<()> {
        let mut active_alerts = self.active_alerts.write().await;
        
        // Find oldest resolved/auto-resolved alert
        let mut oldest_resolved = None;
        let mut oldest_timestamp = SystemTime::now();
        
        for (alert_id, alert) in active_alerts.iter() {
            if matches!(alert.status, AlertStatus::Resolved | AlertStatus::AutoResolved) &&
               alert.timestamp < oldest_timestamp {
                oldest_timestamp = alert.timestamp;
                oldest_resolved = Some(alert_id.clone());
            }
        }
        
        if let Some(alert_id) = oldest_resolved {
            active_alerts.remove(&alert_id);
            debug!("Removed oldest resolved alert to make room: {}", alert_id);
        } else {
            // If no resolved alerts, remove oldest acknowledged
            let mut oldest_acknowledged = None;
            oldest_timestamp = SystemTime::now();
            
            for (alert_id, alert) in active_alerts.iter() {
                if matches!(alert.status, AlertStatus::Acknowledged) &&
                   alert.timestamp < oldest_timestamp {
                    oldest_timestamp = alert.timestamp;
                    oldest_acknowledged = Some(alert_id.clone());
                }
            }
            
            if let Some(alert_id) = oldest_acknowledged {
                active_alerts.remove(&alert_id);
                debug!("Removed oldest acknowledged alert to make room: {}", alert_id);
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_alert_manager_creation() {
        let manager = SecurityAlertManager::new(SecuritySeverity::Medium).await.unwrap();
        assert_eq!(manager.threshold, SecuritySeverity::Medium);
    }

    #[tokio::test]
    async fn test_generate_alert() {
        let manager = SecurityAlertManager::new(SecuritySeverity::Low).await.unwrap();
        
        let alert = manager.generate_alert(
            SecuritySeverity::High,
            "Test Alert".to_string(),
            "This is a test alert".to_string(),
            None,
        ).await.unwrap();
        
        assert_eq!(alert.title, "Test Alert");
        assert_eq!(alert.severity, SecuritySeverity::High);
        assert!(matches!(alert.status, AlertStatus::Active));
    }

    #[tokio::test]
    async fn test_alert_below_threshold() {
        let manager = SecurityAlertManager::new(SecuritySeverity::High).await.unwrap();
        
        let result = manager.generate_alert(
            SecuritySeverity::Low,
            "Low Priority Alert".to_string(),
            "This should be ignored".to_string(),
            None,
        ).await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_acknowledge_alert() {
        let manager = SecurityAlertManager::new(SecuritySeverity::Low).await.unwrap();
        
        let alert = manager.generate_alert(
            SecuritySeverity::Medium,
            "Test Alert".to_string(),
            "Test description".to_string(),
            None,
        ).await.unwrap();
        
        manager.acknowledge_alert(&alert.alert_id, "test_operator").await.unwrap();
        
        let active_alerts = manager.get_active_alerts().await.unwrap();
        assert_eq!(active_alerts.len(), 1);
        assert!(matches!(active_alerts[0].status, AlertStatus::Acknowledged));
    }

    #[tokio::test]
    async fn test_resolve_alert() {
        let manager = SecurityAlertManager::new(SecuritySeverity::Low).await.unwrap();
        
        let alert = manager.generate_alert(
            SecuritySeverity::Medium,
            "Test Alert".to_string(),
            "Test description".to_string(),
            None,
        ).await.unwrap();
        
        manager.resolve_alert(&alert.alert_id, "Issue resolved").await.unwrap();
        
        let active_alerts = manager.get_active_alerts().await.unwrap();
        assert_eq!(active_alerts.len(), 1);
        assert!(matches!(active_alerts[0].status, AlertStatus::Resolved));
    }

    #[tokio::test]
    async fn test_get_recent_alerts() {
        let manager = SecurityAlertManager::new(SecuritySeverity::Low).await.unwrap();
        
        // Generate multiple alerts
        for i in 0..5 {
            manager.generate_alert(
                SecuritySeverity::Medium,
                format!("Test Alert {}", i),
                "Test description".to_string(),
                None,
            ).await.unwrap();
        }
        
        let recent_alerts = manager.get_recent_alerts(3).await.unwrap();
        assert_eq!(recent_alerts.len(), 3);
    }

    #[tokio::test]
    async fn test_alert_categorization() {
        let manager = SecurityAlertManager::new(SecuritySeverity::Low).await.unwrap();
        
        let alert = manager.generate_alert(
            SecuritySeverity::High,
            "Consensus Validation Failed".to_string(),
            "Four-proof consensus validation failed".to_string(),
            None,
        ).await.unwrap();
        
        assert!(matches!(alert.category, AlertCategory::ConsensusValidation));
    }
}