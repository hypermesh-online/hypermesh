//! Security and isolation for HyperMesh extensions
//!
//! Implements capability-based security, resource quotas, and runtime monitoring.

use anyhow::{Context, Result as AnyhowResult};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, error, info, warn};

use super::{
    ExtensionCapability, ExtensionError, ExtensionMetadata, ExtensionResult,
    ResourceLimits,
};

/// Security context for an extension
#[derive(Debug, Clone)]
pub struct SecurityContext {
    /// Extension ID
    pub extension_id: String,

    /// Granted capabilities
    pub capabilities: HashSet<ExtensionCapability>,

    /// Resource quotas
    pub quotas: ResourceQuotas,

    /// Security policy
    pub policy: SecurityPolicy,

    /// Isolation level
    pub isolation: IsolationLevel,

    /// Audit configuration
    pub audit: AuditConfig,
}

/// Resource quotas for extension
#[derive(Debug, Clone)]
pub struct ResourceQuotas {
    /// CPU quota (percentage)
    pub cpu_percent: f32,

    /// Memory quota (bytes)
    pub memory_bytes: u64,

    /// Storage quota (bytes)
    pub storage_bytes: u64,

    /// Network bandwidth (bytes/sec)
    pub network_bandwidth: u64,

    /// File descriptor limit
    pub file_descriptors: u32,

    /// Thread/task limit
    pub max_threads: u32,

    /// Operation rate limit (ops/sec)
    pub ops_per_second: u32,
}

impl From<ResourceLimits> for ResourceQuotas {
    fn from(limits: ResourceLimits) -> Self {
        Self {
            cpu_percent: limits.max_cpu_percent,
            memory_bytes: limits.max_memory_bytes,
            storage_bytes: limits.max_storage_bytes,
            network_bandwidth: limits.max_network_bandwidth,
            file_descriptors: 1024,
            max_threads: 100,
            ops_per_second: 1000,
        }
    }
}

/// Security policy for extension
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    /// Allow network access
    pub allow_network: bool,

    /// Allowed network endpoints
    pub allowed_endpoints: Vec<String>,

    /// Allow file system access
    pub allow_filesystem: bool,

    /// Allowed file paths
    pub allowed_paths: Vec<String>,

    /// Allow system calls
    pub allow_syscalls: bool,

    /// Allowed system calls
    pub allowed_syscalls: Vec<String>,

    /// Allow IPC
    pub allow_ipc: bool,

    /// Require signature verification
    pub require_signature: bool,

    /// Minimum TLS version
    pub min_tls_version: String,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            allow_network: false,
            allowed_endpoints: vec![],
            allow_filesystem: false,
            allowed_paths: vec![],
            allow_syscalls: false,
            allowed_syscalls: vec![],
            allow_ipc: false,
            require_signature: true,
            min_tls_version: "1.3".to_string(),
        }
    }
}

/// Isolation level for extension execution
#[derive(Debug, Clone, PartialEq)]
pub enum IsolationLevel {
    /// No isolation (trusted extensions only)
    None,

    /// Process isolation
    Process,

    /// Container isolation
    Container,

    /// VM isolation
    VirtualMachine,

    /// Hardware isolation (SGX, etc.)
    Hardware,
}

/// Audit configuration
#[derive(Debug, Clone)]
pub struct AuditConfig {
    /// Enable audit logging
    pub enabled: bool,

    /// Log all operations
    pub log_all_ops: bool,

    /// Log failed operations
    pub log_failures: bool,

    /// Log resource usage
    pub log_resources: bool,

    /// Audit retention days
    pub retention_days: u32,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_all_ops: false,
            log_failures: true,
            log_resources: true,
            retention_days: 30,
        }
    }
}

/// Security manager for extensions
pub struct SecurityManager {
    /// Security contexts by extension ID
    contexts: Arc<RwLock<HashMap<String, SecurityContext>>>,

    /// Resource monitors
    monitors: Arc<RwLock<HashMap<String, ResourceMonitor>>>,

    /// Capability validators
    validators: Arc<RwLock<HashMap<ExtensionCapability, Box<dyn CapabilityValidator>>>>,

