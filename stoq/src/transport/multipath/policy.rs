// Copyright (C) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Multi-path policy enforcement engine.
//!
//! Validates paths against scope, federation, and privacy constraints.
//! This is the critical enforcement layer that ensures multi-path
//! connections respect BlockchainScope boundaries, federation nesting
//! hierarchies, and privacy-tier validation requirements.

use std::fmt;

use hypermesh_lib::{AccessScope, BlockchainScope, NetworkId, PrivacyMode};

/// Reason a path was rejected by the policy engine.
#[derive(Debug, Clone)]
pub enum PathRejectionReason {
    /// Device-scope paths must not span remote nodes.
    DeviceScopeRemoteNotAllowed,
    /// Cross-scope transfer attempted but no gateway is available.
    CrossScopeNoGateway,
    /// Cross-scope transfers are disabled by policy.
    CrossScopeDisabled,
    /// Target network is outside the federation chain.
    FederationBoundaryViolation { from: NetworkId, to: NetworkId },
    /// PoS validation failed with the given reason.
    PosValidationFailed(String),
    /// No tunnel configured between the two networks.
    TunnelNotConfigured { from: NetworkId, to: NetworkId },
    /// Maximum path count per connection exceeded.
    MaxPathsExceeded,
    /// Maximum path count per network exceeded.
    MaxPathsPerNetworkExceeded,
    /// The originating path id is not registered on this connection.
    UnknownPath { path_id: u32 },
}

impl fmt::Display for PathRejectionReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeviceScopeRemoteNotAllowed => {
                write!(f, "Device-scope paths cannot span remote nodes")
            }
            Self::CrossScopeNoGateway => {
                write!(f, "Cross-scope transfer requires a gateway node")
            }
            Self::CrossScopeDisabled => {
                write!(f, "Cross-scope transfers are disabled by policy")
            }
            Self::FederationBoundaryViolation { from, to } => {
                write!(
                    f,
                    "Federation boundary violation: {from} -> {to} not in chain"
                )
            }
            Self::PosValidationFailed(reason) => {
                write!(f, "PoS validation failed: {reason}")
            }
            Self::TunnelNotConfigured { from, to } => {
                write!(f, "No tunnel configured between {from} and {to}")
            }
            Self::MaxPathsExceeded => {
                write!(f, "Maximum paths per connection exceeded")
            }
            Self::MaxPathsPerNetworkExceeded => {
                write!(f, "Maximum paths per network exceeded")
            }
            Self::UnknownPath { path_id } => {
                write!(f, "Unknown path id {path_id} on this connection")
            }
        }
    }
}

/// Result of path validation.
#[derive(Debug)]
pub enum PathValidation {
    /// Path is allowed.
    Allowed,
    /// Path was rejected for the given reason.
    Rejected(PathRejectionReason),
}

impl PathValidation {
    /// Returns `true` if the validation result is `Allowed`.
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed)
    }
}

/// Level of Proof-of-State validation required for a privacy tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PosValidationLevel {
    /// No PoS validation required (Anonymous mode).
    None,
    /// Partial validation: stake and timestamp only (Private mode).
    Partial,
    /// Full four-proof validation (Public mode).
    Full,
}

/// Scope enforcement rules controlling Device/Network boundary semantics.
#[derive(Debug, Clone)]
pub struct ScopePolicy {
    /// Whether cross-scope transfers (Device <-> Network) are permitted.
    pub allow_cross_scope: bool,
    /// Whether Device-scope paths are restricted to local-only.
    pub device_scope_local_only: bool,
    /// Whether a gateway node is required for cross-scope transfers.
    pub require_gateway: bool,
}

impl Default for ScopePolicy {
    fn default() -> Self {
        Self {
            allow_cross_scope: false,
            device_scope_local_only: true,
            require_gateway: true,
        }
    }
}

/// Federation boundary enforcement rules.
#[derive(Debug, Clone)]
pub struct FederationPolicy {
    /// Whether federation boundary violations are enforced.
    pub enforce_boundaries: bool,
    /// Maximum depth of nested federation chains.
    pub max_nesting_depth: usize,
}

