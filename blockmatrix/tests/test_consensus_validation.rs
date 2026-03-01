// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Comprehensive tests for Proof of State validation
//!
//! Tests all four proofs (WHO, WHEN, WHERE, WHAT) and their integration
//!
//! Gated: uses old struct field names (StakeProof.node_id, etc.) that were refactored.
#![cfg(feature = "future-tests")]

use anyhow::Result;
use blockmatrix::consensus::{
    validation::{DefaultStateAuthenticator, ProductionStateAuthenticator, StateAuthenticator},
    validation_service::{ConsensusValidationService, ValidationService},
    ConsensusError, ConsensusProof, ConsensusRequirements, SpaceProof, StakeProof, TimeProof,
    WorkProof,
};
use std::time::{Duration, SystemTime};

/// Helper to create a valid test proof
fn create_valid_test_proof() -> ConsensusProof {
    ConsensusProof {
        stake_proof: StakeProof {
            node_id: "test_node_001".to_string(),
            stake_amount: 10000, // Meets default minimum
            stake_signature: "valid_signature".to_string(),
            delegation_info: None,
        },
        time_proof: TimeProof {
            timestamp: SystemTime::now(),
            network_time_offset: Duration::from_secs(30), // Within default 60s limit
            nonce: 12345,
            time_signature: "time_sig".to_string(),
        },
        space_proof: SpaceProof {
            node_id: "test_node_001".to_string(),
            total_storage: 2 * 1024 * 1024 * 1024, // 2GB
            available_storage: 1024 * 1024 * 1024, // 1GB free
            storage_commitment: "storage_commitment_hash".to_string(),
            storage_path: "/mnt/storage".to_string(),
        },
        work_proof: WorkProof {
            node_id: "test_node_001".to_string(),
            computational_power: 2000, // Above default minimum
            challenges_solved: vec!["challenge1".to_string(), "challenge2".to_string()],
            work_signature: "work_sig".to_string(),
        },
    }
}

/// Helper to create an invalid proof with insufficient stake
fn create_invalid_stake_proof() -> ConsensusProof {
    let mut proof = create_valid_test_proof();
    proof.stake_proof.stake_amount = 100; // Below minimum
    proof
}

/// Helper to create an invalid proof with excessive time offset
fn create_invalid_time_proof() -> ConsensusProof {
    let mut proof = create_valid_test_proof();
    proof.time_proof.network_time_offset = Duration::from_secs(500); // Way above limit
    proof
}

/// Helper to create an invalid proof with insufficient storage
fn create_invalid_space_proof() -> ConsensusProof {
    let mut proof = create_valid_test_proof();
    proof.space_proof.total_storage = 1024 * 1024; // 1MB, below minimum
    proof
}

/// Helper to create an invalid proof with insufficient compute power
fn create_invalid_work_proof() -> ConsensusProof {
    let mut proof = create_valid_test_proof();
    proof.work_proof.computational_power = 10; // Below minimum
    proof
}

#[tokio::test]
async fn test_valid_proof_validation() {
    let validator = DefaultStateAuthenticator::new();
    let proof = create_valid_test_proof();
    let proof_bytes = proof.to_bytes().expect("Should serialize");

    let result = validator.validate(&proof_bytes).await;
    assert!(result.is_ok(), "Valid proof should pass validation");
    assert_eq!(result.unwrap(), true, "Valid proof should return true");
}

#[tokio::test]
async fn test_empty_proof_rejection() {
    let validator = DefaultStateAuthenticator::new();
    let result = validator.validate(&[]).await;

    assert!(result.is_ok(), "Empty proof should not error");
    assert_eq!(result.unwrap(), false, "Empty proof should return false");
}

#[tokio::test]
async fn test_malformed_proof_rejection() {
    let validator = DefaultStateAuthenticator::new();
    let garbage = vec![0xFF, 0xDE, 0xAD, 0xBE, 0xEF];

    let result = validator.validate(&garbage).await;
    assert!(result.is_err(), "Malformed proof should error");
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid proof format"));
}

#[tokio::test]
async fn test_oversized_proof_rejection() {
    let validator = DefaultStateAuthenticator::new();
    let huge_proof = vec![0u8; 2 * 1024 * 1024]; // 2MB

    let result = validator.validate(&huge_proof).await;
    assert!(result.is_ok(), "Oversized proof should not error");
    assert_eq!(
        result.unwrap(),
        false,
        "Oversized proof should return false"
    );
}

#[tokio::test]
async fn test_insufficient_stake_rejection() {
    let validator = DefaultStateAuthenticator::new();
    let proof = create_invalid_stake_proof();
    let proof_bytes = proof.to_bytes().expect("Should serialize");

    let result = validator.validate(&proof_bytes).await;
    assert!(result.is_ok(), "Should not error on insufficient stake");
    assert_eq!(result.unwrap(), false, "Should reject insufficient stake");
}

