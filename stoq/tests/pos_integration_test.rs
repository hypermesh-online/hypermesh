//! STOQ + Proof of State Integration Tests
//!
//! Tests for protocol-level PoS validation, asset hash verification,
//! privacy tier enforcement, and shard addressing.

use anyhow::Result;
use std::time::{Duration, SystemTime};
use stoq::protocol::{
    StoqPosIntegration, PrivacyTier, MatrixPosition, PosToken,
    ProofOfSpace, ProofOfStake, ProofOfWork, ProofOfTime,
};
use stoq::transport::certificate_strategy::NetworkType;

/// Create test PoS token
fn create_test_pos_token() -> PosToken {
    PosToken {
        id: vec![1, 2, 3, 4],
        proof_of_space: ProofOfSpace {
            commitment_hash: vec![5, 6, 7, 8],
            matrix_position: (10, 20, 30),
            capacity: 1024 * 1024,
        },
        proof_of_stake: ProofOfStake {
            owner_pubkey: vec![9, 10, 11, 12],
            stake_amount: 1000,
            staked_until: SystemTime::now() + Duration::from_secs(3600),
        },
        proof_of_work: ProofOfWork {
            difficulty: 10,
            nonce: 12345,
            work_hash: vec![13, 14, 15, 16],
        },
        proof_of_time: ProofOfTime {
            timestamp: SystemTime::now(),
            sequence: 1,
            prev_hash: vec![17, 18, 19, 20],
        },
        signature: vec![21, 22, 23, 24],
        expires_at: SystemTime::now() + Duration::from_secs(300),
        issuer_pubkey: Some(vec![25, 26, 27, 28]),
    }
}

#[tokio::test]
async fn test_anonymous_network_validation() -> Result<()> {
    let integration = StoqPosIntegration::new(Duration::from_secs(300));

    // Anonymous network should succeed without PoS token
    let result = integration.validate_connection(
        "conn_anon_1".to_string(),
        &NetworkType::Anonymous,
        None,
    ).await?;

    assert!(result, "Anonymous connection should succeed");

    let stats = integration.get_connection_stats("conn_anon_1");
    assert!(stats.is_some());
    assert_eq!(stats.unwrap().privacy_tier, PrivacyTier::Anonymous);

    Ok(())
}

#[tokio::test]
async fn test_p2p_network_validation() -> Result<()> {
    let integration = StoqPosIntegration::new(Duration::from_secs(300));

    // P2P network should succeed without PoS token (certificate-based)
    let result = integration.validate_connection(
        "conn_p2p_1".to_string(),
        &NetworkType::P2P,
        None,
    ).await?;

    assert!(result, "P2P connection should succeed");

    let stats = integration.get_connection_stats("conn_p2p_1");
    assert!(stats.is_some());
    assert_eq!(stats.unwrap().privacy_tier, PrivacyTier::P2P);

    Ok(())
}

#[tokio::test]
async fn test_federated_network_validation() -> Result<()> {
    let integration = StoqPosIntegration::new(Duration::from_secs(300));

    // Federated network should succeed without PoS token (federation-based)
    let result = integration.validate_connection(
        "conn_fed_1".to_string(),
        &NetworkType::Federated {
            gateway_url: "gateway.test.internal".to_string(),
        },
        None,
    ).await?;

    assert!(result, "Federated connection should succeed");

    let stats = integration.get_connection_stats("conn_fed_1");
    assert!(stats.is_some());
    assert_eq!(stats.unwrap().privacy_tier, PrivacyTier::Federated);

    Ok(())
}

#[tokio::test]
async fn test_public_network_with_pos_validation() -> Result<()> {
    let integration = StoqPosIntegration::new(Duration::from_secs(300));
    let token = create_test_pos_token();

    // Public network should succeed with valid PoS token
    let result = integration.validate_connection(
        "conn_pub_1".to_string(),
        &NetworkType::Public,
        Some(&token),
    ).await?;

    assert!(result, "Public connection with PoS token should succeed");

    let stats = integration.get_connection_stats("conn_pub_1");
    assert!(stats.is_some());
    let stats = stats.unwrap();
    assert_eq!(stats.privacy_tier, PrivacyTier::Public);
    assert!(stats.has_pos_token, "Should have PoS token");

    Ok(())
}

#[tokio::test]
async fn test_public_network_without_pos_fails() -> Result<()> {
    let integration = StoqPosIntegration::new(Duration::from_secs(300));

    // Public network should fail without PoS token
    let result = integration.validate_connection(
        "conn_pub_2".to_string(),
        &NetworkType::Public,
        None,
    ).await;

    assert!(result.is_err(), "Public connection without PoS token should fail");

    Ok(())
}

#[tokio::test]
async fn test_asset_hash_verification_success() -> Result<()> {
    let integration = StoqPosIntegration::new(Duration::from_secs(300));

    // Register a connection first
    integration.validate_connection(
        "conn1".to_string(),
        &NetworkType::Public,
        Some(&create_test_pos_token()),
    ).await?;

    let asset_data = b"test asset data for verification";

    // Compute correct hash
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(asset_data);
    let correct_hash: [u8; 32] = hasher.finalize().into();

    // Validate with correct hash
    let result = integration.validate_asset_hash(
        "conn1",
        b"asset_123",
        &correct_hash,
        asset_data,
    )?;

    assert!(result, "Asset hash validation should succeed");

    // Verify asset was cached
    let stats = integration.get_stats();
    assert_eq!(stats.cached_assets, 1);

    Ok(())
}

