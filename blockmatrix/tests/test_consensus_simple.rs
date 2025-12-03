//! Simple consensus validation test to demonstrate working implementation

use blockmatrix::consensus::{
    validation::{ConsensusValidator, DefaultConsensusValidator},
};

#[tokio::test]
async fn test_empty_proof_returns_false() {
    let validator = DefaultConsensusValidator::new();
    let result = validator.validate(&[]).await.unwrap();
    assert_eq!(result, false, "Empty proof should be rejected");
    println!("✓ Empty proof correctly rejected");
}

#[tokio::test]
async fn test_invalid_proof_returns_error() {
    let validator = DefaultConsensusValidator::new();
    let garbage = vec![0xFF, 0xDE, 0xAD, 0xBE, 0xEF];
    let result = validator.validate(&garbage).await;
    assert!(result.is_err(), "Malformed proof should return error");
    println!("✓ Malformed proof correctly causes error");
}

#[tokio::test]
async fn test_oversized_proof_rejected() {
    let validator = DefaultConsensusValidator::new();
    let huge = vec![0u8; 2 * 1024 * 1024]; // 2MB
    let result = validator.validate(&huge).await.unwrap();
    assert_eq!(result, false, "Oversized proof should be rejected");
    println!("✓ Oversized proof correctly rejected");
}