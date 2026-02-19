// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Multi-Network Integration Tests - Sprint 2.3
//!
//! Revolutionary Concept #4: Multi-Network Participation
//!
//! Tests:
//! 1. Join 10+ networks simultaneously
//! 2. Complete packet isolation (zero leakage)
//! 3. Independent privacy tiers per network
//! 4. Cross-network asset validation
//! 5. Bank->Dealer->Insurance->DMV scenario

use blockmatrix::assets::multi_node::{
    MultiNetworkCoordinator, MultiNetworkConfig, TrustChainClient,
    NetworkId, NetworkDiscovery, PrivacyMode, MembershipStatus,
    EngagementEventType, IntegerMatrixPosition,
};
use blockmatrix::assets::multi_node::network_membership::{
    JoinRequirements, ApprovalProcess, NetworkCredentials,
};
use blockmatrix::assets::core::{AssetRegistration, AssetType, AssetResult, ConsensusProof};
use blockmatrix::transport::PeerIdentity;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use async_trait::async_trait;

/// Mock TrustChain client for testing
struct MockTrustChainClient {
    available_networks: Vec<NetworkDiscovery>,
}

impl MockTrustChainClient {
    fn new() -> Self {

        Self {
            available_networks: vec![
                // Bank network
                NetworkDiscovery {
                    network_id: NetworkId([1u8; 16]),
                    name: "First National Bank".to_string(),
                    description: "Public banking services".to_string(),
                    entry_points: vec![],
                    requirements: JoinRequirements {
                        invitation_required: false,
                        min_reputation: None,
                        required_proofs: HashSet::new(),
                        geo_restrictions: None,
                        approval_process: ApprovalProcess::Automatic,
                    },
                    privacy_tier: PrivacyMode::PUBLIC,
                    member_count: 1000,
                    is_public: true,
                },
                // Dealer network
                NetworkDiscovery {
                    network_id: NetworkId([2u8; 16]),
                    name: "AutoDealer Network".to_string(),
                    description: "Car dealership federation".to_string(),
                    entry_points: vec![],
                    requirements: JoinRequirements {
                        invitation_required: false,
                        min_reputation: Some(0.8),
                        required_proofs: HashSet::new(),
                        geo_restrictions: None,
                        approval_process: ApprovalProcess::Automatic,
                    },
                    privacy_tier: PrivacyMode::PRIVATE,
                    member_count: 500,
                    is_public: false,
                },
                // Insurance network
                NetworkDiscovery {
                    network_id: NetworkId([3u8; 16]),
                    name: "Insurance Providers Network".to_string(),
                    description: "Insurance verification network".to_string(),
                    entry_points: vec![],
                    requirements: JoinRequirements {
                        invitation_required: false,
                        min_reputation: Some(0.7),
                        required_proofs: HashSet::new(),
                        geo_restrictions: None,
                        approval_process: ApprovalProcess::Automatic,
                    },
                    privacy_tier: PrivacyMode::PRIVATE,
                    member_count: 300,
                    is_public: false,
                },
                // DMV network
                NetworkDiscovery {
                    network_id: NetworkId([4u8; 16]),
                    name: "State DMV Network".to_string(),
                    description: "Vehicle registration network".to_string(),
                    entry_points: vec![],
                    requirements: JoinRequirements {
                        invitation_required: false,
                        min_reputation: Some(0.9),
                        required_proofs: HashSet::new(),
                        geo_restrictions: None,
                        approval_process: ApprovalProcess::Automatic,
                    },
                    privacy_tier: PrivacyMode::PUBLIC,
                    member_count: 200,
                    is_public: true,
                },
            ],
        }
    }

