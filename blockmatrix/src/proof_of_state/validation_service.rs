// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Real validation service implementation using TrustChain's Proof of State.
//!
//! D4: a proof is validated against the rules of the NETWORK that owns the
//! asset (selected by [`NetworkScope`]), not one global bar. See
//! [`super::network_rules`] for the resolver and its non-regression contract.

use super::*;
use crate::assets::core::asset_id::NetworkScope;
use crate::proof_of_state::network_rules::NetworkRuleSet;
use crate::proof_of_state::validation::{DefaultStateAuthenticator, StateAuthenticator};
use std::sync::Arc;
use tracing::warn;

/// Validates a [`StateProof`] against the rules of the NETWORK that owns the
/// asset (D4). Holds a [`NetworkRuleSet`] resolver whose default anchor is the
/// service's fixed requirements — so with no published network rulesets (the
/// state today) every scope resolves to that anchor and behaviour is
/// byte-identical to the pre-D4 single-bar model.
pub struct ValidationService {
    validator: Arc<dyn StateAuthenticator>,
    rules: NetworkRuleSet,
}

impl Default for ValidationService {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationService {
    pub fn new() -> Self {
        Self {
            validator: Arc::new(DefaultStateAuthenticator::new()),
            rules: NetworkRuleSet::new(StateRequirements::default()),
        }
    }

    pub fn with_requirements(requirements: StateRequirements) -> Self {
        Self {
            validator: Arc::new(DefaultStateAuthenticator::with_requirements(
                requirements.clone(),
            )),
            rules: NetworkRuleSet::new(requirements),
        }
    }

    pub fn for_production() -> Self {
        Self {
            validator: Arc::new(DefaultStateAuthenticator::with_requirements(
                StateRequirements::production(),
            )),
            rules: NetworkRuleSet::new(StateRequirements::production()),
        }
    }

    /// The SINGLE scope→rules seam both call sites share.
    ///
    /// Today it delegates to [`NetworkRuleSet::resolve`], which is infallible
    /// (an unseen network inherits the anchor) — so it never rejects,
    /// preserving every asset that validates today. This is the one place the
    /// fail-closed hook flips on: swap `resolve` for
    /// [`NetworkRuleSet::resolve_strict`] once networks publish genesis
    /// rulesets, and BOTH the local and received paths gain fail-closed
    /// rejection of unadopted networks together.
    fn resolve_rules(
        &self,
        scope: &NetworkScope,
    ) -> Result<&StateRequirements, StateProofError> {
        Ok(self.rules.resolve(scope))
    }

    /// The resolver backing this service (for tests / future callers).
    pub fn rules(&self) -> &NetworkRuleSet {
        &self.rules
    }

