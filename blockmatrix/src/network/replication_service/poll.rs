// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase E.2: replication-poll loop (30s) + dispersion-aware source selection.
//!
//! Every 30 seconds, ask ngauge which shards need more replicas and proactively
//! fetch additional copies from known providers via `TAG_SHARD_FETCH`, closing
//! the consumer-becomes-provider loop. Flat per-network replication: the loop
//! fetches the nearest provider by dispersion, registers this node as a new
//! provider, and updates the replica count — all keyed by
//! [`DEFAULT_NETWORK`](hypermesh_lib::DEFAULT_NETWORK).

use tracing::{debug, info};

use super::ReplicationService;

/// Spawn the E.2 replication-poll loop.
///
/// `async` + fallible to match the `spawn` seam of the sibling loops; the loop
/// body itself is a fire-and-forget `tokio::spawn`.
pub(super) async fn spawn(svc: &ReplicationService) -> anyhow::Result<()> {
    let rp_analytics = svc.ngauge_analytics.clone();
    let rp_index = svc.shard_location_index.clone();
    let rp_transport = svc.shard_transport.clone();
    let rp_network = svc.network.clone();
    let rp_sync_manager = svc.sync_manager.clone();
    let rp_local_node_id = svc.node_id.clone();

    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(30));
        // Skip the immediate tick so the first fetch happens after
        // the network has had time to come up.
        interval.tick().await;
        loop {
            interval.tick().await;

            // P6: reclaim stale provider hints on the same cadence the
            // DNS cache is swept, so the location index does not keep
            // handing out peers that have gone offline.
            let reclaimed = rp_index.cleanup_expired().await;
            if reclaimed > 0 {
                debug!(
                    "replication-poll: reclaimed {reclaimed} expired provider hint(s)"
                );
            }

            // The networks this node has joined — replication is per-network.
            let networks = super::joined_networks(&rp_sync_manager).await;
            if networks.is_empty() {
                continue;
            }

            // Snapshot connected-peer coordinates once per cycle so the
            // DispersionAdvisor can rank fetch sources by matrix
            // topology instead of taking candidates[0] blindly.
            //
            // P3: rank by PROXIMITY, not by the random identity cell.
            // Each peer's coordinate comes from its measured QUIC RTT
            // (proximity-real); only when a peer has no live measurement
            // do we fall back to its `derive_cell` coordinate, so the
            // ranking degrades deterministically rather than reverting to
            // meaningless hash distance (VISION §5.5).
            let connected = rp_network.get_connected_nodes().await;
            let locality =
                crate::network::locality::provider_from_nodes(&connected);
            let peer_coords: std::collections::HashMap<
                String,
                hypermesh_lib::MatrixPosition,
            > = connected
                .into_iter()
                .map(|n| {
                    let pos = locality.coordinate_for(&n.node_id).unwrap_or(
                        hypermesh_lib::MatrixPosition {
                            x: n.coordinate.x as f64,
                            y: n.coordinate.y as f64,
                            z: n.coordinate.z as f64,
                        },
                    );
                    (n.node_id, pos)
                })
                .collect();

            for network in &networks {
                let signals = match rp_analytics.lock() {
                    Ok(guard) => ngauge::ReplicationTrigger::new(
                        ngauge::ReplicationConfig::default(),
                    )
                    .check_in_network(&guard, *network),
                    Err(e) => {
                        debug!("replication-poll: analytics lock poisoned: {e}");
                        continue;
                    }
                };
                if signals.is_empty() {
                    continue;
                }

                for signal in signals.iter().filter(|s| s.urgency > 0.5) {
                    // Find peers known to provide this shard in this network.
                    let providers = rp_index
                        .get_providers_in_network(*network, &signal.shard_id)
                        .await;
                    // Skip if we are the only known provider (cannot
                    // self-replicate) or no providers at all.
                    let candidates: Vec<String> = providers
                        .iter()
                        .filter(|id| id.as_str() != rp_local_node_id.as_str())
                        .cloned()
                        .collect();
                    if candidates.is_empty() {
                        debug!(
                            "replication-poll: no remote providers for shard {} yet",
                            hex::encode(&signal.shard_id.0[..4])
                        );
                        continue;
                    }

                    // P6 (step 3): dispersion-aware source selection.
                    // Ask the DispersionAdvisor where the swarm WANTS new
                    // replicas (k-means over consumer demand, anti-affinity
                    // to existing provider positions), then pick the
                    // candidate provider nearest a recommended placement.
                    // This spreads fetches toward under-served demand
                    // clusters instead of always hammering candidates[0].
                    let target_node_id = select_dispersion_source(
                        &rp_analytics,
                        *network,
                        &signal.shard_id,
                        &candidates,
                        &peer_coords,
                    );
                    let target_id = hypermesh_lib::NodeId::from_public_key(
                        target_node_id.as_bytes(),
                    );

                    use crate::network::shard_transport::ShardTransport;
                    match rp_transport
                        .fetch_shard(&target_id, &signal.shard_id)
                        .await
                    {
                        Ok(_data) => {
                            // P6 (step 2): CLOSE THE FEEDBACK LOOP. The
                            // local node just became a provider of this
                            // shard — register it in the shared index so
                            // the provider count grows, then report that
                            // count back to SwarmAnalytics via
                            // set_replica_count. Next cycle
                            // ReplicationTrigger::check sees
                            // needed <= replicas and STOPS → convergence.
                            // Without this hook the replica count stayed 0
                            // forever and the loop never converged.
                            rp_index
                                .register_provider_in_network(
                                    *network,
                                    &rp_local_node_id,
                                    &[signal.shard_id],
                                )
                                .await;
                            let replica_count = rp_index
                                .get_providers_in_network(*network, &signal.shard_id)
                                .await
                                .len()
                                as u32;
                            if let Ok(mut guard) = rp_analytics.lock() {
                                guard.set_replica_count_in_network(
                                    *network,
                                    signal.shard_id,
                                    replica_count,
                                );
                            }
                            info!(
                                "replication-poll: fetched extra replica of {} from {} (urgency {:.2}, replicas now {})",
                                hex::encode(&signal.shard_id.0[..4]),
                                &target_node_id[..8.min(target_node_id.len())],
                                signal.urgency,
                                replica_count,
                            );
                        }
                        Err(e) => {
                            debug!(
                                "replication-poll: fetch from {} failed: {}",
                                &target_node_id[..8.min(target_node_id.len())],
                                e,
                            );
                        }
                    }
                }
            }
        }
    });
    info!("Phase E.2 replication-poll loop started (interval=30s)");
    Ok(())
}

