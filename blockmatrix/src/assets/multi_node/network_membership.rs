// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Multi-Network Participation - Network Membership Management
//!
//! Revolutionary Concept #4: Single node joins multiple isolated networks simultaneously
//!
//! This module coordinates with TrustChain for network identity and credentials,
//! while BlockMatrix handles asset routing across networks.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;

use super::PeerIdentity;
use crate::assets::core::{AssetError, AssetRegistration, AssetResult};

/// Canonical NetworkId from hypermesh-lib (newtype over [u8; 16]).
pub use hypermesh_lib::NetworkId;

/// Network membership information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkMembership {
    /// Network identifier
    pub network_id: NetworkId,
    /// Network name (e.g., "Bank Customer Portal", "Employee VPN")
    pub network_name: String,
    /// Membership status
    pub status: MembershipStatus,
    /// When joined
    pub joined_at: SystemTime,
    /// Last activity
    pub last_active: SystemTime,
    /// Network-specific credentials (from TrustChain)
    pub credentials: NetworkCredentials,
    /// Privacy tier for this network
    pub privacy_tier: PrivacyMode,
    /// Assets visible in this network
    pub visible_assets: HashSet<AssetRegistration>,
    /// Role in this network
    pub role: NetworkRole,
}

/// Membership status
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MembershipStatus {
    /// Pending approval
    Pending,
    /// Active member
    Active,
    /// Suspended temporarily
    Suspended,
    /// Left gracefully
    Left,
    /// Kicked/banned
    Banned,
}

/// Network credentials (managed by TrustChain)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkCredentials {
    /// TrustChain certificate for this network
    pub certificate: Vec<u8>,
    /// Public key for network
    pub public_key: Vec<u8>,
    /// Private key (encrypted)
    pub private_key_encrypted: Vec<u8>,
    /// Session tokens
    pub session_tokens: Vec<SessionToken>,
    /// Expiration time
    pub expires_at: SystemTime,
}

/// Session token for network access
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionToken {
    /// Token value
    pub token: Vec<u8>,
    /// Issued at
    pub issued_at: SystemTime,
    /// Expires at
    pub expires_at: SystemTime,
    /// Permissions granted
    pub permissions: HashSet<String>,
}

/// Re-export canonical PrivacyMode from hypermesh-lib.
pub use hypermesh_lib::PrivacyMode;

/// Network role
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NetworkRole {
    /// Regular member
    Member,
    /// Moderator with elevated privileges
    Moderator,
    /// Administrator
    Admin,
    /// Network owner
    Owner,
    /// Guest with limited access
    Guest,
}

/// Network discovery service
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkDiscovery {
    /// Network ID
    pub network_id: NetworkId,
    /// Network name
    pub name: String,
    /// Description
    pub description: String,
    /// Entry point nodes
    pub entry_points: Vec<PeerIdentity>,
    /// Requirements for joining
    pub requirements: JoinRequirements,
    /// Privacy tier
    pub privacy_tier: PrivacyMode,
    /// Number of active members
    pub member_count: u64,
    /// Whether network is public
    pub is_public: bool,
}

/// Requirements for joining a network
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JoinRequirements {
    /// Requires invitation
    pub invitation_required: bool,
    /// Whether joining requires a valid Proof of State
    pub requires_state_proof: bool,
    /// Required proofs
    pub required_proofs: HashSet<NetworkProofType>,
    /// Geographic restrictions
    pub geo_restrictions: Option<GeoRestriction>,
    /// Approval process
    pub approval_process: ApprovalProcess,
}

/// Domain-specific proof type with Identity variant; canonical ProofType in hypermesh_lib.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetworkProofType {
    /// Proof of Space
    Space,
    /// Proof of Stake
    Stake,
    /// Proof of Work
    Work,
    /// Proof of Time
    Time,
    /// Identity verification (network-specific, not in canonical ProofType)
    Identity,
}

/// Geographic restrictions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeoRestriction {
    /// Allowed countries (ISO codes)
    pub allowed_countries: HashSet<String>,
    /// Denied countries (ISO codes)
    pub denied_countries: HashSet<String>,
    /// Required regions
    pub required_regions: HashSet<String>,
}

/// Approval process for joining
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ApprovalProcess {
    /// Automatic approval
    Automatic,
    /// Manual approval by admin
    ManualAdmin,
    /// Joining node must pass full Proof of State (binary: authentic or not)
    StateProofRequired,
    /// Smart contract verification
    SmartContract { contract_address: Vec<u8> },
}

