// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// End-to-end tests for privacy system
// Tests complete workflows and multi-tier interactions
//
// Gated: references private fields and APIs not yet stabilized.
#![cfg(feature = "future-tests")]

use blockmatrix::privacy::{
    PrivacySystem, PrivacyConfig, PrivacyMode,
    PrivacyFlexibilityMatrix, TierSwitcher,
    PolicyAction, ActionType, ValidationType,
    PrivacyEbpfBridge, TransitionResult,
    NodeId, NetworkId, TrustLevel,
    validation_requirements_for,
};
use std::collections::HashSet;
use std::time::Duration;
use std::thread;

#[test]
fn test_e2e_node_lifecycle_with_tier_transitions() {
    // Simulate a node's complete lifecycle through all modes
    let mut system = PrivacySystem::new();

    // Stage 1: Start as Anonymous
    system.switch_tier(PrivacyMode::ANONYMOUS).unwrap();
    assert_eq!(system.caesar_multiplier(), 0.0); // No rewards

    // Stage 2: Build trust, move to Private
    thread::sleep(Duration::from_millis(10)); // Simulate time passing
    system.switch_tier(PrivacyMode::PRIVATE).unwrap();
    assert_eq!(system.caesar_multiplier(), 0.5); // Medium rewards

    // Stage 3: Go fully public for maximum rewards
    thread::sleep(Duration::from_millis(10));
    system.switch_tier(PrivacyMode::PUBLIC).unwrap();
    assert_eq!(system.caesar_multiplier(), 1.0); // Maximum rewards

    // Verify final state
    assert_eq!(system.current_tier(), PrivacyMode::PUBLIC);
}

#[test]
fn test_e2e_mixed_privacy_network() {
    // Create a network with different privacy configurations
    let mut anonymous_nodes = Vec::new();
    let mut public_nodes = Vec::new();
    let mut hybrid_nodes = Vec::new();

    // Create 3 anonymous nodes
    for _ in 0..3 {
        let mut node = PrivacySystem::new();
        node.switch_tier(PrivacyMode::ANONYMOUS).unwrap();
        anonymous_nodes.push(node);
    }

    // Create 3 public nodes
    for _ in 0..3 {
        let mut node = PrivacySystem::new();
        node.switch_tier(PrivacyMode::PUBLIC).unwrap();
        public_nodes.push(node);
    }

    // Create 3 hybrid nodes (anonymous network, public assets)
    for _ in 0..3 {
        let mut node = PrivacySystem::new();
        node.switch_tier(PrivacyMode::ANONYMOUS).unwrap();
        let matrix = PrivacyFlexibilityMatrix::new(
            PrivacyMode::ANONYMOUS,
            PrivacyMode::PUBLIC
        );
        node.update_flexibility_matrix(matrix).unwrap();
        hybrid_nodes.push(node);
    }

    // Verify different configurations
    assert!(anonymous_nodes[0].privacy_score() > public_nodes[0].privacy_score());
    assert!(hybrid_nodes[0].caesar_multiplier() > anonymous_nodes[0].caesar_multiplier());
}

#[test]
fn test_e2e_policy_enforcement_workflow() {
    let mut system = PrivacySystem::new();
    let bridge = PrivacyEbpfBridge::new().unwrap();

    // Workflow 1: Anonymous user trying to access restricted resource
    system.switch_tier(PrivacyMode::ANONYMOUS).unwrap();
    bridge.update_ebpf_for_tier(PrivacyMode::ANONYMOUS, 1000);

    let restricted_action = PolicyAction {
        action_type: ActionType::AccessResource,
        actor: None, // Anonymous
        target: Some([1u8; 32]),
        provided_validations: HashSet::new(),
        queries_identity: true, // This should fail for anonymous
        queries_location: false,
        high_value: false,
    };

    assert!(system.enforce_policy(restricted_action).is_err());

    // Workflow 2: Public node with full validation
    system.switch_tier(PrivacyMode::PUBLIC).unwrap();
    bridge.update_ebpf_for_tier(PrivacyMode::PUBLIC, 1001);

    let mut validations = HashSet::new();
    validations.insert(ValidationType::FullIdentity);
    validations.insert(ValidationType::ProofOfSpace);
    validations.insert(ValidationType::ProofOfStake);
    validations.insert(ValidationType::ProofOfWork);
    validations.insert(ValidationType::ProofOfTime);

    let validated_action = PolicyAction {
        action_type: ActionType::ValidateBlock,
        actor: Some([1u8; 32]),
        target: Some([2u8; 32]),
        provided_validations: validations,
        queries_identity: true,
        queries_location: true,
        high_value: true,
    };

    assert!(system.enforce_policy(validated_action).is_ok());
}

