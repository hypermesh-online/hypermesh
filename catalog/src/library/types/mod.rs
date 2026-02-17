// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Core types for the asset library
//!
//! Lightweight types optimized for in-memory operations and HyperMesh integration.
//! These types are designed to be zero-copy where possible and minimize allocations.
//!
//! MIGRATION: This module now wraps the Asset Registry architecture for backward compatibility.

pub mod core;
pub mod metadata;
pub mod validation;

// Re-export everything publicly for backward compatibility
pub use self::core::*;
pub use self::metadata::*;
// validation module adds impl blocks on LibraryAssetPackage, no separate types to re-export
