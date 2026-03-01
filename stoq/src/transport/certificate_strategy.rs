// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Certificate strategy pattern for network-aware trust models
//!
//! Two certificate modes:
//! - **Anonymous**: Ephemeral self-signed certs per connection (Tor-like, no CA/CT)
//! - **Authenticated**: TrustChain-issued certs (the CA endpoint is configuration,
//!   not a different strategy — P2P, Federated, and Public all use the same
//!   TrustChain mechanism, just pointed at different CA instances)

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use rustls::pki_types::PrivateKeyDer;
use std::net::Ipv6Addr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
// sha2 used by test helpers
use serde::{Deserialize, Serialize};
#[cfg(test)]
use sha2::{Digest, Sha256};

use super::certificates::{StoqNodeCertificate, TrustChainClient};

/// Network-specific certificate strategy
#[async_trait]
pub trait CertificateStrategy: Send + Sync {
    /// Get certificate for this network
    async fn get_certificate(&self) -> Result<Option<StoqNodeCertificate>>;

    /// Validate peer certificate in network context
    async fn validate_certificate(&self, cert: &StoqNodeCertificate) -> Result<bool>;

    /// Strategy name for debugging
    fn strategy_name(&self) -> &str;

    /// Check if strategy requires persistent certificates
    fn requires_certificate(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// Anonymous: ephemeral self-signed certs, no CA, no CT
// ---------------------------------------------------------------------------

/// Anonymous network certificate strategy
///
/// Generates per-connection ephemeral self-signed certificates for QUIC TLS
/// handshakes. No persistent identity, no CA involvement, no CT logging.
/// Each call to `get_certificate()` produces a fresh keypair — Tor-like
/// isolation where no two connections share the same certificate.
pub struct AnonymousCertificateStrategy {
    /// Optional tunnel context for tunnel-aware cert generation.
    tunnel_id: Option<String>,
    /// Hop number within the tunnel (0 = entry).
    hop_number: u32,
}

impl Default for AnonymousCertificateStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl AnonymousCertificateStrategy {
    pub fn new() -> Self {
        info!(
            "Initializing Anonymous certificate strategy: ephemeral per-connection certs, no CA/CT"
        );
        Self {
            tunnel_id: None,
            hop_number: 0,
        }
    }

    /// Create a tunnel-aware anonymous strategy. The tunnel ID and hop number
    /// are embedded in the certificate's common name so tunnel peers can
    /// correlate hops without any CA involvement.
    pub fn for_tunnel(tunnel_id: String, hop_number: u32) -> Self {
        info!(
            "Initializing Anonymous certificate strategy for tunnel {} hop {}",
            tunnel_id, hop_number
        );
        Self {
            tunnel_id: Some(tunnel_id),
            hop_number,
        }
    }

    /// Return the identity scope for anonymous connections: Device scope, untracked.
    pub fn identity_scope(&self) -> hypermesh_lib::IdentityScope {
        hypermesh_lib::IdentityScope::anonymous_device()
    }

    /// Generate a fresh ephemeral self-signed certificate.
    fn generate_ephemeral(&self) -> Result<StoqNodeCertificate> {
        let cn = match &self.tunnel_id {
            Some(tid) => format!("ephemeral-{}-hop{}", tid, self.hop_number),
            None => format!("ephemeral-{}", uuid::Uuid::new_v4()),
        };

        let cert_key = rcgen::generate_simple_self_signed(vec![cn])?;
        let cert_der = cert_key.cert.der().clone();
        let private_key_der = PrivateKeyDer::try_from(cert_key.key_pair.serialize_der())
            .map_err(|e| anyhow!("Failed to serialize ephemeral private key: {e}"))?;

        let fingerprint: [u8; 32] = blake3::hash(cert_der.as_ref()).into();

        let now = SystemTime::now();
        let expires_at = now + Duration::from_secs(86400); // 24 hours

        let metadata = self
            .tunnel_id
            .as_ref()
            .map(|tid| format!("EPHEMERAL:ANONYMOUS:{}:hop{}", tid, self.hop_number).into_bytes());

        Ok(StoqNodeCertificate {
            node_id: format!("ephemeral-{}", hex::encode(&fingerprint[..8])),
            certificate: cert_der,
            private_key: private_key_der,
            issued_at: now,
            expires_at,
            fingerprint_sha256: fingerprint,
            metadata,
        })
    }
}

#[async_trait]
impl CertificateStrategy for AnonymousCertificateStrategy {
    async fn get_certificate(&self) -> Result<Option<StoqNodeCertificate>> {
        debug!("Anonymous network: generating fresh ephemeral certificate");
        Ok(Some(self.generate_ephemeral()?))
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

// ---------------------------------------------------------------------------
// Authenticated: TrustChain-issued certs (Private, Federated, or Public)
// ---------------------------------------------------------------------------

/// Authenticated certificate strategy — single implementation for all
/// non-anonymous modes (Private/P2P, Federated, Public).
///
/// The mechanism is always the same: request a certificate from a TrustChain
/// CA instance. The only difference is *which* CA endpoint:
/// - Private/P2P: local or peer TrustChain
/// - Federated: federation gateway's TrustChain
/// - Public: global TrustChain (trust.hypermesh.online)
///
/// Configuration, not code, determines the trust boundary.
pub struct AuthenticatedCertificateStrategy {
    /// TrustChain CA endpoint to request certificates from
    trustchain_endpoint: String,
    /// Cached TrustChain-issued certificate
    current_cert: Arc<RwLock<Option<StoqNodeCertificate>>>,
    /// TrustChain client for certificate operations
    trustchain_client: Arc<TrustChainClient>,
    /// Human-readable label for logging (e.g. "Private", "Federated", "Public")
    label: String,
    /// Common name for certificate requests
    common_name: String,
    /// IPv6 addresses for certificate SAN extensions
    ipv6_addresses: Vec<Ipv6Addr>,
}

impl AuthenticatedCertificateStrategy {
    /// Create an authenticated strategy pointed at a specific TrustChain CA.
    ///
    /// * `trustchain_endpoint` — QUIC endpoint of the CA (local, gateway, or public)
    /// * `label` — human label for logging ("Private", "Federated", "Public")
    pub fn new(
        trustchain_endpoint: String,
        node_id: String,
        common_name: String,
        ipv6_addresses: Vec<Ipv6Addr>,
        label: String,
    ) -> Self {
        info!(
            "Initializing {} certificate strategy: CA endpoint {}",
            label, trustchain_endpoint
        );

        let trustchain_client =
            Arc::new(TrustChainClient::new(trustchain_endpoint.clone(), node_id));

        Self {
            trustchain_endpoint,
            current_cert: Arc::new(RwLock::new(None)),
            trustchain_client,
            label,
            common_name,
            ipv6_addresses,
        }
    }

    /// Return the identity scope for authenticated connections: Network scope, tracked.
    ///
    /// Authenticated strategies (Private, Federated, Public) all produce
    /// tracked identities on the Network blockchain scope because they
    /// involve TrustChain CA-issued certificates.
    pub fn identity_scope(&self) -> hypermesh_lib::IdentityScope {
        hypermesh_lib::IdentityScope {
            blockchain_scope: hypermesh_lib::BlockchainScope::Network,
            tracked: true,
        }
    }

    /// Request a certificate from the configured TrustChain CA.
    async fn request_certificate(&self) -> Result<StoqNodeCertificate> {
        info!(
            "Requesting {} certificate from TrustChain CA: {}",
            self.label, self.trustchain_endpoint
        );

        let metadata = format!(
            "TRUSTCHAIN:{}:{}",
            self.label.to_uppercase(),
            self.trustchain_endpoint
        )
        .into_bytes();

        let cert = self
            .trustchain_client
            .request_certificate(&self.common_name, &self.ipv6_addresses, Some(&metadata))
            .await?;

        info!(
            "{} certificate obtained: {}",
            self.label,
            cert.fingerprint()
        );
        Ok(cert)
    }
}

#[async_trait]
impl CertificateStrategy for AuthenticatedCertificateStrategy {
    async fn get_certificate(&self) -> Result<Option<StoqNodeCertificate>> {
        let mut cert_guard = self.current_cert.write().await;

        if cert_guard.is_none() || cert_guard.as_ref().is_none_or(|c| c.needs_renewal()) {
            debug!("Requesting new {} certificate", self.label);
            let cert = self.request_certificate().await?;
            *cert_guard = Some(cert.clone());
            Ok(Some(cert))
        } else {
            Ok(cert_guard.clone())
        }
    }

    async fn validate_certificate(&self, cert: &StoqNodeCertificate) -> Result<bool> {
        debug!("Validating certificate via {} TrustChain CA", self.label);

        let is_valid = self
            .trustchain_client
            .validate_certificate(cert.certificate.as_ref())
            .await?;

        if is_valid {
            debug!("{} certificate validated", self.label);
        } else {
            warn!("{} certificate validation failed", self.label);
        }

        Ok(is_valid)
    }

    fn strategy_name(&self) -> &str {
        &self.label
    }
}

// ---------------------------------------------------------------------------
// Backward-compatible type aliases for code that references the old names
// ---------------------------------------------------------------------------

/// Backward-compatible alias — P2P uses `AuthenticatedCertificateStrategy`
/// pointed at the local/peer TrustChain.
pub type P2PCertificateStrategy = AuthenticatedCertificateStrategy;

/// Backward-compatible alias — Federated uses `AuthenticatedCertificateStrategy`
/// pointed at a federation gateway's TrustChain.
pub type FederatedCertificateStrategy = AuthenticatedCertificateStrategy;

/// Backward-compatible alias — Public uses `AuthenticatedCertificateStrategy`
/// pointed at the global TrustChain.
pub type PublicCertificateStrategy = AuthenticatedCertificateStrategy;

// ---------------------------------------------------------------------------
// NetworkType — kept for PrivacyMode mapping in PoS integration
// ---------------------------------------------------------------------------

/// Network type for certificate strategy selection and PrivacyMode mapping.
///
/// The variants map 1:1 to PrivacyMode (see `pos_integration.rs`).
/// All non-Anonymous variants create the same `AuthenticatedCertificateStrategy`
/// pointed at different TrustChain CA endpoints.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetworkType {
    /// Anonymous network — ephemeral certs, no CA/CT
    Anonymous,
    /// Private/P2P network — local TrustChain CA
    P2P,
    /// Federated network — gateway TrustChain CA
    Federated { gateway_url: String },
    /// Public network — global TrustChain CA
    Public,
}

impl NetworkType {
    /// Create appropriate certificate strategy for this network type.
    pub fn create_strategy(
        &self,
        node_id: String,
        common_name: String,
        ipv6_addresses: Vec<Ipv6Addr>,
    ) -> Result<Arc<dyn CertificateStrategy>> {
        match self {
            NetworkType::Anonymous => Ok(Arc::new(AnonymousCertificateStrategy::new())),
            NetworkType::P2P => Ok(Arc::new(AuthenticatedCertificateStrategy::new(
                "local://trustchain".to_string(),
                node_id,
                common_name,
                ipv6_addresses,
                "Private".to_string(),
            ))),
            NetworkType::Federated { gateway_url } => {
                Ok(Arc::new(AuthenticatedCertificateStrategy::new(
                    format!("quic://{gateway_url}"),
                    node_id,
                    common_name,
                    ipv6_addresses,
                    "Federated".to_string(),
                )))
            }
            NetworkType::Public => Ok(Arc::new(AuthenticatedCertificateStrategy::new(
                "quic://trust.hypermesh.online".to_string(),
                node_id,
                common_name,
                ipv6_addresses,
                "Public".to_string(),
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_anonymous_strategy() -> Result<()> {
        let strategy = AnonymousCertificateStrategy::new();

        // Anonymous generates ephemeral certs for QUIC TLS
        let cert = strategy.get_certificate().await?;
        assert!(cert.is_some(), "anonymous should generate ephemeral cert");
        let cert = cert.expect("test: anonymous should produce cert");
        assert!(cert.node_id.starts_with("ephemeral-"));

        // Each call produces a unique cert
        let cert2 = strategy
            .get_certificate()
            .await?
            .expect("test: second cert");
        assert_ne!(cert.fingerprint_sha256, cert2.fingerprint_sha256);

        // Anonymous should accept all certificates
        let dummy_cert = create_dummy_cert();
        assert!(strategy.validate_certificate(&dummy_cert).await?);

        // Anonymous doesn't require persistent certificates
        assert!(!strategy.requires_certificate());

        assert_eq!(strategy.strategy_name(), "Anonymous");
        Ok(())
    }

    #[tokio::test]
    async fn test_anonymous_tunnel_strategy() -> Result<()> {
        let strategy = AnonymousCertificateStrategy::for_tunnel("tun-abc".to_string(), 2);

        let cert = strategy.get_certificate().await?;
        assert!(cert.is_some());
        let cert = cert.expect("test: tunnel cert should exist");

        // Tunnel metadata should be embedded
        assert!(cert.node_id.starts_with("ephemeral-"));
        let meta = cert
            .metadata
            .as_ref()
            .expect("tunnel cert should have metadata");
        let meta_str = String::from_utf8_lossy(meta);
        assert!(meta_str.contains("tun-abc"));
        assert!(meta_str.contains("hop2"));

        Ok(())
    }

    #[tokio::test]
    async fn test_authenticated_strategy_labels() -> Result<()> {
        let node_id = "test-node".to_string();
        let cn = "localhost".to_string();
        let addrs = vec![Ipv6Addr::LOCALHOST];

        // All three non-anonymous types create AuthenticatedCertificateStrategy
        let private =
            NetworkType::P2P.create_strategy(node_id.clone(), cn.clone(), addrs.clone())?;
        assert_eq!(private.strategy_name(), "Private");

        let federated = NetworkType::Federated {
            gateway_url: "gw.test.internal".to_string(),
        }
        .create_strategy(node_id.clone(), cn.clone(), addrs.clone())?;
        assert_eq!(federated.strategy_name(), "Federated");

        let public =
            NetworkType::Public.create_strategy(node_id.clone(), cn.clone(), addrs.clone())?;
        assert_eq!(public.strategy_name(), "Public");

        // Anonymous is distinct
        let anon = NetworkType::Anonymous.create_strategy(node_id, cn, addrs)?;
        assert_eq!(anon.strategy_name(), "Anonymous");

        Ok(())
    }

    #[test]
    fn test_anonymous_identity_scope() {
        let strategy = AnonymousCertificateStrategy::new();
        let scope = strategy.identity_scope();
        assert_eq!(scope.blockchain_scope, hypermesh_lib::BlockchainScope::Device);
        assert!(!scope.tracked);
    }

    #[test]
    fn test_authenticated_identity_scope() {
        let strategy = AuthenticatedCertificateStrategy::new(
            "quic://trust.hypermesh.online".to_string(),
            "test-node".to_string(),
            "localhost".to_string(),
            vec![Ipv6Addr::LOCALHOST],
            "Public".to_string(),
        );
        let scope = strategy.identity_scope();
        assert_eq!(scope.blockchain_scope, hypermesh_lib::BlockchainScope::Network);
        assert!(scope.tracked);
    }

    #[tokio::test]
    async fn test_extract_node_id_from_certificate() -> Result<()> {
        let strategy = AnonymousCertificateStrategy::new();
        let cert = strategy
            .get_certificate()
            .await?
            .expect("test: anonymous cert should exist");

        // extract_node_id parses X.509 SPKI and BLAKE3-hashes it
        let node_id = cert.extract_node_id();
        assert!(node_id.is_some(), "should extract node_id from valid cert");

        // Two calls on the same cert must return the same NodeId
        let node_id2 = cert.extract_node_id();
        assert_eq!(node_id, node_id2);
        Ok(())
    }

    // Helper functions for tests
    fn create_dummy_cert() -> StoqNodeCertificate {
        let cert_key = rcgen::generate_simple_self_signed(vec!["test".to_string()])
            .expect("test: generate self-signed cert");
        let cert_der = cert_key.cert.der().clone();
        let private_key_der = PrivateKeyDer::try_from(cert_key.key_pair.serialize_der())
            .expect("test: serialize private key");

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
}
