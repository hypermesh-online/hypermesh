//! Integration tests for the every-node-blockchain architecture
//!
//! Tests the integration between blockchain components and matrix topology.

#[cfg(test)]
mod tests {
    use crate::blockchain::*;
    use crate::matrix::coordinate::MatrixCoordinate;
    use crate::matrix::neighbors::{find_neighbors, find_k_nearest};
    use tempfile::TempDir;
    use tokio;
    use std::collections::HashMap;

    /// Create a test network of node blockchains
    async fn create_test_network(
        size: usize,
    ) -> Vec<(NodeBlockchain, MatrixCoordinate)> {
        let mut nodes = Vec::new();

        // Create a grid of nodes
        for i in 0..size {
            let x = (i % 3) as i32;
            let y = ((i / 3) % 3) as i32;
            let z = (i / 9) as i32;

            let coord = MatrixCoordinate::new(x as i64, y as i64, z as i64).unwrap();
            let blockchain = NodeBlockchain::new(coord.clone());

            nodes.push((blockchain, coord));
        }

        nodes
    }

    #[tokio::test]
    async fn test_multi_node_independent_chains() {
        // Create 3 independent nodes
        let nodes = create_test_network(3).await;

        // Each node adds blocks to its own chain
        for (i, (blockchain, coord)) in nodes.iter().enumerate() {
            let data = format!("Node {} block 1", i);
            let block = blockchain.add_block_with_data(data.as_bytes().to_vec()).await.unwrap();

            // Verify block belongs to this node
            assert!(block.belongs_to_node(coord));
            assert_eq!(block.index, 1);
        }

        // Verify chains are independent
        assert_eq!(nodes[0].0.get_height().await, 1);
        assert_eq!(nodes[1].0.get_height().await, 1);
        assert_eq!(nodes[2].0.get_height().await, 1);

        // Each chain should have different block hashes (due to different node coordinates)
        let block0 = nodes[0].0.get_block(1).await.unwrap();
        let block1 = nodes[1].0.get_block(1).await.unwrap();
        let block2 = nodes[2].0.get_block(1).await.unwrap();

        assert_ne!(block0.hash, block1.hash);
        assert_ne!(block1.hash, block2.hash);
        assert_ne!(block0.hash, block2.hash);
    }

    #[tokio::test]
    async fn test_propagation_with_matrix_topology() {
        // Create a 3x3x3 grid (27 nodes)
        let network_coords: Vec<MatrixCoordinate> = (0..27)
            .map(|i| {
                let x = (i % 3) as i32;
                let y = ((i / 3) % 3) as i32;
                let z = (i / 9) as i32;
                MatrixCoordinate::new(x as i64, y as i64, z as i64).unwrap()
            })
            .collect();

        // Create blockchain at center node (1,1,1)
        let center = MatrixCoordinate::new(1, 1, 1).unwrap();
        let blockchain = NodeBlockchain::new(center.clone());

        // Create propagator with broadcast strategy
        let propagator = BlockPropagator::new(
            center.clone(),
            PropagationStrategy::Broadcast,
        );

        // Add a block and propagate it
        let block = blockchain.add_block_with_data(b"Test block".to_vec()).await.unwrap();
        let result = propagator.propagate_block(&block, &network_coords).await;

        // Should reach immediate neighbors
        assert!(!result.reached_nodes.is_empty());

        // Verify neighbors are actually adjacent in matrix
        let expected_neighbors = find_neighbors(&center, &network_coords, 1.5);

        // All reached nodes should be in expected neighbors
        for reached in &result.reached_nodes {
            assert!(
                expected_neighbors.contains(reached),
                "Node {:?} not in expected neighbors",
                reached
            );
        }
    }

    #[tokio::test]
    async fn test_chain_state_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let coord = MatrixCoordinate::new(5, 5, 5).unwrap();

        // Create blockchain and state manager
        let blockchain = NodeBlockchain::new(coord.clone());
        let state_manager = ChainStateManager::new(
            coord.clone(),
            temp_dir.path(),
        );
        state_manager.initialize().await.unwrap();

