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

        let manager = Self {
            config: Arc::new(config),
            current_certificate: Arc::new(RwLock::new(None)),
            certificate_cache: Arc::new(DashMap::new()),
            trustchain_client,
            certificate_strategy,
        };

        // Initialize certificate
        manager.initialize_certificate().await?;

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
                let mut root_store = rustls::RootCertStore::empty();
                root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
                rustls::ClientConfig::builder()
                    .with_root_certificates(root_store)
                    .with_no_client_auth()
            }
            CertificateMode::NetworkStrategy => {
                if let Some(ref strategy) = self.certificate_strategy {
                    match strategy.strategy_name() {
                        "Anonymous" | "P2P" => rustls::ClientConfig::builder()
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
                    client.validate_certificate(cert_der).await
                } else {
                    Err(anyhow!("TrustChain client not available"))
                }
            }
            CertificateMode::NetworkStrategy => {
                if let Some(ref strategy) = self.certificate_strategy {
                    let temp_cert = StoqNodeCertificate {
                        node_id: "unknown".to_string(),
                        certificate: CertificateDer::from(cert_der.to_vec()),
                        private_key: PrivateKeyDer::try_from(vec![0u8])
                            .expect("placeholder private key"),
                        issued_at: SystemTime::now(),
                        expires_at: SystemTime::now() + Duration::from_secs(3600),
                        fingerprint_sha256: fingerprint,
                        metadata: None,
                    };
                    strategy.validate_certificate(&temp_cert).await
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

    /// Internal: Rotate certificate
    async fn rotate_certificate(&self) -> Result<()> {
        info!("Rotating certificate");
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