    /// RECEIVED-block validation — the untrusted-mirror path.
    ///
    /// Resolves the owning network's rules through the SAME seam as the local
    /// path (so the fail-closed hook, when activated, gates both), but applies
    /// ONLY the intrinsic structural bar (`proof.validate()`), which is
    /// byte-identical to the pre-D4 `entry.state_proof.validate()` this path
    /// used.
    ///
    /// It deliberately does NOT apply `validate_with_requirements`: that bar
    /// adds a WHEN-freshness gate (`network_time_offset > max_time_offset`),
    /// and a received block may be a HISTORICAL block replayed from a
    /// reflector/sync pool carrying a stale offset. Tightening the received bar
    /// with the freshness gate would reject such honest, currently-accepted
    /// blocks — a regression. The received bar therefore stays structural-only
    /// while still flowing through the scope seam.
    pub fn validate_received(
        &self,
        proof: &StateProof,
        scope: &NetworkScope,
    ) -> Result<bool, StateProofError> {
        // Resolve the owning network (the seam / future fail-closed point);
        // the resolved requirements are intentionally not applied here.
        let _requirements = self.resolve_rules(scope)?;
        if proof.validate() {
            Ok(true)
        } else {
            warn!("Received state proof failed structural validation — mirror rejected");
            Err(StateProofError::ValidationFailed(
                "State proof failed validation requirements".to_string(),
            ))
        }
    }
}

/// Trait for state proof validation service.
pub trait StateProofValidationService: Send + Sync {
    /// Validate `proof` against the rules of the network named by `scope`.
    fn validate(
        &self,
        proof: &StateProof,
        scope: &NetworkScope,
    ) -> Result<bool, StateProofError>;
}

impl StateProofValidationService for ValidationService {
    fn validate(
        &self,
        proof: &StateProof,
        scope: &NetworkScope,
    ) -> Result<bool, StateProofError> {
        let requirements = self.resolve_rules(scope)?;
        if proof.validate_with_requirements(requirements) {
            Ok(true)
        } else {
            // S3.2 debug!-swallow bug: rejections MUST be INFO-visible.
            warn!("State proof failed validation requirements for scope {scope:?}");
            Err(StateProofError::ValidationFailed(
                "State proof failed validation requirements".to_string(),
            ))
        }
    }
}

impl ValidationService {
    pub async fn validate_async(&self, proof: &StateProof) -> Result<bool, StateProofError> {
        let proof_bytes = proof
            .to_bytes()
            .map_err(|e| StateProofError::Other(format!("Failed to serialize proof: {e}")))?;

        match self.validator.validate(&proof_bytes).await {
            Ok(true) => Ok(true),
            Ok(false) => Err(StateProofError::ValidationFailed(
                "State proof validation failed".to_string(),
            )),
            Err(e) => Err(StateProofError::ValidationFailed(format!(
                "Validation error: {e}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::core::asset_id::NodeFingerprint;
    use std::time::Duration;
    use trustchain::proof_of_state::StateProof;

    fn private(seed: u8) -> NetworkScope {
        NetworkScope::Private(NodeFingerprint([seed; 32]))
    }

    /// THE ANCHOR (D4 migration invariant): with no published network
    /// rulesets, a `Global`-scope proof validates EXACTLY as a `Private`-scope
    /// one — both against the single anchor bar. This is the pre-D4 behaviour,
    /// byte-identical: scope selection changes nothing until a network diverges.
    #[test]
    fn global_and_private_validate_identically_at_the_anchor() {
        let service = ValidationService::new();
        let proof = StateProof::new_for_testing();

        let global_res = service.validate(&proof, &NetworkScope::Global);
        let private_res = service.validate(&proof, &private(3));
        assert!(global_res.is_ok(), "Global proof must validate at the anchor");
        assert!(private_res.is_ok(), "Private proof must validate identically");

        // An invalid proof is rejected identically under either scope.
        let mut bad = StateProof::new_for_testing();
        bad.stake_proof.stake_holder_id = String::new();
        assert!(service.validate(&bad, &NetworkScope::Global).is_err());
        assert!(service.validate(&bad, &private(3)).is_err());
    }

    /// NON-REGRESSION (the decisive proof): the received path does NOT apply
    /// the WHEN-freshness gate. A structurally-valid proof carrying a STALE
    /// `network_time_offset` (far beyond the anchor's `max_time_offset`) — as a
    /// historical block replayed from a sync pool would — is REJECTED by the
    /// local `validate` (freshness gate) but ACCEPTED by `validate_received`.
    /// Unifying the received path onto the service therefore did not tighten
    /// its bar.
    #[test]
    fn received_path_does_not_tighten_on_stale_time() {
        let service = ValidationService::new(); // anchor max_time_offset = 60s
        let mut stale = StateProof::new_for_testing();
        // A WHEN offset an hour past the freshness bound; recompute the proof
        // hash so the proof stays structurally valid (only freshness differs).
        stale.time_proof =
            trustchain::proof_of_state::TimeProof::new(Duration::from_secs(3600));
        assert!(stale.validate(), "test: proof is structurally valid");

        // Local path applies the freshness gate → rejected.
        assert!(
            service.validate(&stale, &NetworkScope::Global).is_err(),
            "local path rejects a stale WHEN offset",
        );
        // Received path is structural-only → accepted (no new rejection).
        assert!(
            matches!(
                service.validate_received(&stale, &NetworkScope::Global),
                Ok(true)
            ),
            "received path must NOT reject a stale historical block",
        );
    }

    /// Received path still rejects a structurally-INVALID proof (the gate it
    /// always had is preserved).
    #[test]
    fn received_path_rejects_structurally_invalid() {
        let service = ValidationService::new();
        let mut bad = StateProof::new_for_testing();
        bad.stake_proof.stake_holder_id = String::new(); // WHO unbound → invalid
        assert!(!bad.validate(), "test: proof is structurally invalid");
        assert!(service.validate_received(&bad, &NetworkScope::Global).is_err());
    }
}
