//! Transport layer configuration

use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv6Addr};
use std::time::Duration;

/// Transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    /// Address to bind to (IPv6 preferred)
    pub bind_address: IpAddr,
    
    /// Port to bind to
    pub port: u16,
    
    /// Maximum number of concurrent connections
    pub max_connections: u32,
    
    /// Connection timeout
    pub connection_timeout: Duration,
    
    /// Keep-alive interval
    pub keep_alive_interval: Duration,
    
    /// Maximum packet size (MTU)
    pub max_packet_size: u16,
    
    /// Enable connection migration
    pub enable_migration: bool,
    
    /// Enable 0-RTT connection resumption
    pub enable_0rtt: bool,
    
    /// Maximum idle timeout for connections
    pub max_idle_timeout: Duration,
    
    /// Initial congestion window size
    pub initial_window_size: u32,
    
    /// Maximum stream data buffer size
    pub max_stream_data: u32,
    
    /// Maximum concurrent streams per connection
    pub max_concurrent_streams: u32,
    
    /// Certificate configuration
    pub certificate: CertificateConfig,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            bind_address: IpAddr::V6(Ipv6Addr::UNSPECIFIED),
            port: 7777,
            max_connections: 10000,
            connection_timeout: Duration::from_secs(30),
            keep_alive_interval: Duration::from_secs(30),
            max_packet_size: 1400,
            enable_migration: true,
            enable_0rtt: true,
            max_idle_timeout: Duration::from_secs(300), // 5 minutes
            initial_window_size: 32768,                 // 32KB
            max_stream_data: 1048576,                   // 1MB
            max_concurrent_streams: 1000,
            certificate: CertificateConfig::default(),
        }
    }
}

/// Certificate configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateConfig {
    /// Path to certificate file
    pub cert_path: String,
    
    /// Path to private key file
    pub key_path: String,
    
    /// Path to CA bundle for client certificate verification
    pub ca_bundle_path: Option<String>,
    
    /// Require client certificates
    pub require_client_cert: bool,
    
    /// Certificate rotation interval
    pub rotation_interval: Duration,
    
    /// Subject name for self-signed certificates
    pub subject_name: String,
    
    /// Certificate validity period for self-signed certs
    pub validity_days: u32,
}

impl Default for CertificateConfig {
    fn default() -> Self {
        Self {
            cert_path: "./certs/server.pem".to_string(),
            key_path: "./certs/server.key".to_string(),
            ca_bundle_path: None,
            require_client_cert: true,
            rotation_interval: Duration::from_secs(24 * 60 * 60), // 24 hours
            subject_name: "nexus-node".to_string(),
            validity_days: 365,
        }
    }
}

impl TransportConfig {
    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.port == 0 {
            return Err("Port cannot be zero".to_string());
        }
        
        if self.max_connections == 0 {
            return Err("Maximum connections must be greater than zero".to_string());
        }
        
        if self.max_packet_size < 1200 {
            return Err("Maximum packet size should be at least 1200 bytes".to_string());
        }
        
        if self.max_packet_size > 9000 {
            return Err("Maximum packet size should not exceed 9000 bytes".to_string());
        }
        
        if self.max_concurrent_streams == 0 {
            return Err("Maximum concurrent streams must be greater than zero".to_string());
        }
        
        Ok(())
    }
    
    /// Get socket address for binding
    pub fn socket_addr(&self) -> std::net::SocketAddr {
        std::net::SocketAddr::new(self.bind_address, self.port)
    }
    
    /// Create Quinn client configuration
    pub fn to_quinn_client_config(&self) -> quinn::ClientConfig {
        quinn::ClientConfig::new(std::sync::Arc::new(
            self.create_rustls_client_config()
        ))
    }
    
    /// Create Quinn server configuration
    pub fn to_quinn_server_config(&self, server_config: rustls::ServerConfig) -> quinn::ServerConfig {
        quinn::ServerConfig::with_crypto(std::sync::Arc::new(server_config))
    }
    
    /// Apply common transport configuration to Quinn
    fn apply_quinn_transport_config(&self, transport: &mut std::sync::Arc<quinn::TransportConfig>) {
        let mut transport_config = quinn::TransportConfig::default();
        
        transport_config
            .max_idle_timeout(Some(
                quinn::IdleTimeout::try_from(self.max_idle_timeout)
                    .expect("Invalid idle timeout")
            ))
            .keep_alive_interval(Some(self.keep_alive_interval))
            .max_concurrent_uni_streams(quinn::VarInt::from_u32(self.max_concurrent_streams / 2))
            .max_concurrent_bidi_streams(quinn::VarInt::from_u32(self.max_concurrent_streams / 2))
            .stream_receive_window(quinn::VarInt::from_u32(self.max_stream_data))
            .receive_window(quinn::VarInt::from_u32(self.initial_window_size));
        
        // 0RTT configuration not available in this Quinn version
        // TODO: Implement when supported
        
        *transport = std::sync::Arc::new(transport_config);
    }
    
    /// Create default rustls client configuration
    fn create_rustls_client_config(&self) -> rustls::ClientConfig {
        let mut config = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(rustls::RootCertStore::empty())
            .with_no_client_auth();
        
        // Configure ALPN for QUIC
        config.alpn_protocols = vec![b"nexus/1".to_vec()];
        
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config_validation() {
        let config = TransportConfig::default();
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_invalid_port() {
        let mut config = TransportConfig::default();
        config.port = 0;
        assert!(config.validate().is_err());
    }
    
    #[test]
    fn test_socket_addr() {
        let config = TransportConfig::default();
        let addr = config.socket_addr();
        assert_eq!(addr.port(), 7777);
    }
    
    #[test]
    fn test_packet_size_limits() {
        let mut config = TransportConfig::default();
        
        config.max_packet_size = 1100;
        assert!(config.validate().is_err());
        
        config.max_packet_size = 10000;
        assert!(config.validate().is_err());
        
        config.max_packet_size = 1400;
        assert!(config.validate().is_ok());
    }
}