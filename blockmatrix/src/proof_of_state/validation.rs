// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! State proof validation module
//!
//! Provides real Proof of State validation for HyperMesh operations.
//! Integrates with TrustChain's four-proof state proof system (WHO, WHEN, WHERE, WHAT).
//! Each proof is binary pass/fail.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::time::Instant;
use tracing::{debug, error, info, warn};
use trustchain::proof_of_state::{StateProof, StateRequirements};
use trustchain::proof_of_state::StateProofOps;

/// State proof validator trait (binary pass/fail)
#[async_trait]
pub trait StateAuthenticator: Send + Sync {
    /// Validate a state proof
    async fn validate(&self, proof: &[u8]) -> Result<bool>;

    /// Validate with specific requirements
    async fn validate_with_requirements(
        &self,
        proof: &[u8],
        requirements: &StateRequirements,
    ) -> Result<bool>;

    /// Get validator name
    fn name(&self) -> &str;
}

/// Default state authenticator using TrustChain's Proof of State
pub struct DefaultStateAuthenticator {
    /// Requirements for validation
    requirements: StateRequirements,
    /// Enable detailed validation logging
    verbose: bool,
}

impl Default for DefaultStateAuthenticator {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultStateAuthenticator {
    /// Create new default validator with production requirements
    pub fn new() -> Self {
        Self {
            requirements: StateRequirements::default(),
            verbose: false,
        }
    }

    /// Create validator with custom requirements
    pub fn with_requirements(requirements: StateRequirements) -> Self {
        Self {
            requirements,
            verbose: false,
        }
    }

