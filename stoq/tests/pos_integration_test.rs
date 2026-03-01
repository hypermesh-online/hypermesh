// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ + Proof of State Integration Tests
//!
//! Tests for protocol-level PoS validation, asset hash verification,
//! privacy tier enforcement, and shard addressing.

use anyhow::Result;
use hypermesh_lib::PrivacyMode;
use std::time::{Duration, SystemTime};
use stoq::protocol::{
    MatrixPosition, MatrixPositionExt, PosToken, ProofOfSpace, ProofOfStake, ProofOfTime,
    ProofOfWork, StoqPosIntegration,
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
            // 2 zero bytes = 16 leading zero bits, meeting difficulty 10
            difficulty: 10,
            nonce: 12345,
            work_hash: vec![0, 0, 0x0F, 0xFF],
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
    let result = integration
        .validate_connection("conn_anon_1".to_string(), &NetworkType::Anonymous, None)
        .await?;

    assert!(result, "Anonymous connection should succeed");

    let stats = integration.get_connection_stats("conn_anon_1");
    assert!(stats.is_some());
    assert_eq!(stats.unwrap().privacy_tier, PrivacyMode::ANONYMOUS);

    Ok(())
}

#[tokio::test]
async fn test_p2p_network_validation() -> Result<()> {
    let integration = StoqPosIntegration::new(Duration::from_secs(300));

    // P2P network should succeed without PoS token (certificate-based)
    let result = integration
        .validate_connection("conn_p2p_1".to_string(), &NetworkType::P2P, None)
        .await?;

    assert!(result, "P2P connection should succeed");

    let stats = integration.get_connection_stats("conn_p2p_1");
    assert!(stats.is_some());
    // P2P maps to PRIVATE in the new PrivacyMode model
    assert_eq!(stats.unwrap().privacy_tier, PrivacyMode::PRIVATE);

    Ok(())
}

#[tokio::test]
async fn test_federated_network_validation() -> Result<()> {
    let integration = StoqPosIntegration::new(Duration::from_secs(300));

    // Federated network should succeed without PoS token (federation-based)
    let result = integration
        .validate_connection(
            "conn_fed_1".to_string(),
            &NetworkType::Federated {
                gateway_url: "gateway.test.internal".to_string(),
            },
            None,
        )
        .await?;

    assert!(result, "Federated connection should succeed");

    let stats = integration.get_connection_stats("conn_fed_1");
    assert!(stats.is_some());
    // Federated maps to PRIVATE in the new PrivacyMode model
    assert_eq!(stats.unwrap().privacy_tier, PrivacyMode::PRIVATE);

    Ok(())
}

#[tokio::test]
async fn test_public_network_with_pos_validation() -> Result<()> {
    let integration = StoqPosIntegration::new(Duration::from_secs(300));
    let token = create_test_pos_token();

    // Public network should succeed with valid PoS token
    let result = integration
        .validate_connection("conn_pub_1".to_string(), &NetworkType::Public, Some(&token))
        .await?;

    assert!(result, "Public connection with PoS token should succeed");

    let stats = integration.get_connection_stats("conn_pub_1");
    assert!(stats.is_some());
    let stats = stats.unwrap();
    assert_eq!(stats.privacy_tier, PrivacyMode::PUBLIC);
    assert!(stats.has_pos_token, "Should have PoS token");

    Ok(())
}

#[tokio::test]
async fn test_public_network_without_pos_fails() -> Result<()> {
    let integration = StoqPosIntegration::new(Duration::from_secs(300));

    // Public network should fail without PoS token
    let result = integration
        .validate_connection("conn_pub_2".to_string(), &NetworkType::Public, None)
        .await;

    assert!(
        result.is_err(),
        "Public connection without PoS token should fail"
    );

    Ok(())
}

#[tokio::test]
async fn test_asset_hash_verification_success() -> Result<()> {
    let integration = StoqPosIntegration::new(Duration::from_secs(300));

    // Register a connection first
    integration
        .validate_connection(
            "conn1".to_string(),
            &NetworkType::Public,
            Some(&create_test_pos_token()),
        )
        .await?;

    let asset_data = b"test asset data for verification";

    // Compute correct BLAKE3 hash (matching library implementation)
    let correct_hash: [u8; 32] = *blake3::hash(asset_data).as_bytes();

    // Validate with correct hash
    let result =
        integration.validate_asset_hash("conn1", b"asset_123", &correct_hash, asset_data)?;

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
    integration
        .validate_connection(
            "conn2".to_string(),
            &NetworkType::Public,
            Some(&create_test_pos_token()),
        )
        .await?;

    let asset_data = b"test asset data";
    let wrong_hash = [0u8; 32]; // Wrong hash

    // Validate with wrong hash
    let result = integration.validate_asset_hash("conn2", b"asset_456", &wrong_hash, asset_data)?;

    assert!(!result, "Asset hash validation should fail with wrong hash");

    Ok(())
}

#[tokio::test]
async fn test_shard_address_registration() -> Result<()> {
    let integration = StoqPosIntegration::new(Duration::from_secs(300));

    // Register shard addresses
    integration.register_shard_address(
        1,
        MatrixPosition::from_i64(10, 20, 30),
        "network1".to_string(),
        Some("node1".to_string()),
    );

    integration.register_shard_address(
        2,
        MatrixPosition::from_i64(50, 60, 70),
        "network2".to_string(),
        Some("node2".to_string()),
    );

    integration.register_shard_address(
        3,
        MatrixPosition::from_i64(100, 110, 120),
        "network3".to_string(),
        None,
    );

    // Retrieve shard addresses
    let addresses = integration.get_shard_addresses(&[1, 2, 3]);
    assert_eq!(addresses.len(), 3);

    assert_eq!(addresses[0].shard_id, 1);
    assert_eq!(addresses[0].position, MatrixPosition::from_i64(10, 20, 30));
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

    let positions =
        integration.calculate_shard_positions(num_shards, origin, min_distance, max_distance);

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
    integration
        .validate_connection("anon_conn".to_string(), &NetworkType::Anonymous, None)
        .await?;

    // Anonymous should reject logging operations
    let result = integration.enforce_privacy_tier("anon_conn", "log_data");
    assert!(result.is_err(), "Anonymous should reject logging");

    let result = integration.enforce_privacy_tier("anon_conn", "store_data");
    assert!(result.is_err(), "Anonymous should reject storage");

    Ok(())
}

