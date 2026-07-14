// Written by Richard Christopher, Copyright 2026 HyperMesh Foundation
// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Two-layer shard-location resolution (Phase A2).
//!
//! "Clients ARE mirrors ARE hosts ARE trackers." A shard has TWO complementary
//! location layers that this module reconciles into ONE ordered provider list:
//!
//! 1. **Live mirrors** (freshest, cheapest): the current swarm. Provider
//!    `node_id`s learned from `TAG_SHARD_ANNOUNCE` and local
//!    consumer-becomes-provider fetches, held in [`ShardLocationIndex`]. These
//!    come first — they are the nodes we KNOW are serving the shard right now.
//!
//! 2. **Canonical matrix placement** (authoritative fallback): where the
//!    `InstructionGenerator` / `ContentAddressedStorage` placed the shard in the
//!    matrix. Each placement is a [`MatrixCoordinate`]; the owning node's id is
//!    derived deterministically from that coordinate. This is the authority when
//!    no live mirror is known (or all live mirrors are unreachable).
//!
//! The two layers use DIFFERENT id spaces — live mirrors are real peer
//! identities (`BLAKE3(FALCON pubkey)` hex), canonical placements are
//! coordinate-derived pseudo-ids ([`coordinate_to_node_id`]). This module
//! carries BOTH: a [`ResolvedProvider`] records its `node_id` (uniform hex) plus
//! the [`ProviderSource`] it came from (including the originating coordinate for
//! canonical placements, so a caller can dial the matrix cell directly).
//!
//! The output is a merged, de-duplicated, ordered list: live mirrors first
//! (freshest-first, as [`ShardLocationIndex::get_providers`] already orders),
//! then canonical-placement holders, then any upstream-tracker answers appended
//! by [`merge_upstream`]. Neither layer replaces the other — they are
//! complementary, and this is the ONE resolution authority both fetch paths use.

use hypermesh_lib::{ContentHash, NodeId};

use crate::matrix::MatrixCoordinate;
use crate::network::swarm_provider::ShardLocationIndex;

/// Where a resolved provider came from. Ordering of the merged list follows
/// the declaration order here: `LiveMirror` < `CanonicalPlacement` <
/// `UpstreamTracker`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderSource {
    /// A peer currently announcing this shard in the live swarm
    /// (`ShardLocationIndex`). Freshest and cheapest — try these first.
    LiveMirror,
    /// The node owning the matrix cell where the shard is canonically
    /// placed. Authoritative fallback when no live mirror is reachable.
    CanonicalPlacement {
        /// The matrix coordinate whose owning node holds this placement.
        coordinate: MatrixCoordinate,
    },
    /// A provider learned from a bounded upstream-tracker locate query
    /// (the DNS-style fallback when local + connected-peer layers miss).
    UpstreamTracker,
}

/// A single resolved provider for a shard: its uniform hex `node_id` and the
/// layer it was resolved from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedProvider {
    /// Uniform hex node id. For [`ProviderSource::LiveMirror`] and
    /// [`ProviderSource::UpstreamTracker`] this is a real peer identity; for
    /// [`ProviderSource::CanonicalPlacement`] it is [`coordinate_to_node_id`]
    /// of the placement coordinate.
    pub node_id: String,
    /// The layer this provider was resolved from.
    pub source: ProviderSource,
}

/// Derive the owning `NodeId` for a matrix coordinate (deterministic).
///
/// This is the canonical coordinate→node reconciliation: `BLAKE3(x||y||z)`.
/// It matches `retrieval::client_assembly::fetching::node_id_from_coordinate`
/// exactly so the resolver and the transport fetch path agree on which node
/// owns a matrix cell.
pub fn coordinate_to_node_id(coord: &MatrixCoordinate) -> NodeId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&coord.x.to_le_bytes());
    hasher.update(&coord.y.to_le_bytes());
    hasher.update(&coord.z.to_le_bytes());
    NodeId::from_bytes(*hasher.finalize().as_bytes())
}

