// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

// Privacy Flexibility Matrix - Independent network and asset privacy settings
// Enables flexible privacy configurations where network and asset tiers can differ

use super::tiers::{PrivacyTier, ValidationRequirements};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Asset identifier type
// TODO: Migrate to hypermesh_lib::AssetId once field compatibility is resolved
// (lib uses AssetId(pub String), this uses type alias to [u8; 32])
pub type AssetId = [u8; 32];

/// Privacy Flexibility Matrix - Core of the flexible privacy system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivacyFlexibilityMatrix {
    /// How the node appears on the network
    pub network_tier: PrivacyTier,
    /// How assets are shared and accessed
    pub asset_tier: PrivacyTier,
    /// Per-asset overrides for fine-grained control
    pub asset_overrides: HashMap<AssetId, PrivacyTier>,
    /// Network visibility settings
    pub network_visibility: NetworkVisibility,
    /// Asset sharing settings
    pub asset_sharing: AssetSharing,
}

impl PrivacyFlexibilityMatrix {
    /// Create a new privacy flexibility matrix with uniform settings
    pub fn uniform(tier: PrivacyTier) -> Self {
        Self {
            network_tier: tier,
            asset_tier: tier,
            asset_overrides: HashMap::new(),
            network_visibility: NetworkVisibility::from_tier(tier),
            asset_sharing: AssetSharing::from_tier(tier),
        }
    }

    /// Create with independent network and asset tiers
    pub fn new(network_tier: PrivacyTier, asset_tier: PrivacyTier) -> Self {
        Self {
            network_tier,
            asset_tier,
            asset_overrides: HashMap::new(),
            network_visibility: NetworkVisibility::from_tier(network_tier),
            asset_sharing: AssetSharing::from_tier(asset_tier),
        }
    }

    /// Set a specific privacy tier for an individual asset
    pub fn set_asset_override(&mut self, asset_id: AssetId, tier: PrivacyTier) {
        self.asset_overrides.insert(asset_id, tier);
    }

    /// Get the effective privacy tier for a specific asset
    pub fn get_asset_tier(&self, asset_id: &AssetId) -> PrivacyTier {
        self.asset_overrides
            .get(asset_id)
            .copied()
            .unwrap_or(self.asset_tier)
    }

    /// Remove an asset override, reverting to default asset tier
    pub fn remove_asset_override(&mut self, asset_id: &AssetId) -> Option<PrivacyTier> {
        self.asset_overrides.remove(asset_id)
    }

    /// Check if configuration allows anonymous network with public assets
    pub fn is_anonymous_public(&self) -> bool {
        self.network_tier == PrivacyTier::Anonymous && self.asset_tier == PrivacyTier::Public
    }

    /// Check if configuration is privacy-focused (both tiers private or anonymous)
    pub fn is_privacy_focused(&self) -> bool {
        matches!(self.network_tier, PrivacyTier::Anonymous | PrivacyTier::PrivateP2P)
            && matches!(self.asset_tier, PrivacyTier::Anonymous | PrivacyTier::PrivateP2P)
    }

    /// Calculate combined CAESAR rewards multiplier
    pub fn caesar_multiplier(&self) -> f64 {
        // Average of network and asset tiers, with bonus for public sharing
        let network_mult = self.network_tier.caesar_multiplier();
        let asset_mult = self.asset_tier.caesar_multiplier();

        let base = (network_mult + asset_mult) / 2.0;

        // Bonus for mixed configurations that contribute to network
        if self.is_anonymous_public() {
            base * 1.2 // 20% bonus for anonymous nodes sharing publicly
        } else {
            base
        }
    }

