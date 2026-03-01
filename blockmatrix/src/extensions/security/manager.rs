// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! SecurityManager - core security context management and capability enforcement.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tracing::{info, warn};

use super::super::{ExtensionCapability, ExtensionError, ExtensionMetadata, ExtensionResult};

use super::audit::AuditLogger;
use super::monitoring::{AnomalyDetector, ResourceMonitor};
use super::types::*;

/// Security manager for extension runtime security
pub struct SecurityManager {
    /// Security contexts by extension ID
    pub(crate) contexts: Arc<RwLock<HashMap<String, SecurityContext>>>,

    /// Resource monitors by extension ID
    pub(crate) monitors: Arc<RwLock<HashMap<String, ResourceMonitor>>>,

    /// Capability validators
    pub(crate) validators: Arc<RwLock<HashMap<ExtensionCapability, Box<dyn CapabilityValidator>>>>,

    /// Anomaly detector
    pub(crate) anomaly_detector: Arc<AnomalyDetector>,

    /// Audit logger
    pub(crate) audit_logger: Arc<AuditLogger>,

    /// Configuration
    pub(crate) config: SecurityConfig,
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
        let context =
            contexts
                .get(extension_id)
                .ok_or_else(|| ExtensionError::ExtensionNotFound {
                    id: extension_id.to_string(),
                })?;

        // Check if capability is granted
        if !context.capabilities.contains(capability) {
            self.audit_logger
                .log(AuditEntry {
                    timestamp: SystemTime::now(),
                    extension_id: extension_id.to_string(),
                    event_type: AuditEventType::CapabilityRequest,
                    operation: operation.to_string(),
                    result: AuditResult::Denied(format!("Capability not granted: {capability:?}")),
                    details: None,
                })
                .await;

            return Err(ExtensionError::CapabilityNotGranted {
                capability: format!("{capability:?}"),
            });
        }

        // Additional validation if validator exists
        let validators = self.validators.read().await;
        if let Some(validator) = validators.get(capability) {
            validator
                .validate(extension_id, capability, operation)
                .await?;
        }

        // Audit successful check
        self.audit_logger
            .log(AuditEntry {
                timestamp: SystemTime::now(),
                extension_id: extension_id.to_string(),
                event_type: AuditEventType::CapabilityRequest,
                operation: operation.to_string(),
                result: AuditResult::Success,
                details: None,
            })
            .await;

        Ok(())
    }

    /// Check resource usage against quotas
    pub async fn check_resource_usage(&self, extension_id: &str) -> ExtensionResult<()> {
        if !self.config.enforcement_enabled {
            return Ok(());
        }

        let monitors = self.monitors.read().await;
        let monitor =
            monitors
                .get(extension_id)
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
                self.anomaly_detector.check(extension_id, monitor).await;
            }
        }

        Ok(())
    }

    /// Record security violation
    pub async fn record_violation(&self, extension_id: &str, violation_type: &str, details: &str) {
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
            }
        }

        // Audit the violation
        self.audit_logger
            .log(AuditEntry {
                timestamp: SystemTime::now(),
                extension_id: extension_id.to_string(),
                event_type: AuditEventType::SecurityViolation,
                operation: violation_type.to_string(),
                result: AuditResult::Failure(details.to_string()),
                details: None,
            })
            .await;
    }

    /// Get security metrics for extension
    pub async fn get_metrics(&self, extension_id: &str) -> Option<SecurityMetrics> {
        let monitors = self.monitors.read().await;
        let monitor = monitors.get(extension_id)?;

        Some(monitor.get_metrics().await)
    }
}
