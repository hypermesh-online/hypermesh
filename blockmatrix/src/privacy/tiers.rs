// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// Privacy tiers implementation for Block-MATRIX
// Uses hypermesh_lib::PrivacyMode as canonical type

use hypermesh_lib::{AccessScope, PrivacyMode};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// NodeId in the privacy/tiers module is a 32-byte content hash used for
/// peer identification and public-tier node identity. Re-exported from lib.
pub use hypermesh_lib::ContentHash as NodeId;

/// NetworkId represents a unique identifier for a federated network.
/// Re-exported from lib (128-bit, compatible with UUID bytes).
pub use hypermesh_lib::NetworkId;

/// Trust level for federated networks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Minimal trust - basic connectivity only
    Basic,
    /// Medium trust - resource sharing allowed
    Standard,
    /// High trust - full cooperation
    Premium,
    /// Maximum trust - partner network
    Partner,
}

/// Validation requirements for a privacy tier
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationRequirements {
    pub proof_of_space: bool,
    pub proof_of_stake: bool,
    pub proof_of_work: bool,
    pub proof_of_time: bool,
    pub peer_validation: bool,
    pub federation_validation: bool,
}

impl ValidationRequirements {
    /// No validation required (anonymous mode)
    pub fn none() -> Self {
        Self {
            proof_of_space: false,
            proof_of_stake: false,
            proof_of_work: false,
            proof_of_time: false,
            peer_validation: false,
            federation_validation: false,
        }
    }

    /// Peer validation only (private mode)
    pub fn peer_only() -> Self {
        Self {
            proof_of_space: false,
            proof_of_stake: false,
            proof_of_work: false,
            proof_of_time: false,
            peer_validation: true,
            federation_validation: false,
        }
    }

    /// Federation validation (legacy -- now merged into peer_only for PRIVATE)
    pub fn federation() -> Self {
        Self {
            proof_of_space: true,
            proof_of_stake: true,
            proof_of_work: false,
            proof_of_time: true,
            peer_validation: false,
            federation_validation: true,
        }
    }

    /// Full validation (public mode)
    pub fn full() -> Self {
        Self {
            proof_of_space: true,
            proof_of_stake: true,
            proof_of_work: true,
            proof_of_time: true,
            peer_validation: false,
            federation_validation: false,
        }
    }
}

/// Derive validation requirements from a PrivacyMode.
///
/// - ANONYMOUS (untracked) => none
/// - PRIVATE (Bounded + tracked) => peer_only
/// - PUBLIC (Unbounded + tracked) => full
pub fn validation_requirements_for(mode: &PrivacyMode) -> ValidationRequirements {
    if !mode.tracked {
        return ValidationRequirements::none();
    }
    if mode.scope == AccessScope::Bounded {
        return ValidationRequirements::peer_only();
    }
    ValidationRequirements::full()
}

/// Anonymous tier implementation - Zero identity tracking
#[derive(Debug, Clone)]
pub struct AnonymousTier {
    /// Connection ID for routing (ephemeral)
    connection_id: [u8; 16],
    /// Whether to use Tor-like routing
    use_onion_routing: bool,
    /// Maximum hop count for routing
    max_hops: u8,
}

impl AnonymousTier {
    pub fn new() -> Self {
        let mut connection_id = [0u8; 16];
        // Generate random connection ID
        for byte in connection_id.iter_mut() {
            *byte = rand::random();
        }

        Self {
            connection_id,
            use_onion_routing: true,
            max_hops: 3,
        }
    }

    /// Create with specific routing settings
    pub fn with_routing(use_onion: bool, max_hops: u8) -> Self {
        let mut tier = Self::new();
        tier.use_onion_routing = use_onion;
        tier.max_hops = max_hops.min(7); // Cap at 7 hops
        tier
    }

    /// Get the ephemeral connection ID
    pub fn connection_id(&self) -> &[u8; 16] {
        &self.connection_id
    }

    /// Rotate connection ID for enhanced privacy
    pub fn rotate_identity(&mut self) {
        for byte in self.connection_id.iter_mut() {
            *byte = rand::random();
        }
    }
}

impl Default for AnonymousTier {
    fn default() -> Self {
        Self::new()
    }
}

/// Private P2P tier - Trusted peer circles
#[derive(Debug, Clone)]
pub struct PrivateP2PTier {
    /// Set of trusted peer node IDs
    pub trusted_peers: HashSet<NodeId>,
    /// Peer validator for trust verification
    pub peer_validator: PeerValidator,
    /// Maximum number of trusted peers
    pub max_peers: usize,
    /// Minimum required validating peers
    pub min_validators: usize,
}

impl PrivateP2PTier {
    pub fn new(max_peers: usize) -> Self {
        Self {
            trusted_peers: HashSet::new(),
            peer_validator: PeerValidator::new(),
            max_peers,
            min_validators: 1,
        }
    }

    /// Add a trusted peer
    pub fn add_peer(&mut self, peer_id: NodeId) -> Result<(), String> {
        if self.trusted_peers.len() >= self.max_peers {
            return Err("Maximum peer limit reached".to_string());
        }
        self.trusted_peers.insert(peer_id);
        Ok(())
    }

