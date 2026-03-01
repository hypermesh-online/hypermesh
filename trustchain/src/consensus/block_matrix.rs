// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Block Matrix
//!
//! Block matrix for consensus operations.

use serde::{Deserialize, Serialize};

/// Block matrix (placeholder)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockMatrix;

impl BlockMatrix {
    pub fn new() -> Self {
        Self
    }
}

impl Default for BlockMatrix {
    fn default() -> Self {
        Self::new()
    }
}
