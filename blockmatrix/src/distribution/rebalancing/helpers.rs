// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Internal helper functions for shard rebalancing decisions.
//!
//! Provides scoring, selection, and placement logic used by `RebalanceManager`.

use super::types::{RebalanceAction, ShardId};
use crate::distribution::matrix_optimizer::Octant;
use crate::matrix::coordinate::MatrixCoordinate;
use std::collections::HashMap;

/// Count shards per node (only counting live nodes).
pub(super) fn shard_counts_per_node(
    shard_map: &HashMap<ShardId, Vec<(String, MatrixCoordinate)>>,
    nodes: &HashMap<String, MatrixCoordinate>,
) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for replicas in shard_map.values() {
        for (nid, _) in replicas {
            if nodes.contains_key(nid) {
                *counts.entry(nid.clone()).or_default() += 1;
            }
        }
    }
    counts
}

/// Compute normalised imbalance score (0.0 - 1.0).
pub(super) fn compute_imbalance_score(
    nodes: &HashMap<String, MatrixCoordinate>,
    node_counts: &HashMap<String, usize>,
) -> f64 {
    if nodes.is_empty() {
        return 0.0;
    }
    let total: usize = node_counts.values().sum();
    let avg = total as f64 / nodes.len() as f64;
    if avg < f64::EPSILON {
        return 0.0;
    }

    let mut variance_sum = 0.0;
    for nid in nodes.keys() {
        let count = *node_counts.get(nid).unwrap_or(&0) as f64;
        let diff = count - avg;
        variance_sum += diff * diff;
    }
    let std_dev = (variance_sum / nodes.len() as f64).sqrt();
    (std_dev / avg).min(1.0)
}

/// Count replicas that are on currently-live nodes.
pub(super) fn live_replica_count(
    replicas: &[(String, MatrixCoordinate)],
    nodes: &HashMap<String, MatrixCoordinate>,
) -> usize {
    replicas
        .iter()
        .filter(|(nid, _)| nodes.contains_key(nid))
        .count()
}

/// Find shards whose live replica count is below `min_replicas`.
pub(super) fn find_orphaned_shards(
    shard_map: &HashMap<ShardId, Vec<(String, MatrixCoordinate)>>,
    nodes: &HashMap<String, MatrixCoordinate>,
    min_replicas: usize,
) -> Vec<ShardId> {
    shard_map
        .iter()
        .filter(|(_, replicas)| live_replica_count(replicas, nodes) < min_replicas)
        .map(|(sid, _)| sid.clone())
        .collect()
}

