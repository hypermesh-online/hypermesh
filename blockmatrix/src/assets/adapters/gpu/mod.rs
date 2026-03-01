// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! GPU Asset Adapter with Nova engine and Vulkan compute management
//!
//! Features:
//! - Nova engine GPU compute unit allocation (Vulkan compute shaders)
//! - Vulkan-based memory management (device memory, buffers)
//! - Multi-GPU coordination and scheduling via Nova
//! - Hardware acceleration for consensus proofs
//! - Quantum-resistant security with FALCON-1024
//! - Remote proxy access for distributed GPU compute

mod adapter;
mod detection;
mod operations;
mod types;

pub use types::*;

// detection and operations only provide impl blocks, no additional exports needed

#[cfg(test)]
mod tests;
