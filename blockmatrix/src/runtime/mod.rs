// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! HyperMesh Runtime Components
//!
//! Provides runtime services and SDKs for HyperMesh applications.

pub mod phoenix;

// Re-export Phoenix SDK types
pub use phoenix::{
    PhoenixTransport, PhoenixConfig, PhoenixConnection,
    PerformanceMetrics, PhoenixBuilder,
};