/// Pick target nodes for new replicas, preferring octant diversity.
pub(super) fn pick_replica_targets(
    current_replicas: &[(String, MatrixCoordinate)],
    nodes: &HashMap<String, MatrixCoordinate>,
    count: usize,
) -> Vec<String> {
    let occupied_octants: Vec<u8> = current_replicas
        .iter()
        .filter(|(nid, _)| nodes.contains_key(nid))
        .map(|(_, pos)| Octant::from_coordinate(pos).value())
        .collect();

    let mut candidates: Vec<(&String, &MatrixCoordinate)> = nodes
        .iter()
        .filter(|(nid, _)| !current_replicas.iter().any(|(r, _)| r == *nid))
        .collect();

    candidates.sort_by(|(_, pos_a), (_, pos_b)| {
        let a_new = !occupied_octants.contains(&Octant::from_coordinate(pos_a).value());
        let b_new = !occupied_octants.contains(&Octant::from_coordinate(pos_b).value());

        b_new.cmp(&a_new).then_with(|| {
            let dist_a = avg_distance_to_replicas(pos_a, current_replicas, nodes);
            let dist_b = avg_distance_to_replicas(pos_b, current_replicas, nodes);
            dist_b
                .partial_cmp(&dist_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    candidates
        .into_iter()
        .take(count)
        .map(|(nid, _)| nid.clone())
        .collect()
}

/// Pick replicas to remove when over max_replicas (least diversity).
pub(super) fn pick_replica_removals(
    replicas: &[(String, MatrixCoordinate)],
    nodes: &HashMap<String, MatrixCoordinate>,
    count: usize,
) -> Vec<String> {
    let live: Vec<&(String, MatrixCoordinate)> = replicas
        .iter()
        .filter(|(nid, _)| nodes.contains_key(nid))
        .collect();

    let mut scored: Vec<(String, f64)> = live
        .iter()
        .map(|(nid, pos)| {
            let min_dist = live
                .iter()
                .filter(|(other_nid, _)| other_nid != nid)
                .map(|(_, other_pos)| pos.euclidean_distance(other_pos))
                .fold(f64::MAX, f64::min);
            (nid.clone(), min_dist)
        })
        .collect();

    scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    scored.into_iter().take(count).map(|(nid, _)| nid).collect()
}

/// Average euclidean distance from `pos` to all live replicas.
fn avg_distance_to_replicas(
    pos: &MatrixCoordinate,
    replicas: &[(String, MatrixCoordinate)],
    nodes: &HashMap<String, MatrixCoordinate>,
) -> f64 {
    let live: Vec<&MatrixCoordinate> = replicas
        .iter()
        .filter(|(nid, _)| nodes.contains_key(nid))
        .map(|(_, p)| p)
        .collect();

    if live.is_empty() {
        return 0.0;
    }
    let total: f64 = live.iter().map(|p| pos.euclidean_distance(p)).sum();
    total / live.len() as f64
}

/// Generate departure actions: replicate under-replicated shards.
pub(super) fn generate_departure_actions(
    shard_map: &mut HashMap<ShardId, Vec<(String, MatrixCoordinate)>>,
    nodes: &HashMap<String, MatrixCoordinate>,
    node_id: &str,
    min_replicas: usize,
) -> Vec<RebalanceAction> {
    let mut actions = Vec::new();

    let affected_ids: Vec<ShardId> = shard_map
        .iter()
        .filter(|(_, replicas)| replicas.iter().any(|(nid, _)| nid == node_id))
        .map(|(sid, _)| sid.clone())
        .collect();

    for replicas in shard_map.values_mut() {
        replicas.retain(|(nid, _)| nid != node_id);
    }

    for shard_id in &affected_ids {
        let replicas = match shard_map.get(shard_id) {
            Some(r) => r.clone(),
            None => continue,
        };
        let live = live_replica_count(&replicas, nodes);
        if live < min_replicas {
            let needed = min_replicas - live;
            let targets = pick_replica_targets(&replicas, nodes, needed);
            for target in targets {
                actions.push(RebalanceAction::ReplicateShard {
                    shard_id: shard_id.clone(),
                    to_node: target,
                });
            }
        }
    }

    actions
}

/// Add replicate actions for under-replicated shards targeting new node.
pub(super) fn under_replicated_actions(
    shard_map: &HashMap<ShardId, Vec<(String, MatrixCoordinate)>>,
    nodes: &HashMap<String, MatrixCoordinate>,
    node_id: &str,
    position: &MatrixCoordinate,
    min_replicas: usize,
    actions: &mut Vec<RebalanceAction>,
) {
    let new_octant = Octant::from_coordinate(position).value();

    for (shard_id, replicas) in shard_map {
        let live = live_replica_count(replicas, nodes);
        if live >= min_replicas {
            continue;
        }

        let has_this_octant = replicas
            .iter()
            .filter(|(nid, _)| nodes.contains_key(nid))
            .any(|(_, pos)| Octant::from_coordinate(pos).value() == new_octant);

        if !has_this_octant || live == 0 {
            actions.push(RebalanceAction::ReplicateShard {
                shard_id: shard_id.clone(),
                to_node: node_id.to_string(),
            });
        }
    }
}

/// Add move actions from overloaded nodes to the new (empty) node.
pub(super) fn overload_relief_actions(
    shard_map: &HashMap<ShardId, Vec<(String, MatrixCoordinate)>>,
    nodes: &HashMap<String, MatrixCoordinate>,
    node_id: &str,
    rebalance_threshold: f64,
    min_replicas: usize,
    actions: &mut Vec<RebalanceAction>,
) {
    let counts = shard_counts_per_node(shard_map, nodes);
    let total: usize = counts.values().sum();
    let avg = total as f64 / nodes.len().max(1) as f64;

    if avg < 2.0 {
        return;
    }

    for (overloaded_nid, &count) in &counts {
        if count as f64 <= avg * (1.0 + rebalance_threshold) {
            continue;
        }
        if let Some(shard_id) =
            find_safe_movable_shard(shard_map, nodes, overloaded_nid, node_id, min_replicas)
        {
            actions.push(RebalanceAction::MoveShard {
                shard_id,
                from_node: overloaded_nid.clone(),
                to_node: node_id.to_string(),
            });
        }
    }
}

/// Find a shard on `from_node` movable to `to_node`.
pub(super) fn find_movable_shard(
    shard_map: &HashMap<ShardId, Vec<(String, MatrixCoordinate)>>,
    from_node: &str,
    to_node: &str,
) -> Option<ShardId> {
    shard_map.iter().find_map(|(shard_id, replicas)| {
        let on_from = replicas.iter().any(|(nid, _)| nid == from_node);
        let on_to = replicas.iter().any(|(nid, _)| nid == to_node);
        if on_from && !on_to {
            Some(shard_id.clone())
        } else {
            None
        }
    })
}

/// Find a shard safely movable without dropping below min_replicas.
fn find_safe_movable_shard(
    shard_map: &HashMap<ShardId, Vec<(String, MatrixCoordinate)>>,
    nodes: &HashMap<String, MatrixCoordinate>,
    from_node: &str,
    to_node: &str,
    min_replicas: usize,
) -> Option<ShardId> {
    shard_map.iter().find_map(|(shard_id, replicas)| {
        let on_from = replicas.iter().any(|(nid, _)| nid == from_node);
        let on_to = replicas.iter().any(|(nid, _)| nid == to_node);
        let live = live_replica_count(replicas, nodes);
        if on_from && !on_to && live > min_replicas {
            Some(shard_id.clone())
        } else {
            None
        }
    })
}
