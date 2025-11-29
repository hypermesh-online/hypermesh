//! Certificate management for transport layer authentication

use crate::{Result, TransportError};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use rcgen::{Certificate, CertificateParams, KeyPair, DistinguishedName, DnType};
use rustls::{ServerConfig, ClientConfig};
use rustls_pemfile;

/// Certificate manager handles certificate generation, rotation, and validation
pub struct CertificateManager {
    /// Current server certificate
    server_cert: Arc<parking_lot::RwLock<Certificate>>,
    
    /// Root certificate store for client verification
    root_store: Arc<parking_lot::RwLock<rustls::RootCertStore>>,
    
    /// Certificate rotation interval
    rotation_interval: Duration,
    
    /// Last rotation time
    last_rotation: Arc<parking_lot::RwLock<SystemTime>>,
}

impl CertificateManager {
    /// Create a new certificate manager with self-signed certificate
    pub async fn new_self_signed(
        subject_name: String,
        validity_days: u32,
        rotation_interval: Duration,
    ) -> Result<Self> {
        let cert = generate_self_signed_cert(&subject_name, validity_days)?;
        let mut root_store = rustls::RootCertStore::empty();
        
        // Add self-signed cert to root store for testing
        let cert_der = cert.serialize_der().map_err(|e| TransportError::Certificate {
            message: format!("Failed to serialize certificate: {}", e),
        })?;
        
        root_store
            .add(&rustls::Certificate(cert_der))
            .map_err(|e| TransportError::Certificate {
                message: format!("Failed to add certificate to root store: {}", e),
            })?;
        
        Ok(Self {
            server_cert: Arc::new(parking_lot::RwLock::new(cert)),
            root_store: Arc::new(parking_lot::RwLock::new(root_store)),
            rotation_interval,
            last_rotation: Arc::new(parking_lot::RwLock::new(SystemTime::now())),
        })
    }
    
    /// Load certificate from files
    pub async fn from_files(
        cert_path: &str,
        key_path: &str,
        ca_bundle_path: Option<&str>,
        rotation_interval: Duration,
    ) -> Result<Self> {
        // Load certificate and key
        let cert_pem = std::fs::read_to_string(cert_path)
            .map_err(|e| TransportError::Certificate {
                message: format!("Failed to read certificate file {}: {}", cert_path, e),
            })?;
            
        let key_pem = std::fs::read_to_string(key_path)
            .map_err(|e| TransportError::Certificate {
                message: format!("Failed to read key file {}: {}", key_path, e),
            })?;
        
        // Parse certificate - simplified for now, will need proper implementation
        // TODO: Implement proper certificate loading from PEM files
        let cert = generate_self_signed_cert("loaded-cert", 365)?;
        
        // Load CA bundle if provided
        let mut root_store = rustls::RootCertStore::empty();
        if let Some(ca_path) = ca_bundle_path {
            let ca_pem = std::fs::read_to_string(ca_path)
                .map_err(|e| TransportError::Certificate {
                    message: format!("Failed to read CA bundle {}: {}", ca_path, e),
                })?;
                
            let ca_certs = rustls_pemfile::certs(&mut ca_pem.as_bytes())
                .map_err(|e| TransportError::Certificate {
                    message: format!("Failed to parse CA certificates: {}", e),
                })?;
                
            for ca_cert in ca_certs {
                root_store
                    .add(&rustls::Certificate(ca_cert))
                    .map_err(|e| TransportError::Certificate {
                        message: format!("Failed to add CA certificate: {}", e),
                    })?;
            }
        }
        
        Ok(Self {
            server_cert: Arc::new(parking_lot::RwLock::new(cert)),
            root_store: Arc::new(parking_lot::RwLock::new(root_store)),
            rotation_interval,
            last_rotation: Arc::new(parking_lot::RwLock::new(SystemTime::now())),
        })
    }
    
    /// Get current server certificate
    pub fn server_certificate(&self) -> Arc<parking_lot::RwLock<Certificate>> {
        Arc::clone(&self.server_cert)
    }
    
    /// Create rustls server configuration
    pub fn server_config(&self) -> Result<ServerConfig> {
        let cert = self.server_cert.read();
        let cert_der = cert.serialize_der()
            .map_err(|e| TransportError::Certificate {
                message: format!("Failed to serialize certificate: {}", e),
            })?;
            
        let key_der = cert.serialize_private_key_der();
        
        let cert_chain = vec![rustls::Certificate(cert_der)];
        let private_key = rustls::PrivateKey(key_der);
        
        let mut config = ServerConfig::builder()
            .with_safe_defaults()
            .with_client_cert_verifier(Arc::new(ClientCertVerifier::new(
                self.root_store.clone()
            )))
            .with_single_cert(cert_chain, private_key)
            .map_err(|e| TransportError::Certificate {
                message: format!("Failed to create server config: {}", e),
            })?;
            
        // Configure ALPN for QUIC
        config.alpn_protocols = vec![b"nexus/1".to_vec()];
        
        Ok(config)
    }
    
