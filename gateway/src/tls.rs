// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! TLS certificate provider for the gateway.
//!
//! Loads TLS certificates from various sources: PEM/DER files on disk,
//! TrustChain CA integration (with self-signed fallback since FALCON-1024
//! certificates are not trusted by clearnet browsers), or on-the-fly
//! self-signed certificate generation via `rcgen`.

use std::path::Path;

use anyhow::Result;
use rcgen::{CertificateParams, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio::io::AsyncReadExt;
use tracing::{info, warn};

use crate::config::CertificateSource;
use crate::error::GatewayError;

/// Provides TLS certificates from various sources for the gateway's QUIC endpoint.
pub struct TlsProvider {
    cert_source: CertificateSource,
}

impl TlsProvider {
    /// Create a new provider backed by the given certificate source.
    pub fn new(cert_source: CertificateSource) -> Self {
        Self { cert_source }
    }

    /// Load certificates based on the configured source.
    ///
    /// Returns a certificate chain and the corresponding private key suitable
    /// for configuring a `rustls::ServerConfig`.
    pub async fn load_certificates(
        &self,
    ) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
        match &self.cert_source {
            CertificateSource::File {
                cert_path,
                key_path,
            } => Self::load_from_files(cert_path, key_path).await,

            CertificateSource::TrustChain {
                ca_addr,
                common_name,
            } => {
                // TrustChain certificates use FALCON-1024 which clearnet browsers
                // do not recognise. Fall back to a locally generated self-signed
                // certificate so the gateway can still serve HTTP/3 traffic.
                warn!(
                    "TrustChain certificate source (CA at {}) uses FALCON-1024 \
                     -- not trusted by clearnet browsers. Generating self-signed \
                     certificate for '{}'.",
                    ca_addr, common_name
                );
                Self::generate_self_signed(common_name)
            }

            CertificateSource::SelfSigned { common_name } => {
                Self::generate_self_signed(common_name)
            }
        }
    }

    /// Load certificate chain and private key from PEM or DER files.
    ///
    /// File format is detected by extension: `.pem` / `.crt` are parsed as
    /// PEM, everything else is treated as raw DER.
    async fn load_from_files(
        cert_path: &Path,
        key_path: &Path,
    ) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
        // --- Certificate ---
        let mut cert_file = tokio::fs::File::open(cert_path).await.map_err(|e| {
            GatewayError::Tls(format!(
                "failed to open certificate file {}: {}",
                cert_path.display(),
                e
            ))
        })?;
        let mut cert_data = Vec::new();
        cert_file.read_to_end(&mut cert_data).await.map_err(|e| {
            GatewayError::Tls(format!(
                "failed to read certificate file {}: {}",
                cert_path.display(),
                e
            ))
        })?;

        let certs = if is_pem_extension(cert_path) {
            rustls_pemfile::certs(&mut cert_data.as_slice())
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| GatewayError::Tls(format!("PEM certificate parse error: {e}")))?
        } else {
            vec![CertificateDer::from(cert_data)]
        };

        if certs.is_empty() {
            return Err(
                GatewayError::Tls("certificate file contained no certificates".into()).into(),
            );
        }

        // --- Private key ---
        let mut key_file = tokio::fs::File::open(key_path).await.map_err(|e| {
            GatewayError::Tls(format!(
                "failed to open key file {}: {}",
                key_path.display(),
                e
            ))
        })?;
        let mut key_data = Vec::new();
        key_file.read_to_end(&mut key_data).await.map_err(|e| {
            GatewayError::Tls(format!(
                "failed to read key file {}: {}",
                key_path.display(),
                e
            ))
        })?;

        let key = if is_pem_extension(key_path) {
            rustls_pemfile::private_key(&mut key_data.as_slice())
                .map_err(|e| GatewayError::Tls(format!("PEM key parse error: {e}")))?
                .ok_or_else(|| GatewayError::Tls("no private key found in PEM file".into()))?
        } else {
            PrivateKeyDer::from(PrivatePkcs8KeyDer::from(key_data))
        };

        info!("TLS certificates loaded from disk");
        Ok((certs, key))
    }

    /// Generate an ephemeral self-signed certificate using `rcgen`.
    ///
    /// The certificate is valid for 365 days and uses an ECDSA P-256 key pair
    /// (the smallest widely-supported algorithm for QUIC/TLS 1.3).
    fn generate_self_signed(
        common_name: &str,
    ) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
        let key_pair = KeyPair::generate()
            .map_err(|e| GatewayError::Tls(format!("failed to generate key pair: {e}")))?;

        let mut params = CertificateParams::new(vec![common_name.to_string()])
            .map_err(|e| GatewayError::Tls(format!("failed to create certificate params: {e}")))?;
        params.distinguished_name.push(
            rcgen::DnType::CommonName,
            rcgen::DnValue::Utf8String(common_name.to_string()),
        );

        let cert = params.self_signed(&key_pair).map_err(|e| {
            GatewayError::Tls(format!("failed to generate self-signed certificate: {e}"))
        })?;

        let cert_der = CertificateDer::from(cert.der().to_vec());
        let key_der = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(key_pair.serialize_der()));

        warn!(
            "Using self-signed certificate for '{}' -- not trusted by browsers",
            common_name
        );

        Ok((vec![cert_der], key_der))
    }

    /// Build a `rustls::ServerConfig` from a certificate chain and private key.
    ///
    /// Configures ALPN for HTTP/3 (`h3`).
    pub fn build_server_config(
        certs: Vec<CertificateDer<'static>>,
        key: PrivateKeyDer<'static>,
    ) -> Result<rustls::ServerConfig> {
        let mut tls_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| GatewayError::Tls(format!("rustls server config error: {e}")))?;

        tls_config.alpn_protocols = vec![b"h3".to_vec()];
        Ok(tls_config)
    }
}