/// Hex form of [`coordinate_to_node_id`], for uniformity with the live-mirror
/// index (which keys providers by hex node id).
pub fn coordinate_to_node_id_hex(coord: &MatrixCoordinate) -> String {
    coordinate_to_node_id(coord).to_hex()
}

/// Resolve a shard's providers across BOTH location layers, in order.
///
/// 1. **Live mirrors** from `live_index.get_providers(content_hash)` (already
///    freshest-first, TTL-filtered). Empty when the index is unwired (Private
///    mode / tests) or on a miss.
/// 2. **Canonical placements**: for each coordinate in `canonical_coords`, the
///    owning node id via [`coordinate_to_node_id_hex`], tagged with the
///    originating coordinate so the caller can dial the matrix cell.
///
/// The result is de-duplicated by `node_id` with FIRST occurrence winning, so a
/// node that is both a live mirror AND a canonical holder keeps its (earlier,
/// freshest) live-mirror position and source. Callers take the head of the list
/// to prefer the live swarm, falling through to canonical placement.
///
/// `canonical_coords` is supplied by the caller from whatever it has: a
/// `RetrievalPlan`'s per-shard [`crate::retrieval::ShardLocation`]s on the
/// client-assembly path, or empty on the bare-content-hash IPC path (where only
/// live mirrors + the upstream fallback are available). This keeps the resolver
/// reusable by both fetch paths — the ONE resolution authority.
pub async fn resolve_shard_locations(
    content_hash: &ContentHash,
    live_index: Option<&ShardLocationIndex>,
    canonical_coords: &[MatrixCoordinate],
) -> Vec<ResolvedProvider> {
    let mut out: Vec<ResolvedProvider> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Layer 1: live mirrors (freshest-first).
    if let Some(index) = live_index {
        for node_id in index.get_providers(content_hash).await {
            if seen.insert(node_id.clone()) {
                out.push(ResolvedProvider {
                    node_id,
                    source: ProviderSource::LiveMirror,
                });
            }
        }
    }

    // Layer 2: canonical matrix placement.
    for coord in canonical_coords {
        let node_id = coordinate_to_node_id_hex(coord);
        if seen.insert(node_id.clone()) {
            out.push(ResolvedProvider {
                node_id,
                source: ProviderSource::CanonicalPlacement {
                    coordinate: *coord,
                },
            });
        }
    }

    out
}

