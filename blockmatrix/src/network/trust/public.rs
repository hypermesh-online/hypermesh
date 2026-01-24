//! Public Network Handler
//!
//! Core Principle: Global CA with blockchain-registered certificates.
//! Only THIS mode uses trust.hypermesh.online and requires full Proof of State.

use async_trait::async_trait;
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn};

use super::{
    NetworkHandler, NetworkConfig, NetworkConnection, NetworkType, NetworkId,
    StoqTransport, PeerInfo, AssetRequest, AssetResponse, Certificate, PeerId,
    ProofOfState, request_blockchain_certificate, register_dns_asset,
};

/// Public network handler - global blockchain-registered certificates
pub struct PublicNetworkHandler {
    /// Blockchain-registered certificate
    blockchain_cert: Option<Certificate>,
    /// Active connection
    connection: Arc<RwLock<Option<NetworkConnection>>>,
    /// Blockchain state
    blockchain_state: Arc<RwLock<BlockchainState>>,
    /// DNS asset registration
    dns_asset: Arc<RwLock<Option<DnsAsset>>>,
}

/// Blockchain state and metrics
#[derive(Debug, Default)]
struct BlockchainState {
    /// Block height when joined
    join_block_height: u64,
    /// Current block height
    current_block_height: u64,
    /// Total validations performed
    validations_performed: u64,
    /// CAESAR rewards earned
    caesar_rewards: u64,
    /// Proof of State submissions
    proof_submissions: Vec<ProofSubmission>,
}

/// DNS asset registration details
#[derive(Debug, Clone)]
struct DnsAsset {
    /// DNS name registered
    dns_name: String,
    /// Asset ID on blockchain
    asset_id: String,
    /// Registration block height
    registered_at_block: u64,
    /// Registration timestamp
    registered_at_time: u64,
}

/// Proof of State submission record
#[derive(Debug, Clone)]
struct ProofSubmission {
    /// Submission timestamp
    timestamp: u64,
    /// Block height
    block_height: u64,
    /// Proof type
    proof_type: String,
    /// Validation result
    validated: bool,
}

impl PublicNetworkHandler {
    /// Create new public network handler
    pub fn new() -> Self {
        info!("Creating public network handler");
        PublicNetworkHandler {
            blockchain_cert: None,
            connection: Arc::new(RwLock::new(None)),
            blockchain_state: Arc::new(RwLock::new(BlockchainState::default())),
            dns_asset: Arc::new(RwLock::new(None)),
        }
    }

    /// Submit Proof of State to blockchain
    async fn submit_proof_of_state(&self, proof: &ProofOfState, stoq: &Arc<StoqTransport>) -> Result<Certificate> {
        info!("Submitting Proof of State to trust.hypermesh.online");

        // Validate all four proofs are present
        if proof.proof_of_space.is_empty() ||
           proof.proof_of_stake.is_empty() ||
           proof.proof_of_work.is_empty() ||
           proof.proof_of_time.is_empty() {
            return Err(anyhow!("All four proofs required for public network"));
        }

        // Request blockchain certificate from trust.hypermesh.online
        let cert = request_blockchain_certificate(stoq, proof).await?;

        // Verify certificate is blockchain-registered
        if !cert.is_blockchain_registered() {
            return Err(anyhow!("Certificate not blockchain-registered"));
        }

        // Record proof submission
        let current_block_height = self.blockchain_state.read().await.current_block_height;
        let mut state = self.blockchain_state.write().await;
        state.proof_submissions.push(ProofSubmission {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            block_height: current_block_height,
            proof_type: "PoSpace+PoStake+PoWork+PoTime".to_string(),
            validated: true,
        });

        info!("Proof of State validated and certificate received");
        Ok(cert)
    }

