// Written by Richard Christopher, Copyright 2026 HyperMesh Foundation
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Locality — turning a *real proximity metric* into placement coordinates.
//!
//! ## The flaw this closes (VISION.md §5.5)
//!
//! A node's matrix cell is `derive_cell(node_id) = BLAKE3(node_id)` — a
//! uniform-random point with **no locality**. Feeding that cell to the placement
//! math (neighbour/distance ranking, [`crate::DispersionAdvisor`] k-means) means
//! "distance" is physically meaningless: two nodes on the same LAN are as likely
//! to be hash-adjacent as two nodes on opposite continents. Clustering over it is
//! the random-hash flaw in a nicer coat.
//!
//! [`LocalityProvider`] is the seam that fixes it. It **consumes a proximity
//! metric as input** — measured peer round-trip time (QUIC RTT, exposed by
//! `stoq::Connection::rtt`) — and produces a *placement* coordinate per peer
//! where matrix distance tracks that metric: low mutual RTT → nearby
//! coordinates, high RTT → distant ones. Identity/address derivation is
//! untouched (`base::derive_cell` still answers *what an asset IS*); this only
//! answers *where it should live* — the durable/elastic split P1 established.
//!
//! ## What is real here, and what is a deliberate tie-break
//!
//! - **RTT is the proximity axis (the real signal).** A peer's position along
//!   the dominant `x` axis is its measured RTT. Euclidean distance between two
//!   embedded peers is therefore `≈ |rtt_a − rtt_b|` — the actual proximity a
//!   single node can measure: peers with similar RTT-to-us occupy a similar
//!   network locality. Clustering this recovers real RTT bands, and a band's
//!   centroid sits at its mean RTT. That is the load-bearing property the
//!   placement layer relies on and the proximity tests assert.
//! - **`y`/`z` are a bounded deterministic jitter, NOT a bearing.** A single
//!   node measures RTT *to itself* only; it cannot know the true pairwise
//!   geometry between two remote peers, so we do not invent a compass direction
//!   at RTT-scale (that would corrupt the metric — points scattered on a large
//!   sphere average back toward the origin). Instead two peers with identical
//!   RTT-to-us get a small, bounded `BLAKE3(peer_id)` offset so they do not
//!   collapse onto one exact point, while the RTT axis stays dominant. A later
//!   phase that gossips full pairwise RTT vectors can replace the jitter with a
//!   real multidimensional embedding; the seam already takes the metric as
//!   input, so nothing here has to be faked to look complete.
//!
//! ## Cold start
//!
//! With no proximity samples yet (single node, or a peer we have not measured),
//! [`LocalityProvider::coordinate_for`] returns `None` and the caller falls back
//! to the deterministic `derive_cell` coordinate. Placement then degrades to the
//! pre-P3 behaviour — never panics, never misplaces — until a real measurement
//! arrives.

use std::collections::HashMap;

use hypermesh_lib::MatrixPosition;

/// A single measured proximity sample: the round-trip time from the local node
/// to `peer_id`. `rtt_micros` is the real QUIC-measured RTT in microseconds.
#[derive(Debug, Clone, PartialEq)]
pub struct PeerProximity {
    /// Peer node id (the string form used across the network layer).
    pub peer_id: String,
    /// Measured round-trip time to that peer, in microseconds.
    pub rtt_micros: u64,
}

impl PeerProximity {
    /// Construct a proximity sample.
    pub fn new(peer_id: impl Into<String>, rtt_micros: u64) -> Self {
        Self {
            peer_id: peer_id.into(),
            rtt_micros,
        }
    }
}

/// Microseconds of RTT per unit of matrix distance. Chosen so that sub-millisecond
/// LAN peers (~hundreds of µs) and tens-of-ms WAN peers separate into clearly
/// distinct radii without overflowing the coordinate space: 100 µs → 1 unit,
/// 50 ms → 500 units. Distance is a *relative* proximity ordering, so the exact
/// constant only sets the scale, never the conclusion.
const MICROS_PER_UNIT: f64 = 100.0;

/// Bounded magnitude (in matrix units) of the `y`/`z` dispersion jitter. Small
/// relative to any meaningful RTT separation, so the RTT axis dominates distance
/// while equal-RTT peers still land on distinct points.
const JITTER_UNITS: f64 = 1.0;

/// Turns measured peer RTT into proximity-real placement coordinates.
///
/// One instance is built per placement cycle from the live connected-peer set
/// (each peer's `stoq::Connection::rtt`). It holds the latest RTT sample per
/// peer and embeds each into a [`MatrixPosition`] on demand. It is intentionally
/// tiny and stateless beyond the sample map — the *signal* is owned by the
/// transport; this only projects it into the placement coordinate space.
#[derive(Debug, Clone, Default)]
pub struct LocalityProvider {
    /// Latest measured RTT (µs) per peer id.
    samples: HashMap<String, u64>,
}

