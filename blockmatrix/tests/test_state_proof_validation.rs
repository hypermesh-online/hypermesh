// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Comprehensive tests for Proof of State validation.
//!
//! CANONICAL MODEL: the four proofs answer WHO (PoStake = authorization /
//! identity binding), WHEN (PoTime = temporal freshness), WHERE (PoSpace =
//! location), WHAT (PoWork = hash of work done). Validation is binary
//! pass/fail. There is NO magnitude anywhere — no stake amount, no difficulty,
//! no minimum storage/compute. A proof fails when a binding is missing or
//! malformed, never when a number is "too small".

use blockmatrix::proof_of_state::{
    validation::{DefaultStateAuthenticator, StateAuthenticator},
    StateProof, StateRequirements,
};
use hypermesh_lib::proof::{SpaceProof, StakeProof, TimeProof, WorkProof};
use std::time::Duration;

/// Helper: assert a proof is REJECTED. Rejection may surface as `Ok(false)`
/// (requirements not met) or `Err` (comprehensive validation failed) — never as
/// acceptance. It is never a magnitude comparison.
fn assert_rejected(result: anyhow::Result<bool>, context: &str) {
    match result {
        Ok(valid) => assert!(!valid, "{context}: proof must be rejected, not accepted"),
        Err(_) => {} // comprehensive validation failed — a valid rejection
    }
}

/// Helper: a valid canonical proof — bound identity (WHO), a work hash (WHAT),
/// a storage location (WHERE), and a fresh timestamp (WHEN).
fn create_valid_test_proof() -> StateProof {
    let mut space = SpaceProof::new(
        "test_node_001".to_string(),
        "/mnt/storage".to_string(),
        2 * 1024 * 1024 * 1024, // descriptive capacity, never a gate
    );
    space.file_hash = "storage_commitment_hash".to_string();

    // A small offset keeps the proof within any reasonable freshness window; the
    // TimeProof's internal hash is self-consistent as constructed (do not mutate).
    let time = TimeProof::new(Duration::from_secs(1));

    StateProof::new(
        StakeProof::new("test-holder".to_string(), "test_node_001".to_string()),
        time,
        space,
        WorkProof::from_work(
            "test_node_001".to_string(),
            "workload-001".to_string(),
            b"registration work",
        ),
    )
}

/// A proof whose PoStake carries no bound identity (invalid WHO) — never a
/// "too little stake" magnitude.
fn create_unauthorized_proof() -> StateProof {
    let mut proof = create_valid_test_proof();
    proof.stake_proof.stake_holder_id = String::new();
    proof
}

/// A proof whose PoTime is stale (WHEN freshness bound exceeded).
fn create_stale_time_proof() -> StateProof {
    let mut proof = create_valid_test_proof();
    proof.time_proof.network_time_offset = Duration::from_secs(500);
    proof
}

/// A proof whose PoSpace carries no bound location (invalid WHERE) — the WHERE
/// binding is the node identity + location, never a "too little storage"
/// magnitude. Clearing the bound node id makes the location proof invalid.
fn create_no_location_proof() -> StateProof {
    let mut proof = create_valid_test_proof();
    proof.space_proof.node_id = String::new();
    proof.space_proof.storage_path = String::new();
    proof.space_proof.file_hash = String::new();
    proof
}

/// A proof whose PoWork carries no work hash (invalid WHAT) — never a "too
/// little compute" magnitude.
fn create_no_work_hash_proof() -> StateProof {
    let mut proof = create_valid_test_proof();
    proof.work_proof.work_hash = [0u8; 32];
    proof
}

#[tokio::test]
async fn test_valid_proof_validation() {
    let validator = DefaultStateAuthenticator::for_testing();
    let proof = create_valid_test_proof();
    let proof_bytes = proof.to_bytes().expect("test: serialize");

    let result = validator.validate(&proof_bytes).await;
    assert!(result.is_ok(), "Valid proof should not error");
    assert!(result.expect("test: ok"), "Valid proof should pass");
}

#[tokio::test]
async fn test_empty_proof_rejection() {
    let validator = DefaultStateAuthenticator::new();
    let result = validator.validate(&[]).await;

    assert!(result.is_ok(), "Empty proof should not error");
    assert!(!result.expect("test: ok"), "Empty proof should be rejected");
}

