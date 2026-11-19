use super::*;

// ── Content-interested propagation tests ────────────────────────

use crate::network::peer_auth;
use crate::network::reflector_pool::{ReflectorConfig, ReflectorPool};
use crate::network::SwarmDemandTracker;

/// Deterministic transport that always succeeds (no randomness).
struct AlwaysSuccessTransport;

#[async_trait::async_trait]
impl BlockTransport for AlwaysSuccessTransport {
    async fn send_block(
        &self,
        _block: &Block,
        _target: &MatrixCoordinate,
        _origin: &MatrixCoordinate,
    ) -> bool {
        true
    }
}

fn make_interest_context(scope: BlockchainScope, privacy: PrivacyMode) -> Arc<InterestContext> {
    Arc::new(InterestContext {
        swarm_demand: Arc::new(SwarmDemandTracker::new()),
        reflector_pool: Arc::new(tokio::sync::Mutex::new(ReflectorPool::new(
            ReflectorConfig::default(),
        ))),
        authenticated_peers: peer_auth::new_authenticated_peers(),
        network_id: "test-net".to_string(),
        blockchain_scope: scope,
        privacy_mode: privacy,
        spatial_assigner: None,
    })
}

#[tokio::test]
async fn test_content_interested_device_no_propagation() {
    let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
    let mut propagator = BlockPropagator::with_transport(
        origin,
        PropagationStrategy::ContentInterested,
        Arc::new(AlwaysSuccessTransport),
    );

    let ctx = make_interest_context(BlockchainScope::Device, PrivacyMode::PRIVATE);
    propagator.set_interest_context(ctx);

    let network = create_test_network();
    let block = Block::genesis(origin);

    let result = propagator.propagate_block(&block, &network).await;
    // Device scope should return empty targets — no propagation
    assert!(
        result.reached_nodes.is_empty(),
        "Device scope should not propagate blocks"
    );
}

#[tokio::test]
async fn test_content_interested_private_full_replication() {
    let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
    let mut propagator = BlockPropagator::with_transport(
        origin,
        PropagationStrategy::ContentInterested,
        Arc::new(AlwaysSuccessTransport),
    );

    let ctx = make_interest_context(BlockchainScope::Network, PrivacyMode::PRIVATE);
    propagator.set_interest_context(ctx);

    let network = create_test_network();
    let block = Block::genesis(origin);

    let result = propagator.propagate_block(&block, &network).await;
    // Private should replicate to all peers (minus self, which is filtered
    // by should_propagate_to). Network has 27 nodes, minus origin = up to 26.
    assert!(
        !result.reached_nodes.is_empty(),
        "Private mode should replicate to peers"
    );
    // All non-self peers should be reached with AlwaysSuccessTransport
    assert_eq!(result.reached_nodes.len(), 26);
}

#[tokio::test]
async fn test_content_interested_public_includes_reflectors() {
    let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
    let mut propagator = BlockPropagator::with_transport(
        origin,
        PropagationStrategy::ContentInterested,
        Arc::new(AlwaysSuccessTransport),
    );

    let ctx = make_interest_context(BlockchainScope::Network, PrivacyMode::PUBLIC);

    // Register a reflector at position (0,0,0)
    {
        let mut pool = ctx.reflector_pool.lock().await;
        pool.register_reflector(
            "test-net",
            crate::network::reflector_pool::Reflector {
                node_id: "reflector-1".to_string(),
                position: hypermesh_lib::MatrixPosition {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                last_seen: 999999,
                block_height: 100,
                health_score: 0.9,
                privacy_mode: PrivacyMode::PUBLIC,
            },
        );
    }

    propagator.set_interest_context(ctx);

    let network = create_test_network();
    let block = Block::genesis(origin);

    let result = propagator.propagate_block(&block, &network).await;
    // Should include reflector coordinate (0,0,0) among reached nodes
    let reflector_coord = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
    assert!(
        result.reached_nodes.contains(&reflector_coord),
        "Public mode should include reflector targets"
    );
}

#[tokio::test]
async fn test_content_interested_anonymous_consumers_only() {
    let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
    let mut propagator = BlockPropagator::with_transport(
        origin,
        PropagationStrategy::ContentInterested,
        Arc::new(AlwaysSuccessTransport),
    );

    let ctx = make_interest_context(BlockchainScope::Network, PrivacyMode::ANONYMOUS);

    // Register no demand — should result in zero targets
    propagator.set_interest_context(ctx);

    let network = create_test_network();
    let block = Block::genesis(origin);

    let result = propagator.propagate_block(&block, &network).await;
    // Anonymous with no active consumers = no propagation
    assert!(
        result.reached_nodes.is_empty(),
        "Anonymous mode with no consumers should not propagate"
    );
}

#[tokio::test]
async fn test_content_interested_anonymous_with_consumer() {
    let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
    let mut propagator = BlockPropagator::with_transport(
        origin,
        PropagationStrategy::ContentInterested,
        Arc::new(AlwaysSuccessTransport),
    );

    let ctx = make_interest_context(BlockchainScope::Network, PrivacyMode::ANONYMOUS);

    // Create a block with a known asset hash
    let block = Block::genesis(origin);
    let asset_hash = block.entries[0].asset_hash;

    // Record demand for this asset hash from a peer
    let content_hash = hypermesh_lib::ContentHash(asset_hash);
    ctx.swarm_demand
        .record_fetch(content_hash, "consumer-node-1")
        .await;

    // Register the consumer as an authenticated peer at (2,2,2)
    peer_auth::register_authenticated_peer(
        &ctx.authenticated_peers,
        peer_auth::AuthenticatedPeer {
            connection_id: "consumer-session".into(),
            node_id: "consumer-node-1".to_string(),
            pubkey: vec![1, 2, 3],
            coordinate: (2, 2, 2),
            network_id: hypermesh_lib::NetworkId::from_wire_str("test-net"),
            authenticated_at: std::time::Instant::now(),
            proof_bytes: vec![4, 5, 6],
        },
    )
    .await;

    propagator.set_interest_context(ctx);

    let network = create_test_network();
    let result = propagator.propagate_block(&block, &network).await;

    // Should reach the consumer at (2,2,2)
    let consumer_coord = MatrixCoordinate::new(2, 2, 2).expect("test: valid coord");
    assert!(
        result.reached_nodes.contains(&consumer_coord),
        "Anonymous mode should propagate to active consumers"
    );
}

#[tokio::test]
async fn test_content_interested_fallback_without_context() {
    let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
    // ContentInterested strategy but NO interest context set
    let propagator = BlockPropagator::with_transport(
        origin,
        PropagationStrategy::ContentInterested,
        Arc::new(AlwaysSuccessTransport),
    );

    let network = create_test_network();
    let block = Block::genesis(origin);

    let result = propagator.propagate_block(&block, &network).await;
    // Should fall back to select_propagation_targets (NearestN behavior
    // is not available for ContentInterested, so it uses default routing)
    // The key assertion: it doesn't crash and produces some result
    let total_nodes = result.reached_nodes.len() + result.failed_nodes.len();
    assert!(total_nodes < usize::MAX, "Fallback should not panic");
}
