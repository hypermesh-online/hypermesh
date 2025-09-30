//! STOQ Configuration module
//! 
//! Provides configuration structures for pure STOQ transport protocol

use serde::{Serialize, Deserialize};

// Re-export transport config only
pub use crate::transport::TransportConfig;

/// Pure STOQ transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoqConfig {
    /// Transport layer configuration
    pub transport: TransportConfig,
}

impl Default for StoqConfig {
    fn default() -> Self {
        Self {
            transport: TransportConfig::default(),
        }
    }
}

