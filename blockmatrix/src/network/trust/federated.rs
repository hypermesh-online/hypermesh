// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Federated Network Handler
//!
//! Core Principle: Federation gateway acts as trust anchor for that specific federation.
//! Examples: bank.internal, hospital.federation, government.fed

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::{
    new_random_network_id, request_federation_membership, AssetRequest, AssetResponse, Certificate,
    NetworkConfig, NetworkConnection, NetworkHandler, NetworkType, PeerId, PeerInfo, StoqTransport,
};

/// Federated network handler - federation-specific trust anchor
pub struct FederatedNetworkHandler {
    /// Federation gateway URL (e.g., "bank.internal")
    federation_gateway: Arc<RwLock<String>>,
    /// Certificate from federation CA
    federation_cert: Arc<RwLock<Option<Certificate>>>,
    /// Known federation members
    federation_members: Arc<RwLock<HashSet<PeerId>>>,
    /// Active connection
    connection: Arc<RwLock<Option<NetworkConnection>>>,
    /// Federation metadata
    federation_info: Arc<RwLock<FederationInfo>>,
}

/// Federation information and policies
#[derive(Debug, Default)]
struct FederationInfo {
    /// Federation name
    name: String,
    /// Federation ID
    federation_id: String,
    /// Member count
    member_count: usize,
    /// Join timestamp
    joined_at: Option<u64>,
    /// Federation policies
    _policies: Vec<String>,
    /// Required proof levels
    _required_proofs: Vec<String>,
}

impl FederatedNetworkHandler {
    /// Create new federated network handler
    pub fn new() -> Self {
        info!("Creating federated network handler");
        FederatedNetworkHandler {
            federation_gateway: Arc::new(RwLock::new(String::new())),
            federation_cert: Arc::new(RwLock::new(None)),
            federation_members: Arc::new(RwLock::new(HashSet::new())),
            connection: Arc::new(RwLock::new(None)),
            federation_info: Arc::new(RwLock::new(FederationInfo::default())),
        }
    }

    /// Join a specific federation
    async fn join_federation(
        &self,
        gateway_url: &str,
        stoq: &Arc<StoqTransport>,
    ) -> Result<Certificate> {
        info!("Joining federation at: {}", gateway_url);

        // Request membership from federation gateway
        let federation_cert = request_federation_membership(gateway_url, stoq).await?;

        // Verify certificate is from the expected gateway
        if federation_cert.issuer() != gateway_url {
            return Err(anyhow!(
                "Certificate issuer {} doesn't match gateway {}",
                federation_cert.issuer(),
                gateway_url
            ));
        }

        // Store federation info
        let mut info = self.federation_info.write().await;
        info.name = gateway_url.to_string();
        info.federation_id = federation_cert.fingerprint.clone();
        info.joined_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );

        info!("Successfully joined federation: {}", gateway_url);
        Ok(federation_cert)
    }

    /// Discover other federation members
    async fn discover_members(&self, _stoq: &Arc<StoqTransport>) -> Result<()> {
        debug!("Discovering federation members");

        // In production, would query federation gateway for member list
        // For now, simulate with placeholder
        let mut members = self.federation_members.write().await;

        // Placeholder member discovery
        for i in 0..3 {
            let member_id = PeerId::new(format!("federation-member-{i}"));
            members.insert(member_id);
        }

        let mut info = self.federation_info.write().await;
        info.member_count = members.len();

        info!("Discovered {} federation members", members.len());
        Ok(())
    }

    /// Validate federation membership
    async fn validate_membership(&self, peer_cert: &Certificate) -> bool {
        // Check if certificate is issued by our federation gateway
        let gateway = self.federation_gateway.read().await;
        peer_cert.issuer() == gateway.as_str()
    }
}

