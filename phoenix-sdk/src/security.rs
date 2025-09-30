//! Phoenix Security Management
//!
//! Handles certificate management, authentication, and encryption.

use std::sync::Arc;
use std::time::SystemTime;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use trustchain::{
    TrustChain,
    CertificateRequest,
    IssuedCertificate,
    ConsensusProof,
};

use crate::{
    config::SecurityLevel,
    errors::{PhoenixError, Result},
};

/// Security manager for Phoenix SDK
pub struct SecurityManager {
    trustchain: Arc<TrustChain>,
    security_level: SecurityLevel,
    state: Arc<RwLock<SecurityState>>,
}

#[derive(Debug, Default)]
struct SecurityState {
    certificates: Vec<CertificateInfo>,
    trusted_peers: Vec<String>,
    blocked_peers: Vec<String>,
}

/// Certificate information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateInfo {
    /// Certificate common name
    pub common_name: String,
    /// Certificate serial number
    pub serial_number: String,
    /// Certificate fingerprint
    pub fingerprint: String,
    /// Issuer information
    pub issuer: String,
    /// Valid from timestamp
    pub valid_from: SystemTime,
    /// Valid until timestamp
    pub valid_until: SystemTime,
    /// Is post-quantum secure
    pub is_post_quantum: bool,
}

/// Security context for connections
#[derive(Debug, Clone)]
pub struct SecurityContext {
    /// Security level
    pub security_level: SecurityLevel,
    /// Local certificate
    pub local_cert: Option<CertificateInfo>,
    /// Remote certificate
    pub remote_cert: Option<CertificateInfo>,
    /// Is connection encrypted
    pub is_encrypted: bool,
    /// Is using mutual TLS
    pub is_mutual_tls: bool,
    /// Is using post-quantum crypto
    pub is_post_quantum: bool,
}

impl SecurityManager {
    /// Create new security manager
    pub fn new(trustchain: Arc<TrustChain>, security_level: SecurityLevel) -> Self {
        Self {
            trustchain,
            security_level,
            state: Arc::new(RwLock::new(SecurityState::default())),
        }
    }

    /// Request certificate from TrustChain
    pub async fn request_certificate(
        &self,
        common_name: &str,
        node_id: &str,
    ) -> Result<IssuedCertificate> {
        // Skip certificate request for development mode
        if matches!(self.security_level, SecurityLevel::Development) {
            return Ok(self.create_dev_certificate(common_name));
        }

        let request = CertificateRequest {
            common_name: common_name.to_string(),
            san_entries: vec![common_name.to_string()],
            node_id: node_id.to_string(),
            ipv6_addresses: vec![std::net::Ipv6Addr::LOCALHOST],
            consensus_proof: ConsensusProof::default_for_testing(),
            timestamp: SystemTime::now(),
        };

        self.trustchain.issue_certificate_secure(request).await
            .map_err(|e| PhoenixError::SecurityError(e.to_string()))
    }

    /// Validate certificate
    pub async fn validate_certificate(&self, cert_der: &[u8]) -> Result<bool> {
        // Skip validation for development mode
        if matches!(self.security_level, SecurityLevel::Development) {
            return Ok(true);
        }

        self.trustchain.validate_certificate_secure(cert_der).await
            .map_err(|e| PhoenixError::SecurityError(e.to_string()))
    }

    /// Get security context for connection
    pub fn get_security_context(&self) -> SecurityContext {
        SecurityContext {
            security_level: self.security_level.clone(),
            local_cert: self.get_local_certificate(),
            remote_cert: None,
            is_encrypted: self.security_level.requires_cert_validation(),
            is_mutual_tls: self.security_level.requires_mutual_tls(),
            is_post_quantum: self.security_level.requires_post_quantum(),
        }
    }

    /// Add trusted peer
    pub fn add_trusted_peer(&self, peer_id: String) {
        let mut state = self.state.write();
        if !state.trusted_peers.contains(&peer_id) {
            state.trusted_peers.push(peer_id);
        }
    }

    /// Remove trusted peer
    pub fn remove_trusted_peer(&self, peer_id: &str) {
        let mut state = self.state.write();
        state.trusted_peers.retain(|p| p != peer_id);
    }

    /// Block peer
    pub fn block_peer(&self, peer_id: String) {
        let mut state = self.state.write();
        if !state.blocked_peers.contains(&peer_id) {
            state.blocked_peers.push(peer_id.clone());
        }
        // Remove from trusted if present
        state.trusted_peers.retain(|p| p != &peer_id);
    }

    /// Check if peer is trusted
    pub fn is_trusted(&self, peer_id: &str) -> bool {
        let state = self.state.read();
        state.trusted_peers.contains(&peer_id.to_string())
    }

    /// Check if peer is blocked
    pub fn is_blocked(&self, peer_id: &str) -> bool {
        let state = self.state.read();
        state.blocked_peers.contains(&peer_id.to_string())
    }

    /// Get local certificate
    fn get_local_certificate(&self) -> Option<CertificateInfo> {
        let state = self.state.read();
        state.certificates.first().cloned()
    }

    /// Create development certificate (self-signed)
    fn create_dev_certificate(&self, common_name: &str) -> IssuedCertificate {
        IssuedCertificate {
            common_name: common_name.to_string(),
            serial_number: uuid::Uuid::new_v4().to_string(),
            certificate_der: Vec::new(), // Empty for dev
            private_key_der: Vec::new(), // Empty for dev
            valid_from: SystemTime::now(),
            valid_until: SystemTime::now() + std::time::Duration::from_secs(365 * 24 * 60 * 60),
            fingerprint: "dev-fingerprint".to_string(),
        }
    }

    /// Store certificate
    pub fn store_certificate(&self, cert: CertificateInfo) {
        let mut state = self.state.write();
        state.certificates.push(cert);
    }

    /// Get security metrics
    pub fn get_metrics(&self) -> SecurityMetrics {
        let state = self.state.read();
        SecurityMetrics {
            total_certificates: state.certificates.len(),
            trusted_peers: state.trusted_peers.len(),
            blocked_peers: state.blocked_peers.len(),
            security_level: self.security_level.to_string(),
        }
    }
}

/// Security metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMetrics {
    /// Total certificates managed
    pub total_certificates: usize,
    /// Number of trusted peers
    pub trusted_peers: usize,
    /// Number of blocked peers
    pub blocked_peers: usize,
    /// Current security level
    pub security_level: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_security_context() {
        let trustchain = Arc::new(TrustChain::new_for_testing().await.unwrap());
        let manager = SecurityManager::new(trustchain, SecurityLevel::Standard);

        let context = manager.get_security_context();
        assert!(matches!(context.security_level, SecurityLevel::Standard));
        assert!(context.is_encrypted);
        assert!(!context.is_mutual_tls);
        assert!(!context.is_post_quantum);
    }

    #[test]
    fn test_peer_management() {
        use std::sync::Arc;

        // This test doesn't need actual TrustChain functionality
        let trustchain = unsafe {
            Arc::from_raw(std::ptr::null::<TrustChain>())
        };

        let manager = SecurityManager::new(trustchain.clone(), SecurityLevel::Development);

        manager.add_trusted_peer("peer1".to_string());
        assert!(manager.is_trusted("peer1"));
        assert!(!manager.is_blocked("peer1"));

        manager.block_peer("peer1".to_string());
        assert!(!manager.is_trusted("peer1"));
        assert!(manager.is_blocked("peer1"));

        // Prevent Arc from trying to deallocate null pointer
        std::mem::forget(trustchain);
    }
}