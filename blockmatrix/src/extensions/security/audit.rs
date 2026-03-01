// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Audit logging for extension security events.

use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

use super::types::*;

/// Audit logger for security events
pub struct AuditLogger {
    /// Audit entries
    pub(crate) entries: Arc<RwLock<Vec<AuditEntry>>>,

    /// Audit configuration
    pub(crate) config: AuditConfig,
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
        let retention_cutoff =
            SystemTime::now() - Duration::from_secs(self.config.retention_days as u64 * 86400);

        entries.retain(|e| e.timestamp > retention_cutoff);
    }

    /// Get audit entries for extension
    pub async fn get_entries(&self, extension_id: &str) -> Vec<AuditEntry> {
        let entries = self.entries.read().await;
        entries
            .iter()
            .filter(|e| e.extension_id == extension_id)
            .cloned()
            .collect()
    }
}