#[test]
fn test_e2e_private_group_setup() {
    // Simulate setting up a private group network
    let mut coordinator = PrivacySystem::new();
    // Default is already PRIVATE, but switch explicitly for clarity
    coordinator.switch_tier(PrivacyMode::PRIVATE).unwrap();

    let mut members = Vec::new();
    for _i in 0..5 {
        let mut member = PrivacySystem::new();
        member.switch_tier(PrivacyMode::PRIVATE).unwrap();
        members.push(member);
    }

    // All private group members should have same mode
    for member in &members {
        assert_eq!(member.current_tier(), PrivacyMode::PRIVATE);
        assert_eq!(member.caesar_multiplier(), 0.5);
    }

    // Verify private validation requirements
    let req = validation_requirements_for(&PrivacyMode::PRIVATE);
    assert!(req.peer_validation);
    assert!(!req.proof_of_stake); // PRIVATE uses peer validation, not full PoS
}

#[test]
fn test_e2e_privacy_tier_daily_limits() {
    let mut config = PrivacyConfig::default();
    config.max_switches_per_day = 2;
    let mut system = PrivacySystem::with_config(config);

    // First switch - OK
    assert!(system.switch_tier(PrivacyMode::ANONYMOUS).is_ok());

    // Second switch - OK
    assert!(system.switch_tier(PrivacyMode::PUBLIC).is_ok());

    // Third switch - Should fail (daily limit)
    assert!(system.switch_tier(PrivacyMode::PRIVATE).is_err());
}

#[test]
fn test_e2e_asset_privacy_independence() {
    let mut system = PrivacySystem::new();

    // Set network to anonymous but assets to public
    system.switch_tier(PrivacyMode::ANONYMOUS).unwrap();
    let matrix = PrivacyFlexibilityMatrix::new(
        PrivacyMode::ANONYMOUS,
        PrivacyMode::PUBLIC
    );
    system.update_flexibility_matrix(matrix.clone()).unwrap();

    // Network should be anonymous
    assert_eq!(system.current_tier(), PrivacyMode::ANONYMOUS);

    // But assets should be public (check via matrix)
    assert_eq!(matrix.asset_tier, PrivacyMode::PUBLIC);
    assert!(matrix.is_anonymous_public());

    // Should get bonus CAESAR rewards for this configuration
    assert!(system.caesar_multiplier() > 0.5);
}

#[test]
fn test_e2e_transition_with_active_connections() {
    let mut switcher = TierSwitcher::new(PrivacyMode::PRIVATE);

    // Simulate active private connections
    if let Some(tier) = &mut switcher.private_tier {
        for i in 0..5 {
            tier.add_peer([i; 32]).unwrap();
        }
    }

    // Transition to Public while maintaining connections
    match switcher.switch_tier(PrivacyMode::PUBLIC) {
        Ok(TransitionResult::Success(record)) => {
            assert_eq!(record.from, PrivacyMode::PRIVATE);
            assert_eq!(record.to, PrivacyMode::PUBLIC);
            assert!(record.success);
        }
        _ => panic!("Transition should succeed"),
    }

    // Verify connections were migrated
    assert!(!switcher.migration_state().active_connections.is_empty());
}

#[test]
fn test_e2e_ebpf_policy_sync() {
    let bridge = PrivacyEbpfBridge::new().unwrap();

    // Set policies for multiple connections with different modes
    let connections = vec![
        (100, PrivacyMode::ANONYMOUS),
        (101, PrivacyMode::PRIVATE),
        (103, PrivacyMode::PUBLIC),
    ];

    for (conn_id, tier) in &connections {
        bridge.update_ebpf_for_tier(*tier, *conn_id);
    }

    // Verify each connection has correct policy
    assert_eq!(bridge.get_ebpf_policy(100).privacy_tier, 0); // ANONYMOUS
    assert_eq!(bridge.get_ebpf_policy(101).privacy_tier, 2); // PRIVATE
    assert_eq!(bridge.get_ebpf_policy(103).privacy_tier, 3); // PUBLIC

    // Sync to kernel (would be actual eBPF maps in production)
    assert!(bridge.sync_to_kernel().is_ok());
}