    /// Remove a trusted peer
    pub fn remove_peer(&mut self, peer_id: &NodeId) -> bool {
        self.trusted_peers.remove(peer_id)
    }

    /// Check if a peer is trusted
    pub fn is_trusted(&self, peer_id: &NodeId) -> bool {
        self.trusted_peers.contains(peer_id)
    }

    /// Validate a transaction with trusted peers
    pub fn validate_with_peers(&self, required_validators: usize) -> bool {
        self.trusted_peers.len() >= required_validators
    }
}

/// Peer validator for P2P trust verification
#[derive(Debug, Clone)]
pub struct PeerValidator {
    /// Validation threshold (percentage of peers required)
    pub threshold: f32,
    /// Timeout for peer responses (milliseconds)
    pub timeout_ms: u64,
}

impl PeerValidator {
    pub fn new() -> Self {
        Self {
            threshold: 0.51, // Majority required
            timeout_ms: 5000,
        }
    }

    /// Validate with specific threshold
    pub fn with_threshold(threshold: f32) -> Self {
        Self {
            threshold: threshold.clamp(0.0, 1.0),
            timeout_ms: 5000,
        }
    }
}

impl Default for PeerValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Federated tier - Partner network trust
#[derive(Debug, Clone)]
pub struct FederatedTier {
    /// Partner networks with their trust levels
    pub partner_networks: HashMap<NetworkId, TrustLevel>,
    /// Federation validator
    pub federation_validator: FederationValidator,
    /// Maximum number of partner networks
    pub max_partners: usize,
    /// Minimum required trust level for transactions
    pub min_trust_level: TrustLevel,
}

impl FederatedTier {
    pub fn new(max_partners: usize) -> Self {
        Self {
            partner_networks: HashMap::new(),
            federation_validator: FederationValidator::new(),
            max_partners,
            min_trust_level: TrustLevel::Standard,
        }
    }

    /// Add a partner network
    pub fn add_partner(&mut self, network_id: NetworkId, trust: TrustLevel) -> Result<(), String> {
        if self.partner_networks.len() >= self.max_partners {
            return Err("Maximum partner limit reached".to_string());
        }
        self.partner_networks.insert(network_id, trust);
        Ok(())
    }

    /// Update trust level for a partner
    pub fn update_trust(&mut self, network_id: &NetworkId, trust: TrustLevel) {
        if let Some(level) = self.partner_networks.get_mut(network_id) {
            *level = trust;
        }
    }

    /// Check if network meets minimum trust requirement
    pub fn meets_trust_requirement(&self, network_id: &NetworkId) -> bool {
        self.partner_networks
            .get(network_id)
            .map(|&trust| trust >= self.min_trust_level)
            .unwrap_or(false)
    }
}

/// Federation validator for cross-network trust
#[derive(Debug, Clone)]
pub struct FederationValidator {
    /// Required verification response threshold across federation partners
    pub verification_threshold: f32,
    /// Timeout for federation responses (milliseconds)
    pub timeout_ms: u64,
    /// Require cryptographic proof from partners
    pub require_proof: bool,
}

impl FederationValidator {
    pub fn new() -> Self {
        Self {
            verification_threshold: 0.67, // 2/3 majority
            timeout_ms: 10000,
            require_proof: true,
        }
    }
}

impl Default for FederationValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Public tier - Full transparency with Proof of State validation
#[derive(Debug, Clone)]
pub struct PublicTier {
    /// Proof of State validator
    pub pos_validator: ProofOfStateValidator,
    /// Public node identifier
    pub node_id: NodeId,
    /// Whether this node has passed Proof of State authentication (binary)
    pub authenticated: bool,
    /// Total validated transactions
    pub validated_count: u64,
}

impl PublicTier {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            pos_validator: ProofOfStateValidator::new(),
            node_id,
            authenticated: false,
            validated_count: 0,
        }
    }

    /// Set authentication status based on Proof of State validation (binary pass/fail)
    pub fn set_authenticated(&mut self, valid: bool) {
        self.authenticated = valid;
        if valid {
            self.validated_count += 1;
        }
    }

    /// Get CAESAR reward multiplier: authenticated nodes get full bonus, others get base
    pub fn caesar_bonus(&self) -> f32 {
        if self.authenticated {
            1.5
        } else {
            1.0
        }
    }
}

/// Proof of State validator for public tier
#[derive(Debug, Clone)]
pub struct ProofOfStateValidator {
    /// Enable WHO validation (Proof of Stake)
    pub validate_who: bool,
    /// Enable WHAT validation (Proof of Work)
    pub validate_what: bool,
    /// Enable WHEN validation (Proof of Time)
    pub validate_when: bool,
    /// Enable WHERE validation (Proof of Space)
    pub validate_where: bool,
}

impl ProofOfStateValidator {
    pub fn new() -> Self {
        Self {
            validate_who: true,
            validate_what: true,
            validate_when: true,
            validate_where: true,
        }
    }

