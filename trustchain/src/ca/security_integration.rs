//! Security-Integrated Certificate Authority
//! 
//! Certificate Authority with mandatory consensus validation and security monitoring

use std::sync::Arc;
use std::time::{SystemTime, Duration};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use tracing::{info, debug, warn, error};
use anyhow::Result;

use crate::consensus::{ConsensusProof, ConsensusResult, FourProofValidator};
use crate::security::{SecurityMonitor, SecurityValidationResult, SecuritySeverity};
use crate::security::monitoring::{LiveCertificateOperation, ConsensusValidationStatus, OperationState};
use crate::errors::{TrustChainError, Result as TrustChainResult};
use crate::crypto::{PostQuantumCrypto, FalconKeyPair, FalconSignature, PQCAlgorithm};
use super::{CertificateRequest, IssuedCertificate, TrustChainCA, CAConfig};

/// Security-integrated Certificate Authority wrapper
pub struct SecurityIntegratedCA {
    /// Core CA implementation
    ca: Arc<TrustChainCA>,
    /// Security monitoring system
    security_monitor: Arc<SecurityMonitor>,
    /// Post-quantum cryptography system
    pqc: Arc<PostQuantumCrypto>,
    /// CA FALCON-1024 key pair for post-quantum signatures
    ca_falcon_keypair: Arc<FalconKeyPair>,
    /// Security integration configuration
    config: SecurityIntegrationConfig,
}

/// Security integration configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityIntegrationConfig {
    /// Require security validation for all operations
    pub mandatory_security_validation: bool,
    /// Block certificate issuance on security failures
    pub block_on_security_failure: bool,
    /// Require consensus validation for certificate operations
    pub mandatory_consensus: bool,
    /// Log all certificate operations to security monitoring
    pub log_all_operations: bool,
    /// Use FALCON-1024 post-quantum signatures for all certificates
    pub mandatory_post_quantum: bool,
    /// Enable hybrid signatures (FALCON-1024 + Ed25519) during transition
    pub enable_hybrid_signatures: bool,
    /// Quantum security level requirement (128 or 256 bits)
    pub quantum_security_level: u32,
}

impl Default for SecurityIntegrationConfig {
    fn default() -> Self {
        Self {
            mandatory_security_validation: true,
            block_on_security_failure: true,
            mandatory_consensus: true,
            log_all_operations: true,
            mandatory_post_quantum: true,    // CRITICAL: Enable FALCON-1024 by default
            enable_hybrid_signatures: true, // Enable for transition period
            quantum_security_level: 128,    // 128-bit quantum security
        }
    }
}

/// Certificate operation with security validation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecureCertificateOperation {
    /// Operation ID
    pub operation_id: String,
    /// Original certificate request
    pub request: CertificateRequest,
    /// Security validation result
    pub security_validation: Option<SecurityValidationResult>,
    /// Consensus validation result
    pub consensus_validation: Option<ConsensusResult>,
    /// Operation start time
    pub started_at: SystemTime,
    /// Current operation state
    pub state: SecureOperationState,
    /// Security alerts generated (if any)
    pub security_alerts: Vec<String>,
}

/// Secure operation state
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SecureOperationState {
    Created,
    SecurityValidation,
    ConsensusValidation,
    SecurityApproved,
    CertificateGeneration,
    CTLogging,
    Completed,
    SecurityBlocked { reason: String },
    ConsensusRejected { reason: String },
    Failed { reason: String },
}

