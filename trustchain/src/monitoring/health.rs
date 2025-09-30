//! Health check system for TrustChain

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

/// Health check system
pub struct HealthCheck {
    /// Component health states
    components: Arc<RwLock<HashMap<String, ComponentHealth>>>,
    /// Last check time
    last_check: Arc<RwLock<Instant>>,
}

/// Overall health status
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Overall health
    pub is_healthy: bool,
    /// Health status
    pub status: HealthState,
    /// Component health details
    pub components: HashMap<String, ComponentHealth>,
    /// Last check timestamp
    pub last_check: SystemTime,
    /// Health score (0.0 - 1.0)
    pub health_score: f64,
    /// Any active issues
    pub issues: Vec<HealthIssue>,
}

/// Health state
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum HealthState {
    /// All systems operational
    Healthy,
    /// Minor issues, service operational
    Degraded,
    /// Major issues, service partially operational
    Unhealthy,
    /// Service non-operational
    Critical,
}

/// Component health information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Component name
    pub name: String,
    /// Component status
    pub status: HealthState,
    /// Health check passed
    pub is_healthy: bool,
    /// Last successful check
    pub last_success: Option<SystemTime>,
    /// Last failure
    pub last_failure: Option<SystemTime>,
    /// Consecutive failures
    pub consecutive_failures: u32,
    /// Response time (ms)
    pub response_time_ms: Option<u64>,
    /// Additional details
    pub details: HashMap<String, String>,
}

/// Health issue
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthIssue {
    /// Issue component
    pub component: String,
    /// Issue severity
    pub severity: IssueSeverity,
    /// Issue message
    pub message: String,
    /// Issue timestamp
    pub timestamp: SystemTime,
}

