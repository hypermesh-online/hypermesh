// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Integration test for Phase E.1: eBPF feedback adapter.
//!
//! Confirms the ngauge -> blockmatrix -> eBPF feedback loop. When a
//! [`RoutingIntelFeed`] has an [`EbpfFeedbackAdapter`] attached, calling
//! `publish_update` propagates the congestion-derived privacy action to
//! [`HyperMeshEbpf`]. Routing rule propagation is exercised via the direct
//! adapter API (ngauge does not yet emit `EbpfRoutingRule` from
//! `publish_update`; see Phase E.2).

#![cfg(feature = "intelligence")]

use std::sync::Arc;

use blockmatrix::intelligence::EbpfFeedbackAdapter;
use ngauge::routing_intel::{
    EbpfPolicyFeedback, EbpfPrivacyAction, EbpfRoutingRule, RoutingIntelFeed,
};
use hypermesh_ebpf::{EbpfConfig, HyperMeshEbpf};
use hypermesh_lib::{MatrixPosition, NodeId};

#[test]
fn adapter_attaches_to_routing_intel_feed() {
    let ebpf = Arc::new(
        HyperMeshEbpf::new(EbpfConfig::default()).expect("construct HyperMeshEbpf"),
    );
    let mut feed = RoutingIntelFeed::new(30);
    let adapter: Box<dyn EbpfPolicyFeedback> =
        Box::new(EbpfFeedbackAdapter::new(ebpf.clone()));
    feed.set_ebpf_feedback(adapter);

    // publish_update with no candidates exercises the empty-candidates branch
    // and still drives the privacy-action feedback path (NoChange when there
    // is no congestion data — the orchestrator stays untouched).
    let pos = MatrixPosition { x: 0.0, y: 0.0, z: 0.0 };
    let candidates: Vec<NodeId> = vec![];
    let _update = feed.publish_update(&pos, &pos, &candidates);

    // No rules pushed yet (ngauge does not emit EbpfRoutingRule in
    // publish_update at this phase), so the count is zero — but the call
    // graph executed end-to-end without panicking.
    assert_eq!(ebpf.routing_rule_count(), 0);
}

#[test]
fn apply_routing_rules_mutates_ebpf_state() {
    let ebpf = Arc::new(
        HyperMeshEbpf::new(EbpfConfig::default()).expect("construct HyperMeshEbpf"),
    );
    let adapter = EbpfFeedbackAdapter::new(ebpf.clone());

    let rules = vec![EbpfRoutingRule {
        dest: MatrixPosition { x: 100.0, y: 200.0, z: 300.0 },
        next_hop: MatrixPosition { x: 400.0, y: 500.0, z: 600.0 },
    }];

    assert_eq!(ebpf.routing_rule_count(), 0);
    adapter
        .apply_routing_rules(&rules)
        .expect("apply_routing_rules succeeds");
    assert_eq!(ebpf.routing_rule_count(), 1);

    let stored = ebpf
        .get_routing_rule(&MatrixPosition { x: 100.0, y: 200.0, z: 300.0 })
        .expect("rule should be present after push");
    assert_eq!(stored.x, 400.0);
    assert_eq!(stored.y, 500.0);
    assert_eq!(stored.z, 600.0);
}

#[test]
fn apply_privacy_action_propagates_through_adapter() {
    let ebpf = Arc::new(
        HyperMeshEbpf::new(EbpfConfig::default()).expect("construct HyperMeshEbpf"),
    );
    let adapter = EbpfFeedbackAdapter::new(ebpf.clone());

    // All three variants should round-trip without error.
    adapter
        .apply_privacy_action(EbpfPrivacyAction::NoChange)
        .expect("no-change is a no-op");
    adapter
        .apply_privacy_action(EbpfPrivacyAction::Tighten)
        .expect("tighten propagates");
    adapter
        .apply_privacy_action(EbpfPrivacyAction::Relax)
        .expect("relax propagates");
}
