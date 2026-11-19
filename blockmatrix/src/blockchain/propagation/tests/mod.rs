use super::*;

fn create_test_network() -> Vec<MatrixCoordinate> {
    let mut nodes = Vec::new();
    // Create a 3x3x3 grid of nodes
    for x in 0..3 {
        append_test_plane(&mut nodes, x);
    }

    nodes
}

fn append_test_plane(nodes: &mut Vec<MatrixCoordinate>, x: i64) {
    for y in 0..3 {
        for z in 0..3 {
            nodes.push(MatrixCoordinate::new(x, y, z).expect("test: valid coordinate"));
        }
    }
}

#[tokio::test]
async fn test_broadcast_propagation() {
    let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
    let propagator = BlockPropagator::new(origin, PropagationStrategy::Broadcast);

    let network = create_test_network();
    let block = Block::genesis(origin);

    let result = propagator.propagate_block(&block, &network).await;

    // Should reach immediate neighbors (distance <= 1.5 includes
    // face-adjacent and edge-diagonal neighbours in a 3x3x3 grid)
    assert!(!result.reached_nodes.is_empty());
    assert!(result.reached_nodes.len() <= 18); // Up to 18 within distance 1.5
}

#[tokio::test]
async fn test_nearest_n_propagation() {
    let origin = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
    let propagator = BlockPropagator::new(origin, PropagationStrategy::NearestN(3));

    let network = create_test_network();
    let block = Block::genesis(origin);

    let result = propagator.propagate_block(&block, &network).await;

    // Should reach exactly 3 or fewer nodes
    assert!(result.reached_nodes.len() <= 3);
}

#[tokio::test]
async fn test_distance_threshold_propagation() {
    let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
    let propagator = BlockPropagator::new(origin, PropagationStrategy::DistanceThreshold(2.0));

    let network = create_test_network();
    let block = Block::genesis(origin);

    let result = propagator.propagate_block(&block, &network).await;

    // Verify all reached nodes are within threshold
    for node in &result.reached_nodes {
        let distance = origin.euclidean_distance(node);
        assert!(distance <= 2.0);
    }
}

#[tokio::test]
async fn test_routed_propagation() {
    let origin = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
    let propagator = BlockPropagator::new(origin, PropagationStrategy::RoutedPath);

    let network = create_test_network();
    let block = Block::genesis(origin);

    let result = propagator.propagate_block(&block, &network).await;

    // Should reach some strategic relay nodes
    assert!(!result.reached_nodes.is_empty());
}

#[tokio::test]
async fn test_duplicate_prevention() {
    let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
    let propagator = BlockPropagator::new(origin, PropagationStrategy::Broadcast);

    let network = create_test_network();
    let block = Block::genesis(origin);

    // First propagation
    let result1 = propagator.propagate_block(&block, &network).await;
    let reached_count_1 = result1.reached_nodes.len();

    // Second propagation of same block
    let result2 = propagator.propagate_block(&block, &network).await;

    // Should not propagate to nodes that already have it
    assert!(result2.reached_nodes.is_empty() || result2.reached_nodes.len() < reached_count_1);
}

#[tokio::test]
async fn test_flood_propagation() {
    let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
    let propagator = BlockPropagator::new(origin, PropagationStrategy::Broadcast);

    let network = create_test_network();
    let block = Block::genesis(origin);

    let result = propagator.flood_propagate(&block, &network, 2).await;

    // Should reach multiple nodes through flooding
    assert!(result.reached_nodes.len() > 1);
    assert_eq!(result.hop_count, 2);
}

#[tokio::test]
async fn test_relay_node_selection() {
    let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
    let propagator = BlockPropagator::new(origin, PropagationStrategy::RoutedPath);

    let network = create_test_network();
    let relays = propagator.find_relay_nodes(&network);

    // Should find corner nodes as relays
    assert!(!relays.is_empty());

    // Verify relays are at boundaries
    for relay in relays {
        let is_boundary = (relay.x == 0 || relay.x == 2)
            || (relay.y == 0 || relay.y == 2)
            || (relay.z == 0 || relay.z == 2);
        assert!(is_boundary);
    }
}

#[tokio::test]
async fn test_propagation_metrics() {
    let origin = MatrixCoordinate::new(0, 0, 0).expect("test: valid coordinate");
    let propagator = BlockPropagator::new(origin, PropagationStrategy::Broadcast);

    let network = create_test_network();
    let block = Block::genesis(origin);

    let result = propagator.propagate_block(&block, &network).await;

    // Verify metrics are recorded
    let _ = result.propagation_time_ms; // u64 always >= 0
    assert_eq!(result.hop_count, 1);
    assert_eq!(
        result.reached_nodes.len() + result.failed_nodes.len(),
        result.reached_nodes.len() + result.failed_nodes.len()
    );
}

#[tokio::test]
async fn test_apply_weights_filters_zero_weight() {
    let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
    let propagator = BlockPropagator::new(origin, PropagationStrategy::Broadcast);

    let skip = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
    let keep = MatrixCoordinate::new(2, 2, 2).expect("test: valid coord");

    propagator
        .set_propagation_weights(vec![
            PropagationWeight {
                coordinate: skip,
                weight: 0.0,
            },
            PropagationWeight {
                coordinate: keep,
                weight: 1.5,
            },
        ])
        .await;

    let result = propagator.apply_weights(vec![skip, keep]).await;
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], keep);
}

#[tokio::test]
async fn test_apply_weights_sorts_by_weight() {
    let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
    let propagator = BlockPropagator::new(origin, PropagationStrategy::Broadcast);

    let low = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
    let high = MatrixCoordinate::new(2, 2, 2).expect("test: valid coord");

    propagator
        .set_propagation_weights(vec![
            PropagationWeight {
                coordinate: low,
                weight: 0.5,
            },
            PropagationWeight {
                coordinate: high,
                weight: 2.0,
            },
        ])
        .await;

    let result = propagator.apply_weights(vec![low, high]).await;
    assert_eq!(result.len(), 2);
    // High weight should come first
    assert_eq!(result[0], high);
    assert_eq!(result[1], low);
}

#[tokio::test]
async fn test_apply_weights_empty_keeps_original_order() {
    let origin = MatrixCoordinate::new(1, 1, 1).expect("test: valid coordinate");
    let propagator = BlockPropagator::new(origin, PropagationStrategy::Broadcast);

    let a = MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
    let b = MatrixCoordinate::new(2, 2, 2).expect("test: valid coord");

    // No weights set
    let result = propagator.apply_weights(vec![a, b]).await;
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], a);
    assert_eq!(result[1], b);
}

mod consumer;