impl LocalityProvider {
    /// Create an empty provider (cold start — every lookup falls back).
    pub fn new() -> Self {
        Self {
            samples: HashMap::new(),
        }
    }

    /// Build a provider from a batch of measured samples.
    pub fn from_samples<I>(samples: I) -> Self
    where
        I: IntoIterator<Item = PeerProximity>,
    {
        let mut provider = Self::new();
        for s in samples {
            provider.observe(s.peer_id, s.rtt_micros);
        }
        provider
    }

    /// Record (or refresh) the measured RTT to a peer.
    pub fn observe(&mut self, peer_id: impl Into<String>, rtt_micros: u64) {
        self.samples.insert(peer_id.into(), rtt_micros);
    }

    /// True when no proximity has been measured yet (cold start).
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Number of peers with a measured proximity sample.
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// The proximity-derived placement coordinate for a peer, or `None` if we
    /// have no measurement for it yet (caller falls back to `derive_cell`).
    ///
    /// The `x` axis is the measured RTT (the real signal); `y`/`z` carry a small
    /// bounded `BLAKE3(peer_id)` jitter (a dispersion tie-break, not a bearing —
    /// see module docs).
    pub fn coordinate_for(&self, peer_id: &str) -> Option<MatrixPosition> {
        let rtt = *self.samples.get(peer_id)?;
        Some(embed(peer_id, rtt))
    }

    /// All measured peers projected into placement coordinates. Order is
    /// unspecified (map iteration); callers that need determinism should sort.
    pub fn placement_coordinates(&self) -> Vec<MatrixPosition> {
        self.samples
            .iter()
            .map(|(id, rtt)| embed(id, *rtt))
            .collect()
    }
}

/// Embed a peer into placement space: RTT on the dominant `x` axis, with a small
/// bounded `BLAKE3(peer_id)` jitter on `y`/`z`.
fn embed(peer_id: &str, rtt_micros: u64) -> MatrixPosition {
    let x = rtt_micros as f64 / MICROS_PER_UNIT;
    let (jy, jz) = jitter(peer_id);
    MatrixPosition { x, y: jy, z: jz }
}

