// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Car Purchase Scenario Example
//! 
//! This example demonstrates a more realistic car purchase workflow.
//! It simulates a car purchase involving separate nodes for a buyer, a bank, a dealer, an insurance company, and the DMV.

use blockmatrix::assets::multi_node::{
    MultiNetworkCoordinator, MultiNetworkConfig, TrustChainClient,
    NetworkId, NetworkDiscovery, PrivacyTier, MembershipStatus,
    NetworkMembership, EngagementEventType, IntegerMatrixPosition,
};
use blockmatrix::assets::multi_node::network_membership::{
    JoinRequirements, ApprovalProcess, NetworkCredentials,
};
use blockmatrix::assets::core::{AssetId, AssetType, AssetResult, ConsensusProof};
use blockmatrix::transport::NodeId;
use std::collections::{HashSet, HashMap};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use async_trait::async_trait;

/// Mock TrustChain client for testing
#[derive(Debug, Clone)]
struct MockTrustChainClient {
    available_networks: Vec<NetworkDiscovery>,
}

impl MockTrustChainClient {
    fn new() -> Self {
        Self {
            available_networks: vec![
                // Buyer's private network
                NetworkDiscovery {
                    network_id: [0u8; 16],
                    name: "Buyer Private Network".to_string(),
                    description: "Buyer's private network".to_string(),
                    entry_points: vec![],
                    requirements: JoinRequirements {
                        invitation_required: true,
                        min_reputation: None,
                        required_proofs: HashSet::new(),
                        geo_restrictions: None,
                        approval_process: ApprovalProcess::ManualAdmin,
                    },
                    privacy_tier: PrivacyTier::PrivateP2P,
                    member_count: 1,
                    is_public: false,
                },
                // Bank network
                NetworkDiscovery {
                    network_id: [1u8; 16],
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
                    privacy_tier: PrivacyTier::Public,
                    member_count: 1000,
                    is_public: true,
                },
                // Dealer network
                NetworkDiscovery {
                    network_id: [2u8; 16],
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
                    privacy_tier: PrivacyTier::Federated,
                    member_count: 500,
                    is_public: false,
                },
                // Insurance network
                NetworkDiscovery {
                    network_id: [3u8; 16],
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
                    privacy_tier: PrivacyTier::Federated,
                    member_count: 300,
                    is_public: false,
                },
                // DMV network
                NetworkDiscovery {
                    network_id: [4u8; 16],
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
                    privacy_tier: PrivacyTier::Public,
                    member_count: 200,
                    is_public: true,
                },
            ],
        }
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

fn create_node(name: &str, id: [u8; 32]) -> NodeId {
    NodeId {
        name: name.to_string(),
        id,
        address: "::1".parse().unwrap(),
        pub_key: vec![1, 2, 3, 4],
    }
}

fn create_test_proof(stake_holder: &str, stake_holder_id: &str) -> ConsensusProof {
    use trustchain::consensus::{StakeProof, TimeProof, SpaceProof, WorkProof, WorkloadType, WorkState};
    use std::time::{Duration, SystemTime};

    ConsensusProof {
        stake_proof: StakeProof {
            stake_holder: stake_holder.to_string(),
            stake_holder_id: stake_holder_id.to_string(),
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
            node_id: stake_holder_id.to_string(),
            storage_path: "/data/storage".to_string(),
            total_size: 1024,
            total_storage: 10240,
            file_hash: "abc123".to_string(),
            proof_timestamp: SystemTime::now(),
        },
        work_proof: WorkProof {
            owner_id: stake_holder.to_string(),
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

#[tokio::main]
async fn main() {
    // Real-world scenario: Buy car, validate across Bank→Dealer→Insurance→DMV
    let client = Arc::new(MockTrustChainClient::new());

    // Create separate nodes for each entity
    let buyer_node = create_node("buyer-node", [0; 32]);
    let bank_node = create_node("bank-node", [1; 32]);
    let dealer_node = create_node("dealer-node", [2; 32]);
    let insurance_node = create_node("insurance-node", [3; 32]);
    let dmv_node = create_node("dmv-node", [4; 32]);

    let buyer_coordinator = Arc::new(MultiNetworkCoordinator::new(
        buyer_node.clone(),
        client.clone(),
        MultiNetworkConfig::default(),
    ));
    let bank_coordinator = Arc::new(MultiNetworkCoordinator::new(
        bank_node.clone(),
        client.clone(),
        MultiNetworkConfig::default(),
    ));
    let dealer_coordinator = Arc::new(MultiNetworkCoordinator::new(
        dealer_node.clone(),
        client.clone(),
        MultiNetworkConfig::default(),
    ));
    let insurance_coordinator = Arc::new(MultiNetworkCoordinator::new(
        insurance_node.clone(),
        client.clone(),
        MultiNetworkConfig::default(),
    ));
    let dmv_coordinator = Arc::new(MultiNetworkCoordinator::new(
        dmv_node.clone(),
        client.clone(),
        MultiNetworkConfig::default(),
    ));

    // Discover networks
    buyer_coordinator.discover_networks().await.unwrap();
    bank_coordinator.discover_networks().await.unwrap();
    dealer_coordinator.discover_networks().await.unwrap();
    insurance_coordinator.discover_networks().await.unwrap();
    dmv_coordinator.discover_networks().await.unwrap();
    let networks = client.discover_networks().await.unwrap();

    // Network IDs
    let buyer_private_network = networks[0].network_id;
    let bank_network = networks[1].network_id;
    let dealer_network = networks[2].network_id;
    let insurance_network = networks[3].network_id;
    let dmv_network = networks[4].network_id;

    println!("🚗 Car Purchase Scenario Starting...");

    // Step 1: Entities join their primary networks
    println!("\n  1. Entities joining their primary networks...");
    buyer_coordinator.join_network(buyer_private_network, PrivacyTier::PrivateP2P).await.unwrap();
    bank_coordinator.join_network(bank_network, PrivacyTier::Public).await.unwrap();
    dealer_coordinator.join_network(dealer_network, PrivacyTier::Federated).await.unwrap();
    insurance_coordinator.join_network(insurance_network, PrivacyTier::Federated).await.unwrap();
    dmv_coordinator.join_network(dmv_network, PrivacyTier::Public).await.unwrap();
    println!("     - All entities have joined their primary networks.");

    // Step 2: Buyer finds a car on the dealer's public network and initiates purchase
    println!("\n  2. Buyer initiates purchase on Dealer's public network...");
    let car_for_sale_asset = AssetId::new(AssetType::Storage);
    dealer_coordinator.add_asset_to_network(dealer_network, car_for_sale_asset.clone(), IntegerMatrixPosition { x: 1, y: 1, z: 1 }).await.unwrap();
    println!("     - Dealer lists Car for Sale Asset: {:?}", car_for_sale_asset);

    // 3. Buyer creates a 'Purchase Intent' asset on their private chain
    println!("\n  3. Buyer creates 'Purchase Intent' on their private chain...");
    let purchase_intent = AssetId::new(AssetType::Storage);
    buyer_coordinator.add_asset_to_network(buyer_private_network, purchase_intent.clone(), IntegerMatrixPosition { x: 1, y: 2, z: 1 }).await.unwrap();
    println!("     - Purchase Intent Asset: {:?}", purchase_intent);

    // 4. Dealer validates the 'Purchase Intent'
    println!("\n  4. Dealer validates Buyer's 'Purchase Intent'...");
    dealer_coordinator.join_network(buyer_private_network, PrivacyTier::PrivateP2P).await.unwrap();
    let intent_proof = create_test_proof("buyer-node", "buyer-node");
    let intent_valid = dealer_coordinator.validate_asset_cross_network(
        purchase_intent.clone(),
        buyer_private_network,
        buyer_private_network,
        intent_proof,
    ).await.unwrap();
    assert!(intent_valid);
    println!("     - Dealer validated Purchase Intent.");

    // 5. Dealer creates 'Sales Agreement' on its federated network, sharing with Bank
    println!("\n  5. Dealer creates 'Sales Agreement' and shares with Bank...");
    let sales_agreement = AssetId::new(AssetType::Storage);
    dealer_coordinator.add_asset_to_network(dealer_network, sales_agreement.clone(), IntegerMatrixPosition { x: 2, y: 2, z: 2 }).await.unwrap();
    println!("     - Sales Agreement Asset: {:?}", sales_agreement);

    // 6. Bank validates 'Sales Agreement'
    println!("\n  6. Bank validates 'Sales Agreement'...");
    bank_coordinator.join_network(dealer_network, PrivacyTier::Federated).await.unwrap();
    let sales_agreement_proof = create_test_proof("dealer-node", "dealer-node");
    let sales_agreement_valid = bank_coordinator.validate_asset_cross_network(
        sales_agreement.clone(),
        dealer_network,
        dealer_network,
        sales_agreement_proof,
    ).await.unwrap();
    assert!(sales_agreement_valid);
    println!("     - Bank validated Sales Agreement.");

    // 7. Bank provides 'Proof of Financing'
    println!("\n  7. Bank provides 'Proof of Financing'...");
    let proof_of_financing = AssetId::new(AssetType::Storage);
    bank_coordinator.add_asset_to_network(bank_network, proof_of_financing.clone(), IntegerMatrixPosition { x: 3, y: 3, z: 3 }).await.unwrap();
    println!("     - Proof of Financing Asset: {:?}", proof_of_financing);

    // 8. Dealer validates 'Proof of Financing' and transfers title
    println!("\n  8. Dealer validates 'Proof of Financing' and transfers Title...");
    let financing_proof = create_test_proof("bank-node", "bank-node");
    dealer_coordinator.join_network(bank_network, PrivacyTier::Public).await.unwrap();
    let financing_valid = dealer_coordinator.validate_asset_cross_network(
        proof_of_financing.clone(),
        bank_network,
        bank_network,
        financing_proof,
    ).await.unwrap();
    assert!(financing_valid);
    println!("     - Dealer validated Proof of Financing.");

    let car_title = AssetId::new(AssetType::Storage);
    dealer_coordinator.add_asset_to_network(buyer_private_network, car_title.clone(), IntegerMatrixPosition { x: 1, y: 3, z: 1 }).await.unwrap();
    println!("     - Dealer transferred Car Title to Buyer's private chain: {:?}", car_title);

    // 9. Buyer gets insurance
    println!("\n  9. Buyer purchases insurance...");
    insurance_coordinator.join_network(buyer_private_network, PrivacyTier::PrivateP2P).await.unwrap();
    let title_proof_for_insurance = create_test_proof("buyer-node", "buyer-node");
    let title_valid_for_insurance = insurance_coordinator.validate_asset_cross_network(
        car_title.clone(),
        buyer_private_network,
        buyer_private_network,
        title_proof_for_insurance,
    ).await.unwrap();
    assert!(title_valid_for_insurance);
    println!("     - Insurance company validated Car Title.");

    // Buyer pays for insurance
    let insurance_payment = AssetId::new(AssetType::Storage);
    buyer_coordinator.add_asset_to_network(buyer_private_network, insurance_payment.clone(), IntegerMatrixPosition { x: 1, y: 4, z: 1 }).await.unwrap();
    println!("     - Buyer creates 'Payment for Insurance' asset: {:?}", insurance_payment);

    // Bank validates payment
    bank_coordinator.join_network(buyer_private_network, PrivacyTier::PrivateP2P).await.unwrap();
    let insurance_payment_proof = create_test_proof("buyer-node", "buyer-node");
    let insurance_payment_valid = bank_coordinator.validate_asset_cross_network(
        insurance_payment.clone(),
        buyer_private_network,
        buyer_private_network,
        insurance_payment_proof,
    ).await.unwrap();
    assert!(insurance_payment_valid);
    println!("     - Bank validated insurance payment.");

    let insurance_payment_confirmation = AssetId::new(AssetType::Storage);
    bank_coordinator.add_asset_to_network(buyer_private_network, insurance_payment_confirmation.clone(), IntegerMatrixPosition { x: 1, y: 4, z: 2 }).await.unwrap();
    println!("     - Bank issues 'Payment Confirmation' asset: {:?}", insurance_payment_confirmation);
    
    // Insurance company validates payment confirmation
    let insurance_payment_confirmation_proof = create_test_proof("bank-node", "bank-node");
    let insurance_payment_confirmation_valid = insurance_coordinator.validate_asset_cross_network(
        insurance_payment_confirmation.clone(),
        buyer_private_network,
        buyer_private_network,
        insurance_payment_confirmation_proof,
    ).await.unwrap();
    assert!(insurance_payment_confirmation_valid);
    println!("     - Insurance company validated payment confirmation.");

    let proof_of_insurance = AssetId::new(AssetType::Storage);
    insurance_coordinator.add_asset_to_network(buyer_private_network, proof_of_insurance.clone(), IntegerMatrixPosition { x: 1, y: 4, z: 3 }).await.unwrap();
    println!("     - Insurance company issued Proof of Insurance to Buyer's private chain: {:?}", proof_of_insurance);

    // 10. Buyer registers car with DMV
    println!("\n  10. Buyer registers car with DMV...");
    dmv_coordinator.join_network(buyer_private_network, PrivacyTier::PrivateP2P).await.unwrap();
    let title_proof_for_dmv = create_test_proof("buyer-node", "buyer-node");
    let title_valid_for_dmv = dmv_coordinator.validate_asset_cross_network(
        car_title.clone(),
        buyer_private_network,
        buyer_private_network,
        title_proof_for_dmv,
    ).await.unwrap();
    assert!(title_valid_for_dmv);
    println!("     - DMV validated Car Title.");
    
    let insurance_proof_for_dmv = create_test_proof("buyer-node", "buyer-node");
    let insurance_valid_for_dmv = dmv_coordinator.validate_asset_cross_network(
        proof_of_insurance.clone(),
        buyer_private_network,
        buyer_private_network,
        insurance_proof_for_dmv,
    ).await.unwrap();
    assert!(insurance_valid_for_dmv);
    println!("     - DMV validated Proof of Insurance.");

    // Buyer pays for registration
    let dmv_payment = AssetId::new(AssetType::Storage);
    buyer_coordinator.add_asset_to_network(buyer_private_network, dmv_payment.clone(), IntegerMatrixPosition { x: 1, y: 5, z: 1 }).await.unwrap();
    println!("     - Buyer creates 'Payment for Registration' asset: {:?}", dmv_payment);

    // Bank validates payment
    let dmv_payment_proof = create_test_proof("buyer-node", "buyer-node");
    let dmv_payment_valid = bank_coordinator.validate_asset_cross_network(
        dmv_payment.clone(),
        buyer_private_network,
        buyer_private_network,
        dmv_payment_proof,
    ).await.unwrap();
    assert!(dmv_payment_valid);
    println!("     - Bank validated registration payment.");

    let dmv_payment_confirmation = AssetId::new(AssetType::Storage);
    bank_coordinator.add_asset_to_network(buyer_private_network, dmv_payment_confirmation.clone(), IntegerMatrixPosition { x: 1, y: 5, z: 2 }).await.unwrap();
    println!("     - Bank issues 'Payment Confirmation' asset: {:?}", dmv_payment_confirmation);

    // DMV validates payment confirmation
    let dmv_payment_confirmation_proof = create_test_proof("bank-node", "bank-node");
    let dmv_payment_confirmation_valid = dmv_coordinator.validate_asset_cross_network(
        dmv_payment_confirmation.clone(),
        buyer_private_network,
        buyer_private_network,
        dmv_payment_confirmation_proof,
    ).await.unwrap();
    assert!(dmv_payment_confirmation_valid);
    println!("     - DMV validated payment confirmation.");

    let registered_title = AssetId::new(AssetType::Storage);
    dmv_coordinator.add_asset_to_network(buyer_private_network, registered_title.clone(), IntegerMatrixPosition { x: 1, y: 5, z: 3 }).await.unwrap();
    println!("     - DMV issued Registered Title to Buyer's private chain: {:?}", registered_title);


    println!("\n--- Buyer's Private Chain ---");
    println!("Purchase Offer: {:?}", purchase_intent);
    println!("Car Title: {:?}", car_title);
    println!("Proof of Insurance: {:?}", proof_of_insurance);
    println!("Registered Title: {:?}", registered_title);
    println!("--------------------------");

    println!("\n✅ Car purchase scenario completed successfully!");
    println!("   - A clear chain of dependencies was established between the entities.");
    println!("   - Each entity validated the proofs from the previous entity in the chain.");
}
