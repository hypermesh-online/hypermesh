// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Caesar in-transit and holding amount tracking for engauge analytics.
//!
//! Provides [`CaesarTracker`] which monitors Caesar packet flows:
//! - In-transit packets (sent but not yet delivered)
//! - Holding amounts (awaiting settlement)
//! - Fee earnings per node
//! - Settlement rate tracking

use std::collections::HashMap;

use hypermesh_lib::NodeId;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// EconomicState
// ---------------------------------------------------------------------------

/// Point-in-time economic snapshot for a single node.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeEconomicState {
    /// Number of packets currently in transit through this node.
    pub in_transit_count: u32,
    /// Total value of in-transit packets in gold-grams.
    pub in_transit_value_grams: f64,
    /// Number of packets held awaiting settlement.
    pub holding_count: u32,
    /// Total value of held packets in gold-grams.
    pub holding_value_grams: f64,
    /// Total fees earned by this node in gold-grams.
    pub fees_earned_grams: f64,
    /// Number of settlements completed by this node.
    pub settlements_completed: u64,
}

/// Network-wide economic snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkEconomicSnapshot {
    /// Total in-transit packets across all tracked nodes.
    pub total_in_transit: u32,
    /// Total value in-transit in gold-grams.
    pub total_in_transit_value_grams: f64,
    /// Total holding packets across all tracked nodes.
    pub total_holding: u32,
    /// Total holding value in gold-grams.
    pub total_holding_value_grams: f64,
    /// Total fees earned across all tracked nodes.
    pub total_fees_earned_grams: f64,
    /// Network-wide settlement rate (settlements per epoch).
    pub settlement_rate: f64,
    /// Number of nodes being tracked.
    pub tracked_node_count: usize,
}

// ---------------------------------------------------------------------------
// CaesarTracker
// ---------------------------------------------------------------------------

/// Tracks Caesar economic activity across mesh nodes.
///
/// Accepts updates about packet transit, holding, and settlement events,
/// and produces aggregated economic snapshots for engauge analytics.
pub struct CaesarTracker {
    /// Per-node economic state.
    node_states: HashMap<NodeId, NodeEconomicState>,
    /// Rolling settlement count for rate calculation.
    epoch_settlements: u64,
    /// Current epoch number.
    current_epoch: u64,
    /// Settlement rate from the most recent completed epoch.
    last_settlement_rate: f64,
}

impl CaesarTracker {
    /// Create a new Caesar tracker.
    pub fn new() -> Self {
        Self {
            node_states: HashMap::new(),
            epoch_settlements: 0,
            current_epoch: 0,
            last_settlement_rate: 0.0,
        }
    }

    /// Record a packet entering transit through a node.
    pub fn record_transit(&mut self, node_id: NodeId, value_grams: f64) {
        let state = self.node_states.entry(node_id).or_default();
        state.in_transit_count += 1;
        state.in_transit_value_grams += value_grams;
    }

    /// Record a packet being delivered (leaving transit).
    pub fn record_delivery(&mut self, node_id: NodeId, value_grams: f64) {
        let state = self.node_states.entry(node_id).or_default();
        state.in_transit_count = state.in_transit_count.saturating_sub(1);
        state.in_transit_value_grams = (state.in_transit_value_grams - value_grams).max(0.0);
        // Delivered packets move to holding until settlement.
        state.holding_count += 1;
        state.holding_value_grams += value_grams;
    }

    /// Record a settlement event (packet leaving holding).
    pub fn record_settlement(&mut self, node_id: NodeId, value_grams: f64, fee_grams: f64) {
        let state = self.node_states.entry(node_id).or_default();
        state.holding_count = state.holding_count.saturating_sub(1);
        state.holding_value_grams = (state.holding_value_grams - value_grams).max(0.0);
        state.fees_earned_grams += fee_grams;
        state.settlements_completed += 1;
        self.epoch_settlements += 1;
    }

    /// Advance to the next epoch, computing settlement rate.
    pub fn advance_epoch(&mut self) {
        self.last_settlement_rate = self.epoch_settlements as f64;
        self.epoch_settlements = 0;
        self.current_epoch += 1;
    }

    /// Get the economic state for a specific node.
    pub fn get_node_state(&self, node_id: &NodeId) -> Option<&NodeEconomicState> {
        self.node_states.get(node_id)
    }

    /// Produce a network-wide economic snapshot.
    pub fn network_snapshot(&self) -> NetworkEconomicSnapshot {
        let mut snap = NetworkEconomicSnapshot {
            tracked_node_count: self.node_states.len(),
            settlement_rate: self.last_settlement_rate,
            ..Default::default()
        };

        for state in self.node_states.values() {
            snap.total_in_transit += state.in_transit_count;
            snap.total_in_transit_value_grams += state.in_transit_value_grams;
            snap.total_holding += state.holding_count;
            snap.total_holding_value_grams += state.holding_value_grams;
            snap.total_fees_earned_grams += state.fees_earned_grams;
        }

        snap
    }

