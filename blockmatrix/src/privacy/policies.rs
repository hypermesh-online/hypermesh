// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// Per-tier validation policies for the privacy system
// Defines and enforces rules for each privacy tier

use super::tiers::NodeId;
use hypermesh_lib::PrivacyMode;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Policy enforcement result
pub type PolicyResult = Result<PolicyDecision, PolicyViolation>;

/// Decision made by policy enforcement
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    /// Action is allowed
    Allow,
    /// Action is allowed with conditions
    AllowWithConditions(Vec<PolicyCondition>),
    /// Action is denied
    Deny(String),
}

/// Conditions that must be met for conditional approval
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyCondition {
    pub condition_type: ConditionType,
    pub description: String,
    pub timeout_ms: Option<u64>,
}

/// Types of policy conditions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionType {
    RequireValidation,
    RequirePeerApproval,
    RequireFederationConsensus,
    RequireProofOfState,
    RateLimited,
    TimeLocked,
}

/// Policy violation details
#[derive(Debug, Clone)]
pub struct PolicyViolation {
    pub violation_type: ViolationType,
    pub tier: PrivacyMode,
    pub message: String,
    pub severity: Severity,
}

/// Types of policy violations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViolationType {
    IdentityRequired,
    ValidationMissing,
    InsufficientTrust,
    RateLimitExceeded,
    UnauthorizedAccess,
    InvalidTransition,
}

/// Severity levels for violations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Master policy manager for all tiers
pub struct PolicyManager {
    /// Mode-specific policies
    tier_policies: HashMap<PrivacyMode, TierPolicy>,
    /// Global rate limits
    rate_limits: RateLimits,
    /// Enforcement statistics
    enforcement_stats: EnforcementStats,
}

impl PolicyManager {
    pub fn new() -> Self {
        let mut manager = Self {
            tier_policies: HashMap::new(),
            rate_limits: RateLimits::default(),
            enforcement_stats: EnforcementStats::default(),
        };

        // Initialize policies for each mode
        manager.tier_policies.insert(PrivacyMode::ANONYMOUS, TierPolicy::anonymous());
        manager.tier_policies.insert(PrivacyMode::PRIVATE, TierPolicy::private_p2p());
        manager.tier_policies.insert(PrivacyMode::PUBLIC, TierPolicy::public());

        manager
    }

    /// Enforce policy for a specific action
    pub fn enforce(&mut self, tier: PrivacyMode, action: PolicyAction) -> PolicyResult {
        self.enforcement_stats.total_checks += 1;

        let policy = self.tier_policies.get(&tier)
            .ok_or_else(|| PolicyViolation {
                violation_type: ViolationType::InvalidTransition,
                tier,
                message: "No policy defined for tier".to_string(),
                severity: Severity::Critical,
            })?;

        // Check rate limits
        if !self.rate_limits.check_rate(tier, &action) {
            self.enforcement_stats.violations += 1;
            return Err(PolicyViolation {
                violation_type: ViolationType::RateLimitExceeded,
                tier,
                message: "Rate limit exceeded".to_string(),
                severity: Severity::Medium,
            });
        }

        // Apply tier-specific policy
        let decision = policy.evaluate(action)?;

        match &decision {
            PolicyDecision::Allow | PolicyDecision::AllowWithConditions(_) => {
                self.enforcement_stats.allowed += 1;
            }
            PolicyDecision::Deny(_) => {
                self.enforcement_stats.denied += 1;
            }
        }

        Ok(decision)
    }

    /// Update policy for a specific mode
    pub fn update_policy(&mut self, tier: PrivacyMode, policy: TierPolicy) {
        self.tier_policies.insert(tier, policy);
    }

    /// Get enforcement statistics
    pub fn stats(&self) -> &EnforcementStats {
        &self.enforcement_stats
    }

    /// Reset enforcement statistics
    pub fn reset_stats(&mut self) {
        self.enforcement_stats = EnforcementStats::default();
    }
}

impl Default for PolicyManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Tier-specific policy configuration
#[derive(Debug, Clone)]
pub struct TierPolicy {
    /// Privacy mode this policy applies to
    pub tier: PrivacyMode,
    /// Allowed actions
    pub allowed_actions: HashSet<ActionType>,
    /// Required validations
    pub required_validations: Vec<ValidationType>,
    /// Access control rules
    pub access_rules: AccessRules,
    /// Data retention policy
    pub retention_policy: RetentionPolicy,
}

