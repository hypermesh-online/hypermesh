// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! P2P Network Handler
//!
//! Core Principle: Direct peer trust exchange without intermediary CA.
//! Similar to SSH known_hosts model - users manually approve peer certificates.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::{
    new_random_network_id, AssetRequest, AssetResponse, Certificate, NetworkConfig,
    NetworkConnection, NetworkHandler, NetworkType, PeerId, PeerInfo, StoqTransport,
};

/// P2P network handler - direct peer-to-peer with self-signed certificates
pub struct P2PNetworkHandler {
    /// Self-signed certificate for this node
    self_signed_cert: Certificate,
    /// Trusted peer certificates (manually approved)
    trusted_peers: Arc<RwLock<HashMap<PeerId, Certificate>>>,
    /// Pending peer certificates (awaiting approval)
    pending_peers: Arc<RwLock<HashMap<PeerId, Certificate>>>,
    /// Active connection
    connection: Arc<RwLock<Option<NetworkConnection>>>,
    /// Trust decisions log
    trust_decisions: Arc<RwLock<Vec<TrustDecision>>>,
}

/// Trust decision for a peer
#[derive(Debug, Clone)]
struct TrustDecision {
    /// Peer being decided on
    _peer_id: PeerId,
    /// Decision (accept/reject)
    accepted: bool,
    /// Timestamp of decision
    _timestamp: u64,
    /// Reason for decision
    _reason: String,
}

