#!/usr/bin/env rust-script
//! Test the blockchain module compilation and basic functionality

use blockmatrix::blockchain::{NodeBlockchain, Block, PropagationStrategy, BlockPropagator};
use blockmatrix::matrix::coordinate::MatrixCoordinate;

#[tokio::main]
async fn main() {
    println!("Testing blockchain module compilation...");

    // Create a test coordinate
    let coord = MatrixCoordinate::new(1, 2, 3).expect("Failed to create coordinate");
    println!("✓ Created MatrixCoordinate: ({}, {}, {})", coord.x, coord.y, coord.z);

    // Create a blockchain for this node
    let blockchain = NodeBlockchain::new(coord.clone());
    println!("✓ Created NodeBlockchain for node");

    // Get genesis block
    let genesis = blockchain.get_genesis().await;
    println!("✓ Genesis block hash: {}", genesis.hash);

    // Add a block
    let data = b"Test block data".to_vec();
    let block = blockchain.add_block(data).await.expect("Failed to add block");
    println!("✓ Added block #{}  to chain", block.index);

    // Get chain height
    let height = blockchain.get_height().await;
    println!("✓ Chain height: {}", height);

    // Create a propagator
    let propagator = BlockPropagator::new(coord, PropagationStrategy::Broadcast);
    println!("✓ Created BlockPropagator with Broadcast strategy");

    println!("\n✅ All blockchain module tests passed!");
}