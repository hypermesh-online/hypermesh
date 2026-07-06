// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Integration tests for Matrix-STOQ communication

use anyhow::Result;
use blockmatrix::{
    bootstrap::PrivacyMode,
    matrix::coordinate::MatrixCoordinate,
    network::{stoq_integration::MatrixStoqIntegration, NetworkManager},
};
use std::sync::Arc;
use stoq::{StoqTransport, TransportConfig};

#[tokio::test]
async fn test_matrix_stoq_initialization() -> Result<()> {
    // Create a matrix coordinate
    let coordinate = MatrixCoordinate::new(10, 20, 30)?;

    // Create STOQ transport with test configuration
    let config = TransportConfig {
        port: 0,
        ..TransportConfig::default()
    }; // Use dynamic port for testing
    let transport = Arc::new(StoqTransport::new(config).await?);

    // Create Matrix-STOQ integration
    let integration = MatrixStoqIntegration::new(
        coordinate,
        "test_node_001".to_string(),
        transport.clone(),
        PrivacyMode::PUBLIC,
    )
    .await?;

    // Verify integration was created successfully
    assert_eq!(integration.get_connected_nodes().await.len(), 0);

    Ok(())
}

// F2: `test_matrix_node_communication` and `test_matrix_neighbor_discovery`
// were removed. They exercised the legacy `MatrixStoqIntegration` handshake
// (`connect_to_node` / `handle_incoming_connection`) which exchanged RAW,
// UNSIGNED PoS tokens and accepted them after only a structural `.validate()`
// — the F2 Sybil vector. That path is deleted; the production handshake goes
// through `NetworkManager` (bilateral FALCON-signed PoS). See
// `blockmatrix/tests/bilateral_e2e.rs` for the current handshake coverage.

#[tokio::test]
async fn test_matrix_position_broadcast() -> Result<()> {
    // Create a matrix node
    let coordinate = MatrixCoordinate::new(5, 10, 15)?;

    // Create STOQ transport
    let config = TransportConfig {
        port: 0,
        ..TransportConfig::default()
    };
    let transport = Arc::new(StoqTransport::new(config).await?);

    // Create Matrix-STOQ integration
    let integration = MatrixStoqIntegration::new(
        coordinate,
        "broadcast_node".to_string(),
        transport.clone(),
        PrivacyMode::PUBLIC,
    )
    .await?;

    // Test position broadcast (should succeed even with no connections)
    integration.broadcast_position().await?;

    // Test heartbeat (should succeed even with no connections)
    integration.send_heartbeat().await?;

    Ok(())
}

#[tokio::test]
async fn test_network_manager_with_stoq_integration() -> Result<()> {
    // Create a matrix coordinate
    let coordinate = MatrixCoordinate::new(100, 200, 300)?;

    // Create STOQ transport
    let config = TransportConfig {
        port: 0,
        ..TransportConfig::default()
    };
    let transport = Arc::new(StoqTransport::new(config).await?);

    // Create network manager with STOQ integration and test identity
    let test_identity = blockmatrix::identity::FalconIdentity::generate();
    let signer: Arc<dyn hypermesh_lib::NodeSigner> = Arc::new(test_identity);
    let proof_provider: Arc<dyn hypermesh_lib::StateProofProvider> = Arc::new(
        blockmatrix::proof_of_state::BlockMatrixProofProvider::new(signer.node_id().to_string(), signer.clone()),
    );
    let manager = NetworkManager::new(
        coordinate,
        transport,
        PrivacyMode::PUBLIC,
        vec![], // No bootstrap nodes for test
        signer,
        proof_provider,
        "test-network".to_string(),
    )
    .await?;

    // Test broadcasting matrix position through the manager
    manager.broadcast_matrix_position().await?;

    // Test discovering neighbors through STOQ
    let neighbors = manager.discover_matrix_neighbors_stoq(100.0, 10).await?;
    assert_eq!(neighbors.len(), 0); // No neighbors in isolated test

    Ok(())
}

#[tokio::test]
async fn test_matrix_coordinate_serialization() -> Result<()> {
    use blockmatrix::network::stoq_integration::{MatrixMessage, MatrixNodeAnnouncement};

    // Create a matrix coordinate
    let coordinate = MatrixCoordinate::new(42, 84, 126)?;

    // Create an announcement message
    let announcement = MatrixNodeAnnouncement {
        coordinate,
        node_id: "test_serialization".to_string(),
        privacy_mode: "Public".to_string(),
        protocol_version: "1.0.0".to_string(),
        pos_token: Some(vec![1, 2, 3, 4]),
        services: vec!["matrix".to_string(), "storage".to_string()],
    };

    let message = MatrixMessage::Announcement(announcement);

    // Serialize to JSON
    let json = serde_json::to_string(&message)?;

    // Deserialize back
    let deserialized: MatrixMessage = serde_json::from_str(&json)?;

    // Verify
    match deserialized {
        MatrixMessage::Announcement(ann) => {
            assert_eq!(ann.coordinate, coordinate);
            assert_eq!(ann.node_id, "test_serialization");
            assert_eq!(ann.privacy_mode, "Public");
            assert_eq!(ann.services.len(), 2);
        }
        _ => panic!("Wrong message type after deserialization"),
    }

    Ok(())
}
