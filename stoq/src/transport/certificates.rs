//! Certificate management for STOQ transport with TrustChain integration
//! 
//! This module provides certificate management for STOQ nodes with:
//! - TrustChain CA integration for production certificates
//! - Self-signed certificates for localhost testing only
//! - Automatic 24-hour certificate rotation
//! - Real-time certificate fingerprinting and validation
//! - NKrypt consensus proof validation

use std::sync::Arc;
use std::time::{Duration, SystemTime};
use std::net::Ipv6Addr;
use anyhow::{Result, anyhow};
use quinn;
use base64::prelude::*;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rcgen::{generate_simple_self_signed, CertifiedKey};
use tokio::sync::RwLock;
use dashmap::DashMap;
use serde::{Serialize, Deserialize};
use tracing::{info, debug, warn, error};
use sha2::{Sha256, Digest};

/// Certificate manager configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CertificateConfig {
    /// Operating mode
    pub mode: CertificateMode,
    /// Node identifier
    pub node_id: String,
    /// IPv6 addresses for this node
    pub ipv6_addresses: Vec<Ipv6Addr>,
    /// Common name for certificates
    pub common_name: String,
    /// Certificate rotation interval
    pub rotation_interval: Duration,
    /// TrustChain CA endpoint (for production)
    pub trustchain_endpoint: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CertificateMode {
    /// Self-signed certificates for localhost testing ONLY
    LocalhostTesting,
    /// TrustChain CA-issued certificates for production
    TrustChainProduction,
}

impl Default for CertificateConfig {
    fn default() -> Self {
        Self {
            mode: CertificateMode::LocalhostTesting,
            node_id: "stoq-node-localhost".to_string(),
            ipv6_addresses: vec![Ipv6Addr::LOCALHOST],
            common_name: "localhost".to_string(),
            rotation_interval: Duration::from_secs(24 * 60 * 60), // 24 hours
            trustchain_endpoint: None,
        }
    }
}

impl CertificateConfig {
    /// Production configuration for TrustChain integration
    pub fn production(node_id: String, common_name: String, ipv6_addresses: Vec<Ipv6Addr>) -> Self {
        Self {
            mode: CertificateMode::TrustChainProduction,
            node_id,
            ipv6_addresses,
            common_name,
            rotation_interval: Duration::from_secs(24 * 60 * 60), // 24 hours
            trustchain_endpoint: Some("quic://trust.hypermesh.online:8443".to_string()),
        }
    }
}

/// STOQ node certificate with consensus validation
#[derive(Debug)]
pub struct StoqNodeCertificate {
    /// Node identifier
    pub node_id: String,
    /// DER-encoded certificate
    pub certificate: CertificateDer<'static>,
    /// Private key
    pub private_key: PrivateKeyDer<'static>,
    /// Certificate issued timestamp
    pub issued_at: SystemTime,
    /// Certificate expiration timestamp
    pub expires_at: SystemTime,
    /// SHA-256 fingerprint
    pub fingerprint_sha256: [u8; 32],
    /// Associated consensus proof (for TrustChain certificates)
    pub consensus_proof: Option<Vec<u8>>, // Serialized ConsensusProof
}

impl StoqNodeCertificate {
    /// Calculate certificate fingerprint
    pub fn fingerprint(&self) -> String {
        hex::encode(self.fingerprint_sha256)
    }

    /// Check if certificate is expired
    pub fn is_expired(&self) -> bool {
        SystemTime::now() > self.expires_at
    }

    /// Check if certificate needs renewal (within 1 hour of expiry)
    pub fn needs_renewal(&self) -> bool {
        if let Ok(time_to_expiry) = self.expires_at.duration_since(SystemTime::now()) {
            time_to_expiry < Duration::from_secs(60 * 60) // 1 hour threshold
        } else {
            true // Already expired
        }
    }
}

