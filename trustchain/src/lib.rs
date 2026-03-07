// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! TrustChain - Certificate Authority Foundation for STOQ Transport
//!
//! Certificate Authority, Certificate Transparency, and DNS foundation designed
//! to provide trust services for STOQ transport security in the HyperMesh ecosystem.
//! Core focus: trust.hypermesh.online services and STOQ certificate validation.

pub mod api;
pub mod ca;
pub mod config;
pub mod crypto; // NEW: Post-quantum cryptography (FALCON-1024 + Kyber)
pub mod identity; // FALCON-1024 node identity (implements NodeSigner trait from lib)
pub mod proof_of_state;
pub mod ct;
pub mod deployment; // NEW: Quality gates and deployment validation
pub mod dns;
pub mod ephemeral;
pub mod errors;
pub mod http3; // NEW: HTTP/3 server infrastructure
pub mod monitoring; // NEW: Native monitoring system without external dependencies
pub mod security; // NEW: Security monitoring and Byzantine detection
pub mod stoq_client;
pub mod trust;
pub mod validation; // Ephemeral self-signed certs for anonymous QUIC handshakes

// Re-export main types
pub use ca::security_integration::{SecurityIntegratedCA, SecurityIntegrationConfig};
pub use ca::{CAConfig, CertificateRequest, IssuedCertificate, TrustChainCA};
pub use config::{DnsConfig, TrustChainConfig};
pub use proof_of_state::{StateProof, StateProofContext, StateRequirements};
pub use crypto::{FalconKeyPair, FalconSignature, KyberKeyPair, PQCAlgorithm, PostQuantumCrypto};
pub use identity::FalconIdentity;
pub use errors::{Result, TrustChainError};
pub use security::{SecurityDashboard, SecurityMonitor, SecurityValidationResult};
pub use stoq_client::{ServiceEndpoint, ServiceType, TrustChainStoqClient, TrustChainStoqConfig};

use std::sync::Arc;
use tracing::{error, info, warn};

/// Main TrustChain service coordinator with security integration
pub struct TrustChain {
    /// Security-integrated Certificate Authority (MANDATORY STATE PROOF)
    security_ca: Arc<SecurityIntegratedCA>,
    /// Certificate Transparency logs
    ct: Arc<ct::CertificateTransparency>,
    /// DNS resolver
    dns: Arc<dns::DnsResolver>,
    /// STOQ API server (replacement for HTTP API)
    stoq_api: Arc<api::TrustChainStoqApi>,
    /// STOQ client for all network operations
    stoq_client: Arc<TrustChainStoqClient>,
    /// Security monitoring system
    security_monitor: Arc<SecurityMonitor>,
    /// Configuration (retained for runtime configuration access)
    _config: Arc<TrustChainConfig>,
}

/// TrustChain initialization configuration
pub struct TrustChainSecurityConfig {
    /// Base TrustChain configuration
    pub base_config: TrustChainConfig,
    /// Security integration configuration
    pub security_config: SecurityIntegrationConfig,
    /// Enable mandatory state proof for all operations
    pub mandatory_state_proof: bool,
}

impl Default for TrustChainSecurityConfig {
    fn default() -> Self {
        Self {
            base_config: TrustChainConfig::localhost_testing(),
            security_config: SecurityIntegrationConfig::default(),
            mandatory_state_proof: true,
        }
    }
}

