// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! AcceptanceCriteria -- published once by each user to the Network chain.
//!
//! Enables "Visa model" autonomous settlement: any node can process
//! settlements for a user without them being online.

use chrono::{DateTime, Utc};
use hypermesh_lib::{GoldGrams, MarketTier, NodeId};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Acceptance criteria published by a user to enable autonomous settlement.
///
/// Once published to the Network chain, any node can process settlements
/// for this user without them being online.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptanceCriteria {
    /// Node ID of the user/recipient.
    pub node_id: NodeId,
    /// Accepted market tiers (e.g., accept L0-L2 but not L3).
    pub accepted_tiers: Vec<MarketTier>,
    /// Accepted UPI adapter IDs for settlement (e.g., ["stripe_us", "uniswap_v3"]).
    /// Empty = accept all adapters.
    pub accepted_adapters: Vec<String>,
    /// Auto-settle threshold -- packets below this value settle automatically.
    /// Packets above require explicit confirmation (or delegation).
    pub auto_settle_threshold: GoldGrams,
    /// Delegate nodes that can settle on behalf of this user.
    /// Empty = any qualified node can settle (most permissive).
    pub delegates: Vec<NodeId>,
    /// Preferred settlement denomination (e.g., "USD", "ETH").
    pub preferred_denomination: Option<String>,
    /// Maximum fee tolerance as fraction (e.g., 0.02 = 2%).
    pub max_fee_tolerance: Decimal,
    /// Published timestamp.
    pub published_at: DateTime<Utc>,
    /// Criteria version (increment to update).
    pub version: u64,
    /// Whether this criteria is currently active.
    pub is_active: bool,
}

impl AcceptanceCriteria {
    /// Create new criteria with sensible defaults.
    ///
    /// - Accepts all tiers (L0-L3)
    /// - Accepts all adapters (empty vec)
    /// - Auto-settle threshold = 10g gold (~$800)
    /// - No delegates (any node can settle)
    /// - No preferred denomination
    /// - Max fee tolerance = 2%
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            accepted_tiers: vec![
                MarketTier::L0,
                MarketTier::L1,
                MarketTier::L2,
                MarketTier::L3,
            ],
            accepted_adapters: Vec::new(),
            auto_settle_threshold: GoldGrams::from_decimal(Decimal::new(10, 0)),
            delegates: Vec::new(),
            preferred_denomination: None,
            max_fee_tolerance: Decimal::new(2, 2), // 0.02 = 2%
            published_at: Utc::now(),
            version: 1,
            is_active: true,
        }
    }

    /// Whether the given tier is accepted by this criteria.
    pub fn accepts_tier(&self, tier: MarketTier) -> bool {
        self.accepted_tiers.contains(&tier)
    }

    /// Whether the given adapter is accepted.
    ///
    /// Empty `accepted_adapters` means accept all.
    pub fn accepts_adapter(&self, adapter_id: &str) -> bool {
        if self.accepted_adapters.is_empty() {
            return true;
        }
        self.accepted_adapters.iter().any(|a| a == adapter_id)
    }

    /// Whether the given value can be auto-settled (below threshold).
    pub fn can_auto_settle(&self, value: GoldGrams) -> bool {
        value <= self.auto_settle_threshold
    }

    /// Whether the given node is authorized to settle on behalf of this user.
    ///
    /// Empty `delegates` means any node is authorized.
    pub fn is_authorized_settler(&self, node_id: &NodeId) -> bool {
        if self.delegates.is_empty() {
            return true;
        }
        self.delegates.iter().any(|d| d == node_id)
    }

    /// Whether the given fee fraction is within tolerance.
    pub fn accepts_fee(&self, fee_fraction: Decimal) -> bool {
        fee_fraction <= self.max_fee_tolerance
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn test_node() -> NodeId {
        NodeId::from_public_key(b"test-recipient-node")
    }

    #[test]
    fn default_criteria_accepts_all_tiers() {
        let criteria = AcceptanceCriteria::new(test_node());
        assert!(criteria.accepts_tier(MarketTier::L0));
        assert!(criteria.accepts_tier(MarketTier::L1));
        assert!(criteria.accepts_tier(MarketTier::L2));
        assert!(criteria.accepts_tier(MarketTier::L3));
    }

    #[test]
    fn restricted_tiers_reject_unlisted() {
        let mut criteria = AcceptanceCriteria::new(test_node());
        criteria.accepted_tiers = vec![MarketTier::L0, MarketTier::L1];
        assert!(criteria.accepts_tier(MarketTier::L0));
        assert!(criteria.accepts_tier(MarketTier::L1));
        assert!(!criteria.accepts_tier(MarketTier::L2));
        assert!(!criteria.accepts_tier(MarketTier::L3));
    }

    #[test]
    fn empty_adapters_accepts_all() {
        let criteria = AcceptanceCriteria::new(test_node());
        assert!(criteria.accepts_adapter("stripe_us"));
        assert!(criteria.accepts_adapter("uniswap_v3"));
        assert!(criteria.accepts_adapter("anything"));
    }

    #[test]
    fn restricted_adapters_reject_unlisted() {
        let mut criteria = AcceptanceCriteria::new(test_node());
        criteria.accepted_adapters = vec!["stripe_us".to_string()];
        assert!(criteria.accepts_adapter("stripe_us"));
        assert!(!criteria.accepts_adapter("uniswap_v3"));
    }

    #[test]
    fn auto_settle_below_threshold() {
        let criteria = AcceptanceCriteria::new(test_node());
        // Default threshold is 10g
        let below = GoldGrams::from_decimal(dec!(5));
        let at = GoldGrams::from_decimal(dec!(10));
        let above = GoldGrams::from_decimal(dec!(11));
        assert!(criteria.can_auto_settle(below));
        assert!(criteria.can_auto_settle(at));
        assert!(!criteria.can_auto_settle(above));
    }

    #[test]
    fn empty_delegates_authorizes_anyone() {
        let criteria = AcceptanceCriteria::new(test_node());
        assert!(criteria.is_authorized_settler(&NodeId::from_public_key(b"random-node")));
        assert!(criteria.is_authorized_settler(&NodeId::from_public_key(b"another-node")));
    }

    #[test]
    fn restricted_delegates_reject_unauthorized() {
        let mut criteria = AcceptanceCriteria::new(test_node());
        criteria.delegates = vec![NodeId::from_public_key(b"trusted-node")];
        assert!(criteria.is_authorized_settler(&NodeId::from_public_key(b"trusted-node")));
        assert!(!criteria.is_authorized_settler(&NodeId::from_public_key(b"untrusted-node")));
    }

    #[test]
    fn fee_within_tolerance() {
        let criteria = AcceptanceCriteria::new(test_node());
        // Default tolerance is 2% (0.02)
        assert!(criteria.accepts_fee(dec!(0.01)));
        assert!(criteria.accepts_fee(dec!(0.02)));
        assert!(!criteria.accepts_fee(dec!(0.03)));
    }

    #[test]
    fn new_criteria_has_correct_defaults() {
        let criteria = AcceptanceCriteria::new(test_node());
        assert_eq!(criteria.version, 1);
        assert!(criteria.is_active);
        assert!(criteria.delegates.is_empty());
        assert!(criteria.accepted_adapters.is_empty());
        assert!(criteria.preferred_denomination.is_none());
        assert_eq!(criteria.max_fee_tolerance, dec!(0.02));
        assert_eq!(criteria.auto_settle_threshold.0, dec!(10));
    }
}