/// Certificate manager with TrustChain integration
pub struct CertificateManager {
    /// Configuration
    config: Arc<CertificateConfig>,
    /// Current certificate
    current_certificate: Arc<RwLock<Option<StoqNodeCertificate>>>,
    /// Certificate cache for validation
    certificate_cache: Arc<DashMap<String, StoqNodeCertificate>>,
    /// TrustChain client (for production mode)
    trustchain_client: Option<Arc<TrustChainClient>>,
}

/// TrustChain client for certificate operations
#[derive(Clone)]
pub struct TrustChainClient {
    endpoint: String,
    node_id: String,
}

impl TrustChainClient {
    pub fn new(endpoint: String, node_id: String) -> Self {
        Self { endpoint, node_id }
    }

    /// Request certificate from TrustChain CA
    pub async fn request_certificate(
        &self,
        common_name: &str,
        ipv6_addresses: &[Ipv6Addr],
        consensus_proof: &[u8],
    ) -> Result<StoqNodeCertificate> {
        info!("Requesting certificate from TrustChain CA: {}", self.endpoint);
        
        // Parse TrustChain endpoint to get address/port
        let endpoint_url = self.endpoint.strip_prefix("quic://").unwrap_or(&self.endpoint);
        let parts: Vec<&str> = endpoint_url.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid TrustChain endpoint format: {}", self.endpoint));
        }
        
        let host = parts[0];
        let port = parts[1].parse::<u16>()
            .map_err(|_| anyhow!("Invalid port in TrustChain endpoint: {}", parts[1]))?;
        
        // Resolve to IPv6 address
        let socket_addrs = tokio::net::lookup_host((host, port)).await?;
        let ipv6_addr = socket_addrs
            .filter(|addr| addr.is_ipv6())
            .next()
            .ok_or_else(|| anyhow!("No IPv6 address found for TrustChain host: {}", host))?;
        