impl Default for FederationPolicy {
    fn default() -> Self {
        Self {
            enforce_boundaries: true,
            max_nesting_depth: 8,
        }
    }
}

/// Privacy-tier validation rules.
#[derive(Debug, Clone)]
pub struct PrivacyPolicy {
    /// Whether anonymous connections skip all PoS validation.
    pub anonymous_skip_validation: bool,
    /// Whether private connections require a peer certificate.
    pub private_require_peer_cert: bool,
    /// Whether public connections require full four-proof PoS.
    pub public_require_full_pos: bool,
}

impl Default for PrivacyPolicy {
    fn default() -> Self {
        Self {
            anonymous_skip_validation: true,
            private_require_peer_cert: true,
            public_require_full_pos: true,
        }
    }
}

/// Context for a send operation used in cross-network validation.
#[derive(Debug, Clone)]
pub struct SendContext {
    /// Network the sending path belongs to.
    pub network_id: NetworkId,
    /// Blockchain scope of the sending path.
    pub scope: BlockchainScope,
    /// Federation chain (nesting hierarchy) of the sending path.
    pub federation_chain: Vec<NetworkId>,
}

/// Combined path policy governing all multi-path constraints.
#[derive(Debug, Clone)]
pub struct PathPolicy {
    /// Scope enforcement rules.
    pub scope_policy: ScopePolicy,
    /// Federation boundary rules.
    pub federation_policy: FederationPolicy,
    /// Privacy-tier validation rules.
    pub privacy_policy: PrivacyPolicy,
    /// Maximum number of paths per multi-path connection.
    pub max_paths_per_connection: usize,
    /// Maximum number of paths to a single network.
    pub max_paths_per_network: usize,
}

impl Default for PathPolicy {
    fn default() -> Self {
        Self {
            scope_policy: ScopePolicy::default(),
            federation_policy: FederationPolicy::default(),
            privacy_policy: PrivacyPolicy::default(),
            max_paths_per_connection: 8,
            max_paths_per_network: 4,
        }
    }
}

impl PathPolicy {
    /// Create a new policy with all defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate whether a new path can be added to the connection.
    ///
    /// Checks path limits, scope constraints, and returns `Allowed` or
    /// a rejection reason.
    pub fn validate_path(
        &self,
        scope: &BlockchainScope,
        _privacy_mode: &PrivacyMode,
        _network_id: &NetworkId,
        _federation_chain: &[NetworkId],
        _gateway_node: &Option<String>,
        is_remote: bool,
        current_path_count: usize,
        paths_in_network: usize,
    ) -> PathValidation {
        // 1. Check max paths per connection
        if current_path_count >= self.max_paths_per_connection {
            return PathValidation::Rejected(PathRejectionReason::MaxPathsExceeded);
        }

        // 2. Check max paths per network
        if paths_in_network >= self.max_paths_per_network {
            return PathValidation::Rejected(PathRejectionReason::MaxPathsPerNetworkExceeded);
        }

        // 3. Device scope + remote = not allowed when local-only enforced
        if *scope == BlockchainScope::Device
            && is_remote
            && self.scope_policy.device_scope_local_only
        {
            return PathValidation::Rejected(PathRejectionReason::DeviceScopeRemoteNotAllowed);
        }

        PathValidation::Allowed
    }

    /// Validate whether a send from the given path context to a target
    /// network is permitted.
    pub fn validate_send(
        &self,
        from_path: &SendContext,
        target_network: &NetworkId,
    ) -> PathValidation {
        // Same network is always allowed
        if from_path.network_id == *target_network {
            return PathValidation::Allowed;
        }

        // Target within federation chain is allowed
        if Self::is_in_federation(&from_path.federation_chain, target_network) {
            return PathValidation::Allowed;
        }

        // Federation boundary enforcement
        if self.federation_policy.enforce_boundaries {
            return PathValidation::Rejected(PathRejectionReason::FederationBoundaryViolation {
                from: from_path.network_id,
                to: *target_network,
            });
        }

        // Not enforced but no tunnel — signal that a tunnel would be needed
        PathValidation::Rejected(PathRejectionReason::TunnelNotConfigured {
            from: from_path.network_id,
            to: *target_network,
        })
    }