/// Append upstream-tracker provider ids to an existing resolve (Part 2).
///
/// De-duplicates against providers already present so a node returned by the
/// upstream tracker that we already knew from a live mirror or canonical
/// placement is NOT re-added. New ids are appended as
/// [`ProviderSource::UpstreamTracker`] — LAST, since they are the least-fresh,
/// most-expensive layer.
pub fn merge_upstream(resolved: &mut Vec<ResolvedProvider>, upstream_node_ids: &[String]) {
    let mut seen: std::collections::HashSet<String> =
        resolved.iter().map(|r| r.node_id.clone()).collect();
    for id in upstream_node_ids {
        if seen.insert(id.clone()) {
            resolved.push(ResolvedProvider {
                node_id: id.clone(),
                source: ProviderSource::UpstreamTracker,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(seed: u8) -> ContentHash {
        ContentHash([seed; 32])
    }

    fn coord(x: i64, y: i64, z: i64) -> MatrixCoordinate {
        MatrixCoordinate::new(x, y, z).expect("test: valid coordinate")
    }

    #[test]
    fn test_coordinate_to_node_id_deterministic() {
        let c = coord(3, 7, 11);
        assert_eq!(coordinate_to_node_id(&c), coordinate_to_node_id(&c));
        assert_ne!(
            coordinate_to_node_id(&c),
            coordinate_to_node_id(&coord(3, 7, 12)),
        );
    }

    #[test]
    fn test_coordinate_hex_matches_fetching_derivation() {
        // The resolver's coordinate→node derivation MUST match the transport
        // fetch path's `node_id_from_coordinate`, or the two layers would dial
        // different nodes for the same matrix cell.
        let c = coord(5, 0, 2);
        let via_resolver = coordinate_to_node_id(&c);
        let via_fetching =
            crate::retrieval::client_assembly::fetching::node_id_from_coordinate(&c);
        assert_eq!(via_resolver, via_fetching);
    }

    #[tokio::test]
    async fn test_resolve_live_mirrors_first_then_canonical() {
        let index = ShardLocationIndex::new();
        let hash = ch(0xAA);

        // Two live mirrors.
        index.register_provider("live-node-1", &[hash]).await;
        index.register_provider("live-node-2", &[hash]).await;

        // Two canonical placements.
        let placements = [coord(1, 0, 0), coord(2, 0, 0)];

        let resolved = resolve_shard_locations(&hash, Some(&index), &placements).await;

        // First two are live mirrors, last two are canonical placements.
        assert_eq!(resolved.len(), 4);
        assert!(matches!(resolved[0].source, ProviderSource::LiveMirror));
        assert!(matches!(resolved[1].source, ProviderSource::LiveMirror));
        assert!(matches!(
            resolved[2].source,
            ProviderSource::CanonicalPlacement { .. }
        ));
        assert!(matches!(
            resolved[3].source,
            ProviderSource::CanonicalPlacement { .. }
        ));

        // Canonical node_ids match the coordinate derivation.
        assert_eq!(resolved[2].node_id, coordinate_to_node_id_hex(&placements[0]));
        assert_eq!(resolved[3].node_id, coordinate_to_node_id_hex(&placements[1]));
    }

    #[tokio::test]
    async fn test_resolve_dedups_node_appearing_in_both_layers() {
        let index = ShardLocationIndex::new();
        let hash = ch(0xBB);

        // A canonical placement coordinate whose derived node id we ALSO
        // register as a live mirror — the node is both a live mirror and the
        // canonical holder. It must appear ONCE, in the live-mirror position.
        let dual_coord = coord(4, 4, 4);
        let dual_id = coordinate_to_node_id_hex(&dual_coord);
        index.register_provider(&dual_id, &[hash]).await;

        let resolved = resolve_shard_locations(&hash, Some(&index), &[dual_coord]).await;

        assert_eq!(resolved.len(), 1, "dual-role node must appear exactly once");
        assert_eq!(resolved[0].node_id, dual_id);
        assert!(
            matches!(resolved[0].source, ProviderSource::LiveMirror),
            "dual-role node keeps its freshest (live-mirror) position",
        );
    }

    #[tokio::test]
    async fn test_resolve_canonical_only_when_no_live_index() {
        let hash = ch(0xCC);
        let placements = [coord(9, 9, 9)];

        // No live index (Private mode / tests) — canonical placement still
        // resolves as the authoritative fallback.
        let resolved = resolve_shard_locations(&hash, None, &placements).await;
        assert_eq!(resolved.len(), 1);
        assert!(matches!(
            resolved[0].source,
            ProviderSource::CanonicalPlacement { .. }
        ));
    }

    #[tokio::test]
    async fn test_resolve_empty_when_both_layers_miss() {
        let index = ShardLocationIndex::new();
        let resolved = resolve_shard_locations(&ch(0xDD), Some(&index), &[]).await;
        assert!(resolved.is_empty());
    }

    #[test]
    fn test_merge_upstream_appends_new_and_dedups_known() {
        let mut resolved = vec![
            ResolvedProvider {
                node_id: "known-live".to_string(),
                source: ProviderSource::LiveMirror,
            },
        ];

        // "known-live" is already present → skipped; "fresh-upstream" appended.
        merge_upstream(
            &mut resolved,
            &["known-live".to_string(), "fresh-upstream".to_string()],
        );

        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[1].node_id, "fresh-upstream");
        assert!(matches!(resolved[1].source, ProviderSource::UpstreamTracker));
    }
}
