// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Certificate types and configuration for STOQ transport

use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use serde::{Deserialize, Serialize};
use std::net::Ipv6Addr;
use std::time::{Duration, SystemTime};

use super::super::certificate_strategy::NetworkType;

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
    /// Network type for strategy-based certificate management
    pub network_type: Option<NetworkType>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CertificateMode {
    /// Self-signed certificates for localhost testing ONLY
    LocalhostTesting,
    /// TrustChain CA-issued certificates for production
    TrustChainProduction,
    /// Network-aware strategy-based certificate management
    NetworkStrategy,
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
            network_type: None,
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
            // Use local CA for development, trust.hypermesh.online for production
            trustchain_endpoint: Some("quic://[::1]:8443".to_string()),
            network_type: None,
        }
    }

    /// Localhost testing configuration with self-signed certificates
    pub fn localhost_testing(
        node_id: String,
        common_name: String,
        ipv6_addresses: Vec<Ipv6Addr>,
    ) -> Self {
        Self {
            mode: CertificateMode::LocalhostTesting,
            node_id,
            ipv6_addresses,
            common_name,
            rotation_interval: Duration::from_secs(24 * 60 * 60), // 24 hours
            trustchain_endpoint: None,
            network_type: None,
        }
    }

    /// Network-aware configuration with strategy pattern
    pub fn with_network_type(
        node_id: String,
        common_name: String,
        ipv6_addresses: Vec<Ipv6Addr>,
        network_type: NetworkType,
    ) -> Self {
        Self {
            mode: CertificateMode::NetworkStrategy,
            node_id,
            ipv6_addresses,
            common_name,
            rotation_interval: Duration::from_secs(24 * 60 * 60), // 24 hours
            trustchain_endpoint: None,
            network_type: Some(network_type),
        }
    }
}

/// STOQ node certificate
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
    /// Optional application-specific metadata (for custom validators)
    pub metadata: Option<Vec<u8>>,
}

impl Clone for StoqNodeCertificate {
    fn clone(&self) -> Self {
        Self {
            node_id: self.node_id.clone(),
            certificate: self.certificate.clone(),
            private_key: self.private_key.clone_key(),
            issued_at: self.issued_at,
            expires_at: self.expires_at,
            fingerprint_sha256: self.fingerprint_sha256,
            metadata: self.metadata.clone(),
        }
    }
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

/// Certificate verifier that accepts all certificates (for localhost testing only)
#[derive(Debug)]
pub(crate) struct AcceptAllVerifier;

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
