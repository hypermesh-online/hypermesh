// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Predictive container scaling with CPE-enhanced decisions.

pub mod engine;
pub mod types;

// Re-export all public types
pub use engine::PredictiveScaler;
pub use types::*;
