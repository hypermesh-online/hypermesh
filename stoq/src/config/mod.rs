// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ Configuration module
//!
//! Provides configuration structures for pure STOQ transport protocol

use serde::{Deserialize, Serialize};

// Re-export transport config only
pub use crate::transport::TransportConfig;

/// Pure STOQ transport configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StoqConfig {
    /// Transport layer configuration
    pub transport: TransportConfig,
}