impl TierPolicy {
    /// Create policy for anonymous mode
    pub fn anonymous() -> Self {
        Self {
            tier: PrivacyMode::ANONYMOUS,
            allowed_actions: vec![
                ActionType::Connect,
                ActionType::Disconnect,
                ActionType::QueryPublic,
            ].into_iter().collect(),
            required_validations: vec![],
            access_rules: AccessRules {
                allow_identity_query: false,
                allow_location_query: false,
                allow_metric_collection: false,
                allow_resource_discovery: false,
            },
            retention_policy: RetentionPolicy {
                connection_logs: false,
                transaction_history: false,
                retention_hours: 0,
            },
        }
    }

    /// Create policy for private P2P mode
    pub fn private_p2p() -> Self {
        Self {
            tier: PrivacyMode::PRIVATE,
            allowed_actions: vec![
                ActionType::Connect,
                ActionType::Disconnect,
                ActionType::QueryPublic,
                ActionType::ShareResource,
                ActionType::ValidatePeer,
            ].into_iter().collect(),
            required_validations: vec![
                ValidationType::PeerIdentity,
                ValidationType::PeerTrust,
            ],
            access_rules: AccessRules {
                allow_identity_query: true,
                allow_location_query: false,
                allow_metric_collection: false,
                allow_resource_discovery: true,
            },
            retention_policy: RetentionPolicy {
                connection_logs: true,
                transaction_history: true,
                retention_hours: 24,
            },
        }
    }

    /// Create policy for public mode
    pub fn public() -> Self {
        Self {
            tier: PrivacyMode::PUBLIC,
            allowed_actions: ActionType::all(),
            required_validations: vec![
                ValidationType::FullIdentity,
                ValidationType::ProofOfSpace,
                ValidationType::ProofOfStake,
                ValidationType::ProofOfWork,
                ValidationType::ProofOfTime,
            ],
            access_rules: AccessRules {
                allow_identity_query: true,
                allow_location_query: true,
                allow_metric_collection: true,
                allow_resource_discovery: true,
            },
            retention_policy: RetentionPolicy {
                connection_logs: true,
                transaction_history: true,
                retention_hours: 8760, // 1 year
            },
        }
    }

    /// Evaluate a policy action
    pub fn evaluate(&self, action: PolicyAction) -> PolicyResult {
        // Check if action type is allowed
        if !self.allowed_actions.contains(&action.action_type) {
            return Err(PolicyViolation {
                violation_type: ViolationType::UnauthorizedAccess,
                tier: self.tier,
                message: format!("Action {:?} not allowed in {} mode", action.action_type, self.tier),
                severity: Severity::High,
            });
        }

        // Check required validations
        for required in &self.required_validations {
            if !action.provided_validations.contains(required) {
                return Err(PolicyViolation {
                    violation_type: ViolationType::ValidationMissing,
                    tier: self.tier,
                    message: format!("Missing required validation: {:?}", required),
                    severity: Severity::Medium,
                });
            }
        }

        // Apply access rules
        if let Some(violation) = self.check_access_rules(&action) {
            return Err(violation);
        }

        // Check if conditions are needed
        let conditions = self.get_required_conditions(&action);
        if !conditions.is_empty() {
            Ok(PolicyDecision::AllowWithConditions(conditions))
        } else {
            Ok(PolicyDecision::Allow)
        }
    }

    /// Check access rules
    fn check_access_rules(&self, action: &PolicyAction) -> Option<PolicyViolation> {
        if action.queries_identity && !self.access_rules.allow_identity_query {
            return Some(PolicyViolation {
                violation_type: ViolationType::UnauthorizedAccess,
                tier: self.tier,
                message: "Identity queries not allowed".to_string(),
                severity: Severity::High,
            });
        }

        if action.queries_location && !self.access_rules.allow_location_query {
            return Some(PolicyViolation {
                violation_type: ViolationType::UnauthorizedAccess,
                tier: self.tier,
                message: "Location queries not allowed".to_string(),
                severity: Severity::Medium,
            });
        }

        None
    }

    /// Get required conditions for an action
    fn get_required_conditions(&self, action: &PolicyAction) -> Vec<PolicyCondition> {
        let mut conditions = Vec::new();

        // Add mode-specific conditions
        if self.tier == PrivacyMode::PRIVATE && action.action_type == ActionType::ShareResource {
            conditions.push(PolicyCondition {
                condition_type: ConditionType::RequirePeerApproval,
                description: "Requires approval from trusted peers".to_string(),
                timeout_ms: Some(5000),
            });
        } else if self.tier == PrivacyMode::PUBLIC && action.action_type == ActionType::ValidateBlock {
            conditions.push(PolicyCondition {
                condition_type: ConditionType::RequireProofOfState,
                description: "Block validation requires full proof of state".to_string(),
                timeout_ms: None,
            });
        }

        conditions
    }
}

