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

    // The DMS I/O side: ngauge's `DmsDriver` DECIDES the plan; this executor
    // EXECUTES it (fetch -> register-provider -> set-replica-count). Built from
    // the SAME Arcs the service already holds.
    let executor = super::executor::StoqDmsExecutor::new(
        rp_transport,
        rp_index.clone(),
        rp_local_node_id.clone(),
        rp_analytics.clone(),
    );

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
                // (1) NGauge decides WHICH shards need copies + urgency. The
                // SwarmAnalytics guard is held only for this call and dropped
                // at the end of the match expression (it is `!Send`).
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

                // (2) blockmatrix I/O: gather the candidate providers for each
                // urgent shard (urgency > 0.5) from the shared location index.
                let mut bundles: Vec<ngauge::ShardCandidates> = Vec::new();
                for signal in signals.iter().filter(|s| s.urgency > 0.5) {
                    let providers = rp_index
                        .get_providers_in_network(*network, &signal.shard_id)
                        .await;
                    // Skip self (cannot self-replicate).
                    let all_ids: Vec<String> = providers
                        .iter()
                        .filter(|id| id.as_str() != rp_local_node_id.as_str())
                        .cloned()
                        .collect();
                    if all_ids.is_empty() {
                        debug!(
                            "replication-poll: no remote providers for shard {} yet",
                            hex::encode(&signal.shard_id.0[..4])
                        );
                        continue;
                    }
                    // Candidates with a live proximity coordinate participate in
                    // dispersion + centrality selection; the rest are carried in
                    // `all_ids` for the deterministic last-resort pick.
                    let positioned: Vec<ngauge::ReplicaCandidate> = all_ids
                        .iter()
                        .filter_map(|id| {
                            peer_coords.get(id).map(|p| {
                                ngauge::ReplicaCandidate::new(id.clone(), *p, 1.0)
                            })
                        })
                        .collect();
                    bundles.push(ngauge::ShardCandidates {
                        shard_id: signal.shard_id,
                        positioned,
                        all_ids,
                    });
                }
                if bundles.is_empty() {
                    continue;
                }

                // (3) NGauge builds the concrete plan (source selection via the
                // now-live `replica_selection`) WHILE holding the guard, then
                // the guard is DROPPED before any await.
                let plan = match rp_analytics.lock() {
                    Ok(guard) => {
                        let plan = ngauge::DmsDriver::plan(
                            &guard, *network, &bundles, 0.5,
                        );
                        drop(guard);
                        plan
                    }
                    Err(e) => {
                        debug!("replication-poll: analytics lock poisoned: {e}");
                        continue;
                    }
                };

                // (4) blockmatrix I/O: execute the plan. Each mirror action is
                // the P6 fetch -> register-provider -> set-replica-count loop;
                // reflect actions are dormant until the Phase-4 head observer.
                use ngauge::{MirrorExecutor, ReflectExecutor};
                for action in &plan.mirror {
                    match executor.fetch_and_register(action).await {
                        Ok(replica_count) => {
                            let src = action.source.to_hex();
                            info!(
                                "replication-poll: fetched extra replica of {} from {} (urgency {:.2}, replicas now {})",
                                hex::encode(&action.shard_id.0[..4]),
                                &src[..8.min(src.len())],
                                action.urgency,
                                replica_count,
                            );
                        }
                        Err(e) => {
                            debug!("replication-poll: mirror fetch failed: {e}");
                        }
                    }
                }
                for action in &plan.reflect {
                    if let Err(e) = executor.announce(action).await {
                        debug!("replication-poll: reflect announce failed: {e}");
                    }
                }
            }
        }
    });
    info!("Phase E.2 replication-poll loop started (interval=30s)");
    Ok(())
}
