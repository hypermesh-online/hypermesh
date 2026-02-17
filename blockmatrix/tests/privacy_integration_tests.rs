// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// Integration tests for the privacy system
// Tests all four tiers, flexibility matrix, tier switching, and eBPF integration

use blockmatrix::privacy::{
    PrivacySystem, PrivacyConfig, PrivacyTier, TrustLevel,
    PrivacyFlexibilityMatrix, TierSwitcher, PolicyManager,
    PolicyAction, PolicyDecision, ActionType, ValidationType,
    PrivacyEbpfBridge, PrivacyEbpfMetrics, EbpfEventType,
    PrivacyPresets, AnonymousTier, PrivateP2PTier, FederatedTier, PublicTier,
};
use std::collections::HashSet;

#[test]
fn test_all_four_privacy_tiers() {
    // Test Anonymous tier
    let anon = AnonymousTier::new();
    assert!(anon.connection_id().len() == 16);

    // Test Private P2P tier
    let mut p2p = PrivateP2PTier::new(10);
    let peer_id = [1u8; 32];
    assert!(p2p.add_peer(peer_id).is_ok());
    assert!(p2p.is_trusted(&peer_id));

    // Test Federated tier
    let mut federated = FederatedTier::new(5);
    let network_id = [1u8; 16];
    assert!(federated.add_partner(network_id, TrustLevel::Partner).is_ok());
    assert!(federated.meets_trust_requirement(&network_id));

    // Test Public tier
    let mut public = PublicTier::new([0u8; 32]);
    public.update_reputation(true);
    assert!(public.reputation > 0.5);
}

#[test]
fn test_privacy_flexibility_matrix_configurations() {
    // Test uniform configuration
    let uniform = PrivacyFlexibilityMatrix::uniform(PrivacyTier::Public);
    assert_eq!(uniform.network_tier, PrivacyTier::Public);
    assert_eq!(uniform.asset_tier, PrivacyTier::Public);
    assert_eq!(uniform.caesar_multiplier(), 1.0);

    // Test anonymous network with public assets
    let anon_public = PrivacyFlexibilityMatrix::new(
        PrivacyTier::Anonymous,
        PrivacyTier::Public
    );
    assert!(anon_public.is_anonymous_public());
    assert!(anon_public.caesar_multiplier() > 0.5); // Gets bonus

    // Test privacy-focused configuration
    let private_config = PrivacyFlexibilityMatrix::new(
        PrivacyTier::PrivateP2P,
        PrivacyTier::PrivateP2P
    );
    assert!(private_config.is_privacy_focused());
    assert_eq!(private_config.privacy_score(), 0.7);

    // Test asset overrides
    let mut matrix = PrivacyFlexibilityMatrix::uniform(PrivacyTier::Federated);
    let asset_id = [42u8; 32];
    matrix.set_asset_override(asset_id, PrivacyTier::Anonymous);
    assert_eq!(matrix.get_asset_tier(&asset_id), PrivacyTier::Anonymous);
}

#[test]
fn test_seamless_tier_switching() {
    let mut switcher = TierSwitcher::new(PrivacyTier::Anonymous);

    // Test switching from Anonymous to Public
    let result = switcher.switch_tier(PrivacyTier::Public);
    assert!(result.is_ok());
    assert_eq!(switcher.current_tier(), PrivacyTier::Public);

    // Test switching to Private P2P
    let result = switcher.switch_tier(PrivacyTier::PrivateP2P);
    assert!(result.is_ok());
    assert_eq!(switcher.current_tier(), PrivacyTier::PrivateP2P);

    // Check transition history
    let history = switcher.transition_history();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].from, PrivacyTier::Anonymous);
    assert_eq!(history[0].to, PrivacyTier::Public);
}

#[test]
fn test_tier_specific_policies() {
    let mut manager = PolicyManager::new();

    // Test Anonymous tier policy
    let mut validations = HashSet::new();
    let anon_action = PolicyAction {
        action_type: ActionType::Connect,
        actor: None,
        target: None,
        provided_validations: validations.clone(),
        queries_identity: false,
        queries_location: false,
        high_value: false,
    };
    assert!(manager.enforce(PrivacyTier::Anonymous, anon_action).is_ok());

    // Test Public tier policy - requires full validation
    validations.insert(ValidationType::FullIdentity);
    validations.insert(ValidationType::ProofOfSpace);
    validations.insert(ValidationType::ProofOfStake);
    validations.insert(ValidationType::ProofOfWork);
    validations.insert(ValidationType::ProofOfTime);

    let public_action = PolicyAction {
        action_type: ActionType::ValidateBlock,
        actor: Some([1u8; 32]),
        target: Some([2u8; 32]),
        provided_validations: validations,
        queries_identity: false,
        queries_location: false,
        high_value: true,
    };
    assert!(manager.enforce(PrivacyTier::Public, public_action).is_ok());
}