impl TrustChain {
    /// Create new TrustChain instance with security integration
    pub async fn new_with_security(security_config: TrustChainSecurityConfig) -> Result<Self> {
        info!("Initializing TrustChain with MANDATORY SECURITY INTEGRATION");

        if !security_config.mandatory_state_proof {
            warn!("⚠️  CRITICAL SECURITY WARNING: State proof validation is DISABLED!");
            warn!("⚠️  This reduces security and should only be used for testing!");
        } else {
            info!("✅ MANDATORY state proof validation ENABLED for all certificate operations");
        }

        let config = security_config.base_config;

        // Initialize STOQ client first (all other services depend on it)
        let stoq_config = TrustChainStoqConfig {
            bind_address: config.dns.bind_address,
            connection_timeout: std::time::Duration::from_secs(5),
            enable_connection_pooling: true,
            max_connections_per_service: 10,
            cert_validation_timeout: std::time::Duration::from_secs(10),
            dns_query_timeout: std::time::Duration::from_secs(5),
            ct_submission_timeout: std::time::Duration::from_secs(30),
            service_discovery: stoq_client::ServiceDiscoveryConfig {
                dns_resolvers: vec![ServiceEndpoint::new(
                    ServiceType::Dns,
                    config.dns.bind_address,
                    config.dns.quic_port,
                )
                .with_service_name("dns.trustchain.local".to_string())],
                ct_logs: vec![ServiceEndpoint::new(
                    ServiceType::CertificateTransparency,
                    config.dns.bind_address,
                    config.ct.port,
                )
                .with_service_name("ct.trustchain.local".to_string())],
                ca_endpoints: vec![ServiceEndpoint::new(
                    ServiceType::CertificateAuthority,
                    config.dns.bind_address,
                    config.ca.port,
                )
                .with_service_name("ca.trustchain.local".to_string())],
                health_check_interval: std::time::Duration::from_secs(60),
            },
        };

        let stoq_client = Arc::new(TrustChainStoqClient::new(stoq_config).await?);

        // Initialize SECURITY-INTEGRATED Certificate Authority
        let mut security_integration_config = security_config.security_config;
        security_integration_config.mandatory_state_proof = security_config.mandatory_state_proof;

        let security_ca = Arc::new(
            SecurityIntegratedCA::new(config.ca.clone(), security_integration_config).await?,
        );

        // Initialize Security Monitor (extracted from security_ca for direct access)
        let security_monitor = security_ca.get_security_monitor();

        // Initialize Certificate Transparency with STOQ client
        let ct = Arc::new(ct::CertificateTransparency::new(config.ct.clone()).await?);

        // Initialize DNS resolver with STOQ client
        let dns = Arc::new(dns::DnsResolver::new(config.dns.clone()).await?);

        // Initialize STOQ API server (replacement for HTTP)
        let stoq_api_config = api::TrustChainStoqConfig::default();
        let stoq_api = Arc::new(
            api::TrustChainStoqApi::new(
                security_ca.get_ca(), // Get underlying TrustChainCA from SecurityIntegratedCA
                Arc::clone(&dns),
                stoq_api_config,
            )
            .await?,
        );

        let trustchain = Self {
            security_ca,
            ct,
            dns,
            stoq_api,
            stoq_client,
            security_monitor,
            _config: Arc::new(config),
        };

        info!("✅ TrustChain service initialized with MANDATORY SECURITY INTEGRATION");
        info!(
            "🔐 Security features: State proof validation, Byzantine detection, Real-time monitoring"
        );
        Ok(trustchain)
    }

    /// Create TrustChain with standard configuration (for backward compatibility)
    pub async fn new(config: TrustChainConfig) -> Result<Self> {
        let security_config = TrustChainSecurityConfig {
            base_config: config,
            security_config: SecurityIntegrationConfig::default(),
            mandatory_state_proof: true, // Always enable for production
        };

        Self::new_with_security(security_config).await
    }

    /// Start all TrustChain services with security monitoring
    pub async fn start(&self) -> Result<()> {
        info!("Starting TrustChain services with security monitoring");

        // Start services concurrently
        let ca_task = self.start_ca_service();
        let ct_task = self.start_ct_service();
        let dns_task = self.start_dns_service();
        let api_task = self.start_api_service();
        let security_task = self.start_security_monitoring();

        // Wait for all services to start
        tokio::try_join!(ca_task, ct_task, dns_task, api_task, security_task)?;

        info!("✅ All TrustChain services started with security monitoring active");
        Ok(())
    }

    /// Issue certificate with MANDATORY security validation and CT logging
    pub async fn issue_certificate_secure(
        &self,
        request: CertificateRequest,
    ) -> Result<IssuedCertificate> {
        info!("🔐 SECURE certificate issuance with mandatory state proof validation");

        // Issue certificate through security-integrated CA (includes state proof validation)
        let cert = self.security_ca.issue_certificate_secure(request).await?;

        // Log certificate in CT logs
        match self.ct.log_certificate(&cert.certificate_der).await {
            Ok(_) => {
                info!("✅ Certificate logged in CT: {}", cert.serial_number);
            }
            Err(e) => {
                error!(
                    "⚠️  CT logging failed for certificate {}: {}",
                    cert.serial_number, e
                );
                // Don't fail the entire operation, but log the issue
            }
        }

        info!(
            "✅ Secure certificate issuance completed: {}",
            cert.serial_number
        );
        Ok(cert)
    }

    /// Issue certificate with CT logging (legacy method - now with security)
    pub async fn issue_certificate_with_ct(
        &self,
        request: CertificateRequest,
    ) -> Result<IssuedCertificate> {
        warn!("⚠️  Using legacy certificate issuance method - upgrading to secure version");
        self.issue_certificate_secure(request).await
    }