    /// Anomaly detector
    anomaly_detector: Arc<AnomalyDetector>,

    /// Audit logger
    audit_logger: Arc<AuditLogger>,

    /// Configuration
    config: SecurityConfig,
}

/// Security manager configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Enable security enforcement
    pub enforcement_enabled: bool,

    /// Enable anomaly detection
    pub anomaly_detection: bool,

    /// Enable audit logging
    pub audit_enabled: bool,

    /// Default isolation level
    pub default_isolation: IsolationLevel,

    /// Maximum violations before blocking
    pub max_violations: u32,

    /// Violation reset interval
    pub violation_reset_interval: Duration,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enforcement_enabled: true,
            anomaly_detection: true,
            audit_enabled: true,
            default_isolation: IsolationLevel::Process,
            max_violations: 10,
            violation_reset_interval: Duration::from_secs(3600),
        }
    }
}

/// Resource monitor for an extension
pub struct ResourceMonitor {
    /// Extension ID
    extension_id: String,

    /// Resource quotas
    quotas: ResourceQuotas,

    /// Current usage
    usage: Arc<RwLock<ResourceUsage>>,

    /// Rate limiter
    rate_limiter: Arc<Semaphore>,

    /// Violation counter
    violations: Arc<RwLock<ViolationCounter>>,
}

/// Current resource usage
#[derive(Debug, Default)]
pub struct ResourceUsage {
    /// CPU usage percentage
    pub cpu_percent: f32,

    /// Memory usage bytes
    pub memory_bytes: u64,

    /// Storage usage bytes
    pub storage_bytes: u64,

    /// Network bytes transferred
    pub network_bytes: u64,

    /// Open file descriptors
    pub file_descriptors: u32,

    /// Active threads
    pub thread_count: u32,

    /// Operations per second
    pub ops_per_second: f32,

    /// Last update time
    pub last_update: Option<SystemTime>,
}

/// Violation tracking
#[derive(Debug, Default)]
pub struct ViolationCounter {
    /// Total violations
    pub total: u32,

    /// Violations by type
    pub by_type: HashMap<String, u32>,

    /// Last violation time
    pub last_violation: Option<SystemTime>,

    /// Reset time
    pub reset_at: Option<SystemTime>,
}

/// Capability validator trait
#[async_trait::async_trait]
pub trait CapabilityValidator: Send + Sync {
    /// Validate capability request
    async fn validate(
        &self,
        extension_id: &str,
        capability: &ExtensionCapability,
        operation: &str,
    ) -> ExtensionResult<()>;
}

/// Anomaly detector for extension behavior
pub struct AnomalyDetector {
    /// Detection rules
    rules: Vec<Box<dyn AnomalyRule>>,

    /// Historical data
    history: Arc<RwLock<HashMap<String, ExtensionHistory>>>,

    /// Alert threshold
    alert_threshold: f32,
}

/// Anomaly detection rule trait
#[async_trait::async_trait]
pub trait AnomalyRule: Send + Sync {
    /// Check for anomalies
    async fn check(
        &self,
        extension_id: &str,
        current: &ResourceUsage,
        history: &ExtensionHistory,
    ) -> Option<Anomaly>;
}

/// Extension historical data
#[derive(Debug, Default)]
pub struct ExtensionHistory {
    /// CPU usage history
    pub cpu_history: Vec<f32>,

    /// Memory usage history
    pub memory_history: Vec<u64>,

    /// Operation rate history
    pub ops_history: Vec<f32>,

    /// Violation history
    pub violations: Vec<ViolationRecord>,
}

/// Violation record
#[derive(Debug, Clone)]
pub struct ViolationRecord {
    /// Violation type
    pub violation_type: String,

    /// Timestamp
    pub timestamp: SystemTime,

    /// Details
    pub details: String,
}

/// Detected anomaly
#[derive(Debug, Clone)]
pub struct Anomaly {
    /// Anomaly type
    pub anomaly_type: String,

    /// Severity (0-1)
    pub severity: f32,

    /// Description
    pub description: String,

    /// Recommended action
    pub action: AnomalyAction,
}

/// Recommended action for anomaly
#[derive(Debug, Clone)]
pub enum AnomalyAction {
    /// Log the anomaly
    Log,

