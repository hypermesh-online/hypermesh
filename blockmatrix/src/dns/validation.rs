// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! DNS Validation with Proof of State
//!
//! PoS-driven DNS access validation using state proof system.

use super::{DnsError, DnsResult, Domain};
use crate::proof_of_state::validation::{DefaultStateAuthenticator, StateAuthenticator};
use crate::proof_of_state::{StateProof, StateRequirements};
use serde::{Deserialize, Serialize};
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
    /// State proof validator
    state_validator: Arc<dyn StateAuthenticator>,
    /// State proof requirements for DNS operations
    requirements: StateRequirements,
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
            StateRequirements::production()
        } else {
            // Use lenient requirements for testing
            StateRequirements::localhost_testing()
        };

        Self {
            state_validator: Arc::new(DefaultStateAuthenticator::with_requirements(
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
        proof: &StateProof,
    ) -> DnsResult<ValidationResult> {
        debug!("Validating DNS access for domain: {}", domain.full);

        // Serialize proof for validation
        let proof_bytes = proof.to_bytes().map_err(|e| DnsError::ValidationFailed {
            reason: format!("Failed to serialize proof: {e}"),
        })?;

        // Validate state proof
        let is_valid = self
            .state_validator
            .validate(&proof_bytes)
            .await
            .map_err(|e| DnsError::ValidationFailed {
                reason: format!("State proof validation failed: {e}"),
            })?;

        if !is_valid {
            warn!("DNS access denied for domain: {}", domain.full);
            return Ok(ValidationResult {
                valid: false,
                reason: Some("State proof validation failed".to_string()),
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
        proof: &StateProof,
    ) -> DnsResult<ValidationResult> {
        debug!("Validating DNS registration for domain: {}", domain.full);

        // Registration requires all four proofs
        if !proof.validate_with_requirements(&self.requirements) {
            return Ok(ValidationResult {
                valid: false,
                reason: Some("Insufficient state proofs for registration".to_string()),
                validated_proofs: vec![],
            });
        }

        // Perform full validation
        self.validate_dns_access(domain, proof).await
    }

    /// Validate network access for federated domains.
    ///
    /// Public domains (no subdomains) are always accessible. Federated
    /// domains require the requester to supply a network ID. Full
    /// membership verification via blockchain will be added with
    /// Network scope sync.
    pub fn validate_network_access(
        &self,
        domain: &Domain,
        requester_network: Option<&str>,
    ) -> DnsResult<bool> {
        if domain.is_public() {
            return Ok(true);
        }

        match requester_network {
            Some(_network) => {
                // Network ID provided; accept. Full membership validation
                // requires Network scope blockchain sync.
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Check if domain requires full federation.
    ///
    /// Heuristic: two or more subdomain levels (e.g. `classified.internal.gov`)
    /// indicates fully federated, requiring multi-level membership validation.
    pub fn is_fully_federated(&self, domain: &Domain) -> bool {
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
    use crate::proof_of_state::proof_of_state_integration::{
        SpaceProof, StakeProof, TimeProof, WorkProof,
    };
    use std::time::Duration;

    fn create_test_proof() -> StateProof {
        let stake = StakeProof::new("holder".to_string(), "holder-id".to_string());
        let time = TimeProof::new(Duration::from_secs(10));
        let space = SpaceProof::new("node".to_string(), "/storage".to_string(), 1024 * 1024);
        let work = WorkProof::new("owner".to_string(), "workload".to_string(), *blake3::hash(format!("{}:{}", "owner".to_string(), "workload".to_string()).as_bytes()).as_bytes());

        StateProof::new(stake, time, space, work)
    }

    #[tokio::test]
    async fn test_dns_access_validation() {
        let validator = DnsValidator::new(false); // Non-strict for testing
        let domain = Domain::parse("nike").expect("test: expected success");
        let proof = create_test_proof();

        let result = validator
            .validate_dns_access(&domain, &proof)
            .await
            .expect("test: expected success");
        assert!(result.valid);
        assert_eq!(result.validated_proofs.len(), 4);
    }

    #[tokio::test]
    async fn test_registration_validation() {
        let validator = DnsValidator::new(false);
        let domain = Domain::parse("nike").expect("test: expected success");
        let proof = create_test_proof();

        let result = validator
            .validate_registration(&domain, &proof)
            .await
            .expect("test: expected success");
        assert!(result.valid);
    }

    #[test]
    fn test_network_access_validation() {
        let validator = DnsValidator::new(true);

        // Public domain accessible
        let domain = Domain::parse("nike").expect("test: expected success");
        assert!(validator.validate_network_access(&domain, None).expect("test: validation"));

        // Federated domain requires network
        let domain = Domain::parse("admin.nike").expect("test: expected success");
        assert!(validator
            .validate_network_access(&domain, Some("nike-internal"))
            .expect("test: expected success"));
        assert!(!validator.validate_network_access(&domain, None).expect("test: validation"));
    }

    #[test]
    fn test_fully_federated_check() {
        let validator = DnsValidator::new(true);

        let domain = Domain::parse("nike").expect("test: expected success");
        assert!(!validator.is_fully_federated(&domain));

        let domain = Domain::parse("admin.nike").expect("test: expected success");
        assert!(!validator.is_fully_federated(&domain));

        let domain = Domain::parse("classified.internal.gov").expect("test: expected success");
        assert!(validator.is_fully_federated(&domain));
    }
}
