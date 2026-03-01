// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Security module for HyperMesh
//!
//! Provides security abstractions and policies for the HyperMesh system.

pub mod capabilities;
pub mod certificates;
pub mod config;
pub mod ebpf;
pub mod error;
pub mod intrusion;
pub mod monitoring;
pub mod policies;
pub mod types;

#[cfg(test)]
pub mod tests;

// Re-export main types from config module
pub use config::{
    CapabilityConfig, CertificateConfig, EBPFConfig, IntrusionDetectionConfig, MonitoringConfig,
    PolicyConfig, SecurityConfig,
};

// Re-export error types
pub use error::{Result, SecurityError};

// Re-export core security types
pub use types::{
    AccessDecision, HyperMeshSecurity, NetworkPacket, Operation, Principal, ProcessContext,
    Resource, SecurityContext, SecurityEvent, SeverityLevel, SystemCall,
};

/// Security manager for HyperMesh
pub struct SecurityManager {
    config: SecurityConfig,
}

impl SecurityManager {
    /// Create new security manager
    pub fn new(config: SecurityConfig) -> Self {
        Self { config }
    }

    /// Validate security configuration.
    ///
    /// Checks that all security subsystems are consistently configured:
    /// - eBPF config has a valid program directory when enabled
    /// - Certificate config has non-zero validity periods
    /// - Policy evaluation mode is a recognized value
    /// - Monitoring collection interval is reasonable
    pub fn validate(&self) -> Result<()> {
        // Validate eBPF configuration
        if self.config.ebpf.enabled && self.config.ebpf.program_dir.as_os_str().is_empty() {
            return Err(SecurityError::ConfigurationError {
                message: "eBPF enabled but program_dir is empty".into(),
            });
        }

        // Validate certificate lifecycle
        if self.config.certificates.lifecycle.default_validity_days == 0 {
            return Err(SecurityError::ConfigurationError {
                message: "Certificate default_validity_days must be > 0".into(),
            });
        }

        if self
            .config
            .certificates
            .lifecycle
            .maximum_certificate_age_days
            < self.config.certificates.lifecycle.default_validity_days
        {
            return Err(SecurityError::ConfigurationError {
                message: format!(
                    "maximum_certificate_age_days ({}) < default_validity_days ({})",
                    self.config
                        .certificates
                        .lifecycle
                        .maximum_certificate_age_days,
                    self.config.certificates.lifecycle.default_validity_days,
                ),
            });
        }

        // Validate policy evaluation mode
        let valid_modes = ["enforcing", "permissive", "disabled"];
        if !valid_modes.contains(&self.config.policies.evaluation_mode.as_str()) {
            return Err(SecurityError::ConfigurationError {
                message: format!(
                    "Unknown policy evaluation mode '{}'; expected one of {:?}",
                    self.config.policies.evaluation_mode, valid_modes
                ),
            });
        }

        // Validate monitoring interval (at least 1 second)
        if self.config.monitoring.enabled
            && self.config.monitoring.collection_interval.as_secs() == 0
        {
            return Err(SecurityError::ConfigurationError {
                message: "Monitoring collection_interval must be >= 1 second".into(),
            });
        }

        Ok(())
    }
}