/// Multi-network coordinator - manages membership across networks
pub struct MultiNetworkMembership {
    /// Node ID
    _local_node: PeerIdentity,
    /// Current memberships
    memberships: Arc<RwLock<HashMap<NetworkId, NetworkMembership>>>,
    /// Discovered networks
    discovered_networks: Arc<RwLock<HashMap<NetworkId, NetworkDiscovery>>>,
    /// TrustChain client for credentials (interface to TrustChain)
    trustchain_client: Arc<dyn TrustChainClient>,
}

/// TrustChain client interface
#[async_trait]
pub trait TrustChainClient: Send + Sync {
    /// Request credentials for a network
    async fn request_credentials(&self, network_id: NetworkId) -> AssetResult<NetworkCredentials>;

    /// Revoke credentials
    async fn revoke_credentials(&self, network_id: NetworkId) -> AssetResult<()>;

    /// Validate network certificate
    async fn validate_certificate(&self, cert: &[u8]) -> AssetResult<bool>;

    /// Discover available networks
    async fn discover_networks(&self) -> AssetResult<Vec<NetworkDiscovery>>;
}

impl MultiNetworkMembership {
    /// Create new multi-network membership manager
    pub fn new(local_node: PeerIdentity, trustchain_client: Arc<dyn TrustChainClient>) -> Self {
        Self {
            _local_node: local_node,
            memberships: Arc::new(RwLock::new(HashMap::new())),
            discovered_networks: Arc::new(RwLock::new(HashMap::new())),
            trustchain_client,
        }
    }

    /// Discover available networks
    pub async fn discover_networks(&self) -> AssetResult<Vec<NetworkDiscovery>> {
        let networks = self.trustchain_client.discover_networks().await?;

        let mut discovered = self.discovered_networks.write().await;
        for network in &networks {
            discovered.insert(network.network_id, network.clone());
        }

        Ok(networks)
    }

    /// Join a network
    pub async fn join_network(
        &self,
        network_id: NetworkId,
        privacy_tier: PrivacyMode,
    ) -> AssetResult<()> {
        // Check if already member
        {
            let memberships = self.memberships.read().await;
            if let Some(membership) = memberships.get(&network_id) {
                if membership.status == MembershipStatus::Active {
                    return Err(AssetError::NetworkError {
                        message: "Already member of this network".to_string(),
                    });
                }
            }
        }

        // Get network info
        let network_info = {
            let discovered = self.discovered_networks.read().await;
            discovered
                .get(&network_id)
                .cloned()
                .ok_or_else(|| AssetError::NetworkError {
                    message: "Network not found".to_string(),
                })?
        };

        // Request credentials from TrustChain
        let credentials = self
            .trustchain_client
            .request_credentials(network_id)
            .await?;

        // Create membership
        let membership = NetworkMembership {
            network_id,
            network_name: network_info.name,
            status: MembershipStatus::Active, // Active immediately after joining
            joined_at: SystemTime::now(),
            last_active: SystemTime::now(),
            credentials,
            privacy_tier,
            visible_assets: HashSet::new(),
            role: NetworkRole::Member,
        };

        // Store membership
        let mut memberships = self.memberships.write().await;
        memberships.insert(network_id, membership);

        tracing::info!(
            "Joined network {} with privacy tier {:?}",
            network_id,
            privacy_tier
        );

        Ok(())
    }

    /// Leave a network
    pub async fn leave_network(&self, network_id: NetworkId) -> AssetResult<()> {
        let mut memberships = self.memberships.write().await;

        if let Some(membership) = memberships.get_mut(&network_id) {
            membership.status = MembershipStatus::Left;
            membership.last_active = SystemTime::now();

            // Revoke credentials via TrustChain
            self.trustchain_client
                .revoke_credentials(network_id)
                .await?;

            tracing::info!("Left network {}", network_id);
            Ok(())
        } else {
            Err(AssetError::NetworkError {
                message: "Not a member of this network".to_string(),
            })
        }
    }

    /// Get active memberships
    pub async fn active_memberships(&self) -> Vec<NetworkMembership> {
        let memberships = self.memberships.read().await;
        memberships
            .values()
            .filter(|m| m.status == MembershipStatus::Active)
            .cloned()
            .collect()
    }

    /// Add asset to network visibility
    pub async fn add_asset_to_network(
        &self,
        network_id: NetworkId,
        asset_id: AssetRegistration,
    ) -> AssetResult<()> {
        let mut memberships = self.memberships.write().await;

        if let Some(membership) = memberships.get_mut(&network_id) {
            membership.visible_assets.insert(asset_id);
            Ok(())
        } else {
            Err(AssetError::NetworkError {
                message: "Not a member of this network".to_string(),
            })
        }
    }

