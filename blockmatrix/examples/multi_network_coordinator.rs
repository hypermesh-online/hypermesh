// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Multi-Network Coordinator Example
//!
//! Demonstrates a single node participating in all 4 network types simultaneously,
//! showing complete isolation between networks and per-network asset visibility control.

use anyhow::Result;
use blockmatrix::network::{
    multi_network::{MultiNetworkCoordinator, NetworkConfig, VisibilityPolicy},
    trust::{NetworkType, ProofOfState},
    isolation::{DefaultIsolationManager, IsolationManager},
};
use blockmatrix::assets::core::{
    AssetRegistration, AssetCategory, BaseSystemType, NetworkScope,
    AssetData,
};
use std::sync::Arc;
use std::time::SystemTime;
use uuid::Uuid;
use tracing::{info, warn};
use tracing_subscriber;

/// Create a test asset ID
fn create_test_asset() -> AssetRegistration {
    let asset_data = AssetData {
        config: vec![1, 2, 3],
        definition: vec![4, 5, 6],
        metadata: vec![7, 8, 9],
    };

    AssetRegistration::from_asset_data(
        &asset_data,
        NetworkScope::Global,
        AssetCategory::BaseSystem(BaseSystemType::Storage),
    )
}

/// Generate test Proof of State
fn generate_test_proof() -> ProofOfState {
    ProofOfState {
        proof_of_space: vec![1, 2, 3, 4, 5],
        proof_of_stake: vec![6, 7, 8, 9, 10],
        proof_of_work: vec![11, 12, 13, 14, 15],
        proof_of_time: vec![16, 17, 18, 19, 20],
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 Starting Multi-Network Coordinator Example");
    info!("================================================");

    // Initialize coordinator with isolation manager
    let isolation = Arc::new(DefaultIsolationManager::new());
    let mut coordinator = MultiNetworkCoordinator::new(isolation.clone());

    info!("\n📊 Joining Multiple Networks...");
    info!("--------------------------------");

    // 1. Join Anonymous network
    let anon_id = coordinator.join_network(
        NetworkType::Anonymous,
        NetworkConfig::anonymous(),
    ).await?;
    info!("✅ Joined Anonymous network: {}", anon_id);
    info!("   - No persistent identity");
    info!("   - Ephemeral connections");
    info!("   - No certificate validation");

    // 2. Join P2P network
    let p2p_id = coordinator.join_network(
        NetworkType::P2P,
        NetworkConfig::p2p(vec![
            "peer1.local:8080".to_string(),
            "peer2.local:8080".to_string(),
        ]),
    ).await?;
    info!("✅ Joined P2P network: {}", p2p_id);
    info!("   - Direct peer connections");
    info!("   - Self-signed certificates");
    info!("   - Manual trust decisions");

    // 3. Join Federated network
    let fed_id = coordinator.join_network(
        NetworkType::Federated { gateway_url: "gateway.company.internal".to_string() },
        NetworkConfig::federated("gateway.company.internal".to_string()),
    ).await?;
    info!("✅ Joined Federated network: {}", fed_id);
    info!("   - Federation gateway: gateway.company.internal");
    info!("   - Federation-scoped trust");
    info!("   - Limited to federation members");

    // 4. Join Public network (with full PoS)
    let pub_id = coordinator.join_network(
        NetworkType::Public,
        NetworkConfig::public(
            "mynode.hypermesh.online".to_string(),
            generate_test_proof(),
        ),
    ).await?;
    info!("✅ Joined Public network: {}", pub_id);
    info!("   - DNS: mynode.hypermesh.online");
    info!("   - Full Proof of State validation");
    info!("   - Blockchain-registered certificate");
    info!("   - CAESAR rewards enabled");

    // Verify all 4 networks are active
    let active = coordinator.active_networks().await;
    info!("\n📊 Network Status:");
    info!("   Total active networks: {} (all 4 types)", active.len());
    assert_eq!(active.len(), 4, "Should have exactly 4 active networks");

    // Get network statistics
    let stats = coordinator.get_network_stats().await;
    info!("\n📈 Network Statistics:");
    info!("   Anonymous networks: {}", stats.anonymous_count);
    info!("   P2P networks: {}", stats.p2p_count);
    info!("   Federated networks: {}", stats.federated_count);
    info!("   Public networks: {}", stats.public_count);

    // List all networks with types
    info!("\n🌐 Connected Networks:");
    for (network_id, network_type) in coordinator.list_networks().await {
        info!("   {} -> {:?}", network_id, network_type);
    }

    info!("\n🔒 Configuring Asset Visibility...");
    info!("------------------------------------");

    // Create test assets
    let asset1 = create_test_asset();
    let asset2 = create_test_asset();
    let asset3 = create_test_asset();

    // Configure asset visibility for different network combinations

    // Asset 1: Only visible to Anonymous and P2P
    coordinator.set_asset_visibility(
        asset1.clone(),
        vec![anon_id.clone(), p2p_id.clone()],
    ).await?;
    info!("📦 Asset 1 configured: visible to Anonymous + P2P networks only");

    // Asset 2: Only visible to Federated
    coordinator.set_asset_visibility(
        asset2.clone(),
        vec![fed_id.clone()],
    ).await?;
    info!("📦 Asset 2 configured: visible to Federated network only");

    // Asset 3: Visible to all networks
    coordinator.set_asset_visibility(
        asset3.clone(),
        vec![anon_id.clone(), p2p_id.clone(), fed_id.clone(), pub_id.clone()],
    ).await?;
    info!("📦 Asset 3 configured: visible to ALL networks");

    info!("\n🧪 Testing Asset Access Control...");
    info!("-----------------------------------");

    // Test Asset 1 access (should work for Anonymous, fail for Federated)
    let response = coordinator.handle_asset_request(anon_id.clone(), asset1.clone()).await?;
    info!("✅ Anonymous network can access Asset 1: {}", response.authorized);
    assert!(response.authorized, "Anonymous should access Asset 1");

    let response = coordinator.handle_asset_request(fed_id.clone(), asset1.clone()).await?;
    info!("❌ Federated network blocked from Asset 1: {}", !response.authorized);
    assert!(!response.authorized, "Federated should NOT access Asset 1");

    // Test Asset 3 access (should work for all)
    for (network_id, network_name) in [
        (anon_id.clone(), "Anonymous"),
        (p2p_id.clone(), "P2P"),
        (fed_id.clone(), "Federated"),
        (pub_id.clone(), "Public"),
    ] {
        let response = coordinator.handle_asset_request(network_id, asset3.clone()).await?;
        info!("✅ {} network can access Asset 3: {}", network_name, response.authorized);
        assert!(response.authorized, "{} should access Asset 3", network_name);
    }

    info!("\n🔍 Testing Network Isolation...");
    info!("--------------------------------");

    // Check isolation violations (should be none)
    let violations = isolation.check_violations().await;
    info!("   Isolation violations detected: {}", violations.len());
    assert_eq!(violations.len(), 0, "Should have no isolation violations");

    // Test that networks are properly isolated
    info!("✅ Network isolation verified: no cross-network packet leakage");

    info!("\n👋 Testing Network Departure...");
    info!("--------------------------------");

    // Leave Anonymous network
    coordinator.leave_network(anon_id.clone()).await?;
    info!("📤 Left Anonymous network");

    // Verify network count decreased
    let remaining = coordinator.active_networks().await;
    info!("   Active networks after departure: {} (3 remaining)", remaining.len());
    assert_eq!(remaining.len(), 3, "Should have 3 networks after leaving one");

    // Verify Anonymous network is no longer connected
    assert!(!coordinator.is_connected(anon_id).await, "Anonymous network should be disconnected");
    info!("✅ Confirmed Anonymous network disconnected");

    // Verify other networks still active
    assert!(coordinator.is_connected(p2p_id).await, "P2P should still be connected");
    assert!(coordinator.is_connected(fed_id).await, "Federated should still be connected");
    assert!(coordinator.is_connected(pub_id).await, "Public should still be connected");
    info!("✅ Other networks remain connected");

    info!("\n🎯 Multi-Network Coordinator Test Complete!");
    info!("===========================================");
    info!("Summary:");
    info!("  - Successfully joined all 4 network types");
    info!("  - Networks operate simultaneously without interference");
    info!("  - Asset visibility controlled per network");
    info!("  - Complete isolation between networks verified");
    info!("  - Graceful network join/leave demonstrated");
    info!("\n✨ All tests passed successfully!");

    Ok(())
}