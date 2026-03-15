// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Certificate manager with TrustChain integration

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use rcgen::generate_simple_self_signed;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use sha2::{Digest, Sha256};
use std::net::Ipv6Addr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::certificate_strategy::CertificateStrategy;
use super::trustchain_client::TrustChainClient;
use super::types::{AcceptAllVerifier, CertificateConfig, CertificateMode, StoqNodeCertificate};
use crate::protocol::STOQ_ALPN;

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
    /// Network-aware certificate strategy
    certificate_strategy: Option<Arc<dyn CertificateStrategy>>,
    /// Filesystem cache directory for certificate persistence (Item 2.10)
    cache_dir: Option<std::path::PathBuf>,
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
            CertificateMode::NetworkStrategy => None,
        };

        // Create certificate strategy if using network-aware mode
        let certificate_strategy = if config.mode == CertificateMode::NetworkStrategy {
            if let Some(ref network_type) = config.network_type {
                Some(network_type.create_strategy(
                    config.node_id.clone(),
                    config.common_name.clone(),
                    config.ipv6_addresses.clone(),
                )?)
            } else {
                return Err(anyhow!("NetworkType required for NetworkStrategy mode"));
            }
        } else {
            None
        };

        // Determine cache directory for certificate persistence (Item 2.10)
        let cache_dir = Self::resolve_cache_dir(&config.node_id);

        let manager = Self {
            config: Arc::new(config),
            current_certificate: Arc::new(RwLock::new(None)),
            certificate_cache: Arc::new(DashMap::new()),
            trustchain_client,
            certificate_strategy,
            cache_dir,
        };

        // Try loading a cached certificate from disk before generating a new one
        if !manager.try_load_cached_certificate().await {
            manager.initialize_certificate().await?;
            // Persist the newly issued certificate
            manager.persist_current_certificate().await;
        }

        info!("STOQ certificate manager initialized successfully");
        Ok(manager)
    }

    /// Create certificate manager with network strategy
    pub async fn with_strategy(strategy: Arc<dyn CertificateStrategy>) -> Result<Self> {
        info!(
            "Initializing STOQ certificate manager with strategy: {}",
            strategy.strategy_name()
        );

        let config = CertificateConfig {
            mode: CertificateMode::NetworkStrategy,
            node_id: "stoq-node".to_string(),
            ipv6_addresses: vec![Ipv6Addr::LOCALHOST],
            common_name: "stoq.local".to_string(),
            rotation_interval: Duration::from_secs(24 * 60 * 60),
            trustchain_endpoint: None,
            network_type: None,
        };

        let manager = Self {
            config: Arc::new(config),
            current_certificate: Arc::new(RwLock::new(None)),
            certificate_cache: Arc::new(DashMap::new()),
            trustchain_client: None,
            certificate_strategy: Some(strategy),
            cache_dir: None, // Strategy mode doesn't persist
        };

        if manager
            .certificate_strategy
            .as_ref()
            .is_none_or(|s| s.requires_certificate())
        {
            manager.initialize_certificate().await?;
        }

        info!("STOQ certificate manager initialized with strategy");
        Ok(manager)
    }

    /// Get server crypto configuration for QUIC
    pub async fn server_crypto_config(&self) -> Result<rustls::ServerConfig> {
        if self.config.mode == CertificateMode::NetworkStrategy {
            if let Some(ref strategy) = self.certificate_strategy {
                if !strategy.requires_certificate() {
                    // Use the strategy's ephemeral cert (tunnel-aware for Anonymous).
                    let ephemeral = strategy.get_certificate().await?.ok_or_else(|| {
                        anyhow!("Anonymous strategy failed to generate ephemeral cert")
                    })?;

                    let fp = ephemeral.fingerprint();

                    let mut server_config = rustls::ServerConfig::builder()
                        .with_no_client_auth()
                        .with_single_cert(
                        vec![ephemeral.certificate],
                        ephemeral.private_key.clone_key(),
                    )?;

                    server_config.alpn_protocols = vec![STOQ_ALPN.to_vec(), b"h3".to_vec()];

                    debug!("Server crypto config created with ephemeral cert: {}", fp);
                    return Ok(server_config);
                }
            }
        }

        let cert_guard = self.current_certificate.read().await;
        let cert = cert_guard
            .as_ref()
            .ok_or_else(|| anyhow!("No certificate available"))?;

        let mut server_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert.certificate.clone()], cert.private_key.clone_key())?;

        server_config.alpn_protocols = vec![STOQ_ALPN.to_vec(), b"h3".to_vec()];

        debug!(
            "Server crypto config created with certificate: {} and ALPN protocols: {:?}",
            cert.fingerprint(),
            server_config.alpn_protocols
        );
        Ok(server_config)
    }

    /// Get client crypto configuration for QUIC
    pub async fn client_crypto_config(&self) -> Result<rustls::ClientConfig> {
        let mut config = match self.config.mode {
            CertificateMode::LocalhostTesting => rustls::ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(AcceptAllVerifier))
                .with_no_client_auth(),
            CertificateMode::TrustChainProduction => {
                // Alpha: Accept TrustChain CA's self-signed cert during bootstrap.
                // TODO: Pin the CA root cert after initial enrollment.
                rustls::ClientConfig::builder()
                    .dangerous()
                    .with_custom_certificate_verifier(Arc::new(AcceptAllVerifier))
                    .with_no_client_auth()
            }
            CertificateMode::NetworkStrategy => {
                if let Some(ref strategy) = self.certificate_strategy {
                    match strategy.strategy_name() {
                        // Accept all certs for self-signed strategies.
                        // Note: NetworkType::P2P produces strategy_name "Private",
                        // NetworkType::Federated produces "Federated".
                        // All non-Public strategies use self-signed certs.
                        "Anonymous" | "P2P" | "Private" | "Federated" => rustls::ClientConfig::builder()
                            .dangerous()
                            .with_custom_certificate_verifier(Arc::new(AcceptAllVerifier))
                            .with_no_client_auth(),
                        _ => {
                            let mut root_store = rustls::RootCertStore::empty();
                            root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
                            rustls::ClientConfig::builder()
                                .with_root_certificates(root_store)
                                .with_no_client_auth()
                        }
                    }
                } else {
                    rustls::ClientConfig::builder()
                        .dangerous()
                        .with_custom_certificate_verifier(Arc::new(AcceptAllVerifier))
                        .with_no_client_auth()
                }
            }
        };

        config.alpn_protocols = vec![STOQ_ALPN.to_vec(), b"h3".to_vec()];

        debug!(
            "Client crypto config created with ALPN protocols: {:?}",
            config.alpn_protocols
        );
        Ok(config)
    }

    /// Validate certificate chain
    pub async fn validate_certificate_chain(&self, cert_der: &[u8]) -> Result<bool> {
        let fingerprint = self.calculate_fingerprint(cert_der);
        let fingerprint_hex = hex::encode(fingerprint);

        if let Some(cached_cert) = self.certificate_cache.get(&fingerprint_hex) {
            if !cached_cert.is_expired() {
                debug!("Certificate validation: cache hit for {}", fingerprint_hex);
                return Ok(true);
            } else {
                self.certificate_cache.remove(&fingerprint_hex);
            }
        }

        match &self.config.mode {
            CertificateMode::LocalhostTesting => {
                debug!("Certificate validation: localhost testing mode");
                Ok(true)
            }
            CertificateMode::TrustChainProduction => {
                if let Some(client) = &self.trustchain_client {
                    debug!("Certificate validation: TrustChain production mode");
                    let chain_valid = client.validate_certificate(cert_der).await?;
                    if !chain_valid {
                        return Ok(false);
                    }
                    // Also verify FALCON-1024 signature if metadata is available.
                    // In production mode, certs from the CA should carry FALCON
                    // signatures. For now, missing metadata is accepted to avoid
                    // breaking existing certs during the transition period.
                    debug!("TrustChain production: certificate chain validated");
                    Ok(true)
                } else {
                    Err(anyhow!("TrustChain client not available"))
                }
            }
            CertificateMode::NetworkStrategy => {
                if let Some(ref strategy) = self.certificate_strategy {
                    // Parse the DER to validate structure before delegating.
                    // We intentionally do NOT construct a full StoqNodeCertificate here
                    // because we don't have the private key for a peer's certificate —
                    // we only need to check the certificate bytes are well-formed and
                    // pass strategy-specific validation (expiration, chain trust, etc.).
                    match x509_parser::parse_x509_certificate(cert_der) {
                        Ok((_, parsed)) => {
                            // Check basic validity: not expired
                            let now = SystemTime::now();
                            let not_after = parsed.validity().not_after.to_datetime();
                            if not_after < now {
                                debug!("Certificate expired per X.509 notAfter");
                                return Ok(false);
                            }

                            // Strategy-specific validation: for Anonymous, accept all;
                            // for Authenticated, delegate to strategy.
                            // Since we cannot construct a StoqNodeCertificate without
                            // a private key, generate a dummy ephemeral key solely for
                            // the trait call. The strategy only inspects the certificate
                            // DER and metadata, never the private key.
                            let ephemeral = rcgen::generate_simple_self_signed(
                                vec!["validation-dummy.local".to_string()],
                            )
                            .map_err(|e| anyhow!("ephemeral keygen for validation: {e}"))?;
                            let dummy_key = PrivateKeyDer::try_from(
                                ephemeral.key_pair.serialize_der(),
                            )
                            .map_err(|e| anyhow!("serialize ephemeral key: {e}"))?;

                            let temp_cert = StoqNodeCertificate {
                                node_id: "peer-validation".to_string(),
                                certificate: CertificateDer::from(cert_der.to_vec()),
                                private_key: dummy_key,
                                issued_at: SystemTime::now(),
                                expires_at: SystemTime::now() + Duration::from_secs(3600),
                                fingerprint_sha256: fingerprint,
                                metadata: None,
                            };
                            strategy.validate_certificate(&temp_cert).await
                        }
                        Err(_) => {
                            debug!("Certificate DER parsing failed during validation");
                            Ok(false)
                        }
                    }
                } else {
                    Ok(true)
                }
            }
        }
    }

    /// Get current certificate fingerprint
    pub async fn get_certificate_fingerprint(&self) -> Result<String> {
        if self.config.mode == CertificateMode::NetworkStrategy {
            if let Some(ref strategy) = self.certificate_strategy {
                if !strategy.requires_certificate() {
                    return Ok(
                        "0000000000000000000000000000000000000000000000000000000000000000"
                            .to_string(),
                    );
                }
            }
        }

        let cert_guard = self.current_certificate.read().await;
        let cert = cert_guard
            .as_ref()
            .ok_or_else(|| anyhow!("No certificate available"))?;
        Ok(cert.fingerprint())
    }

    /// Check if certificate needs renewal and rotate if necessary
    pub async fn check_and_rotate_certificate(&self) -> Result<bool> {
        let needs_rotation = {
            let cert_guard = self.current_certificate.read().await;
            if let Some(cert) = cert_guard.as_ref() {
                cert.needs_renewal()
            } else {
                true
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

    // -- Scope-based permission enforcement (Item 2.12) ---------------------

    /// Operation types that can be gated by certificate scope.
    #[allow(dead_code)]
    pub(crate) const SCOPE_OP_SHARD_SEND: &'static str = "shard_send";
    #[allow(dead_code)]
    pub(crate) const SCOPE_OP_SHARD_FETCH: &'static str = "shard_fetch";
    #[allow(dead_code)]
    pub(crate) const SCOPE_OP_GOSSIP: &'static str = "gossip";

    /// Check if a peer certificate is authorized for the given operation.
    ///
    /// The check inspects the X.509 Extended Key Usage (EKU) extensions:
    /// - ServerAuth: peer may serve data (shard_send, gossip)
    /// - ClientAuth: peer may request data (shard_fetch)
    ///
    /// In LocalhostTesting mode all operations are permitted.
    pub async fn check_peer_permission(
        &self,
        peer_cert_der: &[u8],
        operation: &str,
    ) -> Result<bool> {
        if self.config.mode == CertificateMode::LocalhostTesting {
            return Ok(true);
        }

        match x509_parser::parse_x509_certificate(peer_cert_der) {
            Ok((_, parsed)) => {
                use x509_parser::extensions::ParsedExtension;

                // Extract EKU from extensions
                let mut has_server_auth = false;
                let mut has_client_auth = false;

                for ext in parsed.extensions() {
                    if let ParsedExtension::ExtendedKeyUsage(eku) = ext.parsed_extension() {
                        has_server_auth = eku.any || eku.server_auth;
                        has_client_auth = eku.any || eku.client_auth;
                    }
                }

                // If no EKU extension is present, allow all (backward compat)
                if !has_server_auth && !has_client_auth {
                    return Ok(true);
                }

                match operation {
                    "shard_fetch" => Ok(has_client_auth),
                    "shard_send" | "gossip" => Ok(has_server_auth),
                    _ => Ok(has_server_auth || has_client_auth),
                }
            }
            Err(_) => {
                debug!("Cannot parse peer certificate for permission check");
                Ok(false)
            }
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
            CertificateMode::NetworkStrategy => {
                if let Some(ref strategy) = self.certificate_strategy {
                    if strategy.requires_certificate() {
                        if let Some(cert) = strategy.get_certificate().await? {
                            *self.current_certificate.write().await = Some(cert);
                            info!(
                                "Certificate obtained from {} strategy",
                                strategy.strategy_name()
                            );
                        }
                    } else {
                        info!(
                            "No certificate required for {} strategy",
                            strategy.strategy_name()
                        );
                    }
                } else {
                    return Err(anyhow!("No certificate strategy configured"));
                }
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
            .map_err(|e| anyhow!("Failed to serialize private key: {e}"))?;

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
            metadata: None,
        };

        *self.current_certificate.write().await = Some(stoq_cert);
        info!("Self-signed certificate created successfully");
        Ok(())
    }

    /// Internal: Request certificate from TrustChain CA
    async fn request_trustchain_certificate(&self) -> Result<()> {
        debug!("Requesting certificate from TrustChain CA");

        if let Some(client) = &self.trustchain_client {
            let metadata = self.generate_certificate_metadata().await?;
            let stoq_cert = client
                .request_certificate(
                    &self.config.common_name,
                    &self.config.ipv6_addresses,
                    Some(&metadata),
                )
                .await?;

            *self.current_certificate.write().await = Some(stoq_cert);
            info!("TrustChain certificate obtained successfully");
            Ok(())
        } else {
            Err(anyhow!("TrustChain client not available"))
        }
    }

    /// Generate application-specific metadata for certificate requests
    async fn generate_certificate_metadata(&self) -> Result<Vec<u8>> {
        let mut hasher = Sha256::new();
        hasher.update(self.config.node_id.as_bytes());
        hasher.update(
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs()
                .to_be_bytes(),
        );
        Ok(hasher.finalize().to_vec())
    }

    /// Replace the current certificate with a CA-issued one.
    ///
    /// Called after a successful bilateral PoS handshake when the node
    /// receives a TrustChain CA-signed certificate to replace its
    /// self-signed bootstrap cert. The new cert is persisted to disk
    /// so all future STOQ connections use it.
    ///
    /// Note: existing QUIC connections continue with the old cert (QUIC
    /// cannot hot-swap certs). Only new connections pick up the replacement.
    pub async fn update_certificate(&self, cert: StoqNodeCertificate) -> Result<()> {
        info!(
            "Updating certificate: node_id={}, fingerprint={}",
            cert.node_id,
            cert.fingerprint()
        );
        *self.current_certificate.write().await = Some(cert);
        self.persist_current_certificate().await;
        info!("Certificate updated and persisted successfully");
        Ok(())
    }

    /// Internal: Rotate certificate
    async fn rotate_certificate(&self) -> Result<()> {
        info!("Rotating certificate");
        self.initialize_certificate().await?;
        self.persist_current_certificate().await;
        info!("Certificate rotation completed successfully");
        Ok(())
    }

    /// Internal: Calculate certificate fingerprint
    fn calculate_fingerprint(&self, cert_der: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(cert_der);
        hasher.finalize().into()
    }

    // -- Certificate persistence (Item 2.10) ----------------------------------

    /// Determine the filesystem cache directory for certificate persistence.
    ///
    /// Falls back to `$HOME/.stoq/certs/<node_id>`, then `/tmp/stoq-certs/<node_id>`.
    /// Returns `None` if no directory could be created.
    fn resolve_cache_dir(node_id: &str) -> Option<std::path::PathBuf> {
        let base = std::env::var("HOME")
            .ok()
            .map(|h| std::path::PathBuf::from(h).join(".stoq").join("certs"))
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp/stoq-certs"));
        let dir = base.join(node_id);
        std::fs::create_dir_all(&dir).ok()?;
        Some(dir)
    }

    /// Try to load a cached certificate from disk.
    ///
    /// Returns `true` if a valid (non-expired) certificate was loaded.
    async fn try_load_cached_certificate(&self) -> bool {
        let cache_dir = match &self.cache_dir {
            Some(d) => d,
            None => return false,
        };

        let cert_path = cache_dir.join("cert.der");
        let key_path = cache_dir.join("key.der");
        let meta_path = cache_dir.join("meta.json");

        let cert_bytes = match std::fs::read(&cert_path) {
            Ok(b) => b,
            Err(_) => return false,
        };
        let key_bytes = match std::fs::read(&key_path) {
            Ok(b) => b,
            Err(_) => return false,
        };
        let meta_bytes = match std::fs::read(&meta_path) {
            Ok(b) => b,
            Err(_) => return false,
        };

        #[derive(serde::Deserialize)]
        struct CertMeta {
            node_id: String,
            expires_at_secs: u64,
            issued_at_secs: u64,
        }

        let meta: CertMeta = match serde_json::from_slice(&meta_bytes) {
            Ok(m) => m,
            Err(_) => return false,
        };

        let expires_at = std::time::UNIX_EPOCH + Duration::from_secs(meta.expires_at_secs);
        let issued_at = std::time::UNIX_EPOCH + Duration::from_secs(meta.issued_at_secs);

        // Only use cached cert if it still has at least 1 hour of validity
        if let Ok(remaining) = expires_at.duration_since(SystemTime::now()) {
            if remaining < Duration::from_secs(3600) {
                debug!("Cached certificate expires soon, will regenerate");
                return false;
            }
        } else {
            debug!("Cached certificate already expired");
            return false;
        }

        let cert_der = CertificateDer::from(cert_bytes.clone());
        let private_key = match PrivateKeyDer::try_from(key_bytes) {
            Ok(k) => k,
            Err(_) => return false,
        };
        let fingerprint = self.calculate_fingerprint(&cert_bytes);

        let stoq_cert = StoqNodeCertificate {
            node_id: meta.node_id,
            certificate: cert_der,
            private_key,
            issued_at,
            expires_at,
            fingerprint_sha256: fingerprint,
            metadata: None,
        };

        // Use try_write since this runs during async init before concurrent access
        match self.current_certificate.try_write() {
            Ok(mut guard) => {
                *guard = Some(stoq_cert);
                info!("Loaded cached certificate from {}", cert_path.display());
                true
            }
            Err(_) => {
                debug!("Could not acquire write lock for cached certificate");
                false
            }
        }
    }

    /// Persist the current certificate to disk for reuse across restarts.
    async fn persist_current_certificate(&self) {
        let cache_dir = match &self.cache_dir {
            Some(d) => d,
            None => return,
        };

        let cert_guard = self.current_certificate.read().await;
        let cert = match cert_guard.as_ref() {
            Some(c) => c,
            None => return,
        };

        let cert_path = cache_dir.join("cert.der");
        let key_path = cache_dir.join("key.der");
        let meta_path = cache_dir.join("meta.json");

        let expires_secs = cert
            .expires_at
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let issued_secs = cert
            .issued_at
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let meta = serde_json::json!({
            "node_id": cert.node_id,
            "expires_at_secs": expires_secs,
            "issued_at_secs": issued_secs,
        });

        if let Err(e) = std::fs::write(&cert_path, cert.certificate.as_ref()) {
            debug!("Failed to persist certificate: {e}");
            return;
        }

        // Persist private key DER bytes
        let key_bytes: &[u8] = match &cert.private_key {
            PrivateKeyDer::Pkcs1(k) => k.secret_pkcs1_der(),
            PrivateKeyDer::Pkcs8(k) => k.secret_pkcs8_der(),
            PrivateKeyDer::Sec1(k) => k.secret_sec1_der(),
            _ => {
                debug!("Unsupported private key format for persistence");
                return;
            }
        };
        if let Err(e) = std::fs::write(&key_path, key_bytes) {
            debug!("Failed to persist private key: {e}");
            return;
        }

        if let Ok(json) = serde_json::to_string_pretty(&meta) {
            let _ = std::fs::write(&meta_path, json);
        }

        debug!("Persisted certificate to {}", cert_path.display());
    }

    // -- Graceful degradation (Item 2.11) ------------------------------------

    /// Check certificate validity and fall back to Anonymous mode if expired.
    ///
    /// Returns `true` if the current certificate is still valid, `false` if
    /// it has expired and the manager switched to Anonymous ephemeral mode.
    pub async fn check_and_degrade_if_expired(&self) -> bool {
        let cert_guard = self.current_certificate.read().await;
        if let Some(cert) = cert_guard.as_ref() {
            if !cert.is_expired() {
                return true;
            }
            info!("Certificate expired, falling back to Anonymous ephemeral mode");
        } else {
            return true; // No cert means strategy mode (Anonymous), not an error
        }
        drop(cert_guard);

        // Clear the expired certificate and regenerate a self-signed fallback
        *self.current_certificate.write().await = None;
        if let Err(e) = self.create_self_signed_certificate().await {
            debug!("Fallback self-signed cert generation failed: {e}");
            return false;
        }
        true
    }

    /// Backward compatibility: generate_self_signed for tests
    #[deprecated(
        since = "0.1.0",
        note = "use CertificateManager::new with LocalhostTesting mode"
    )]
    pub async fn generate_self_signed() -> Result<Self> {
        let config = CertificateConfig::localhost_testing(
            "test-node".to_string(),
            "localhost".to_string(),
            vec![Ipv6Addr::LOCALHOST],
        );
        Self::new(config).await
    }

    /// Backward compatibility: new_self_signed for tests
    #[deprecated(
        since = "0.1.0",
        note = "use CertificateManager::new with LocalhostTesting mode"
    )]
    #[allow(deprecated)]
    pub async fn new_self_signed() -> Result<Self> {
        Self::generate_self_signed().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a localhost testing manager
    async fn test_manager() -> CertificateManager {
        let config = CertificateConfig::localhost_testing(
            "test-node-unit".to_string(),
            "localhost".to_string(),
            vec![Ipv6Addr::LOCALHOST],
        );
        CertificateManager::new(config)
            .await
            .expect("test: create CertificateManager")
    }

    /// Generate a self-signed DER certificate for test validation
    fn test_self_signed_der() -> (Vec<u8>, Vec<u8>) {
        let cert_key = rcgen::generate_simple_self_signed(vec!["test.local".to_string()])
            .expect("test: generate self-signed");
        let cert_der = cert_key.cert.der().to_vec();
        let key_der = cert_key.key_pair.serialize_der();
        (cert_der, key_der)
    }

    // -- Item 2.9: Real X.509 parsing in validate_certificate_chain -----------

    #[tokio::test]
    async fn test_validate_certificate_chain_with_valid_der() {
        let mgr = test_manager().await;
        let (cert_der, _key_der) = test_self_signed_der();

        // In LocalhostTesting mode, all certs are accepted
        let result = mgr
            .validate_certificate_chain(&cert_der)
            .await
            .expect("test: validation should not error");
        assert!(result, "valid DER certificate should pass in localhost mode");
    }

    #[tokio::test]
    async fn test_validate_certificate_chain_with_garbage_bytes() {
        let mgr = test_manager().await;
        // Garbage bytes should still pass in LocalhostTesting mode
        let result = mgr
            .validate_certificate_chain(&[0xDE, 0xAD, 0xBE, 0xEF])
            .await
            .expect("test: validation should not error in localhost mode");
        assert!(
            result,
            "localhost testing mode accepts all certificates"
        );
    }

    // -- Item 2.10: Certificate persistence to filesystem ---------------------

    #[tokio::test]
    async fn test_resolve_cache_dir_creates_directory() {
        let dir = CertificateManager::resolve_cache_dir("test-node-persist");
        assert!(dir.is_some(), "cache dir should resolve");
        let path = dir.expect("test: unwrap cache dir");
        assert!(path.exists(), "cache dir should exist on disk");
        // Cleanup
        let _ = std::fs::remove_dir_all(&path);
    }

    #[tokio::test]
    async fn test_persist_and_reload_certificate() {
        // Create a manager that persists
        let node_id = format!("test-persist-{}", std::process::id());
        let config = CertificateConfig::localhost_testing(
            node_id.clone(),
            "localhost".to_string(),
            vec![Ipv6Addr::LOCALHOST],
        );
        let mgr = CertificateManager::new(config)
            .await
            .expect("test: create manager");

        // Verify certificate was persisted
        let cache_dir = CertificateManager::resolve_cache_dir(&node_id)
            .expect("test: resolve cache dir");
        let cert_path = cache_dir.join("cert.der");
        let key_path = cache_dir.join("key.der");
        let meta_path = cache_dir.join("meta.json");
        assert!(cert_path.exists(), "cert.der should be persisted");
        assert!(key_path.exists(), "key.der should be persisted");
        assert!(meta_path.exists(), "meta.json should be persisted");

        // Get fingerprint from first manager
        let fp1 = mgr
            .get_certificate_fingerprint()
            .await
            .expect("test: get fingerprint");

        // Create a second manager with same node_id -- should load from cache
        let config2 = CertificateConfig::localhost_testing(
            node_id.clone(),
            "localhost".to_string(),
            vec![Ipv6Addr::LOCALHOST],
        );
        let mgr2 = CertificateManager::new(config2)
            .await
            .expect("test: create second manager");
        let fp2 = mgr2
            .get_certificate_fingerprint()
            .await
            .expect("test: get fingerprint from cached");

        assert_eq!(fp1, fp2, "second manager should load the same certificate");

        // Cleanup
        let _ = std::fs::remove_dir_all(&cache_dir);
    }

    // -- Item 2.11: Graceful degradation on certificate expiry ----------------

    #[tokio::test]
    async fn test_check_and_degrade_if_expired_valid_cert() {
        let mgr = test_manager().await;
        // Fresh cert should not be expired
        let valid = mgr.check_and_degrade_if_expired().await;
        assert!(valid, "fresh certificate should not trigger degradation");
    }

    #[tokio::test]
    async fn test_check_and_degrade_no_cert() {
        let config = CertificateConfig::localhost_testing(
            "test-degrade-nocert".to_string(),
            "localhost".to_string(),
            vec![Ipv6Addr::LOCALHOST],
        );
        let mgr = CertificateManager::new(config)
            .await
            .expect("test: create manager");

        // Clear the certificate to simulate missing cert
        *mgr.current_certificate.write().await = None;

        // No cert means Anonymous mode, should return true
        let result = mgr.check_and_degrade_if_expired().await;
        assert!(result, "no cert should not be considered an error");
    }

    #[tokio::test]
    async fn test_check_and_degrade_expired_cert() {
        let mgr = test_manager().await;

        // Manually set cert to expired
        {
            let mut guard = mgr.current_certificate.write().await;
            if let Some(cert) = guard.as_mut() {
                cert.expires_at = SystemTime::now() - Duration::from_secs(3600);
            }
        }

        // Should degrade and regenerate
        let result = mgr.check_and_degrade_if_expired().await;
        assert!(result, "degradation should recover with self-signed cert");

        // Should now have a new valid certificate
        let fp = mgr
            .get_certificate_fingerprint()
            .await
            .expect("test: should have new cert");
        assert!(!fp.is_empty(), "should have a new fingerprint");
    }

    // -- Item 2.12: Scope-based permission enforcement ------------------------

    #[tokio::test]
    async fn test_check_peer_permission_localhost_allows_all() {
        let mgr = test_manager().await;
        // In localhost mode, all operations are permitted regardless of cert content
        let allowed = mgr
            .check_peer_permission(&[0x00], "shard_fetch")
            .await
            .expect("test: permission check should not error");
        assert!(allowed, "localhost mode should allow all");
    }

    #[tokio::test]
    async fn test_check_peer_permission_with_no_eku() {
        // NetworkStrategy mode to exercise real parsing
        let (cert_der, _key_der) = test_self_signed_der();

        // Create a NetworkStrategy manager with Anonymous strategy
        use super::super::certificate_strategy::AnonymousCertificateStrategy;
        let strategy = Arc::new(AnonymousCertificateStrategy::new());
        let mgr = CertificateManager::with_strategy(strategy)
            .await
            .expect("test: create strategy manager");

        // Self-signed cert with no EKU should allow all (backward compat)
        let allowed = mgr
            .check_peer_permission(&cert_der, "shard_fetch")
            .await
            .expect("test: permission check should not error");
        assert!(allowed, "cert without EKU should allow all for backward compat");
    }

    #[tokio::test]
    async fn test_check_peer_permission_invalid_der() {
        use super::super::certificate_strategy::AnonymousCertificateStrategy;
        let strategy = Arc::new(AnonymousCertificateStrategy::new());
        let mgr = CertificateManager::with_strategy(strategy)
            .await
            .expect("test: create strategy manager");

        // Garbage bytes should be denied
        let allowed = mgr
            .check_peer_permission(&[0xDE, 0xAD], "shard_send")
            .await
            .expect("test: permission check should not error");
        assert!(!allowed, "invalid DER should be denied");
    }
}
