//! Certificate management for STOQ transport

use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use rustls::{ServerConfig, ClientConfig};
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use rcgen::{Certificate, DistinguishedName, DnType};

pub struct CertificateManager {
    rotation_interval: Duration,
}

impl CertificateManager {
    pub fn new(rotation_interval: Duration) -> Result<Self> {
        Ok(Self { rotation_interval })
    }
    
    pub fn server_crypto_config(&self) -> Result<rustls::ServerConfig> {
        // Generate self-signed cert for now
        let mut cert_params = rcgen::CertificateParams::new(vec!["localhost".to_string()]);
        cert_params.distinguished_name = DistinguishedName::new();
        cert_params.distinguished_name.push(DnType::CommonName, "STOQ Server");

        let cert = Certificate::from_params(cert_params)?;
        let cert_der = cert.serialize_der()?;
        let key_der = cert.serialize_private_key_der();

        Ok(ServerConfig::builder()
            .with_cipher_suites(&[
                rustls::cipher_suite::TLS13_AES_256_GCM_SHA384,
                rustls::cipher_suite::TLS13_AES_128_GCM_SHA256,
            ])
            .with_safe_default_kx_groups()
            .with_safe_default_protocol_versions()?
            .with_no_client_auth()
            .with_single_cert(
                vec![rustls::Certificate(cert_der)],
                rustls::PrivateKey(key_der),
            )?)
    }
}