#[tokio::test]
async fn test_malformed_proof_rejection() {
    let validator = DefaultStateAuthenticator::new();
    let garbage = vec![0xFF, 0xDE, 0xAD, 0xBE, 0xEF];

    let result = validator.validate(&garbage).await;
    // Malformed bytes either error or are rejected — never accepted.
    match result {
        Ok(valid) => assert!(!valid, "Malformed proof must not be accepted"),
        Err(_) => {}
    }
}

#[tokio::test]
async fn test_unauthorized_identity_rejection() {
    let validator = DefaultStateAuthenticator::for_testing();
    let proof = create_unauthorized_proof();
    let proof_bytes = proof.to_bytes().expect("test: serialize");

    let result = validator.validate(&proof_bytes).await;
    assert_rejected(result, "no bound identity (WHO)");
}

#[tokio::test]
async fn test_excessive_time_offset_rejection() {
    let validator = DefaultStateAuthenticator::for_testing();
    let proof = create_stale_time_proof();
    let proof_bytes = proof.to_bytes().expect("test: serialize");

    let result = validator.validate(&proof_bytes).await;
    assert_rejected(result, "stale WHEN");
}

#[tokio::test]
async fn test_missing_location_rejection() {
    let validator = DefaultStateAuthenticator::for_testing();
    let proof = create_no_location_proof();
    let proof_bytes = proof.to_bytes().expect("test: serialize");

    let result = validator.validate(&proof_bytes).await;
    assert_rejected(result, "no bound location (WHERE)");
}

#[tokio::test]
async fn test_missing_work_hash_rejection() {
    let validator = DefaultStateAuthenticator::for_testing();
    let proof = create_no_work_hash_proof();
    let proof_bytes = proof.to_bytes().expect("test: serialize");

    let result = validator.validate(&proof_bytes).await;
    assert_rejected(result, "no work hash (WHAT)");
}

#[tokio::test]
async fn test_custom_requirements_time_bound() {
    // The only quantitative requirement is the WHEN-freshness bound — there is
    // no minimum stake / storage / compute to configure.
    let strict_requirements = StateRequirements {
        max_time_offset: Duration::from_nanos(1),
    };

    let validator = DefaultStateAuthenticator::with_requirements(strict_requirements);
    let proof = create_valid_test_proof(); // 1s offset exceeds the 1ns bound
    let proof_bytes = proof.to_bytes().expect("test: serialize");

    let result = validator.validate(&proof_bytes).await;
    assert_rejected(result, "strict WHEN-freshness bound");
}

#[tokio::test]
async fn test_lenient_requirements_accept() {
    let lenient_requirements = StateRequirements {
        max_time_offset: Duration::from_secs(3600),
    };

    let validator = DefaultStateAuthenticator::for_testing();
    let proof = create_valid_test_proof();
    let proof_bytes = proof.to_bytes().expect("test: serialize");

    let result = validator
        .validate_with_requirements(&proof_bytes, &lenient_requirements)
        .await;
    assert!(result.is_ok(), "Should not error");
    assert!(
        result.expect("test: ok"),
        "Valid proof should pass a lenient WHEN-freshness bound"
    );
}

#[tokio::test]
async fn test_all_four_proofs_required() {
    let validator = DefaultStateAuthenticator::for_testing();

    // Each proof answers one question; missing any binding fails validation.
    let cases = [
        (create_unauthorized_proof(), "WHO"),
        (create_stale_time_proof(), "WHEN"),
        (create_no_location_proof(), "WHERE"),
        (create_no_work_hash_proof(), "WHAT"),
    ];

    for (proof, which) in cases {
        let proof_bytes = proof.to_bytes().expect("test: serialize");
        let result = validator.validate(&proof_bytes).await;
        assert_rejected(result, which);
    }
}

#[test]
fn test_state_requirements_defaults_have_no_magnitude() {
    // The canonical StateRequirements carries only a WHEN-freshness bound.
    let default = StateRequirements::default();
    assert!(default.max_time_offset > Duration::ZERO);

    let prod = StateRequirements::production();
    assert!(prod.max_time_offset > Duration::ZERO);

    let localhost = StateRequirements::localhost_testing();
    assert!(localhost.max_time_offset > Duration::ZERO);
}
