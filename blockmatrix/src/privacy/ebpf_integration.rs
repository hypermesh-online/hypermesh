// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// eBPF Integration for Privacy Tiers
// Bridges the privacy system with kernel-level enforcement via hypermesh-ebpf

use super::flexibility_matrix::PrivacyFlexibilityMatrix;
use super::policies::PolicyManager as PrivacyPolicyManager;
use hypermesh_lib::PrivacyMode;
use hypermesh_ebpf::policy_maps::{ValidationPolicy, PolicyManager as EbpfPolicyManager};
use std::sync::Arc;

/// Privacy-aware eBPF integration
pub struct PrivacyEbpfBridge {
    /// Privacy policy manager
    _privacy_manager: Arc<PrivacyPolicyManager>,
    /// eBPF policy manager
    ebpf_manager: Arc<EbpfPolicyManager>,
}

impl PrivacyEbpfBridge {
    /// Create a new privacy-eBPF bridge
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            _privacy_manager: Arc::new(PrivacyPolicyManager::new()),
            ebpf_manager: Arc::new(EbpfPolicyManager::new()?),
        })
    }

    /// Update eBPF policies based on privacy mode
    pub fn update_ebpf_for_tier(&self, mode: PrivacyMode, connection_id: u64) {
        let ebpf_policy = Self::privacy_mode_to_ebpf_policy(mode);
        self.ebpf_manager.set_policy(connection_id, ebpf_policy);
    }

    /// Update eBPF policies for flexibility matrix configuration
    pub fn update_ebpf_for_matrix(&self, matrix: &PrivacyFlexibilityMatrix, connection_id: u64) {
        // Use the stricter of network or asset tier for eBPF enforcement
        let stricter_tier = if matrix.network_tier.caesar_multiplier() > matrix.asset_tier.caesar_multiplier() {
            matrix.asset_tier // Lower multiplier = more private
        } else {
            matrix.network_tier
        };

        self.update_ebpf_for_tier(stricter_tier, connection_id);
    }

    /// Convert privacy mode to eBPF validation policy
    fn privacy_mode_to_ebpf_policy(mode: PrivacyMode) -> ValidationPolicy {
        ValidationPolicy::for_privacy_tier(mode.to_ebpf_u8())
    }

    /// Sync all privacy policies to kernel
    pub fn sync_to_kernel(&self) -> anyhow::Result<()> {
        self.ebpf_manager.sync_to_kernel()
    }

    /// Get eBPF policy for connection
    pub fn get_ebpf_policy(&self, connection_id: u64) -> ValidationPolicy {
        self.ebpf_manager.get_policy(connection_id)
    }

    /// Clear all eBPF policies
    pub fn clear_ebpf_policies(&self) {
        self.ebpf_manager.clear_policies();
    }

    /// Set default eBPF policy based on privacy mode
    pub fn set_default_ebpf_tier(&self, mode: PrivacyMode) {
        let ebpf_policy = Self::privacy_mode_to_ebpf_policy(mode);
        self.ebpf_manager.set_default_policy(ebpf_policy);
    }
}

/// eBPF-aware privacy metrics
#[derive(Debug, Clone, Default)]
pub struct PrivacyEbpfMetrics {
    /// Packets filtered by tier
    pub anonymous_filtered: u64,
    pub private_p2p_filtered: u64,
    pub federated_filtered: u64,
    pub public_filtered: u64,
    /// Validation failures
    pub pos_validation_failures: u64,
    pub asset_hash_failures: u64,
    pub matrix_routing_failures: u64,
    /// Rate limit violations per tier
    pub rate_limit_violations: [u64; 4],
}

impl PrivacyEbpfMetrics {
    /// Update metrics based on eBPF events
    pub fn update_from_ebpf_event(&mut self, tier: u8, event_type: EbpfEventType) {
        match event_type {
            EbpfEventType::PacketFiltered => {
                match tier {
                    0 => self.anonymous_filtered += 1,
                    1 => self.private_p2p_filtered += 1,
                    2 => self.federated_filtered += 1,
                    3 => self.public_filtered += 1,
                    _ => {}
                }
            }
            EbpfEventType::PosValidationFailed => self.pos_validation_failures += 1,
            EbpfEventType::AssetHashFailed => self.asset_hash_failures += 1,
            EbpfEventType::MatrixRoutingFailed => self.matrix_routing_failures += 1,
            EbpfEventType::RateLimitExceeded => {
                if tier < 4 {
                    self.rate_limit_violations[tier as usize] += 1;
                }
            }
        }
    }