    /// Alert administrators
    Alert,

    /// Throttle the extension
    Throttle,

    /// Suspend the extension
    Suspend,

    /// Terminate the extension
    Terminate,
}

/// Audit logger for security events
pub struct AuditLogger {
    /// Audit log entries
    entries: Arc<RwLock<Vec<AuditEntry>>>,

    /// Configuration
    config: AuditConfig,
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Timestamp
    pub timestamp: SystemTime,

    /// Extension ID
    pub extension_id: String,

    /// Event type
    pub event_type: AuditEventType,

    /// Operation
    pub operation: String,

    /// Result
    pub result: AuditResult,

    /// Details
    pub details: Option<serde_json::Value>,
}

/// Audit event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    /// Capability request
    CapabilityRequest,

    /// Resource access
    ResourceAccess,

    /// Security violation
    SecurityViolation,

    /// Anomaly detected
    AnomalyDetected,

    /// Configuration change
    ConfigurationChange,
}

/// Audit result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditResult {
    /// Operation succeeded
    Success,

    /// Operation failed
    Failure(String),

    /// Operation denied
    Denied(String),
}

impl SecurityManager {
    /// Create new security manager
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            contexts: Arc::new(RwLock::new(HashMap::new())),
            monitors: Arc::new(RwLock::new(HashMap::new())),
            validators: Arc::new(RwLock::new(HashMap::new())),
            anomaly_detector: Arc::new(AnomalyDetector::new()),
            audit_logger: Arc::new(AuditLogger::new(AuditConfig::default())),
            config,
        }
    }

    /// Create security context for extension
    pub async fn create_context(
        &self,
        extension_id: String,
        metadata: &ExtensionMetadata,
        granted_capabilities: HashSet<ExtensionCapability>,
        quotas: ResourceQuotas,
    ) -> ExtensionResult<SecurityContext> {
        // Validate granted capabilities are subset of required
        for cap in &granted_capabilities {
            if !metadata.required_capabilities.contains(cap) {
                warn!(
                    "Granting capability {:?} not in required set for {}",
                    cap, extension_id
                );
            }
        }

        let context = SecurityContext {
            extension_id: extension_id.clone(),
            capabilities: granted_capabilities,
            quotas: quotas.clone(),
            policy: SecurityPolicy::default(),
            isolation: self.config.default_isolation.clone(),
            audit: AuditConfig::default(),
        };

        // Create resource monitor
        let monitor = ResourceMonitor::new(extension_id.clone(), quotas);

        // Store context and monitor
        {
            let mut contexts = self.contexts.write().await;
            contexts.insert(extension_id.clone(), context.clone());
        }

        {
            let mut monitors = self.monitors.write().await;
            monitors.insert(extension_id.clone(), monitor);
        }

        info!("Created security context for extension: {}", extension_id);
        Ok(context)
    }

    /// Check if capability is granted
    pub async fn check_capability(
        &self,
        extension_id: &str,
        capability: &ExtensionCapability,
        operation: &str,
    ) -> ExtensionResult<()> {
        if !self.config.enforcement_enabled {
            return Ok(());
        }

        // Get security context
        let contexts = self.contexts.read().await;
        let context = contexts.get(extension_id)
            .ok_or_else(|| ExtensionError::ExtensionNotFound {
                id: extension_id.to_string(),
            })?;

        // Check if capability is granted
        if !context.capabilities.contains(capability) {
            self.audit_logger.log(AuditEntry {
                timestamp: SystemTime::now(),
                extension_id: extension_id.to_string(),
                event_type: AuditEventType::CapabilityRequest,
                operation: operation.to_string(),
                result: AuditResult::Denied(format!("Capability not granted: {:?}", capability)),
                details: None,
            }).await;

            return Err(ExtensionError::CapabilityNotGranted {
                capability: format!("{:?}", capability),
            });
        }

        // Additional validation if validator exists
        let validators = self.validators.read().await;
        if let Some(validator) = validators.get(capability) {
            validator.validate(extension_id, capability, operation).await?;
        }

        // Audit successful check
        self.audit_logger.log(AuditEntry {
            timestamp: SystemTime::now(),
            extension_id: extension_id.to_string(),
            event_type: AuditEventType::CapabilityRequest,
            operation: operation.to_string(),
            result: AuditResult::Success,
            details: None,
        }).await;

        Ok(())
    }

    /// Check resource usage against quotas
    pub async fn check_resource_usage(
        &self,
        extension_id: &str,
    ) -> ExtensionResult<()> {
        if !self.config.enforcement_enabled {
            return Ok(());
        }

        let monitors = self.monitors.read().await;
        let monitor = monitors.get(extension_id)
            .ok_or_else(|| ExtensionError::ExtensionNotFound {
                id: extension_id.to_string(),
            })?;

        monitor.check_quotas().await
    }

    /// Update resource usage
    pub async fn update_usage(
        &self,
        extension_id: &str,
        usage: ResourceUsage,
    ) -> ExtensionResult<()> {
        let monitors = self.monitors.read().await;
        if let Some(monitor) = monitors.get(extension_id) {
            monitor.update_usage(usage).await?;

            // Check for anomalies if enabled
            if self.config.anomaly_detection {
                self.anomaly_detector.check(extension_id, &monitor).await;
            }
        }

        Ok(())
    }

    /// Record security violation
    pub async fn record_violation(
        &self,
        extension_id: &str,
        violation_type: &str,
        details: &str,
    ) {
        let monitors = self.monitors.read().await;
        if let Some(monitor) = monitors.get(extension_id) {
            monitor.record_violation(violation_type, details).await;

            // Check if max violations exceeded
            let violations = monitor.get_violation_count().await;
            if violations > self.config.max_violations {
                warn!(
                    "Extension {} exceeded max violations ({}), suspending",
                    extension_id, violations
                );
                // TODO: Suspend extension
            }
        }

        // Audit the violation
        self.audit_logger.log(AuditEntry {
            timestamp: SystemTime::now(),
            extension_id: extension_id.to_string(),
            event_type: AuditEventType::SecurityViolation,
            operation: violation_type.to_string(),
            result: AuditResult::Failure(details.to_string()),
            details: None,
        }).await;
    }

    /// Get security metrics for extension
    pub async fn get_metrics(&self, extension_id: &str) -> Option<SecurityMetrics> {
        let monitors = self.monitors.read().await;
        let monitor = monitors.get(extension_id)?;

        Some(monitor.get_metrics().await)
    }
}