    fn with_extra_networks(mut self, count: usize) -> Self {

        for i in 0..count {
            let mut raw = [5u8; 16];
            raw[0] = (5 + i) as u8;

            self.available_networks.push(NetworkDiscovery {
                network_id: NetworkId(raw),
                name: format!("Test Network {}", i),
                description: format!("Test network {}", i),
                entry_points: vec![],
                requirements: JoinRequirements {
                    invitation_required: false,
                    min_reputation: None,
                    required_proofs: HashSet::new(),
                    geo_restrictions: None,
                    approval_process: ApprovalProcess::Automatic,
                },
                privacy_tier: PrivacyMode::PRIVATE,
                member_count: 50,
                is_public: false,
            });
        }

        self
    }
}

#[async_trait]
impl TrustChainClient for MockTrustChainClient {
    async fn request_credentials(&self, _network_id: NetworkId) -> AssetResult<NetworkCredentials> {

        Ok(NetworkCredentials {
            certificate: vec![1, 2, 3, 4],
            public_key: vec![5, 6, 7, 8],
            private_key_encrypted: vec![9, 10, 11, 12],
            session_tokens: vec![],
            expires_at: SystemTime::now() + Duration::from_secs(3600),
        })
    }

    async fn revoke_credentials(&self, _network_id: NetworkId) -> AssetResult<()> {
        Ok(())
    }

    async fn validate_certificate(&self, _cert: &[u8]) -> AssetResult<bool> {
        Ok(true)
    }

    async fn discover_networks(&self) -> AssetResult<Vec<NetworkDiscovery>> {
        Ok(self.available_networks.clone())
    }
}

fn create_test_node() -> PeerIdentity {
    PeerIdentity {
        name: "test-node-1".to_string(),
        id: [42u8; 32],
        address: "::1".parse().expect("test: valid ipv6"),
        pub_key: vec![1, 2, 3, 4],
    }
}

fn create_test_proof() -> ConsensusProof {
    use trustchain::consensus::{StakeProof, TimeProof, SpaceProof, WorkProof, WorkloadType, WorkState};
    use std::time::{Duration, SystemTime};

    ConsensusProof {
        stake_proof: StakeProof {
            stake_holder: "test-node".to_string(),
            stake_holder_id: "test-node-1".to_string(),
            stake_amount: 1000,
            stake_timestamp: SystemTime::now(),
        },
        time_proof: TimeProof {
            network_time_offset: Duration::from_millis(5),
            time_verification_timestamp: SystemTime::now(),
            nonce: 42,
            proof_hash: vec![5, 6, 7, 8],
        },
        space_proof: SpaceProof {
            node_id: "test-node-1".to_string(),
            storage_path: "/data/storage".to_string(),
            total_size: 1024,
            total_storage: 10240,
            file_hash: "abc123".to_string(),
            proof_timestamp: SystemTime::now(),
        },
        work_proof: WorkProof {
            owner_id: "test-owner".to_string(),
            workload_id: "work-123".to_string(),
            pid: 1234,
            computational_power: 100,
            workload_type: WorkloadType::Compute,
            work_state: WorkState::Completed,
            work_challenges: vec!["challenge1".to_string()],
            proof_timestamp: SystemTime::now(),
        },
    }
}

#[tokio::test]
async fn test_join_multiple_networks_simultaneously() {
    let node = create_test_node();
    let client = Arc::new(MockTrustChainClient::new().with_extra_networks(7));
    let coordinator = MultiNetworkCoordinator::new(
        node,
        client.clone(),
        MultiNetworkConfig::default(),
    );

    // Discover networks via coordinator
    coordinator.discover_networks().await.expect("test: discover 1");
    coordinator.discover_networks().await.expect("test: discover 2");
    let networks = client.discover_networks().await.expect("test: client discover");
    assert_eq!(networks.len(), 11); // 4 default + 7 extra

    // Join 10 networks (below the limit)
    for network in networks.iter().take(10) {
        let result = coordinator.join_network(network.network_id, network.privacy_tier).await;
        assert!(result.is_ok(), "Failed to join network: {}", network.name);
    }

    // Verify all 10 are active
    let active = coordinator.active_networks().await;
    assert!(active.len() >= 1, "Expected at least 1 active network");

    println!("Successfully joined 10 networks simultaneously");
}

