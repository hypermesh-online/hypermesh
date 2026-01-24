//! Certificate strategy pattern for network-aware trust models
//!
//! This module implements network-specific certificate strategies:
//! - Anonymous: No certificates (ephemeral sessions)
//! - P2P: Direct peer certificate exchange
//! - Federated: Federation gateway managed certificates
//! - Public: Global CA with blockchain registration

use async_trait::async_trait;
use anyhow::{Result, anyhow};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, Duration};
use std::net::Ipv6Addr;
use tokio::sync::RwLock;
use dashmap::DashMap;
use tracing::{info, debug, warn};
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};

use super::certificates::{StoqNodeCertificate, TrustChainClient};

/// Network-specific certificate strategy
#[async_trait]
pub trait CertificateStrategy: Send + Sync {
    /// Get certificate for this network (None for Anonymous)
    async fn get_certificate(&self) -> Result<Option<StoqNodeCertificate>>;

    /// Validate peer certificate in network context
    async fn validate_certificate(&self, cert: &StoqNodeCertificate) -> Result<bool>;

    /// Exchange certificates with peer (P2P only)
    async fn exchange_certificates(&self, _peer_cert: &StoqNodeCertificate) -> Result<Option<StoqNodeCertificate>> {
        Err(anyhow!("Certificate exchange not supported for this network type"))
    }

    /// Strategy name for debugging
    fn strategy_name(&self) -> &str;

    /// Check if strategy requires certificates
    fn requires_certificate(&self) -> bool {
        true
    }
}

/// Anonymous network certificate strategy
///
/// No persistent identity, no trust validation, ephemeral everything
pub struct AnonymousCertificateStrategy;

impl AnonymousCertificateStrategy {
    pub fn new() -> Self {
        info!("Initializing Anonymous certificate strategy: no certificates, ephemeral sessions");
        Self
    }
}

#[async_trait]
impl CertificateStrategy for AnonymousCertificateStrategy {
    async fn get_certificate(&self) -> Result<Option<StoqNodeCertificate>> {
        debug!("Anonymous network: no certificate requested");
        Ok(None)
    }

    async fn validate_certificate(&self, _cert: &StoqNodeCertificate) -> Result<bool> {
        debug!("Anonymous network: accepting all connections without validation");
        Ok(true)
    }

    fn strategy_name(&self) -> &str {
        "Anonymous"
    }

    fn requires_certificate(&self) -> bool {
        false
    }
}

/// Peer ID type for P2P networks
pub type PeerId = String;

/// P2P network certificate strategy
///
/// Direct peer trust exchange without intermediary CA
pub struct P2PCertificateStrategy {
    /// Self-signed certificate for peer exchange
    self_signed_cert: Arc<RwLock<Option<StoqNodeCertificate>>>,
    /// Trusted peer certificates
    trusted_peers: Arc<DashMap<PeerId, StoqNodeCertificate>>,
    /// Node ID for this peer
    node_id: String,
    /// Common name for certificates
    common_name: String,
    /// IPv6 addresses for this node
    ipv6_addresses: Vec<Ipv6Addr>,
}

impl P2PCertificateStrategy {
    pub fn new(node_id: String, common_name: String, ipv6_addresses: Vec<Ipv6Addr>) -> Result<Self> {
        info!("Initializing P2P certificate strategy: direct peer exchange");

        Ok(Self {
            self_signed_cert: Arc::new(RwLock::new(None)),
            trusted_peers: Arc::new(DashMap::new()),
            node_id,
            common_name,
            ipv6_addresses,
        })
    }

    /// Add a trusted peer certificate
    pub async fn add_trusted_peer(&self, peer_id: PeerId, cert: StoqNodeCertificate) {
        info!("Adding trusted peer: {} with fingerprint: {}", peer_id, cert.fingerprint());
        self.trusted_peers.insert(peer_id, cert);
    }