impl ResourceMonitor {
    /// Create new resource monitor
    pub fn new(extension_id: String, quotas: ResourceQuotas) -> Self {
        let rate_limiter = Arc::new(Semaphore::new(quotas.ops_per_second as usize));

        Self {
            extension_id,
            quotas,
            usage: Arc::new(RwLock::new(ResourceUsage::default())),
            rate_limiter,
            violations: Arc::new(RwLock::new(ViolationCounter::default())),
        }
    }

    /// Check if resource usage is within quotas
    pub async fn check_quotas(&self) -> ExtensionResult<()> {
        let usage = self.usage.read().await;

        if usage.cpu_percent > self.quotas.cpu_percent {
            return Err(ExtensionError::ResourceLimitExceeded {
                resource: format!("CPU: {:.1}% > {:.1}%", usage.cpu_percent, self.quotas.cpu_percent),
            });
        }

        if usage.memory_bytes > self.quotas.memory_bytes {
            return Err(ExtensionError::ResourceLimitExceeded {
                resource: format!("Memory: {} > {}", usage.memory_bytes, self.quotas.memory_bytes),
            });
        }

        if usage.storage_bytes > self.quotas.storage_bytes {
            return Err(ExtensionError::ResourceLimitExceeded {
                resource: format!("Storage: {} > {}", usage.storage_bytes, self.quotas.storage_bytes),
            });
        }

        if usage.file_descriptors > self.quotas.file_descriptors {
            return Err(ExtensionError::ResourceLimitExceeded {
                resource: format!("FDs: {} > {}", usage.file_descriptors, self.quotas.file_descriptors),
            });
        }

        if usage.thread_count > self.quotas.max_threads {
            return Err(ExtensionError::ResourceLimitExceeded {
                resource: format!("Threads: {} > {}", usage.thread_count, self.quotas.max_threads),
            });
        }

        Ok(())
    }