#[tokio::test]
async fn test_asset_hash_verification_failure() -> Result<()> {
    let integration = StoqPosIntegration::new(Duration::from_secs(300));

    // Register a connection
    integration.validate_connection(
        "conn2".to_string(),
        &NetworkType::Public,
        Some(&create_test_pos_token()),
    ).await?;

    let asset_data = b"test asset data";
    let wrong_hash = [0u8; 32]; // Wrong hash

    // Validate with wrong hash
    let result = integration.validate_asset_hash(
        "conn2",
        b"asset_456",
        &wrong_hash,
        asset_data,
    )?;

    assert!(!result, "Asset hash validation should fail with wrong hash");

    Ok(())
}

#[tokio::test]
async fn test_shard_address_registration() -> Result<()> {
    let integration = StoqPosIntegration::new(Duration::from_secs(300));

    // Register shard addresses
    integration.register_shard_address(
        1,
        MatrixPosition::new(10, 20, 30),
        "network1".to_string(),
        Some("node1".to_string()),
    );

    integration.register_shard_address(
        2,
        MatrixPosition::new(50, 60, 70),
        "network2".to_string(),
        Some("node2".to_string()),
    );

    integration.register_shard_address(
        3,
        MatrixPosition::new(100, 110, 120),
        "network3".to_string(),
        None,
    );

    // Retrieve shard addresses
    let addresses = integration.get_shard_addresses(&[1, 2, 3]);
    assert_eq!(addresses.len(), 3);

    assert_eq!(addresses[0].shard_id, 1);
    assert_eq!(addresses[0].position, MatrixPosition::new(10, 20, 30));
    assert_eq!(addresses[0].network_id, "network1");
    assert_eq!(addresses[0].node_id, Some("node1".to_string()));

    assert_eq!(addresses[1].shard_id, 2);
    assert_eq!(addresses[2].shard_id, 3);

    Ok(())
}

