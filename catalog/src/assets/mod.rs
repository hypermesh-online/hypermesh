// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Core Asset Package System
//!
//! Provides the foundational asset package format parsing, validation, and management
//! for the Catalog asset library ecosystem.

pub mod operations;
pub mod registry;
pub mod types;

// Re-export everything publicly for backward compatibility
pub use registry::*;
pub use types::*;
// operations module adds impl blocks on AssetPackage, no separate types to re-export