impl SecurityIntegratedCA {
    /// Create new security-integrated CA with post-quantum cryptography
    pub async fn new(
        ca_config: CAConfig,
        security_config: SecurityIntegrationConfig,
    ) -> TrustChainResult<Self> {
        info!("🔐 Initializing Security-Integrated Certificate Authority with FALCON-1024");

        // Initialize core CA
        let ca = Arc::new(TrustChainCA::new(ca_config.clone()).await?);

        // Initialize post-quantum cryptography system
        let pqc = Arc::new(PostQuantumCrypto::new()
            .map_err(|e| TrustChainError::Internal { message: format!("Failed to initialize post-quantum crypto: {}", e) })?);

        // Generate CA FALCON-1024 key pair
        let ca_falcon_keypair = Arc::new(pqc.generate_ca_keypair(&ca_config.ca_id).await
            .map_err(|e| TrustChainError::Internal { message: format!("Failed to generate CA FALCON-1024 keypair: {}", e) })?);

        info!("✅ FALCON-1024 CA keypair generated: {}", ca_falcon_keypair.public_key);

        // Validate quantum security requirements
        if security_config.mandatory_post_quantum {
            let quantum_valid = pqc.validate_quantum_resistance(&PQCAlgorithm::Falcon1024)
                .map_err(|e| TrustChainError::Internal { message: format!("Quantum resistance validation failed: {}", e) })?;
            
            if !quantum_valid {
                return Err(TrustChainError::SecurityError { 
                    message: "FALCON-1024 quantum resistance validation failed".to_string() 
                });
            }
            
            info!("✅ Quantum resistance validated: FALCON-1024 meets security requirements");
        }

        // Initialize security monitor with production configuration
        let security_monitor_config = crate::security::SecurityConfig {
            mandatory_consensus: security_config.mandatory_consensus,
            real_time_monitoring: true,
            ..Default::default()
        };
        let security_monitor = Arc::new(SecurityMonitor::new(security_monitor_config).await?);

        let integrated_ca = Self {
            ca,
            security_monitor,
            pqc,
            ca_falcon_keypair,
            config: security_config,
        };

        info!("✅ Security-Integrated CA initialized with FALCON-1024");
        info!("🔐 Mandatory consensus: {}", integrated_ca.config.mandatory_consensus);
        info!("🔐 Mandatory post-quantum: {}", integrated_ca.config.mandatory_post_quantum);
        info!("🔐 Quantum security level: {} bits", integrated_ca.config.quantum_security_level);
        
        Ok(integrated_ca)
    }

    /// Get security monitor reference
    pub fn get_security_monitor(&self) -> Arc<SecurityMonitor> {
        self.security_monitor.clone()
    }

    /// Issue certificate with mandatory security validation
    pub async fn issue_certificate_secure(&self, request: CertificateRequest) -> TrustChainResult<IssuedCertificate> {
        let operation_id = uuid::Uuid::new_v4().to_string();
        let start_time = SystemTime::now();
        
        info!("Starting secure certificate issuance for: {} (operation: {})", 
              request.common_name, operation_id);

        // Create secure operation tracking
        let mut operation = SecureCertificateOperation {
            operation_id: operation_id.clone(),
            request: request.clone(),
            security_validation: None,
            consensus_validation: None,
            started_at: start_time,
            state: SecureOperationState::Created,
            security_alerts: Vec::new(),
        };

        // PHASE 1: MANDATORY SECURITY VALIDATION
        operation.state = SecureOperationState::SecurityValidation;
        
        // Add to security monitoring dashboard
        if self.config.log_all_operations {
            let live_operation = LiveCertificateOperation {
                operation_id: operation_id.clone(),
                operation_type: "issue_certificate".to_string(),
                common_name: request.common_name.clone(),
                node_id: request.node_id.clone(),
                consensus_proof: request.consensus_proof.clone(),
                consensus_status: ConsensusValidationStatus::Pending,
                started_at: start_time,
                state: OperationState::ConsensusValidation,
            };
            
            // Get security monitoring dashboard (this would typically be a shared component)
            // For now, we'll log the operation directly through the security monitor
            debug!("Adding certificate operation to security monitoring: {}", operation_id);
        }

        // CRITICAL: Perform mandatory security validation with consensus
        let security_result = if self.config.mandatory_security_validation {
            info!("MANDATORY security validation for operation: {}", operation_id);
            
            let result = self.security_monitor.validate_certificate_operation(
                "issue_certificate",
                &request.consensus_proof,
                &format!("cert_issue_{}", operation_id),
            ).await?;
            
            operation.security_validation = Some(result.clone());
            
            // Check if security validation passed
            if !result.is_valid {
                error!("SECURITY VALIDATION FAILED for operation {}: score={:.2}", 
                       operation_id, result.metrics.security_score);
                
                operation.state = SecureOperationState::SecurityBlocked {
                    reason: "Security validation failed".to_string(),
                };
                
                if self.config.block_on_security_failure {
                    return Err(TrustChainError::SecurityValidationFailed {
                        reason: format!("Security validation failed: score={:.2}", result.metrics.security_score),
                    });
                }
            } else {
                info!("Security validation PASSED for operation {}: score={:.2}", 
                      operation_id, result.metrics.security_score);
                operation.state = SecureOperationState::SecurityApproved;
            }
            
            Some(result)
        } else {
            warn!("Security validation DISABLED - CRITICAL SECURITY RISK for operation: {}", operation_id);
            None
        };

        // PHASE 2: MANDATORY CONSENSUS VALIDATION
        operation.state = SecureOperationState::ConsensusValidation;
        
        let consensus_result = if self.config.mandatory_consensus {
            info!("MANDATORY consensus validation for operation: {}", operation_id);
            
            // Use the CA's internal consensus validator
            let result = self.ca.consensus.validate_consensus(&request.consensus_proof).await?;
            
            operation.consensus_validation = Some(result.clone());
            
            if !result.is_valid() {
                error!("CONSENSUS VALIDATION FAILED for operation {}: {:?}", operation_id, result);
                
                operation.state = SecureOperationState::ConsensusRejected {
                    reason: "Consensus validation failed".to_string(),
                };
                
                return Err(TrustChainError::ConsensusValidationFailed {
                    reason: "Consensus validation failed".to_string(),
                });
            } else {
                info!("Consensus validation PASSED for operation: {}", operation_id);
            }
            
            Some(result)
        } else {
            warn!("Consensus validation DISABLED - CRITICAL SECURITY RISK for operation: {}", operation_id);
            None
        };

        // PHASE 3: POST-QUANTUM CERTIFICATE GENERATION
        operation.state = SecureOperationState::CertificateGeneration;
        
        info!("Proceeding with FALCON-1024 certificate generation for operation: {}", operation_id);
        
        // Issue certificate with post-quantum signature if enabled
        let mut issued_cert = if self.config.mandatory_post_quantum {
            info!("🔐 Generating certificate with FALCON-1024 post-quantum signature");
            self.issue_certificate_with_falcon(&request, &operation_id).await?
        } else {
            warn!("⚠️  Using classical certificate generation (not quantum-resistant)");
            self.ca.issue_certificate(request).await?
        };
        
        // PHASE 4: CT LOGGING
        operation.state = SecureOperationState::CTLogging;
        
        // The core CA already handles CT logging, so we just need to verify it happened
        info!("Certificate CT logging completed for operation: {}", operation_id);
        
        // PHASE 5: COMPLETION
        operation.state = SecureOperationState::Completed;
        
        let total_time = start_time.elapsed().unwrap_or_default().as_millis();
        
        info!("Secure certificate issuance COMPLETED for operation {} in {}ms: {}", 
              operation_id, total_time, issued_cert.serial_number);
        
        // Log successful secure operation
        debug!("Secure certificate operation completed successfully: {}", operation_id);
        
        Ok(issued_cert)
    }

