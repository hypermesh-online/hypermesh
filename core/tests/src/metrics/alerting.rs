//! Alerting system for monitoring and notifications

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

pub struct AlertingSystem {
    alerts: Arc<RwLock<HashMap<String, Alert>>>,
    rules: Arc<RwLock<Vec<AlertRule>>>,
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl AlertingSystem {
    pub fn new() -> Self {
        let mut rules = Vec::new();
        
        // Define default alert rules
        rules.push(AlertRule {
            id: "high_cpu_usage".to_string(),
            name: "High CPU Usage".to_string(),
            condition: AlertCondition::MetricThreshold {
                metric_name: "cpu_usage_percent".to_string(),
                operator: ThresholdOperator::GreaterThan,
                value: 80.0,
                duration_seconds: 300, // 5 minutes
            },
            severity: AlertSeverity::Warning,
            component: "runtime".to_string(),
            enabled: true,
        });
        
        rules.push(AlertRule {
            id: "high_memory_usage".to_string(),
            name: "High Memory Usage".to_string(),
            condition: AlertCondition::MetricThreshold {
                metric_name: "memory_utilization_percent".to_string(),
                operator: ThresholdOperator::GreaterThan,
                value: 90.0,
                duration_seconds: 180, // 3 minutes
            },
            severity: AlertSeverity::Critical,
            component: "runtime".to_string(),
            enabled: true,
        });
        
        rules.push(AlertRule {
            id: "consensus_high_latency".to_string(),
            name: "Consensus High Latency".to_string(),
            condition: AlertCondition::HistogramThreshold {
                metric_name: "consensus_commit_latency_ms".to_string(),
                percentile: 95.0,
                value: 100.0,
                duration_seconds: 300,
            },
            severity: AlertSeverity::Warning,
            component: "consensus".to_string(),
            enabled: true,
        });
        
        rules.push(AlertRule {
            id: "network_partition".to_string(),
            name: "Network Partition Detected".to_string(),
            condition: AlertCondition::MetricThreshold {
                metric_name: "consensus_cluster_nodes_healthy".to_string(),
                operator: ThresholdOperator::LessThan,
                value: 3.0, // Less than quorum
                duration_seconds: 60,
            },
            severity: AlertSeverity::Critical,
            component: "consensus".to_string(),
            enabled: true,
        });
        
        rules.push(AlertRule {
            id: "ebpf_program_slow".to_string(),
            name: "eBPF Program Slow Execution".to_string(),
            condition: AlertCondition::HistogramThreshold {
                metric_name: "ebpf_program_execution_time_ns".to_string(),
                percentile: 99.0,
                value: 10000.0, // 10μs
                duration_seconds: 600,
            },
            severity: AlertSeverity::Warning,
            component: "ebpf".to_string(),
            enabled: true,
        });
        
        Self {
            alerts: Arc::new(RwLock::new(HashMap::new())),
            rules: Arc::new(RwLock::new(rules)),
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
    
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.running.store(true, std::sync::atomic::Ordering::Relaxed);
        
        // Start alert monitoring loop
        let alerts = self.alerts.clone();
        let rules = self.rules.clone();
        let running = self.running.clone();
        
        tokio::spawn(async move {
            while running.load(std::sync::atomic::Ordering::Relaxed) {
                // In real implementation, this would:
                // 1. Query metrics from MetricsStorage
                // 2. Evaluate alert rules
                // 3. Trigger/resolve alerts
                // 4. Send notifications
                
                // Simulate alert evaluation
                if let Err(e) = Self::evaluate_alerts(&alerts, &rules).await {
                    tracing::error!("Alert evaluation failed: {}", e);
                }
                
                tokio::time::sleep(Duration::from_secs(30)).await; // Check every 30 seconds
            }
        });
        
        tracing::info!("✅ Alerting system started");
        Ok(())
    }
    
    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.running.store(false, std::sync::atomic::Ordering::Relaxed);
        tracing::info!("Alerting system stopped");
        Ok(())
    }
    
    async fn evaluate_alerts(
        alerts: &Arc<RwLock<HashMap<String, Alert>>>,
        rules: &Arc<RwLock<Vec<AlertRule>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let rules_guard = rules.read().await;
        
        for rule in rules_guard.iter() {
            if !rule.enabled {
                continue;
            }
            
            // Simulate rule evaluation
            let should_trigger = Self::simulate_rule_evaluation(rule).await;
            
            let mut alerts_guard = alerts.write().await;
            
            match alerts_guard.get(&rule.id) {
                Some(existing_alert) => {
                    if !should_trigger && !existing_alert.resolved {
                        // Resolve the alert
                        let mut resolved_alert = existing_alert.clone();
                        resolved_alert.resolved = true;
                        resolved_alert.resolved_at = Some(SystemTime::now());
                        alerts_guard.insert(rule.id.clone(), resolved_alert);
                        
                        tracing::info!("🔔 Alert resolved: {}", rule.name);
                    }
                },
                None => {
                    if should_trigger {
                        // Create new alert
                        let alert = Alert {
                            id: format!("alert-{}-{}", rule.id, 
                                      SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?.as_secs()),
                            rule_id: rule.id.clone(),
                            severity: rule.severity.clone(),
                            component: rule.component.clone(),
                            message: Self::generate_alert_message(rule),
                            timestamp: SystemTime::now(),
                            resolved: false,
                            resolved_at: None,
                        };
                        
                        alerts_guard.insert(rule.id.clone(), alert.clone());
                        tracing::warn!("🚨 Alert triggered: {}", alert.message);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    async fn simulate_rule_evaluation(rule: &AlertRule) -> bool {
        // Simulate rule evaluation with some randomness
        match &rule.condition {
            AlertCondition::MetricThreshold { value, .. } => {
                // Simulate threshold evaluation
                let current_value = rand::random::<f64>() * 100.0;
                current_value > *value
            },
            AlertCondition::HistogramThreshold { value, .. } => {
                // Simulate histogram percentile evaluation
                let current_percentile_value = rand::random::<f64>() * (*value * 2.0);
                current_percentile_value > *value
            },
        }
    }
    
    fn generate_alert_message(rule: &AlertRule) -> String {
        match &rule.condition {
            AlertCondition::MetricThreshold { metric_name, operator, value, .. } => {
                format!("{} {} {:.2} for {}", 
                       metric_name, 
                       match operator {
                           ThresholdOperator::GreaterThan => "is greater than",
                           ThresholdOperator::LessThan => "is less than",
                           ThresholdOperator::Equals => "equals",
                       },
                       value,
                       rule.component)
            },
            AlertCondition::HistogramThreshold { metric_name, percentile, value, .. } => {
                format!("P{} of {} exceeds {:.2} for {}", 
                       percentile, metric_name, value, rule.component)
            },
        }
    }
    
    pub async fn trigger_alert(&self, alert: Alert) -> Result<(), Box<dyn std::error::Error>> {
        let mut alerts_guard = self.alerts.write().await;
        alerts_guard.insert(alert.id.clone(), alert.clone());
        
        // In real implementation, would send notifications here
        tracing::info!("🚨 Manual alert triggered: {}", alert.message);
        Ok(())
    }
    
    pub async fn resolve_alert(&self, alert_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut alerts_guard = self.alerts.write().await;
        
        if let Some(alert) = alerts_guard.get_mut(alert_id) {
            alert.resolved = true;
            alert.resolved_at = Some(SystemTime::now());
            tracing::info!("🔔 Alert resolved: {}", alert.message);
        }
        
        Ok(())
    }
    
    pub async fn get_active_alerts(&self) -> Result<Vec<Alert>, Box<dyn std::error::Error>> {
        let alerts_guard = self.alerts.read().await;
        let active_alerts = alerts_guard
            .values()
            .filter(|alert| !alert.resolved)
            .cloned()
            .collect();
        
        Ok(active_alerts)
    }
    
    pub async fn get_alert_history(&self, limit: Option<usize>) -> Result<Vec<Alert>, Box<dyn std::error::Error>> {
        let alerts_guard = self.alerts.read().await;
        let mut all_alerts: Vec<Alert> = alerts_guard.values().cloned().collect();
        
        // Sort by timestamp, newest first
        all_alerts.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        if let Some(limit) = limit {
            all_alerts.truncate(limit);
        }
        
        Ok(all_alerts)
    }
    
    pub async fn add_alert_rule(&self, rule: AlertRule) -> Result<(), Box<dyn std::error::Error>> {
        let mut rules_guard = self.rules.write().await;
        
        // Check if rule with same ID already exists
        if rules_guard.iter().any(|r| r.id == rule.id) {
            return Err("Alert rule with same ID already exists".into());
        }
        
        rules_guard.push(rule);
        Ok(())
    }
    
    pub async fn remove_alert_rule(&self, rule_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut rules_guard = self.rules.write().await;
        rules_guard.retain(|rule| rule.id != rule_id);
        Ok(())
    }
    
    pub async fn get_alert_rules(&self) -> Result<Vec<AlertRule>, Box<dyn std::error::Error>> {
        let rules_guard = self.rules.read().await;
        Ok(rules_guard.clone())
    }
}

// Data structures

#[derive(Debug, Clone)]
pub struct Alert {
    pub id: String,
    pub rule_id: String,
    pub severity: AlertSeverity,
    pub component: String,
    pub message: String,
    pub timestamp: SystemTime,
    pub resolved: bool,
    pub resolved_at: Option<SystemTime>,
}

#[derive(Debug, Clone)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub condition: AlertCondition,
    pub severity: AlertSeverity,
    pub component: String,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub enum AlertCondition {
    MetricThreshold {
        metric_name: String,
        operator: ThresholdOperator,
        value: f64,
        duration_seconds: u64,
    },
    HistogramThreshold {
        metric_name: String,
        percentile: f64,
        value: f64,
        duration_seconds: u64,
    },
}

#[derive(Debug, Clone)]
pub enum ThresholdOperator {
    GreaterThan,
    LessThan,
    Equals,
}