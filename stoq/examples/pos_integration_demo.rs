// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ + Proof of State Integration Demo
//!
//! Demonstrates protocol-level PoS validation, asset hash verification,
//! privacy mode enforcement, and matrix-aware shard addressing.

use anyhow::Result;
use hypermesh_lib::PrivacyMode;
use std::time::{Duration, SystemTime};
use stoq::protocol::{
    MatrixPosition, MatrixPositionExt, PosToken, SpaceProof, StakeProof, StateProof, TimeProof,
    WorkProof,
};
use stoq::transport::certificate_strategy::NetworkType;
use stoq::transport::{StoqTransport, TransportConfig};
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("=== STOQ + Proof of State Integration Demo ===\n");

    // Create STOQ transport with random port for testing
    let config = TransportConfig {
        port: 0, // OS assigns random available port
        ..Default::default()
    };
    let transport = StoqTransport::new(config).await?;

    info!("STOQ transport initialized\n");

    // Demo 1: Anonymous Network (no PoS validation)
    info!("--- Demo 1: Anonymous Network ---");
    demo_anonymous_network(&transport).await?;

    // Demo 2: P2P Network (peer-based validation)
    info!("\n--- Demo 2: P2P Network ---");
    demo_p2p_network(&transport).await?;

    // Demo 3: Federated Network (federation-based validation)
    info!("\n--- Demo 3: Federated Network ---");
    demo_federated_network(&transport).await?;

    // Demo 4: Public Network (full PoS validation)
    info!("\n--- Demo 4: Public Network ---");
    demo_public_network(&transport).await?;

    // Demo 5: Asset Hash Verification
    info!("\n--- Demo 5: Asset Hash Verification ---");
    demo_asset_verification(&transport).await?;

    // Demo 6: Matrix-Aware Shard Distribution
    info!("\n--- Demo 6: Matrix-Aware Shard Distribution ---");
    demo_shard_distribution(&transport).await?;

    // Demo 7: Privacy Mode Enforcement
    info!("\n--- Demo 7: Privacy Mode Enforcement ---");
    demo_privacy_enforcement(&transport).await?;

    // Display final statistics
    info!("\n--- Integration Statistics ---");
    let stats = transport.get_pos_integration_stats();
    info!("Total connections: {}", stats.total_connections);
    info!("  Anonymous: {}", stats.anonymous_connections);
    info!("  Private: {}", stats.private_connections);
    info!("  Public: {}", stats.public_connections);
    info!("Cached assets: {}", stats.cached_assets);
    info!("Registered shards: {}", stats.registered_shards);
    info!(
        "PoS validations: {} (cache hits: {}, failures: {})",
        stats.pos_validations, stats.pos_cache_hits, stats.pos_failures
    );

    info!("\n=== Demo Complete ===");

    Ok(())
}

async fn demo_anonymous_network(transport: &StoqTransport) -> Result<()> {
    info!("Connecting to Anonymous network...");

    let result = transport
        .validate_connection_with_pos(
            "anon_conn_1".to_string(),
            &NetworkType::Anonymous,
            None, // No PoS token needed
        )
        .await?;

    if result {
        info!("Anonymous connection established (no validation required)");
        info!("  - No persistent identity");
        info!("  - No logging");
        info!("  - Ephemeral sessions only");
    }

    Ok(())
}

async fn demo_p2p_network(transport: &StoqTransport) -> Result<()> {
    info!("Connecting to P2P network...");

    let result = transport
        .validate_connection_with_pos(
            "p2p_conn_1".to_string(),
            &NetworkType::P2P,
            None, // Certificate-based validation
        )
        .await?;

    if result {
        info!("P2P connection established (peer-based validation)");
        info!("  - Direct peer trust exchange");
        info!("  - Self-signed certificates");
        info!("  - Maps to Private privacy mode");
    }

    Ok(())
}