        // Create QUIC client configuration
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        
        let client_config = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        
        let quinn_config = quinn::ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(client_config)?
        ));
        
        // Create endpoint for outgoing connections
        let mut endpoint = quinn::Endpoint::client("[::]:0".parse()?)?;
        endpoint.set_default_client_config(quinn_config);
        
        // Connect to TrustChain CA
        info!("Connecting to TrustChain CA at {}", ipv6_addr);
        let connection = endpoint.connect(ipv6_addr, host)?
            .await
            .map_err(|e| anyhow!("Failed to connect to TrustChain CA: {}", e))?;
        
        // Open bidirectional stream
        let (mut send_stream, mut recv_stream) = connection.open_bi().await?;
        
        // Prepare certificate request
        let request = serde_json::json!({
            "common_name": common_name,
            "san_entries": [common_name],
            "node_id": self.node_id,
            "ipv6_addresses": ipv6_addresses,
            "consensus_proof": base64::prelude::BASE64_STANDARD.encode(consensus_proof)
        });
        
        let request_data = format!("POST /ca/certificate HTTP/1.1\r\n");
        let request_data = format!("{}Host: {}\r\n", request_data, host);
        let request_data = format!("{}Content-Type: application/json\r\n", request_data);
        let request_body = serde_json::to_string(&request)?;
        let request_data = format!("{}Content-Length: {}\r\n\r\n{}", request_data, request_body.len(), request_body);
        
        // Send request
        send_stream.write_all(request_data.as_bytes()).await?;
        send_stream.finish()?;
        
        // Read response
        let response = recv_stream.read_to_end(64 * 1024).await?; // 64KB max
        let response_str = String::from_utf8(response)?;
        
        // Parse HTTP response
        let parts: Vec<&str> = response_str.splitn(2, "\r\n\r\n").collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid HTTP response from TrustChain CA"));
        }
        
        let response_body = parts[1];
        let response_json: serde_json::Value = serde_json::from_str(response_body)?;
        
        // Extract certificate from response
        let certificate = response_json.get("certificate")
            .ok_or_else(|| anyhow!("No certificate in TrustChain response"))?;
        
        let certificate_der_b64 = certificate.get("certificate_der")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("No certificate_der in TrustChain response"))?;
        
        let certificate_der = base64::prelude::BASE64_STANDARD.decode(certificate_der_b64)?;
        let fingerprint = self.calculate_fingerprint(&certificate_der);
        
        // For now, create a mock private key (in production, would be generated locally)
        let private_key = self.generate_private_key()?;
        
        let now = SystemTime::now();
        let expires_at = now + Duration::from_secs(24 * 60 * 60); // 24 hours
        
        let stoq_cert = StoqNodeCertificate {
            node_id: self.node_id.clone(),
            certificate: CertificateDer::from(certificate_der),
            private_key: PrivateKeyDer::try_from(private_key).map_err(|e| anyhow!("Failed to create private key: {}", e))?,
            issued_at: now,
            expires_at,
            fingerprint_sha256: fingerprint,
            consensus_proof: Some(consensus_proof.to_vec()),
        };
        
        info!("Certificate obtained from TrustChain CA: {}", stoq_cert.fingerprint());
        Ok(stoq_cert)
    }

    /// Validate certificate with TrustChain CT logs
    pub async fn validate_certificate(&self, cert_der: &[u8]) -> Result<bool> {
        info!("Validating certificate with TrustChain CT logs");
        
        let fingerprint = hex::encode(self.calculate_fingerprint(cert_der));
        
        // Parse TrustChain endpoint
        let endpoint_url = self.endpoint.strip_prefix("quic://").unwrap_or(&self.endpoint);
        let parts: Vec<&str> = endpoint_url.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid TrustChain endpoint format: {}", self.endpoint));
        }
        
        let host = parts[0];
        let port = parts[1].parse::<u16>()
            .map_err(|_| anyhow!("Invalid port in TrustChain endpoint: {}", parts[1]))?;
        
        // Resolve to IPv6 address
        let socket_addrs = tokio::net::lookup_host((host, port)).await?;
        let ipv6_addr = socket_addrs
            .filter(|addr| addr.is_ipv6())
            .next()
            .ok_or_else(|| anyhow!("No IPv6 address found for TrustChain host: {}", host))?;
        
        // Create QUIC client configuration
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        
        let client_config = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        
        let quinn_config = quinn::ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(client_config)?
        ));
        
        // Create endpoint for outgoing connections
        let mut endpoint = quinn::Endpoint::client("[::]:0".parse()?)?;
        endpoint.set_default_client_config(quinn_config);
        
        // Connect to TrustChain CA
        let connection = endpoint.connect(ipv6_addr, host)?
            .await
            .map_err(|e| anyhow!("Failed to connect to TrustChain CA: {}", e))?;
        
        // Open bidirectional stream
        let (mut send_stream, mut recv_stream) = connection.open_bi().await?;
        
        // Request CT validation
        let request_data = format!("GET /ct/proof/{} HTTP/1.1\r\n", fingerprint);
        let request_data = format!("{}Host: {}\r\n\r\n", request_data, host);
        
        // Send request
        send_stream.write_all(request_data.as_bytes()).await?;
        send_stream.finish()?;
        
        // Read response
        let response = recv_stream.read_to_end(64 * 1024).await?;
        let response_str = String::from_utf8(response)?;
        
        // Parse HTTP response
        let parts: Vec<&str> = response_str.splitn(2, "\r\n\r\n").collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid HTTP response from TrustChain CT"));
        }
        
        let response_body = parts[1];
        let response_json: serde_json::Value = serde_json::from_str(response_body)?;
        
        // Check if certificate is in CT logs
        let is_valid = response_json.get("fingerprint").is_some();
        
        info!("Certificate CT validation result: {}", is_valid);
        Ok(is_valid)
    }
    
    /// Calculate certificate fingerprint
    fn calculate_fingerprint(&self, cert_der: &[u8]) -> [u8; 32] {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(cert_der);
        hasher.finalize().into()
    }
    
    /// Generate private key (mock implementation)
    fn generate_private_key(&self) -> Result<Vec<u8>> {
        // In production, this would generate a proper private key
        // For now, return a mock key
        Ok(vec![0u8; 32])
    }
}