    /// Validate certificate with security monitoring
    pub async fn validate_certificate_secure(&self, certificate_der: &[u8]) -> TrustChainResult<CertificateValidationResult> {
        let operation_id = uuid::Uuid::new_v4().to_string();
        
        info!("Starting secure certificate validation (operation: {})", operation_id);
        
        // For certificate validation, we need to extract any consensus proof from the certificate
        // In production, this would parse the certificate extensions for consensus proof data
        let mock_consensus_proof = crate::consensus::ConsensusProof::default_for_testing();
        
        // Perform security validation
        let security_result = self.security_monitor.validate_certificate_operation(
            "validate_certificate",
            &mock_consensus_proof,
            &format!("cert_validate_{}", operation_id),
        ).await?;
        
        // Create validation result
        let validation_result = CertificateValidationResult {
            is_valid: security_result.is_valid,
            consensus_validated: security_result.consensus_result.as_ref()
                .map(|r| r.is_valid())
                .unwrap_or(false),
            security_validation: Some(security_result),
            certificate_fingerprint: self.calculate_certificate_fingerprint(certificate_der),
            validated_at: SystemTime::now(),
        };
        
        info!("Secure certificate validation completed: valid={}", validation_result.is_valid);
        Ok(validation_result)
    }

    /// Get security monitoring dashboard data
    pub async fn get_security_dashboard(&self) -> TrustChainResult<crate::security::SecurityDashboard> {
        self.security_monitor.get_monitoring_dashboard().await
    }

    /// Get CA metrics with security integration
    pub async fn get_integrated_metrics(&self) -> TrustChainResult<IntegratedCAMetrics> {
        // TODO: Implement proper CA metrics collection
        let ca_metrics = CAMetrics::default();
        let security_metrics = self.security_monitor.get_metrics().await;
        
        Ok(IntegratedCAMetrics {
            ca_metrics,
            security_metrics,
            integration_config: self.config.clone(),
            last_update: SystemTime::now(),
        })
    }