    /// Create validator for testing with relaxed requirements
    pub fn for_testing() -> Self {
        Self {
            requirements: StateRequirements::localhost_testing(),
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
impl StateAuthenticator for DefaultStateAuthenticator {
    async fn validate(&self, proof: &[u8]) -> Result<bool> {
        let start = Instant::now();

        if proof.is_empty() {
            warn!("State proof validation failed: empty proof provided");
            return Ok(false);
        }

        if proof.len() > 1024 * 1024 {
            warn!(
                "State proof validation failed: proof size {} exceeds maximum",
                proof.len()
            );
            return Ok(false);
        }

        let state_proof = match StateProof::from_bytes(proof) {
            Ok(p) => {
                if self.verbose {
                    debug!("Successfully deserialized state proof");
                }
                p
            }
            Err(e) => {
                error!("Failed to deserialize state proof: {}", e);
                return Err(anyhow!("Invalid proof format: {e}"));
            }
        };

        match state_proof.validate_comprehensive().await {
            Ok(true) => {
                if state_proof.validate_with_requirements(&self.requirements) {
                    let duration = start.elapsed();
                    info!(
                        "State proof validation successful for {} ({}ms)",
                        self.name(),
                        duration.as_millis()
                    );

                    if self.verbose {
                        debug!(
                            "Pass: Stake proof: authorized identity {}",
                            state_proof.stake_proof.stake_holder_id
                        );
                        debug!(
                            "Pass: Time proof: offset {} ms",
                            state_proof.time_proof.network_time_offset.as_millis()
                        );
                        debug!(
                            "Pass: Space proof: {} GB storage",
                            state_proof.space_proof.total_storage / (1024 * 1024 * 1024)
                        );
                        debug!(
                            "Pass: Work proof: hash {}",
                            hex::encode(&state_proof.work_proof.work_hash[..4])
                        );
                    }

                    Ok(true)
                } else {
                    let mut failed_requirements = Vec::new();

                    // CANONICAL MODEL: PoStake is authorization (WHO), never a
                    // magnitude — the failure is "no bound identity", not "too
                    // little stake".
                    if state_proof.stake_proof.stake_holder_id.is_empty() {
                        failed_requirements
                            .push("PoStake carries no bound identity".to_string());
                    }

                    if state_proof.time_proof.network_time_offset
                        > self.requirements.max_time_offset
                    {
                        failed_requirements.push(format!(
                            "Time offset too large: {:?} > {:?}",
                            state_proof.time_proof.network_time_offset,
                            self.requirements.max_time_offset
                        ));
                    }

                    // CANONICAL MODEL: PoSpace is WHERE (location) — a required
                    // space proof needs a bound location commitment, never a
                    // minimum storage magnitude.
                    if state_proof.space_proof.file_hash.is_empty()
                        && state_proof.space_proof.storage_path.is_empty()
                    {
                        failed_requirements
                            .push("PoSpace carries no bound location".to_string());
                    }

                    // CANONICAL MODEL: PoWork is the HASH of work done (WHAT),
                    // never a capacity magnitude — the failure is "no work
                    // hash", not "too little compute".
                    if state_proof.work_proof.work_hash == [0u8; 32] {
                        failed_requirements
                            .push("PoWork carries no work hash".to_string());
                    }

                    warn!(
                        "State proof validation failed requirements check: {}",
                        failed_requirements.join(", ")
                    );

                    Ok(false)
                }
            }
            Ok(false) => {
                warn!("State proof validation failed: proof validation returned false");
                Ok(false)
            }
            Err(e) => {
                error!("State proof validation failed with error: {}", e);

                if e.to_string().contains("Stake proof") {
                    warn!("FAIL: Stake proof (WHO) validation failed");
                }
                if e.to_string().contains("Time proof") {
                    warn!("FAIL: Time proof (WHEN) validation failed");
                }
                if e.to_string().contains("Space proof") {
                    warn!("FAIL: Space proof (WHERE) validation failed");
                }
                if e.to_string().contains("Work proof") {
                    warn!("FAIL: Work proof (WHAT) validation failed");
                }

                Err(anyhow!("State proof validation failed: {e}"))
            }
        }
    }

    async fn validate_with_requirements(
        &self,
        proof: &[u8],
        requirements: &StateRequirements,
    ) -> Result<bool> {
        let validator = DefaultStateAuthenticator::with_requirements(requirements.clone())
            .verbose(self.verbose);
        validator.validate(proof).await
    }

    fn name(&self) -> &str {
        "proof-of-state"
    }
}

/// Production state authenticator with strict requirements
pub struct ProductionStateAuthenticator {
    inner: DefaultStateAuthenticator,
}

impl Default for ProductionStateAuthenticator {
    fn default() -> Self {
        Self::new()
    }
}

impl ProductionStateAuthenticator {
    pub fn new() -> Self {
        Self {
            inner: DefaultStateAuthenticator::with_requirements(StateRequirements::production()),
        }
    }
}

#[async_trait]
impl StateAuthenticator for ProductionStateAuthenticator {
    async fn validate(&self, proof: &[u8]) -> Result<bool> {
        self.inner.validate(proof).await
    }

    async fn validate_with_requirements(
        &self,
        proof: &[u8],
        requirements: &StateRequirements,
    ) -> Result<bool> {
        self.inner
            .validate_with_requirements(proof, requirements)
            .await
    }

    fn name(&self) -> &str {
        "production-proof-of-state"
    }
}

/// Export the trait
pub use StateAuthenticator as Validator;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_empty_proof_validation() {
        let validator = DefaultStateAuthenticator::new();
        let result = validator.validate(&[]).await;
        assert!(result.is_ok());
        assert!(!result.expect("test: expected success"));
    }

    #[tokio::test]
    async fn test_oversized_proof_validation() {
        let validator = DefaultStateAuthenticator::new();
        let huge_proof = vec![0u8; 2 * 1024 * 1024]; // 2MB
        let result = validator.validate(&huge_proof).await;
        assert!(result.is_ok());
        assert!(!result.expect("test: expected success"));
    }

    #[tokio::test]
    async fn test_malformed_proof_validation() {
        let validator = DefaultStateAuthenticator::new();
        let malformed = vec![0xFF, 0xBA, 0xDC, 0x0D, 0xE5];
        let result = validator.validate(&malformed).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid proof format"));
    }
}
