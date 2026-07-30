// Written by Richard Christopher, Copyright 2026 HyperMesh Foundation
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Chunk arithmetic — a network's elastic slice of the matrix.
//!
//! A **chunk** is the set of matrix cells a network currently occupies — the
//! coordinates of its members. Membership resizes, so a chunk is elastic, never
//! a fixed cube; this module is the minimal REAL geometry over that set, and
//! nothing more.
//!
//! It runs on the real primitives only — an integer [`MatrixCoordinate`]
//! centroid and [`find_k_nearest`] over those cells — fed the **proximity-
//! derived** coordinates P3 made meaningful (measured QUIC RTT, not the
//! identity-hash cell). Membership itself (which shard belongs to which
//! network) is owned upstream by the network model; this is only the geometry
//! of a network's slice, kept on the BlockMatrix side because the matrix primitives
//! (`MatrixCoordinate`, `neighbors`) live here.
//!
//! Scope discipline (this is P7): exactly one consumed operation is built.
//! [`most_central`] aggregates a shard's demand (its candidate providers) into
//! a chunk and returns the geometrically central one. Spread/extent and
//! inter-chunk distance are intentionally absent — no caller needs them yet, and
//! an unconsumed operation is the decorative tensor code P7 removes.

use crate::matrix::coordinate::MatrixCoordinate;
use crate::matrix::neighbors::find_k_nearest;

/// A network's slice of the matrix: the coordinates of its member cells.
#[derive(Debug, Clone, Default)]
pub struct Chunk {
    members: Vec<MatrixCoordinate>,
}

impl Chunk {
    /// Build a chunk from the coordinates of a network's members.
    pub fn from_coords(members: Vec<MatrixCoordinate>) -> Self {
        Self { members }
    }

    /// Whether the chunk holds no cells.
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Number of cells in the chunk.
    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// The chunk's integer centroid — the component-wise mean cell, the single
    /// point that best represents where the network's slice sits in the matrix.
    /// `None` for an empty chunk.
    ///
    /// Accumulates in `i128` so a large membership cannot overflow; each mean
    /// component is back within `MatrixCoordinate` bounds, so `new` never fails
    /// and the origin fallback is defensive only.
    pub fn centroid(&self) -> Option<MatrixCoordinate> {
        if self.members.is_empty() {
            return None;
        }
        let n = self.members.len() as i128;
        let (mut sx, mut sy, mut sz) = (0i128, 0i128, 0i128);
        for m in &self.members {
            sx += i128::from(m.x);
            sy += i128::from(m.y);
            sz += i128::from(m.z);
        }
        Some(
            MatrixCoordinate::new((sx / n) as i64, (sy / n) as i64, (sz / n) as i64)
                .unwrap_or_else(|_| MatrixCoordinate::origin()),
        )
    }

    /// The member cell nearest `point`, via [`find_k_nearest`] (k = 1). `None`
    /// for an empty chunk.
    pub fn nearest_member(&self, point: &MatrixCoordinate) -> Option<MatrixCoordinate> {
        find_k_nearest(point, &self.members, 1)
            .first()
            .map(|(coord, _dist)| *coord)
    }
}