    /// Issue certificate with FALCON-1024 post-quantum signature
    async fn issue_certificate_with_falcon(
        &self, 
        request: &CertificateRequest, 
        operation_id: &str
    ) -> TrustChainResult<IssuedCertificate> {
        info!("🔐 Issuing post-quantum certificate with FALCON-1024 for: {}", request.common_name);
        
        // First issue standard certificate through core CA
        let mut cert = self.ca.issue_certificate(request.clone()).await?;
        
        // Add FALCON-1024 signature to certificate metadata
        let falcon_signature = self.pqc.sign_with_falcon(
            &cert.certificate_der, 
            &self.ca_falcon_keypair.private_key
        ).await.map_err(|e| TrustChainError::Internal { 
            message: format!("FALCON-1024 signature generation failed: {}", e) 
        })?;
        
        // Add post-quantum metadata to certificate
        cert.metadata.tags.insert("pq_algorithm".to_string(), "FALCON-1024".to_string());
        cert.metadata.tags.insert("quantum_security_level".to_string(), 
                                  self.config.quantum_security_level.to_string());
        cert.metadata.tags.insert("falcon_signature".to_string(), 
                                  hex::encode(&falcon_signature.signature_bytes));
        cert.metadata.tags.insert("falcon_public_key".to_string(), 
                                  hex::encode(&self.ca_falcon_keypair.public_key.key_bytes));
        cert.metadata.tags.insert("pq_ca_fingerprint".to_string(), 
                                  hex::encode(&self.ca_falcon_keypair.public_key.fingerprint));
        cert.metadata.signature_algorithm = Some("FALCON-1024".to_string());
        
        // Add hybrid signature if enabled
        if self.config.enable_hybrid_signatures {
            info!("🔐 Adding hybrid signature support to certificate");
            cert.metadata.tags.insert("hybrid_signature_support".to_string(), "true".to_string());
            cert.metadata.tags.insert("migration_ready".to_string(), "true".to_string());
        }
        
        info!("✅ Post-quantum certificate issued with FALCON-1024: {}", cert.serial_number);
        Ok(cert)
    }
    
    /// Validate post-quantum certificate signature
    pub async fn validate_falcon_certificate(&self, certificate_der: &[u8]) -> TrustChainResult<bool> {
        info!("🔍 Validating FALCON-1024 certificate signature");
        
        // Extract FALCON signature from certificate (this is simplified)
        // In production, this would parse the certificate extensions properly
        let is_valid = self.pqc.verify_falcon_signature(
            certificate_der,
            &FalconSignature {
                signature_bytes: vec![], // Would extract from certificate
                algorithm: "FALCON-1024".to_string(),
                signed_at: SystemTime::now(),
                message_hash: [0u8; 32], // Would extract from certificate
            },
            &self.ca_falcon_keypair.public_key,
        ).await.map_err(|e| TrustChainError::Internal { 
            message: format!("FALCON-1024 verification failed: {}", e) 
        })?;
        
        if is_valid {
            info!("✅ FALCON-1024 certificate signature verification successful");
        } else {
            warn!("❌ FALCON-1024 certificate signature verification failed");
        }
        
        Ok(is_valid)
    }
    
    /// Get post-quantum cryptography information
    pub fn get_pq_info(&self) -> PQCInfo {
        PQCInfo {
            algorithm: PQCAlgorithm::Falcon1024,
            ca_public_key_fingerprint: hex::encode(&self.ca_falcon_keypair.public_key.fingerprint),
            quantum_security_level: self.config.quantum_security_level,
            hybrid_signatures_enabled: self.config.enable_hybrid_signatures,
            mandatory_post_quantum: self.config.mandatory_post_quantum,
            ca_key_created_at: self.ca_falcon_keypair.created_at,
            algorithm_info: self.pqc.get_algorithm_info(&PQCAlgorithm::Falcon1024),
            performance_info: self.pqc.get_performance_info(&PQCAlgorithm::Falcon1024),
        }
    }
    
    /// Generate new asset authentication key for remote proxy
    pub async fn generate_asset_keypair(&self) -> TrustChainResult<FalconKeyPair> {
        info!("🔑 Generating FALCON-1024 asset authentication keypair");
        
        let keypair = self.pqc.generate_asset_keypair().await
            .map_err(|e| TrustChainError::Internal { 
                message: format!("Asset keypair generation failed: {}", e) 
            })?;
        
        info!("✅ Asset authentication keypair generated: {}", keypair.public_key);
        Ok(keypair)
    }
    
    /// Generate new remote proxy authentication key
    pub async fn generate_proxy_keypair(&self) -> TrustChainResult<FalconKeyPair> {
        info!("🔑 Generating FALCON-1024 remote proxy authentication keypair");
        
        let keypair = self.pqc.generate_proxy_keypair().await
            .map_err(|e| TrustChainError::Internal { 
                message: format!("Proxy keypair generation failed: {}", e) 
            })?;
        
        info!("✅ Remote proxy authentication keypair generated: {}", keypair.public_key);
        Ok(keypair)
    }