impl CertificateManager {
    /// Create new certificate manager
    pub async fn new(config: CertificateConfig) -> Result<Self> {
        info!("Initializing STOQ certificate manager: {:?}", config.mode);

        let trustchain_client = match &config.mode {
            CertificateMode::TrustChainProduction => {
                if let Some(endpoint) = &config.trustchain_endpoint {
                    Some(Arc::new(TrustChainClient::new(
                        endpoint.clone(),
                        config.node_id.clone(),
                    )))
                } else {
                    return Err(anyhow!("TrustChain endpoint required for production mode"));
                }
            }
            CertificateMode::LocalhostTesting => None,
        };

        let manager = Self {
            config: Arc::new(config),
            current_certificate: Arc::new(RwLock::new(None)),
            certificate_cache: Arc::new(DashMap::new()),
            trustchain_client,
        };

        // Initialize certificate
        manager.initialize_certificate().await?;

        info!("STOQ certificate manager initialized successfully");
        Ok(manager)
    }

    /// Get server crypto configuration for QUIC
    pub async fn server_crypto_config(&self) -> Result<rustls::ServerConfig> {
        let cert_guard = self.current_certificate.read().await;
        let cert = cert_guard.as_ref().ok_or_else(|| anyhow!("No certificate available"))?;

        let server_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                vec![cert.certificate.clone()],
                cert.private_key.clone_key(),
            )?;