/// P6 (step 3): pick which provider to fetch a replica from using the
/// ngauge [`DispersionAdvisor`] instead of always taking `candidates[0]`.
///
/// The advisor runs k-means over the shard's consumer demand map (with
/// anti-affinity to positions already holding replicas) and returns the
/// matrix positions where the swarm most wants NEW replicas. We then select
/// the candidate provider whose coordinate is closest to a recommended
/// placement — pulling the copy toward under-served demand. When we lack
/// coordinates or demand data (advisor returns nothing), we fall back to a
/// stable deterministic pick (lexicographically smallest node id) so behavior
/// is reproducible rather than arbitrary hash ordering.
///
/// [`DispersionAdvisor`]: ngauge::DispersionAdvisor
fn select_dispersion_source(
    analytics: &std::sync::Mutex<ngauge::SwarmAnalytics>,
    network: hypermesh_lib::NetworkId,
    shard_id: &hypermesh_lib::ContentHash,
    candidates: &[String],
    peer_coords: &std::collections::HashMap<String, hypermesh_lib::MatrixPosition>,
) -> String {
    debug_assert!(!candidates.is_empty(), "caller guarantees non-empty candidates");

    // Recommend placements from live analytics (k-means over demand).
    let recommendations = match analytics.lock() {
        Ok(guard) => {
            let advisor = ngauge::DispersionAdvisor::new();
            // recommend within the shard's own network.
            advisor.recommend_placement_in_network(
                network,
                shard_id,
                &guard,
                candidates.len().max(1),
            )
        }
        Err(_) => Vec::new(),
    };

    // If we have both recommended placements and candidate coordinates, pick
    // the candidate nearest to any recommended placement.
    if !recommendations.is_empty() {
        let mut best: Option<(String, f64)> = None;
        for cand in candidates {
            let Some(pos) = peer_coords.get(cand) else { continue };
            let nearest = recommendations
                .iter()
                .map(|r| {
                    let dx = r.x - pos.x;
                    let dy = r.y - pos.y;
                    let dz = r.z - pos.z;
                    (dx * dx + dy * dy + dz * dz).sqrt()
                })
                .fold(f64::INFINITY, f64::min);
            match &best {
                Some((_, d)) if *d <= nearest => {}
                _ => best = Some((cand.clone(), nearest)),
            }
        }
        if let Some((node_id, _)) = best {
            return node_id;
        }
    }

    // Fallback: no demand recommendation. Prefer the geometrically central
    // provider of the shard's demand chunk (VISION §5.5) — the candidate whose
    // proximity cell (P3) sits nearest the chunk centroid — over an arbitrary
    // lexical pick. Candidates without a live coordinate are simply absent from
    // the chunk.
    let candidate_cells: Vec<(String, crate::matrix::MatrixCoordinate)> = candidates
        .iter()
        .filter_map(|id| {
            peer_coords.get(id).map(|p| {
                let cell = crate::matrix::MatrixCoordinate::new(
                    p.x.round() as i64,
                    p.y.round() as i64,
                    p.z.round() as i64,
                )
                .unwrap_or_else(|_| crate::matrix::MatrixCoordinate::origin());
                (id.clone(), cell)
            })
        })
        .collect();
    if let Some(central) = crate::network::chunk::most_central(&candidate_cells) {
        return central.clone();
    }

    // Deterministic last resort (no coordinates at all): smallest node id.
    candidates
        .iter()
        .min()
        .cloned()
        .unwrap_or_else(|| candidates[0].clone())
}
