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

#[cfg(test)]
pub mod tests;

// Re-export main types from config module
pub use config::SecurityConfig;

use anyhow::Result;

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
    pub fn validate(&self) -> Result<()> {
        // Add validation logic here
        Ok(())
    }
}