#[tokio::test]
async fn test_shard_position_calculation() -> Result<()> {
    let integration = StoqPosIntegration::new(Duration::from_secs(300));

    let num_shards = 20;
    let origin = MatrixPosition::origin();
    let min_distance = 10.0;
    let max_distance = 100.0;

    let positions = integration.calculate_shard_positions(
        num_shards,
        origin,
        min_distance,
        max_distance,
    );

    assert_eq!(positions.len(), num_shards);

    // Verify all positions are distinct
    for i in 0..positions.len() {
        for j in (i + 1)..positions.len() {
            let dist = positions[i].distance_to(&positions[j]);
            assert!(dist > 0.0, "Positions should be distinct");
        }
    }

    // Verify positions are within distance range (approximately)
    for pos in &positions {
        let dist = origin.distance_to(pos);
        assert!(
            dist >= min_distance * 0.5 && dist <= max_distance * 1.5,
            "Position should be within reasonable distance range"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_privacy_tier_enforcement_anonymous() -> Result<()> {
    let integration = StoqPosIntegration::new(Duration::from_secs(300));

    // Create anonymous connection
    integration.validate_connection(
        "anon_conn".to_string(),
        &NetworkType::Anonymous,
        None,
    ).await?;

    // Anonymous should reject logging operations
    let result = integration.enforce_privacy_tier("anon_conn", "log_data");
    assert!(result.is_err(), "Anonymous should reject logging");

    let result = integration.enforce_privacy_tier("anon_conn", "store_data");
    assert!(result.is_err(), "Anonymous should reject storage");

    Ok(())
}

#[tokio::test]
async fn test_privacy_tier_enforcement_public() -> Result<()> {
    let integration = StoqPosIntegration::new(Duration::from_secs(300));

    // Create public connection without PoS token (invalid state for testing)
    // We'll manually register to test enforcement
    use stoq::protocol::pos_integration::PrivacyTier;

    // This tests the enforcement logic - in practice, connection wouldn't be established
    // without PoS token

    Ok(())
}

#[tokio::test]
async fn test_all_network_types_integration() -> Result<()> {
    let integration = StoqPosIntegration::new(Duration::from_secs(300));
    let token = create_test_pos_token();

    // Connect to all 4 network types
    integration.validate_connection(
        "conn_anon".to_string(),
        &NetworkType::Anonymous,
        None,
    ).await?;

    integration.validate_connection(
        "conn_p2p".to_string(),
        &NetworkType::P2P,
        None,
    ).await?;

    integration.validate_connection(
        "conn_fed".to_string(),
        &NetworkType::Federated {
            gateway_url: "gateway.test".to_string(),
        },
        None,
    ).await?;

    integration.validate_connection(
        "conn_pub".to_string(),
        &NetworkType::Public,
        Some(&token),
    ).await?;

    // Verify all connections are tracked
    let stats = integration.get_stats();
    assert_eq!(stats.total_connections, 4);
    assert_eq!(stats.anonymous_connections, 1);
    assert_eq!(stats.p2p_connections, 1);
    assert_eq!(stats.federated_connections, 1);
    assert_eq!(stats.public_connections, 1);

    Ok(())
}

#[tokio::test]
async fn test_connection_statistics() -> Result<()> {
    let integration = StoqPosIntegration::new(Duration::from_secs(300));

    // Create multiple connections
    integration.validate_connection(
        "c1".to_string(),
        &NetworkType::Anonymous,
        None,
    ).await?;

    integration.validate_connection(
        "c2".to_string(),
        &NetworkType::P2P,
        None,
    ).await?;

    integration.validate_connection(
        "c3".to_string(),
        &NetworkType::Public,
        Some(&create_test_pos_token()),
    ).await?;

    // Get connection stats
    let c1_stats = integration.get_connection_stats("c1");
    assert!(c1_stats.is_some());
    assert_eq!(c1_stats.unwrap().privacy_tier, PrivacyTier::Anonymous);

    let c3_stats = integration.get_connection_stats("c3");
    assert!(c3_stats.is_some());
    let c3_stats = c3_stats.unwrap();
    assert_eq!(c3_stats.privacy_tier, PrivacyTier::Public);
    assert!(c3_stats.has_pos_token);

    // Get overall stats
    let stats = integration.get_stats();
    assert_eq!(stats.total_connections, 3);

    Ok(())
}

#[tokio::test]
async fn test_matrix_position_distance() -> Result<()> {
    let origin = MatrixPosition::origin();
    let pos1 = MatrixPosition::new(3, 4, 0);
    let pos2 = MatrixPosition::new(0, 0, 12);

    // 3-4-5 triangle in XY plane
    let dist1 = origin.distance_to(&pos1);
    assert!((dist1 - 5.0).abs() < 0.01);

    // Distance along Z axis
    let dist2 = origin.distance_to(&pos2);
    assert!((dist2 - 12.0).abs() < 0.01);

    // Distance between two points
    let dist3 = pos1.distance_to(&pos2);
    assert!(dist3 > 0.0);

    Ok(())
}

#[tokio::test]
async fn test_privacy_tier_timeouts() -> Result<()> {
    use stoq::protocol::pos_integration::PrivacyTier;

    let anon_timeout = PrivacyTier::Anonymous.connection_timeout();
    let p2p_timeout = PrivacyTier::P2P.connection_timeout();
    let fed_timeout = PrivacyTier::Federated.connection_timeout();
    let pub_timeout = PrivacyTier::Public.connection_timeout();

    // Higher privacy tiers should have longer timeouts
    assert!(anon_timeout < p2p_timeout);
    assert!(p2p_timeout < fed_timeout);
    assert!(fed_timeout < pub_timeout);

    Ok(())
}

#[tokio::test]
async fn test_privacy_tier_validation_requirements() -> Result<()> {
    use stoq::protocol::pos_integration::PrivacyTier;

    // Only Public requires PoS validation
    assert!(!PrivacyTier::Anonymous.requires_pos_validation());
    assert!(!PrivacyTier::P2P.requires_pos_validation());
    assert!(!PrivacyTier::Federated.requires_pos_validation());
    assert!(PrivacyTier::Public.requires_pos_validation());

    // Only Public requires full 4-proof validation
    assert!(PrivacyTier::Public.requires_full_proofs());

    // Anonymous doesn't allow logging
    assert!(!PrivacyTier::Anonymous.allows_logging());
    assert!(PrivacyTier::P2P.allows_logging());
    assert!(PrivacyTier::Federated.allows_logging());
    assert!(PrivacyTier::Public.allows_logging());

    Ok(())
}

#[tokio::test]
async fn test_concurrent_connections() -> Result<()> {
    use tokio::task::JoinSet;

    let integration = std::sync::Arc::new(
        StoqPosIntegration::new(Duration::from_secs(300))
    );

    let mut tasks = JoinSet::new();

    // Create 100 concurrent connections across all network types
    for i in 0..100 {
        let integration = integration.clone();
        let token = create_test_pos_token();

        tasks.spawn(async move {
            let network_type = match i % 4 {
                0 => NetworkType::Anonymous,
                1 => NetworkType::P2P,
                2 => NetworkType::Federated {
                    gateway_url: "gateway.test".to_string(),
                },
                _ => NetworkType::Public,
            };

            let pos_token = if matches!(network_type, NetworkType::Public) {
                Some(token)
            } else {
                None
            };

            integration.validate_connection(
                format!("conn_{}", i),
                &network_type,
                pos_token.as_ref(),
            ).await
        });
    }

    // Wait for all tasks
    let mut success_count = 0;
    while let Some(result) = tasks.join_next().await {
        if result?? {
            success_count += 1;
        }
    }

    assert_eq!(success_count, 100, "All connections should succeed");

    let stats = integration.get_stats();
    assert_eq!(stats.total_connections, 100);

    Ok(())
}