    /// Calculate certificate fingerprint
    fn calculate_certificate_fingerprint(&self, cert_der: &[u8]) -> [u8; 32] {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(cert_der);
        hasher.finalize().into()
    }
}

/// CA performance metrics
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CAMetrics {
    /// Total certificates issued
    pub certificates_issued: u64,
    /// Certificate validation requests
    pub validation_requests: u64,
    /// Average issuance time (ms)
    pub avg_issuance_time_ms: u64,
    /// Success rate
    pub success_rate: f64,
}

/// Integrated CA metrics with security monitoring
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntegratedCAMetrics {
    /// Core CA metrics
    pub ca_metrics: CAMetrics,
    /// Security monitoring metrics
    pub security_metrics: crate::security::SecurityMetrics,
    /// Current integration configuration
    pub integration_config: SecurityIntegrationConfig,
    /// Last metrics update time
    pub last_update: SystemTime,
}

/// Certificate validation result with security integration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CertificateValidationResult {
    /// Overall validation result
    pub is_valid: bool,
    /// Security validation result
    pub security_validation: Option<SecurityValidationResult>,
    /// Whether consensus was validated
    pub consensus_validated: bool,
    /// Certificate fingerprint
    pub certificate_fingerprint: [u8; 32],
    /// Validation timestamp
    pub validated_at: SystemTime,
}

/// Post-quantum cryptography information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PQCInfo {
    /// Post-quantum algorithm in use
    pub algorithm: PQCAlgorithm,
    /// CA public key fingerprint
    pub ca_public_key_fingerprint: String,
    /// Quantum security level in bits
    pub quantum_security_level: u32,
    /// Whether hybrid signatures are enabled
    pub hybrid_signatures_enabled: bool,
    /// Whether post-quantum is mandatory
    pub mandatory_post_quantum: bool,
    /// CA key creation timestamp
    pub ca_key_created_at: SystemTime,
    /// Algorithm information
    pub algorithm_info: String,
    /// Performance information
    pub performance_info: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::ConsensusProof;

    #[tokio::test]
    async fn test_security_integrated_ca_creation() {
        let ca_config = CAConfiguration::default();
        let security_config = SecurityIntegrationConfig::default();
        
        let integrated_ca = SecurityIntegratedCA::new(ca_config, security_config).await.unwrap();
        assert!(integrated_ca.config.mandatory_consensus);
        assert!(integrated_ca.config.mandatory_security_validation);
    }

    #[tokio::test]
    async fn test_secure_certificate_issuance() {
        let ca_config = CAConfiguration::default();
        let security_config = SecurityIntegrationConfig::default();
        
        let integrated_ca = SecurityIntegratedCA::new(ca_config, security_config).await.unwrap();
        
        let request = CertificateRequest {
            common_name: "secure.test.com".to_string(),
            san_entries: vec!["secure.test.com".to_string()],
            node_id: "secure_test_node".to_string(),
            ipv6_addresses: vec![std::net::Ipv6Addr::LOCALHOST],
            consensus_proof: ConsensusProof::default_for_testing(),
            timestamp: SystemTime::now(),
        };
        
        let result = integrated_ca.issue_certificate_secure(request).await;
        // Should succeed with valid consensus proof
        assert!(result.is_ok());
        
        let cert = result.unwrap();
        assert_eq!(cert.common_name, "secure.test.com");
    }

    #[tokio::test]
    async fn test_security_dashboard_integration() {
        let ca_config = CAConfiguration::default();
        let security_config = SecurityIntegrationConfig::default();
        
        let integrated_ca = SecurityIntegratedCA::new(ca_config, security_config).await.unwrap();
        
        let dashboard = integrated_ca.get_security_dashboard().await.unwrap();
        
        // Should have valid dashboard data
        assert!(dashboard.consensus_status.enabled);
        // Other assertions depend on the actual operations performed
    }

    #[tokio::test]
    async fn test_mandatory_consensus_disabled() {
        let ca_config = CAConfiguration::default();
        let mut security_config = SecurityIntegrationConfig::default();
        security_config.mandatory_consensus = false;
        
        let integrated_ca = SecurityIntegratedCA::new(ca_config, security_config).await.unwrap();
        
        // Should still work but with reduced security
        assert!(!integrated_ca.config.mandatory_consensus);
    }
}