/// Action being evaluated by policy
#[derive(Debug, Clone)]
pub struct PolicyAction {
    pub action_type: ActionType,
    pub actor: Option<NodeId>,
    pub target: Option<NodeId>,
    pub provided_validations: HashSet<ValidationType>,
    pub queries_identity: bool,
    pub queries_location: bool,
    pub high_value: bool,
}

/// Types of actions that can be performed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionType {
    Connect,
    Disconnect,
    QueryPublic,
    QueryPrivate,
    ShareResource,
    AccessResource,
    ValidatePeer,
    ValidateBlock,
    SubmitTransaction,
}

impl ActionType {
    pub fn all() -> HashSet<Self> {
        vec![
            Self::Connect,
            Self::Disconnect,
            Self::QueryPublic,
            Self::QueryPrivate,
            Self::ShareResource,
            Self::AccessResource,
            Self::ValidatePeer,
            Self::ValidateBlock,
            Self::SubmitTransaction,
        ].into_iter().collect()
    }
}

/// Types of validation that can be provided
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValidationType {
    None,
    PeerIdentity,
    PeerTrust,
    NetworkIdentity,
    FederationMembership,
    FullIdentity,
    ProofOfSpace,
    ProofOfStake,
    ProofOfWork,
    ProofOfTime,
}

/// Access control rules
#[derive(Debug, Clone)]
pub struct AccessRules {
    pub allow_identity_query: bool,
    pub allow_location_query: bool,
    pub allow_metric_collection: bool,
    pub allow_resource_discovery: bool,
}

/// Data retention policy
#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    pub connection_logs: bool,
    pub transaction_history: bool,
    pub retention_hours: u64,
}

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimits {
    /// Limits per mode
    tier_limits: HashMap<PrivacyMode, RateLimit>,
    /// Action counts for rate limiting
    action_counts: HashMap<(PrivacyMode, ActionType), u64>,
}

impl RateLimits {
    pub fn check_rate(&mut self, tier: PrivacyMode, action: &PolicyAction) -> bool {
        let key = (tier, action.action_type);
        let count = self.action_counts.entry(key).or_insert(0);

        let limit = self.tier_limits
            .get(&tier)
            .and_then(|l| l.get_limit(action.action_type))
            .unwrap_or(1000);

        if *count >= limit {
            return false;
        }

        *count += 1;
        true
    }
}

impl Default for RateLimits {
    fn default() -> Self {
        let mut limits = HashMap::new();

        // Configure mode-specific rate limits
        limits.insert(PrivacyMode::ANONYMOUS, RateLimit::restrictive());
        limits.insert(PrivacyMode::PRIVATE, RateLimit::moderate());
        limits.insert(PrivacyMode::PUBLIC, RateLimit::unlimited());

        Self {
            tier_limits: limits,
            action_counts: HashMap::new(),
        }
    }
}

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimit {
    pub actions_per_minute: HashMap<ActionType, u64>,
}

impl RateLimit {
    pub fn restrictive() -> Self {
        let mut limits = HashMap::new();
        limits.insert(ActionType::Connect, 1);
        limits.insert(ActionType::QueryPublic, 10);
        Self { actions_per_minute: limits }
    }

    pub fn moderate() -> Self {
        let mut limits = HashMap::new();
        limits.insert(ActionType::Connect, 10);
        limits.insert(ActionType::QueryPublic, 100);
        limits.insert(ActionType::ShareResource, 50);
        Self { actions_per_minute: limits }
    }

    pub fn permissive() -> Self {
        let mut limits = HashMap::new();
        limits.insert(ActionType::Connect, 100);
        limits.insert(ActionType::QueryPublic, 1000);
        limits.insert(ActionType::ShareResource, 500);
        Self { actions_per_minute: limits }
    }

    pub fn unlimited() -> Self {
        Self { actions_per_minute: HashMap::new() }
    }

    pub fn get_limit(&self, action: ActionType) -> Option<u64> {
        self.actions_per_minute.get(&action).copied()
    }
}

/// Enforcement statistics
#[derive(Debug, Clone, Default)]
pub struct EnforcementStats {
    pub total_checks: u64,
    pub allowed: u64,
    pub denied: u64,
    pub violations: u64,
}

impl std::fmt::Display for PolicyViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}] {} mode: {} (Severity: {:?})",
               self.violation_type, self.tier, self.message, self.severity)
    }
}

