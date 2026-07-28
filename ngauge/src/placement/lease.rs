// Written by Richard Christopher, Copyright 2026 HyperMesh Foundation
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! The identity / placement boundary — named as a type.
//!
//! HyperMesh conflates two different questions in one place today: the
//! `AssetAddress` packs a matrix cell *into* the address, and that cell is
//! `derive_cell(node_id) = BLAKE3(node_id)` — a uniform-random point with no
//! locality. That derivation answers *what an asset IS* (a durable, verifiable
//! identity fingerprint). It does **not** answer *where the asset currently
//! lives* — that is an elastic, demand-driven placement decision NGauge owns
//! (VISION.md §5.5: identity is durable, location is elastic and NGauge-owned).
//!
//! [`PlacementLease`] is the named seam between those two. It pairs a durable
//! identity ([`content_hash`](PlacementLease::content_hash)) with the elastic
//! coordinate NGauge currently places it at, scoped to the world (network) the
//! placement belongs to and ordered by a demand [`priority`] scheduler hint.
//!
//! It is deliberately **unused** at this stage. P1 introduces the boundary as a
//! type + documentation only; it does not change any placement behavior and does
//! not touch address bytes. Live placement decisions consume `PlacementLease` in
//! P2/P3/P4.
//!
//! ## `priority` is a scheduler hint, NOT a Proof-of-State magnitude
//!
//! [`priority`](PlacementLease::priority) is a demand signal — the same kind of
//! scalar a torrent uses to seed hot content first. It orders placement work; it
//! never gates PoS admission and never lands in a `StateProof`. PoStake is
//! authorization (WHO), never a magnitude — see
//! `scripts/check-no-pos-magnitude.sh`.

use hypermesh_lib::{ContentHash, MatrixPosition, NetworkId};
use serde::{Deserialize, Serialize};

/// A demand-driven placement of one asset within one world.
///
/// This is the boundary between *identity* (durable, verifiable, never moves —
/// [`content_hash`](Self::content_hash)) and *location* (elastic, demand-driven,
/// NGauge-owned — [`coordinate`](Self::coordinate)). The address's packed cell is
/// an identity fingerprint; the lease is where the asset actually lives *now*.
///
/// The coordinate type is [`MatrixPosition`] to stay consistent with NGauge's
/// existing placement APIs — `swarm_analytics` (`register_replica`,
/// `DispersionAdvisor::recommend`) and `routing_intel` both express placement in
/// `MatrixPosition`, so leases speak the same coordinate as the code that will
/// consume them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlacementLease {
    /// The asset's durable identity — the BLAKE3 content hash. This is the thing
    /// that never moves; a lease may be re-issued at a new coordinate but the
    /// `content_hash` it names is invariant.
    content_hash: ContentHash,

    /// Which world (network) this placement belongs to. Worlds are emergent,
    /// nestable sub-meshes (VISION.md §5.5); the same asset identity can hold
    /// distinct leases in distinct worlds.
    world_id: NetworkId,

    /// The matrix coordinate where NGauge currently places the asset. Elastic:
    /// NGauge may re-place the asset (extend/partition worlds, re-replicate) and
    /// issue a fresh lease at a different coordinate for the same identity.
    coordinate: MatrixPosition,

    /// Demand / priority signal — a scheduler hint (like a torrent seed
    /// priority), higher means place/replicate sooner. NOT a PoS magnitude: it
    /// never gates admission and never enters a `StateProof`.
    priority: f64,
}

impl PlacementLease {
    /// Create a lease binding a durable identity to an elastic coordinate within
    /// one world, ordered by a demand `priority` hint.
    pub fn new(
        content_hash: ContentHash,
        world_id: NetworkId,
        coordinate: MatrixPosition,
        priority: f64,
    ) -> Self {
        Self {
            content_hash,
            world_id,
            coordinate,
            priority,
        }
    }

    /// The asset's durable identity (BLAKE3 content hash). Invariant across
    /// re-placement.
    pub fn content_hash(&self) -> &ContentHash {
        &self.content_hash
    }

    /// The world (network) this placement belongs to.
    pub fn world_id(&self) -> &NetworkId {
        &self.world_id
    }

    /// The matrix coordinate NGauge currently places the asset at. Elastic.
    pub fn coordinate(&self) -> &MatrixPosition {
        &self.coordinate
    }

    /// The demand `priority` scheduler hint. Not a PoS magnitude.
    pub fn priority(&self) -> f64 {
        self.priority
    }

    /// Re-place the asset at a new coordinate, keeping its identity, world, and
    /// priority. Location is elastic; identity is durable.
    pub fn with_coordinate(self, coordinate: MatrixPosition) -> Self {
        Self { coordinate, ..self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> PlacementLease {
        PlacementLease::new(
            ContentHash::from_bytes([7u8; 32]),
            NetworkId([3u8; 16]),
            MatrixPosition {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            0.75,
        )
    }

    #[test]
    fn accessors_return_constructed_values() {
        let lease = sample();
        assert_eq!(lease.content_hash(), &ContentHash::from_bytes([7u8; 32]));
        assert_eq!(lease.world_id(), &NetworkId([3u8; 16]));
        assert_eq!(lease.coordinate().x, 1.0);
        assert_eq!(lease.priority(), 0.75);
    }

    #[test]
    fn re_placement_keeps_identity_changes_location() {
        let lease = sample();
        let moved = lease.clone().with_coordinate(MatrixPosition {
            x: 9.0,
            y: 9.0,
            z: 9.0,
        });
        // Identity durable, location elastic.
        assert_eq!(moved.content_hash(), lease.content_hash());
        assert_eq!(moved.world_id(), lease.world_id());
        assert_eq!(moved.priority(), lease.priority());
        assert_eq!(moved.coordinate().x, 9.0);
    }

    #[test]
    fn serde_round_trips() {
        let lease = sample();
        let json = serde_json::to_string(&lease).expect("test: serialize lease");
        let back: PlacementLease = serde_json::from_str(&json).expect("test: deserialize lease");
        assert_eq!(lease, back);
    }
}