async fn demo_federated_network(transport: &StoqTransport) -> Result<()> {
    info!("Connecting to Federated network...");

    let result = transport
        .validate_connection_with_pos(
            "fed_conn_1".to_string(),
            &NetworkType::Federated {
                gateway_url: "gateway.example.internal".to_string(),
            },
            None, // Federation gateway validates
        )
        .await?;

    if result {
        info!("Federated connection established (gateway-based validation)");
        info!("  - Federation gateway as trust anchor");
        info!("  - Network-scoped logging");
        info!("  - Maps to Private privacy mode");
    }

    Ok(())
}

async fn demo_public_network(transport: &StoqTransport) -> Result<()> {
    info!("Connecting to Public network...");

    // Create PoS token with all 4 proofs
    let pos_token = create_demo_pos_token();

    info!("Created PoS token with 4 proofs:");
    info!(
        "  Proof of Space (WHERE): Matrix position ({}, {}, {}) at {}",
        pos_token.matrix_position.0,
        pos_token.matrix_position.1,
        pos_token.matrix_position.2,
        pos_token.proof.space_proof.storage_path
    );
    info!(
        "  Proof of Stake (WHO): authorized identity {} (authorization, not an amount)",
        pos_token.proof.stake_proof.stake_holder_id
    );
    info!(
        "  Proof of Work (WHAT): work hash {} (a hash, not a difficulty)",
        hex_prefix(&pos_token.proof.work_proof.work_hash)
    );
    info!(
        "  Proof of Time (WHEN): sequence {}",
        pos_token.sequence
    );

    let result = transport
        .validate_connection_with_pos(
            "pub_conn_1".to_string(),
            &NetworkType::Public,
            Some(&pos_token),
        )
        .await?;

    if result {
        info!("Public connection established (full PoS validation)");
        info!("  - Global CA with blockchain registration");
        info!("  - Full transparency and logging");
        info!("  - Maximum CAESAR rewards");
    }

    Ok(())
}

async fn demo_asset_verification(transport: &StoqTransport) -> Result<()> {
    info!("Demonstrating asset hash verification...");

    // First establish a connection
    let pos_token = create_demo_pos_token();
    transport
        .validate_connection_with_pos(
            "asset_conn".to_string(),
            &NetworkType::Public,
            Some(&pos_token),
        )
        .await?;

    // Create test asset
    let asset_data = b"Example asset data for verification";
    let asset_id = b"asset_demo_123";

    // Compute BLAKE3 hash (matching library implementation)
    let content_hash: [u8; 32] = *blake3::hash(asset_data).as_bytes();

    info!("Asset ID: {:?}", String::from_utf8_lossy(asset_id));
    info!("Asset size: {} bytes", asset_data.len());
    info!("Content hash: {:02x?}...", &content_hash[..8]);

    // Validate at protocol level
    let is_valid =
        transport.validate_asset_hash("asset_conn", asset_id, &content_hash, asset_data)?;

    if is_valid {
        info!("Asset hash verified at protocol level");
        info!("  - Prevents corrupted data from reaching application");
        info!("  - Tamper detection built into transport");
    }

    Ok(())
}

