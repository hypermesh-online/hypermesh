// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Resource monitoring and anomaly detection for extensions.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::{RwLock, Semaphore};
use tracing::warn;

use super::super::ExtensionError;
use super::types::*;

/// Resource monitor for tracking extension resource usage
pub struct ResourceMonitor {
    /// Extension ID
    pub(crate) _extension_id: String,

    /// Resource quotas
    pub(crate) quotas: ResourceQuotas,

    /// Current resource usage
    pub(crate) usage: Arc<RwLock<ResourceUsage>>,

    /// Rate limiter
    pub(crate) rate_limiter: Arc<Semaphore>,

    /// Violation counter
    pub(crate) violations: Arc<RwLock<ViolationCounter>>,
}

impl ResourceMonitor {
    /// Create new resource monitor
    pub fn new(extension_id: String, quotas: ResourceQuotas) -> Self {
        let rate_limiter = Arc::new(Semaphore::new(quotas.ops_per_second as usize));

        Self {
            _extension_id: extension_id,
            quotas,
            usage: Arc::new(RwLock::new(ResourceUsage::default())),
            rate_limiter,
            violations: Arc::new(RwLock::new(ViolationCounter::default())),
        }
    }

    /// Check if resource usage is within quotas
    pub async fn check_quotas(&self) -> super::super::ExtensionResult<()> {
        let usage = self.usage.read().await;

        if usage.cpu_percent > self.quotas.cpu_percent {
            return Err(ExtensionError::ResourceLimitExceeded {
                resource: format!(
                    "CPU: {:.1}% > {:.1}%",
                    usage.cpu_percent, self.quotas.cpu_percent
                ),
            });
        }

        if usage.memory_bytes > self.quotas.memory_bytes {
            return Err(ExtensionError::ResourceLimitExceeded {
                resource: format!(
                    "Memory: {} > {}",
                    usage.memory_bytes, self.quotas.memory_bytes
                ),
            });
        }

        if usage.storage_bytes > self.quotas.storage_bytes {
            return Err(ExtensionError::ResourceLimitExceeded {
                resource: format!(
                    "Storage: {} > {}",
                    usage.storage_bytes, self.quotas.storage_bytes
                ),
            });
        }

        if usage.file_descriptors > self.quotas.file_descriptors {
            return Err(ExtensionError::ResourceLimitExceeded {
                resource: format!(
                    "FDs: {} > {}",
                    usage.file_descriptors, self.quotas.file_descriptors
                ),
            });
        }

        if usage.thread_count > self.quotas.max_threads {
            return Err(ExtensionError::ResourceLimitExceeded {
                resource: format!(
                    "Threads: {} > {}",
                    usage.thread_count, self.quotas.max_threads
                ),
            });
        }

        Ok(())
    }

    /// Update resource usage
    pub async fn update_usage(
        &self,
        new_usage: ResourceUsage,
    ) -> super::super::ExtensionResult<()> {
        let mut usage = self.usage.write().await;
        *usage = new_usage;
        usage.last_update = Some(SystemTime::now());
        Ok(())
    }

    /// Record a violation
    pub async fn record_violation(&self, violation_type: &str, _details: &str) {
        let mut violations = self.violations.write().await;
        violations.total += 1;
        *violations
            .by_type
            .entry(violation_type.to_string())
            .or_insert(0) += 1;
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
    pub async fn acquire_permit(&self) -> super::super::ExtensionResult<()> {
        match self.rate_limiter.try_acquire() {
            Ok(permit) => {
                drop(permit);
                Ok(())
            }
            Err(_) => Err(ExtensionError::ResourceLimitExceeded {
                resource: format!("Rate limit: {} ops/sec", self.quotas.ops_per_second),
            }),
        }
    }
}

/// Anomaly detector for extension behavior analysis
pub struct AnomalyDetector {
    /// Detection rules
    pub(crate) rules: Vec<Box<dyn AnomalyRule>>,

    /// Extension history
    pub(crate) history: Arc<RwLock<HashMap<String, ExtensionHistory>>>,

    /// Alert threshold
    pub(crate) alert_threshold: f32,
}

impl Default for AnomalyDetector {
    fn default() -> Self {
        Self::new()
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
                }
            }
        }
    }
}

/// CPU anomaly detection rule
pub struct CPUAnomalyRule {
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
        let variance: f32 = history
            .cpu_history
            .iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f32>()
            / history.cpu_history.len() as f32;
        let std_dev = variance.sqrt();

        if (current.cpu_percent - mean).abs() > self.threshold * std_dev {
            return Some(Anomaly {
                anomaly_type: "CPU Usage Spike".to_string(),
                severity: ((current.cpu_percent - mean).abs() / (self.threshold * std_dev))
                    .min(1.0),
                description: format!(
                    "CPU usage {:.1}% deviates from mean {:.1}% by {:.1} std devs",
                    current.cpu_percent,
                    mean,
                    (current.cpu_percent - mean).abs() / std_dev
                ),
                action: AnomalyAction::Alert,
            });
        }

        None
    }
}

/// Memory anomaly detection rule
pub struct MemoryAnomalyRule {
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
        let threshold_f64 = self.threshold as f64;

        if growth_rate > threshold_f64 {
            return Some(Anomaly {
                anomaly_type: "Memory Leak".to_string(),
                severity: (growth_rate / threshold_f64).min(1.0) as f32,
                description: format!(
                    "Memory usage grew by {:.1}% (from {} to {})",
                    growth_rate * 100.0,
                    prev,
                    current.memory_bytes
                ),
                action: if growth_rate > threshold_f64 * 2.0 {
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
                    current.ops_per_second,
                    current.ops_per_second / mean,
                    mean
                ),
                action: AnomalyAction::Throttle,
            });
        }

        None
    }
}