    /// Get combined validation requirements
    pub fn combined_requirements(&self) -> ValidationRequirements {
        let net_req = self.network_tier.validation_requirements();
        let asset_req = self.asset_tier.validation_requirements();

        // Use the stricter requirement for each validation type
        ValidationRequirements {
            proof_of_space: net_req.proof_of_space || asset_req.proof_of_space,
            proof_of_stake: net_req.proof_of_stake || asset_req.proof_of_stake,
            proof_of_work: net_req.proof_of_work || asset_req.proof_of_work,
            proof_of_time: net_req.proof_of_time || asset_req.proof_of_time,
            peer_validation: net_req.peer_validation || asset_req.peer_validation,
            federation_validation: net_req.federation_validation || asset_req.federation_validation,
        }
    }

    /// Validate a privacy configuration for consistency
    pub fn validate_configuration(&self) -> Result<(), ValidationError> {
        // Check for invalid combinations
        if self.network_tier == PrivacyTier::Anonymous
            && self.asset_tier == PrivacyTier::PrivateP2P {
            return Err(ValidationError::InvalidCombination(
                "Cannot have anonymous network with private P2P assets (no identity for peer trust)".into()
            ));
        }

        // Warn about reduced rewards for certain combinations
        if self.caesar_multiplier() < 0.2 {
            return Err(ValidationError::LowRewards(
                "Configuration results in very low CAESAR rewards".into()
            ));
        }

        Ok(())
    }

    /// Get a privacy score (0.0 = no privacy, 1.0 = maximum privacy)
    pub fn privacy_score(&self) -> f32 {
        let network_score = match self.network_tier {
            PrivacyTier::Anonymous => 1.0,
            PrivacyTier::PrivateP2P => 0.7,
            PrivacyTier::Federated => 0.4,
            PrivacyTier::Public => 0.0,
        };

        let asset_score = match self.asset_tier {
            PrivacyTier::Anonymous => 1.0,
            PrivacyTier::PrivateP2P => 0.7,
            PrivacyTier::Federated => 0.4,
            PrivacyTier::Public => 0.0,
        };

        (network_score + asset_score) / 2.0
    }

    /// Get an openness score (0.0 = closed, 1.0 = fully open)
    pub fn openness_score(&self) -> f32 {
        1.0 - self.privacy_score()
    }
}

/// Network visibility settings
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkVisibility {
    /// Whether node ID is visible
    pub show_node_id: bool,
    /// Whether location is visible
    pub show_location: bool,
    /// Whether resources are visible
    pub show_resources: bool,
    /// Whether metrics are visible
    pub show_metrics: bool,
}

impl NetworkVisibility {
    pub fn from_tier(tier: PrivacyTier) -> Self {
        match tier {
            PrivacyTier::Anonymous => Self {
                show_node_id: false,
                show_location: false,
                show_resources: false,
                show_metrics: false,
            },
            PrivacyTier::PrivateP2P => Self {
                show_node_id: true,
                show_location: false,
                show_resources: true,
                show_metrics: false,
            },
            PrivacyTier::Federated => Self {
                show_node_id: true,
                show_location: true,
                show_resources: true,
                show_metrics: true,
            },
            PrivacyTier::Public => Self {
                show_node_id: true,
                show_location: true,
                show_resources: true,
                show_metrics: true,
            },
        }
    }
}

/// Asset sharing settings
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetSharing {
    /// Whether assets are discoverable
    pub discoverable: bool,
    /// Whether asset metadata is visible
    pub show_metadata: bool,
    /// Whether asset content is accessible
    pub allow_access: bool,
    /// Whether usage metrics are tracked
    pub track_usage: bool,
}

impl AssetSharing {
    pub fn from_tier(tier: PrivacyTier) -> Self {
        match tier {
            PrivacyTier::Anonymous => Self {
                discoverable: false,
                show_metadata: false,
                allow_access: false,
                track_usage: false,
            },
            PrivacyTier::PrivateP2P => Self {
                discoverable: true,
                show_metadata: true,
                allow_access: true,
                track_usage: false,
            },
            PrivacyTier::Federated => Self {
                discoverable: true,
                show_metadata: true,
                allow_access: true,
                track_usage: true,
            },
            PrivacyTier::Public => Self {
                discoverable: true,
                show_metadata: true,
                allow_access: true,
                track_usage: true,
            },
        }
    }
}

