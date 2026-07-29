// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Example: Using tensor operations for intelligent routing in Block-MATRIX
//!
//! This example demonstrates how to use the tensor operations library
//! to implement intelligent routing decisions in the Block-MATRIX topology.

use blockmatrix::matrix::{
    coordinate::MatrixCoordinate,
    tensor::{
        calculate_load_balanced_routes, calculate_orthogonal_routes, calculate_routing_path,
        calculate_routing_vector, find_aligned_nodes, score_route_quality,
    },
};

fn main() {
    println!("=== Block-MATRIX Tensor Routing Example ===\n");

    // Scenario: Route data from source node to destination
    let source = MatrixCoordinate::new(0, 0, 0).expect("Valid coordinate");
    let destination = MatrixCoordinate::new(100, 50, 25).expect("Valid coordinate");

    println!("Source: ({}, {}, {})", source.x, source.y, source.z);
    println!(
        "Destination: ({}, {}, {})",
        destination.x, destination.y, destination.z
    );
    println!("Distance: {:.2}\n", source.euclidean_distance(&destination));

    // Step 1: Calculate optimal routing direction
    println!("Step 1: Calculate Routing Direction");
    println!("====================================");
    let direction = calculate_routing_vector(&source, &destination);
    println!(
        "Optimal direction: ({:.3}, {:.3}, {:.3})",
        direction.x, direction.y, direction.z
    );
    println!("Direction magnitude: {:.3}\n", direction.magnitude());

    // Step 2: Find intermediate hops for multi-hop routing
    println!("Step 2: Multi-Hop Routing Path");
    println!("===============================");
    let max_hop_distance = 30.0;
    let routing_path = calculate_routing_path(&source, &destination, max_hop_distance);
    println!("Path with max hop distance {max_hop_distance:.1}:");
    for (i, coord) in routing_path.iter().enumerate() {
        println!("  Hop {}: ({}, {}, {})", i, coord.x, coord.y, coord.z);
    }
    println!();

    // Step 3: Find aligned relay nodes
    println!("Step 3: Find Aligned Relay Nodes");
    println!("=================================");
    let candidate_nodes = vec![
        MatrixCoordinate::new(50, 25, 12).expect("Valid"), // Well aligned
        MatrixCoordinate::new(45, 30, 10).expect("Valid"), // Slightly off
        MatrixCoordinate::new(10, 50, 40).expect("Valid"), // Poor alignment
        MatrixCoordinate::new(80, 40, 20).expect("Valid"), // Good alignment
        MatrixCoordinate::new(-20, 10, 5).expect("Valid"), // Wrong direction
    ];

    let alignment_threshold = 0.9; // 90% alignment required
    let aligned = find_aligned_nodes(&source, &direction, &candidate_nodes, alignment_threshold);

    println!("Candidates aligned with routing direction (threshold={alignment_threshold:.1}):");
    for node in &aligned {
        let node_dir = calculate_routing_vector(&source, node);
        let similarity = direction.dot(&node_dir);
        println!(
            "  Node ({}, {}, {}) - Similarity: {:.3}",
            node.x, node.y, node.z, similarity
        );
    }
    println!();

    // Step 4: Calculate alternative routes for load balancing
    println!("Step 4: Load-Balanced Alternative Routes");
    println!("=========================================");
    let alternatives = calculate_load_balanced_routes(&source, &destination, 3, 0.3);

    println!("Primary + {} alternative routes:", alternatives.len() - 1);
    for (i, route) in alternatives.iter().enumerate() {
        let label = if i == 0 { "Primary" } else { "Alternative" };
        println!(
            "  {} {}: Direction ({:.3}, {:.3}, {:.3})",
            label, i, route.x, route.y, route.z
        );
    }
    println!();

    // Step 5: Calculate orthogonal routes
    println!("Step 5: Orthogonal Routes (for redundancy)");
    println!("===========================================");
    let orthogonals = calculate_orthogonal_routes(&direction);

    for (i, ortho) in orthogonals.iter().enumerate() {
        println!(
            "  Orthogonal {}: ({:.3}, {:.3}, {:.3})",
            i + 1,
            ortho.x,
            ortho.y,
            ortho.z
        );
        let dot_product = direction.dot(ortho);
        println!("    Verification (should be ~0): {dot_product:.6}");
    }
    println!();

    // Step 6: Score route quality
    println!("Step 6: Route Quality Scoring");
    println!("==============================");
    let ideal_hop = 25.0;
    let quality = score_route_quality(&routing_path, ideal_hop);
    println!("Route quality score: {quality:.2}/100");
    println!("  (Based on hop distance, direction changes, efficiency)\n");

    println!("\n=== Example Complete ===");
}