#[test]
fn test_ebpf_integration_with_privacy_tiers() {
    let bridge = PrivacyEbpfBridge::new().unwrap();

    // Test setting policies for each tier
    bridge.update_ebpf_for_tier(PrivacyTier::Anonymous, 100);
    let policy = bridge.get_ebpf_policy(100);
    assert_eq!(policy.privacy_tier, 0);
    assert!(!policy.requires_pos);

    bridge.update_ebpf_for_tier(PrivacyTier::PrivateP2P, 101);
    let policy = bridge.get_ebpf_policy(101);
    assert_eq!(policy.privacy_tier, 1);
    assert!(!policy.requires_pos);
    assert!(policy.validate_asset_hash);

    bridge.update_ebpf_for_tier(PrivacyTier::Federated, 102);
    let policy = bridge.get_ebpf_policy(102);
    assert_eq!(policy.privacy_tier, 2);
    assert!(policy.requires_pos);

    bridge.update_ebpf_for_tier(PrivacyTier::Public, 103);
    let policy = bridge.get_ebpf_policy(103);
    assert_eq!(policy.privacy_tier, 3);
    assert!(policy.requires_pos);
    assert!(policy.validate_asset_hash);
    assert!(policy.check_matrix_routing);
}

#[test]
fn test_privacy_system_end_to_end() {
    let mut system = PrivacySystem::new();

    // Start in default tier (Federated)
    assert_eq!(system.current_tier(), PrivacyTier::Federated);

    // Switch to Anonymous
    assert!(system.switch_tier(PrivacyTier::Anonymous).is_ok());
    assert_eq!(system.current_tier(), PrivacyTier::Anonymous);

    // Update flexibility matrix
    let matrix = PrivacyFlexibilityMatrix::new(
        PrivacyTier::Anonymous,
        PrivacyTier::Public
    );
    assert!(system.update_flexibility_matrix(matrix).is_ok());

    // Check CAESAR multiplier
    let multiplier = system.caesar_multiplier();
    assert!(multiplier > 0.5); // Should get bonus for anonymous-public

    // Check privacy score
    assert_eq!(system.privacy_score(), 0.5); // Average of anonymous and public

    // Test policy enforcement
    let action = PolicyAction {
        action_type: ActionType::QueryPublic,
        actor: None,
        target: None,
        provided_validations: HashSet::new(),
        queries_identity: false,
        queries_location: false,
        high_value: false,
    };
    assert!(system.enforce_policy(action).is_ok());
}

#[test]
fn test_privacy_presets() {
    // Test maximum privacy preset
    let max_privacy = PrivacyPresets::maximum_privacy();
    assert_eq!(max_privacy.network_tier, PrivacyTier::Anonymous);
    assert_eq!(max_privacy.asset_tier, PrivacyTier::Anonymous);
    assert_eq!(max_privacy.privacy_score(), 1.0);

    // Test maximum rewards preset
    let max_rewards = PrivacyPresets::maximum_rewards();
    assert_eq!(max_rewards.network_tier, PrivacyTier::Public);
    assert_eq!(max_rewards.asset_tier, PrivacyTier::Public);
    assert_eq!(max_rewards.caesar_multiplier(), 1.0);

    // Test balanced preset
    let balanced = PrivacyPresets::balanced();
    assert_eq!(balanced.network_tier, PrivacyTier::Federated);
    assert_eq!(balanced.asset_tier, PrivacyTier::Federated);

    // Test anonymous contributor preset
    let anon_contrib = PrivacyPresets::anonymous_contributor();
    assert!(anon_contrib.is_anonymous_public());

    // Test private group preset
    let private_group = PrivacyPresets::private_group();
    assert!(private_group.is_privacy_focused());
}

#[test]
fn test_privacy_metrics_tracking() {
    let mut metrics = PrivacyEbpfMetrics::default();

    // Simulate events for each tier
    metrics.update_from_ebpf_event(0, EbpfEventType::PacketFiltered);
    metrics.update_from_ebpf_event(1, EbpfEventType::PacketFiltered);
    metrics.update_from_ebpf_event(2, EbpfEventType::PacketFiltered);
    metrics.update_from_ebpf_event(3, EbpfEventType::PacketFiltered);

    assert_eq!(metrics.total_filtered(), 4);

    // Simulate validation failures
    metrics.update_from_ebpf_event(2, EbpfEventType::PosValidationFailed);
    metrics.update_from_ebpf_event(3, EbpfEventType::AssetHashFailed);
    metrics.update_from_ebpf_event(3, EbpfEventType::MatrixRoutingFailed);

    assert_eq!(metrics.total_validation_failures(), 3);

    // Simulate rate limit violations
    for tier in 0..4 {
        metrics.update_from_ebpf_event(tier, EbpfEventType::RateLimitExceeded);
    }
    assert_eq!(metrics.rate_limit_violations.iter().sum::<u64>(), 4);
}

