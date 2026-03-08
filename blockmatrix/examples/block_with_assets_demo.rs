// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Block-MATRIX: Blocks Contain Asset Entries Demonstration
//!
//! Each block contains `BlockAssetEntry` items — self-contained records
//! with content hash, proof hash, state proof, storage pointer, and
//! asset registration.  Timestamp and node coordinate are NOT block
//! fields; they live inside the state proof (PoTime/PoSpace).
//!
//! Key Concepts Demonstrated:
//! 1. Blocks enforce non-empty entries (cannot create empty blocks)
//! 2. Each entry carries its own state proof
//! 3. Multiple entries can exist in a single block
//! 4. Different asset types can coexist in blocks
//! 5. Ledger secures integrity; storage layer holds data

use blockmatrix::assets::core::{
    AssetCategory, AssetData, AssetRegistration, BaseSystemType, NetworkScope,
};
use blockmatrix::blockchain::block::{Block, BlockAssetEntry, StoragePointer};
use blockmatrix::matrix::coordinate::MatrixCoordinate;
use trustchain::proof_of_state::StateProof;

/// Build a `BlockAssetEntry` from an `AssetRegistration` and a `StateProof`.
fn make_entry(reg: AssetRegistration, proof: &StateProof) -> BlockAssetEntry {
    let asset_hash = reg.content_hash;
    let proof_bytes = serde_json::to_vec(proof).unwrap_or_default();
    let proof_hash = *blake3::hash(&proof_bytes).as_bytes();

    BlockAssetEntry {
        asset_hash,
        proof_hash,
        state_proof: proof.clone(),
        storage_pointer: StoragePointer::Genesis,
        registration: reg,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║  Block-MATRIX: Blocks Contain Asset Entries Demonstration        ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝\n");

    // PART 1: Matrix Coordinate
    println!("📍 PART 1: Creating Matrix Coordinate for Node Position\n");
    let node_coord = MatrixCoordinate::new(10, 20, 0)?;
    println!(
        "   ✅ Node Coordinate: ({}, {}, {})\n",
        node_coord.x, node_coord.y, node_coord.z
    );

    // PART 2: Creating Assets
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("🎯 PART 2: Creating Multiple Assets\n");

    let proof = StateProof::default();

    let genesis_asset = AssetRegistration::genesis(node_coord);
    println!("   ✅ Asset 1 (Genesis): {genesis_asset}");

    let cpu_data = AssetData {
        config: format!(
            "CPU asset for node at ({}, {}, {})",
            node_coord.x, node_coord.y, node_coord.z
        )
        .into_bytes(),
        definition: b"CPU_RESOURCE".to_vec(),
        metadata: b"8-core processor".to_vec(),
    };
    let cpu_asset = AssetRegistration::from_asset_data(
        &cpu_data,
        NetworkScope::Global,
        AssetCategory::BaseSystem(BaseSystemType::Cpu),
    );
    println!("   ✅ Asset 2 (CPU): {cpu_asset}");

    let memory_data = AssetData {
        config: format!(
            "Memory asset for node at ({}, {}, {})",
            node_coord.x, node_coord.y, node_coord.z
        )
        .into_bytes(),
        definition: b"MEMORY_RESOURCE".to_vec(),
        metadata: b"16GB RAM".to_vec(),
    };
    let memory_asset = AssetRegistration::from_asset_data(
        &memory_data,
        NetworkScope::Global,
        AssetCategory::BaseSystem(BaseSystemType::Memory),
    );
    println!("   ✅ Asset 3 (Memory): {memory_asset}");

    let storage_data = AssetData {
        config: format!(
            "Storage asset for node at ({}, {}, {})",
            node_coord.x, node_coord.y, node_coord.z
        )
        .into_bytes(),
        definition: b"STORAGE_RESOURCE".to_vec(),
        metadata: b"1TB storage".to_vec(),
    };
    let storage_asset = AssetRegistration::from_asset_data(
        &storage_data,
        NetworkScope::Global,
        AssetCategory::BaseSystem(BaseSystemType::Storage),
    );
    println!("   ✅ Asset 4 (Storage): {storage_asset}\n");

    // PART 3: Creating a Block with Entries
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("🔗 PART 3: Creating Block Containing Multiple Entries\n");

    let entries = vec![
        make_entry(genesis_asset, &proof),
        make_entry(cpu_asset, &proof),
        make_entry(memory_asset, &proof),
        make_entry(storage_asset, &proof),
    ];

    println!("   Creating block #1 with {} entries...", entries.len());
    let block = Block::new(1, entries, "genesis_hash".to_string());
    println!("   ✅ Block created successfully!\n");

    // PART 4: Block Information
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("📊 PART 4: Block Information and Entry Details\n");

    println!("   Block Details:");
    println!("   ├─ Index: #{}", block.index);
    println!(
        "   ├─ Previous Hash: {}...{}",
        &block.previous_hash[..8.min(block.previous_hash.len())],
        &block.previous_hash[block.previous_hash.len().saturating_sub(8)..]
    );
    println!(
        "   ├─ Block Hash: {}...{}",
        &block.hash[..8],
        &block.hash[block.hash.len().saturating_sub(8)..]
    );
    println!("   ├─ Block Size: {} bytes", block.size());
    println!("   └─ Entry Count: {}\n", block.asset_count());

    println!("   Entries in Block:");
    for (idx, asset) in block.get_assets().iter().enumerate() {
        println!("   │");
        println!("   ├─ Entry #{}: {}", idx + 1, asset);
        println!("   │  ├─ Category: {:?}", asset.category);
        println!("   │  ├─ Network Scope: {:?}", asset.network_scope);
        println!(
            "   │  └─ Content Hash: {}...{}",
            hex::encode(&asset.content_hash[..4]),
            hex::encode(&asset.content_hash[28..])
        );
    }
    println!("   └─ [End of Entries]\n");

    // PART 5: Block Validation
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("✅ PART 5: Block Validation\n");

    let hash_valid = block.verify_hash();
    println!(
        "   ├─ Hash integrity: {}",
        if hash_valid { "✅ VALID" } else { "❌ INVALID" }
    );

    let belongs = block.belongs_to_node(&node_coord);
    println!(
        "   └─ Belongs to node: {}\n",
        if belongs { "✅ YES" } else { "❌ NO" }
    );

    // PART 6: Genesis Block
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("🌱 PART 6: Genesis Block\n");

    let genesis_coord = MatrixCoordinate::new(5, 5, 5)?;
    let genesis_block = Block::genesis(genesis_coord);

    println!("   Genesis Block Details:");
    println!("   ├─ Is Genesis: {}", genesis_block.is_genesis());
    println!("   ├─ Index: #{} (always 0)", genesis_block.index);
    println!(
        "   ├─ Entry Count: {} (genesis entry)",
        genesis_block.asset_count()
    );
    println!(
        "   └─ Previous Hash: {} (all zeros)\n",
        genesis_block.previous_hash
    );

    // PART 7: Empty Block Enforcement
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("🚫 PART 7: Enforcement - Blocks MUST Contain Entries\n");

    let result =
        std::panic::catch_unwind(|| Block::new(2, vec![], "prev_hash".to_string()));

    match result {
        Ok(_) => {
            println!("   ❌ ERROR: Block was created without entries!");
        }
        Err(_) => {
            println!("   ✅ SUCCESS: Empty block creation was prevented!");
            println!("   → Assertion: 'Block must contain at least one entry'\n");
        }
    }

    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║  🎉 Demonstration Complete!                                       ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝\n");

    Ok(())
}
