// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// Privacy module for Block-MATRIX
// Revolutionary Concept #5: Four privacy tiers with flexibility matrix

pub mod tiers;
pub mod flexibility_matrix;
pub mod switching;
pub mod policies;
pub mod ebpf_integration;

pub use tiers::{
    AnonymousTier, FederatedTier, NodeId, NetworkId, PrivacyTier,
    PrivateP2PTier, PublicTier, TrustLevel, ValidationRequirements,
    PeerValidator, FederationValidator, ProofOfStateValidator,
};

pub use flexibility_matrix::{
    PrivacyFlexibilityMatrix, NetworkVisibility, AssetSharing,
    ValidationError, PrivacyPresets, AssetId,
};

pub use switching::{
    TierSwitcher, TransitionResult, TransitionError, TransitionRecord,
    MigrationState, ConnectionInfo, ConnectionType, TransactionInfo,
    TransactionState, AssetState, ReputationData,
};

pub use policies::{
    PolicyManager, TierPolicy, PolicyAction, PolicyDecision,
    PolicyViolation, PolicyCondition, ConditionType, ViolationType,
    Severity, ActionType, ValidationType, AccessRules, RetentionPolicy,
    RateLimits, RateLimit, EnforcementStats,
};

pub use ebpf_integration::{
    PrivacyEbpfBridge, PrivacyEbpfMetrics, EbpfEventType,
};

/// Privacy system configuration
#[derive(Debug, Clone)]
pub struct PrivacyConfig {
    /// Default privacy tier for new nodes
    pub default_tier: PrivacyTier,
    /// Whether to allow tier switching
    pub allow_switching: bool,
    /// Maximum number of tier switches per day
    pub max_switches_per_day: u32,
    /// Whether to enforce strict policies
    pub strict_enforcement: bool,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            default_tier: PrivacyTier::Federated,
            allow_switching: true,
            max_switches_per_day: 3,
            strict_enforcement: true,
        }
    }
}

/// Main privacy system orchestrator
pub struct PrivacySystem {
    /// Configuration
    pub config: PrivacyConfig,
    /// Tier switcher for managing transitions
    pub tier_switcher: TierSwitcher,
    /// Privacy flexibility matrix
    pub flexibility_matrix: PrivacyFlexibilityMatrix,
    /// Policy manager for enforcement
    pub policy_manager: PolicyManager,
}

impl PrivacySystem {
    /// Create a new privacy system with default configuration
    pub fn new() -> Self {
        let config = PrivacyConfig::default();
        Self::with_config(config)
    }

    /// Create with specific configuration
    pub fn with_config(config: PrivacyConfig) -> Self {
        Self {
            tier_switcher: TierSwitcher::new(config.default_tier),
            flexibility_matrix: PrivacyFlexibilityMatrix::uniform(config.default_tier),
            policy_manager: PolicyManager::new(),
            config,
        }
    }

    /// Get current privacy tier
    pub fn current_tier(&self) -> PrivacyTier {
        self.tier_switcher.current_tier()
    }

    /// Switch to a new privacy tier
    pub fn switch_tier(&mut self, new_tier: PrivacyTier) -> Result<TransitionResult, TransitionError> {
        if !self.config.allow_switching {
            return Err(TransitionError::InvalidTransition("Tier switching disabled".into()));
        }

        // Check daily switch limit
        let today_switches = self.tier_switcher
            .transition_history()
            .iter()
            .filter(|r| {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                now - r.timestamp < 86400 // 24 hours
            })
            .count();

        if today_switches >= self.config.max_switches_per_day as usize {
            return Err(TransitionError::InvalidTransition(
                format!("Daily switch limit ({}) reached", self.config.max_switches_per_day)
            ));
        }

        // Perform the switch
        let result = self.tier_switcher.switch_tier(new_tier)?;

        // Update flexibility matrix network tier
        self.flexibility_matrix.network_tier = new_tier;

        Ok(result)
    }

    /// Update privacy flexibility matrix
    pub fn update_flexibility_matrix(&mut self, matrix: PrivacyFlexibilityMatrix) -> Result<(), ValidationError> {
        matrix.validate_configuration()?;
        self.flexibility_matrix = matrix;
        Ok(())
    }

    /// Enforce policy for an action
    pub fn enforce_policy(&mut self, action: PolicyAction) -> Result<PolicyDecision, PolicyViolation> {
        let tier = self.current_tier();

        if self.config.strict_enforcement {
            self.policy_manager.enforce(tier, action)
        } else {
            // Lenient mode - log violations but allow actions
            match self.policy_manager.enforce(tier, action) {
                Err(_) => Ok(PolicyDecision::Allow),
                ok => ok,
            }
        }
    }

