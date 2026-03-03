// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! TrustChain client for certificate operations over QUIC

use anyhow::{anyhow, Result};
use base64::prelude::*;
use quinn;
use rcgen::generate_simple_self_signed;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use sha2::{Digest, Sha256};
use std::net::Ipv6Addr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tracing::{debug, info, warn};

use super::types::StoqNodeCertificate;
use crate::protocol::STOQ_ALPN;

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
        metadata: Option<&[u8]>,
    ) -> Result<StoqNodeCertificate> {
        // Local TrustChain: node is its own CA — generate self-signed cert
        if self.endpoint.starts_with("local://") {
            return self.generate_local_certificate(common_name, ipv6_addresses, metadata);
        }

        info!(
            "Requesting certificate from TrustChain CA: {}",
            self.endpoint
        );

        let (host, port) = self.parse_endpoint()?;
        let ipv6_addr = self.resolve_ipv6(host, port).await?;

        // Create QUIC client configuration
        let quinn_config = self.build_quinn_client_config()?;

        // Create endpoint for outgoing connections
        let mut endpoint = quinn::Endpoint::client("[::]:0".parse()?)?;
        endpoint.set_default_client_config(quinn_config);

        // Connect to TrustChain CA
        info!("Connecting to TrustChain CA at {}", ipv6_addr);
        let connection = endpoint
            .connect(ipv6_addr, host)?
            .await
            .map_err(|e| anyhow!("Failed to connect to TrustChain CA: {e}"))?;

        // Open bidirectional stream
        let (mut send_stream, mut recv_stream) = connection.open_bi().await?;

        // Prepare certificate request
        let mut request_json = serde_json::json!({
            "common_name": common_name,
            "san_entries": [common_name],
            "node_id": self.node_id,
            "ipv6_addresses": ipv6_addresses,
        });

        // Add optional metadata if provided
        if let Some(meta) = metadata {
            request_json["metadata"] = serde_json::json!(BASE64_STANDARD.encode(meta));
        }

        let request_body = serde_json::to_string(&request_json)?;
        let request_data = format!(
            "POST /ca/certificate HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            host, request_body.len(), request_body
        );

        // Send request
        send_stream.write_all(request_data.as_bytes()).await?;
        send_stream.finish()?;

        // Read response
        let response = recv_stream.read_to_end(64 * 1024).await?; // 64KB max
        let response_str = String::from_utf8(response)?;

        // Parse HTTP response
        let response_body = self.extract_http_body(&response_str)?;
        let response_json: serde_json::Value = serde_json::from_str(response_body)?;

        // Extract certificate from response
        let certificate = response_json
            .get("certificate")
            .ok_or_else(|| anyhow!("No certificate in TrustChain response"))?;

        let certificate_der_b64 = certificate
            .get("certificate_der")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("No certificate_der in TrustChain response"))?;

        let certificate_der = BASE64_STANDARD.decode(certificate_der_b64)?;
        let fingerprint = self.calculate_fingerprint(&certificate_der);

        // SECURITY FIX: Generate real private key instead of mock
        let private_key = self.generate_real_private_key()?;

        let now = SystemTime::now();
        let expires_at = now + Duration::from_secs(24 * 60 * 60); // 24 hours

        let stoq_cert = StoqNodeCertificate {
            node_id: self.node_id.clone(),
            certificate: CertificateDer::from(certificate_der),
            private_key: PrivateKeyDer::try_from(private_key)
                .map_err(|e| anyhow!("Failed to create private key: {e}"))?,
            issued_at: now,
            expires_at,
            fingerprint_sha256: fingerprint,
            metadata: metadata.map(|m| m.to_vec()),
        };

        info!(
            "Certificate obtained from TrustChain CA: {}",
            stoq_cert.fingerprint()
        );
        Ok(stoq_cert)
    }

    /// Validate certificate with TrustChain CT logs (SECURITY HARDENED)
    pub async fn validate_certificate(&self, cert_der: &[u8]) -> Result<bool> {
        // Local TrustChain: do local structure validation only (no remote CT check)
        if self.endpoint.starts_with("local://") {
            info!("Local TrustChain: validating certificate structure locally");
            return self
                .validate_certificate_structure(cert_der)
                .and_then(|ok| {
                    if ok {
                        self.validate_certificate_expiration(cert_der)
                    } else {
                        Ok(false)
                    }
                })
                .and_then(|ok| {
                    if ok {
                        self.validate_certificate_crypto_strength(cert_der)
                    } else {
                        Ok(false)
                    }
                });
        }

        info!("Validating certificate with TrustChain CT logs (hardened validation)");

        if !self.validate_certificate_structure(cert_der)? {
            warn!("Certificate failed basic structure validation");
            return Ok(false);
        }
        if !self.validate_certificate_expiration(cert_der)? {
            warn!("Certificate expired or invalid time range");
            return Ok(false);
        }
        if !self.validate_certificate_crypto_strength(cert_der)? {
            warn!("Certificate crypto strength insufficient");
            return Ok(false);
        }

        let fingerprint = hex::encode(self.calculate_fingerprint(cert_der));
        let (host, port) = self.parse_endpoint()?;
        let ipv6_addr = self.resolve_ipv6(host, port).await?;

        let quinn_config = self.build_quinn_client_config()?;
        let mut endpoint = quinn::Endpoint::client("[::]:0".parse()?)?;
        endpoint.set_default_client_config(quinn_config);

        let connection = endpoint
            .connect(ipv6_addr, host)?
            .await
            .map_err(|e| anyhow!("Failed to connect to TrustChain CA: {e}"))?;

        let (mut send_stream, mut recv_stream) = connection.open_bi().await?;

        let request_data = format!("GET /ct/proof/{fingerprint} HTTP/1.1\r\nHost: {host}\r\n\r\n");
        send_stream.write_all(request_data.as_bytes()).await?;
        send_stream.finish()?;

        let response = recv_stream.read_to_end(64 * 1024).await?;
        let response_str = String::from_utf8(response)?;
        let response_body = self.extract_http_body(&response_str)?;
        let response_json: serde_json::Value = serde_json::from_str(response_body)?;

        let is_valid = response_json.get("fingerprint").is_some();

        if is_valid {
            let is_not_revoked = self.check_certificate_revocation(cert_der).await?;
            if !is_not_revoked {
                warn!("Certificate has been revoked");
                return Ok(false);
            }
        }

        info!("Certificate CT validation result: {}", is_valid);
        Ok(is_valid)
    }

    /// SECURITY: Check if certificate has been revoked
    async fn check_certificate_revocation(&self, cert_der: &[u8]) -> Result<bool> {
        let fingerprint = hex::encode(self.calculate_fingerprint(cert_der));
        let (host, port) = self.parse_endpoint()?;
        let ipv6_addr = self.resolve_ipv6(host, port).await?;

        // Alpha: Accept TrustChain CA's self-signed cert
        let client_config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(super::types::AcceptAllVerifier))
            .with_no_client_auth();

        let quinn_config = quinn::ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(client_config)?,
        ));

        let mut endpoint = quinn::Endpoint::client("[::]:0".parse()?)?;
        endpoint.set_default_client_config(quinn_config);

        let connection =
            tokio::time::timeout(Duration::from_secs(10), endpoint.connect(ipv6_addr, host)?)
                .await
                .map_err(|_| anyhow!("TrustChain connection timeout"))?
                .map_err(|e| anyhow!("Failed to connect to TrustChain CA: {e}"))?;

        let (mut send_stream, mut recv_stream) = connection.open_bi().await?;

        let request_data =
            format!("GET /ca/revocation/{fingerprint} HTTP/1.1\r\nHost: {host}\r\n\r\n");

        tokio::time::timeout(
            Duration::from_secs(5),
            send_stream.write_all(request_data.as_bytes()),
        )
        .await
        .map_err(|_| anyhow!("TrustChain request timeout"))??;

        send_stream.finish()?;

        let response =
            tokio::time::timeout(Duration::from_secs(5), recv_stream.read_to_end(16 * 1024))
                .await
                .map_err(|_| anyhow!("TrustChain response timeout"))??;

        let response_str = String::from_utf8(response)?;
        let response_body = self.extract_http_body(&response_str)?;
        let response_json: serde_json::Value = serde_json::from_str(response_body)?;

        let is_not_revoked = response_json
            .get("status")
            .and_then(|s| s.as_str())
            .map(|s| s == "not_found" || s == "valid")
            .unwrap_or(false);

        debug!(
            "Certificate revocation check: not_revoked={}",
            is_not_revoked
        );
        Ok(is_not_revoked)
    }

    /// Generate a locally-signed certificate for P2P/local TrustChain mode.
    /// When the endpoint is `local://`, the node acts as its own CA
    /// (node-as-DNS-provider-first principle).
    fn generate_local_certificate(
        &self,
        common_name: &str,
        ipv6_addresses: &[Ipv6Addr],
        metadata: Option<&[u8]>,
    ) -> Result<StoqNodeCertificate> {
        info!(
            "Local TrustChain: generating self-signed certificate for {}",
            common_name
        );

        let mut san_entries = vec![common_name.to_string()];
        for addr in ipv6_addresses {
            san_entries.push(format!("{addr}"));
        }

        let cert_key = generate_simple_self_signed(san_entries)?;
        let cert_der = cert_key.cert.der().clone();
        let private_key_der = PrivateKeyDer::try_from(cert_key.key_pair.serialize_der())
            .map_err(|e| anyhow!("Failed to serialize private key: {e}"))?;

        let fingerprint = self.calculate_fingerprint(cert_der.as_ref());
        let now = SystemTime::now();
        let expires_at = now + Duration::from_secs(24 * 60 * 60); // 24 hours

        let stoq_cert = StoqNodeCertificate {
            node_id: self.node_id.clone(),
            certificate: cert_der,
            private_key: private_key_der,
            issued_at: now,
            expires_at,
            fingerprint_sha256: fingerprint,
            metadata: metadata.map(|m| m.to_vec()),
        };

        info!(
            "Local TrustChain certificate generated: {}",
            stoq_cert.fingerprint()
        );
        Ok(stoq_cert)
    }

    /// Calculate certificate fingerprint
    pub(crate) fn calculate_fingerprint(&self, cert_der: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(cert_der);
        hasher.finalize().into()
    }

    /// Generate cryptographically secure private key (SECURITY FIX)
    pub(crate) fn generate_real_private_key(&self) -> Result<Vec<u8>> {
        let subject_alt_names = vec!["localhost".to_string()];
        let cert = generate_simple_self_signed(subject_alt_names)?;
        let private_key_der = cert.key_pair.serialize_der();
        Ok(private_key_der)
    }

    /// Parse endpoint URL into host and port
    fn parse_endpoint(&self) -> Result<(&str, u16)> {
        let endpoint_url = self
            .endpoint
            .strip_prefix("quic://")
            .unwrap_or(&self.endpoint);

        let (host, port_str) = if endpoint_url.starts_with('[') {
            let close_bracket = endpoint_url
                .find(']')
                .ok_or_else(|| anyhow!("Invalid IPv6 endpoint format: {}", self.endpoint))?;
            let ipv6_addr = &endpoint_url[1..close_bracket];
            let port_part = &endpoint_url[close_bracket + 1..];
            (ipv6_addr, port_part.trim_start_matches(':'))
        } else {
            let parts: Vec<&str> = endpoint_url.split(':').collect();
            if parts.len() != 2 {
                return Err(anyhow!(
                    "Invalid TrustChain endpoint format: {}",
                    self.endpoint
                ));
            }
            (parts[0], parts[1])
        };

        let port = port_str
            .parse::<u16>()
            .map_err(|_| anyhow!("Invalid port in TrustChain endpoint: {port_str}"))?;

        Ok((host, port))
    }

    /// Resolve hostname to IPv6 address
    async fn resolve_ipv6(&self, host: &str, port: u16) -> Result<std::net::SocketAddr> {
        let mut socket_addrs = tokio::net::lookup_host((host, port)).await?;
        socket_addrs
            .find(|addr| addr.is_ipv6())
            .ok_or_else(|| anyhow!("No IPv6 address found for TrustChain host: {host}"))
    }

    /// Build quinn client config with STOQ ALPN
    fn build_quinn_client_config(&self) -> Result<quinn::ClientConfig> {
        // Alpha: Accept TrustChain CA's self-signed cert
        let mut client_config = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(super::types::AcceptAllVerifier))
            .with_no_client_auth();

        client_config.alpn_protocols = vec![STOQ_ALPN.to_vec(), b"h3".to_vec()];

        Ok(quinn::ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(client_config)?,
        )))
    }

    /// Extract HTTP response body from raw response
    fn extract_http_body<'a>(&self, response: &'a str) -> Result<&'a str> {
        let parts: Vec<&str> = response.splitn(2, "\r\n\r\n").collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid HTTP response from TrustChain"));
        }
        Ok(parts[1])
    }

    /// SECURITY: Validate certificate structure and basic constraints
    fn validate_certificate_structure(&self, cert_der: &[u8]) -> Result<bool> {
        match x509_parser::parse_x509_certificate(cert_der) {
            Ok((_, cert)) => {
                if cert.tbs_certificate.subject.iter_common_name().count() == 0 {
                    return Ok(false);
                }
                if let Ok(Some(key_usage)) = cert.tbs_certificate.key_usage() {
                    if !key_usage.value.digital_signature() || !key_usage.value.key_encipherment() {
                        debug!("Certificate missing required key usage flags");
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Err(_) => {
                debug!("Failed to parse certificate DER structure");
                Ok(false)
            }
        }
    }

    /// SECURITY: Validate certificate expiration with time tolerance
    fn validate_certificate_expiration(&self, cert_der: &[u8]) -> Result<bool> {
        match x509_parser::parse_x509_certificate(cert_der) {
            Ok((_, cert)) => {
                let now = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)?
                    .as_secs() as i64;

                let tolerance = 5 * 60; // 5-minute clock skew tolerance

                let not_before = cert.tbs_certificate.validity.not_before.timestamp();
                let not_after = cert.tbs_certificate.validity.not_after.timestamp();

                if now < (not_before - tolerance) {
                    debug!("Certificate not yet valid (clock skew considered)");
                    return Ok(false);
                }
                if now > (not_after + tolerance) {
                    debug!("Certificate expired (clock skew considered)");
                    return Ok(false);
                }
                Ok(true)
            }
            Err(_) => {
                debug!("Failed to parse certificate for expiration check");
                Ok(false)
            }
        }
    }

    /// SECURITY: Validate certificate cryptographic strength
    fn validate_certificate_crypto_strength(&self, cert_der: &[u8]) -> Result<bool> {
        match x509_parser::parse_x509_certificate(cert_der) {
            Ok((_, cert)) => {
                let sig_alg = &cert.signature_algorithm.algorithm;

                if sig_alg == &x509_parser::oid_registry::OID_PKCS1_MD5WITHRSAENC
                    || sig_alg == &x509_parser::oid_registry::OID_PKCS1_SHA1WITHRSA
                {
                    debug!("Certificate uses weak signature algorithm");
                    return Ok(false);
                }

                let alg_oid = cert
                    .tbs_certificate
                    .subject_pki
                    .algorithm
                    .algorithm
                    .to_id_string();
                if alg_oid.contains("1.2.840.113549.1.1") {
                    debug!("RSA algorithm detected: {}", alg_oid);
                }

                Ok(true)
            }
            Err(_) => {
                debug!("Failed to parse certificate for crypto strength check");
                Ok(false)
            }
        }
    }
}