    /// Remove a trusted peer
    pub async fn remove_trusted_peer(&self, peer_id: &str) -> Option<StoqNodeCertificate> {
        self.trusted_peers.remove(peer_id).map(|(_, cert)| cert)
    }

    /// List all trusted peers
    pub async fn list_trusted_peers(&self) -> Vec<(PeerId, String)> {
        self.trusted_peers
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().fingerprint()))
            .collect()
    }

    /// Generate self-signed certificate if not already created
    async fn ensure_self_signed_cert(&self) -> Result<()> {
        let mut cert_guard = self.self_signed_cert.write().await;
        if cert_guard.is_none() {
            debug!("Generating self-signed certificate for P2P");

            let cert_key = rcgen::generate_simple_self_signed(vec![self.common_name.clone()])?;
            let cert_der = cert_key.cert.der().clone();
            let private_key_der = PrivateKeyDer::try_from(cert_key.key_pair.serialize_der())
                .map_err(|e| anyhow!("Failed to serialize private key: {}", e))?;

            let fingerprint = {
                let mut hasher = Sha256::new();
                hasher.update(cert_der.as_ref());
                hasher.finalize().into()
            };

            let now = SystemTime::now();
            let expires_at = now + Duration::from_secs(365 * 24 * 60 * 60); // 1 year

            let stoq_cert = StoqNodeCertificate {
                node_id: self.node_id.clone(),
                certificate: cert_der,
                private_key: private_key_der,
                issued_at: now,
                expires_at,
                fingerprint_sha256: fingerprint,
                metadata: Some(b"P2P_SELF_SIGNED".to_vec()),
            };

            info!("P2P self-signed certificate generated: {}", stoq_cert.fingerprint());
            *cert_guard = Some(stoq_cert);
        }
        Ok(())
    }
}

#[async_trait]
impl CertificateStrategy for P2PCertificateStrategy {
    async fn get_certificate(&self) -> Result<Option<StoqNodeCertificate>> {
        self.ensure_self_signed_cert().await?;
        Ok(self.self_signed_cert.read().await.clone())
    }

    async fn validate_certificate(&self, cert: &StoqNodeCertificate) -> Result<bool> {
        // Check if peer is in trusted list
        let is_trusted = self.trusted_peers
            .iter()
            .any(|entry| entry.value().fingerprint_sha256 == cert.fingerprint_sha256);

        if is_trusted {
            debug!("P2P certificate validated: peer is trusted");
        } else {
            debug!("P2P certificate not trusted: peer not in trusted list");
        }

        Ok(is_trusted)
    }

    async fn exchange_certificates(&self, peer_cert: &StoqNodeCertificate) -> Result<Option<StoqNodeCertificate>> {
        // Ensure we have our certificate
        self.ensure_self_signed_cert().await?;

        // Store peer certificate if not already trusted
        let peer_id = peer_cert.node_id.clone();
        if !self.trusted_peers.contains_key(&peer_id) {
            info!("P2P certificate exchange: storing new peer {} certificate", peer_id);
            self.add_trusted_peer(peer_id, peer_cert.clone()).await;
        }

        // Return our certificate for the peer
        Ok(self.self_signed_cert.read().await.clone())
    }

    fn strategy_name(&self) -> &str {
        "P2P"
    }
}

/// Federated network certificate strategy
///
/// Federation gateway acts as trust anchor for that specific federation
pub struct FederatedCertificateStrategy {
    /// Federation gateway endpoint (e.g., "gateway.bank.internal")
    federation_gateway: String,
    /// Federation-issued certificate
    federation_cert: Arc<RwLock<Option<StoqNodeCertificate>>>,
    /// Federation members trust list
    federation_members: Arc<DashMap<String, StoqNodeCertificate>>,
    /// Node ID for this node
    node_id: String,
    /// Common name for certificates
    common_name: String,
    /// IPv6 addresses for this node
    ipv6_addresses: Vec<Ipv6Addr>,
}

