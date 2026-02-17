// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Multi-Network Participation - Network Membership Management
//!
//! Revolutionary Concept #4: Single node joins multiple isolated networks simultaneously
//!
//! This module coordinates with TrustChain for network identity and credentials,
//! while BlockMatrix handles asset routing across networks.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use async_trait::async_trait;

use crate::assets::core::{AssetId, AssetResult, AssetError};
use super::NodeId;

/// Network identifier (maps to TrustChain NetworkId)
pub type NetworkId = [u8; 16];

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
    pub privacy_tier: PrivacyTier,
    /// Assets visible in this network
    pub visible_assets: HashSet<AssetId>,
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

/// Privacy tier for network (maps to blockmatrix/src/privacy/tiers.rs)
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PrivacyTier {
    /// Anonymous - Zero identity tracking
    Anonymous,
    /// Private P2P - Trusted peer circles
    PrivateP2P,
    /// Federated - Cross-network partner trust
    Federated,
    /// Public - Full transparency with PoS validation
    Public,
}

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
    pub entry_points: Vec<NodeId>,
    /// Requirements for joining
    pub requirements: JoinRequirements,
    /// Privacy tier
    pub privacy_tier: PrivacyTier,
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
    /// Minimum reputation score
    pub min_reputation: Option<f64>,
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
    /// Vote by existing members
    MemberVote { threshold: f64 },
    /// Smart contract verification
    SmartContract { contract_address: Vec<u8> },
}

/// Multi-network coordinator - manages membership across networks
#[allow(dead_code)] // Fields used during network membership management
pub struct MultiNetworkMembership {
    /// Node ID
    local_node: NodeId,
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
    pub fn new(local_node: NodeId, trustchain_client: Arc<dyn TrustChainClient>) -> Self {
        Self {
            local_node,
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
        privacy_tier: PrivacyTier,
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
            discovered.get(&network_id).cloned()
                .ok_or_else(|| AssetError::NetworkError {
                    message: "Network not found".to_string(),
                })?
        };

        // Request credentials from TrustChain
        let credentials = self.trustchain_client
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
            hex::encode(&network_id),
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
            self.trustchain_client.revoke_credentials(network_id).await?;

            tracing::info!("Left network {}", hex::encode(&network_id));
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
        memberships.values()
            .filter(|m| m.status == MembershipStatus::Active)
            .cloned()
            .collect()
    }

    /// Add asset to network visibility
    pub async fn add_asset_to_network(
        &self,
        network_id: NetworkId,
        asset_id: AssetId,
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
        asset_id: &AssetId,
    ) -> bool {
        let memberships = self.memberships.read().await;

        memberships.get(&network_id)
            .map(|m| m.visible_assets.contains(asset_id))
            .unwrap_or(false)
    }

    /// Get networks where asset is visible
    pub async fn networks_for_asset(&self, asset_id: &AssetId) -> Vec<NetworkId> {
        let memberships = self.memberships.read().await;

        memberships.iter()
            .filter(|(_, m)| m.visible_assets.contains(asset_id))
            .map(|(id, _)| *id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::test_utils::test_asset_id;

    struct MockTrustChainClient;

    #[async_trait]
    impl TrustChainClient for MockTrustChainClient {
        async fn request_credentials(&self, _network_id: NetworkId) -> AssetResult<NetworkCredentials> {
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
            Ok(vec![
                NetworkDiscovery {
                    network_id: [1u8; 16],
                    name: "Bank Customer Portal".to_string(),
                    description: "Public banking services".to_string(),
                    entry_points: vec![],
                    requirements: JoinRequirements {
                        invitation_required: false,
                        min_reputation: None,
                        required_proofs: HashSet::new(),
                        geo_restrictions: None,
                        approval_process: ApprovalProcess::Automatic,
                    },
                    privacy_tier: PrivacyTier::Public,
                    member_count: 1000,
                    is_public: true,
                },
            ])
        }
    }

    #[tokio::test]
    async fn test_network_discovery() {
        let node_id = NodeId {
            name: "test-node".to_string(),
            id: [0u8; 32],
            address: "::1".parse().unwrap(),
            pub_key: vec![],
        };

        let client = Arc::new(MockTrustChainClient);
        let membership = MultiNetworkMembership::new(node_id, client);

        let networks = membership.discover_networks().await.unwrap();
        assert_eq!(networks.len(), 1);
        assert_eq!(networks[0].name, "Bank Customer Portal");
    }

    #[tokio::test]
    async fn test_join_leave_network() {
        let node_id = NodeId {
            name: "test-node".to_string(),
            id: [0u8; 32],
            address: "::1".parse().unwrap(),
            pub_key: vec![],
        };

        let client = Arc::new(MockTrustChainClient);
        let membership = MultiNetworkMembership::new(node_id, client);

        // Discover networks first
        let networks = membership.discover_networks().await.unwrap();
        let network_id = networks[0].network_id;

        // Join network
        membership.join_network(network_id, PrivacyTier::Public).await.unwrap();

        // Leave network
        membership.leave_network(network_id).await.unwrap();
    }

    #[tokio::test]
    async fn test_asset_visibility() {
        let node_id = NodeId {
            name: "test-node".to_string(),
            id: [0u8; 32],
            address: "::1".parse().unwrap(),
            pub_key: vec![],
        };

        let client = Arc::new(MockTrustChainClient);
        let membership = MultiNetworkMembership::new(node_id, client);

        // Discover and join network
        let networks = membership.discover_networks().await.unwrap();
        let network_id = networks[0].network_id;
        membership.join_network(network_id, PrivacyTier::Public).await.unwrap();

        // Add asset to network
        use crate::assets::core::AssetType;
        let asset_id = test_asset_id(AssetType::Cpu);
        membership.add_asset_to_network(network_id, asset_id.clone()).await.unwrap();

        // Check visibility
        assert!(membership.is_asset_visible(network_id, &asset_id).await);

        // Get networks for asset
        let networks = membership.networks_for_asset(&asset_id).await;
        assert_eq!(networks.len(), 1);
        assert_eq!(networks[0], network_id);
    }
}