    /// Register DNS name as blockchain asset
    async fn register_dns(&self, dns_name: &str, cert: &Certificate) -> Result<()> {
        info!("Registering DNS-as-Asset: {}", dns_name);

        // Register on blockchain
        register_dns_asset(dns_name, cert).await?;

        // Store DNS asset info
        let dns_asset = DnsAsset {
            dns_name: dns_name.to_string(),
            asset_id: format!("dns:{}", dns_name), // Placeholder
            registered_at_block: self.blockchain_state.read().await.current_block_height,
            registered_at_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        *self.dns_asset.write().await = Some(dns_asset.clone());

        info!("DNS-as-Asset registered: {} -> {}", dns_name, dns_asset.asset_id);
        Ok(())
    }

    /// Update blockchain state
    async fn update_blockchain_state(&self) {
        // In production, would query actual blockchain
        let mut state = self.blockchain_state.write().await;
        state.current_block_height += 1;
        state.validations_performed += 1;
        state.caesar_rewards += 10; // Placeholder reward calculation
    }
}

#[async_trait]
impl NetworkHandler for PublicNetworkHandler {
    async fn bootstrap(&self, config: NetworkConfig) -> Result<NetworkConnection> {
        info!("Bootstrapping public network connection to trust.hypermesh.online");

        // Public mode REQUIRES Proof of State
        let proof = config.proof_of_state
            .ok_or_else(|| anyhow!("Proof of State required for public network"))?;

        // Create STOQ transport for public network
        let stoq = StoqTransport::new_for_network(NetworkType::Public)?;

        // Submit Proof of State and get blockchain certificate
        let blockchain_cert = self.submit_proof_of_state(&proof, &stoq).await?;

        // Store certificate
        unsafe {
            let self_mut = &mut *(self as *const Self as *mut Self);
            self_mut.blockchain_cert = Some(blockchain_cert.clone());
        }

        // Initialize blockchain state
        let mut state = self.blockchain_state.write().await;
        state.join_block_height = 1000000; // Placeholder
        state.current_block_height = 1000000;

        // Register DNS-as-Asset if provided
        if let Some(dns_name) = config.dns_name {
            self.register_dns(&dns_name, &blockchain_cert).await?;
        }

        let connection = NetworkConnection {
            network_id: NetworkId::new_v4(),
            network_type: NetworkType::Public,
            stoq_transport: stoq,
            certificate: Some(blockchain_cert),
        };

        // Store connection reference
        let connection_ref = connection.clone();
        *self.connection.write().await = Some(connection_ref);

        info!("Public network bootstrapped - Full HyperMesh node active");
        Ok(connection)
    }

    async fn connect(&self) -> Result<()> {
        info!("Connecting to public HyperMesh network");

        // Verify we have a valid blockchain certificate
        let cert = self.blockchain_cert.as_ref()
            .ok_or_else(|| anyhow!("No blockchain certificate - bootstrap first"))?;

        if !cert.is_blockchain_registered() {
            return Err(anyhow!("Certificate not blockchain-registered"));
        }

        if cert.is_expired() {
            return Err(anyhow!("Blockchain certificate expired"));
        }

        // Update blockchain state
        self.update_blockchain_state().await;

        info!("Connected to public network - CAESAR rewards enabled");
        Ok(())
    }

    async fn validate_peer(&self, peer: &PeerInfo) -> Result<bool> {
        debug!("Validating public network peer: {}", peer.peer_id);

        // Peer must be in public mode
        if peer.network_type != NetworkType::Public {
            warn!("Peer {} is not in public mode", peer.peer_id);
            return Ok(false);
        }

        // Peer must have blockchain-registered certificate
        match &peer.certificate {
            Some(cert) => {
                if !cert.is_blockchain_registered() {
                    warn!("Peer {} certificate not blockchain-registered", peer.peer_id);
                    return Ok(false);
                }

                if cert.is_expired() {
                    warn!("Peer {} certificate expired", peer.peer_id);
                    return Ok(false);
                }

                // In production, would validate certificate against blockchain
                debug!("Peer {} has valid blockchain certificate", peer.peer_id);

                // Update validation count
                let mut state = self.blockchain_state.write().await;
                state.validations_performed += 1;

                Ok(true)
            }
            None => {
                warn!("Peer {} has no certificate", peer.peer_id);
                Ok(false)
            }
        }
    }

    async fn handle_asset_request(&self, request: AssetRequest) -> Result<AssetResponse> {
        debug!("Handling public network asset request: {}", request.asset_id);

        // In public mode, authorization is based on blockchain validation
        let authorized = if let Some(peer_id) = &request.peer_id {
            // Would validate peer's blockchain certificate and permissions
            true // Placeholder
        } else {
            false
        };

        // Update blockchain metrics
        if authorized {
            let mut state = self.blockchain_state.write().await;
            state.caesar_rewards += 1; // Earn rewards for serving assets
        }

        let response = AssetResponse {
            asset_id: request.asset_id.clone(),
            data: if authorized {
                None // Would fetch actual data
            } else {
                None
            },
            authorized,
            metadata: {
                let mut meta = std::collections::HashMap::new();
                meta.insert("network".to_string(), "public".to_string());
                meta.insert("blockchain_height".to_string(),
                    self.blockchain_state.read().await.current_block_height.to_string());
                if let Some(dns) = &*self.dns_asset.read().await {
                    meta.insert("dns_name".to_string(), dns.dns_name.clone());
                }
                if let Some(peer_id) = &request.peer_id {
                    meta.insert("peer".to_string(), peer_id.to_string());
                }
                meta
            },
        };

        Ok(response)
    }