    /// Update resource usage
    pub async fn update_usage(&self, new_usage: ResourceUsage) -> ExtensionResult<()> {
        let mut usage = self.usage.write().await;
        *usage = new_usage;
        usage.last_update = Some(SystemTime::now());
        Ok(())
    }

    /// Record a violation
    pub async fn record_violation(&self, violation_type: &str, _details: &str) {
        let mut violations = self.violations.write().await;
        violations.total += 1;
        *violations.by_type.entry(violation_type.to_string()).or_insert(0) += 1;
        violations.last_violation = Some(SystemTime::now());
    }

    /// Get violation count
    pub async fn get_violation_count(&self) -> u32 {
        let violations = self.violations.read().await;
        violations.total
    }

    /// Get security metrics
    pub async fn get_metrics(&self) -> SecurityMetrics {
        let usage = self.usage.read().await;
        let violations = self.violations.read().await;

        SecurityMetrics {
            cpu_usage: usage.cpu_percent,
            memory_usage: usage.memory_bytes,
            storage_usage: usage.storage_bytes,
            network_usage: usage.network_bytes,
            violations: violations.total,
            last_violation: violations.last_violation,
        }
    }

    /// Acquire rate limit permit
    pub async fn acquire_permit(&self) -> ExtensionResult<()> {
        match self.rate_limiter.try_acquire() {
            Ok(permit) => {
                // Permit acquired, will be dropped automatically
                drop(permit);
                Ok(())
            }
            Err(_) => {
                Err(ExtensionError::ResourceLimitExceeded {
                    resource: format!("Rate limit: {} ops/sec", self.quotas.ops_per_second),
                })
            }
        }
    }
}

impl AnomalyDetector {
    /// Create new anomaly detector
    pub fn new() -> Self {
        Self {
            rules: vec![
                Box::new(CPUAnomalyRule::new(2.0)),
                Box::new(MemoryAnomalyRule::new(2.0)),
                Box::new(RateAnomalyRule::new(3.0)),
            ],
            history: Arc::new(RwLock::new(HashMap::new())),
            alert_threshold: 0.7,
        }
    }

    /// Check for anomalies
    pub async fn check(&self, extension_id: &str, monitor: &ResourceMonitor) {
        let usage = monitor.usage.read().await;
        let mut history = self.history.write().await;
        let ext_history = history.entry(extension_id.to_string()).or_default();

        // Update history
        ext_history.cpu_history.push(usage.cpu_percent);
        ext_history.memory_history.push(usage.memory_bytes);
        ext_history.ops_history.push(usage.ops_per_second);

        // Keep only recent history (last 100 samples)
        if ext_history.cpu_history.len() > 100 {
            ext_history.cpu_history.remove(0);
            ext_history.memory_history.remove(0);
            ext_history.ops_history.remove(0);
        }

        // Check all rules
        for rule in &self.rules {
            if let Some(anomaly) = rule.check(extension_id, &usage, ext_history).await {
                if anomaly.severity >= self.alert_threshold {
                    warn!(
                        "Anomaly detected for {}: {} (severity: {:.2})",
                        extension_id, anomaly.description, anomaly.severity
                    );

                    // TODO: Take action based on anomaly.action
                }
            }
        }
    }
}

/// CPU anomaly detection rule
pub struct CPUAnomalyRule {
    /// Standard deviation threshold
    threshold: f32,
}

impl CPUAnomalyRule {
    pub fn new(threshold: f32) -> Self {
        Self { threshold }
    }
}

#[async_trait::async_trait]
impl AnomalyRule for CPUAnomalyRule {
    async fn check(
        &self,
        _extension_id: &str,
        current: &ResourceUsage,
        history: &ExtensionHistory,
    ) -> Option<Anomaly> {
        if history.cpu_history.len() < 10 {
            return None;
        }

        let mean: f32 = history.cpu_history.iter().sum::<f32>() / history.cpu_history.len() as f32;
        let variance: f32 = history.cpu_history.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f32>() / history.cpu_history.len() as f32;
        let std_dev = variance.sqrt();

        if (current.cpu_percent - mean).abs() > self.threshold * std_dev {
            return Some(Anomaly {
                anomaly_type: "CPU Usage Spike".to_string(),
                severity: ((current.cpu_percent - mean).abs() / (self.threshold * std_dev)).min(1.0),
                description: format!(
                    "CPU usage {:.1}% deviates from mean {:.1}% by {:.1} std devs",
                    current.cpu_percent, mean, (current.cpu_percent - mean).abs() / std_dev
                ),
                action: AnomalyAction::Alert,
            });
        }

        None
    }
}

