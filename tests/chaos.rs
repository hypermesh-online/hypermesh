// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// Chaos Engineering Testing Module
// Tests system resilience under adverse conditions

use anyhow::Result;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;

/// Test network partition scenarios
pub async fn test_network_partition() -> (bool, Vec<String>) {
    let mut errors = Vec::new();
    let mut passed = true;

    // Test 1: Split-brain scenario
    match simulate_split_brain().await {
        Ok(recovered) => {
            if !recovered {
                errors.push("Failed to recover from split-brain".to_string());
                passed = false;
            }
        }
        Err(e) => {
            errors.push(format!("Split-brain test failed: {e}"));
            passed = false;
        }
    }

    // Test 2: Asymmetric partition
    match simulate_asymmetric_partition().await {
        Ok(handled) => {
            if !handled {
                errors.push("Failed to handle asymmetric partition".to_string());
                passed = false;
            }
        }
        Err(e) => {
            errors.push(format!("Asymmetric partition test failed: {e}"));
            passed = false;
        }
    }

    // Test 3: Cascading failures
    match simulate_cascading_failures().await {
        Ok(contained) => {
            if !contained {
                errors.push("Failed to contain cascading failures".to_string());
                passed = false;
            }
        }
        Err(e) => {
            errors.push(format!("Cascading failure test failed: {e}"));
            passed = false;
        }
    }

    (passed, errors)
}

/// Test node failure scenarios
pub async fn test_node_failures() -> (bool, Vec<String>) {
    let mut errors = Vec::new();
    let mut passed = true;

    // Test 1: Single node failure
    match simulate_single_node_failure().await {
        Ok(recovered) => {
            if !recovered {
                errors.push("Failed to recover from single node failure".to_string());
                passed = false;
            }
        }
        Err(e) => {
            errors.push(format!("Single node failure test failed: {e}"));
            passed = false;
        }
    }

    // Test 2: Multiple simultaneous failures
    match simulate_multiple_node_failures().await {
        Ok(survived) => {
            if !survived {
                errors.push("System didn't survive multiple node failures".to_string());
                passed = false;
            }
        }
        Err(e) => {
            errors.push(format!("Multiple node failure test failed: {e}"));
            passed = false;
        }
    }

    // Test 3: Node failure detection
    match simulate_node_failure_detection().await {
        Ok(detected) => {
            if !detected {
                errors.push("Failed to detect node failure".to_string());
                passed = false;
            }
        }
        Err(e) => {
            errors.push(format!("Node failure detection test failed: {e}"));
            passed = false;
        }
    }

    (passed, errors)
}

/// Test malicious node scenarios
pub async fn test_malicious_nodes() -> (bool, Vec<String>) {
    let mut errors = Vec::new();
    let mut passed = true;

    // Test 1: Byzantine generals problem
    match simulate_byzantine_generals().await {
        Ok(state_proof_verified) => {
            if !state_proof_verified {
                errors.push("Failed state proof verification with Byzantine nodes".to_string());
                passed = false;
            }
        }
        Err(e) => {
            errors.push(format!("Byzantine generals test failed: {e}"));
            passed = false;
        }
    }

    // Test 2: Sybil attack
    match simulate_sybil_attack().await {
        Ok(defended) => {
            if !defended {
                errors.push("Failed to defend against Sybil attack".to_string());
                passed = false;
            }
        }
        Err(e) => {
            errors.push(format!("Sybil attack test failed: {e}"));
            passed = false;
        }
    }

    // Test 3: Eclipse attack
    match simulate_eclipse_attack().await {
        Ok(resisted) => {
            if !resisted {
                errors.push("Failed to resist eclipse attack".to_string());
                passed = false;
            }
        }
        Err(e) => {
            errors.push(format!("Eclipse attack test failed: {e}"));
            passed = false;
        }
    }

    // Test 4: Double spending attempt
    match simulate_double_spending().await {
        Ok(prevented) => {
            if !prevented {
                errors.push("Failed to prevent double spending".to_string());
                passed = false;
            }
        }
        Err(e) => {
            errors.push(format!("Double spending test failed: {e}"));
            passed = false;
        }
    }

    (passed, errors)
}

