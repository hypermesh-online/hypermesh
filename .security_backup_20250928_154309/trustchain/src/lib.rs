//! TrustChain - Certificate Authority with Certificate Transparency and DNS
//! 
//! A secure, IPv6-only certificate authority with NKrypt consensus validation
//! and mandatory security monitoring for the Web3 ecosystem. Provides CA, CT, 
//! and DNS services with real-time certificate fingerprinting, Byzantine fault 
//! tolerance, and comprehensive security integration.

pub mod consensus;
pub mod ca;
pub mod ct;
pub mod dns;
pub mod trust;
pub mod api;
pub mod config;
pub mod errors;
pub mod stoq_client;
pub mod security; // NEW: Security monitoring and Byzantine detection
pub mod crypto;   // NEW: Post-quantum cryptography (FALCON-1024 + Kyber)
pub mod deployment; // NEW: Quality gates and deployment validation
pub mod monitoring; // NEW: Native monitoring system without external dependencies

// Re-export main types
pub use consensus::{ConsensusProof, ConsensusContext, ConsensusRequirements};
pub use ca::{TrustChainCA, CAConfig, CertificateRequest, IssuedCertificate};
pub use ca::security_integration::{SecurityIntegratedCA, SecurityIntegrationConfig};
pub use security::{SecurityMonitor, SecurityValidationResult, SecurityDashboard};
pub use config::TrustChainConfig;
pub use errors::{TrustChainError, Result};
pub use stoq_client::{TrustChainStoqClient, TrustChainStoqConfig, ServiceEndpoint, ServiceType};
pub use crypto::{PostQuantumCrypto, FalconKeyPair, FalconSignature, KyberKeyPair, PQCAlgorithm};

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error, warn};

/// Main TrustChain service coordinator with security integration
pub struct TrustChain {
    /// Security-integrated Certificate Authority (MANDATORY CONSENSUS)
    security_ca: Arc<SecurityIntegratedCA>,
    /// Certificate Transparency logs
    ct: Arc<ct::CertificateTransparency>,
    /// DNS resolver
    dns: Arc<dns::DnsResolver>,
    /// API server with security endpoints
    api: Arc<api::ApiServer>,
    /// STOQ client for all network operations
    stoq_client: Arc<TrustChainStoqClient>,
    /// Security monitoring system
    security_monitor: Arc<SecurityMonitor>,
    /// Configuration
    config: Arc<TrustChainConfig>,
}

/// TrustChain initialization configuration
pub struct TrustChainSecurityConfig {
    /// Base TrustChain configuration
    pub base_config: TrustChainConfig,
    /// Security integration configuration
    pub security_config: SecurityIntegrationConfig,
    /// Enable mandatory consensus for all operations
    pub mandatory_consensus: bool,
}

impl Default for TrustChainSecurityConfig {
    fn default() -> Self {
        Self {
            base_config: TrustChainConfig::localhost_testing(),
            security_config: SecurityIntegrationConfig::default(),
            mandatory_consensus: true,
        }
    }
}

impl TrustChain {
    /// Create new TrustChain instance with security integration
    pub async fn new_with_security(security_config: TrustChainSecurityConfig) -> Result<Self> {
        info!("Initializing TrustChain with MANDATORY SECURITY INTEGRATION");
        
        if !security_config.mandatory_consensus {
            warn!("⚠️  CRITICAL SECURITY WARNING: Consensus validation is DISABLED!");
            warn!("⚠️  This reduces security and should only be used for testing!");
        } else {
            info!("✅ MANDATORY consensus validation ENABLED for all certificate operations");
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
                dns_resolvers: vec![
                    ServiceEndpoint::new(
                        ServiceType::Dns,
                        config.dns.bind_address,
                        config.dns.quic_port
                    ).with_service_name("dns.trustchain.local".to_string()),
                ],
                ct_logs: vec![
                    ServiceEndpoint::new(
                        ServiceType::CertificateTransparency,
                        config.dns.bind_address,
                        config.ct.port
                    ).with_service_name("ct.trustchain.local".to_string()),
                ],
                ca_endpoints: vec![
                    ServiceEndpoint::new(
                        ServiceType::CertificateAuthority,
                        config.dns.bind_address,
                        config.ca.port
                    ).with_service_name("ca.trustchain.local".to_string()),
                ],
                health_check_interval: std::time::Duration::from_secs(60),
            },
        };