    /// Validate a cross-scope transfer (Device <-> Network).
    pub fn validate_cross_scope(
        &self,
        from_scope: &BlockchainScope,
        to_scope: &BlockchainScope,
        gateway_node: &Option<String>,
    ) -> PathValidation {
        // Same scope is always allowed
        if from_scope == to_scope {
            return PathValidation::Allowed;
        }

        if !self.scope_policy.allow_cross_scope {
            return PathValidation::Rejected(PathRejectionReason::CrossScopeDisabled);
        }

        if self.scope_policy.require_gateway && gateway_node.is_none() {
            return PathValidation::Rejected(PathRejectionReason::CrossScopeNoGateway);
        }

        PathValidation::Allowed
    }

    /// Check whether the target network appears anywhere in the
    /// federation chain (nesting hierarchy).
    pub fn is_in_federation(chain: &[NetworkId], target: &NetworkId) -> bool {
        chain.iter().any(|n| n == target)
    }

    /// Determine the PoS validation level required for the given privacy mode.
    pub fn requires_pos_validation(&self, privacy_mode: &PrivacyMode) -> PosValidationLevel {
        // Anonymous: unbounded + untracked
        if privacy_mode.scope == AccessScope::Unbounded && !privacy_mode.tracked {
            return PosValidationLevel::None;
        }

        // Private: bounded + tracked
        if privacy_mode.scope == AccessScope::Bounded && privacy_mode.tracked {
            return PosValidationLevel::Partial;
        }

        // Public (or any other combination): full validation
        PosValidationLevel::Full
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_network() -> NetworkId {
        hypermesh_lib::DEFAULT_NETWORK
    }

    fn network(id: u8) -> NetworkId {
        NetworkId([id; 16])
    }

    #[test]
    fn test_device_scope_local_allowed() {
        let policy = PathPolicy::new();
        let result = policy.validate_path(
            &BlockchainScope::Device,
            &PrivacyMode::PUBLIC,
            &default_network(),
            &[],
            &None,
            false, // not remote
            0,
            0,
        );
        assert!(
            result.is_allowed(),
            "Device scope, local (not remote) should be allowed"
        );
    }

    #[test]
    fn test_device_scope_remote_rejected() {
        let policy = PathPolicy::new();
        let result = policy.validate_path(
            &BlockchainScope::Device,
            &PrivacyMode::PUBLIC,
            &default_network(),
            &[],
            &None,
            true, // remote
            0,
            0,
        );
        assert!(
            matches!(
                result,
                PathValidation::Rejected(PathRejectionReason::DeviceScopeRemoteNotAllowed)
            ),
            "Device scope, remote should be rejected"
        );
    }

    #[test]
    fn test_cross_scope_no_gateway() {
        let policy = PathPolicy {
            scope_policy: ScopePolicy {
                allow_cross_scope: true,
                require_gateway: true,
                ..ScopePolicy::default()
            },
            ..PathPolicy::default()
        };
        let result =
            policy.validate_cross_scope(&BlockchainScope::Device, &BlockchainScope::Network, &None);
        assert!(
            matches!(
                result,
                PathValidation::Rejected(PathRejectionReason::CrossScopeNoGateway)
            ),
            "Cross-scope without gateway should be rejected"
        );
    }

    #[test]
    fn test_cross_scope_with_gateway() {
        let policy = PathPolicy {
            scope_policy: ScopePolicy {
                allow_cross_scope: true,
                require_gateway: true,
                ..ScopePolicy::default()
            },
            ..PathPolicy::default()
        };
        let result = policy.validate_cross_scope(
            &BlockchainScope::Device,
            &BlockchainScope::Network,
            &Some("gateway-node-1".to_string()),
        );
        assert!(
            result.is_allowed(),
            "Cross-scope with gateway should be allowed"
        );
    }

    #[test]
    fn test_cross_scope_disabled() {
        let policy = PathPolicy {
            scope_policy: ScopePolicy {
                allow_cross_scope: false,
                ..ScopePolicy::default()
            },
            ..PathPolicy::default()
        };
        let result = policy.validate_cross_scope(
            &BlockchainScope::Device,
            &BlockchainScope::Network,
            &Some("gateway-node-1".to_string()),
        );
        assert!(
            matches!(
                result,
                PathValidation::Rejected(PathRejectionReason::CrossScopeDisabled)
            ),
            "Cross-scope when disabled should be rejected"
        );
    }

    #[test]
    fn test_federation_same_network() {
        let policy = PathPolicy::new();
        let net_a = network(1);
        let ctx = SendContext {
            network_id: net_a,
            scope: BlockchainScope::Network,
            federation_chain: vec![],
        };
        let result = policy.validate_send(&ctx, &net_a);
        assert!(
            result.is_allowed(),
            "Sending to the same network should be allowed"
        );
    }

    #[test]
    fn test_federation_in_chain() {
        let policy = PathPolicy::new();
        let net_a = network(1);
        let net_b = network(2);
        let ctx = SendContext {
            network_id: net_a,
            scope: BlockchainScope::Network,
            federation_chain: vec![net_a, net_b],
        };
        let result = policy.validate_send(&ctx, &net_b);
        assert!(
            result.is_allowed(),
            "Sending to a network in the federation chain should be allowed"
        );
    }

    #[test]
    fn test_federation_boundary_violation() {
        let policy = PathPolicy::new();
        let net_a = network(1);
        let net_c = network(3);
        let ctx = SendContext {
            network_id: net_a,
            scope: BlockchainScope::Network,
            federation_chain: vec![net_a],
        };
        let result = policy.validate_send(&ctx, &net_c);
        assert!(
            matches!(
                result,
                PathValidation::Rejected(PathRejectionReason::FederationBoundaryViolation { .. })
            ),
            "Sending outside federation chain should be rejected"
        );
    }

    #[test]
    fn test_nested_federation() {
        let policy = PathPolicy::new();
        let net_a = network(1);
        let net_a_sub1 = network(2);
        let net_b = network(3);

        let ctx = SendContext {
            network_id: net_a,
            scope: BlockchainScope::Network,
            federation_chain: vec![net_a, net_a_sub1],
        };

        // Target in nested chain should be allowed
        let result = policy.validate_send(&ctx, &net_a_sub1);
        assert!(
            result.is_allowed(),
            "Nested federation target should be allowed"
        );

        // Target outside chain should be rejected
        let result = policy.validate_send(&ctx, &net_b);
        assert!(
            matches!(
                result,
                PathValidation::Rejected(PathRejectionReason::FederationBoundaryViolation { .. })
            ),
            "Target outside nested federation should be rejected"
        );
    }

    #[test]
    fn test_privacy_validation_levels() {
        let policy = PathPolicy::new();

        assert_eq!(
            policy.requires_pos_validation(&PrivacyMode::ANONYMOUS),
            PosValidationLevel::None,
            "Anonymous should require no PoS validation"
        );
        assert_eq!(
            policy.requires_pos_validation(&PrivacyMode::PRIVATE),
            PosValidationLevel::Partial,
            "Private should require partial PoS validation"
        );
        assert_eq!(
            policy.requires_pos_validation(&PrivacyMode::PUBLIC),
            PosValidationLevel::Full,
            "Public should require full PoS validation"
        );
    }

    #[test]
    fn test_max_paths_exceeded() {
        let policy = PathPolicy {
            max_paths_per_connection: 2,
            ..PathPolicy::default()
        };

        // First two paths should be allowed
        let r1 = policy.validate_path(
            &BlockchainScope::Network,
            &PrivacyMode::PUBLIC,
            &default_network(),
            &[],
            &None,
            false,
            0,
            0,
        );
        assert!(r1.is_allowed());

        let r2 = policy.validate_path(
            &BlockchainScope::Network,
            &PrivacyMode::PUBLIC,
            &default_network(),
            &[],
            &None,
            false,
            1,
            0,
        );
        assert!(r2.is_allowed());

        // Third should be rejected
        let r3 = policy.validate_path(
            &BlockchainScope::Network,
            &PrivacyMode::PUBLIC,
            &default_network(),
            &[],
            &None,
            false,
            2,
            0,
        );
        assert!(
            matches!(
                r3,
                PathValidation::Rejected(PathRejectionReason::MaxPathsExceeded)
            ),
            "Third path should exceed max_paths_per_connection"
        );
    }
}