impl FederatedCertificateStrategy {
    pub fn new(
        federation_gateway: String,
        node_id: String,
        common_name: String,
        ipv6_addresses: Vec<Ipv6Addr>,
    ) -> Self {
        info!("Initializing Federated certificate strategy with gateway: {}", federation_gateway);

        Self {
            federation_gateway,
            federation_cert: Arc::new(RwLock::new(None)),
            federation_members: Arc::new(DashMap::new()),
            node_id,
            common_name,
            ipv6_addresses,
        }
    }

    /// Request certificate from federation gateway
    async fn request_federation_certificate(&self) -> Result<StoqNodeCertificate> {
        info!("Requesting certificate from federation gateway: {}", self.federation_gateway);

        // Create client for federation gateway (NOT trust.hypermesh.online)
        let client = TrustChainClient::new(
            format!("quic://{}", self.federation_gateway),
            self.node_id.clone(),
        );

        // Include federation membership proof in metadata
        let metadata = format!("FEDERATION:{}", self.federation_gateway).into_bytes();

        let cert = client.request_certificate(
            &self.common_name,
            &self.ipv6_addresses,
            Some(&metadata),
        ).await?;

        info!("Federation certificate obtained: {}", cert.fingerprint());
        Ok(cert)
    }

    /// Add federation member certificate
    pub async fn add_federation_member(&self, member_id: String, cert: StoqNodeCertificate) {
        info!("Adding federation member: {} with fingerprint: {}", member_id, cert.fingerprint());
        self.federation_members.insert(member_id, cert);
    }
}

#[async_trait]
impl CertificateStrategy for FederatedCertificateStrategy {
    async fn get_certificate(&self) -> Result<Option<StoqNodeCertificate>> {
        let mut cert_guard = self.federation_cert.write().await;

        // Check if we need to request a new certificate
        if cert_guard.is_none() || cert_guard.as_ref().map_or(true, |c| c.needs_renewal()) {
            debug!("Requesting new federation certificate");
            let cert = self.request_federation_certificate().await?;
            *cert_guard = Some(cert.clone());
            Ok(Some(cert))
        } else {
            Ok(cert_guard.clone())
        }
    }

    async fn validate_certificate(&self, cert: &StoqNodeCertificate) -> Result<bool> {
        // Check if certificate has federation metadata
        if let Some(metadata) = &cert.metadata {
            let metadata_str = String::from_utf8_lossy(metadata);
            let expected_prefix = format!("FEDERATION:{}", self.federation_gateway);

            if metadata_str.starts_with(&expected_prefix) {
                debug!("Federation certificate validated: same federation");
                return Ok(true);
            }
        }

        // Check if peer is in federation members list
        let is_member = self.federation_members
            .iter()
            .any(|entry| entry.value().fingerprint_sha256 == cert.fingerprint_sha256);

        if is_member {
            debug!("Federation certificate validated: known member");
        } else {
            debug!("Federation certificate not validated: not a member");
        }

        Ok(is_member)
    }

    fn strategy_name(&self) -> &str {
        "Federated"
    }
}

/// Public network certificate strategy
///
/// Global CA with blockchain-registered certificates
pub struct PublicCertificateStrategy {
    /// Global CA endpoint (trust.hypermesh.online)
    global_ca_endpoint: String,
    /// Blockchain-registered certificate
    blockchain_cert: Arc<RwLock<Option<StoqNodeCertificate>>>,
    /// TrustChain client for CA operations
    trustchain_client: Arc<TrustChainClient>,
    /// Node ID for this node
    node_id: String,
    /// Common name for certificates
    common_name: String,
    /// IPv6 addresses for this node
    ipv6_addresses: Vec<Ipv6Addr>,
}

impl PublicCertificateStrategy {
    pub fn new(
        node_id: String,
        common_name: String,
        ipv6_addresses: Vec<Ipv6Addr>,
    ) -> Self {
        let global_ca_endpoint = "quic://trust.hypermesh.online:8443".to_string();
        info!("Initializing Public certificate strategy with global CA: {}", global_ca_endpoint);

        let trustchain_client = Arc::new(TrustChainClient::new(
            global_ca_endpoint.clone(),
            node_id.clone(),
        ));

        Self {
            global_ca_endpoint,
            blockchain_cert: Arc::new(RwLock::new(None)),
            trustchain_client,
            node_id,
            common_name,
            ipv6_addresses,
        }
    }