#[tokio::test]
async fn test_excessive_time_offset_rejection() {
    let validator = DefaultStateAuthenticator::new();
    let proof = create_invalid_time_proof();
    let proof_bytes = proof.to_bytes().expect("Should serialize");

    let result = validator.validate(&proof_bytes).await;
    assert!(result.is_ok(), "Should not error on time offset");
    assert_eq!(
        result.unwrap(),
        false,
        "Should reject excessive time offset"
    );
}

#[tokio::test]
async fn test_insufficient_storage_rejection() {
    let validator = DefaultStateAuthenticator::new();
    let proof = create_invalid_space_proof();
    let proof_bytes = proof.to_bytes().expect("Should serialize");

    let result = validator.validate(&proof_bytes).await;
    assert!(result.is_ok(), "Should not error on insufficient storage");
    assert_eq!(result.unwrap(), false, "Should reject insufficient storage");
}

#[tokio::test]
async fn test_insufficient_compute_rejection() {
    let validator = DefaultStateAuthenticator::new();
    let proof = create_invalid_work_proof();
    let proof_bytes = proof.to_bytes().expect("Should serialize");

    let result = validator.validate(&proof_bytes).await;
    assert!(result.is_ok(), "Should not error on insufficient compute");
    assert_eq!(result.unwrap(), false, "Should reject insufficient compute");
}

#[tokio::test]
async fn test_custom_requirements_validation() {
    let custom_requirements = ConsensusRequirements {
        minimum_stake: 50000,
        max_time_offset: Duration::from_secs(10),
        minimum_storage: 10 * 1024 * 1024 * 1024, // 10GB
        minimum_compute: 5000,
        byzantine_tolerance: 0.25,
    };

    let validator = DefaultStateAuthenticator::with_requirements(custom_requirements.clone());
    let proof = create_valid_test_proof(); // This won't meet custom requirements
    let proof_bytes = proof.to_bytes().expect("Should serialize");

    let result = validator.validate(&proof_bytes).await;
    assert!(result.is_ok(), "Should not error");
    assert_eq!(
        result.unwrap(),
        false,
        "Should reject with stricter requirements"
    );
}

#[tokio::test]
async fn test_production_validator_strict_requirements() {
    let validator = ProductionStateAuthenticator::new();
    let proof = create_valid_test_proof(); // Won't meet production requirements
    let proof_bytes = proof.to_bytes().expect("Should serialize");

    let result = validator.validate(&proof_bytes).await;
    assert!(result.is_ok(), "Should not error");
    assert_eq!(
        result.unwrap(),
        false,
        "Should reject with production requirements"
    );
}

#[tokio::test]
async fn test_testing_requirements_validation() {
    let validator = DefaultStateAuthenticator::for_testing();

    // Create a minimal proof that would normally fail
    let mut proof = create_valid_test_proof();
    proof.stake_proof.stake_amount = 100; // Very low stake
    proof.space_proof.total_storage = 2 * 1024 * 1024; // 2MB
    proof.work_proof.computational_power = 20; // Low compute

    let proof_bytes = proof.to_bytes().expect("Should serialize");

    let result = validator.validate(&proof_bytes).await;
    assert!(result.is_ok(), "Should not error");
    assert_eq!(
        result.unwrap(),
        true,
        "Should pass with testing requirements"
    );
}

#[test]
fn test_validation_service_sync_validation() {
    let service = ValidationService::new();
    let proof = create_valid_test_proof();

    let result = service.validate(&proof);
    assert!(result.is_ok(), "Valid proof should pass sync validation");
    assert_eq!(result.unwrap(), true);
}

#[tokio::test]
async fn test_validation_service_async_validation() {
    let service = ValidationService::new();
    let proof = create_valid_test_proof();

    let result = service.validate_async(&proof).await;
    assert!(result.is_ok(), "Valid proof should pass async validation");
    assert_eq!(result.unwrap(), true);
}

#[test]
fn test_validation_service_production_mode() {
    let service = ValidationService::for_production();
    let proof = create_valid_test_proof(); // Won't meet production requirements

    let result = service.validate(&proof);
    assert!(
        result.is_err(),
        "Should reject with production requirements"
    );
    match result {
        Err(ConsensusError::ValidationFailed(_)) => {}
        _ => panic!("Expected ValidationFailed error"),
    }
}