    /// Convert to an engauge EconomicSnapshot for streaming.
    pub fn to_economic_snapshot(&self) -> crate::streaming::protocol::EconomicSnapshot {
        let snap = self.network_snapshot();
        crate::streaming::protocol::EconomicSnapshot {
            in_flight_float_grams: snap.total_in_transit_value_grams
                + snap.total_holding_value_grams,
            settlement_rate_per_epoch: snap.settlement_rate,
            active_packets: snap.total_in_transit + snap.total_holding,
            holdings_by_tier_grams: [0.0; 4], // Tier breakdown requires external input.
            fee_rate_per_epoch_grams: snap.total_fees_earned_grams,
            in_transit_count: snap.total_in_transit,
            in_transit_value_grams: snap.total_in_transit_value_grams,
        }
    }

    /// Current epoch number.
    pub fn current_epoch(&self) -> u64 {
        self.current_epoch
    }

    /// Number of tracked nodes.
    pub fn tracked_node_count(&self) -> usize {
        self.node_states.len()
    }
}

impl Default for CaesarTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_node(name: &str) -> NodeId {
        NodeId::from_public_key(name.as_bytes())
    }

    #[test]
    fn track_transit_and_delivery() {
        let mut tracker = CaesarTracker::new();
        let node = test_node("relay-1");

        tracker.record_transit(node, 10.0);
        tracker.record_transit(node, 5.0);

        let state = tracker.get_node_state(&node).expect("test: node state");
        assert_eq!(state.in_transit_count, 2);
        assert!((state.in_transit_value_grams - 15.0).abs() < 1e-9);

        // Deliver one packet.
        tracker.record_delivery(node, 10.0);

        let state = tracker.get_node_state(&node).expect("test: node state");
        assert_eq!(state.in_transit_count, 1);
        assert!((state.in_transit_value_grams - 5.0).abs() < 1e-9);
        assert_eq!(state.holding_count, 1);
        assert!((state.holding_value_grams - 10.0).abs() < 1e-9);
    }

    #[test]
    fn settlement_updates_fees_and_holding() {
        let mut tracker = CaesarTracker::new();
        let node = test_node("settler-1");

        tracker.record_transit(node, 20.0);
        tracker.record_delivery(node, 20.0);
        tracker.record_settlement(node, 20.0, 0.5);

        let state = tracker.get_node_state(&node).expect("test: node state");
        assert_eq!(state.holding_count, 0);
        assert!((state.holding_value_grams).abs() < 1e-9);
        assert!((state.fees_earned_grams - 0.5).abs() < 1e-9);
        assert_eq!(state.settlements_completed, 1);
    }

    #[test]
    fn network_snapshot_aggregates_across_nodes() {
        let mut tracker = CaesarTracker::new();
        let node_a = test_node("node-a");
        let node_b = test_node("node-b");

        tracker.record_transit(node_a, 10.0);
        tracker.record_transit(node_b, 20.0);

        let snap = tracker.network_snapshot();
        assert_eq!(snap.total_in_transit, 2);
        assert!((snap.total_in_transit_value_grams - 30.0).abs() < 1e-9);
        assert_eq!(snap.tracked_node_count, 2);
    }

    #[test]
    fn epoch_advance_computes_settlement_rate() {
        let mut tracker = CaesarTracker::new();
        let node = test_node("node-a");

        // Simulate 5 settlements in epoch 0.
        for _ in 0..5 {
            tracker.record_transit(node, 1.0);
            tracker.record_delivery(node, 1.0);
            tracker.record_settlement(node, 1.0, 0.01);
        }

        tracker.advance_epoch();

        let snap = tracker.network_snapshot();
        assert!((snap.settlement_rate - 5.0).abs() < 1e-9);
    }

    #[test]
    fn to_economic_snapshot_conversion() {
        let mut tracker = CaesarTracker::new();
        let node = test_node("converter");

        // Transit two packets, deliver one, settle one.
        tracker.record_transit(node, 50.0);
        tracker.record_transit(node, 30.0);
        tracker.record_delivery(node, 30.0);
        tracker.record_settlement(node, 30.0, 1.5);

        let econ = tracker.to_economic_snapshot();
        // in_transit: 1 packet (50.0), holding: 0 (settled), fees: 1.5
        assert_eq!(econ.in_transit_count, 1);
        assert!((econ.in_transit_value_grams - 50.0).abs() < 1e-9);
        assert!((econ.fee_rate_per_epoch_grams - 1.5).abs() < 1e-9);
    }
}
