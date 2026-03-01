// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// Privacy module for Block-MATRIX
// Uses hypermesh_lib::PrivacyMode as canonical type

pub mod ebpf_integration;
pub mod flexibility_matrix;
pub mod policies;
pub mod switching;
pub mod tiers;

pub use tiers::{
    validation_requirements_for, AnonymousTier, FederatedTier, FederationValidator, NetworkId,
    NodeId, PeerValidator, PrivateP2PTier, ProofOfStateValidator, PublicTier, TrustLevel,
    ValidationRequirements,
};

pub use flexibility_matrix::{
    AssetSharing, NetworkVisibility, PrivacyFlexibilityMatrix, PrivacyPresets, ValidationError,
};

pub use switching::{
    AssetState, AuthenticationData, ConnectionInfo, ConnectionType, MigrationState, TierSwitcher,
    TransactionInfo, TransactionState, TransitionError, TransitionRecord, TransitionResult,
};

pub use policies::{
    AccessRules, ActionType, ConditionType, EnforcementStats, PolicyAction, PolicyCondition,
    PolicyDecision, PolicyManager, PolicyViolation, RateLimit, RateLimits, RetentionPolicy,
    Severity, TierPolicy, ValidationType, ViolationType,
};

pub use ebpf_integration::{EbpfEventType, PrivacyEbpfBridge, PrivacyEbpfMetrics};

// Re-export PrivacyMode from hypermesh_lib for convenience
pub use hypermesh_lib::PrivacyMode;

/// Privacy system configuration
#[derive(Debug, Clone)]
pub struct PrivacyConfig {
    /// Default privacy mode for new nodes
    pub default_tier: PrivacyMode,
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
            default_tier: PrivacyMode::PRIVATE,
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
    /// Optional eBPF bridge for kernel-level privacy enforcement.
    /// When set, tier changes are automatically pushed to the eBPF layer.
    ebpf_bridge: Option<PrivacyEbpfBridge>,
    /// Connection ID counter for eBPF policy tracking
    next_connection_id: u64,
}

impl PrivacySystem {
    /// Create a new privacy system with default configuration
    pub fn new() -> Self {
        let config = PrivacyConfig::default();
        Self::with_config(config)
    }

    /// Create with specific configuration
    pub fn with_config(config: PrivacyConfig) -> Self {
        // Try to create the eBPF bridge; fall back gracefully if unavailable
        let ebpf_bridge = PrivacyEbpfBridge::new().ok();
        if ebpf_bridge.is_some() {
            tracing::info!("Privacy eBPF bridge initialized");
        } else {
            tracing::debug!("Privacy eBPF bridge unavailable (non-critical)");
        }

        Self {
            tier_switcher: TierSwitcher::new(config.default_tier),
            flexibility_matrix: PrivacyFlexibilityMatrix::uniform(config.default_tier),
            policy_manager: PolicyManager::new(),
            ebpf_bridge,
            next_connection_id: 0,
            config,
        }
    }

    /// Get current privacy mode
    pub fn current_tier(&self) -> PrivacyMode {
        self.tier_switcher.current_tier()
    }

    /// Switch to a new privacy mode.
    ///
    /// When an eBPF bridge is available, this also pushes the new tier
    /// to the kernel-level eBPF policy layer via
    /// [`PrivacyEbpfBridge::update_ebpf_for_tier`].
    pub fn switch_tier(
        &mut self,
        new_tier: PrivacyMode,
    ) -> Result<TransitionResult, TransitionError> {
        if !self.config.allow_switching {
            return Err(TransitionError::InvalidTransition(
                "Tier switching disabled".into(),
            ));
        }

        // Check daily switch limit
        let today_switches = self
            .tier_switcher
            .transition_history()
            .iter()
            .filter(|r| {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("system time should be after UNIX epoch")
                    .as_secs();
                now - r.timestamp < 86400 // 24 hours
            })
            .count();

        if today_switches >= self.config.max_switches_per_day as usize {
            return Err(TransitionError::InvalidTransition(format!(
                "Daily switch limit ({}) reached",
                self.config.max_switches_per_day
            )));
        }

        // Perform the switch
        let result = self.tier_switcher.switch_tier(new_tier)?;

        // Update flexibility matrix network tier
        self.flexibility_matrix.network_tier = new_tier;

        // Push tier change to eBPF layer if bridge is available
        if let Some(ref bridge) = self.ebpf_bridge {
            let conn_id = self.next_connection_id;
            self.next_connection_id = self.next_connection_id.wrapping_add(1);
            bridge.update_ebpf_for_tier(new_tier, conn_id);
            bridge.set_default_ebpf_tier(new_tier);
            tracing::debug!(
                "eBPF tier updated: mode={:?}, connection_id={}",
                new_tier,
                conn_id
            );
        }

        Ok(result)
    }

