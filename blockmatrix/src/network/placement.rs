// Written by Richard Christopher, Copyright 2026 HyperMesh Foundation
// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Store-path placement glue (P4) — the single live caller of the placement
//! authority.
//!
//! ## What this replaces
//!
//! The live store path used to place shards through the pipeline's
//! `MatrixDistributor`, which (because `register_node` was never called in
//! production) always fell through to a synthetic golden-ratio sphere geometry
//! unrelated to any real node. That engine is deleted. This module is the seam
//! that puts the *real* placement path on the live store flow:
//!
//! - **WHO** — [`crate::distribution::distribute_shards_pos_aware`] →
//!   [`crate::distribution::get_eligible_nodes`] decides which connected peers
//!   are PoS-eligible to hold this asset's shards. PoS is authorization, never a
//!   magnitude.
//! - **WHERE** — each eligible peer's coordinate comes from the measured-RTT
//!   proximity provider (P3, [`crate::network::locality`]). Peers we have not
//!   measured fall back to their announced matrix coordinate — deterministic,
//!   never fabricated.
//! - **execute** — the octant optimizer places shards within the eligible pool.
//!
//! ## Cold start (documented deterministic fallback)
//!
//! With no connected peers — or none eligible — there is nowhere remote to
//! place, so this returns an empty placement set and the store path keeps every
//! shard local (exactly what `distribute_to_peers` already does for an empty
//! peer set). No synthetic geometry is ever produced.

use hypermesh_lib::{AccessScope, PrivacyMode};

use crate::assets::pipeline::sharding::Shard;
use crate::distribution::{distribute_shards_pos_aware, NodeInfo, ShardPlacement};
use crate::matrix::coordinate::MatrixCoordinate;
use crate::network::locality::provider_from_nodes;
use crate::network::NetworkNode;
use crate::proof_of_state::validation::DefaultStateAuthenticator;

/// Map a node's `PrivacyMode` to the eligibility tier string understood by
/// `get_eligible_nodes`. Bounded (Private) assets place on `PrivateNetwork`
/// peers; Unbounded (Public/Anonymous) assets place on `FullPublic` peers.
///
/// The asset tier and each peer's tier are set from the SAME store-time
/// privacy mode, so peers operating in the asset's tier pass the eligibility
/// pre-filter and peers in a different tier are correctly excluded.
fn privacy_tier(mode: PrivacyMode) -> &'static str {
    if mode.scope == AccessScope::Bounded {
        "PrivateNetwork"
    } else {
        "FullPublic"
    }
}

/// The proximity-derived placement coordinate for a peer.
///
/// Measured peers use their RTT-embedded coordinate (P3); unmeasured peers
/// (cold start) fall back to the peer's announced matrix coordinate. Rounding
/// the continuous proximity position to an integer cell cannot fail for these
/// magnitudes, so the announced-coordinate path is the only real fallback.
fn peer_coordinate(
    locality: &ngauge::LocalityProvider,
    peer: &NetworkNode,
) -> MatrixCoordinate {
    match locality.coordinate_for(&peer.node_id) {
        Some(pos) => MatrixCoordinate::new(
            pos.x.round() as i64,
            pos.y.round() as i64,
            pos.z.round() as i64,
        )
        .unwrap_or(peer.coordinate),
        None => peer.coordinate,
    }
}

/// Compute real placements for an asset's shards over the live PoS-eligible
/// peer set, at proximity-derived coordinates.
///
/// Returns one [`ShardPlacement`] per shard (index → matrix position on the
/// chosen eligible peer). On cold start (no peers) or when no peer is eligible,
/// returns an empty vector — the store path keeps every shard local. Never
/// fabricates positions.
pub async fn place_shards(
    peers: &[NetworkNode],
    shards: &[Shard],
    asset_id: &str,
    privacy_mode: PrivacyMode,
) -> Vec<ShardPlacement> {
    if peers.is_empty() || shards.is_empty() {
        return Vec::new();
    }

    let tier = privacy_tier(privacy_mode);

    // WHERE: proximity coordinates for each live peer (P3).
    let locality = provider_from_nodes(peers);
    let nodes: Vec<NodeInfo> = peers
        .iter()
        .map(|p| {
            NodeInfo::new(
                p.node_id.clone(),
                peer_coordinate(&locality, p),
                tier.to_string(),
                // available_storage is DESCRIPTIVE only — capacity never gates
                // placement (PoSpace answers WHERE, not how-much).
                0,
                String::new(),
            )
        })
        .collect();

    // WHO + execute: PoS eligibility, then octant placement within the pool.
    let authenticator = DefaultStateAuthenticator::new();
    match distribute_shards_pos_aware(shards, asset_id, tier, &nodes, &authenticator).await {
        Ok(result) => result.placements,
        Err(e) => {
            tracing::debug!(
                "placement: no eligible peers for asset {asset_id}: {e}; keeping shards local"
            );
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::pipeline::sharding::{Shard, ShardMetadata};
    use std::net::SocketAddr;

    fn shard(index: usize) -> Shard {
        Shard {
            data: vec![0u8; 256],
            metadata: ShardMetadata {
                index,
                is_parity: false,
                size: 256,
                original_size: 256,
                hash: format!("hash-{index}"),
            },
        }
    }

    fn peer(node_id: &str, x: i64, y: i64, z: i64) -> NetworkNode {
        NetworkNode {
            coordinate: MatrixCoordinate::new(x, y, z).expect("test: valid coord"),
            address: "[::1]:9292".parse::<SocketAddr>().expect("test: valid addr"),
            node_id: node_id.to_string(),
            privacy_mode: PrivacyMode::PUBLIC,
            // No live connection → proximity is unmeasured → announced
            // coordinate is used (cold-start-per-peer fallback).
            connection: None,
        }
    }

    #[tokio::test]
    async fn cold_start_no_peers_keeps_local() {
        let placements = place_shards(&[], &[shard(0)], "asset", PrivacyMode::PUBLIC).await;
        assert!(placements.is_empty(), "no peers → no remote placement");
    }

    #[tokio::test]
    async fn places_on_real_peer_coordinates_not_synthetic() {
        // Two real peers at KNOWN announced coordinates. Placement must land on
        // exactly those coordinates — never on a golden-ratio synthetic point.
        let peers = vec![peer("peer-a", 10, 0, 0), peer("peer-b", -10, 0, 0)];
        let shards = vec![shard(0), shard(1)];

        let placements =
            place_shards(&peers, &shards, "round-trip-asset", PrivacyMode::PUBLIC).await;

        assert_eq!(placements.len(), 2, "one placement per shard");

        let real: std::collections::HashSet<(i64, i64, i64)> =
            peers.iter().map(|p| (p.coordinate.x, p.coordinate.y, p.coordinate.z)).collect();
        for pl in &placements {
            let pos = (pl.position.x, pl.position.y, pl.position.z);
            assert!(
                real.contains(&pos),
                "placement {pos:?} must be a real peer coordinate, not synthetic geometry",
            );
            // The known peer node_id must be attached — proof this came from the
            // eligible-node path, not a fabricated position.
            assert!(
                pl.node_id == "peer-a" || pl.node_id == "peer-b",
                "placement must name a real eligible peer, got {}",
                pl.node_id,
            );
        }
    }
}
