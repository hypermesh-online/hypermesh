//! Asset status tracking and monitoring
//!
//! Real-time asset status monitoring with state management,
//! resource usage tracking, and consensus proof validation.

use std::collections::HashMap;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};

use super::{AssetId, ConsensusProof};
use super::privacy::PrivacyLevel;
use super::proxy::ProxyAddress;
use super::adapter::ResourceUsage;

/// Current status of an asset instance
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetStatus {
    /// Unique asset identifier
    pub asset_id: AssetId,
    /// Current operational state
    pub state: AssetState,
    /// When asset was allocated
    pub allocated_at: SystemTime,
    /// Last access timestamp
    pub last_accessed: SystemTime,
    /// Current resource usage metrics
    pub resource_usage: ResourceUsage,
    /// Privacy level configuration
    pub privacy_level: PrivacyLevel,
    /// Remote proxy address if assigned
    pub proxy_address: Option<ProxyAddress>,
    /// Valid consensus proofs for this asset
    pub consensus_proofs: Vec<ConsensusProof>,
    /// Owner certificate fingerprint
    pub owner_certificate_fingerprint: String,
    /// Asset metadata and tags
    pub metadata: HashMap<String, String>,
    /// Health status
    pub health_status: AssetHealthStatus,
    /// Performance metrics
    pub performance_metrics: AssetPerformanceMetrics,
}

/// Asset operational states
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetState {
    /// Asset is available for allocation
    Available,
    /// Asset is allocated but not actively used
    Allocated,
    /// Asset is actively in use
    InUse,
    /// Asset is under maintenance
    Maintenance,
    /// Asset has failed and is unavailable
    Failed,
}

impl AssetState {
    /// Check if asset is operational (can be used)
    pub fn is_operational(&self) -> bool {
        matches!(self, AssetState::Available | AssetState::Allocated | AssetState::InUse)
    }
    
    /// Check if asset is actively processing
    pub fn is_active(&self) -> bool {
        matches!(self, AssetState::InUse)
    }
    
    /// Check if asset is available for allocation
    pub fn is_available(&self) -> bool {
        matches!(self, AssetState::Available)
    }
    
    /// Get state priority for scheduling (lower is higher priority)
    pub fn priority(&self) -> u8 {
        match self {
            AssetState::Available => 0,
            AssetState::Allocated => 1,
            AssetState::InUse => 2,
            AssetState::Maintenance => 3,
            AssetState::Failed => 4,
        }
    }
    
    /// Get human-readable state description
    pub fn description(&self) -> &'static str {
        match self {
            AssetState::Available => "Ready for allocation",
            AssetState::Allocated => "Allocated but idle",
            AssetState::InUse => "Actively processing",
            AssetState::Maintenance => "Under maintenance",
            AssetState::Failed => "Failed and unavailable",
        }
    }
}

impl std::fmt::Display for AssetState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Asset health status tracking
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetHealthStatus {
    /// Overall health score (0.0 - 1.0)
    pub health_score: f32,
    /// Health check timestamp
    pub last_health_check: SystemTime,
    /// Detailed health metrics
    pub health_metrics: HashMap<String, f32>,
    /// Active alerts and warnings
    pub alerts: Vec<AssetAlert>,
    /// Health trend (improving, stable, degrading)
    pub health_trend: HealthTrend,
}

/// Asset performance metrics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetPerformanceMetrics {
    /// Uptime since allocation
    pub uptime_seconds: u64,
    /// Total operations processed
    pub operations_count: u64,
    /// Average operation latency in microseconds
    pub avg_latency_us: f32,
    /// Throughput (operations per second)
    pub throughput_ops: f32,
    /// Error rate (0.0 - 1.0)
    pub error_rate: f32,
    /// Performance score (0.0 - 1.0)
    pub performance_score: f32,
    /// Last metrics update
    pub last_update: SystemTime,
}

/// Asset alert types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetAlert {
    /// Alert severity level
    pub severity: AlertSeverity,
    /// Alert message
    pub message: String,
    /// Alert category
    pub category: AlertCategory,
    /// When alert was raised
    pub timestamp: SystemTime,
    /// Alert source component
    pub source: String,
    /// Additional alert metadata
    pub metadata: HashMap<String, String>,
}