    /// Get CAESAR rewards multiplier for current configuration
    pub fn caesar_multiplier(&self) -> f64 {
        self.flexibility_matrix.caesar_multiplier()
    }

    /// Get privacy score (0.0 = no privacy, 1.0 = maximum privacy)
    pub fn privacy_score(&self) -> f32 {
        self.flexibility_matrix.privacy_score()
    }

    /// Get openness score (0.0 = closed, 1.0 = fully open)
    pub fn openness_score(&self) -> f32 {
        self.flexibility_matrix.openness_score()
    }

    /// Check if system is in privacy-focused mode
    pub fn is_privacy_focused(&self) -> bool {
        self.flexibility_matrix.is_privacy_focused()
    }

    /// Get enforcement statistics
    pub fn enforcement_stats(&self) -> &EnforcementStats {
        self.policy_manager.stats()
    }
}

impl Default for PrivacySystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_privacy_system_creation() {
        let system = PrivacySystem::new();
        assert_eq!(system.current_tier(), PrivacyTier::Federated);
        assert!(system.config.allow_switching);
    }

    #[test]
    fn test_privacy_system_tier_switching() {
        let mut system = PrivacySystem::new();
        let result = system.switch_tier(PrivacyTier::Public);
        assert!(result.is_ok());
        assert_eq!(system.current_tier(), PrivacyTier::Public);
    }

    #[test]
    fn test_daily_switch_limit() {
        let mut config = PrivacyConfig::default();
        config.max_switches_per_day = 1;
        let mut system = PrivacySystem::with_config(config);

        // First switch should succeed
        assert!(system.switch_tier(PrivacyTier::Public).is_ok());

        // Second switch should fail
        let result = system.switch_tier(PrivacyTier::Anonymous);
        assert!(matches!(result, Err(TransitionError::InvalidTransition(_))));
    }

    #[test]
    fn test_flexibility_matrix_update() {
        let mut system = PrivacySystem::new();
        let matrix = PrivacyFlexibilityMatrix::new(
            PrivacyTier::Anonymous,
            PrivacyTier::Public
        );

        assert!(system.update_flexibility_matrix(matrix).is_ok());
        assert!(system.flexibility_matrix.is_anonymous_public());
    }

    #[test]
    fn test_policy_enforcement() {
        let mut system = PrivacySystem::new();
        let mut validations = HashSet::new();
        validations.insert(ValidationType::NetworkIdentity);
        validations.insert(ValidationType::FederationMembership);
        validations.insert(ValidationType::ProofOfSpace);
        validations.insert(ValidationType::ProofOfTime);

        let action = PolicyAction {
            action_type: ActionType::ShareResource,
            actor: Some([1u8; 32]),
            target: Some([2u8; 32]),
            provided_validations: validations,
            queries_identity: false,
            queries_location: false,
            high_value: false,
        };

        let result = system.enforce_policy(action);
        assert!(result.is_ok());
    }

    #[test]
    fn test_caesar_multiplier() {
        let mut system = PrivacySystem::new();
        system.switch_tier(PrivacyTier::Public).unwrap();
        system.flexibility_matrix.asset_tier = PrivacyTier::Public;

        assert_eq!(system.caesar_multiplier(), 1.0);
    }

    #[test]
    fn test_privacy_scores() {
        let mut system = PrivacySystem::new();
        system.switch_tier(PrivacyTier::Anonymous).unwrap();
        system.flexibility_matrix.asset_tier = PrivacyTier::Anonymous;

        assert_eq!(system.privacy_score(), 1.0);
        assert_eq!(system.openness_score(), 0.0);
        assert!(system.is_privacy_focused());
    }

    #[test]
    fn test_lenient_enforcement_mode() {
        let mut config = PrivacyConfig::default();
        config.strict_enforcement = false;
        let mut system = PrivacySystem::with_config(config);

        // Action that would normally be denied
        let action = PolicyAction {
            action_type: ActionType::ValidateBlock,
            actor: None,
            target: None,
            provided_validations: HashSet::new(),
            queries_identity: true,
            queries_location: true,
            high_value: true,
        };

        system.switch_tier(PrivacyTier::Anonymous).unwrap();

        // Should allow in lenient mode
        let result = system.enforce_policy(action);
        assert!(matches!(result, Ok(PolicyDecision::Allow)));
    }

    #[test]
    fn test_switching_disabled() {
        let mut config = PrivacyConfig::default();
        config.allow_switching = false;
        let mut system = PrivacySystem::with_config(config);

        let result = system.switch_tier(PrivacyTier::Public);
        assert!(matches!(result, Err(TransitionError::InvalidTransition(_))));
    }
}