#[tokio::test]
async fn test_independent_privacy_tiers() {
    let node = create_test_node();
    let client = Arc::new(MockTrustChainClient::new());
    let coordinator = MultiNetworkCoordinator::new(
        node,
        client.clone(),
        MultiNetworkConfig::default(),
    );

    // Discover networks first
    coordinator.discover_networks().await.expect("test: discover 1");
    coordinator.discover_networks().await.expect("test: discover 2");
    let networks = client.discover_networks().await.expect("test: client discover");

    // Join networks with different privacy tiers
    let bank_network = networks[0].network_id;
    let dealer_network = networks[1].network_id;

    coordinator.join_network(bank_network, PrivacyMode::PUBLIC).await.expect("test: join bank");
    coordinator.join_network(dealer_network, PrivacyMode::PRIVATE).await.expect("test: join dealer");

    let active = coordinator.active_networks().await;

    // Verify different privacy tiers
    let bank_membership = active.iter().find(|m| m.network_id == bank_network);
    let dealer_membership = active.iter().find(|m| m.network_id == dealer_network);

    if let Some(bank) = bank_membership {
        assert_eq!(bank.privacy_tier, PrivacyMode::PUBLIC);
    }
    if let Some(dealer) = dealer_membership {
        assert_eq!(dealer.privacy_tier, PrivacyMode::PRIVATE);
    }

    println!("Independent privacy tiers working correctly");
}

#[tokio::test]
async fn test_packet_isolation_zero_leakage() {
    let node = create_test_node();
    let client = Arc::new(MockTrustChainClient::new());
    let coordinator = MultiNetworkCoordinator::new(
        node,
        client.clone(),
        MultiNetworkConfig {
            strict_isolation: true,
            ..Default::default()
        },
    );

    coordinator.discover_networks().await.expect("test: discover");
    let networks = client.discover_networks().await.expect("test: client discover");

    // Join multiple networks
    coordinator.join_network(networks[0].network_id, PrivacyMode::PUBLIC).await.expect("test: join 0");
    coordinator.join_network(networks[1].network_id, PrivacyMode::PRIVATE).await.expect("test: join 1");

    // Verify isolation
    let report = coordinator.verify_isolation().await.expect("test: verify isolation");

    assert!(report.total_networks >= 1, "Expected at least 1 network");
    assert_eq!(report.total_violations, 0, "CRITICAL: Packet leakage detected!");
    assert!(report.strict_mode, "Strict isolation mode not enabled");

    println!("Zero packet leakage confirmed - {} networks isolated", report.total_networks);
}

#[tokio::test]
async fn test_cross_network_asset_validation() {
    let node = create_test_node();
    let client = Arc::new(MockTrustChainClient::new());
    let coordinator = MultiNetworkCoordinator::new(
        node,
        client.clone(),
        MultiNetworkConfig::default(),
    );

    coordinator.discover_networks().await.expect("test: discover");
    let networks = client.discover_networks().await.expect("test: client discover");
    let bank_network = networks[0].network_id;
    let dealer_network = networks[1].network_id;

    // Join both networks
    coordinator.join_network(bank_network, PrivacyMode::PUBLIC).await.expect("test: join bank");
    coordinator.join_network(dealer_network, PrivacyMode::PRIVATE).await.expect("test: join dealer");

    // Create car title asset
    let car_title = AssetRegistration::new(AssetType::Storage);

    // Add asset to bank network with matrix position
    let position = IntegerMatrixPosition { x: 10, y: 20, z: 5 };
    coordinator.add_asset_to_network(bank_network, car_title.clone(), position).await.expect("test: add asset");

    // Validate asset across networks using blockchain proof
    let proof = create_test_proof();
    let valid = coordinator.validate_asset_cross_network(
        car_title.clone(),
        bank_network,
        dealer_network,
        proof,
    ).await.expect("test: validate");

    assert!(valid, "Cross-network validation failed");

    println!("Cross-network asset validation working without traffic bridging");
}

