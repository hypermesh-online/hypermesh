// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Gate 2: Asset System Integration Test
//!
//! Verifies the asset management system and the CANONICAL four-proof model:
//! PoStake = authorization (WHO), PoSpace = location (WHERE), PoWork = the
//! hash of work done (WHAT), PoTime = temporal (WHEN). No proof carries a
//! magnitude, and no magnitude gates admission.

use blockmatrix::assets::adapters::AdapterRegistry;
use blockmatrix::assets::core::{
    AssetType, SpaceProof, StakeProof, StateProof, TimeProof, WorkProof,
};

use std::time::Duration;

/// Build a canonical four-proof state proof.
///
/// Every field answers a question; none expresses a magnitude that gates.
/// `total_storage` is supplied only because capacity is a DESCRIPTIVE
/// attribute of the node — it is never an admission criterion.
fn canonical_state_proof() -> StateProof {
    // PoStake: WHO — binds an authorized identity. No stake amount exists.
    let stake_proof = StakeProof::new("test-holder".to_string(), "holder-id-123".to_string());

    // PoSpace: WHERE — binds a node and a storage path.
    let mut space_proof = SpaceProof::new(
        "test-node-001".to_string(),
        "/test/storage/path".to_string(),
        10 * 1024 * 1024,
    );
    space_proof.file_hash =
        "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2".to_string();

    // PoWork: WHAT — the hash of the work actually done. No difficulty target,
    // no nonce, no mining.
    let work_proof = WorkProof::from_work(
        "test-owner".to_string(),
        "workload-gate2-test".to_string(),
        b"gate2-work-material",
    );

    // PoTime: WHEN — temporal freshness.
    let time_proof = TimeProof::new(Duration::from_secs(1));

    StateProof::new(stake_proof, time_proof, space_proof, work_proof)
}

#[tokio::test]
async fn test_gate2_adapter_registry_complete() {
    let registry = AdapterRegistry::new().await;

    for asset_type in [
        AssetType::Cpu,
        AssetType::Gpu,
        AssetType::Memory,
        AssetType::Storage,
        AssetType::Network,
        AssetType::Container,
    ] {
        assert!(
            registry.get_adapter(&asset_type).is_some(),
            "adapter for {asset_type:?} must be registered"
        );
    }

    assert!(registry.get_all_adapters().len() >= 6);
}

#[test]
fn test_gate2_canonical_four_proof_validates() {
    let proof = canonical_state_proof();

    assert!(
        !proof.stake_proof.stake_holder_id.is_empty(),
        "PoStake must bind an identity (WHO)"
    );
    assert!(
        !proof.space_proof.node_id.is_empty() && !proof.space_proof.storage_path.is_empty(),
        "PoSpace must bind a location (WHERE)"
    );
    assert!(
        proof.work_proof.work_hash != [0u8; 32],
        "PoWork must carry a real work hash (WHAT)"
    );

    assert!(proof.validate(), "canonical four-proof must validate");
}

/// A zero-capacity proof — a freshly-provisioned node storing nothing — must
/// still be ADMITTED through the adapter path. Capacity is descriptive and
/// must NEVER gate admission.
#[tokio::test]
async fn test_gate2_zero_capacity_is_admitted_by_adapters() {
    let mut proof = canonical_state_proof();
    proof.space_proof.total_size = 0;
    proof.space_proof.total_storage = 0;

    let registry = AdapterRegistry::new().await;

    for asset_type in [
        AssetType::Storage,
        AssetType::Network,
        AssetType::Container,
    ] {
        let adapter = registry
            .get_adapter(&asset_type)
            .expect("test: adapter must exist");
        let admitted = adapter
            .validate_state_proof(&proof)
            .await
            .expect("test: adapter validation should not error");
        assert!(
            admitted,
            "{asset_type:?} adapter must ADMIT a zero-capacity, location-bound \
             proof — capacity is descriptive and must never gate admission"
        );
    }
}
