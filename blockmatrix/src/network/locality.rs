// Written by Richard Christopher, Copyright 2026 HyperMesh Foundation
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Live proximity → placement glue (P3).
//!
//! This is the crate that owns both halves of the proximity signal: the live
//! `stoq::Connection` (which carries QUIC's measured RTT) and the geographic
//! clustering engine. It bridges them so the placement layer stops clustering
//! over the identity-derived (uniform-random) cell and starts clustering over a
//! real proximity metric (VISION.md §5.5).
//!
//! Two responsibilities, kept small:
//!
//! 1. [`provider_from_nodes`] reads the measured RTT off every live peer
//!    connection and hands NGauge a populated [`LocalityProvider`]. That is the
//!    ingest of the real signal.
//! 2. [`locality_centroids`] runs [`GeographicClustering`] over the
//!    proximity-embedded peer coordinates to find locality centroids — the first
//!    real runtime caller of that previously-dormant engine, fed the real
//!    signal. Demand-driven placement builds on these centroids.
//!
//! Identity/address derivation (`base::derive_cell`) is untouched — this only
//! decides *where* assets live, never *what* they are.

use hypermesh_lib::MatrixPosition;
use ngauge::LocalityProvider;

use crate::matrix::coordinate::MatrixCoordinate;
use crate::matrix::geospatial::GeographicClustering;
use crate::network::NetworkNode;

/// Build a [`LocalityProvider`] from the measured RTT of every live peer
/// connection.
///
/// A peer contributes a sample only when it has an active connection (so the
/// RTT is a real, current measurement). Peers without a live connection are
/// simply absent — `coordinate_for` returns `None` for them and the caller
/// falls back to the deterministic `derive_cell` coordinate (cold start).
pub fn provider_from_nodes(nodes: &[NetworkNode]) -> LocalityProvider {
    let mut provider = LocalityProvider::new();
    for node in nodes {
        let Some(conn) = node.connection.as_ref() else {
            continue;
        };
        if !conn.is_active() {
            continue;
        }
        let rtt_micros = conn.rtt().as_micros().min(u128::from(u64::MAX)) as u64;
        provider.observe(node.node_id.clone(), rtt_micros);
    }
    provider
}

/// Cluster the proximity-embedded peer coordinates into up to `k` locality
/// centroids using [`GeographicClustering`] (k-means).
///
/// This is the promotion of `GeographicClustering` from a dormant, tested-only
/// engine to a live caller: it now runs over coordinates whose distances carry a
/// real proximity metric, so its centroids describe genuine network localities
/// rather than random hash points. Returns an empty vector on cold start (no
/// samples) — never panics.
pub fn locality_centroids(provider: &LocalityProvider, k: usize) -> Vec<MatrixPosition> {
    if provider.is_empty() || k == 0 {
        return Vec::new();
    }

    let coords: Vec<MatrixCoordinate> = provider
        .placement_coordinates()
        .into_iter()
        .map(position_to_coordinate)
        .collect();

    let mut clustering = GeographicClustering::new();
    clustering.kmeans(&coords, k, 25);

    clustering
        .get_clusters()
        .into_iter()
        .map(|c| coordinate_to_position(&c.centroid))
        .collect()
}

/// Round a floating [`MatrixPosition`] into an integer [`MatrixCoordinate`].
///
/// Placement coordinates are continuous (RTT is continuous); the clustering
/// engine keys on integer cells. Rounding is lossless enough for locality — the
/// proximity ordering survives — and `MatrixCoordinate::new` cannot fail for
/// these magnitudes (bounds are `±i64::MAX/4`), so the origin fallback is
/// defensive only.
fn position_to_coordinate(p: MatrixPosition) -> MatrixCoordinate {
    MatrixCoordinate::new(p.x.round() as i64, p.y.round() as i64, p.z.round() as i64)
        .unwrap_or_else(|_| MatrixCoordinate::origin())
}

/// Inverse of [`position_to_coordinate`].
fn coordinate_to_position(c: &MatrixCoordinate) -> MatrixPosition {
    MatrixPosition {
        x: c.x as f64,
        y: c.y as f64,
        z: c.z as f64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ngauge::PeerProximity;

    fn origin_dist(p: &MatrixPosition) -> f64 {
        (p.x * p.x + p.y * p.y + p.z * p.z).sqrt()
    }

    #[test]
    fn cold_start_yields_no_centroids() {
        let provider = LocalityProvider::new();
        assert!(locality_centroids(&provider, 3).is_empty());
    }

    #[test]
    fn geographic_clustering_reflects_proximity() {
        // Two real proximity bands: a nearby LAN cluster (~0.5 ms) and a distant
        // WAN cluster (~80 ms). GeographicClustering must recover them as two
        // centroids at clearly different radii — proof that, fed a real metric,
        // its output reflects proximity (not the random hash cell).
        let mut samples = Vec::new();
        for i in 0..6 {
            samples.push(PeerProximity::new(format!("lan-{i}"), 500 + i as u64 * 30));
            samples.push(PeerProximity::new(format!("wan-{i}"), 80_000 + i as u64 * 400));
        }
        let provider = LocalityProvider::from_samples(samples);

        let centroids = locality_centroids(&provider, 2);
        assert_eq!(centroids.len(), 2, "expected two locality centroids");

        let mut radii: Vec<f64> = centroids.iter().map(origin_dist).collect();
        radii.sort_by(|a, b| a.partial_cmp(b).expect("test: finite radii"));

        // Near centroid ~ 5 units (0.5ms/0.1ms), far centroid ~ 800 units.
        assert!(
            radii[0] < 50.0,
            "near locality centroid must sit close to origin, got {radii:?}"
        );
        assert!(
            radii[1] > 500.0,
            "far locality centroid must sit far from origin, got {radii:?}"
        );
    }
}