    /// Create rustls client configuration
    pub fn client_config(&self) -> Result<ClientConfig> {
        let config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(self.root_store.read().clone())
            .with_no_client_auth();
            
        Ok(config)
    }
    
    /// Check if certificate needs rotation
    pub fn needs_rotation(&self) -> bool {
        let last_rotation = *self.last_rotation.read();
        SystemTime::now()
            .duration_since(last_rotation)
            .unwrap_or(Duration::ZERO)
            >= self.rotation_interval
    }
    
    /// Rotate certificate (generate new self-signed)
    pub async fn rotate_certificate(&self, subject_name: &str, validity_days: u32) -> Result<()> {
        let new_cert = generate_self_signed_cert(subject_name, validity_days)?;
        
        // Update certificate and root store
        let cert_der = new_cert.serialize_der().map_err(|e| TransportError::Certificate {
            message: format!("Failed to serialize certificate: {}", e),
        })?;
        
        *self.server_cert.write() = new_cert;
        
        // Update root store with new certificate
        let mut root_store = rustls::RootCertStore::empty();
        root_store
            .add(&rustls::Certificate(cert_der))
            .map_err(|e| TransportError::Certificate {
                message: format!("Failed to add rotated certificate to root store: {}", e),
            })?;
            
        *self.root_store.write() = root_store;
        *self.last_rotation.write() = SystemTime::now();
        
        tracing::info!("Certificate rotated successfully");
        Ok(())
    }
}

impl std::fmt::Debug for CertificateManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CertificateManager")
            .field("rotation_interval", &self.rotation_interval)
            .field("last_rotation", &*self.last_rotation.read())
            .finish_non_exhaustive()
    }
}

/// Custom client certificate verifier
struct ClientCertVerifier {
    root_store: Arc<parking_lot::RwLock<rustls::RootCertStore>>,
}

impl ClientCertVerifier {
    fn new(root_store: Arc<parking_lot::RwLock<rustls::RootCertStore>>) -> Self {
        Self { root_store }
    }
}

impl rustls::server::ClientCertVerifier for ClientCertVerifier {
    fn offer_client_auth(&self) -> bool {
        true
    }
    
    fn client_auth_root_subjects(&self) -> &[rustls::DistinguishedName] {
        &[]
    }
    
    fn verify_client_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _now: std::time::SystemTime,
    ) -> std::result::Result<rustls::server::ClientCertVerified, rustls::Error> {
        // For now, accept all client certificates
        // TODO: Implement proper verification against root store
        Ok(rustls::server::ClientCertVerified::assertion())
    }
}

/// Generate a self-signed certificate
pub fn generate_self_signed_cert(subject_name: &str, validity_days: u32) -> Result<Certificate> {
    let mut params = CertificateParams::default();
    
    // Set subject name
    let mut distinguished_name = DistinguishedName::new();
    distinguished_name.push(DnType::CommonName, subject_name);
    params.distinguished_name = distinguished_name;
    
    // Set validity period
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
        
    params.not_before = time::OffsetDateTime::from_unix_timestamp(now.as_secs() as i64)
        .expect("Invalid timestamp");
        
    params.not_after = time::OffsetDateTime::from_unix_timestamp(
        (now.as_secs() + (validity_days as u64 * 24 * 60 * 60)) as i64
    ).expect("Invalid timestamp");
    
    // Add subject alternative names
    params.subject_alt_names = vec![
        rcgen::SanType::DnsName(subject_name.to_string()),
        rcgen::SanType::IpAddress(std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST)),
        rcgen::SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
    ];
    
    // Generate certificate
    Certificate::from_params(params).map_err(|e| TransportError::Certificate {
        message: format!("Failed to generate self-signed certificate: {}", e),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_self_signed_certificate_generation() {
        let cert_manager = CertificateManager::new_self_signed(
            "test-node".to_string(),
            365,
            Duration::from_secs(3600),
        ).await.unwrap();
        
        let cert = cert_manager.server_certificate();
        assert!(cert.read().serialize_pem().unwrap().contains("BEGIN CERTIFICATE"));
    }
    
    #[tokio::test]
    async fn test_server_config_creation() {
        let cert_manager = CertificateManager::new_self_signed(
            "test-node".to_string(),
            365,
            Duration::from_secs(3600),
        ).await.unwrap();
        
        let server_config = cert_manager.server_config().unwrap();
        assert_eq!(server_config.alpn_protocols, vec![b"nexus/1".to_vec()]);
    }
    
    #[tokio::test]
    async fn test_certificate_rotation() {
        let cert_manager = CertificateManager::new_self_signed(
            "test-node".to_string(),
            365,
            Duration::from_secs(1), // 1 second rotation
        ).await.unwrap();
        
        let original_cert = cert_manager.server_certificate();
        
        // Wait for rotation interval
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        assert!(cert_manager.needs_rotation());
        
        cert_manager.rotate_certificate("test-node", 365).await.unwrap();
        let new_cert = cert_manager.server_certificate();
        
        assert_ne!(
            original_cert.read().serialize_der().unwrap(),
            new_cert.read().serialize_der().unwrap()
        );
    }
}