impl std::error::Error for PolicyViolation {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_manager_creation() {
        let manager = PolicyManager::new();
        assert_eq!(manager.tier_policies.len(), 3);
    }

    #[test]
    fn test_anonymous_tier_policy() {
        let policy = TierPolicy::anonymous();
        assert!(!policy.access_rules.allow_identity_query);
        assert!(!policy.retention_policy.connection_logs);
        assert!(policy.allowed_actions.contains(&ActionType::Connect));
        assert!(!policy.allowed_actions.contains(&ActionType::ValidateBlock));
    }

    #[test]
    fn test_public_tier_policy() {
        let policy = TierPolicy::public();
        assert!(policy.access_rules.allow_identity_query);
        assert!(policy.retention_policy.connection_logs);
        assert_eq!(policy.retention_policy.retention_hours, 8760);
        assert!(policy.required_validations.contains(&ValidationType::ProofOfStake));
    }

    #[test]
    fn test_policy_enforcement_allow() {
        let mut manager = PolicyManager::new();
        let action = PolicyAction {
            action_type: ActionType::Connect,
            actor: None,
            target: None,
            provided_validations: HashSet::new(),
            queries_identity: false,
            queries_location: false,
            high_value: false,
        };

        let result = manager.enforce(PrivacyMode::ANONYMOUS, action);
        assert!(matches!(result, Ok(PolicyDecision::Allow)));
    }

    #[test]
    fn test_policy_enforcement_deny() {
        let mut manager = PolicyManager::new();
        let action = PolicyAction {
            action_type: ActionType::ValidateBlock,
            actor: None,
            target: None,
            provided_validations: HashSet::new(),
            queries_identity: false,
            queries_location: false,
            high_value: false,
        };

        let result = manager.enforce(PrivacyMode::ANONYMOUS, action);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_validation_enforcement() {
        let policy = TierPolicy::private_p2p();
        let action = PolicyAction {
            action_type: ActionType::ShareResource,
            actor: Some(NodeId([1u8; 32])),
            target: Some(NodeId([2u8; 32])),
            provided_validations: HashSet::new(),
            queries_identity: false,
            queries_location: false,
            high_value: false,
        };

        let result = policy.evaluate(action);
        assert!(matches!(result, Err(ref e) if e.violation_type == ViolationType::ValidationMissing));
    }

    #[test]
    fn test_conditional_approval() {
        let policy = TierPolicy::private_p2p();
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

        let result = policy.evaluate(action);
        assert!(matches!(result, Ok(PolicyDecision::AllowWithConditions(_))));
    }

    #[test]
    fn test_rate_limiting() {
        let mut rate_limits = RateLimits::default();
        let action = PolicyAction {
            action_type: ActionType::Connect,
            actor: None,
            target: None,
            provided_validations: HashSet::new(),
            queries_identity: false,
            queries_location: false,
            high_value: false,
        };

        // Anonymous mode has very restrictive limits
        assert!(rate_limits.check_rate(PrivacyMode::ANONYMOUS, &action));
        assert!(!rate_limits.check_rate(PrivacyMode::ANONYMOUS, &action)); // Second attempt fails
    }

    #[test]
    fn test_enforcement_statistics() {
        let mut manager = PolicyManager::new();
        let action = PolicyAction {
            action_type: ActionType::Connect,
            actor: None,
            target: None,
            provided_validations: HashSet::new(),
            queries_identity: false,
            queries_location: false,
            high_value: false,
        };

        manager.enforce(PrivacyMode::ANONYMOUS, action.clone()).unwrap();
        // Second call should fail due to rate limit, but we need to handle the error
        let _ = manager.enforce(PrivacyMode::ANONYMOUS, action);

        let stats = manager.stats();
        assert_eq!(stats.total_checks, 2);
        assert_eq!(stats.allowed, 1); // First succeeds
        assert_eq!(stats.violations, 1); // Second fails due to rate limit
    }

    #[test]
    fn test_access_rules_enforcement() {
        let policy = TierPolicy::anonymous();
        let action = PolicyAction {
            action_type: ActionType::QueryPublic,
            actor: None,
            target: None,
            provided_validations: HashSet::new(),
            queries_identity: true, // This violates anonymous policy
            queries_location: false,
            high_value: false,
        };

        let result = policy.evaluate(action);
        assert!(matches!(result, Err(ref e) if e.violation_type == ViolationType::UnauthorizedAccess));
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Low < Severity::Medium);
        assert!(Severity::Medium < Severity::High);
        assert!(Severity::High < Severity::Critical);
    }
}