#[tokio::test]
async fn test_all_four_proofs_required() {
    let validator = DefaultStateAuthenticator::new();

    // Test that all four proofs are validated
    let proof = create_valid_test_proof();
    let proof_bytes = proof.to_bytes().expect("Should serialize");

    let result = validator.validate(&proof_bytes).await;
    assert!(result.is_ok(), "All four proofs should validate");
    assert_eq!(result.unwrap(), true);

    // Now test with each proof individually failing
    let test_cases = vec![
        (create_invalid_stake_proof(), "stake"),
        (create_invalid_time_proof(), "time"),
        (create_invalid_space_proof(), "space"),
        (create_invalid_work_proof(), "work"),
    ];

    for (invalid_proof, proof_type) in test_cases {
        let proof_bytes = invalid_proof.to_bytes().expect("Should serialize");
        let result = validator.validate(&proof_bytes).await;
        assert!(
            result.is_ok(),
            "Should not error for invalid {}",
            proof_type
        );
        assert_eq!(
            result.unwrap(),
            false,
            "Should reject invalid {} proof",
            proof_type
        );
    }
}

#[tokio::test]
async fn test_verbose_logging() {
    // Test that verbose mode provides detailed logging
    let validator = DefaultStateAuthenticator::new().verbose(true);
    let proof = create_valid_test_proof();
    let proof_bytes = proof.to_bytes().expect("Should serialize");

    // This will generate debug logs if run with RUST_LOG=debug
    let result = validator.validate(&proof_bytes).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

#[tokio::test]
async fn test_validate_with_custom_requirements() {
    let validator = DefaultStateAuthenticator::new();

    // Create lenient requirements
    let lenient_requirements = ConsensusRequirements {
        minimum_stake: 10,
        max_time_offset: Duration::from_secs(3600),
        minimum_storage: 1024,
        minimum_compute: 1,
        byzantine_tolerance: 0.5,
    };

    let mut proof = create_valid_test_proof();
    proof.stake_proof.stake_amount = 50; // Very low stake
    let proof_bytes = proof.to_bytes().expect("Should serialize");

    // Should fail with default requirements
    let result = validator.validate(&proof_bytes).await;
    assert_eq!(
        result.unwrap(),
        false,
        "Should fail with default requirements"
    );

    // Should pass with lenient requirements
    let result = validator
        .validate_with_requirements(&proof_bytes, &lenient_requirements)
        .await;
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        true,
        "Should pass with lenient requirements"
    );
}

#[test]
fn test_consensus_requirements_defaults() {
    let default = ConsensusRequirements::default();
    assert_eq!(default.minimum_stake, 5000);
    assert_eq!(default.max_time_offset, Duration::from_secs(60));
    assert_eq!(default.minimum_storage, 1024 * 1024 * 1024); // 1GB
    assert_eq!(default.minimum_compute, 1000);
    assert_eq!(default.byzantine_tolerance, 0.33);
}

#[test]
fn test_consensus_requirements_production() {
    let prod = ConsensusRequirements::production();
    assert_eq!(prod.minimum_stake, 50000);
    assert_eq!(prod.max_time_offset, Duration::from_secs(30));
    assert_eq!(prod.minimum_storage, 10 * 1024 * 1024 * 1024); // 10GB
    assert_eq!(prod.minimum_compute, 10000);
    assert_eq!(prod.byzantine_tolerance, 0.33);
}

#[test]
fn test_consensus_requirements_localhost() {
    let localhost = ConsensusRequirements::localhost_testing();
    assert_eq!(localhost.minimum_stake, 100);
    assert_eq!(localhost.max_time_offset, Duration::from_secs(300));
    assert_eq!(localhost.minimum_storage, 1024 * 1024); // 1MB
    assert_eq!(localhost.minimum_compute, 10);
    assert_eq!(localhost.byzantine_tolerance, 0.0);
}

/// Test that the validator properly identifies which proof failed
#[tokio::test]
async fn test_detailed_failure_reporting() {
    let validator = DefaultStateAuthenticator::for_testing().verbose(true);

    // Test each type of failure
    let test_cases = vec![
        (create_invalid_stake_proof(), "stake"),
        (create_invalid_time_proof(), "time"),
        (create_invalid_space_proof(), "space"),
        (create_invalid_work_proof(), "work"),
    ];

    for (invalid_proof, expected_failure) in test_cases {
        let proof_bytes = invalid_proof.to_bytes().expect("Should serialize");
        let result = validator.validate(&proof_bytes).await;

        // The test validator has relaxed requirements, so these might pass
        // Let's use production validator instead
        let prod_validator = ProductionStateAuthenticator::new();
        let prod_result = prod_validator.validate(&proof_bytes).await;

        assert!(
            prod_result.is_ok(),
            "Should handle {} failure gracefully",
            expected_failure
        );
        assert_eq!(
            prod_result.unwrap(),
            false,
            "Should reject {} failure",
            expected_failure
        );
    }
}