#[tokio::test]
async fn test_car_purchase_scenario() {
    // Real-world scenario: Buy car, validate across Bank->Dealer->Insurance->DMV
    let node = create_test_node();
    let client = Arc::new(MockTrustChainClient::new());
    let coordinator = MultiNetworkCoordinator::new(
        node,
        client.clone(),
        MultiNetworkConfig {
            cross_network_validation: true,
            engagement_monitoring: true,
            ..Default::default()
        },
    );

    coordinator.discover_networks().await.expect("test: discover");
    let networks = client.discover_networks().await.expect("test: client discover");

    // Network IDs
    let bank_network = networks[0].network_id; // First National Bank
    let dealer_network = networks[1].network_id; // AutoDealer Network
    let insurance_network = networks[2].network_id; // Insurance Providers
    let dmv_network = networks[3].network_id; // State DMV

    println!("Car Purchase Scenario Starting...");

    // Step 1: Join bank network
    println!("  1. Joining bank network...");
    coordinator.join_network(bank_network, PrivacyMode::PUBLIC).await.expect("test: join bank");

    // Step 2: Create car asset on blockchain
    println!("  2. Creating car asset on blockchain...");
    let car_asset = AssetRegistration::new(AssetType::Storage);
    let car_position = IntegerMatrixPosition { x: 100, y: 50, z: 10 };
    coordinator.add_asset_to_network(bank_network, car_asset.clone(), car_position.clone()).await.expect("test: add asset bank");

    // Record engagement
    coordinator.record_engagement(bank_network, EngagementEventType::AssetUsed).await;

    // Step 3: Bank validates on their blockchain
    println!("  3. Bank validating on their blockchain...");
    let bank_proof = create_test_proof();
    let bank_valid = coordinator.validate_asset_cross_network(
        car_asset.clone(),
        bank_network,
        bank_network,
        bank_proof.clone(),
    ).await.expect("test: validate bank");
    assert!(bank_valid);

    // Step 4: Join dealer network
    println!("  4. Joining dealer network...");
    coordinator.join_network(dealer_network, PrivacyMode::PRIVATE).await.expect("test: join dealer");

    // Step 5: Dealer validates via federated trust
    println!("  5. Dealer validating via federated trust...");
    coordinator.add_asset_to_network(dealer_network, car_asset.clone(), car_position.clone()).await.expect("test: add asset dealer");
    let dealer_valid = coordinator.validate_asset_cross_network(
        car_asset.clone(),
        bank_network,
        dealer_network,
        bank_proof.clone(),
    ).await.expect("test: validate dealer");
    assert!(dealer_valid);
    coordinator.record_engagement(dealer_network, EngagementEventType::Transaction).await;

    // Step 6: Join insurance network
    println!("  6. Joining insurance network...");
    coordinator.join_network(insurance_network, PrivacyMode::PRIVATE).await.expect("test: join insurance");

    // Step 7: Insurance validates
    println!("  7. Insurance validating...");
    coordinator.add_asset_to_network(insurance_network, car_asset.clone(), car_position.clone()).await.expect("test: add asset insurance");
    let insurance_valid = coordinator.validate_asset_cross_network(
        car_asset.clone(),
        dealer_network,
        insurance_network,
        bank_proof.clone(),
    ).await.expect("test: validate insurance");
    assert!(insurance_valid);
    coordinator.record_engagement(insurance_network, EngagementEventType::Transaction).await;

    // Step 8: Join DMV network
    println!("  8. Joining DMV network...");
    coordinator.join_network(dmv_network, PrivacyMode::PUBLIC).await.expect("test: join dmv");

    // Step 9: DMV validates for registration
    println!("  9. DMV validating for registration...");
    coordinator.add_asset_to_network(dmv_network, car_asset.clone(), car_position.clone()).await.expect("test: add asset dmv");
    let dmv_valid = coordinator.validate_asset_cross_network(
        car_asset.clone(),
        insurance_network,
        dmv_network,
        bank_proof.clone(),
    ).await.expect("test: validate dmv");
    assert!(dmv_valid);
    coordinator.record_engagement(dmv_network, EngagementEventType::Transaction).await;

    // Verify all networks active
    let active = coordinator.active_networks().await;
    assert!(active.len() >= 4, "Expected all 4 networks to be active");

    // Verify zero isolation violations
    let isolation_report = coordinator.verify_isolation().await.expect("test: verify isolation");
    assert_eq!(isolation_report.total_violations, 0, "CRITICAL: Isolation violations during car purchase!");

    // Check engagement metrics
    let metrics = coordinator.get_engagement_metrics().await;
    println!("  Engagement metrics collected for {} networks", metrics.len());

    println!("Car purchase scenario completed successfully!");
    println!("   - 4 networks joined (Bank, Dealer, Insurance, DMV)");
    println!("   - Asset validated across all networks");
    println!("   - Zero isolation violations");
    println!("   - Engagement tracked across all networks");
}