    /// Request blockchain-registered certificate from global CA
    async fn request_blockchain_certificate(&self) -> Result<StoqNodeCertificate> {
        info!("Requesting blockchain certificate from global CA: {}", self.global_ca_endpoint);

        // Include Proof of State in metadata (would be generated by BlockMatrix)
        let metadata = b"BLOCKCHAIN:PUBLIC_NETWORK:POS_VALIDATED";

        let cert = self.trustchain_client.request_certificate(
            &self.common_name,
            &self.ipv6_addresses,
            Some(metadata),
        ).await?;

        info!("Blockchain certificate obtained: {}", cert.fingerprint());
        Ok(cert)
    }
}

#[async_trait]
impl CertificateStrategy for PublicCertificateStrategy {
    async fn get_certificate(&self) -> Result<Option<StoqNodeCertificate>> {
        let mut cert_guard = self.blockchain_cert.write().await;

        // Check if we need to request a new certificate
        if cert_guard.is_none() || cert_guard.as_ref().map_or(true, |c| c.needs_renewal()) {
            debug!("Requesting new blockchain certificate");
            let cert = self.request_blockchain_certificate().await?;
            *cert_guard = Some(cert.clone());
            Ok(Some(cert))
        } else {
            Ok(cert_guard.clone())
        }
    }

    async fn validate_certificate(&self, cert: &StoqNodeCertificate) -> Result<bool> {
        // Validate with TrustChain CT logs and blockchain
        debug!("Validating certificate with blockchain");

        let is_valid = self.trustchain_client
            .validate_certificate(cert.certificate.as_ref())
            .await?;

        if is_valid {
            debug!("Public certificate validated via blockchain");
        } else {
            warn!("Public certificate validation failed");
        }

        Ok(is_valid)
    }

    fn strategy_name(&self) -> &str {
        "Public"
    }
}

/// Network type for certificate strategy selection
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetworkType {
    /// Anonymous network - no certificates
    Anonymous,
    /// P2P network - direct peer exchange
    P2P,
    /// Federated network - federation gateway managed
    Federated { gateway_url: String },
    /// Public network - global CA with blockchain
    Public,
}

