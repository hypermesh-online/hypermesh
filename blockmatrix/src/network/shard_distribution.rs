// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Placement-aware shard distribution to network peers.
//!
//! After the asset pipeline produces shards and the placement authority
//! ([`crate::network::placement::place_shards`] →
//! [`crate::distribution::distribute_shards_pos_aware`]) computes each shard's
//! target matrix position on a real PoS-eligible peer, this module sends shards
//! to the connected peers closest to those target positions.
//!
//! Graceful degradation: shards whose target position has no nearby peer
//! remain stored locally.

use crate::distribution::ShardPlacement;
use crate::matrix::coordinate::MatrixCoordinate;
use crate::network::shard_transport::ShardTransport;
use crate::network::NetworkNode;
use hypermesh_lib::{ContentHash, NodeId};

/// Result of distributing shards to network peers.
#[derive(Debug)]
pub struct NetworkDistributionResult {
    /// Number of shards successfully sent to remote peers.
    pub sent: usize,
    /// Number of shards kept locally (no suitable peer found or send failed).
    pub kept_local: usize,
    /// Number of send attempts that failed.
    pub failed: usize,
}

/// Distribute shards to connected peers using the placement map.
///
/// For each shard, finds the authenticated peer whose matrix coordinate is
/// closest to the shard's target position and sends the shard via the
/// provided transport. Shards with no reachable peer are kept locally.
///
/// # Arguments
///
/// * `shards` - Pairs of (content_hash, shard_data) from the pipeline
/// * `placements` - Target matrix positions from the placement authority
///   ([`crate::network::placement::place_shards`])
/// * `peers` - Currently connected and authenticated peers
/// * `transport` - STOQ shard transport for sending
pub async fn distribute_to_peers(
    shards: &[(ContentHash, Vec<u8>)],
    placements: &[ShardPlacement],
    peers: &[NetworkNode],
    transport: &dyn ShardTransport,
) -> NetworkDistributionResult {
    if peers.is_empty() {
        tracing::info!(
            "No connected peers; all {} shards kept locally",
            shards.len()
        );
        return NetworkDistributionResult {
            sent: 0,
            kept_local: shards.len(),
            failed: 0,
        };
    }

    let mut sent = 0usize;
    let mut kept_local = 0usize;
    let mut failed = 0usize;

    for (i, (shard_hash, shard_data)) in shards.iter().enumerate() {
        let target_pos = placement_position(i, placements);
        match find_closest_peer(target_pos, peers) {
            Some(peer) => {
                let result = send_shard_to_peer(
                    transport, peer, i, shard_hash, shard_data, target_pos,
                )
                .await;
                if result {
                    sent += 1;
                } else {
                    failed += 1;
                    kept_local += 1;
                }
            }
            None => {
                tracing::debug!(
                    "Shard {i}: no peer near target position, kept locally"
                );
                kept_local += 1;
            }
        }
    }

    tracing::info!(
        "Shard distribution: {sent} sent, {kept_local} kept locally, {failed} failed"
    );

    NetworkDistributionResult {
        sent,
        kept_local,
        failed,
    }
}

/// Send a single shard to a peer. Returns true on success.
async fn send_shard_to_peer(
    transport: &dyn ShardTransport,
    peer: &NetworkNode,
    shard_index: usize,
    shard_hash: &ContentHash,
    shard_data: &[u8],
    target_pos: MatrixCoordinate,
) -> bool {
    let node_id = peer_to_node_id(peer);
    match transport.send_shard(&node_id, shard_hash, shard_data).await {
        Ok(()) => {
            tracing::debug!(
                "Shard {} sent to peer {} (target ({},{},{}))",
                shard_index,
                &peer.node_id[..8.min(peer.node_id.len())],
                target_pos.x,
                target_pos.y,
                target_pos.z,
            );
            true
        }
        Err(e) => {
            tracing::warn!(
                "Shard {} send to {} failed: {e}",
                shard_index,
                &peer.node_id[..8.min(peer.node_id.len())],
            );
            false
        }
    }
}

/// Look up the target position for a shard index, falling back to origin.
fn placement_position(
    shard_index: usize,
    placements: &[ShardPlacement],
) -> MatrixCoordinate {
    placements
        .iter()
        .find(|p| p.shard_index == shard_index)
        .map(|p| p.position)
        .unwrap_or_else(MatrixCoordinate::origin)
}

/// Find the connected peer whose coordinate is closest to the target.
pub(crate) fn find_closest_peer<'a>(
    target: MatrixCoordinate,
    peers: &'a [NetworkNode],
) -> Option<&'a NetworkNode> {
    peers
        .iter()
        .filter(|p| p.connection.is_some())
        .min_by(|a, b| {
            let da = a.coordinate.euclidean_distance(&target);
            let db = b.coordinate.euclidean_distance(&target);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// Derive a `NodeId` from the peer's string node_id (BLAKE3 hash).
fn peer_to_node_id(peer: &NetworkNode) -> NodeId {
    NodeId::from_bytes(*blake3::hash(peer.node_id.as_bytes()).as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::shard_transport::MockShardTransport;

    fn make_placement(
        index: usize,
        x: i64,
        y: i64,
        z: i64,
    ) -> ShardPlacement {
        let position = MatrixCoordinate::new(x, y, z).expect("test: valid coord");
        ShardPlacement {
            shard_index: index,
            position,
            node_id: format!("node-{index}"),
            octant: 0,
            distance_from_origin: position.euclidean_distance(&MatrixCoordinate::origin()),
        }
    }

    #[tokio::test]
    async fn test_distribute_no_peers() {
        let transport = MockShardTransport::new();
        let hash = ContentHash([0xAA; 32]);
        let shards = vec![(hash, vec![1, 2, 3])];

        let result =
            distribute_to_peers(&shards, &[], &[], &transport).await;
        assert_eq!(result.sent, 0);
        assert_eq!(result.kept_local, 1);
    }

    #[test]
    fn test_placement_position_lookup() {
        let placements = vec![
            make_placement(0, 10, 20, 30),
            make_placement(1, 40, 50, 60),
        ];

        let pos0 = placement_position(0, &placements);
        assert_eq!(pos0.x, 10);

        let pos1 = placement_position(1, &placements);
        assert_eq!(pos1.x, 40);

        // Missing index falls back to origin
        let pos_missing = placement_position(99, &placements);
        assert_eq!(pos_missing.x, 0);
    }

    #[test]
    fn test_closest_peer_no_connection_filtered() {
        use crate::bootstrap::PrivacyMode;

        // Peer with connection=None should be filtered out
        let peers = vec![NetworkNode {
            coordinate: MatrixCoordinate::new(1, 1, 1)
                .expect("test: valid coord"),
            address: "[::1]:9292".parse().expect("test: valid addr"),
            node_id: "no-conn".to_string(),
            privacy_mode: PrivacyMode::PUBLIC,
            connection: None,
        }];

        let target =
            MatrixCoordinate::new(1, 1, 1).expect("test: valid coord");
        let closest = find_closest_peer(target, &peers);
        assert!(closest.is_none());
    }

    #[test]
    fn test_peer_to_node_id_deterministic() {
        use crate::bootstrap::PrivacyMode;
        let peer = NetworkNode {
            coordinate: MatrixCoordinate::origin(),
            address: "[::1]:9292".parse().expect("test: valid addr"),
            node_id: "test-node-1".to_string(),
            privacy_mode: PrivacyMode::PUBLIC,
            connection: None,
        };

        let id1 = peer_to_node_id(&peer);
        let id2 = peer_to_node_id(&peer);
        assert_eq!(id1.to_hex(), id2.to_hex());
    }
}
