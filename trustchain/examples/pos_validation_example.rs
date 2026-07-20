// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Proof of State Validation Example
//!
//! This example demonstrates the complete four-proof PoS validation system
//! with detailed error reporting and BlockMatrix AssetId integration.

use std::time::Duration;
use trustchain::proof_of_state::{AssetProofRequirements, AssetValidationContext, StateProof};
use trustchain::proof_of_state::StateProofOps;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Proof of State Validation Examples ===\n");

    // Example 1: Generate and validate full state proof
    example_1_full_validation().await?;

    // Example 2: Validate with minimum requirements
    example_2_minimum_requirements().await?;

    // Example 3: Asset-specific validation (BlockMatrix integration)
    example_3_asset_validation().await?;

    // Example 4: Partial proof requirements
    example_4_partial_requirements().await?;

    Ok(())
}

/// Example 1: Full four-proof validation
async fn example_1_full_validation() -> anyhow::Result<()> {
    println!("Example 1: Full Four-Proof Validation");
    println!("--------------------------------------");

    // Generate state proof from network (in production)
    // let proof = StateProof::generate_from_network("node-001").await?;

    // For this example, use test proof
    let proof = StateProof::new_for_testing();

    // Validate all four proofs
    let validation = proof.verify_all()?;

    println!("PoSpace (WHERE) valid: {}", validation.space_valid);
    println!("PoStake (WHO) valid: {}", validation.stake_valid);
    println!("PoWork (WHAT/HOW) valid: {}", validation.work_valid);
    println!("PoTime (WHEN) valid: {}", validation.time_valid);
    println!("Overall valid: {}", validation.all_valid);
    println!("Proofs passed: {}/4", validation.proofs_passed());

    if !validation.errors.is_empty() {
        println!("\nValidation errors:");
        for error in &validation.errors {
            println!("  - {:?}: {}", error.proof_type, error.error_message);
        }
    }

    println!("\n");
    Ok(())
}

/// Example 2: Validation with minimum requirements
async fn example_2_minimum_requirements() -> anyhow::Result<()> {
    println!("Example 2: Validation with Minimum Requirements");
    println!("-----------------------------------------------");

    let proof = StateProof::new_for_testing();

    // Validate with a time-freshness bound only. PoStake is authorization (no
    // magnitude), PoWork is a work hash (no compute magnitude), and PoSpace is
    // a location (capacity is descriptive) — so there is no minimum-anything
    // gate; the only bound is the WHEN proof's freshness.
    let validation = proof.verify_with_requirements(
        Duration::from_secs(60), // max_time_offset (60 seconds)
    )?;

    println!("Validation result: {}", validation.all_valid);
    println!("Proofs passed: {}/4", validation.proofs_passed());

    if !validation.all_valid {
        println!("Failed requirements: {}", validation.error_summary());
    }

    println!("\n");
    Ok(())
}

/// Example 3: Asset-specific validation (BlockMatrix integration)
async fn example_3_asset_validation() -> anyhow::Result<()> {
    println!("Example 3: Asset-Specific Validation (BlockMatrix)");
    println!("--------------------------------------------------");

    let proof = StateProof::new_for_testing();

    // Create asset validation context. All four proofs are required to be
    // present and self-consistent — there are no minimum magnitude gates.
    let context = AssetValidationContext::new(
        "asset_cpu_core_001".to_string(),
        AssetProofRequirements::all(), // All 4 proofs required
    );

    // Validate for asset
    let validation = proof.validate_for_asset(&context)?;

    println!("Asset ID: {}", context.asset_id);
    println!("All proofs valid: {}", validation.all_valid);
    println!("Proofs passed: {}/4", validation.proofs_passed());

    if validation.all_valid {
        println!("✅ Asset requirements satisfied");
    } else {
        println!("❌ Asset requirements not met:");
        println!("   {}", validation.error_summary());
    }

    println!("\n");
    Ok(())
}

/// Example 4: Partial proof requirements
async fn example_4_partial_requirements() -> anyhow::Result<()> {
    println!("Example 4: Partial Proof Requirements");
    println!("--------------------------------------");

    let proof = StateProof::new_for_testing();

    // Asset requires only stake and space (not work/time)
    let requirements = AssetProofRequirements::custom(
        true,  // require_space
        true,  // require_stake
        false, // require_work (not needed)
        false, // require_time (not needed)
    );

    let context = AssetValidationContext::new("asset_storage_001".to_string(), requirements);

    let validation = proof.validate_for_asset(&context)?;

    println!("Asset: Storage-only (no work/time required)");
    println!("Space valid: {}", validation.space_valid);
    println!("Stake valid: {}", validation.stake_valid);
    println!("Work valid (not required): {}", validation.work_valid);
    println!("Time valid (not required): {}", validation.time_valid);
    println!("Overall valid: {}", validation.all_valid);
    println!(
        "Proofs passed: {}/4 (2 required)",
        validation.proofs_passed()
    );

    println!("\n");
    Ok(())
}