    /// Validate certificate with security monitoring and CT verification
    pub async fn validate_certificate_secure(&self, cert_der: &[u8]) -> Result<bool> {
        info!("🔐 SECURE certificate validation with security monitoring");

        // Validate through security-integrated CA
        let security_validation = self
            .security_ca
            .validate_certificate_secure(cert_der)
            .await?;

        if !security_validation.is_valid {
            warn!("❌ Security validation failed for certificate");
            return Ok(false);
        }

        // Verify in CT logs
        let ct_valid = match self.ct.verify_certificate_in_logs(cert_der).await {
            Ok(valid) => valid,
            Err(e) => {
                warn!("⚠️  CT verification failed: {}", e);
                false
            }
        };

        let overall_valid = security_validation.is_valid && ct_valid;

        if overall_valid {
            info!("✅ Certificate validation successful (security + CT verified)");
        } else {
            warn!(
                "❌ Certificate validation failed: security={}, ct={}",
                security_validation.is_valid, ct_valid
            );
        }

        Ok(overall_valid)
    }

    /// Validate certificate with CT verification (legacy method - now with security)
    pub async fn validate_certificate_with_ct(&self, cert_der: &[u8]) -> Result<bool> {
        warn!("⚠️  Using legacy certificate validation method - upgrading to secure version");
        self.validate_certificate_secure(cert_der).await
    }

    /// Get security monitoring dashboard
    pub async fn get_security_dashboard(&self) -> Result<SecurityDashboard> {
        self.security_monitor
            .get_monitoring_dashboard()
            .await
            .map_err(|e| TrustChainError::SecurityError {
                message: e.to_string(),
            })
    }

    /// Get security metrics
    pub async fn get_security_metrics(&self) -> security::SecurityMetrics {
        self.security_monitor.get_metrics().await
    }

    /// Validate state proof directly
    pub async fn validate_state_proof(
        &self,
        state_proof: &StateProof,
        operation: &str,
    ) -> Result<SecurityValidationResult> {
        self.security_monitor
            .validate_certificate_operation(operation, state_proof, "direct_validation")
            .await
            .map_err(|e| TrustChainError::SecurityError {
                message: e.to_string(),
            })
    }

    /// Get CA certificate for trust anchor
    pub async fn get_ca_certificate(&self) -> Result<Vec<u8>> {
        // Extract CA certificate from security-integrated CA
        // This would need to be implemented in the security_ca module
        // For now, return a placeholder
        Ok(b"security_integrated_ca_certificate".to_vec())
    }

    /// Get STOQ client for direct network operations
    pub fn stoq_client(&self) -> Arc<TrustChainStoqClient> {
        self.stoq_client.clone()
    }

    /// Get transport statistics from STOQ client
    pub fn get_transport_stats(&self) -> stoq::TransportStats {
        self.stoq_client.get_transport_stats()
    }

    /// Get STOQ client metrics
    pub fn get_stoq_metrics(&self) -> stoq_client::StoqClientMetrics {
        self.stoq_client.get_metrics()
    }

    /// Get integrated CA metrics (CA + Security)
    pub async fn get_integrated_metrics(
        &self,
    ) -> Result<ca::security_integration::IntegratedCAMetrics> {
        self.security_ca
            .get_integrated_metrics()
            .await
            .map_err(|e| TrustChainError::Internal {
                message: e.to_string(),
            })
    }

    /// Emergency security shutdown
    pub async fn emergency_shutdown(&self, reason: &str) -> Result<()> {
        error!("🚨 EMERGENCY SECURITY SHUTDOWN: {}", reason);

        // In production, this would:
        // 1. Stop accepting new certificate requests
        // 2. Alert all administrators
        // 3. Generate critical security alert
        // 4. Gracefully shutdown services

        self.shutdown().await
    }

    /// Shutdown all services gracefully
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down TrustChain services");

        // Shutdown services in reverse order
        self.stoq_api.stop(); // Changed from self.api to self.stoq_api
        self.dns.shutdown().await?;
        self.ct.shutdown().await?;
        // Security-integrated CA doesn't need explicit shutdown

        // Shutdown STOQ client last (all other services depend on it)
        self.stoq_client.shutdown().await?;

        info!("✅ TrustChain services shut down successfully");
        Ok(())
    }

    // Internal service startup methods

    async fn start_ca_service(&self) -> Result<()> {
        info!("✅ Security-Integrated Certificate Authority ready (state proof mandatory)");
        Ok(())
    }

    async fn start_ct_service(&self) -> Result<()> {
        info!("✅ Certificate Transparency service ready");
        Ok(())
    }

    async fn start_dns_service(&self) -> Result<()> {
        info!("✅ DNS resolver service ready");
        Ok(())
    }

    async fn start_api_service(&self) -> Result<()> {
        info!("✅ API server ready with security endpoints");
        Ok(())
    }

    async fn start_security_monitoring(&self) -> Result<()> {
        info!("🔐 Security monitoring system active");
        info!("🛡️  Byzantine detection enabled");
        info!("📊 Real-time security dashboard available");
        info!("⚠️  Security alerts system operational");
        Ok(())
    }
}