async fn demo_shard_distribution(transport: &StoqTransport) -> Result<()> {
    info!("Demonstrating matrix-aware shard distribution...");

    let num_shards = 10;
    let origin = MatrixPosition::origin();
    let min_distance = 10.0;
    let max_distance = 100.0;

    info!("Calculating optimal shard positions:");
    info!("  - Number of shards: {}", num_shards);
    info!("  - Origin: ({}, {}, {})", origin.x, origin.y, origin.z);
    info!(
        "  - Distance range: {:.1} - {:.1}",
        min_distance, max_distance
    );

    let positions =
        transport.calculate_shard_positions(num_shards, origin, min_distance, max_distance);

    info!(
        "Generated {} shard positions using golden ratio sphere packing",
        positions.len()
    );

    // Register shard addresses
    for (i, position) in positions.iter().enumerate().take(5) {
        let shard_id = i as u32;
        transport.register_shard_address(
            shard_id,
            *position,
            format!("network_{}", i % 3),
            Some(format!("node_{i}")),
        );

        info!(
            "  Shard {}: position ({:3}, {:3}, {:3}) on network_{}",
            shard_id,
            position.x,
            position.y,
            position.z,
            i % 3
        );
    }

    // Retrieve shard addresses
    let shard_ids: Vec<u32> = (0..5).collect();
    let addresses = transport.get_shard_addresses(&shard_ids);

    info!(
        "Retrieved {} shard addresses for instruction-based retrieval",
        addresses.len()
    );
    info!("  - Receiver uses addresses to query matrix positions");
    info!("  - Bandwidth efficiency (send addresses not data)");
    info!("  - Distributed load across matrix nodes");

    Ok(())
}

async fn demo_privacy_enforcement(transport: &StoqTransport) -> Result<()> {
    info!("Demonstrating privacy mode enforcement...");

    let _pos_integration = transport.pos_integration();

    info!("Privacy Mode Comparison:");

    info!("\n  Anonymous:");
    info!(
        "    - Requires identity: {}",
        PrivacyMode::ANONYMOUS.requires_identity()
    );
    info!(
        "    - Allows logging: {}",
        PrivacyMode::ANONYMOUS.allows_logging()
    );
    info!(
        "    - Connection timeout: {}s",
        PrivacyMode::ANONYMOUS.connection_timeout_secs()
    );

    info!("\n  Private (covers P2P and Federated networks):");
    info!(
        "    - Requires identity: {}",
        PrivacyMode::PRIVATE.requires_identity()
    );
    info!(
        "    - Allows logging: {}",
        PrivacyMode::PRIVATE.allows_logging()
    );
    info!(
        "    - Connection timeout: {}s",
        PrivacyMode::PRIVATE.connection_timeout_secs()
    );

    info!("\n  Public:");
    info!(
        "    - Requires identity: {}",
        PrivacyMode::PUBLIC.requires_identity()
    );
    info!(
        "    - Allows logging: {}",
        PrivacyMode::PUBLIC.allows_logging()
    );
    info!(
        "    - Connection timeout: {}s",
        PrivacyMode::PUBLIC.connection_timeout_secs()
    );

    info!("\nPrivacy mode enforcement ensures protocol behavior matches network type");

    Ok(())
}

/// Build a canonical four-proof set for the demo.
///
/// CANONICAL MODEL: authorization (WHO) with NO amount, work HASH (WHAT),
/// location (WHERE, capacity descriptive only), time (WHEN).
/// First 8 bytes of a hash rendered as hex, for logging.
fn hex_prefix(bytes: &[u8; 32]) -> String {
    bytes[..8].iter().map(|b| format!("{b:02x}")).collect()
}

fn demo_proof() -> StateProof {
    let mut space = SpaceProof::new(
        "demo-node-001".to_string(),
        "hypermesh://demo-node-001/store".to_string(),
        1024 * 1024 * 1024,
    );
    space.file_hash = "d3m0c0nt3nth45h".to_string();

    StateProof::new(
        StakeProof::new("demo-owner".to_string(), "unbound".to_string()),
        TimeProof::new(Duration::from_secs(1)),
        space,
        WorkProof::from_work(
            "demo-owner".to_string(),
            "demo-workload".to_string(),
            b"the work the demo actually did",
        ),
    )
}

fn create_demo_pos_token() -> PosToken {
    PosToken::for_identity(
        vec![1, 2, 3, 4, 5, 6, 7, 8],
        vec![200, 201, 202, 203],
        demo_proof(),
        (100, 200, 300),
        42,
        vec![5, 10, 15, 20, 25, 30, 35, 40],
        Duration::from_secs(3600),
    )
}