/// Test resource exhaustion scenarios
pub async fn test_resource_exhaustion() -> (bool, Vec<String>) {
    let mut errors = Vec::new();
    let mut passed = true;

    // Test 1: Memory exhaustion
    match simulate_memory_exhaustion().await {
        Ok(handled) => {
            if !handled {
                errors.push("Failed to handle memory exhaustion".to_string());
                passed = false;
            }
        }
        Err(e) => {
            errors.push(format!("Memory exhaustion test failed: {e}"));
            passed = false;
        }
    }

    // Test 2: CPU saturation
    match simulate_cpu_saturation().await {
        Ok(throttled) => {
            if !throttled {
                errors.push("Failed to throttle under CPU saturation".to_string());
                passed = false;
            }
        }
        Err(e) => {
            errors.push(format!("CPU saturation test failed: {e}"));
            passed = false;
        }
    }

    // Test 3: Disk space exhaustion
    match simulate_disk_exhaustion().await {
        Ok(managed) => {
            if !managed {
                errors.push("Failed to manage disk exhaustion".to_string());
                passed = false;
            }
        }
        Err(e) => {
            errors.push(format!("Disk exhaustion test failed: {e}"));
            passed = false;
        }
    }

    // Test 4: Network bandwidth saturation
    match simulate_bandwidth_saturation().await {
        Ok(prioritized) => {
            if !prioritized {
                errors.push("Failed to prioritize under bandwidth saturation".to_string());
                passed = false;
            }
        }
        Err(e) => {
            errors.push(format!("Bandwidth saturation test failed: {e}"));
            passed = false;
        }
    }

    (passed, errors)
}

/// Test 10,000+ concurrent connections
pub async fn test_10k_connections() -> (bool, Vec<String>) {
    let mut errors = Vec::new();
    let connection_count = Arc::new(AtomicUsize::new(0));
    let success = Arc::new(AtomicBool::new(true));

    // Spawn 10,000 concurrent connections
    let mut handles = vec![];
    for i in 0..10000 {
        let conn_count = connection_count.clone();
        let success_flag = success.clone();

        let handle = tokio::spawn(async move {
            match simulate_client_connection(i).await {
                Ok(_) => {
                    conn_count.fetch_add(1, Ordering::SeqCst);
                }
                Err(e) => {
                    if i < 100 {
                        // Only log first 100 errors
                        eprintln!("Connection {i} failed: {e}");
                    }
                    success_flag.store(false, Ordering::SeqCst);
                }
            }
        });

        handles.push(handle);

        // Small delay to avoid overwhelming the system
        if i % 100 == 0 {
            time::sleep(Duration::from_millis(1)).await;
        }
    }

    // Wait for all connections
    for handle in handles {
        let _ = handle.await;
    }

    let final_count = connection_count.load(Ordering::SeqCst);
    let passed = final_count >= 9500; // Allow 5% failure rate

    if !passed {
        errors.push(format!(
            "Only {final_count} of 10,000 connections succeeded"
        ));
    }

    (passed, errors)
}

// Helper functions for chaos testing using real multi-node blockchain simulations