/// Alert severity levels
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Informational alert
    Info,
    /// Warning condition
    Warning,
    /// Error condition
    Error,
    /// Critical condition requiring immediate attention
    Critical,
}

/// Alert categories
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AlertCategory {
    /// Performance-related alerts
    Performance,
    /// Resource utilization alerts
    Resource,
    /// Security-related alerts
    Security,
    /// Network connectivity alerts
    Network,
    /// Hardware failure alerts
    Hardware,
    /// Configuration alerts
    Configuration,
    /// Custom alert category
    Custom(String),
}

/// Health trend indicators
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum HealthTrend {
    /// Health is improving
    Improving,
    /// Health is stable
    Stable,
    /// Health is degrading
    Degrading,
    /// Insufficient data to determine trend
    Unknown,
}

impl AssetStatus {
    /// Create new asset status with default values
    pub fn new(
        asset_id: AssetId,
        owner_certificate_fingerprint: String,
        privacy_level: PrivacyLevel,
    ) -> Self {
        let now = SystemTime::now();
        
        Self {
            asset_id,
            state: AssetState::Available,
            allocated_at: now,
            last_accessed: now,
            resource_usage: ResourceUsage {
                cpu_usage: None,
                gpu_usage: None,
                memory_usage: None,
                storage_usage: None,
                network_usage: None,
                measurement_timestamp: now,
            },
            privacy_level,
            proxy_address: None,
            consensus_proofs: Vec::new(),
            owner_certificate_fingerprint,
            metadata: HashMap::new(),
            health_status: AssetHealthStatus::default(),
            performance_metrics: AssetPerformanceMetrics::default(),
        }
    }
    
    /// Update asset state
    pub fn update_state(&mut self, new_state: AssetState) {
        self.state = new_state;
        self.last_accessed = SystemTime::now();
    }
    
    /// Add consensus proof
    pub fn add_consensus_proof(&mut self, proof: ConsensusProof) {
        self.consensus_proofs.push(proof);
    }
    
    /// Set proxy address
    pub fn set_proxy_address(&mut self, proxy_address: ProxyAddress) {
        self.proxy_address = Some(proxy_address);
    }
    
    /// Update resource usage
    pub fn update_resource_usage(&mut self, usage: ResourceUsage) {
        self.resource_usage = usage;
        self.last_accessed = SystemTime::now();
    }
    
    /// Add metadata entry
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
    
    /// Add health alert
    pub fn add_alert(&mut self, alert: AssetAlert) {
        // Update health score based on alert severity
        match alert.severity {
            AlertSeverity::Critical => self.health_status.health_score *= 0.5,
            AlertSeverity::Error => self.health_status.health_score *= 0.7,
            AlertSeverity::Warning => self.health_status.health_score *= 0.9,
            AlertSeverity::Info => {} // No impact on health score
        }
        
        self.health_status.alerts.push(alert);
        
        // Ensure health score doesn't go below 0
        self.health_status.health_score = self.health_status.health_score.max(0.0);
    }
    
    /// Clear resolved alerts
    pub fn clear_resolved_alerts(&mut self, category: Option<AlertCategory>) {
        match category {
            Some(cat) => {
                self.health_status.alerts.retain(|alert| 
                    std::mem::discriminant(&alert.category) != std::mem::discriminant(&cat)
                );
            },
            None => self.health_status.alerts.clear(),
        }
    }
    
    /// Calculate uptime since allocation
    pub fn uptime(&self) -> Option<std::time::Duration> {
        SystemTime::now().duration_since(self.allocated_at).ok()
    }
    
    /// Check if asset requires maintenance
    pub fn requires_maintenance(&self) -> bool {
        self.health_status.health_score < 0.5 ||
        self.health_status.alerts.iter().any(|alert| 
            alert.severity == AlertSeverity::Critical
        )
    }
    
    /// Get age since last access
    pub fn idle_time(&self) -> Option<std::time::Duration> {
        SystemTime::now().duration_since(self.last_accessed).ok()
    }
    