    /// Check if all validations are enabled
    pub fn is_full_validation(&self) -> bool {
        self.validate_who && self.validate_what && self.validate_when && self.validate_where
    }
}

impl Default for ProofOfStateValidator {
    fn default() -> Self {
        Self::new()
    }
}

// Use rand crate for random number generation
use rand;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privacy_mode_caesar_multipliers() {
        assert_eq!(PrivacyMode::ANONYMOUS.caesar_multiplier(), 0.0);
        assert_eq!(PrivacyMode::PRIVATE.caesar_multiplier(), 0.5);
        assert_eq!(PrivacyMode::PUBLIC.caesar_multiplier(), 1.0);
    }

    #[test]
    fn test_privacy_mode_identity_requirements() {
        assert!(!PrivacyMode::ANONYMOUS.requires_identity());
        assert!(PrivacyMode::PRIVATE.requires_identity());
        assert!(PrivacyMode::PUBLIC.requires_identity());
    }

    #[test]
    fn test_anonymous_tier_creation() {
        let tier = AnonymousTier::new();
        assert!(tier.use_onion_routing);
        assert_eq!(tier.max_hops, 3);
    }

    #[test]
    fn test_anonymous_tier_identity_rotation() {
        let mut tier = AnonymousTier::new();
        let initial_id = tier.connection_id;
        tier.rotate_identity();
        assert_ne!(initial_id, tier.connection_id);
    }

    #[test]
    fn test_private_p2p_tier_peer_management() {
        let mut tier = PrivateP2PTier::new(5);
        let peer1 = NodeId([1u8; 32]);
        let peer2 = NodeId([2u8; 32]);

        assert!(tier.add_peer(peer1).is_ok());
        assert!(tier.is_trusted(&peer1));
        assert!(!tier.is_trusted(&peer2));

        assert!(tier.remove_peer(&peer1));
        assert!(!tier.is_trusted(&peer1));
    }

    #[test]
    fn test_private_p2p_tier_max_peers() {
        let mut tier = PrivateP2PTier::new(2);
        assert!(tier.add_peer(NodeId([1u8; 32])).is_ok());
        assert!(tier.add_peer(NodeId([2u8; 32])).is_ok());
        assert!(tier.add_peer(NodeId([3u8; 32])).is_err());
    }

    #[test]
    fn test_federated_tier_partner_management() {
        let mut tier = FederatedTier::new(10);
        let network1 = NetworkId([1u8; 16]);

        assert!(tier.add_partner(network1, TrustLevel::Partner).is_ok());
        assert!(tier.meets_trust_requirement(&network1));

        tier.update_trust(&network1, TrustLevel::Basic);
        assert!(!tier.meets_trust_requirement(&network1));
    }

    #[test]
    fn test_federated_tier_trust_levels() {
        let _tier = FederatedTier::new(10);
        assert!(TrustLevel::Partner >= TrustLevel::Premium);
        assert!(TrustLevel::Premium >= TrustLevel::Standard);
        assert!(TrustLevel::Standard >= TrustLevel::Basic);
    }

    #[test]
    fn test_public_tier_authentication() {
        let mut tier = PublicTier::new(NodeId([0u8; 32]));
        assert!(!tier.authenticated);

        tier.set_authenticated(true);
        assert!(tier.authenticated);
        assert_eq!(tier.validated_count, 1);

        tier.set_authenticated(false);
        assert!(!tier.authenticated);
    }

    #[test]
    fn test_public_tier_caesar_bonus() {
        let mut tier = PublicTier::new(NodeId([0u8; 32]));
        tier.authenticated = true;
        assert_eq!(tier.caesar_bonus(), 1.5);

        tier.authenticated = false;
        assert_eq!(tier.caesar_bonus(), 1.0);
    }

    #[test]
    fn test_validation_requirements_for_modes() {
        let anon_req = validation_requirements_for(&PrivacyMode::ANONYMOUS);
        assert!(!anon_req.proof_of_stake);
        assert!(!anon_req.peer_validation);

        let private_req = validation_requirements_for(&PrivacyMode::PRIVATE);
        assert!(private_req.peer_validation);
        assert!(!private_req.proof_of_stake);

        let pub_req = validation_requirements_for(&PrivacyMode::PUBLIC);
        assert!(pub_req.proof_of_stake);
        assert!(pub_req.proof_of_work);
        assert!(pub_req.proof_of_time);
        assert!(pub_req.proof_of_space);
    }

    #[test]
    fn test_proof_of_state_validator() {
        let validator = ProofOfStateValidator::new();
        assert!(validator.is_full_validation());

        let mut partial = ProofOfStateValidator::new();
        partial.validate_what = false;
        assert!(!partial.is_full_validation());
    }

    #[test]
    fn test_privacy_mode_constants() {
        let tier: PrivacyMode = PrivacyMode::PUBLIC;
        assert_eq!(tier, PrivacyMode::PUBLIC);
        assert_eq!(tier.caesar_multiplier(), 1.0);
    }
}
