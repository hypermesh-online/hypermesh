// Written by Richard Christopher, Copyright 2026 HyperMesh Foundation
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Placement — the identity / location boundary NGauge owns.
//!
//! Identity is durable (the asset's content hash — an `AssetAddress` fingerprint
//! that never moves); location is elastic and demand-driven (VISION.md §5.5).
//! This module names that boundary as a type, [`PlacementLease`], without yet
//! changing any placement behavior. See [`lease`] for the full rationale.

pub mod lease;

pub use lease::PlacementLease;