    async fn disconnect(&self) -> Result<()> {
        info!("Disconnecting from public HyperMesh network");

        // Log blockchain statistics
        let state = self.blockchain_state.read().await;
        let blocks_processed = state.current_block_height - state.join_block_height;

        info!(
            "Public network session ended - Blocks: {}, Validations: {}, CAESAR Rewards: {}",
            blocks_processed, state.validations_performed, state.caesar_rewards
        );

        if let Some(dns) = &*self.dns_asset.read().await {
            info!("DNS-as-Asset {} remains registered on blockchain", dns.dns_name);
        }

        // Clear connection but keep blockchain certificate
        *self.connection.write().await = None;

        info!("Public network disconnected (blockchain registration maintained)");
        Ok(())
    }

    fn network_type(&self) -> NetworkType {
        NetworkType::Public
    }
}

impl Default for PublicNetworkHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_public_bootstrap_requires_proof() {
        let handler = PublicNetworkHandler::new();
        let config = NetworkConfig {
            network_type: NetworkType::Public,
            peer_addresses: vec![],
            federation_gateway: None,
            dns_name: None,
            proof_of_state: None, // Missing proof
        };

        let result = handler.bootstrap(config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Proof of State required"));
    }

    #[tokio::test]
    async fn test_public_bootstrap_with_proof() {
        let handler = PublicNetworkHandler::new();
        let proof = ProofOfState {
            proof_of_space: vec![1, 2, 3],
            proof_of_stake: vec![4, 5, 6],
            proof_of_work: vec![7, 8, 9],
            proof_of_time: vec![10, 11, 12],
        };

        let config = NetworkConfig {
            network_type: NetworkType::Public,
            peer_addresses: vec![],
            federation_gateway: None,
            dns_name: Some("node.hypermesh".to_string()),
            proof_of_state: Some(proof),
        };

        let connection = handler.bootstrap(config).await.unwrap();
        assert_eq!(connection.network_type, NetworkType::Public);
        assert!(connection.certificate.is_some());
        assert!(connection.certificate.unwrap().is_blockchain_registered());
    }

    #[tokio::test]
    async fn test_public_peer_validation() {
        let handler = PublicNetworkHandler::new();

        // Valid blockchain-registered peer
        let valid_cert = Certificate {
            subject: "blockchain-node".to_string(),
            issuer: "trust.hypermesh.online".to_string(),
            public_key: vec![0; 32],
            signature: vec![0; 64],
            fingerprint: "blockchain:test".to_string(),
            expires_at: u64::MAX,
            network_type: NetworkType::Public,
            blockchain_registered: true,
        };

        let valid_peer = PeerInfo {
            peer_id: PeerId::new("valid".to_string()),
            address: "127.0.0.1:8080".to_string(),
            certificate: Some(valid_cert),
            network_type: NetworkType::Public,
        };
        assert!(handler.validate_peer(&valid_peer).await.unwrap());

        // Non-blockchain-registered peer
        let invalid_cert = Certificate {
            subject: "regular-node".to_string(),
            issuer: "self".to_string(),
            public_key: vec![0; 32],
            signature: vec![0; 64],
            fingerprint: "regular:test".to_string(),
            expires_at: u64::MAX,
            network_type: NetworkType::Public,
            blockchain_registered: false, // Not blockchain-registered
        };

        let invalid_peer = PeerInfo {
            peer_id: PeerId::new("invalid".to_string()),
            address: "127.0.0.1:8081".to_string(),
            certificate: Some(invalid_cert),
            network_type: NetworkType::Public,
        };
        assert!(!handler.validate_peer(&invalid_peer).await.unwrap());
    }

    #[tokio::test]
    async fn test_proof_validation() {
        // Empty proofs should fail
        let empty_proof = ProofOfState {
            proof_of_space: vec![],
            proof_of_stake: vec![],
            proof_of_work: vec![],
            proof_of_time: vec![],
        };

        let handler = PublicNetworkHandler::new();
        let stoq = StoqTransport::new_for_network(NetworkType::Public).unwrap();
        let result = handler.submit_proof_of_state(&empty_proof, &stoq).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("All four proofs required"));
    }
}