// Written by Richard Christopher, Copyright 2026 HyperMesh Foundation
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Placement — the identity / location boundary NGauge owns.
//!
//! Identity is durable (the asset's content hash — an `AssetAddress` fingerprint
//! that never moves); location is elastic and demand-driven (VISION.md §5.5).
//! [`lease`] names that boundary as a type, [`PlacementLease`]. [`locality`]
//! supplies the *placement* coordinate from a real proximity metric (measured
//! peer RTT) so matrix distance is physically meaningful — the precondition for
//! demand clustering, and the replacement for the
//! identity-derived (uniform-random) cell as a location source.

pub mod lease;
pub mod locality;
pub mod replica_selection;

pub use lease::PlacementLease;
pub use locality::{LocalityProvider, PeerProximity};
pub use replica_selection::{
    order_by_proximity, FallbackStrategy, ReplicaCandidate, ReplicaSelector, SelectionCriteria,
};
