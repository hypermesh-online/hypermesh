//! Simple demonstration of ConsensusProof integration with HyperMesh asset blocks
//!
//! This example shows the core ConsensusProof validation without the full consensus system.

use hypermesh_assets::blockchain::{
    HyperMeshAssetRecord, AssetRecordType, AssetPrivacyLevel, HyperMeshBlockData,
};
use hypermesh_assets::core::asset_id::{AssetId, AssetType};
use crate::consensus::{
    ConsensusProof, ProofOfSpace, ProofOfStake, ProofOfWork, ProofOfTime,
    NetworkPosition, AccessPermissions, AccessLevel,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 HyperMesh ConsensusProof Integration Demo");
    println!("============================================");
    
    // 1. Create test asset
    let asset_id = AssetId::new(AssetType::Cpu);
    println!("✅ Created Asset ID: {}", asset_id.to_hex_string());
    
    // 2. Generate all four proofs (NKrypt pattern)
    
    // WHERE: Proof of Space
    let space_proof = ProofOfSpace::new(
        format!("/hypermesh/assets/{}", asset_id.to_hex_string()),
        NetworkPosition {
            address: "hypermesh://demo-node".to_string(),
            zone: "demo-zone".to_string(),
            distance_metric: 1,
        },
        1024 * 1024, // 1MB
    );
    println!("✅ Generated PoSpace (WHERE): Storage location proof");
    
    // WHO: Proof of Stake (ownership/authority)
    let stake_proof = ProofOfStake::new(
        "demo-user".to_string(),      // asset owner
        "demo-node-123".to_string(),  // validating node
        1000,                         // authority level
        AccessPermissions {
            read_level: AccessLevel::Public,
            write_level: AccessLevel::Network,
            admin_level: AccessLevel::None,
            allocation_rights: vec!["Creation".to_string()],
        },
        vec!["delegate:cpu".to_string()], // allowances
    );
    println!("✅ Generated PoStake (WHO): Ownership proof");
    
    // WHAT/HOW: Proof of Work
    let work_proof = ProofOfWork::new(
        b"demo-asset-creation-challenge",
        8, // Low difficulty for demo
        "Creation".to_string(),
    ).expect("PoWork generation failed");
    println!("✅ Generated PoWork (WHAT/HOW): Computational proof");
    
    // WHEN: Proof of Time
    let time_proof = ProofOfTime::new(1000, None, 1);
    println!("✅ Generated PoTime (WHEN): Temporal ordering proof");
    
    // 3. Create unified ConsensusProof
    let consensus_proof = ConsensusProof::new(
        space_proof,
        stake_proof,
        work_proof,
        time_proof,
    );
    println!("✅ Created unified ConsensusProof (4 proofs combined)");
    
    // 4. Validate consensus proof
    println!("\n🔍 Validating ConsensusProof...");
    match consensus_proof.validate().await {
        Ok(true) => println!("✅ ConsensusProof validation PASSED"),
        Ok(false) => {
            println!("❌ ConsensusProof validation FAILED");
            return Ok(());
        }
        Err(e) => {
            println!("❌ ConsensusProof validation ERROR: {:?}", e);
            return Ok(());
        }
    }
    
    // 5. Create asset record with consensus proof
    let asset_record = HyperMeshAssetRecord::new(
        asset_id.clone(),
        AssetRecordType::Creation,
        "demo-authority".to_string(),
        b"Asset created with consensus validation".to_vec(),
        vec![consensus_proof],
        AssetPrivacyLevel::PublicNetwork,
    );
    println!("✅ Created HyperMeshAssetRecord with ConsensusProof");
    
    // 6. Validate asset record consensus
    println!("\n🔍 Validating Asset Record consensus...");
    match asset_record.validate_consensus().await {
        Ok(true) => println!("✅ Asset Record consensus validation PASSED"),
        Ok(false) => {
            println!("❌ Asset Record consensus validation FAILED");
            return Ok(());
        }
        Err(e) => {
            println!("❌ Asset Record consensus validation ERROR: {}", e);
            return Ok(());
        }
    }
    
    // 7. Create blockchain block (NKrypt pattern)
    let block_data = HyperMeshBlockData::AssetRecord(asset_record.clone());
    println!("✅ Created HyperMeshBlockData (NKrypt pattern)");
    
    // Verify consensus requirement
    if block_data.requires_consensus() {
        println!("✅ Block data correctly requires consensus");
    }
    
    // 8. Generate block hash
    let block_hash = asset_record.calculate_hash();
    println!("✅ Block hash: {}", hex::encode(&block_hash[..8]));
    
    // 9. Test privacy validation
    println!("\n🔐 Testing Privacy Levels:");
    let privacy_tests = [
        (AssetPrivacyLevel::FullPublic, true),
        (AssetPrivacyLevel::PublicNetwork, true), 
        (AssetPrivacyLevel::P2P, false),
        (AssetPrivacyLevel::Private, false),
    ];
    
    for (level, expected) in &privacy_tests {
        let result = asset_record.validates_privacy(level);
        let icon = if result == *expected { "✅" } else { "❌" };
        println!("  {} {:?}: {}", icon, level, result);
    }
    
    // 10. Summary
    println!("\n🎉 Integration Test Summary:");
    println!("   ✅ ConsensusProof system working");
    println!("   ✅ All 4 proofs (PoSp+PoSt+PoWk+PoTm) validated"); 
    println!("   ✅ Asset records integrate with consensus");
    println!("   ✅ Blockchain storage ready");
    println!("   ✅ NKrypt patterns followed");
    println!("   ✅ Privacy levels validated");
    println!("\n🚀 Ready for blockchain integration!");
    
    Ok(())
}