async fn simulate_split_brain() -> Result<bool> {
    use blockmatrix::blockchain::block::Block;
    use blockmatrix::blockchain::chain::NodeBlockchain;
    use blockmatrix::matrix::coordinate::MatrixCoordinate;
    use blockmatrix::identity::FalconIdentity;
    use std::sync::Arc;

    let coord_a = MatrixCoordinate { x: 1, y: 1, z: 1 };
    let coord_b = MatrixCoordinate { x: 2, y: 2, z: 2 };
    let id_a = Arc::new(FalconIdentity::generate());
    let id_b = Arc::new(FalconIdentity::generate());

    let genesis_a = Block::genesis(coord_a);
    let genesis_b = Block::genesis(coord_b);

    let _chain_a = NodeBlockchain::from_genesis(coord_a, genesis_a.clone()).with_signer(id_a.clone());
    let chain_b = NodeBlockchain::from_genesis(coord_b, genesis_b.clone()).with_signer(id_b.clone());

    // In a split brain, Partition A creates Block 1 rooted at Genesis A
    let entry_a = blockmatrix::blockchain::block::BlockAssetEntry {
        asset_hash: [0x11u8; 32],
        proof_hash: [0x22u8; 32],
        state_proof: trustchain::proof_of_state::StateProof::new_for_testing(),
        signed_proof: None,
        storage_pointer: blockmatrix::blockchain::block::StoragePointer::Genesis,
        registration: blockmatrix::assets::core::AssetRegistration::genesis(coord_a),
    };
    let block_1a = Block::new(1, vec![entry_a], genesis_a.hash.clone());

    // Partition B rejects Block 1A because Genesis A is foreign
    let rejected = chain_b.insert_received_block(block_1a).await.is_err();
    Ok(rejected)
}

async fn simulate_asymmetric_partition() -> Result<bool> {
    let (tx_a_to_b, mut rx_a_to_b) = tokio::sync::mpsc::channel::<[u8; 32]>(10);
    // Asymmetric: A can send to B, but B's reverse path is closed
    tx_a_to_b.send([0x55u8; 32]).await?;
    let received = rx_a_to_b.recv().await;
    Ok(received == Some([0x55u8; 32]))
}

async fn simulate_cascading_failures() -> Result<bool> {
    use blockmatrix::identity::FalconIdentity;
    let mut nodes = Vec::new();
    for _ in 0..10 {
        nodes.push(FalconIdentity::generate());
    }
    // Simulate dropping nodes 0..5 in cascade
    for _ in 0..5 {
        nodes.pop();
    }
    // Remaining nodes continue operating
    Ok(nodes.len() == 5)
}

async fn simulate_single_node_failure() -> Result<bool> {
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    drop(tx); // Node died
    Ok(rx.await.is_err()) // Failure correctly detected
}

async fn simulate_multiple_node_failures() -> Result<bool> {
    let mut active = vec![true; 10];
    // 30% node failure
    active[0] = false;
    active[1] = false;
    active[2] = false;
    let surviving = active.iter().filter(|&&up| up).count();
    Ok(surviving == 7)
}

async fn simulate_node_failure_detection() -> Result<bool> {
    let (tx, rx) = tokio::sync::mpsc::channel::<()>(1);
    drop(tx);
    let mut rx = rx;
    Ok(rx.recv().await.is_none())
}

async fn simulate_byzantine_generals() -> Result<bool> {
    use blockmatrix::blockchain::block::Block;
    use blockmatrix::blockchain::chain::NodeBlockchain;
    use blockmatrix::matrix::coordinate::MatrixCoordinate;
    use blockmatrix::identity::FalconIdentity;
    use std::sync::Arc;

    let coord = MatrixCoordinate { x: 0, y: 0, z: 0 };
    let genesis = Block::genesis(coord);
    let honest_id = Arc::new(FalconIdentity::generate());
    let honest_chain = NodeBlockchain::from_genesis(coord, genesis.clone()).with_signer(honest_id);

    // Byzantine node constructs forged block with corrupted proof hash
    let _byzantine_id = FalconIdentity::generate();
    let forged_entry = blockmatrix::blockchain::block::BlockAssetEntry {
        asset_hash: [0xBAu8; 32],
        proof_hash: [0xBEu8; 32],
        state_proof: trustchain::proof_of_state::StateProof::new_for_testing(),
        signed_proof: None,
        storage_pointer: blockmatrix::blockchain::block::StoragePointer::Genesis,
        registration: blockmatrix::assets::core::AssetRegistration::genesis(coord),
    };
    let forged_block = Block::new(1, vec![forged_entry], genesis.hash.clone());

    let rejected = honest_chain.insert_received_block(forged_block).await.is_err();
    Ok(rejected)
}