/// Memory anomaly detection rule
pub struct MemoryAnomalyRule {
    /// Growth rate threshold
    threshold: f32,
}

impl MemoryAnomalyRule {
    pub fn new(threshold: f32) -> Self {
        Self { threshold }
    }
}

#[async_trait::async_trait]
impl AnomalyRule for MemoryAnomalyRule {
    async fn check(
        &self,
        _extension_id: &str,
        current: &ResourceUsage,
        history: &ExtensionHistory,
    ) -> Option<Anomaly> {
        if history.memory_history.len() < 2 {
            return None;
        }

        let prev = history.memory_history[history.memory_history.len() - 1];
        if prev == 0 {
            return None;
        }

        let growth_rate = (current.memory_bytes as f64 / prev as f64) - 1.0;

        if growth_rate > self.threshold {
            return Some(Anomaly {
                anomaly_type: "Memory Leak".to_string(),
                severity: (growth_rate / self.threshold).min(1.0) as f32,
                description: format!(
                    "Memory usage grew by {:.1}% (from {} to {})",
                    growth_rate * 100.0, prev, current.memory_bytes
                ),
                action: if growth_rate > self.threshold * 2.0 {
                    AnomalyAction::Throttle
                } else {
                    AnomalyAction::Alert
                },
            });
        }

        None
    }
}

/// Operation rate anomaly detection rule
pub struct RateAnomalyRule {
    /// Rate spike threshold
    threshold: f32,
}

impl RateAnomalyRule {
    pub fn new(threshold: f32) -> Self {
        Self { threshold }
    }
}

#[async_trait::async_trait]
impl AnomalyRule for RateAnomalyRule {
    async fn check(
        &self,
        _extension_id: &str,
        current: &ResourceUsage,
        history: &ExtensionHistory,
    ) -> Option<Anomaly> {
        if history.ops_history.len() < 5 {
            return None;
        }

        let mean: f32 = history.ops_history.iter().sum::<f32>() / history.ops_history.len() as f32;

        if mean > 0.0 && current.ops_per_second / mean > self.threshold {
            return Some(Anomaly {
                anomaly_type: "Rate Spike".to_string(),
                severity: ((current.ops_per_second / mean) / self.threshold).min(1.0),
                description: format!(
                    "Operation rate {:.1} ops/sec is {:.1}x normal rate {:.1} ops/sec",
                    current.ops_per_second, current.ops_per_second / mean, mean
                ),
                action: AnomalyAction::Throttle,
            });
        }

        None
    }
}

impl AuditLogger {
    /// Create new audit logger
    pub fn new(config: AuditConfig) -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            config,
        }
    }

    /// Log an audit entry
    pub async fn log(&self, entry: AuditEntry) {
        if !self.config.enabled {
            return;
        }

        // Check if we should log this type of entry
        match entry.event_type {
            AuditEventType::SecurityViolation => {
                if !self.config.log_failures {
                    return;
                }
            }
            _ => {
                if !self.config.log_all_ops {
                    return;
                }
            }
        }

        let mut entries = self.entries.write().await;
        entries.push(entry);

        // Clean up old entries
        let retention_cutoff = SystemTime::now() - Duration::from_secs(
            self.config.retention_days as u64 * 86400
        );

        entries.retain(|e| e.timestamp > retention_cutoff);
    }

    /// Get audit entries for extension
    pub async fn get_entries(&self, extension_id: &str) -> Vec<AuditEntry> {
        let entries = self.entries.read().await;
        entries.iter()
            .filter(|e| e.extension_id == extension_id)
            .cloned()
            .collect()
    }
}

