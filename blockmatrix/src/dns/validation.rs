// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! DNS Validation with Proof of State
//!
//! PoS-driven DNS access validation using consensus system.

use super::{DnsError, DnsResult, Domain};
use crate::consensus::{ConsensusProof, ConsensusRequirements};
use crate::consensus::validation::{ConsensusValidator, DefaultConsensusValidator};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tracing::{debug, warn};

/// DNS validation result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Validation passed
    pub valid: bool,
    /// Validation reason
    pub reason: Option<String>,
    /// Required proofs that were validated
    pub validated_proofs: Vec<String>,
}

/// DNS validator with PoS integration
pub struct DnsValidator {
    /// Consensus validator
    consensus_validator: Arc<dyn ConsensusValidator>,
    /// Consensus requirements for DNS operations
    requirements: ConsensusRequirements,
    /// Enable strict validation
    _strict_mode: bool,
}

impl DnsValidator {
    /// Create new DNS validator
    ///
    /// # Arguments
    /// * `strict_mode` - If true, uses production-level requirements. If false, uses lenient testing requirements.
    pub fn new(strict_mode: bool) -> Self {
        let requirements = if strict_mode {
            ConsensusRequirements::production()
        } else {
            // Use lenient requirements for testing
            ConsensusRequirements::localhost_testing()
        };

        Self {
            consensus_validator: Arc::new(DefaultConsensusValidator::with_requirements(
                requirements.clone(),
            )),
            requirements,
            _strict_mode: strict_mode,
        }
    }

    /// Validate DNS access with PoS
    pub async fn validate_dns_access(
        &self,
        domain: &Domain,
        proof: &ConsensusProof,
    ) -> DnsResult<ValidationResult> {
        debug!("Validating DNS access for domain: {}", domain.full);

        // Serialize proof for validation
        let proof_bytes = proof
            .to_bytes()
            .map_err(|e| DnsError::ValidationFailed {
                reason: format!("Failed to serialize proof: {}", e),
            })?;

        // Validate consensus proof
        let is_valid = self
            .consensus_validator
            .validate(&proof_bytes)
            .await
            .map_err(|e| DnsError::ValidationFailed {
                reason: format!("Consensus validation failed: {}", e),
            })?;

        if !is_valid {
            warn!("DNS access denied for domain: {}", domain.full);
            return Ok(ValidationResult {
                valid: false,
                reason: Some("Consensus proof validation failed".to_string()),
                validated_proofs: vec![],
            });
        }

        // All proofs validated
        let validated_proofs = vec![
            "PoStake (WHO)".to_string(),
            "PoTime (WHEN)".to_string(),
            "PoSpace (WHERE)".to_string(),
            "PoWork (WHAT)".to_string(),
        ];

        Ok(ValidationResult {
            valid: true,
            reason: None,
            validated_proofs,
        })
    }

    /// Validate DNS registration
    pub async fn validate_registration(
        &self,
        domain: &Domain,
        proof: &ConsensusProof,
    ) -> DnsResult<ValidationResult> {
        debug!("Validating DNS registration for domain: {}", domain.full);

        // Registration requires all four proofs
        if !proof.validate_with_requirements(&self.requirements) {
            return Ok(ValidationResult {
                valid: false,
                reason: Some("Insufficient consensus proofs for registration".to_string()),
                validated_proofs: vec![],
            });
        }

        // Perform full validation
        self.validate_dns_access(domain, proof).await
    }

    /// Validate network access for federated domains
    pub fn validate_network_access(
        &self,
        domain: &Domain,
        requester_network: Option<&str>,
    ) -> DnsResult<bool> {
        // Public domains (no subdomains) are always accessible
        if domain.is_public() {
            return Ok(true);
        }

        // Federated domains require network membership
        match requester_network {
            Some(_network) => {
                // TODO: Validate network membership via blockchain
                // For now, allow if network ID is provided
                Ok(true)
            }
            None => {
                // No network ID provided, cannot access federated domain
                Ok(false)
            }
        }
    }

    /// Check if domain requires full federation
    pub fn is_fully_federated(&self, domain: &Domain) -> bool {
        // Check if domain is marked as fully federated
        // TODO: Query blockchain for federation status
        // For now, use heuristic: if has multiple subdomain levels
        domain.subdomains.len() >= 2
    }
}

impl Default for DnsValidator {
    fn default() -> Self {
        Self::new(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::proof_of_state_integration::{
        SpaceProof, StakeProof, TimeProof, WorkProof, WorkState, WorkloadType,
    };
    use std::time::Duration;

    fn create_test_proof() -> ConsensusProof {
        let stake = StakeProof::new("holder".to_string(), "holder-id".to_string(), 1000);
        let time = TimeProof::new(Duration::from_secs(10));
        let space = SpaceProof::new("node".to_string(), "/storage".to_string(), 1024 * 1024);
        let work = WorkProof::new(
            "owner".to_string(),
            "workload".to_string(),
            12345,
            100,
            WorkloadType::Compute,
            WorkState::Completed,
        );

        ConsensusProof::new(stake, time, space, work)
    }

    #[tokio::test]
    async fn test_dns_access_validation() {
        let validator = DnsValidator::new(false); // Non-strict for testing
        let domain = Domain::parse("nike").unwrap();
        let proof = create_test_proof();

        let result = validator.validate_dns_access(&domain, &proof).await.unwrap();
        assert!(result.valid);
        assert_eq!(result.validated_proofs.len(), 4);
    }

    #[tokio::test]
    async fn test_registration_validation() {
        let validator = DnsValidator::new(false);
        let domain = Domain::parse("nike").unwrap();
        let proof = create_test_proof();

        let result = validator
            .validate_registration(&domain, &proof)
            .await
            .unwrap();
        assert!(result.valid);
    }

    #[test]
    fn test_network_access_validation() {
        let validator = DnsValidator::new(true);

        // Public domain accessible
        let domain = Domain::parse("nike").unwrap();
        assert!(validator
            .validate_network_access(&domain, None)
            .unwrap());

        // Federated domain requires network
        let domain = Domain::parse("admin.nike").unwrap();
        assert!(validator
            .validate_network_access(&domain, Some("nike-internal"))
            .unwrap());
        assert!(!validator
            .validate_network_access(&domain, None)
            .unwrap());
    }

    #[test]
    fn test_fully_federated_check() {
        let validator = DnsValidator::new(true);

        let domain = Domain::parse("nike").unwrap();
        assert!(!validator.is_fully_federated(&domain));

        let domain = Domain::parse("admin.nike").unwrap();
        assert!(!validator.is_fully_federated(&domain));

        let domain = Domain::parse("classified.internal.gov").unwrap();
        assert!(validator.is_fully_federated(&domain));
    }
}