/// Issue severity
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum IssueSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl HealthCheck {
    /// Create new health check system
    pub fn new() -> Self {
        let mut components = HashMap::new();

        // Initialize component health states
        for component in &["ca", "ct", "dns", "consensus", "stoq", "api"] {
            components.insert(
                component.to_string(),
                ComponentHealth {
                    name: component.to_string(),
                    status: HealthState::Healthy,
                    is_healthy: true,
                    last_success: Some(SystemTime::now()),
                    last_failure: None,
                    consecutive_failures: 0,
                    response_time_ms: None,
                    details: HashMap::new(),
                }
            );
        }

        Self {
            components: Arc::new(RwLock::new(components)),
            last_check: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// Perform health check on all components
    pub async fn check_all(&self) {
        debug!("Performing health checks on all components");

        // Check CA service
        self.check_ca_health().await;

        // Check CT service
        self.check_ct_health().await;

        // Check DNS service
        self.check_dns_health().await;

        // Check consensus service
        self.check_consensus_health().await;

        // Check STOQ transport
        self.check_stoq_health().await;

        // Check API service
        self.check_api_health().await;

        // Update last check time
        let mut last = self.last_check.write().await;
        *last = Instant::now();
    }

    /// Check CA service health
    async fn check_ca_health(&self) {
        let start = Instant::now();
        let mut is_healthy = true;
        let mut details = HashMap::new();

        // Simulate CA health check
        // In production, would check:
        // - Certificate generation capability
        // - Key storage access
        // - Rotation status
        details.insert("certificate_generation".to_string(), "operational".to_string());
        details.insert("key_storage".to_string(), "accessible".to_string());
        details.insert("rotation_enabled".to_string(), "true".to_string());

        let response_time = start.elapsed().as_millis() as u64;

        self.update_component_health(
            "ca",
            is_healthy,
            response_time,
            details
        ).await;
    }

    /// Check CT service health
    async fn check_ct_health(&self) {
        let start = Instant::now();
        let mut is_healthy = true;
        let mut details = HashMap::new();

        // Simulate CT health check
        details.insert("log_storage".to_string(), "operational".to_string());
        details.insert("merkle_tree".to_string(), "consistent".to_string());
        details.insert("append_capability".to_string(), "enabled".to_string());

        let response_time = start.elapsed().as_millis() as u64;

        self.update_component_health(
            "ct",
            is_healthy,
            response_time,
            details
        ).await;
    }

    /// Check DNS service health
    async fn check_dns_health(&self) {
        let start = Instant::now();
        let mut is_healthy = true;
        let mut details = HashMap::new();

        // Simulate DNS health check
        details.insert("quic_listener".to_string(), "active".to_string());
        details.insert("upstream_resolvers".to_string(), "reachable".to_string());
        details.insert("cache".to_string(), "operational".to_string());

        let response_time = start.elapsed().as_millis() as u64;

        self.update_component_health(
            "dns",
            is_healthy,
            response_time,
            details
        ).await;
    }

    /// Check consensus service health
    async fn check_consensus_health(&self) {
        let start = Instant::now();
        let mut is_healthy = true;
        let mut details = HashMap::new();

        // Simulate consensus health check
        details.insert("proof_validation".to_string(), "operational".to_string());
        details.insert("byzantine_detection".to_string(), "active".to_string());
        details.insert("hypermesh_connection".to_string(), "established".to_string());

        let response_time = start.elapsed().as_millis() as u64;

        self.update_component_health(
            "consensus",
            is_healthy,
            response_time,
            details
        ).await;
    }

    /// Check STOQ transport health
    async fn check_stoq_health(&self) {
        let start = Instant::now();
        let mut is_healthy = true;
        let mut details = HashMap::new();

        // Simulate STOQ health check
        details.insert("quic_transport".to_string(), "operational".to_string());
        details.insert("connection_pool".to_string(), "healthy".to_string());
        details.insert("certificate_rotation".to_string(), "active".to_string());

        let response_time = start.elapsed().as_millis() as u64;

        self.update_component_health(
            "stoq",
            is_healthy,
            response_time,
            details
        ).await;
    }

    /// Check API service health
    async fn check_api_health(&self) {
        let start = Instant::now();
        let mut is_healthy = true;
        let mut details = HashMap::new();

        // Simulate API health check
        details.insert("http_server".to_string(), "running".to_string());
        details.insert("endpoints".to_string(), "responsive".to_string());
        details.insert("rate_limiting".to_string(), "active".to_string());

        let response_time = start.elapsed().as_millis() as u64;

        self.update_component_health(
            "api",
            is_healthy,
            response_time,
            details
        ).await;
    }

    /// Update component health status
    async fn update_component_health(
        &self,
        component: &str,
        is_healthy: bool,
        response_time_ms: u64,
        details: HashMap<String, String>,
    ) {
        let mut components = self.components.write().await;

        if let Some(health) = components.get_mut(component) {
            health.is_healthy = is_healthy;
            health.response_time_ms = Some(response_time_ms);
            health.details = details;

            if is_healthy {
                health.status = HealthState::Healthy;
                health.last_success = Some(SystemTime::now());
                health.consecutive_failures = 0;
            } else {
                health.last_failure = Some(SystemTime::now());
                health.consecutive_failures += 1;

                // Update status based on consecutive failures
                health.status = match health.consecutive_failures {
                    1..=2 => HealthState::Degraded,
                    3..=5 => HealthState::Unhealthy,
                    _ => HealthState::Critical,
                };
            }

            debug!("Updated health for {}: {:?}", component, health.status);
        }
    }

    /// Get current health status
    pub async fn get_status(&self) -> HealthStatus {
        let components = self.components.read().await.clone();
        let mut issues = Vec::new();
        let mut total_score = 0.0;
        let mut is_healthy = true;

        // Calculate overall health
        for (name, health) in &components {
            let component_score = match health.status {
                HealthState::Healthy => 1.0,
                HealthState::Degraded => 0.7,
                HealthState::Unhealthy => 0.3,
                HealthState::Critical => 0.0,
            };
            total_score += component_score;

            if health.status != HealthState::Healthy {
                is_healthy = false;
                issues.push(HealthIssue {
                    component: name.clone(),
                    severity: match health.status {
                        HealthState::Degraded => IssueSeverity::Low,
                        HealthState::Unhealthy => IssueSeverity::Medium,
                        HealthState::Critical => IssueSeverity::Critical,
                        _ => IssueSeverity::Low,
                    },
                    message: format!("{} is in {} state", name,
                        match health.status {
                            HealthState::Healthy => "healthy",
                            HealthState::Degraded => "degraded",
                            HealthState::Unhealthy => "unhealthy",
                            HealthState::Critical => "critical",
                        }
                    ),
                    timestamp: SystemTime::now(),
                });
            }
        }

        let health_score = total_score / components.len() as f64;
        let overall_status = if health_score >= 0.9 {
            HealthState::Healthy
        } else if health_score >= 0.7 {
            HealthState::Degraded
        } else if health_score >= 0.3 {
            HealthState::Unhealthy
        } else {
            HealthState::Critical
        };

        HealthStatus {
            is_healthy,
            status: overall_status,
            components,
            last_check: SystemTime::now(),
            health_score,
            issues,
        }
    }

    /// Check specific component health
    pub async fn check_component(&self, component: &str) -> Option<ComponentHealth> {
        let components = self.components.read().await;
        components.get(component).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check_creation() {
        let health = HealthCheck::new();
        let status = health.get_status().await;

        assert!(status.is_healthy);
        assert_eq!(status.status, HealthState::Healthy);
        assert_eq!(status.components.len(), 6);
    }

    #[tokio::test]
    async fn test_component_health_update() {
        let health = HealthCheck::new();

        // Update a component to unhealthy
        health.update_component_health(
            "ca",
            false,
            100,
            HashMap::new()
        ).await;

        let status = health.get_status().await;
        assert!(!status.is_healthy);
        assert_eq!(status.issues.len(), 1);
    }

    #[tokio::test]
    async fn test_health_score_calculation() {
        let health = HealthCheck::new();

        // Set different health states
        health.update_component_health("ca", true, 10, HashMap::new()).await;
        health.update_component_health("ct", false, 100, HashMap::new()).await;

        let status = health.get_status().await;
        assert!(status.health_score < 1.0);
        assert!(status.health_score > 0.0);
    }
}