    /// Get total filtered packets across all tiers
    pub fn total_filtered(&self) -> u64 {
        self.anonymous_filtered + self.private_p2p_filtered +
        self.federated_filtered + self.public_filtered
    }

    /// Get total validation failures
    pub fn total_validation_failures(&self) -> u64 {
        self.pos_validation_failures + self.asset_hash_failures + self.matrix_routing_failures
    }
}

/// eBPF event types for privacy system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EbpfEventType {
    PacketFiltered,
    PosValidationFailed,
    AssetHashFailed,
    MatrixRoutingFailed,
    RateLimitExceeded,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privacy_ebpf_bridge_creation() {
        let bridge = PrivacyEbpfBridge::new();
        assert!(bridge.is_ok());
    }

    #[test]
    fn test_tier_to_ebpf_policy_conversion() {
        let bridge = PrivacyEbpfBridge::new().unwrap();

        // Test Anonymous mode
        bridge.update_ebpf_for_tier(PrivacyMode::ANONYMOUS, 100);
        let policy = bridge.get_ebpf_policy(100);
        assert!(!policy.requires_pos);
        assert!(!policy.validate_asset_hash);
        assert_eq!(policy.privacy_tier, 0);

        // Test Public mode
        bridge.update_ebpf_for_tier(PrivacyMode::PUBLIC, 200);
        let policy = bridge.get_ebpf_policy(200);
        assert!(policy.requires_pos);
        assert!(policy.validate_asset_hash);
        assert_eq!(policy.privacy_tier, 3);
    }

    #[test]
    fn test_flexibility_matrix_ebpf_update() {
        let bridge = PrivacyEbpfBridge::new().unwrap();
        let matrix = PrivacyFlexibilityMatrix::new(
            PrivacyMode::ANONYMOUS,
            PrivacyMode::PUBLIC,
        );

        bridge.update_ebpf_for_matrix(&matrix, 300);
        let policy = bridge.get_ebpf_policy(300);

        // Should use Anonymous mode (stricter)
        assert_eq!(policy.privacy_tier, 0);
    }

    #[test]
    fn test_default_ebpf_tier_setting() {
        let bridge = PrivacyEbpfBridge::new().unwrap();

        bridge.set_default_ebpf_tier(PrivacyMode::PRIVATE);

        // Unknown connection should use default
        let policy = bridge.get_ebpf_policy(999);
        assert_eq!(policy.privacy_tier, 2); // Private maps to ebpf u8 = 2
    }

    #[test]
    fn test_privacy_ebpf_metrics() {
        let mut metrics = PrivacyEbpfMetrics::default();

        metrics.update_from_ebpf_event(0, EbpfEventType::PacketFiltered);
        metrics.update_from_ebpf_event(3, EbpfEventType::PacketFiltered);
        metrics.update_from_ebpf_event(0, EbpfEventType::PosValidationFailed);
        metrics.update_from_ebpf_event(2, EbpfEventType::RateLimitExceeded);

        assert_eq!(metrics.anonymous_filtered, 1);
        assert_eq!(metrics.public_filtered, 1);
        assert_eq!(metrics.total_filtered(), 2);
        assert_eq!(metrics.pos_validation_failures, 1);
        assert_eq!(metrics.rate_limit_violations[2], 1);
    }

    #[test]
    fn test_clear_ebpf_policies() {
        let bridge = PrivacyEbpfBridge::new().unwrap();

        bridge.update_ebpf_for_tier(PrivacyMode::PUBLIC, 400);
        bridge.update_ebpf_for_tier(PrivacyMode::ANONYMOUS, 401);

        bridge.clear_ebpf_policies();

        // Should revert to default policy
        let policy = bridge.get_ebpf_policy(400);
        assert_eq!(policy.privacy_tier, 1); // Default tier
    }
}