    /// Check if asset is visible in network
    pub async fn is_asset_visible(
        &self,
        network_id: NetworkId,
        asset_id: &AssetRegistration,
    ) -> bool {
        let memberships = self.memberships.read().await;

        memberships
            .get(&network_id)
            .map(|m| m.visible_assets.contains(asset_id))
            .unwrap_or(false)
    }

    /// Get networks where asset is visible
    pub async fn networks_for_asset(&self, asset_id: &AssetRegistration) -> Vec<NetworkId> {
        let memberships = self.memberships.read().await;

        memberships
            .iter()
            .filter(|(_, m)| m.visible_assets.contains(asset_id))
            .map(|(id, _)| *id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_asset_id;
    use std::time::Duration;

    struct MockTrustChainClient;

    #[async_trait]
    impl TrustChainClient for MockTrustChainClient {
        async fn request_credentials(
            &self,
            _network_id: NetworkId,
        ) -> AssetResult<NetworkCredentials> {
            Ok(NetworkCredentials {
                certificate: vec![1, 2, 3],
                public_key: vec![4, 5, 6],
                private_key_encrypted: vec![7, 8, 9],
                session_tokens: vec![],
                expires_at: SystemTime::now() + Duration::from_secs(3600),
            })
        }

        async fn revoke_credentials(&self, _network_id: NetworkId) -> AssetResult<()> {
            Ok(())
        }

        async fn validate_certificate(&self, _cert: &[u8]) -> AssetResult<bool> {
            Ok(true)
        }

        async fn discover_networks(&self) -> AssetResult<Vec<NetworkDiscovery>> {
            Ok(vec![NetworkDiscovery {
                network_id: NetworkId([1u8; 16]),
                name: "Bank Customer Portal".to_string(),
                description: "Public banking services".to_string(),
                entry_points: vec![],
                requirements: JoinRequirements {
                    invitation_required: false,
                    requires_state_proof: true,
                    required_proofs: HashSet::new(),
                    geo_restrictions: None,
                    approval_process: ApprovalProcess::Automatic,
                },
                privacy_tier: PrivacyMode::PUBLIC,
                member_count: 1000,
                is_public: true,
            }])
        }
    }

    #[tokio::test]
    async fn test_network_discovery() {
        let node_id = PeerIdentity {
            name: "test-node".to_string(),
            id: [0u8; 32],
            address: "::1".parse().expect("test: valid ipv6"),
            pub_key: vec![],
        };

        let client = Arc::new(MockTrustChainClient);
        let membership = MultiNetworkMembership::new(node_id, client);

        let networks = membership
            .discover_networks()
            .await
            .expect("test: discover");
        assert_eq!(networks.len(), 1);
        assert_eq!(networks[0].name, "Bank Customer Portal");
    }

    #[tokio::test]
    async fn test_join_leave_network() {
        let node_id = PeerIdentity {
            name: "test-node".to_string(),
            id: [0u8; 32],
            address: "::1".parse().expect("test: valid ipv6"),
            pub_key: vec![],
        };

        let client = Arc::new(MockTrustChainClient);
        let membership = MultiNetworkMembership::new(node_id, client);

        // Discover networks first
        let networks = membership
            .discover_networks()
            .await
            .expect("test: discover");
        let network_id = networks[0].network_id;

        // Join network
        membership
            .join_network(network_id, PrivacyMode::PUBLIC)
            .await
            .expect("test: join");

        // Leave network
        membership
            .leave_network(network_id)
            .await
            .expect("test: leave");
    }

    #[tokio::test]
    async fn test_asset_visibility() {
        let node_id = PeerIdentity {
            name: "test-node".to_string(),
            id: [0u8; 32],
            address: "::1".parse().expect("test: valid ipv6"),
            pub_key: vec![],
        };

        let client = Arc::new(MockTrustChainClient);
        let membership = MultiNetworkMembership::new(node_id, client);

        // Discover and join network
        let networks = membership
            .discover_networks()
            .await
            .expect("test: discover");
        let network_id = networks[0].network_id;
        membership
            .join_network(network_id, PrivacyMode::PUBLIC)
            .await
            .expect("test: join");

        // Add asset to network
        use crate::assets::core::AssetType;
        let asset_id = test_asset_id(AssetType::Cpu);
        membership
            .add_asset_to_network(network_id, asset_id.clone())
            .await
            .expect("test: add asset");

        // Check visibility
        assert!(membership.is_asset_visible(network_id, &asset_id).await);

        // Get networks for asset
        let networks = membership.networks_for_asset(&asset_id).await;
        assert_eq!(networks.len(), 1);
        assert_eq!(networks[0], network_id);
    }
}
