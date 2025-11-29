//! Node Authentication for HyperMesh Transport

use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use tokio::time::{interval, Instant, Duration};
use tracing::{debug, warn, error};
use sha2::{Sha256, Digest};
// X509 parsing removed for MVP simplicity

use super::config::AuthenticationConfig;
use super::error::{TransportError, Result};

/// Certificate validation result
#[derive(Debug, Clone)]
pub struct CertificateValidation {
    pub is_valid: bool,
    pub node_id: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub fingerprint: String,
    pub validation_errors: Vec<String>,
}

/// Cached certificate information
#[derive(Debug, Clone)]
struct CachedCertInfo {
    node_id: String,
    fingerprint: String,
    expires_at: chrono::DateTime<chrono::Utc>,
    cached_at: Instant,
    validation_result: bool,
}

/// Node authenticator for certificate-based authentication
pub struct NodeAuthenticator {
    config: AuthenticationConfig,
    cert_cache: Arc<RwLock<HashMap<String, CachedCertInfo>>>,
    trust_store: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

/// Auth manager (alias for NodeAuthenticator for compatibility)
pub type AuthManager = NodeAuthenticator;

impl NodeAuthenticator {
    /// Create a new node authenticator
    pub fn new(config: AuthenticationConfig) -> Self {
        let cert_cache = Arc::new(RwLock::new(HashMap::new()));
        let trust_store = Arc::new(RwLock::new(HashMap::new()));

        Self {
            config,
            cert_cache,
            trust_store,
        }
    }

    /// Authenticate a node by ID
    pub async fn authenticate_node(&self, node_id: &str) -> Result<bool> {
        debug!("Authenticating node {}", node_id);

        if !self.config.enable_cert_validation {
            debug!("Certificate validation disabled, auto-accepting");
            return Ok(true);
        }

        // Check cache first
        {
            let cache = self.cert_cache.read();
            if let Some(cached) = cache.get(node_id) {
                if cached.cached_at.elapsed() < Duration::from_secs(300) {
                    debug!("Using cached validation result for node {}", node_id);
                    return Ok(cached.validation_result);
                }
            }
        }

        // For MVP, perform basic node ID validation
        let is_valid = self.validate_node_id(node_id);

        // Cache the result
        {
            let mut cache = self.cert_cache.write();
            cache.insert(node_id.to_string(), CachedCertInfo {
                node_id: node_id.to_string(),
                fingerprint: self.generate_fingerprint(node_id),
                expires_at: chrono::Utc::now() + chrono::Duration::hours(24),
                cached_at: Instant::now(),
                validation_result: is_valid,
            });
        }

        Ok(is_valid)
    }

    /// Validate a node ID format
    fn validate_node_id(&self, node_id: &str) -> bool {
        // Basic validation: node ID should be non-empty and follow a pattern
        !node_id.is_empty() && (
            node_id.starts_with("node-") ||
            node_id.starts_with("hypermesh-") ||
            node_id.starts_with("hm-")
        )
    }

    /// Generate a fingerprint for a node ID
    fn generate_fingerprint(&self, node_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(node_id.as_bytes());
        hasher.update(b"hypermesh-transport");
        let hash = hasher.finalize();
        format!("{:x}", hash)
    }

    /// Validate a certificate (placeholder for future implementation)
    pub async fn validate_certificate(&self, cert_data: &[u8]) -> Result<CertificateValidation> {
        // For MVP, generate a mock validation result
        let mut hasher = Sha256::new();
        hasher.update(cert_data);
        let fingerprint = format!("{:x}", hasher.finalize());

        Ok(CertificateValidation {
            is_valid: true,
            node_id: "node-mvp".to_string(),
            expires_at: chrono::Utc::now() + chrono::Duration::days(365),
            fingerprint,
            validation_errors: vec![],
        })
    }

    /// Clear the certificate cache
    pub fn clear_cache(&self) {
        let mut cache = self.cert_cache.write();
        cache.clear();
        debug!("Certificate cache cleared");
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cert_cache.read();
        let total = cache.len();
        let valid = cache.values().filter(|c| c.validation_result).count();
        (total, valid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_node_authentication() {
        let config = AuthenticationConfig::default();
        let auth = NodeAuthenticator::new(config);

        // Test valid node IDs
        assert!(auth.authenticate_node("node-123").await.unwrap());
        assert!(auth.authenticate_node("hypermesh-abc").await.unwrap());
        assert!(auth.authenticate_node("hm-xyz").await.unwrap());

        // Test invalid node ID
        assert!(!auth.authenticate_node("invalid").await.unwrap());
    }

    #[test]
    fn test_fingerprint_generation() {
        let config = AuthenticationConfig::default();
        let auth = NodeAuthenticator::new(config);

        let fp1 = auth.generate_fingerprint("node-123");
        let fp2 = auth.generate_fingerprint("node-123");
        let fp3 = auth.generate_fingerprint("node-456");

        // Same input should produce same fingerprint
        assert_eq!(fp1, fp2);
        // Different input should produce different fingerprint
        assert_ne!(fp1, fp3);
    }
}