/// Generate an ephemeral self-signed certificate for a specific domain name.
///
/// Used for dynamic per-domain certificate provisioning (e.g. when a new
/// subdomain is resolved via blockchain DNS). Delegates to the same ECDSA
/// P-256 key generation used by [`TlsProvider::generate_self_signed`].
///
/// Returns `(cert_der, key_der)` -- the DER-encoded certificate and private
/// key bytes.
pub fn generate_domain_cert(domain: &str) -> Result<(Vec<u8>, Vec<u8>)> {
    let key_pair = KeyPair::generate()
        .map_err(|e| GatewayError::Tls(format!("failed to generate domain key pair: {e}")))?;

    let mut params = CertificateParams::new(vec![domain.to_string()])
        .map_err(|e| GatewayError::Tls(format!("failed to create domain cert params: {e}")))?;
    params.distinguished_name.push(
        rcgen::DnType::CommonName,
        rcgen::DnValue::Utf8String(domain.to_string()),
    );

    let cert = params.self_signed(&key_pair).map_err(|e| {
        GatewayError::Tls(format!(
            "failed to generate self-signed cert for '{}': {}",
            domain, e
        ))
    })?;

    Ok((cert.der().to_vec(), key_pair.serialize_der()))
}

/// Check whether a file path has a PEM-like extension.
fn is_pem_extension(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e == "pem" || e == "crt" || e == "key")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn tls_provider_creation() {
        let source = CertificateSource::SelfSigned {
            common_name: "test.local".into(),
        };
        let provider = TlsProvider::new(source);
        assert!(matches!(
            provider.cert_source,
            CertificateSource::SelfSigned { .. }
        ));
    }

    #[test]
    fn self_signed_generates_valid_cert_and_key() {
        let (certs, _key) =
            TlsProvider::generate_self_signed("gateway.test").expect("test: generate self-signed");
        assert_eq!(certs.len(), 1, "should produce exactly one certificate");
        assert!(!certs[0].is_empty(), "certificate DER should not be empty");
    }

    #[test]
    fn build_server_config_succeeds_with_self_signed() {
        // Ensure rustls crypto provider is installed (may already be from another test)
        let _ = rustls::crypto::ring::default_provider().install_default();

        let (certs, key) =
            TlsProvider::generate_self_signed("server.test").expect("test: generate self-signed");
        let config = TlsProvider::build_server_config(certs, key);
        assert!(config.is_ok(), "server config should build successfully");

        let config = config.expect("test: already checked");
        assert_eq!(config.alpn_protocols, vec![b"h3".to_vec()]);
    }

    #[tokio::test]
    async fn load_certificates_self_signed_via_provider() {
        let provider = TlsProvider::new(CertificateSource::SelfSigned {
            common_name: "provider.test".into(),
        });
        let result = provider.load_certificates().await;
        assert!(result.is_ok(), "self-signed load should succeed");

        let (certs, _key) = result.expect("test: already checked");
        assert_eq!(certs.len(), 1);
    }

    #[tokio::test]
    async fn load_certificates_trustchain_falls_back_to_self_signed() {
        let provider = TlsProvider::new(CertificateSource::TrustChain {
            ca_addr: "[::1]:9000".parse().expect("test: valid addr"),
            common_name: "tc-fallback.test".into(),
        });
        let result = provider.load_certificates().await;
        assert!(
            result.is_ok(),
            "TrustChain source should fall back to self-signed"
        );
    }

    #[tokio::test]
    async fn load_from_missing_file_returns_error() {
        let provider = TlsProvider::new(CertificateSource::File {
            cert_path: PathBuf::from("/nonexistent/cert.pem"),
            key_path: PathBuf::from("/nonexistent/key.pem"),
        });
        let result = provider.load_certificates().await;
        assert!(result.is_err(), "missing file should return error");

        let err_msg = result.expect_err("test: already checked").to_string();
        assert!(
            err_msg.contains("failed to open"),
            "error should mention file open failure: {err_msg}"
        );
    }

    #[test]
    fn is_pem_extension_detection() {
        assert!(is_pem_extension(Path::new("server.pem")));
        assert!(is_pem_extension(Path::new("server.crt")));
        assert!(is_pem_extension(Path::new("server.key")));
        assert!(!is_pem_extension(Path::new("server.der")));
        assert!(!is_pem_extension(Path::new("server")));
    }

    // ===== generate_domain_cert (3 tests) =================================

    #[test]
    fn generate_domain_cert_produces_nonempty_output() {
        let (cert_der, key_der) =
            generate_domain_cert("test.hypermesh.online").expect("test: generate domain cert");
        assert!(!cert_der.is_empty(), "cert DER should not be empty");
        assert!(!key_der.is_empty(), "key DER should not be empty");
    }

    #[test]
    fn generate_domain_cert_different_domains_produce_different_keys() {
        let (_, key1) =
            generate_domain_cert("a.hypermesh.online").expect("test: generate cert a");
        let (_, key2) =
            generate_domain_cert("b.hypermesh.online").expect("test: generate cert b");
        // Different key pairs should produce different DER encodings.
        assert_ne!(key1, key2);
    }

    #[test]
    fn generate_domain_cert_usable_with_rustls() {
        let _ = rustls::crypto::ring::default_provider().install_default();

        let (cert_der, key_der) =
            generate_domain_cert("rustls-test.hypermesh.online").expect("test: generate cert");
        let cert = CertificateDer::from(cert_der);
        let key = PrivateKeyDer::from(PrivatePkcs8KeyDer::from(key_der));
        let config = TlsProvider::build_server_config(vec![cert], key);
        assert!(config.is_ok(), "domain cert should build a valid server config");
    }
}
