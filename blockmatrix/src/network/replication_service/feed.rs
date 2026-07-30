// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! H3: NGaugeBridge periodic demand-feed loop (10s).
//!
//! Every 10 seconds, snapshot the live per-shard demand and feed it into
//! `SwarmAnalytics`, recording each request at a proximity-real coordinate
//! (P3) in the single implicit network (`DEFAULT_NETWORK`, P2). Byte-identical to
//! the loop previously inline in `start_network`.

use tracing::debug;

use super::ReplicationService;

/// Spawn the H3 demand-feed loop. Fire-and-forget `tokio::spawn`.
pub(super) fn spawn(svc: &ReplicationService) {
    let bridge_position = hypermesh_lib::MatrixPosition {
        x: svc.coord.x as f64,
        y: svc.coord.y as f64,
        z: svc.coord.z as f64,
    };
    // Spawn periodic feed: every 10 seconds, feed demand data into SwarmAnalytics.
    // We implement the loop here instead of calling run_periodic_feed() because
    // the std::sync::MutexGuard held by that method is not Send-safe across await.
    let feed_tracker = svc.swarm_demand_tracker.clone();
    let feed_analytics = svc.ngauge_analytics.clone();
    let feed_position = bridge_position;
    let feed_network = svc.network.clone();
    tokio::spawn(async move {
        let interval = std::time::Duration::from_secs(10);
        loop {
            tokio::time::sleep(interval).await;
            // Snapshot demand data (async lock).
            let snapshot = feed_tracker.snapshot().await;
            // P3: build a proximity locality provider from the live peer
            // RTT (QUIC-measured), so demand is recorded at a
            // proximity-real coordinate instead of this node's random
            // identity cell. Cold start (no measured peer) falls back to
            // `feed_position` (our derive_cell coordinate) — deterministic,
            // never a fabricated signal (VISION §5.5).
            let locality = crate::network::locality::provider_from_nodes(
                &feed_network.get_connected_nodes().await,
            );
            // Feed into analytics (sync lock, no await while held).
            match feed_analytics.lock() {
                Ok(mut analytics) => {
                    for (shard_id, entry) in &snapshot {
                        for requester_id in &entry.requester_ids {
                            let consumer_id = hypermesh_lib::NodeId::from_public_key(
                                requester_id.as_bytes(),
                            );
                            // P3: proximity-derived placement coordinate for
                            // the requesting peer; fall back to our own
                            // coordinate when unmeasured (cold start).
                            let consumer_pos = locality
                                .coordinate_for(requester_id)
                                .unwrap_or(feed_position);
                            // the single default network (DEFAULT_NETWORK).
                            analytics.record_request_in_network(
                                hypermesh_lib::DEFAULT_NETWORK,
                                *shard_id,
                                consumer_id,
                                consumer_pos,
                                entry.last_request_us,
                            );
                        }
                    }
                    if !snapshot.is_empty() {
                        debug!(
                            "Fed {} shard demand entries into SwarmAnalytics",
                            snapshot.len(),
                        );
                    }
                }
                Err(e) => {
                    debug!("Failed to lock analytics for feed: {e}");
                }
            }
        }
    });
    tracing::info!("ngauge intelligence bridge started (periodic_feed=10s)");
}