/// A small, bounded, deterministic `(y, z)` offset in `[-JITTER_UNITS,
/// JITTER_UNITS]`, derived from the peer id (pure `BLAKE3`, reproducible across
/// nodes and boots). Keeps equal-RTT peers from collapsing onto one exact point
/// without displacing them along the RTT axis — never a geographic bearing.
fn jitter(peer_id: &str) -> (f64, f64) {
    let digest = blake3::hash(peer_id.as_bytes());
    let b = digest.as_bytes();
    let axis = |lo: usize| {
        let v = i16::from_be_bytes([b[lo], b[lo + 1]]) as f64;
        (v / i16::MAX as f64) * JITTER_UNITS
    };
    (axis(0), axis(2))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dist(a: &MatrixPosition, b: &MatrixPosition) -> f64 {
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        let dz = a.z - b.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    fn origin_dist(p: &MatrixPosition) -> f64 {
        (p.x * p.x + p.y * p.y + p.z * p.z).sqrt()
    }

    #[test]
    fn cold_start_returns_none_no_panic() {
        let provider = LocalityProvider::new();
        assert!(provider.is_empty());
        assert_eq!(provider.coordinate_for("anyone"), None);
        assert!(provider.placement_coordinates().is_empty());
    }

    #[test]
    fn radius_tracks_rtt_monotonically() {
        // Same peer id, different RTT: higher RTT => strictly larger radius.
        let near = embed("peer", 500); // 0.5 ms
        let far = embed("peer", 50_000); // 50 ms
        assert!(
            origin_dist(&far) > origin_dist(&near) * 10.0,
            "radius must scale with RTT: near={:?} far={:?}",
            near,
            far
        );
    }

    #[test]
    fn embedding_is_deterministic_and_rtt_axis_dominant() {
        assert_eq!(embed("peer-x", 1234), embed("peer-x", 1234));
        // x equals rtt/scale exactly; jitter stays bounded and off-axis.
        let p = embed("peer-x", 10_000);
        assert!((p.x - 100.0).abs() < 1e-9, "x must be the RTT axis");
        assert!(p.y.abs() <= JITTER_UNITS && p.z.abs() <= JITTER_UNITS);
    }

    #[test]
    fn equal_rtt_peers_do_not_collapse() {
        // Same RTT, different ids => same x, distinct points (bounded jitter).
        let a = embed("peer-a", 5_000);
        let b = embed("peer-b", 5_000);
        assert!((a.x - b.x).abs() < 1e-9, "equal RTT => equal proximity axis");
        assert_ne!(a, b, "distinct ids must not collapse onto one point");
    }

    #[test]
    fn low_rtt_peers_cluster_near_high_rtt_peers_are_distant() {
        // A cluster of low-RTT peers (a "nearby" locality) and a cluster of
        // high-RTT peers (a distant one). The proof P3 exists to give: peers
        // that are physically close (low mutual RTT) land at nearby placement
        // coordinates; far peers land far away.
        let mut provider = LocalityProvider::new();
        for i in 0..5 {
            provider.observe(format!("near-{i}"), 400 + i as u64 * 20); // ~0.4 ms
            provider.observe(format!("far-{i}"), 60_000 + i as u64 * 500); // ~60 ms
        }

        // Every low-RTT peer sits well inside every high-RTT peer's radius.
        for i in 0..5 {
            let near = provider
                .coordinate_for(&format!("near-{i}"))
                .expect("test: near sample present");
            let far = provider
                .coordinate_for(&format!("far-{i}"))
                .expect("test: far sample present");
            assert!(
                origin_dist(&near) < 10.0,
                "near peer must be close to origin, got {near:?}"
            );
            assert!(
                origin_dist(&far) > 500.0,
                "far peer must be distant from origin, got {far:?}"
            );
        }

        // Mean pairwise distance within the near cluster is far smaller than the
        // distance between the two clusters — i.e. proximity is preserved.
        let near_coords: Vec<_> = (0..5)
            .map(|i| provider.coordinate_for(&format!("near-{i}")).expect("test: near sample"))
            .collect();
        let far_coords: Vec<_> = (0..5)
            .map(|i| provider.coordinate_for(&format!("far-{i}")).expect("test: far sample"))
            .collect();

        let mut intra_near = 0.0;
        let mut n = 0.0;
        for a in &near_coords {
            for b in &near_coords {
                intra_near += dist(a, b);
                n += 1.0;
            }
        }
        intra_near /= n;

        let mut inter = 0.0;
        let mut m = 0.0;
        for a in &near_coords {
            for b in &far_coords {
                inter += dist(a, b);
                m += 1.0;
            }
        }
        inter /= m;

        assert!(
            inter > intra_near * 5.0,
            "inter-cluster distance ({inter:.1}) must dominate intra-near ({intra_near:.1})"
        );
    }

    #[test]
    fn from_samples_round_trips() {
        let provider = LocalityProvider::from_samples([
            PeerProximity::new("a", 1000),
            PeerProximity::new("b", 2000),
        ]);
        assert_eq!(provider.len(), 2);
        assert!(provider.coordinate_for("a").is_some());
        assert!(provider.coordinate_for("c").is_none());
    }

    /// End-to-end proof that placement follows proximity: demand recorded at
    /// proximity-derived coordinates makes [`crate::DispersionAdvisor`] steer
    /// replicas toward the low-RTT locality rather than a random hash point.
    #[test]
    fn dispersion_advisor_places_by_proximity() {
        use crate::{DispersionAdvisor, SwarmAnalytics};
        use hypermesh_lib::{ContentHash, NodeId, DEFAULT_NETWORK};

        // Consumers with low RTT (a nearby locality) dominate demand for a
        // shard; a couple of far consumers also request it.
        let mut provider = LocalityProvider::new();
        for i in 0..8 {
            provider.observe(format!("near-{i}"), 500 + i as u64 * 25);
        }
        provider.observe("far-0", 90_000);
        provider.observe("far-1", 92_000);

        let shard = ContentHash::from_bytes([0x5a; 32]);
        let mut analytics = SwarmAnalytics::new();
        for i in 0..8 {
            let id = format!("near-{i}");
            let pos = provider.coordinate_for(&id).expect("test: near sample");
            analytics.record_request(shard, NodeId::from_public_key(id.as_bytes()), pos, i);
        }
        for id in ["far-0", "far-1"] {
            let pos = provider.coordinate_for(id).expect("test: far sample");
            analytics.record_request(shard, NodeId::from_public_key(id.as_bytes()), pos, 100);
        }

        let advisor = DispersionAdvisor::new();
        let placements = advisor.recommend_placement_in_network(DEFAULT_NETWORK, &shard, &analytics, 2);
        assert!(!placements.is_empty(), "advisor must recommend a placement");

        // The dominant recommended placement sits in the low-RTT band (x ≈ 5-6
        // units), not out at the far band (x ≈ 900) — placement followed the
        // real proximity signal.
        let nearest_x = placements
            .iter()
            .map(|p| p.x)
            .fold(f64::INFINITY, f64::min);
        assert!(
            nearest_x < 100.0,
            "a recommended placement must land in the nearby locality, got x={nearest_x}"
        );
    }
}
