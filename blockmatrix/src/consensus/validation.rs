// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Consensus validation module
//!
//! Provides real Proof of State consensus validation for HyperMesh operations.
//! Integrates with TrustChain's four-proof consensus system (WHO, WHEN, WHERE, WHAT).

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use trustchain::consensus::{ConsensusProof, ConsensusRequirements};
use tracing::{debug, warn, error, info};
use std::time::Instant;

/// Consensus validator trait
#[async_trait]
pub trait ConsensusValidator: Send + Sync {
    /// Validate a consensus proof
    async fn validate(&self, proof: &[u8]) -> Result<bool>;

    /// Validate with specific requirements
    async fn validate_with_requirements(&self, proof: &[u8], requirements: &ConsensusRequirements) -> Result<bool>;

    /// Get validator name
    fn name(&self) -> &str;
}

/// Default consensus validator implementation using TrustChain's Proof of State
pub struct DefaultConsensusValidator {
    /// Requirements for consensus validation
    requirements: ConsensusRequirements,
    /// Enable detailed validation logging
    verbose: bool,
}

impl DefaultConsensusValidator {
    /// Create new default validator with production requirements
    pub fn new() -> Self {
        Self {
            requirements: ConsensusRequirements::default(),
            verbose: false,
        }
    }

    /// Create validator with custom requirements
    pub fn with_requirements(requirements: ConsensusRequirements) -> Self {
        Self {
            requirements,
            verbose: false,
        }
    }

    /// Create validator for testing with relaxed requirements
    pub fn for_testing() -> Self {
        Self {
            requirements: ConsensusRequirements::localhost_testing(),
            verbose: true,
        }
    }

    /// Enable verbose logging for debugging
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
}

#[async_trait]
impl ConsensusValidator for DefaultConsensusValidator {
    async fn validate(&self, proof: &[u8]) -> Result<bool> {
        // Start validation timer
        let start = Instant::now();

        // Basic validation - check proof is not empty
        if proof.is_empty() {
            warn!("Consensus validation failed: empty proof provided");
            return Ok(false);
        }

        // Check proof size (reasonable bounds)
        if proof.len() > 1024 * 1024 {  // 1MB max
            warn!("Consensus validation failed: proof size {} exceeds maximum", proof.len());
            return Ok(false);
        }

        // Deserialize the ConsensusProof from bytes
        let consensus_proof = match ConsensusProof::from_bytes(proof) {
            Ok(p) => {
                if self.verbose {
                    debug!("Successfully deserialized consensus proof");
                }
                p
            },
            Err(e) => {
                error!("Failed to deserialize consensus proof: {}", e);
                return Err(anyhow!("Invalid proof format: {}", e));
            }
        };

        // Perform comprehensive validation with detailed error reporting
        match consensus_proof.validate_comprehensive().await {
            Ok(true) => {
                // Now validate against specific requirements
                if consensus_proof.validate_with_requirements(&self.requirements) {
                    let duration = start.elapsed();
                    info!(
                        "Consensus validation successful for {} ({}ms)",
                        self.name(),
                        duration.as_millis()
                    );

                    if self.verbose {
                        debug!("✓ Stake proof: {} tokens validated", consensus_proof.stake_proof.stake_amount);
                        debug!("✓ Time proof: offset {} ms", consensus_proof.time_proof.network_time_offset.as_millis());
                        debug!("✓ Space proof: {} GB storage", consensus_proof.space_proof.total_storage / (1024 * 1024 * 1024));
                        debug!("✓ Work proof: {} compute units", consensus_proof.work_proof.computational_power);
                    }

                    Ok(true)
                } else {
                    // Check which requirement failed
                    let mut failed_requirements = Vec::new();

                    if consensus_proof.stake_proof.stake_amount < self.requirements.minimum_stake {
                        failed_requirements.push(format!(
                            "Insufficient stake: {} < {}",
                            consensus_proof.stake_proof.stake_amount,
                            self.requirements.minimum_stake
                        ));
                    }

                    if consensus_proof.time_proof.network_time_offset > self.requirements.max_time_offset {
                        failed_requirements.push(format!(
                            "Time offset too large: {:?} > {:?}",
                            consensus_proof.time_proof.network_time_offset,
                            self.requirements.max_time_offset
                        ));
                    }

                    if consensus_proof.space_proof.total_storage < self.requirements.minimum_storage {
                        failed_requirements.push(format!(
                            "Insufficient storage: {} < {}",
                            consensus_proof.space_proof.total_storage,
                            self.requirements.minimum_storage
                        ));
                    }

                    if consensus_proof.work_proof.computational_power < self.requirements.minimum_compute {
                        failed_requirements.push(format!(
                            "Insufficient compute power: {} < {}",
                            consensus_proof.work_proof.computational_power,
                            self.requirements.minimum_compute
                        ));
                    }

                    warn!(
                        "Consensus validation failed requirements check: {}",
                        failed_requirements.join(", ")
                    );

                    Ok(false)
                }
            },
            Ok(false) => {
                warn!("Consensus validation failed: proof validation returned false");
                Ok(false)
            },
            Err(e) => {
                error!("Consensus validation failed with error: {}", e);

                // Provide detailed failure information
                if e.to_string().contains("Stake proof") {
                    warn!("✗ Stake proof (WHO) validation failed");
                }
                if e.to_string().contains("Time proof") {
                    warn!("✗ Time proof (WHEN) validation failed");
                }
                if e.to_string().contains("Space proof") {
                    warn!("✗ Space proof (WHERE) validation failed");
                }
                if e.to_string().contains("Work proof") {
                    warn!("✗ Work proof (WHAT) validation failed");
                }

                Err(anyhow!("Consensus validation failed: {}", e))
            }
        }
    }

    async fn validate_with_requirements(&self, proof: &[u8], requirements: &ConsensusRequirements) -> Result<bool> {
        // Create a temporary validator with custom requirements
        let validator = DefaultConsensusValidator::with_requirements(requirements.clone())
            .verbose(self.verbose);
        validator.validate(proof).await
    }

    fn name(&self) -> &str {
        "proof-of-state"
    }
}

/// Production consensus validator with strict requirements
pub struct ProductionConsensusValidator {
    inner: DefaultConsensusValidator,
}

impl ProductionConsensusValidator {
    pub fn new() -> Self {
        Self {
            inner: DefaultConsensusValidator::with_requirements(ConsensusRequirements::production()),
        }
    }
}

#[async_trait]
impl ConsensusValidator for ProductionConsensusValidator {
    async fn validate(&self, proof: &[u8]) -> Result<bool> {
        self.inner.validate(proof).await
    }

    async fn validate_with_requirements(&self, proof: &[u8], requirements: &ConsensusRequirements) -> Result<bool> {
        self.inner.validate_with_requirements(proof, requirements).await
    }

    fn name(&self) -> &str {
        "production-proof-of-state"
    }
}

/// Export the trait
pub use ConsensusValidator as Validator;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_empty_proof_validation() {
        let validator = DefaultConsensusValidator::new();
        let result = validator.validate(&[]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
    }

    #[tokio::test]
    async fn test_oversized_proof_validation() {
        let validator = DefaultConsensusValidator::new();
        let huge_proof = vec![0u8; 2 * 1024 * 1024]; // 2MB
        let result = validator.validate(&huge_proof).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
    }

    #[tokio::test]
    async fn test_malformed_proof_validation() {
        let validator = DefaultConsensusValidator::new();
        let malformed = vec![0xFF, 0xBA, 0xDC, 0x0D, 0xE5];
        let result = validator.validate(&malformed).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid proof format"));
    }
}