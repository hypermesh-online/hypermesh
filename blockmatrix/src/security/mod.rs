// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Security module for HyperMesh
//!
//! Provides security abstractions and policies for the HyperMesh system.

pub mod config;
pub mod error;
pub mod certificates;
pub mod capabilities;
pub mod ebpf;
pub mod intrusion;
pub mod monitoring;
pub mod policies;
pub mod types;

#[cfg(test)]
pub mod tests;

// Re-export main types from config module
pub use config::{
    SecurityConfig, EBPFConfig, CertificateConfig, CapabilityConfig,
    IntrusionDetectionConfig, PolicyConfig, MonitoringConfig,
};

// Re-export error types
pub use error::{SecurityError, Result};

// Re-export core security types
pub use types::{
    HyperMeshSecurity, SecurityContext, Principal, Resource, Operation,
    AccessDecision, SeverityLevel, NetworkPacket, SystemCall, ProcessContext,
    SecurityEvent,
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

    /// Validate security configuration
    // STUB: Security validation not implemented
    // TODO: Implement actual security configuration validation
    // Priority: HIGH - Required for Option 2 (Security Layer)
    pub fn validate(&self) -> Result<()> {
        // Add validation logic here
        Ok(())
    }
}