        // Add blocks and persist them
        for i in 0..5 {
            let data = format!("Block {}", i);
            let block = blockchain.add_block_with_data(data.as_bytes().to_vec()).await.unwrap();
            state_manager.store_block(&block).await.unwrap();
        }

        // Create snapshot
        let head = blockchain.get_head().await.unwrap();
        let snapshot = state_manager.create_snapshot(
            head.index,
            head.hash.clone(),
        ).await.unwrap();

        assert_eq!(snapshot.height, 5);
        assert_eq!(snapshot.total_blocks, 6); // Including genesis

        // Query blocks
        let query = BlockQuery {
            from_index: Some(2),
            to_index: Some(4),
            ..Default::default()
        };

        let results = state_manager.query_blocks(query).await.unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].index, 2);
        assert_eq!(results[2].index, 4);
    }

    #[tokio::test]
    async fn test_routed_propagation_with_pathfinding() {
        // Create network
        let network_coords: Vec<MatrixCoordinate> = (0..27)
            .map(|i| {
                let x = (i % 3) as i32;
                let y = ((i / 3) % 3) as i32;
                let z = (i / 9) as i32;
                MatrixCoordinate::new(x as i64, y as i64, z as i64).unwrap()
            })
            .collect();

        // Start at corner (0,0,0)
        let origin = MatrixCoordinate::new(0, 0, 0).unwrap();
        let blockchain = NodeBlockchain::new(origin.clone());

        // Use routed propagation strategy
        let propagator = BlockPropagator::new(
            origin.clone(),
            PropagationStrategy::RoutedPath,
        );

        // Add block and propagate
        let block = blockchain.add_block_with_data(b"Routed block".to_vec()).await.unwrap();
        let result = propagator.propagate_block(&block, &network_coords).await;

        // Should reach strategic relay nodes
        assert!(!result.reached_nodes.is_empty());

        // Verify nodes are reachable
        for reached in &result.reached_nodes {
            let distance = origin.euclidean_distance(reached);
            assert!(distance > 0.0, "Same node as origin: {:?}", reached);
        }
    }

    #[tokio::test]
    async fn test_flood_propagation() {
        // Create network
        let network_coords: Vec<MatrixCoordinate> = (0..27)
            .map(|i| {
                let x = (i % 3) as i32;
                let y = ((i / 3) % 3) as i32;
                let z = (i / 9) as i32;
                MatrixCoordinate::new(x as i64, y as i64, z as i64).unwrap()
            })
            .collect();

        let center = MatrixCoordinate::new(1, 1, 1).unwrap();
        let blockchain = NodeBlockchain::new(center.clone());
        let propagator = BlockPropagator::new(
            center.clone(),
            PropagationStrategy::Broadcast,
        );

        // Add critical block and flood propagate
        let block = blockchain.add_block_with_data(b"Critical block".to_vec()).await.unwrap();
        let result = propagator.flood_propagate(&block, &network_coords, 2).await;

        // Should reach many nodes within 2 hops
        assert!(result.reached_nodes.len() > 6); // More than immediate neighbors
        assert_eq!(result.hop_count, 2);

        // Verify all reached nodes are within 2 hops
        for reached in &result.reached_nodes {
            let distance = center.manhattan_distance(reached) as f64;
            assert!(distance <= 2.1, "Node {:?} too far: {}", reached, distance);
        }
    }

    #[tokio::test]
    async fn test_chain_fork_detection() {
        let coord = MatrixCoordinate::new(7, 7, 7).unwrap();
        let blockchain = NodeBlockchain::new(coord.clone());

        // Build main chain
        for i in 0..5 {
            let data = format!("Main chain block {}", i);
            blockchain.add_block_with_data(data.as_bytes().to_vec()).await.unwrap();
        }

        let main_chain = blockchain.get_chain().await;
        assert_eq!(main_chain.len(), 6); // 5 + genesis

        // In our architecture, each node has its own chain, so "forks"
        // are actually different nodes' chains. But we can still validate
        // chain integrity
        assert!(blockchain.validate_chain().await);

        // Verify no cross-node merkle consolidation
        // Each block should belong to this node only
        for block in &main_chain {
            assert!(block.belongs_to_node(&coord));
        }
    }

    #[tokio::test]
    async fn test_concurrent_block_addition() {
        let coord = MatrixCoordinate::new(9, 9, 9).unwrap();
        let blockchain = NodeBlockchain::new(coord);

        // Spawn multiple tasks adding blocks concurrently
        let mut handles = Vec::new();

        for i in 0..10 {
            let bc = &blockchain;
            let handle = tokio::spawn(async move {
                let data = format!("Concurrent block {}", i);
                bc.add_block_with_data(data.as_bytes().to_vec()).await
            });
            handles.push(handle);
        }

        // Wait for all tasks
        let mut results = Vec::new();
        for handle in handles {
            if let Ok(Ok(block)) = handle.await {
                results.push(block);
            }
        }

        // Should have added all blocks
        assert_eq!(blockchain.get_height().await, 10);

        // Verify chain validity
        assert!(blockchain.validate_chain().await);

        // All blocks should have unique indices
        let mut indices: Vec<u64> = results.iter().map(|b| b.index).collect();
        indices.sort();
        indices.dedup();
        assert_eq!(indices.len(), 10);
    }

    #[tokio::test]
    async fn test_performance_thousand_blocks() {
        let coord = MatrixCoordinate::new(10, 10, 10).unwrap();
        let blockchain = NodeBlockchain::new(coord);

        let start = std::time::Instant::now();

        // Add 1000 blocks
        for i in 0..1000 {
            let data = vec![i as u8; 100];
            blockchain.add_block_with_data(data).await.unwrap();
        }

        let elapsed = start.elapsed();

        // Should complete in reasonable time
        assert!(elapsed.as_secs() < 10, "Took too long: {:?}", elapsed);

        // Verify chain
        assert_eq!(blockchain.get_height().await, 1000);

        // Validation should also be fast
        let val_start = std::time::Instant::now();
        assert!(blockchain.validate_chain().await);
        let val_elapsed = val_start.elapsed();

        assert!(
            val_elapsed.as_millis() < 100,
            "Validation took too long: {:?}",
            val_elapsed
        );
    }

    #[tokio::test]
    async fn test_end_to_end_three_node_scenario() {
        // Create 3 independent nodes with their own blockchains
        let coord1 = MatrixCoordinate::new(0, 0, 0).unwrap();
        let coord2 = MatrixCoordinate::new(1, 0, 0).unwrap();
        let coord3 = MatrixCoordinate::new(0, 1, 0).unwrap();

        let chain1 = NodeBlockchain::new(coord1.clone());
        let chain2 = NodeBlockchain::new(coord2.clone());
        let chain3 = NodeBlockchain::new(coord3.clone());

        // Each node adds its own blocks
        chain1.add_block_with_data(b"Node 1 data A".to_vec()).await.unwrap();
        chain1.add_block_with_data(b"Node 1 data B".to_vec()).await.unwrap();

        chain2.add_block_with_data(b"Node 2 data X".to_vec()).await.unwrap();
        chain2.add_block_with_data(b"Node 2 data Y".to_vec()).await.unwrap();
        chain2.add_block_with_data(b"Node 2 data Z".to_vec()).await.unwrap();

        chain3.add_block_with_data(b"Node 3 data 1".to_vec()).await.unwrap();

        // Verify independence
        assert_eq!(chain1.get_height().await, 2);
        assert_eq!(chain2.get_height().await, 3);
        assert_eq!(chain3.get_height().await, 1);

        // Each chain is valid independently
        assert!(chain1.validate_chain().await);
        assert!(chain2.validate_chain().await);
        assert!(chain3.validate_chain().await);

        // NO cross-chain validation or merkle consolidation
        // This is the revolutionary aspect - complete independence
    }
}