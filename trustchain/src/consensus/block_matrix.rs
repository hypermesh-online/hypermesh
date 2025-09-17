//! Block Matrix
//! 
//! Block matrix for consensus operations.

use serde::{Serialize, Deserialize};

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