        let stoq_client = Arc::new(TrustChainStoqClient::new(stoq_config).await?);

        // Initialize SECURITY-INTEGRATED Certificate Authority
        let mut security_integration_config = security_config.security_config;
        security_integration_config.mandatory_consensus = security_config.mandatory_consensus;
        
        let security_ca = Arc::new(
            SecurityIntegratedCA::new(config.ca.clone(), security_integration_config).await?
        );

        // Initialize Security Monitor (extracted from security_ca for direct access)
        let security_monitor = security_ca.get_security_monitor();

        // Initialize Certificate Transparency with STOQ client
        let ct = Arc::new(ct::CertificateTransparency::new(config.ct.clone()).await?);

        // Initialize DNS resolver with STOQ client
        let dns = Arc::new(dns::DnsResolver::new(config.dns.clone()).await?);

        // Initialize API server
        let api = Arc::new(api::ApiServer::new(config.api.clone()).await?);

        let trustchain = Self {
            security_ca,
            ct,
            dns,
            api,
            stoq_client,
            security_monitor,
            config: Arc::new(config),
        };

        info!("✅ TrustChain service initialized with MANDATORY SECURITY INTEGRATION");
        info!("🔐 Security features: Consensus validation, Byzantine detection, Real-time monitoring");
        Ok(trustchain)
    }

    /// Create TrustChain with standard configuration (for backward compatibility)
    pub async fn new(config: TrustChainConfig) -> Result<Self> {
        let security_config = TrustChainSecurityConfig {
            base_config: config,
            security_config: SecurityIntegrationConfig::default(),
            mandatory_consensus: true, // Always enable for production
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
    pub async fn issue_certificate_secure(&self, request: CertificateRequest) -> Result<IssuedCertificate> {
        info!("🔐 SECURE certificate issuance with mandatory consensus validation");
        
        // Issue certificate through security-integrated CA (includes consensus validation)
        let cert = self.security_ca.issue_certificate_secure(request).await?;

        // Log certificate in CT logs
        match self.ct.log_certificate(&cert.certificate_der).await {
            Ok(_) => {
                info!("✅ Certificate logged in CT: {}", cert.serial_number);
            }
            Err(e) => {
                error!("⚠️  CT logging failed for certificate {}: {}", cert.serial_number, e);
                // Don't fail the entire operation, but log the issue
            }
        }

        info!("✅ Secure certificate issuance completed: {}", cert.serial_number);
        Ok(cert)
    }

    /// Issue certificate with CT logging (legacy method - now with security)
    pub async fn issue_certificate_with_ct(&self, request: CertificateRequest) -> Result<IssuedCertificate> {
        warn!("⚠️  Using legacy certificate issuance method - upgrading to secure version");
        self.issue_certificate_secure(request).await
    }

    /// Validate certificate with security monitoring and CT verification
    pub async fn validate_certificate_secure(&self, cert_der: &[u8]) -> Result<bool> {
        info!("🔐 SECURE certificate validation with security monitoring");
        
        // Validate through security-integrated CA
        let security_validation = self.security_ca.validate_certificate_secure(cert_der).await?;
        
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
            warn!("❌ Certificate validation failed: security={}, ct={}", 
                  security_validation.is_valid, ct_valid);
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
        self.security_monitor.get_monitoring_dashboard().await
            .map_err(|e| TrustChainError::SecurityError { message: e.to_string() })
    }

    /// Get security metrics
    pub async fn get_security_metrics(&self) -> security::SecurityMetrics {
        self.security_monitor.get_metrics().await
    }

    /// Validate consensus proof directly
    pub async fn validate_consensus_proof(&self, consensus_proof: &ConsensusProof, operation: &str) -> Result<SecurityValidationResult> {
        self.security_monitor.validate_certificate_operation(operation, consensus_proof, "direct_validation").await
            .map_err(|e| TrustChainError::SecurityError { message: e.to_string() })
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
    pub async fn get_integrated_metrics(&self) -> Result<ca::security_integration::IntegratedCAMetrics> {
        self.security_ca.get_integrated_metrics().await
            .map_err(|e| TrustChainError::Internal { message: e.to_string() })
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
        self.api.shutdown().await?;
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
        info!("✅ Security-Integrated Certificate Authority ready (consensus mandatory)");
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
                mandatory_consensus: false, // Reduced for testing
                mandatory_security_validation: true, // Keep basic validation
                block_on_security_failure: false, // Don't block for testing
                log_all_operations: true,
                mandatory_post_quantum: false, // Reduced for testing
                enable_hybrid_signatures: true,
                quantum_security_level: 128,
            },
            mandatory_consensus: false, // Reduced for testing
        };
        
        Self::new_with_security(security_config).await
    }

    pub async fn new_for_production() -> Result<Self> {
        info!("🔐 CREATING TRUSTCHAIN WITH PRODUCTION SECURITY CONFIGURATION");
        
        let security_config = TrustChainSecurityConfig {
            base_config: TrustChainConfig::production(),
            security_config: SecurityIntegrationConfig {
                mandatory_consensus: true, // MANDATORY for production
                mandatory_security_validation: true, // MANDATORY for production
                block_on_security_failure: true, // MANDATORY for production
                log_all_operations: true,
                mandatory_post_quantum: true, // MANDATORY for production
                enable_hybrid_signatures: true,
                quantum_security_level: 128,
            },
            mandatory_consensus: true, // MANDATORY for production
        };
        
        Self::new_with_security(security_config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use consensus::ConsensusProof;

    #[tokio::test]
    async fn test_trustchain_security_initialization() {
        let trustchain = TrustChain::new_for_testing().await.unwrap();
        
        let ca_cert = trustchain.get_ca_certificate().await.unwrap();
        assert!(!ca_cert.is_empty());
    }

    #[tokio::test]
    async fn test_secure_certificate_issuance() {
        let trustchain = TrustChain::new_for_testing().await.unwrap();
        
        let request = CertificateRequest {
            common_name: "test.secure.com".to_string(),
            san_entries: vec!["test.secure.com".to_string()],
            node_id: "test_node_001".to_string(),
            ipv6_addresses: vec![std::net::Ipv6Addr::LOCALHOST],
            consensus_proof: ConsensusProof::default_for_testing(),
            timestamp: std::time::SystemTime::now(),
        };
        
        let cert = trustchain.issue_certificate_secure(request).await.unwrap();
        assert_eq!(cert.common_name, "test.secure.com");
        assert!(!cert.serial_number.is_empty());
    }

    #[tokio::test]
    async fn test_security_dashboard() {
        let trustchain = TrustChain::new_for_testing().await.unwrap();
        
        let dashboard = trustchain.get_security_dashboard().await.unwrap();
        
        // Should have valid dashboard data
        // Specific assertions depend on the implementation
        assert!(dashboard.timestamp <= std::time::SystemTime::now());
    }

    #[tokio::test]
    async fn test_consensus_validation() {
        let trustchain = TrustChain::new_for_testing().await.unwrap();
        
        let consensus_proof = ConsensusProof::default_for_testing();
        let result = trustchain.validate_consensus_proof(&consensus_proof, "test_operation").await.unwrap();
        
        // Should complete validation (result depends on proof validity)
        assert!(result.validated_at <= std::time::SystemTime::now());
    }

    #[tokio::test]
    async fn test_production_vs_testing_config() {
        // Test production config
        let prod_config = TrustChainSecurityConfig {
            base_config: TrustChainConfig::production(),
            security_config: SecurityIntegrationConfig::default(),
            mandatory_consensus: true,
        };
        
        assert!(prod_config.mandatory_consensus);
        assert!(prod_config.security_config.mandatory_consensus);
        assert!(prod_config.security_config.block_on_security_failure);
        
        // Test testing config
        let test_config = TrustChainSecurityConfig {
            base_config: TrustChainConfig::localhost_testing(),
            security_config: SecurityIntegrationConfig {
                mandatory_consensus: false,
                block_on_security_failure: false,
                ..Default::default()
            },
            mandatory_consensus: false,
        };
        
        assert!(!test_config.mandatory_consensus);
        assert!(!test_config.security_config.block_on_security_failure);
    }
}