#[test]
fn test_validation_requirements_per_tier() {
    // Anonymous - no requirements
    let anon_req = PrivacyTier::Anonymous.validation_requirements();
    assert!(!anon_req.proof_of_space);
    assert!(!anon_req.proof_of_stake);
    assert!(!anon_req.proof_of_work);
    assert!(!anon_req.proof_of_time);

    // Private P2P - peer validation only
    let p2p_req = PrivacyTier::PrivateP2P.validation_requirements();
    assert!(p2p_req.peer_validation);
    assert!(!p2p_req.proof_of_stake);

    // Federated - federation validation
    let fed_req = PrivacyTier::Federated.validation_requirements();
    assert!(fed_req.federation_validation);
    assert!(fed_req.proof_of_space);
    assert!(fed_req.proof_of_time);

    // Public - full validation
    let pub_req = PrivacyTier::Public.validation_requirements();
    assert!(pub_req.proof_of_space);
    assert!(pub_req.proof_of_stake);
    assert!(pub_req.proof_of_work);
    assert!(pub_req.proof_of_time);
}

#[test]
fn test_caesar_rewards_calculation() {
    // Test each tier's base multiplier
    assert_eq!(PrivacyTier::Anonymous.caesar_multiplier(), 0.1);
    assert_eq!(PrivacyTier::PrivateP2P.caesar_multiplier(), 0.4);
    assert_eq!(PrivacyTier::Federated.caesar_multiplier(), 0.7);
    assert_eq!(PrivacyTier::Public.caesar_multiplier(), 1.0);

    // Test flexibility matrix combined multiplier
    let matrix = PrivacyFlexibilityMatrix::new(
        PrivacyTier::Anonymous,
        PrivacyTier::Public
    );
    // Should get bonus for contributing publicly while anonymous
    assert!(matrix.caesar_multiplier() > 0.55);
}

#[test]
fn test_connection_migration_during_switch() {
    let mut switcher = TierSwitcher::new(PrivacyTier::PrivateP2P);

    // Add some peers to P2P tier
    if let Some(tier) = &mut switcher.private_tier {
        tier.add_peer([1u8; 32]).unwrap();
        tier.add_peer([2u8; 32]).unwrap();
    }

    // Switch to Federated
    let result = switcher.switch_tier(PrivacyTier::Federated);
    assert!(result.is_ok());

    // Check that migration state has connections
    let migration_state = switcher.migration_state();
    assert!(!migration_state.active_connections.is_empty());
}

#[test]
fn test_multi_tier_network_simulation() {
    // Simulate a network with nodes in different tiers
    let mut nodes = Vec::new();

    // Create nodes with different privacy configurations
    nodes.push(PrivacySystem::new()); // Federated (default)

    let mut anon_node = PrivacySystem::new();
    anon_node.switch_tier(PrivacyTier::Anonymous).unwrap();
    nodes.push(anon_node);

    let mut public_node = PrivacySystem::new();
    public_node.switch_tier(PrivacyTier::Public).unwrap();
    nodes.push(public_node);

    // Verify different CAESAR multipliers
    assert_ne!(nodes[0].caesar_multiplier(), nodes[1].caesar_multiplier());
    assert_ne!(nodes[1].caesar_multiplier(), nodes[2].caesar_multiplier());

    // Verify different privacy scores
    assert!(nodes[1].privacy_score() > nodes[2].privacy_score()); // Anonymous > Public
}

#[test]
fn test_policy_enforcement_across_tiers() {
    let mut system = PrivacySystem::new();

    // Create an action that requires validation
    let mut validations = HashSet::new();
    validations.insert(ValidationType::ProofOfStake);

    let action = PolicyAction {
        action_type: ActionType::ValidateBlock,
        actor: Some([1u8; 32]),
        target: Some([2u8; 32]),
        provided_validations: validations.clone(),
        queries_identity: false,
        queries_location: false,
        high_value: true,
    };

    // Should fail in Anonymous tier (no validation)
    system.switch_tier(PrivacyTier::Anonymous).unwrap();
    assert!(system.enforce_policy(action.clone()).is_err());

    // Should fail in P2P tier (peer validation only)
    system.switch_tier(PrivacyTier::PrivateP2P).unwrap();
    assert!(system.enforce_policy(action.clone()).is_err());

    // Add more validations for Public tier
    let mut full_validations = validations;
    full_validations.insert(ValidationType::FullIdentity);
    full_validations.insert(ValidationType::ProofOfSpace);
    full_validations.insert(ValidationType::ProofOfWork);
    full_validations.insert(ValidationType::ProofOfTime);

    let public_action = PolicyAction {
        action_type: ActionType::ValidateBlock,
        actor: Some([1u8; 32]),
        target: Some([2u8; 32]),
        provided_validations: full_validations,
        queries_identity: false,
        queries_location: false,
        high_value: true,
    };

    // Should succeed in Public tier with full validation
    system.switch_tier(PrivacyTier::Public).unwrap();
    assert!(system.enforce_policy(public_action).is_ok());
}