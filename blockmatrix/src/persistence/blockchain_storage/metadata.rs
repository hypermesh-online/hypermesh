// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Blockchain metadata and query parameter types.

use serde::{Deserialize, Serialize};

/// Blockchain metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainMetadata {
    /// Genesis block hash
    pub genesis_hash: String,
    /// Current chain height
    pub chain_height: u64,
    /// Last block hash
    pub last_block_hash: String,
    /// Total blocks
    pub total_blocks: u64,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last modified
    pub last_modified: chrono::DateTime<chrono::Utc>,
}

/// Block query parameters
#[derive(Debug, Clone)]
pub enum BlockQuery {
    /// Query by index
    ByIndex(u64),
    /// Query by hash
    ByHash(String),
    /// Query range of indices
    Range(u64, u64),
    /// Get last N blocks
    Last(u64),
}
