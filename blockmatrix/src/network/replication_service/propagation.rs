// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! H4/H5: propagation-weight + replication-signal loop (15s).
//!
//! Every 15 seconds, compute routing-intelligence weight adjustments and push
//! them into the block propagator (H4), then check replication triggers in the
//! single implicit network and log any urgent replication signals (H5). Reuses
//! the SAME eBPF orchestrator shared with the `PeerContext` (P5/E.1) so kernel-
//! map state stays consistent. Byte-identical to the loop previously inline in
//! `start_network`.

use tracing::{debug, info};

use super::ReplicationService;

/// Spawn the H4/H5 propagation-weight + replication-signal loop.
/// Fire-and-forget `tokio::spawn`.
pub(super) fn spawn(svc: &ReplicationService) {
    // --- Phase E.1: eBPF feedback adapter for the routing intelligence
    // feed. Reuses the SAME orchestrator shared with the PeerContext
    // (P5) so kernel-map state is consistent between the peer-auth
    // mirror and the congestion-derived routing feed.
    let ebpf_for_feedback = svc.ebpf.clone();
    if ebpf_for_feedback.is_some() {
        info!("eBPF feedback adapter ready for ngauge routing intelligence");
    }

    // --- H4: Spawn propagation weight feed loop ---
    let h4_analytics = svc.ngauge_analytics.clone();
    let h4_propagator = svc.block_propagator.clone();
    let h4_network = svc.network.clone();
    let h4_coord = svc.coord;
    let h4_ebpf = ebpf_for_feedback.clone();
    tokio::spawn(async move {
        // Construct a RoutingIntelFeed once and reuse it across iterations
        // so the eBPF feedback adapter remains attached.
        let mut feed = ngauge::routing_intel::RoutingIntelFeed::new(30);
        if let Some(ebpf) = h4_ebpf.clone() {
            let adapter: Box<dyn ngauge::routing_intel::EbpfPolicyFeedback> =
                Box::new(crate::intelligence::EbpfFeedbackAdapter::new(ebpf));
            feed.set_ebpf_feedback(adapter);
        }

        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(15));
        loop {
            interval.tick().await;
            // Build a node_id -> coordinate map from connected peers.
            let nodes = h4_network.get_connected_nodes().await;
            if nodes.is_empty() {
                continue;
            }
            let mut node_coords: std::collections::HashMap<
                hypermesh_lib::NodeId,
                crate::matrix::coordinate::MatrixCoordinate,
            > = std::collections::HashMap::new();
            let mut candidate_ids: Vec<hypermesh_lib::NodeId> = Vec::new();
            for node in &nodes {
                let nid = hypermesh_lib::NodeId::from_public_key(node.node_id.as_bytes());
                node_coords.insert(nid, node.coordinate);
                candidate_ids.push(nid);
            }
            // Use RoutingIntelligence to compute weight adjustments.
            // In alpha, we create a fresh instance per cycle (no subscriber data
            // accumulated yet, so weights will be neutral=1.0). When the
            // MetricsIngestionPipeline starts feeding RoutingIntelligence
            // in a later sprint, these weights become meaningful.
            let ri = ngauge::RoutingIntelligence::new(30);
            let source_pos = hypermesh_lib::MatrixPosition {
                x: h4_coord.x as f64,
                y: h4_coord.y as f64,
                z: h4_coord.z as f64,
            };
            // Drive the feed's eBPF feedback hook by publishing an update.
            // When the adapter is attached, this propagates congestion-
            // derived privacy actions to HyperMeshEbpf.
            let _ = feed.publish_update(&source_pos, &source_pos, &candidate_ids);

            let modifiers = ngauge::RoutingAdvisor::compute_weight_adjustments(
                &ri, &source_pos, &source_pos, &candidate_ids,
            );
            if !modifiers.is_empty() {
                let weights = crate::intelligence::ngauge_bridge::compute_propagation_weights(
                    &modifiers, &node_coords,
                );
                if !weights.is_empty() {
                    h4_propagator.lock().await.set_propagation_weights(weights).await;
                    debug!("Updated propagation weights from ngauge ({} modifiers)", modifiers.len());
                }
            }

            // --- H5: Check replication triggers ---
            match h4_analytics.lock() {
                Ok(analytics) => {
                    let trigger = ngauge::ReplicationTrigger::new(
                        ngauge::ReplicationConfig::default(),
                    );
                    // check the single implicit network.
                    let signals =
                        trigger.check_in_network(&analytics, hypermesh_lib::DEFAULT_NETWORK);
                    for signal in &signals {
                        if signal.urgency > 0.5 {
                            info!(
                                "Replication signal: shard {} needs {} more replicas (urgency: {:.2}, rate: {})",
                                hex::encode(&signal.shard_id.0[..4]),
                                signal.suggested_count,
                                signal.urgency,
                                signal.current_request_rate,
                            );
                        }
                    }
                }
                Err(e) => {
                    debug!("Failed to lock analytics for replication check: {e}");
                }
            }
        }
    });
    info!("ngauge propagation weight + replication loop started (interval=15s)");
}