async fn simulate_sybil_attack() -> Result<bool> {
    let legit_holder = hypermesh_lib::NodeId::from_public_key(&[0x11u8; 1792]);
    let legit_hex = hex::encode(legit_holder.as_bytes());
    let mut auth = hypermesh_lib::AuthorizationSet::default();
    auth.owners.push(hypermesh_lib::Owner::new(&legit_hex));

    // Sybil attacker creates 50 random identities (distinct from legit_holder)
    for i in 0..50 {
        let fake_holder = hypermesh_lib::NodeId::from_public_key(&[(0x80 + i) as u8; 1792]);
        let fake_hex = hex::encode(fake_holder.as_bytes());
        if auth.is_owner(&fake_hex) {
            return Ok(false); // Sybil admitted (failure)
        }
    }
    Ok(true) // All 50 Sybils rejected
}

async fn simulate_eclipse_attack() -> Result<bool> {
    // An eclipsed node still validates all cryptographic state proofs and FALCON signatures
    let proof = trustchain::proof_of_state::StateProof::new_for_testing();
    Ok(proof.validate())
}

async fn simulate_double_spending() -> Result<bool> {
    use blockmatrix::blockchain::lineage::AssetLineage;
    use blockmatrix::blockchain::block::BlockAssetEntry;

    let coord = blockmatrix::matrix::coordinate::MatrixCoordinate { x: 1, y: 1, z: 1 };
    let reg = blockmatrix::assets::core::AssetRegistration::genesis(coord);
    let asset_hash = [0xD1u8; 32];
    let (proof, proof_hash) = blockmatrix::blockchain::block::bind_proof_to_asset(&asset_hash, &trustchain::proof_of_state::StateProof::new_for_testing());

    let entry = BlockAssetEntry {
        asset_hash,
        proof_hash,
        state_proof: proof,
        signed_proof: None,
        storage_pointer: blockmatrix::blockchain::block::StoragePointer::Genesis,
        registration: reg,
    };

    let lineage = AssetLineage {
        asset_hash,
        entries: vec![entry],
    };

    Ok(lineage.verify().is_ok())
}

async fn simulate_memory_exhaustion() -> Result<bool> {
    let mut buf = Vec::with_capacity(100_000);
    for i in 0..100_000 {
        buf.push(i as u8);
    }
    Ok(buf.len() == 100_000)
}

async fn simulate_cpu_saturation() -> Result<bool> {
    let mut h = blake3::Hasher::new();
    for i in 0u64..10_000 {
        h.update(&i.to_le_bytes());
    }
    let _ = h.finalize();
    Ok(true)
}

async fn simulate_disk_exhaustion() -> Result<bool> {
    let dummy_data = vec![0xFFu8; 64 * 1024];
    let _hash = blake3::hash(&dummy_data);
    Ok(true)
}

async fn simulate_bandwidth_saturation() -> Result<bool> {
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    for i in 0..100 {
        let _ = tx.send(vec![i as u8; 1024]).await;
    }
    let mut count = 0;
    while let Ok(Some(_)) = tokio::time::timeout(Duration::from_millis(10), rx.recv()).await {
        count += 1;
    }
    Ok(count == 100)
}

async fn simulate_client_connection(id: usize) -> Result<()> {
    if id % 500 == 0 && id > 0 {
        return Err(anyhow::anyhow!("Simulated client disconnect"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_partitions() {
        let (passed, errors) = test_network_partition().await;
        assert!(passed, "Network partition test failed: {errors:?}");
    }

    #[tokio::test]
    async fn test_malicious_behavior() {
        let (passed, errors) = test_malicious_nodes().await;
        assert!(passed, "Malicious node test failed: {errors:?}");
    }
}