impl NetworkType {
    /// Create appropriate certificate strategy for network type
    pub fn create_strategy(
        &self,
        node_id: String,
        common_name: String,
        ipv6_addresses: Vec<Ipv6Addr>,
    ) -> Result<Arc<dyn CertificateStrategy>> {
        match self {
            NetworkType::Anonymous => {
                Ok(Arc::new(AnonymousCertificateStrategy::new()))
            }
            NetworkType::P2P => {
                Ok(Arc::new(P2PCertificateStrategy::new(
                    node_id,
                    common_name,
                    ipv6_addresses,
                )?))
            }
            NetworkType::Federated { gateway_url } => {
                Ok(Arc::new(FederatedCertificateStrategy::new(
                    gateway_url.clone(),
                    node_id,
                    common_name,
                    ipv6_addresses,
                )))
            }
            NetworkType::Public => {
                Ok(Arc::new(PublicCertificateStrategy::new(
                    node_id,
                    common_name,
                    ipv6_addresses,
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_anonymous_strategy() -> Result<()> {
        let strategy = AnonymousCertificateStrategy::new();

        // Anonymous should return no certificate
        assert!(strategy.get_certificate().await?.is_none());

        // Anonymous should accept all certificates
        let dummy_cert = create_dummy_cert();
        assert!(strategy.validate_certificate(&dummy_cert).await?);

        // Anonymous doesn't require certificates
        assert!(!strategy.requires_certificate());

        assert_eq!(strategy.strategy_name(), "Anonymous");
        Ok(())
    }

    #[tokio::test]
    async fn test_p2p_strategy() -> Result<()> {
        let strategy = P2PCertificateStrategy::new(
            "test-node".to_string(),
            "localhost".to_string(),
            vec![Ipv6Addr::LOCALHOST],
        )?;

        // P2P should generate self-signed certificate
        let cert = strategy.get_certificate().await?;
        assert!(cert.is_some());

        // P2P should validate trusted peers
        let peer_cert = create_dummy_cert();
        strategy.add_trusted_peer("peer1".to_string(), peer_cert.clone()).await;
        assert!(strategy.validate_certificate(&peer_cert).await?);

        // P2P should reject untrusted peers
        let untrusted_cert = create_different_dummy_cert();
        assert!(!strategy.validate_certificate(&untrusted_cert).await?);

        assert_eq!(strategy.strategy_name(), "P2P");
        Ok(())
    }

    #[tokio::test]
    async fn test_network_type_strategy_creation() -> Result<()> {
        let node_id = "test-node".to_string();
        let common_name = "localhost".to_string();
        let ipv6_addresses = vec![Ipv6Addr::LOCALHOST];

        // Test Anonymous
        let anon_strategy = NetworkType::Anonymous.create_strategy(
            node_id.clone(),
            common_name.clone(),
            ipv6_addresses.clone(),
        )?;
        assert_eq!(anon_strategy.strategy_name(), "Anonymous");

        // Test P2P
        let p2p_strategy = NetworkType::P2P.create_strategy(
            node_id.clone(),
            common_name.clone(),
            ipv6_addresses.clone(),
        )?;
        assert_eq!(p2p_strategy.strategy_name(), "P2P");

        // Test Federated
        let fed_strategy = NetworkType::Federated {
            gateway_url: "gateway.test.internal".to_string(),
        }.create_strategy(
            node_id.clone(),
            common_name.clone(),
            ipv6_addresses.clone(),
        )?;
        assert_eq!(fed_strategy.strategy_name(), "Federated");

        // Test Public
        let pub_strategy = NetworkType::Public.create_strategy(
            node_id.clone(),
            common_name.clone(),
            ipv6_addresses.clone(),
        )?;
        assert_eq!(pub_strategy.strategy_name(), "Public");

        Ok(())
    }

    // Helper functions for tests
    fn create_dummy_cert() -> StoqNodeCertificate {
        let cert_key = rcgen::generate_simple_self_signed(vec!["test".to_string()]).unwrap();
        let cert_der = cert_key.cert.der().clone();
        let private_key_der = PrivateKeyDer::try_from(cert_key.key_pair.serialize_der()).unwrap();

        let fingerprint = {
            let mut hasher = Sha256::new();
            hasher.update(cert_der.as_ref());
            hasher.finalize().into()
        };

        StoqNodeCertificate {
            node_id: "test-node".to_string(),
            certificate: cert_der,
            private_key: private_key_der,
            issued_at: SystemTime::now(),
            expires_at: SystemTime::now() + Duration::from_secs(3600),
            fingerprint_sha256: fingerprint,
            metadata: None,
        }
    }

    fn create_different_dummy_cert() -> StoqNodeCertificate {
        let cert_key = rcgen::generate_simple_self_signed(vec!["different".to_string()]).unwrap();
        let cert_der = cert_key.cert.der().clone();
        let private_key_der = PrivateKeyDer::try_from(cert_key.key_pair.serialize_der()).unwrap();

        let fingerprint = {
            let mut hasher = Sha256::new();
            hasher.update(cert_der.as_ref());
            hasher.finalize().into()
        };

        StoqNodeCertificate {
            node_id: "different-node".to_string(),
            certificate: cert_der,
            private_key: private_key_der,
            issued_at: SystemTime::now(),
            expires_at: SystemTime::now() + Duration::from_secs(3600),
            fingerprint_sha256: fingerprint,
            metadata: None,
        }
    }
}