/// Security metrics for an extension
#[derive(Debug, Clone)]
pub struct SecurityMetrics {
    /// CPU usage percentage
    pub cpu_usage: f32,

    /// Memory usage bytes
    pub memory_usage: u64,

    /// Storage usage bytes
    pub storage_usage: u64,

    /// Network usage bytes
    pub network_usage: u64,

    /// Total violations
    pub violations: u32,

    /// Last violation time
    pub last_violation: Option<SystemTime>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_quotas_from_limits() {
        let limits = ResourceLimits {
            max_cpu_percent: 50.0,
            max_memory_bytes: 1024 * 1024 * 1024,
            max_storage_bytes: 10 * 1024 * 1024 * 1024,
            max_network_bandwidth: 100 * 1024 * 1024,
            ..Default::default()
        };

        let quotas = ResourceQuotas::from(limits.clone());
        assert_eq!(quotas.cpu_percent, limits.max_cpu_percent);
        assert_eq!(quotas.memory_bytes, limits.max_memory_bytes);
        assert_eq!(quotas.storage_bytes, limits.max_storage_bytes);
    }

    #[tokio::test]
    async fn test_resource_monitor() {
        let quotas = ResourceQuotas {
            cpu_percent: 50.0,
            memory_bytes: 1024 * 1024,
            storage_bytes: 10 * 1024 * 1024,
            network_bandwidth: 1024 * 1024,
            file_descriptors: 100,
            max_threads: 10,
            ops_per_second: 100,
        };

        let monitor = ResourceMonitor::new("test".to_string(), quotas);

        // Update with usage within limits
        let usage = ResourceUsage {
            cpu_percent: 25.0,
            memory_bytes: 512 * 1024,
            storage_bytes: 1024 * 1024,
            network_bytes: 0,
            file_descriptors: 10,
            thread_count: 5,
            ops_per_second: 50.0,
            last_update: Some(SystemTime::now()),
        };

        monitor.update_usage(usage).await.unwrap();
        assert!(monitor.check_quotas().await.is_ok());

        // Update with usage exceeding limits
        let excessive_usage = ResourceUsage {
            cpu_percent: 75.0,
            memory_bytes: 2 * 1024 * 1024,
            ..Default::default()
        };

        monitor.update_usage(excessive_usage).await.unwrap();
        assert!(monitor.check_quotas().await.is_err());
    }

    #[tokio::test]
    async fn test_security_manager() {
        let config = SecurityConfig::default();
        let manager = SecurityManager::new(config);

        let metadata = ExtensionMetadata {
            id: "test".to_string(),
            name: "Test".to_string(),
            version: semver::Version::parse("1.0.0").unwrap(),
            description: "Test".to_string(),
            author: "Test".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            category: super::super::ExtensionCategory::AssetLibrary,
            hypermesh_version: semver::Version::parse("1.0.0").unwrap(),
            dependencies: vec![],
            required_capabilities: HashSet::from([
                ExtensionCapability::AssetManagement,
                ExtensionCapability::NetworkAccess,
            ]),
            provided_assets: vec![],
            certificate_fingerprint: None,
            config_schema: None,
        };

        let quotas = ResourceQuotas {
            cpu_percent: 50.0,
            memory_bytes: 1024 * 1024,
            storage_bytes: 10 * 1024 * 1024,
            network_bandwidth: 1024 * 1024,
            file_descriptors: 100,
            max_threads: 10,
            ops_per_second: 100,
        };

        // Create context with limited capabilities
        let context = manager.create_context(
            "test".to_string(),
            &metadata,
            HashSet::from([ExtensionCapability::AssetManagement]),
            quotas,
        ).await.unwrap();

        assert_eq!(context.extension_id, "test");
        assert!(context.capabilities.contains(&ExtensionCapability::AssetManagement));
        assert!(!context.capabilities.contains(&ExtensionCapability::NetworkAccess));

        // Check granted capability
        assert!(manager.check_capability(
            "test",
            &ExtensionCapability::AssetManagement,
            "test_operation"
        ).await.is_ok());

        // Check non-granted capability
        assert!(manager.check_capability(
            "test",
            &ExtensionCapability::NetworkAccess,
            "test_operation"
        ).await.is_err());
    }
}