#[async_trait]
impl NetworkHandler for FederatedNetworkHandler {
    async fn bootstrap(&self, config: NetworkConfig) -> Result<NetworkConnection> {
        info!("Bootstrapping federated network connection");

        let gateway_url = config
            .federation_gateway
            .ok_or_else(|| anyhow!("Federation gateway URL required for federated mode"))?;

        // Create STOQ transport for this federation
        let stoq = StoqTransport::new_for_network(NetworkType::Federated {
            gateway_url: gateway_url.clone(),
        })?;

        // Store the gateway URL
        *self.federation_gateway.write().await = gateway_url.clone();

        // Request certificate from federation gateway
        let federation_cert = self.join_federation(&gateway_url, &stoq).await?;

        // Store the certificate
        *self.federation_cert.write().await = Some(federation_cert.clone());

        // Discover other federation members
        self.discover_members(&stoq).await?;

        let connection = NetworkConnection {
            network_id: new_random_network_id(),
            network_type: NetworkType::Federated { gateway_url },
            stoq_transport: stoq,
            certificate: Some(federation_cert),
        };

        // Store connection reference
        let connection_ref = connection.clone();
        *self.connection.write().await = Some(connection_ref);

        let member_count = self.federation_members.read().await.len();
        info!(
            "Federated network bootstrapped with {} members",
            member_count
        );
        Ok(connection)
    }

    async fn connect(&self) -> Result<()> {
        info!("Connecting to federated network");

        // Verify we have a valid federation certificate
        let cert_opt = self.federation_cert.read().await;
        if cert_opt.is_none() {
            return Err(anyhow!("No federation certificate - bootstrap first"));
        }

        let cert = cert_opt.as_ref().expect("certificate existence checked above");
        if cert.is_expired() {
            return Err(anyhow!("Federation certificate expired"));
        }

        let gateway = self.federation_gateway.read().await;
        info!("Connected to federation: {}", gateway);
        Ok(())
    }

    async fn validate_peer(&self, peer: &PeerInfo) -> Result<bool> {
        debug!("Validating federated peer: {}", peer.peer_id);

        // Peer must be in federated mode
        let our_gateway = self.federation_gateway.read().await;
        match &peer.network_type {
            NetworkType::Federated { gateway_url } => {
                // Must be same federation
                if gateway_url != our_gateway.as_str() {
                    warn!(
                        "Peer {} is in different federation: {}",
                        peer.peer_id, gateway_url
                    );
                    return Ok(false);
                }
            }
            _ => {
                warn!("Peer {} is not in federated mode", peer.peer_id);
                return Ok(false);
            }
        }

        // Peer must have certificate from same federation gateway
        match &peer.certificate {
            Some(cert) => {
                let valid = self.validate_membership(cert).await;
                if valid {
                    debug!("Peer {} has valid federation certificate", peer.peer_id);
                } else {
                    warn!("Peer {} certificate not from our federation", peer.peer_id);
                }
                Ok(valid)
            }
            None => {
                warn!("Peer {} has no certificate", peer.peer_id);
                Ok(false)
            }
        }
    }

    async fn handle_asset_request(&self, request: AssetRequest) -> Result<AssetResponse> {
        debug!("Handling federated asset request: {}", request.asset_id);

        // Check if requester is a federation member
        let authorized = if let Some(peer_id) = &request.peer_id {
            // In production, would validate against federation member list
            self.federation_members.read().await.contains(peer_id)
        } else {
            false
        };

        let response = AssetResponse {
            asset_id: request.asset_id.clone(),
            data: None, // Would fetch actual data when authorized
            authorized,
            metadata: {
                let mut meta = std::collections::HashMap::new();
                meta.insert("network".to_string(), "federated".to_string());
                meta.insert(
                    "federation".to_string(),
                    self.federation_gateway.read().await.clone(),
                );
                meta.insert(
                    "member_count".to_string(),
                    self.federation_members.read().await.len().to_string(),
                );
                if let Some(peer_id) = &request.peer_id {
                    meta.insert("peer".to_string(), peer_id.to_string());
                }
                meta
            },
        };

        Ok(response)
    }

