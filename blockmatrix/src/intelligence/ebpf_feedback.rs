// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Phase E.1: Bridge from ngauge routing intelligence to kernel-level eBPF policy.
//!
//! NGauge produces [`EbpfRoutingRule`] and [`EbpfPrivacyAction`] signals based
//! on observed congestion + traffic classification. This adapter consumes them
//! and applies the resulting policy to the live [`HyperMeshEbpf`] BPF maps via
//! the existing `set_routing_rule` and `set_privacy_tier` APIs.
//!
//! `EbpfPolicyFeedback` is a sync trait. Both `set_routing_rule` and
//! `set_privacy_tier` are sync calls on the orchestrator, so the adapter is
//! a thin synchronous translation layer.

use std::sync::Arc;

use ngauge::routing_intel::{EbpfPolicyFeedback, EbpfPrivacyAction, EbpfRoutingRule};
use hypermesh_ebpf::HyperMeshEbpf;
use hypermesh_lib::{NetworkId, PrivacyMode};

/// Adapter that pushes ngauge routing intelligence decisions into the
/// kernel-level eBPF policy maps managed by [`HyperMeshEbpf`].
///
/// The adapter holds a shared [`Arc<HyperMeshEbpf>`] so that multiple feeds
/// (BlockMatrix routing, STOQ path scheduler) can share a single eBPF
/// orchestrator instance.
pub struct EbpfFeedbackAdapter {
    ebpf: Arc<HyperMeshEbpf>,
    /// Network identifier used when no per-network privacy mapping exists.
    /// Phase E.1 wires a single default network; richer routing comes later.
    default_network: NetworkId,
}

impl EbpfFeedbackAdapter {
    /// Construct a new adapter wired to the given eBPF orchestrator.
    ///
    /// Uses [`NetworkId([0u8; 16])`] as the default network identifier for
    /// privacy-action propagation. Callers that need per-network behavior
    /// should use [`with_default_network`].
    pub fn new(ebpf: Arc<HyperMeshEbpf>) -> Self {
        Self {
            ebpf,
            default_network: NetworkId([0u8; 16]),
        }
    }

    /// Construct an adapter with an explicit default network identifier.
    pub fn with_default_network(ebpf: Arc<HyperMeshEbpf>, network: NetworkId) -> Self {
        Self {
            ebpf,
            default_network: network,
        }
    }

    /// Map a congestion-derived [`EbpfPrivacyAction`] to a [`PrivacyMode`].
    ///
    /// - `Tighten` -> `PRIVATE` (bounded, tracked: most restrictive policy)
    /// - `Relax`   -> `PUBLIC`  (unbounded, tracked: most permissive policy)
    /// - `NoChange` returns `None` so the caller skips the eBPF call.
    fn action_to_mode(action: EbpfPrivacyAction) -> Option<PrivacyMode> {
        match action {
            EbpfPrivacyAction::Tighten => Some(PrivacyMode::PRIVATE),
            EbpfPrivacyAction::Relax => Some(PrivacyMode::PUBLIC),
            EbpfPrivacyAction::NoChange => None,
        }
    }
}

impl EbpfPolicyFeedback for EbpfFeedbackAdapter {
    fn apply_routing_rules(&self, rules: &[EbpfRoutingRule]) -> Result<(), String> {
        for rule in rules {
            self.ebpf
                .set_routing_rule(rule.dest, rule.next_hop)
                .map_err(|e| format!("set_routing_rule failed: {e}"))?;
        }
        Ok(())
    }

    fn apply_privacy_action(&self, action: EbpfPrivacyAction) -> Result<(), String> {
        let Some(mode) = Self::action_to_mode(action) else {
            return Ok(());
        };
        self.ebpf
            .set_privacy_tier(self.default_network, mode)
            .map_err(|e| format!("set_privacy_tier failed: {e}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hypermesh_ebpf::EbpfConfig;
    use hypermesh_lib::MatrixPosition;

    fn make_ebpf() -> Arc<HyperMeshEbpf> {
        Arc::new(
            HyperMeshEbpf::new(EbpfConfig::default())
                .expect("construct HyperMeshEbpf in userspace mode"),
        )
    }

    #[test]
    fn apply_routing_rules_writes_to_ebpf_map() {
        let ebpf = make_ebpf();
        let adapter = EbpfFeedbackAdapter::new(ebpf.clone());

        let rules = vec![
            EbpfRoutingRule {
                dest: MatrixPosition { x: 1.0, y: 2.0, z: 3.0 },
                next_hop: MatrixPosition { x: 4.0, y: 5.0, z: 6.0 },
            },
            EbpfRoutingRule {
                dest: MatrixPosition { x: 7.0, y: 8.0, z: 9.0 },
                next_hop: MatrixPosition { x: 10.0, y: 11.0, z: 12.0 },
            },
        ];

        assert_eq!(ebpf.routing_rule_count(), 0);
        adapter
            .apply_routing_rules(&rules)
            .expect("apply rules succeeds");
        assert_eq!(ebpf.routing_rule_count(), 2);

        let got = ebpf
            .get_routing_rule(&MatrixPosition { x: 1.0, y: 2.0, z: 3.0 })
            .expect("rule present");
        assert_eq!(got.x, 4.0);
        assert_eq!(got.y, 5.0);
        assert_eq!(got.z, 6.0);
    }

    #[test]
    fn apply_privacy_action_no_change_is_noop() {
        let ebpf = make_ebpf();
        let adapter = EbpfFeedbackAdapter::new(ebpf.clone());
        adapter
            .apply_privacy_action(EbpfPrivacyAction::NoChange)
            .expect("no-change is a no-op");
    }

    #[test]
    fn apply_privacy_action_tighten_succeeds() {
        let ebpf = make_ebpf();
        let adapter = EbpfFeedbackAdapter::new(ebpf.clone());
        adapter
            .apply_privacy_action(EbpfPrivacyAction::Tighten)
            .expect("tighten propagates to ebpf");
    }
}
