// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Security type definitions - structs, enums, and traits.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, SystemTime};

use super::super::{
    ExtensionCapability, ExtensionResult,
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

/// Resource quotas for an extension
#[derive(Debug, Clone)]
pub struct ResourceQuotas {
    /// Maximum CPU percentage
    pub cpu_percent: f32,

    /// Maximum memory in bytes
    pub memory_bytes: u64,

    /// Maximum storage in bytes
    pub storage_bytes: u64,

    /// Maximum network bandwidth
    pub network_bandwidth: u64,

    /// Maximum file descriptors
    pub file_descriptors: u32,

    /// Maximum threads
    pub max_threads: u32,

    /// Operations per second limit
    pub ops_per_second: u32,
}

impl From<ResourceLimits> for ResourceQuotas {
    fn from(limits: ResourceLimits) -> Self {
        Self {
            cpu_percent: limits.max_cpu_percent,
            memory_bytes: limits.max_memory_bytes,
            storage_bytes: limits.max_storage_bytes,
            network_bandwidth: limits.max_network_bandwidth,
            file_descriptors: 256,
            max_threads: 16,
            ops_per_second: limits.max_concurrent_operations as u32,
        }
    }
}

/// Security policy for an extension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Allow network access
    pub allow_network: bool,

    /// Allow filesystem access
    pub allow_filesystem: bool,

    /// Allow subprocess spawning
    pub allow_subprocess: bool,

    /// Allow system calls
    pub allow_syscalls: bool,

    /// Allowed network destinations
    pub allowed_destinations: Vec<String>,

    /// Allowed filesystem paths
    pub allowed_paths: Vec<String>,

    /// Maximum execution time
    pub max_execution_time: Duration,

    /// Require signed code
    pub require_signed: bool,

    /// Minimum trust score
    pub min_trust_score: f32,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            allow_network: false,
            allow_filesystem: false,
            allow_subprocess: false,
            allow_syscalls: false,
            allowed_destinations: Vec::new(),
            allowed_paths: Vec::new(),
            max_execution_time: Duration::from_secs(300),
            require_signed: true,
            min_trust_score: 0.5,
        }
    }
}

/// Isolation level for extension execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IsolationLevel {
    /// No isolation (trusted extensions only)
    None,

    /// Process-level isolation
    Process,

    /// Container-level isolation
    Container,

    /// VM-level isolation
    VirtualMachine,

    /// Hardware-level isolation
    Hardware,
}

/// Audit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Enable audit logging
    pub enabled: bool,

    /// Log all operations
    pub log_all_ops: bool,

    /// Log failures only
    pub log_failures: bool,

    /// Retention period in days
    pub retention_days: u32,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_all_ops: false,
            log_failures: true,
            retention_days: 90,
        }
    }
}

/// Security manager configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Enable enforcement
    pub enforcement_enabled: bool,

    /// Default isolation level
    pub default_isolation: IsolationLevel,

    /// Maximum violations before suspension
    pub max_violations: u32,

    /// Enable anomaly detection
    pub anomaly_detection: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enforcement_enabled: true,
            default_isolation: IsolationLevel::Process,
            max_violations: 10,
            anomaly_detection: true,
        }
    }
}

/// Resource usage tracking
#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    /// CPU usage percentage
    pub cpu_percent: f32,

    /// Memory usage in bytes
    pub memory_bytes: u64,

    /// Storage usage in bytes
    pub storage_bytes: u64,

    /// Network bytes transferred
    pub network_bytes: u64,

    /// File descriptors in use
    pub file_descriptors: u32,

    /// Thread count
    pub thread_count: u32,

    /// Operations per second
    pub ops_per_second: f32,

    /// Last update timestamp
    pub last_update: Option<SystemTime>,
}

/// Violation counter
#[derive(Debug, Clone, Default)]
pub struct ViolationCounter {
    /// Total violations
    pub total: u32,

    /// Violations by type
    pub by_type: HashMap<String, u32>,

    /// Last violation time
    pub last_violation: Option<SystemTime>,
}

/// Trait for capability validation
#[async_trait::async_trait]
pub trait CapabilityValidator: Send + Sync {
    /// Validate a capability request
    async fn validate(
        &self,
        extension_id: &str,
        capability: &ExtensionCapability,
        operation: &str,
    ) -> ExtensionResult<()>;
}

/// Extension history for anomaly detection
#[derive(Debug, Clone, Default)]
pub struct ExtensionHistory {
    /// CPU usage history
    pub cpu_history: Vec<f32>,

    /// Memory usage history
    pub memory_history: Vec<u64>,

    /// Operations per second history
    pub ops_history: Vec<f32>,
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

    /// Severity
    pub severity: f32,
}

/// Detected anomaly
#[derive(Debug, Clone)]
pub struct Anomaly {
    /// Anomaly type
    pub anomaly_type: String,

    /// Severity (0.0 - 1.0)
    pub severity: f32,

    /// Description
    pub description: String,

    /// Recommended action
    pub action: AnomalyAction,
}

/// Actions for detected anomalies
#[derive(Debug, Clone)]
pub enum AnomalyAction {
    /// Log and alert
    Alert,

    /// Throttle the extension
    Throttle,

    /// Suspend the extension
    Suspend,

    /// Terminate the extension
    Terminate,
}

/// Audit log entry
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// Timestamp
    pub timestamp: SystemTime,

    /// Extension ID
    pub extension_id: String,

    /// Event type
    pub event_type: AuditEventType,

    /// Operation performed
    pub operation: String,

    /// Result of the operation
    pub result: AuditResult,

    /// Additional details
    pub details: Option<String>,
}

/// Audit event types
#[derive(Debug, Clone)]
pub enum AuditEventType {
    /// Extension loaded
    ExtensionLoaded,

    /// Extension unloaded
    ExtensionUnloaded,

    /// Capability request
    CapabilityRequest,

    /// Resource allocation
    ResourceAllocation,

    /// Configuration change
    ConfigChange,

    /// Security violation
    SecurityViolation,

    /// Anomaly detected
    AnomalyDetected,
}

/// Audit result
#[derive(Debug, Clone)]
pub enum AuditResult {
    /// Operation succeeded
    Success,

    /// Operation failed
    Failure(String),

    /// Operation denied
    Denied(String),
}

/// Trait for anomaly detection rules
#[async_trait::async_trait]
pub trait AnomalyRule: Send + Sync {
    /// Check for anomalies in current usage
    async fn check(
        &self,
        extension_id: &str,
        current: &ResourceUsage,
        history: &ExtensionHistory,
    ) -> Option<Anomaly>;
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