/// Validation errors for privacy configurations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    InvalidCombination(String),
    LowRewards(String),
    SecurityRisk(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::InvalidCombination(msg) => write!(f, "Invalid combination: {}", msg),
            ValidationError::LowRewards(msg) => write!(f, "Low rewards warning: {}", msg),
            ValidationError::SecurityRisk(msg) => write!(f, "Security risk: {}", msg),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Privacy configuration presets for common use cases
pub struct PrivacyPresets;

impl PrivacyPresets {
    /// Maximum privacy configuration
    pub fn maximum_privacy() -> PrivacyFlexibilityMatrix {
        PrivacyFlexibilityMatrix::uniform(PrivacyTier::Anonymous)
    }

    /// Maximum rewards configuration
    pub fn maximum_rewards() -> PrivacyFlexibilityMatrix {
        PrivacyFlexibilityMatrix::uniform(PrivacyTier::Public)
    }

    /// Balanced privacy and rewards
    pub fn balanced() -> PrivacyFlexibilityMatrix {
        PrivacyFlexibilityMatrix::new(PrivacyTier::Federated, PrivacyTier::Federated)
    }

    /// Anonymous contributor (anonymous network, public assets)
    pub fn anonymous_contributor() -> PrivacyFlexibilityMatrix {
        PrivacyFlexibilityMatrix::new(PrivacyTier::Anonymous, PrivacyTier::Public)
    }

    /// Private group sharing
    pub fn private_group() -> PrivacyFlexibilityMatrix {
        PrivacyFlexibilityMatrix::uniform(PrivacyTier::PrivateP2P)
    }

    /// Federated partner
    pub fn federated_partner() -> PrivacyFlexibilityMatrix {
        PrivacyFlexibilityMatrix::new(PrivacyTier::Federated, PrivacyTier::Federated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uniform_matrix_creation() {
        let matrix = PrivacyFlexibilityMatrix::uniform(PrivacyTier::Public);
        assert_eq!(matrix.network_tier, PrivacyTier::Public);
        assert_eq!(matrix.asset_tier, PrivacyTier::Public);
    }

    #[test]
    fn test_independent_tiers() {
        let matrix = PrivacyFlexibilityMatrix::new(
            PrivacyTier::Anonymous,
            PrivacyTier::Public
        );
        assert_eq!(matrix.network_tier, PrivacyTier::Anonymous);
        assert_eq!(matrix.asset_tier, PrivacyTier::Public);
        assert!(matrix.is_anonymous_public());
    }

    #[test]
    fn test_asset_overrides() {
        let mut matrix = PrivacyFlexibilityMatrix::uniform(PrivacyTier::Public);
        let asset_id = [1u8; 32];

        matrix.set_asset_override(asset_id, PrivacyTier::Anonymous);
        assert_eq!(matrix.get_asset_tier(&asset_id), PrivacyTier::Anonymous);

        let other_asset = [2u8; 32];
        assert_eq!(matrix.get_asset_tier(&other_asset), PrivacyTier::Public);

        matrix.remove_asset_override(&asset_id);
        assert_eq!(matrix.get_asset_tier(&asset_id), PrivacyTier::Public);
    }

    #[test]
    fn test_caesar_multiplier_calculation() {
        let public_matrix = PrivacyFlexibilityMatrix::uniform(PrivacyTier::Public);
        assert_eq!(public_matrix.caesar_multiplier(), 1.0);

        let anon_matrix = PrivacyFlexibilityMatrix::uniform(PrivacyTier::Anonymous);
        assert_eq!(anon_matrix.caesar_multiplier(), 0.1);

        let anon_public = PrivacyFlexibilityMatrix::new(
            PrivacyTier::Anonymous,
            PrivacyTier::Public
        );
        // (0.1 + 1.0) / 2.0 * 1.2 = 0.66
        assert!((anon_public.caesar_multiplier() - 0.66).abs() < 0.01);
    }

    #[test]
    fn test_privacy_focused_detection() {
        let private_matrix = PrivacyFlexibilityMatrix::uniform(PrivacyTier::PrivateP2P);
        assert!(private_matrix.is_privacy_focused());

        let mixed_matrix = PrivacyFlexibilityMatrix::new(
            PrivacyTier::Anonymous,
            PrivacyTier::Public
        );
        assert!(!mixed_matrix.is_privacy_focused());
    }

    #[test]
    fn test_combined_requirements() {
        let matrix = PrivacyFlexibilityMatrix::new(
            PrivacyTier::PrivateP2P,
            PrivacyTier::Public
        );
        let reqs = matrix.combined_requirements();

        assert!(reqs.peer_validation); // From P2P
        assert!(reqs.proof_of_stake);  // From Public
        assert!(reqs.proof_of_work);   // From Public
    }

    #[test]
    fn test_configuration_validation() {
        let valid = PrivacyFlexibilityMatrix::uniform(PrivacyTier::Public);
        assert!(valid.validate_configuration().is_ok());

        let invalid = PrivacyFlexibilityMatrix::new(
            PrivacyTier::Anonymous,
            PrivacyTier::PrivateP2P
        );
        assert!(invalid.validate_configuration().is_err());
    }

    #[test]
    fn test_privacy_and_openness_scores() {
        let anon = PrivacyFlexibilityMatrix::uniform(PrivacyTier::Anonymous);
        assert_eq!(anon.privacy_score(), 1.0);
        assert_eq!(anon.openness_score(), 0.0);

        let public = PrivacyFlexibilityMatrix::uniform(PrivacyTier::Public);
        assert_eq!(public.privacy_score(), 0.0);
        assert_eq!(public.openness_score(), 1.0);

        let mixed = PrivacyFlexibilityMatrix::new(
            PrivacyTier::Anonymous,
            PrivacyTier::Public
        );
        assert_eq!(mixed.privacy_score(), 0.5);
        assert_eq!(mixed.openness_score(), 0.5);
    }

    #[test]
    fn test_network_visibility_settings() {
        let anon_vis = NetworkVisibility::from_tier(PrivacyTier::Anonymous);
        assert!(!anon_vis.show_node_id);
        assert!(!anon_vis.show_location);

        let public_vis = NetworkVisibility::from_tier(PrivacyTier::Public);
        assert!(public_vis.show_node_id);
        assert!(public_vis.show_location);
        assert!(public_vis.show_resources);
        assert!(public_vis.show_metrics);
    }

    #[test]
    fn test_asset_sharing_settings() {
        let anon_share = AssetSharing::from_tier(PrivacyTier::Anonymous);
        assert!(!anon_share.discoverable);
        assert!(!anon_share.allow_access);

        let public_share = AssetSharing::from_tier(PrivacyTier::Public);
        assert!(public_share.discoverable);
        assert!(public_share.show_metadata);
        assert!(public_share.allow_access);
        assert!(public_share.track_usage);
    }

    #[test]
    fn test_privacy_presets() {
        let max_privacy = PrivacyPresets::maximum_privacy();
        assert_eq!(max_privacy.privacy_score(), 1.0);

        let max_rewards = PrivacyPresets::maximum_rewards();
        assert_eq!(max_rewards.caesar_multiplier(), 1.0);

        let anon_contrib = PrivacyPresets::anonymous_contributor();
        assert!(anon_contrib.is_anonymous_public());

        let balanced = PrivacyPresets::balanced();
        assert_eq!(balanced.network_tier, PrivacyTier::Federated);
        assert_eq!(balanced.asset_tier, PrivacyTier::Federated);
    }
}