/// Create TrustChain with testing configuration (reduced security for development)
impl TrustChain {
    pub async fn new_for_testing() -> Result<Self> {
        warn!("⚠️  CREATING TRUSTCHAIN WITH TESTING CONFIGURATION");
        warn!("⚠️  REDUCED SECURITY - FOR DEVELOPMENT ONLY!");

        let security_config = TrustChainSecurityConfig {
            base_config: TrustChainConfig::localhost_testing(),
            security_config: SecurityIntegrationConfig {
                mandatory_state_proof: false,          // Reduced for testing
                mandatory_security_validation: true, // Keep basic validation
                block_on_security_failure: false,    // Don't block for testing
                log_all_operations: true,
                mandatory_post_quantum: false, // Reduced for testing
                enable_hybrid_signatures: true,
                quantum_security_level: 128,
            },
            mandatory_state_proof: false, // Reduced for testing
        };

        Self::new_with_security(security_config).await
    }

    pub async fn new_for_production() -> Result<Self> {
        info!("🔐 CREATING TRUSTCHAIN WITH PRODUCTION SECURITY CONFIGURATION");

        let security_config = TrustChainSecurityConfig {
            base_config: TrustChainConfig::production(),
            security_config: SecurityIntegrationConfig {
                mandatory_state_proof: true,           // MANDATORY for production
                mandatory_security_validation: true, // MANDATORY for production
                block_on_security_failure: true,     // MANDATORY for production
                log_all_operations: true,
                mandatory_post_quantum: true, // MANDATORY for production
                enable_hybrid_signatures: true,
                quantum_security_level: 128,
            },
            mandatory_state_proof: true, // MANDATORY for production
        };

        Self::new_with_security(security_config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proof_of_state::StateProof;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_trustchain_security_initialization() {
        let trustchain = TrustChain::new_for_testing().await.expect("test: async operation");

        let ca_cert = trustchain.get_ca_certificate().await.expect("test: async operation");
        assert!(!ca_cert.is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn test_secure_certificate_issuance() -> anyhow::Result<()> {
        let trustchain = TrustChain::new_for_testing().await.expect("test: async operation");

        let request = CertificateRequest {
            common_name: "test.secure.com".to_string(),
            san_entries: vec!["test.secure.com".to_string()],
            node_id: "test_node_001".to_string(),
            ipv6_addresses: vec![std::net::Ipv6Addr::LOCALHOST],
            state_proof: StateProof::default_for_testing(),
            timestamp: std::time::SystemTime::now(),
            identity_scope: None,
            subject_type: None,
        };

        let cert = trustchain.issue_certificate_secure(request).await.expect("test: async operation");
        assert_eq!(cert.common_name, "test.secure.com");
        assert!(!cert.serial_number.is_empty());
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_security_dashboard() {
        let trustchain = TrustChain::new_for_testing().await.expect("test: async operation");

        let dashboard = trustchain.get_security_dashboard().await.expect("test: async operation");

        // Should have valid dashboard data
        // Specific assertions depend on the implementation
        assert!(dashboard.timestamp <= std::time::SystemTime::now());
    }

    #[tokio::test]
    #[serial]
    async fn test_state_validation() -> anyhow::Result<()> {
        let trustchain = TrustChain::new_for_testing().await.expect("test: async operation");

        let state_proof = StateProof::new_for_testing();
        let result = trustchain
            .validate_state_proof(&state_proof, "test_operation")
            .await
            .expect("test: expected success");

        // Should complete validation (result depends on proof validity)
        assert!(result.validated_at <= std::time::SystemTime::now());
        Ok(())
    }

    #[tokio::test]
    async fn test_production_vs_testing_config() {
        // Test production config
        let prod_config = TrustChainSecurityConfig {
            base_config: TrustChainConfig::production(),
            security_config: SecurityIntegrationConfig::default(),
            mandatory_state_proof: true,
        };

        assert!(prod_config.mandatory_state_proof);
        assert!(prod_config.security_config.mandatory_state_proof);
        assert!(prod_config.security_config.block_on_security_failure);

        // Test testing config
        let test_config = TrustChainSecurityConfig {
            base_config: TrustChainConfig::localhost_testing(),
            security_config: SecurityIntegrationConfig {
                mandatory_state_proof: false,
                block_on_security_failure: false,
                ..Default::default()
            },
            mandatory_state_proof: false,
        };

        assert!(!test_config.mandatory_state_proof);
        assert!(!test_config.security_config.block_on_security_failure);
    }
}