    /// Check if consensus proofs are valid
    pub fn validate_consensus_proofs(&self) -> bool {
        !self.consensus_proofs.is_empty() &&
        self.consensus_proofs.iter().all(|proof| proof.validate())
    }
}

impl Default for AssetHealthStatus {
    fn default() -> Self {
        Self {
            health_score: 1.0,
            last_health_check: SystemTime::now(),
            health_metrics: HashMap::new(),
            alerts: Vec::new(),
            health_trend: HealthTrend::Unknown,
        }
    }
}

impl Default for AssetPerformanceMetrics {
    fn default() -> Self {
        Self {
            uptime_seconds: 0,
            operations_count: 0,
            avg_latency_us: 0.0,
            throughput_ops: 0.0,
            error_rate: 0.0,
            performance_score: 1.0,
            last_update: SystemTime::now(),
        }
    }
}

impl AssetAlert {
    /// Create new asset alert
    pub fn new(
        severity: AlertSeverity,
        message: String,
        category: AlertCategory,
        source: String,
    ) -> Self {
        Self {
            severity,
            message,
            category,
            timestamp: SystemTime::now(),
            source,
            metadata: HashMap::new(),
        }
    }
    
    /// Add metadata to alert
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
    
    /// Check if alert is critical
    pub fn is_critical(&self) -> bool {
        self.severity == AlertSeverity::Critical
    }
    
    /// Get alert age
    pub fn age(&self) -> Option<std::time::Duration> {
        SystemTime::now().duration_since(self.timestamp).ok()
    }
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertSeverity::Info => write!(f, "INFO"),
            AlertSeverity::Warning => write!(f, "WARN"),
            AlertSeverity::Error => write!(f, "ERROR"),
            AlertSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

impl std::fmt::Display for AlertCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertCategory::Performance => write!(f, "Performance"),
            AlertCategory::Resource => write!(f, "Resource"),
            AlertCategory::Security => write!(f, "Security"),
            AlertCategory::Network => write!(f, "Network"),
            AlertCategory::Hardware => write!(f, "Hardware"),
            AlertCategory::Configuration => write!(f, "Configuration"),
            AlertCategory::Custom(name) => write!(f, "{}", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{AssetId, AssetType};
    use super::privacy::PrivacyLevel;
    
    #[test]
    fn test_asset_state_operations() {
        assert!(AssetState::Available.is_operational());
        assert!(AssetState::Available.is_available());
        assert!(!AssetState::Failed.is_operational());
        assert!(!AssetState::InUse.is_available());
        assert!(AssetState::InUse.is_active());
    }
    
    #[test]
    fn test_asset_status_creation() {
        let asset_id = AssetId::new(AssetType::Cpu);
        let status = AssetStatus::new(
            asset_id.clone(),
            "test-cert-fingerprint".to_string(),
            PrivacyLevel::Private,
        );
        
        assert_eq!(status.asset_id, asset_id);
        assert_eq!(status.state, AssetState::Available);
        assert_eq!(status.privacy_level, PrivacyLevel::Private);
        assert_eq!(status.health_status.health_score, 1.0);
    }
    
    #[test]
    fn test_asset_alert_creation() {
        let alert = AssetAlert::new(
            AlertSeverity::Warning,
            "High CPU usage detected".to_string(),
            AlertCategory::Performance,
            "cpu-monitor".to_string(),
        );
        
        assert_eq!(alert.severity, AlertSeverity::Warning);
        assert!(!alert.is_critical());
        assert!(alert.message.contains("CPU"));
    }
    
    #[test]
    fn test_health_status_with_alerts() {
        let asset_id = AssetId::new(AssetType::Memory);
        let mut status = AssetStatus::new(
            asset_id,
            "test-cert".to_string(),
            PrivacyLevel::FullPublic,
        );
        
        // Add critical alert
        let critical_alert = AssetAlert::new(
            AlertSeverity::Critical,
            "Memory failure".to_string(),
            AlertCategory::Hardware,
            "hardware-monitor".to_string(),
        );
        
        status.add_alert(critical_alert);
        assert!(status.health_status.health_score < 1.0);
        assert!(status.requires_maintenance());
    }
}