    /// Update privacy flexibility matrix.
    ///
    /// When an eBPF bridge is available, also pushes the matrix
    /// configuration to the kernel-level policy layer.
    pub fn update_flexibility_matrix(
        &mut self,
        matrix: PrivacyFlexibilityMatrix,
    ) -> Result<(), ValidationError> {
        matrix.validate_configuration()?;

        // Push matrix to eBPF layer if bridge is available
        if let Some(ref bridge) = self.ebpf_bridge {
            let conn_id = self.next_connection_id;
            self.next_connection_id = self.next_connection_id.wrapping_add(1);
            bridge.update_ebpf_for_matrix(&matrix, conn_id);
        }

        self.flexibility_matrix = matrix;
        Ok(())
    }

    /// Get a reference to the eBPF bridge, if available.
    pub fn ebpf_bridge(&self) -> Option<&PrivacyEbpfBridge> {
        self.ebpf_bridge.as_ref()
    }

    /// Enforce policy for an action
    pub fn enforce_policy(
        &mut self,
        action: PolicyAction,
    ) -> Result<PolicyDecision, PolicyViolation> {
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
        assert_eq!(system.current_tier(), PrivacyMode::PRIVATE);
        assert!(system.config.allow_switching);
    }

    #[test]
    fn test_privacy_system_tier_switching() {
        let mut system = PrivacySystem::new();
        let result = system.switch_tier(PrivacyMode::PUBLIC);
        assert!(result.is_ok());
        assert_eq!(system.current_tier(), PrivacyMode::PUBLIC);
    }

    #[test]
    fn test_daily_switch_limit() {
        let config = PrivacyConfig {
            max_switches_per_day: 1,
            ..PrivacyConfig::default()
        };
        let mut system = PrivacySystem::with_config(config);

        // First switch should succeed
        assert!(system.switch_tier(PrivacyMode::PUBLIC).is_ok());

        // Second switch should fail
        let result = system.switch_tier(PrivacyMode::ANONYMOUS);
        assert!(matches!(result, Err(TransitionError::InvalidTransition(_))));
    }

    #[test]
    fn test_flexibility_matrix_update() {
        let mut system = PrivacySystem::new();
        let matrix = PrivacyFlexibilityMatrix::new(PrivacyMode::ANONYMOUS, PrivacyMode::PUBLIC);

        assert!(system.update_flexibility_matrix(matrix).is_ok());
        assert!(system.flexibility_matrix.is_anonymous_public());
    }

    #[test]
    fn test_policy_enforcement() {
        let mut system = PrivacySystem::new();
        let mut validations = HashSet::new();
        validations.insert(ValidationType::PeerIdentity);
        validations.insert(ValidationType::PeerTrust);

        let action = PolicyAction {
            action_type: ActionType::ShareResource,
            actor: Some(NodeId([1u8; 32])),
            target: Some(NodeId([2u8; 32])),
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
        system.switch_tier(PrivacyMode::PUBLIC).expect("test: mode switch");
        system.flexibility_matrix.asset_tier = PrivacyMode::PUBLIC;

        assert_eq!(system.caesar_multiplier(), 1.0);
    }

    #[test]
    fn test_privacy_scores() {
        let mut system = PrivacySystem::new();
        system.switch_tier(PrivacyMode::ANONYMOUS).expect("test: mode switch");
        system.flexibility_matrix.asset_tier = PrivacyMode::ANONYMOUS;

        assert_eq!(system.privacy_score(), 1.0);
        assert_eq!(system.openness_score(), 0.0);
        assert!(system.is_privacy_focused());
    }

    #[test]
    fn test_lenient_enforcement_mode() {
        let config = PrivacyConfig {
            strict_enforcement: false,
            ..PrivacyConfig::default()
        };
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

        system.switch_tier(PrivacyMode::ANONYMOUS).expect("test: mode switch");

        // Should allow in lenient mode
        let result = system.enforce_policy(action);
        assert!(matches!(result, Ok(PolicyDecision::Allow)));
    }

    #[test]
    fn test_switching_disabled() {
        let config = PrivacyConfig {
            allow_switching: false,
            ..PrivacyConfig::default()
        };
        let mut system = PrivacySystem::with_config(config);

        let result = system.switch_tier(PrivacyMode::PUBLIC);
        assert!(matches!(result, Err(TransitionError::InvalidTransition(_))));
    }
}