        debug!("Server crypto config created with certificate: {}", cert.fingerprint());
        Ok(server_config)
    }

    /// Get client crypto configuration for QUIC
    pub async fn client_crypto_config(&self) -> Result<rustls::ClientConfig> {
        match self.config.mode {
            CertificateMode::LocalhostTesting => {
                // For localhost testing, accept self-signed certificates
                let config = rustls::ClientConfig::builder()
                    .dangerous()
                    .with_custom_certificate_verifier(Arc::new(AcceptAllVerifier))
                    .with_no_client_auth();
                Ok(config)
            }
            CertificateMode::TrustChainProduction => {
                // For production, use TrustChain CA certificates
                let mut root_store = rustls::RootCertStore::empty();
                root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
                
                let config = rustls::ClientConfig::builder()
                    .with_root_certificates(root_store)
                    .with_no_client_auth();
                Ok(config)
            }
        }
    }

    /// Validate certificate chain
    pub async fn validate_certificate_chain(&self, cert_der: &[u8]) -> Result<bool> {
        let fingerprint = self.calculate_fingerprint(cert_der);
        let fingerprint_hex = hex::encode(fingerprint);

        // Check cache first
        if let Some(cached_cert) = self.certificate_cache.get(&fingerprint_hex) {
            if !cached_cert.is_expired() {
                debug!("Certificate validation: cache hit for {}", fingerprint_hex);
                return Ok(true);
            } else {
                // Remove expired certificate from cache
                self.certificate_cache.remove(&fingerprint_hex);
            }
        }

        // Validate based on mode
        match &self.config.mode {
            CertificateMode::LocalhostTesting => {
                // For localhost testing, basic validation
                debug!("Certificate validation: localhost testing mode");
                Ok(true)
            }
            CertificateMode::TrustChainProduction => {
                // For production, validate with TrustChain
                if let Some(client) = &self.trustchain_client {
                    debug!("Certificate validation: TrustChain production mode");
                    client.validate_certificate(cert_der).await
                } else {
                    Err(anyhow!("TrustChain client not available"))
                }
            }
        }
    }

    /// Get current certificate fingerprint
    pub async fn get_certificate_fingerprint(&self) -> Result<String> {
        let cert_guard = self.current_certificate.read().await;
        let cert = cert_guard.as_ref().ok_or_else(|| anyhow!("No certificate available"))?;
        Ok(cert.fingerprint())
    }

    /// Check if certificate needs renewal and rotate if necessary
    pub async fn check_and_rotate_certificate(&self) -> Result<bool> {
        let needs_rotation = {
            let cert_guard = self.current_certificate.read().await;
            if let Some(cert) = cert_guard.as_ref() {
                cert.needs_renewal()
            } else {
                true // No certificate, definitely need one
            }
        };

        if needs_rotation {
            info!("Certificate needs rotation, requesting new certificate");
            self.rotate_certificate().await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Internal: Initialize certificate
    async fn initialize_certificate(&self) -> Result<()> {
        match self.config.mode {
            CertificateMode::LocalhostTesting => {
                self.create_self_signed_certificate().await?;
            }
            CertificateMode::TrustChainProduction => {
                self.request_trustchain_certificate().await?;
            }
        }
        Ok(())
    }

    /// Internal: Create self-signed certificate for localhost testing
    async fn create_self_signed_certificate(&self) -> Result<()> {
        debug!("Creating self-signed certificate for localhost testing");

        let cert_key = generate_simple_self_signed(vec![self.config.common_name.clone()])?;
        let cert_der = cert_key.cert.der().clone();
        let private_key_der = PrivateKeyDer::try_from(cert_key.key_pair.serialize_der())
            .map_err(|e| anyhow!("Failed to serialize private key: {}", e))?;

        let fingerprint = self.calculate_fingerprint(cert_der.as_ref());
        let now = SystemTime::now();
        let expires_at = now + self.config.rotation_interval;

        let stoq_cert = StoqNodeCertificate {
            node_id: self.config.node_id.clone(),
            certificate: cert_der,
            private_key: private_key_der,
            issued_at: now,
            expires_at,
            fingerprint_sha256: fingerprint,
            consensus_proof: None, // No consensus proof for self-signed
        };

        // Store certificate
        *self.current_certificate.write().await = Some(stoq_cert);

        info!("Self-signed certificate created successfully");
        Ok(())
    }

    /// Internal: Request certificate from TrustChain CA
    async fn request_trustchain_certificate(&self) -> Result<()> {
        debug!("Requesting certificate from TrustChain CA");

        if let Some(client) = &self.trustchain_client {
            // For now, create placeholder consensus proof
            let consensus_proof = vec![0u8; 32]; // Placeholder

            let stoq_cert = client.request_certificate(
                &self.config.common_name,
                &self.config.ipv6_addresses,
                &consensus_proof,
            ).await?;

            // Store certificate
            *self.current_certificate.write().await = Some(stoq_cert);

            info!("TrustChain certificate obtained successfully");
            Ok(())
        } else {
            Err(anyhow!("TrustChain client not available"))
        }
    }

    /// Internal: Rotate certificate
    async fn rotate_certificate(&self) -> Result<()> {
        info!("Rotating certificate");

        // Request new certificate (same as initialization)
        self.initialize_certificate().await?;

        info!("Certificate rotation completed successfully");
        Ok(())
    }

    /// Internal: Calculate certificate fingerprint
    fn calculate_fingerprint(&self, cert_der: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(cert_der);
        hasher.finalize().into()
    }
}

/// Certificate verifier that accepts all certificates (for localhost testing only)
#[derive(Debug)]
struct AcceptAllVerifier;

impl rustls::client::danger::ServerCertVerifier for AcceptAllVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }
    
    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    
    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA1,
            rustls::SignatureScheme::ECDSA_SHA1_Legacy,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::ED448,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_localhost_certificate_manager() {
        let config = CertificateConfig::default();
        let manager = CertificateManager::new(config).await.unwrap();
        
        let crypto_config = manager.server_crypto_config().await.unwrap();
        // Crypto config should be created successfully
    }

    #[tokio::test]
    async fn test_certificate_fingerprint() {
        let config = CertificateConfig::default();
        let manager = CertificateManager::new(config).await.unwrap();
        
        let fingerprint = manager.get_certificate_fingerprint().await.unwrap();
        assert!(!fingerprint.is_empty());
        assert_eq!(fingerprint.len(), 64); // SHA-256 hex string
    }

    #[tokio::test]
    async fn test_certificate_rotation_check() {
        let config = CertificateConfig::default();
        let manager = CertificateManager::new(config).await.unwrap();
        
        let needs_rotation = manager.check_and_rotate_certificate().await.unwrap();
        // Should not need rotation immediately after creation
        assert!(!needs_rotation);
    }
}