/// Pick the geometrically **central** provider of a shard's demand.
///
/// The candidate providers' proximity coordinates (P3) form the shard's demand
/// chunk; this returns the candidate whose cell is nearest that chunk's
/// centroid — the most representative point of where the demand actually sits.
/// It is the topology-aware replacement for an arbitrary lexical pick when the
/// swarm analytics offer no demand recommendation.
///
/// Returns `None` when no candidate carries a coordinate, so the caller can
/// fall back to its deterministic last resort.
pub fn most_central<'a>(candidates: &'a [(String, MatrixCoordinate)]) -> Option<&'a String> {
    let chunk = Chunk::from_coords(candidates.iter().map(|(_, coord)| *coord).collect());
    let centroid = chunk.centroid()?;
    let target = chunk.nearest_member(&centroid)?;
    candidates
        .iter()
        .find(|(_, coord)| *coord == target)
        .map(|(id, _)| id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ngauge::{LocalityProvider, PeerProximity};

    /// Round a proximity position (continuous RTT-derived) to an integer cell,
    /// exactly as the placement path does.
    fn cell(p: hypermesh_lib::MatrixPosition) -> MatrixCoordinate {
        MatrixCoordinate::new(p.x.round() as i64, p.y.round() as i64, p.z.round() as i64)
            .unwrap_or_else(|_| MatrixCoordinate::origin())
    }

    #[test]
    fn empty_chunk_has_no_centroid_or_nearest() {
        let chunk = Chunk::from_coords(Vec::new());
        assert!(chunk.is_empty());
        assert!(chunk.centroid().is_none());
        assert!(chunk.nearest_member(&MatrixCoordinate::origin()).is_none());
    }

    #[test]
    fn centroid_is_the_component_mean() {
        let chunk = Chunk::from_coords(vec![
            MatrixCoordinate::new(0, 0, 0).expect("test: valid coord"),
            MatrixCoordinate::new(10, 20, 30).expect("test: valid coord"),
            MatrixCoordinate::new(20, 40, 60).expect("test: valid coord"),
        ]);
        assert_eq!(
            chunk.centroid(),
            Some(MatrixCoordinate::new(10, 20, 30).expect("test: valid coord"))
        );
    }

    #[test]
    fn chunk_centroid_reflects_real_proximity_geometry() {
        // P3 makes coordinates proximity-real: a peer's cell derives from its
        // measured RTT. Build a dense LAN band (~0.5 ms) plus one distant WAN
        // outlier (~80 ms) and take the chunk over their real placement cells.
        // The centroid must sit inside the dense band, not be dragged out to the
        // midpoint the raw outlier would imply — proof the chunk arithmetic runs
        // on the genuine metric, not the uniform-random identity cell.
        let mut samples = Vec::new();
        for i in 0..8 {
            samples.push(PeerProximity::new(format!("lan-{i}"), 500 + i as u64 * 20));
        }
        samples.push(PeerProximity::new("wan-outlier", 80_000));
        let provider = LocalityProvider::from_samples(samples);

        let cells: Vec<MatrixCoordinate> =
            provider.placement_coordinates().into_iter().map(cell).collect();
        let chunk = Chunk::from_coords(cells);
        let centroid = chunk.centroid().expect("test: populated chunk has a centroid");

        // The dense LAN band sits ~5 units from origin (0.5 ms / 0.1 ms); the
        // outlier ~800. Nine members, one outlier ⇒ mean well under a quarter of
        // the outlier's radius — inside the band's neighbourhood, not halfway.
        let origin = MatrixCoordinate::origin();
        let centroid_radius_sq = origin.squared_euclidean_distance(&centroid);
        assert!(
            centroid_radius_sq < 200 * 200,
            "centroid must sit near the dense proximity band, radius^2 = {centroid_radius_sq}"
        );

        // nearest_member to the centroid must be a LAN cell (small radius), not
        // the WAN outlier — the find_k_nearest primitive over the real metric.
        let nearest = chunk
            .nearest_member(&centroid)
            .expect("test: populated chunk has a nearest member");
        assert!(
            origin.squared_euclidean_distance(&nearest) < 200 * 200,
            "central member must be a LAN cell, not the WAN outlier"
        );
    }

    #[test]
    fn most_central_picks_the_geometric_center_consumer() {
        // Consumer path: three providers form a demand chunk; the one at the
        // geometric center is chosen over the two edges — and over an arbitrary
        // lexical order (here the center id is NOT the lexical minimum).
        let candidates = vec![
            ("z-left".to_string(), MatrixCoordinate::new(-100, 0, 0).expect("test: coord")),
            ("m-center".to_string(), MatrixCoordinate::new(0, 0, 0).expect("test: coord")),
            ("a-right".to_string(), MatrixCoordinate::new(100, 0, 0).expect("test: coord")),
        ];
        let chosen = most_central(&candidates).expect("test: coordinates present");
        assert_eq!(chosen, "m-center", "central provider wins, not the lexical min");
    }

    #[test]
    fn most_central_none_when_no_coordinates() {
        assert!(most_central(&[]).is_none());
    }
}