    async fn disconnect(&self) -> Result<()> {
        info!("Disconnecting from federated network");

        // Log federation statistics
        let info = self.federation_info.read().await;
        if let Some(joined_at) = info.joined_at {
            let duration = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                - joined_at;
            info!(
                "Federation session ended - Federation: {}, Duration: {}s, Members: {}",
                info.name, duration, info.member_count
            );
        }

        // Clear connection but keep federation certificate for rejoining
        *self.connection.write().await = None;

        info!("Federated network disconnected");
        Ok(())
    }

    fn network_type(&self) -> NetworkType {
        // Note: This is synchronous so we can't await. We'll need to store a cached value
        // For now, return a placeholder
        NetworkType::Federated {
            gateway_url: String::from("federation"),
        }
    }
}

impl Default for FederatedNetworkHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_federated_bootstrap() {
        let handler = FederatedNetworkHandler::new();
        let config = NetworkConfig {
            network_type: NetworkType::Federated {
                gateway_url: "bank.internal".to_string(),
            },
            peer_addresses: vec![],
            federation_gateway: Some("bank.internal".to_string()),
            dns_name: None,
            proof_of_state: None,
        };

        let connection = handler.bootstrap(config).await.expect("test: async operation");
        match connection.network_type {
            NetworkType::Federated { gateway_url } => {
                assert_eq!(gateway_url, "bank.internal");
            }
            _ => panic!("Wrong network type"),
        }
        assert!(connection.certificate.is_some());
    }

    #[tokio::test]
    async fn test_federated_peer_validation() {
        let handler = FederatedNetworkHandler::new();
        *handler.federation_gateway.write().await = "bank.internal".to_string();

        // Create a certificate from the same federation
        let valid_cert = Certificate {
            subject: "member".to_string(),
            issuer: "bank.internal".to_string(),
            public_key: vec![0; 32],
            signature: vec![0; 64],
            fingerprint: "test".to_string(),
            expires_at: u64::MAX,
            network_type: NetworkType::Federated {
                gateway_url: "bank.internal".to_string(),
            },
            blockchain_registered: false,
        };

        // Valid federation member
        let valid_peer = PeerInfo {
            peer_id: PeerId::new("valid".to_string()),
            address: "127.0.0.1:8080".to_string(),
            certificate: Some(valid_cert),
            network_type: NetworkType::Federated {
                gateway_url: "bank.internal".to_string(),
            },
        };
        assert!(handler.validate_peer(&valid_peer).await.expect("test: async operation"));

        // Different federation
        let different_cert = Certificate {
            subject: "member".to_string(),
            issuer: "hospital.federation".to_string(),
            public_key: vec![0; 32],
            signature: vec![0; 64],
            fingerprint: "test2".to_string(),
            expires_at: u64::MAX,
            network_type: NetworkType::Federated {
                gateway_url: "hospital.federation".to_string(),
            },
            blockchain_registered: false,
        };

        let different_peer = PeerInfo {
            peer_id: PeerId::new("different".to_string()),
            address: "127.0.0.1:8081".to_string(),
            certificate: Some(different_cert),
            network_type: NetworkType::Federated {
                gateway_url: "hospital.federation".to_string(),
            },
        };
        assert!(!handler.validate_peer(&different_peer).await.expect("test: async operation"));
    }

    #[tokio::test]
    async fn test_federation_requires_gateway() {
        let handler = FederatedNetworkHandler::new();
        let config = NetworkConfig {
            network_type: NetworkType::Federated {
                gateway_url: "bank.internal".to_string(),
            },
            peer_addresses: vec![],
            federation_gateway: None, // Missing gateway
            dns_name: None,
            proof_of_state: None,
        };

        let result = handler.bootstrap(config).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("gateway URL required"));
    }
}