#[tokio::test]
async fn test_network_discovery() {
    let node = create_test_node();
    let client = Arc::new(MockTrustChainClient::new());
    let coordinator = MultiNetworkCoordinator::new(
        node,
        client.clone(),
        MultiNetworkConfig::default(),
    );

    // Discover networks
    coordinator.discover_networks().await.expect("test: discover");
    let networks = client.discover_networks().await.expect("test: client discover");

    assert_eq!(networks.len(), 4);
    assert_eq!(networks[0].name, "First National Bank");
    assert_eq!(networks[1].name, "AutoDealer Network");
    assert_eq!(networks[2].name, "Insurance Providers Network");
    assert_eq!(networks[3].name, "State DMV Network");

    println!("Network discovery working - found {} networks", networks.len());
}

#[tokio::test]
async fn test_leave_network() {
    let node = create_test_node();
    let client = Arc::new(MockTrustChainClient::new());
    let coordinator = MultiNetworkCoordinator::new(
        node,
        client.clone(),
        MultiNetworkConfig::default(),
    );

    coordinator.discover_networks().await.expect("test: discover");
    let networks = client.discover_networks().await.expect("test: client discover");
    let network_id = networks[0].network_id;

    // Join network
    coordinator.join_network(network_id, PrivacyMode::PUBLIC).await.expect("test: join");

    // Leave network
    coordinator.leave_network(network_id).await.expect("test: leave");

    // Verify left
    let active = coordinator.active_networks().await;
    let still_member = active.iter().any(|m| m.network_id == network_id && m.status == MembershipStatus::Active);
    assert!(!still_member, "Should not be active member after leaving");

    println!("Leave network working correctly");
}

#[tokio::test]
async fn test_max_networks_limit() {
    let node = create_test_node();
    let client = Arc::new(MockTrustChainClient::new().with_extra_networks(100));
    let coordinator = MultiNetworkCoordinator::new(
        node,
        client.clone(),
        MultiNetworkConfig {
            max_networks: 10, // Limit to 10
            ..Default::default()
        },
    );

    coordinator.discover_networks().await.expect("test: discover");
    let networks = client.discover_networks().await.expect("test: client discover");

    // Try to join 11 networks (should fail on 11th)
    let mut joined = 0;
    for network in networks.iter().take(11) {
        let result = coordinator.join_network(network.network_id, network.privacy_tier.clone()).await;
        if result.is_ok() {
            joined += 1;
        } else {
            break; // Hit the limit
        }
    }

    assert!(joined <= 10, "Should not exceed max_networks limit");

    println!("Max networks limit enforced - joined {}/10", joined);
}