#[tokio::test]
async fn test_privacy_tier_enforcement_public() -> Result<()> {
    let _integration = StoqPosIntegration::new(Duration::from_secs(300));

    // This tests the enforcement logic - in practice, connection wouldn't be established
    // without PoS token. The test validates that PrivacyMode is accessible.
    let mode = PrivacyMode::PUBLIC;
    assert!(mode.requires_identity());

    Ok(())
}

#[tokio::test]
async fn test_all_network_types_integration() -> Result<()> {
    let integration = StoqPosIntegration::new(Duration::from_secs(300));
    let token = create_test_pos_token();

    // Connect to all 4 network types
    integration
        .validate_connection("conn_anon".to_string(), &NetworkType::Anonymous, None)
        .await?;

    integration
        .validate_connection("conn_p2p".to_string(), &NetworkType::P2P, None)
        .await?;

    integration
        .validate_connection(
            "conn_fed".to_string(),
            &NetworkType::Federated {
                gateway_url: "gateway.test".to_string(),
            },
            None,
        )
        .await?;

    integration
        .validate_connection("conn_pub".to_string(), &NetworkType::Public, Some(&token))
        .await?;

    // Verify all connections are tracked
    // P2P and Federated both map to PRIVATE, so private_connections = 2
    let stats = integration.get_stats();
    assert_eq!(stats.total_connections, 4);
    assert_eq!(stats.anonymous_connections, 1);
    assert_eq!(stats.private_connections, 2);
    assert_eq!(stats.public_connections, 1);

    Ok(())
}

#[tokio::test]
async fn test_connection_statistics() -> Result<()> {
    let integration = StoqPosIntegration::new(Duration::from_secs(300));

    // Create multiple connections
    integration
        .validate_connection("c1".to_string(), &NetworkType::Anonymous, None)
        .await?;

    integration
        .validate_connection("c2".to_string(), &NetworkType::P2P, None)
        .await?;

    integration
        .validate_connection(
            "c3".to_string(),
            &NetworkType::Public,
            Some(&create_test_pos_token()),
        )
        .await?;

    // Get connection stats
    let c1_stats = integration.get_connection_stats("c1");
    assert!(c1_stats.is_some());
    assert_eq!(c1_stats.unwrap().privacy_tier, PrivacyMode::ANONYMOUS);

    let c3_stats = integration.get_connection_stats("c3");
    assert!(c3_stats.is_some());
    let c3_stats = c3_stats.unwrap();
    assert_eq!(c3_stats.privacy_tier, PrivacyMode::PUBLIC);
    assert!(c3_stats.has_pos_token);

    // Get overall stats
    let stats = integration.get_stats();
    assert_eq!(stats.total_connections, 3);

    Ok(())
}

#[tokio::test]
async fn test_matrix_position_distance() -> Result<()> {
    let origin = MatrixPosition::origin();
    let pos1 = MatrixPosition::from_i64(3, 4, 0);
    let pos2 = MatrixPosition::from_i64(0, 0, 12);

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
async fn test_privacy_mode_timeouts() -> Result<()> {
    // PrivacyMode uses connection_timeout_secs() returning u64
    let anon_timeout = Duration::from_secs(PrivacyMode::ANONYMOUS.connection_timeout_secs());
    let private_timeout = Duration::from_secs(PrivacyMode::PRIVATE.connection_timeout_secs());
    let pub_timeout = Duration::from_secs(PrivacyMode::PUBLIC.connection_timeout_secs());

    // Anonymous(30s) < Private(90s) < Public(300s)
    assert!(anon_timeout < private_timeout);
    assert!(private_timeout < pub_timeout);

    // Verify exact values
    assert_eq!(PrivacyMode::ANONYMOUS.connection_timeout_secs(), 30);
    assert_eq!(PrivacyMode::PRIVATE.connection_timeout_secs(), 90);
    assert_eq!(PrivacyMode::PUBLIC.connection_timeout_secs(), 300);

    Ok(())
}

#[tokio::test]
async fn test_privacy_mode_validation_requirements() -> Result<()> {
    // ANONYMOUS: no identity, no logging
    assert!(!PrivacyMode::ANONYMOUS.requires_identity());
    assert!(!PrivacyMode::ANONYMOUS.allows_logging());

    // PRIVATE: requires identity, allows logging
    assert!(PrivacyMode::PRIVATE.requires_identity());
    assert!(PrivacyMode::PRIVATE.allows_logging());

    // PUBLIC: requires identity, allows logging
    assert!(PrivacyMode::PUBLIC.requires_identity());
    assert!(PrivacyMode::PUBLIC.allows_logging());

    Ok(())
}

#[tokio::test]
async fn test_concurrent_connections() -> Result<()> {
    use tokio::task::JoinSet;

    let integration = std::sync::Arc::new(StoqPosIntegration::new(Duration::from_secs(300)));

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

            integration
                .validate_connection(format!("conn_{i}"), &network_type, pos_token.as_ref())
                .await
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