#[test]
fn test_e2e_privacy_preset_deployment() {
    use blockmatrix::privacy::PrivacyPresets;

    // Deploy different preset configurations
    let configs = vec![
        ("MaxPrivacy", PrivacyPresets::maximum_privacy()),
        ("MaxRewards", PrivacyPresets::maximum_rewards()),
        ("Balanced", PrivacyPresets::balanced()),
        ("AnonContributor", PrivacyPresets::anonymous_contributor()),
        ("PrivateGroup", PrivacyPresets::private_group()),
        ("FederatedPartner", PrivacyPresets::federated_partner()),
    ];

    for (name, config) in configs {
        // Each preset should have valid configuration
        assert!(config.validate_configuration().is_ok(),
                "Preset {} failed validation", name);

        // Each should have different characteristics
        let privacy_score = config.privacy_score();
        let caesar_mult = config.caesar_multiplier();

        match name {
            "MaxPrivacy" => {
                assert_eq!(privacy_score, 1.0);
                assert_eq!(caesar_mult, 0.0); // ANONYMOUS has 0.0 caesar multiplier
            }
            "MaxRewards" => {
                assert_eq!(privacy_score, 0.0);
                assert_eq!(caesar_mult, 1.0);
            }
            "AnonContributor" => {
                assert!(config.is_anonymous_public());
                assert!(caesar_mult > 0.5); // Gets bonus
            }
            _ => {}
        }
    }
}

#[test]
fn test_e2e_multi_tier_resource_sharing() {
    // Simulate resource sharing across different privacy modes
    let mut nodes = Vec::new();

    // Create nodes in each mode
    for tier in &[PrivacyMode::ANONYMOUS, PrivacyMode::PRIVATE, PrivacyMode::PUBLIC] {
        let mut node = PrivacySystem::new();
        node.switch_tier(*tier).unwrap();
        nodes.push((*tier, node));
    }

    // Test resource sharing permissions
    for (tier, node) in &nodes {
        let mut validations = HashSet::new();

        // Add validations based on mode
        if *tier == PrivacyMode::ANONYMOUS {
            // No validations needed
        } else if *tier == PrivacyMode::PRIVATE {
            validations.insert(ValidationType::PeerIdentity);
            validations.insert(ValidationType::PeerTrust);
            validations.insert(ValidationType::NetworkIdentity);
            validations.insert(ValidationType::FederationMembership);
        } else {
            // PUBLIC
            validations.insert(ValidationType::FullIdentity);
            validations.insert(ValidationType::ProofOfSpace);
        }

        let share_action = PolicyAction {
            action_type: ActionType::ShareResource,
            actor: Some([1u8; 32]),
            target: Some([2u8; 32]),
            provided_validations: validations,
            queries_identity: false,
            queries_location: false,
            high_value: false,
        };

        // Anonymous mode doesn't allow resource sharing
        if *tier == PrivacyMode::ANONYMOUS {
            continue; // Skip anonymous as it has limited permissions
        }

        // Other modes should allow with proper validation
        let result = node.enforce_policy(share_action);
        assert!(result.is_ok(), "Resource sharing failed for {:?}", tier);
    }
}

#[test]
fn test_e2e_privacy_score_impact() {
    let mut systems = Vec::new();

    // Create systems with different privacy scores (3 modes, not 4)
    let configs = vec![
        (PrivacyMode::ANONYMOUS, PrivacyMode::ANONYMOUS),   // Score: 1.0
        (PrivacyMode::PRIVATE, PrivacyMode::PRIVATE),       // Score: 0.7
        (PrivacyMode::PUBLIC, PrivacyMode::PUBLIC),          // Score: 0.0
    ];

    for (network_tier, asset_tier) in configs {
        let mut system = PrivacySystem::new();
        system.switch_tier(network_tier).unwrap();
        let matrix = PrivacyFlexibilityMatrix::new(network_tier, asset_tier);
        system.update_flexibility_matrix(matrix).unwrap();

        let privacy_score = system.privacy_score();
        let openness_score = system.openness_score();

        // Privacy and openness should be inverse
        assert!((privacy_score + openness_score - 1.0).abs() < 0.01);

        systems.push((network_tier, privacy_score));
    }

    // Verify privacy scores are in descending order
    for i in 0..systems.len() - 1 {
        assert!(systems[i].1 >= systems[i + 1].1,
                "{:?} should have higher privacy than {:?}",
                systems[i].0, systems[i + 1].0);
    }
}