impl P2PNetworkHandler {
    /// Create new P2P network handler
    pub fn new() -> Self {
        info!("Creating P2P network handler");

        // Generate self-signed certificate
        let self_signed_cert = Self::generate_self_signed_cert();

        P2PNetworkHandler {
            self_signed_cert,
            trusted_peers: Arc::new(RwLock::new(HashMap::new())),
            pending_peers: Arc::new(RwLock::new(HashMap::new())),
            connection: Arc::new(RwLock::new(None)),
            trust_decisions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Generate self-signed certificate for P2P.
    ///
    /// Uses BLAKE3 for deterministic key derivation and fingerprinting:
    /// - Private key: random 32 bytes
    /// - Public key: BLAKE3(private_key)
    /// - Signature: BLAKE3(subject || public_key)
    /// - Fingerprint: hex(BLAKE3(public_key || subject))
    pub fn generate_self_signed_cert() -> Certificate {
        use rand::Rng;
        use std::time::{SystemTime, UNIX_EPOCH};

        let node_id = format!("p2p-node-{}", uuid::Uuid::new_v4());
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Key derivation via BLAKE3
        let mut rng = rand::thread_rng();
        let mut private_key = [0u8; 32];
        rng.fill(&mut private_key);
        let public_key = blake3::hash(&private_key).as_bytes().to_vec();

        // Self-signature: BLAKE3(subject || public_key)
        let mut sig_hasher = blake3::Hasher::new();
        sig_hasher.update(node_id.as_bytes());
        sig_hasher.update(&public_key);
        let signature = sig_hasher.finalize().as_bytes().to_vec();

        // Fingerprint: BLAKE3(public_key || subject)
        let mut fp_hasher = blake3::Hasher::new();
        fp_hasher.update(&public_key);
        fp_hasher.update(node_id.as_bytes());
        let fingerprint = hex::encode(fp_hasher.finalize().as_bytes());

        Certificate {
            subject: node_id.clone(),
            issuer: node_id, // Self-signed
            public_key,
            signature,
            fingerprint,
            expires_at: now + 365 * 24 * 3600, // 1 year
            network_type: NetworkType::P2P,
            blockchain_registered: false,
        }
    }

    /// Connect to a specific peer and exchange certificates
    async fn connect_to_peer(&self, peer_addr: &str, stoq: &Arc<StoqTransport>) -> Result<()> {
        info!("Connecting to P2P peer: {}", peer_addr);

        // Exchange certificates with peer
        let peer_cert = stoq
            .exchange_certificate(peer_addr, &self.self_signed_cert)
            .await?;
        let peer_id = PeerId::from_cert(&peer_cert);

        debug!(
            "Received certificate from peer {}: {:?}",
            peer_id, peer_cert.fingerprint
        );

        // Add to pending peers for manual approval
        self.pending_peers
            .write()
            .await
            .insert(peer_id.clone(), peer_cert.clone());

        // For now, auto-accept (TODO: implement UI for manual approval)
        self.approve_peer(peer_id.clone(), "Auto-approved for testing")
            .await?;

        Ok(())
    }

    /// Manually approve a peer
    pub async fn approve_peer(&self, peer_id: PeerId, reason: &str) -> Result<()> {
        info!("Approving P2P peer: {}", peer_id);

        // Move from pending to trusted
        let cert = self
            .pending_peers
            .write()
            .await
            .remove(&peer_id)
            .ok_or_else(|| anyhow!("Peer {peer_id} not found in pending list"))?;

        self.trusted_peers
            .write()
            .await
            .insert(peer_id.clone(), cert);

        // Log trust decision
        self.trust_decisions.write().await.push(TrustDecision {
            _peer_id: peer_id.clone(),
            accepted: true,
            _timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            _reason: reason.to_string(),
        });

        info!("Peer {} approved and added to trusted list", peer_id);
        Ok(())
    }

    /// Manually reject a peer
    pub async fn reject_peer(&self, peer_id: PeerId, reason: &str) -> Result<()> {
        warn!("Rejecting P2P peer: {}", peer_id);

        // Remove from pending
        self.pending_peers.write().await.remove(&peer_id);

        // Log trust decision
        self.trust_decisions.write().await.push(TrustDecision {
            _peer_id: peer_id.clone(),
            accepted: false,
            _timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            _reason: reason.to_string(),
        });

        warn!("Peer {} rejected", peer_id);
        Ok(())
    }

    /// Get list of pending peers awaiting approval
    pub async fn get_pending_peers(&self) -> Vec<(PeerId, Certificate)> {
        self.pending_peers
            .read()
            .await
            .iter()
            .map(|(id, cert)| (id.clone(), cert.clone()))
            .collect()
    }

    /// Verify a self-signed certificate using BLAKE3.
    ///
    /// Checks that `signature == BLAKE3(subject || public_key)` and
    /// `fingerprint == hex(BLAKE3(public_key || subject))`.
    pub fn verify_self_signed_cert(cert: &Certificate) -> bool {
        if !cert.is_self_signed() {
            return false;
        }

        // Verify signature
        let mut sig_hasher = blake3::Hasher::new();
        sig_hasher.update(cert.subject.as_bytes());
        sig_hasher.update(&cert.public_key);
        let expected_sig = sig_hasher.finalize().as_bytes().to_vec();
        if cert.signature != expected_sig {
            return false;
        }

        // Verify fingerprint
        let mut fp_hasher = blake3::Hasher::new();
        fp_hasher.update(&cert.public_key);
        fp_hasher.update(cert.subject.as_bytes());
        let expected_fp = hex::encode(fp_hasher.finalize().as_bytes());
        cert.fingerprint == expected_fp
    }

    /// Get list of trusted peers
    pub async fn get_trusted_peers(&self) -> Vec<(PeerId, Certificate)> {
        self.trusted_peers
            .read()
            .await
            .iter()
            .map(|(id, cert)| (id.clone(), cert.clone()))
            .collect()
    }
}

#[async_trait]
impl NetworkHandler for P2PNetworkHandler {
    async fn bootstrap(&self, config: NetworkConfig) -> Result<NetworkConnection> {
        info!("Bootstrapping P2P network connection");

        // Create STOQ transport for P2P
        let stoq = StoqTransport::new_for_network(NetworkType::P2P)?;

        // Connect to specified peers and exchange certificates
        for peer_addr in &config.peer_addresses {
            match self.connect_to_peer(peer_addr, &stoq).await {
                Ok(_) => info!("Successfully connected to peer: {}", peer_addr),
                Err(e) => warn!("Failed to connect to peer {}: {}", peer_addr, e),
            }
        }

        let connection = NetworkConnection {
            network_id: new_random_network_id(),
            network_type: NetworkType::P2P,
            stoq_transport: stoq,
            certificate: Some(self.self_signed_cert.clone()),
        };

        // Store connection reference
        let connection_ref = connection.clone();
        *self.connection.write().await = Some(connection_ref);

        info!(
            "P2P network bootstrapped with {} trusted peers",
            self.trusted_peers.read().await.len()
        );
        Ok(connection)
    }

    async fn connect(&self) -> Result<()> {
        info!("Connecting to P2P network");

        // Check if we have any trusted peers
        let trusted_count = self.trusted_peers.read().await.len();
        if trusted_count == 0 {
            warn!("No trusted peers - P2P network will be isolated");
        } else {
            info!("P2P network ready with {} trusted peers", trusted_count);
        }

        Ok(())
    }

    async fn validate_peer(&self, peer: &PeerInfo) -> Result<bool> {
        debug!("Validating P2P peer: {}", peer.peer_id);

        // Peer must be in P2P mode
        if peer.network_type != NetworkType::P2P {
            warn!("Peer {} is not in P2P mode", peer.peer_id);
            return Ok(false);
        }

        // Check if peer is in trusted list
        let trusted = self.trusted_peers.read().await;
        if trusted.contains_key(&peer.peer_id) {
            debug!("Peer {} is trusted", peer.peer_id);
            return Ok(true);
        }

        // Check if peer is pending approval
        let pending = self.pending_peers.read().await;
        if pending.contains_key(&peer.peer_id) {
            debug!("Peer {} is pending approval", peer.peer_id);
            return Ok(false); // Not yet trusted
        }

        // Unknown peer - add to pending if we have a valid self-signed certificate
        if let Some(cert) = &peer.certificate {
            if cert.is_self_signed() && Self::verify_self_signed_cert(cert) {
                drop(pending); // Release read lock
                self.pending_peers
                    .write()
                    .await
                    .insert(peer.peer_id.clone(), cert.clone());
                info!("Added peer {} to pending approval list", peer.peer_id);
            } else if !Self::verify_self_signed_cert(cert) {
                warn!(
                    "Peer {} certificate failed BLAKE3 verification",
                    peer.peer_id
                );
            }
        }

        Ok(false)
    }

    async fn handle_asset_request(&self, request: AssetRequest) -> Result<AssetResponse> {
        debug!("Handling P2P asset request: {}", request.asset_id);

        // Check if requester is trusted
        let authorized = if let Some(peer_id) = &request.peer_id {
            self.trusted_peers.read().await.contains_key(peer_id)
        } else {
            false
        };

        let response = AssetResponse {
            asset_id: request.asset_id.clone(),
            data: None, // Would fetch actual data when authorized
            authorized,
            metadata: {
                let mut meta = std::collections::HashMap::new();
                meta.insert("network".to_string(), "p2p".to_string());
                if let Some(peer_id) = &request.peer_id {
                    meta.insert("peer".to_string(), peer_id.to_string());
                }
                meta.insert(
                    "trusted_peers".to_string(),
                    self.trusted_peers.read().await.len().to_string(),
                );
                meta
            },
        };

        Ok(response)
    }

    async fn disconnect(&self) -> Result<()> {
        info!("Disconnecting from P2P network");

        // Log trust statistics
        let trusted_count = self.trusted_peers.read().await.len();
        let pending_count = self.pending_peers.read().await.len();
        let decisions = self.trust_decisions.read().await;
        let accepted = decisions.iter().filter(|d| d.accepted).count();
        let rejected = decisions.len() - accepted;

        info!(
            "P2P session ended - Trusted: {}, Pending: {}, Accepted: {}, Rejected: {}",
            trusted_count, pending_count, accepted, rejected
        );

        // Clear connection but keep trusted peers for next session
        *self.connection.write().await = None;

        info!("P2P network disconnected (trusted peers preserved)");
        Ok(())
    }

    fn network_type(&self) -> NetworkType {
        NetworkType::P2P
    }
}

impl Default for P2PNetworkHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_p2p_bootstrap() {
        let handler = P2PNetworkHandler::new();
        let config = NetworkConfig {
            network_type: NetworkType::P2P,
            peer_addresses: vec!["127.0.0.1:8080".to_string()],
            federation_gateway: None,
            dns_name: None,
            proof_of_state: None,
        };

        let connection = handler.bootstrap(config).await.expect("test: async operation");
        assert_eq!(connection.network_type, NetworkType::P2P);
        assert!(connection.certificate.is_some());
        assert!(connection.certificate.expect("test: certificate operation").is_self_signed());
    }

    #[tokio::test]
    async fn test_p2p_peer_approval() {
        let handler = P2PNetworkHandler::new();

        // Create a test peer certificate
        let peer_cert = P2PNetworkHandler::generate_self_signed_cert();
        let peer_id = PeerId::from_cert(&peer_cert);

        // Add to pending
        handler
            .pending_peers
            .write()
            .await
            .insert(peer_id.clone(), peer_cert);
        assert_eq!(handler.get_pending_peers().await.len(), 1);
        assert_eq!(handler.get_trusted_peers().await.len(), 0);

        // Approve peer
        handler
            .approve_peer(peer_id.clone(), "Test approval")
            .await
            .expect("test: expected success");
        assert_eq!(handler.get_pending_peers().await.len(), 0);
        assert_eq!(handler.get_trusted_peers().await.len(), 1);
    }

    #[test]
    fn test_blake3_cert_generation_and_verification() {
        let cert = P2PNetworkHandler::generate_self_signed_cert();

        // Certificate must be self-signed
        assert!(cert.is_self_signed());

        // Public key must be 32 bytes (BLAKE3 output)
        assert_eq!(cert.public_key.len(), 32);

        // Signature must be 32 bytes (BLAKE3 output)
        assert_eq!(cert.signature.len(), 32);

        // Fingerprint must be 64 hex chars (BLAKE3 output)
        assert_eq!(cert.fingerprint.len(), 64);

        // Verify must pass
        assert!(
            P2PNetworkHandler::verify_self_signed_cert(&cert),
            "generated cert must verify"
        );
    }

    #[test]
    fn test_tampered_cert_fails_verification() {
        let mut cert = P2PNetworkHandler::generate_self_signed_cert();

        // Tamper with signature
        cert.signature[0] ^= 0xFF;
        assert!(
            !P2PNetworkHandler::verify_self_signed_cert(&cert),
            "tampered cert must fail"
        );
    }

    #[test]
    fn test_cert_fingerprint_deterministic() {
        let cert = P2PNetworkHandler::generate_self_signed_cert();
        // Recompute fingerprint from public_key + subject
        let mut hasher = blake3::Hasher::new();
        hasher.update(&cert.public_key);
        hasher.update(cert.subject.as_bytes());
        let expected = hex::encode(hasher.finalize().as_bytes());
        assert_eq!(cert.fingerprint, expected);
    }

    #[tokio::test]
    async fn test_p2p_peer_validation() {
        let handler = P2PNetworkHandler::new();

        // Create and approve a test peer
        let peer_cert = P2PNetworkHandler::generate_self_signed_cert();
        let peer_id = PeerId::from_cert(&peer_cert);
        handler
            .trusted_peers
            .write()
            .await
            .insert(peer_id.clone(), peer_cert.clone());

        // Trusted peer should validate
        let peer = PeerInfo {
            peer_id: peer_id.clone(),
            address: "127.0.0.1:8080".to_string(),
            certificate: Some(peer_cert),
            network_type: NetworkType::P2P,
        };
        assert!(handler.validate_peer(&peer).await.expect("test: async operation"));

        // Unknown peer should not validate
        let unknown_peer = PeerInfo {
            peer_id: PeerId::new("unknown".to_string()),
            address: "127.0.0.1:8081".to_string(),
            certificate: None,
            network_type: NetworkType::P2P,
        };
        assert!(!handler.validate_peer(&unknown_peer).await.expect("test: async operation"));
    }
}
