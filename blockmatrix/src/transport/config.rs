//! Configuration for HyperMesh Transport Layer

use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::net::Ipv6Addr;

// Import STOQ configuration types
use stoq;

// Type alias for backwards compatibility
pub type TransportConfig = HyperMeshTransportConfig;

/// HyperMesh transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperMeshTransportConfig {
    /// Network configuration
    pub network: NetworkConfig,
    /// Connection pool configuration
    pub pool: ConnectionPoolConfig,
    /// Authentication configuration
    pub auth: AuthenticationConfig,
    /// Monitoring configuration
    pub monitoring: MonitoringConfig,
    /// Performance tuning
    pub performance: PerformanceConfig,
}

/// Network layer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// IPv6 bind address
    pub bind_address: Ipv6Addr,
    /// Port to bind to
    pub port: u16,
    /// Maximum concurrent connections
    pub max_connections: Option<u32>,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Enable connection migration
    pub enable_migration: bool,
    /// Enable 0-RTT resumption
    pub enable_0rtt: bool,
    /// Maximum idle timeout
    pub max_idle_timeout: Duration,
    /// Maximum concurrent streams per connection
    pub max_concurrent_streams: u32,
}

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolConfig {
    /// Maximum connections to keep in pool
    pub max_pool_size: usize,
    /// Connection idle timeout
    pub idle_timeout: Duration,
    /// Health check interval
    pub health_check_interval: Duration,
    /// Enable connection reuse
    pub enable_reuse: bool,
    /// Maximum connection age before forced refresh
    pub max_connection_age: Duration,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationConfig {
    /// Certificate store path
    pub cert_store_path: String,
    /// Certificate rotation interval
    pub cert_rotation_interval: Duration,
    /// Enable certificate validation
    pub enable_cert_validation: bool,
    /// Trust store path
    pub trust_store_path: String,
    /// Certificate revocation check interval
    pub revocation_check_interval: Duration,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable Prometheus metrics
    pub enable_prometheus: bool,
    /// Metrics collection interval
    pub metrics_interval: Duration,
    /// Enable tracing
    pub enable_tracing: bool,
    /// Maximum metrics history to keep
    pub max_metrics_history: usize,
    /// Health check endpoints
    pub health_endpoints: Vec<String>,
}

/// Performance tuning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Send buffer size per connection
    pub send_buffer_size: usize,
    /// Receive buffer size per connection
    pub receive_buffer_size: usize,
    /// Worker thread count (0 = auto)
    pub worker_threads: usize,
    /// Enable zero-copy optimizations
    pub enable_zero_copy: bool,
    /// Batch size for operations
    pub batch_size: usize,
}

impl HyperMeshTransportConfig {
    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate performance settings
        if self.performance.send_buffer_size == 0 {
            return Err("send_buffer_size cannot be zero".to_string());
        }
        if self.performance.receive_buffer_size == 0 {
            return Err("receive_buffer_size cannot be zero".to_string());
        }
        if self.pool.max_pool_size == 0 {
            return Err("max_pool_size cannot be zero".to_string());
        }
        Ok(())
    }
}

impl Default for HyperMeshTransportConfig {
    fn default() -> Self {
        Self {
            network: NetworkConfig::default(),
            pool: ConnectionPoolConfig::default(),
            auth: AuthenticationConfig::default(),
            monitoring: MonitoringConfig::default(),
            performance: PerformanceConfig::default(),
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            bind_address: Ipv6Addr::UNSPECIFIED,
            port: 30000,
            max_connections: Some(100_000),
            connection_timeout: Duration::from_secs(10),
            enable_migration: true,
            enable_0rtt: true,
            max_idle_timeout: Duration::from_secs(30),
            max_concurrent_streams: 1_000_000,
        }
    }
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_pool_size: 10_000,
            idle_timeout: Duration::from_secs(300), // 5 minutes
            health_check_interval: Duration::from_secs(60),
            enable_reuse: true,
            max_connection_age: Duration::from_secs(24 * 3600), // 24 hours
        }
    }
}

impl Default for AuthenticationConfig {
    fn default() -> Self {
        Self {
            cert_store_path: "/etc/hypermesh/certs".to_string(),
            cert_rotation_interval: Duration::from_secs(24 * 3600), // 24 hours
            enable_cert_validation: true,
            trust_store_path: "/etc/hypermesh/trust".to_string(),
            revocation_check_interval: Duration::from_secs(3600), // 1 hour
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_prometheus: true,
            metrics_interval: Duration::from_secs(10),
            enable_tracing: true,
            max_metrics_history: 10_000,
            health_endpoints: vec!["/health".to_string(), "/metrics".to_string()],
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            send_buffer_size: 1024 * 1024, // 1MB
            receive_buffer_size: 1024 * 1024, // 1MB
            worker_threads: 0, // Auto-detect
            enable_zero_copy: true,
            batch_size: 100,
        }
    }
}

impl HyperMeshTransportConfig {
    /// Create a basic network configuration for internal use
    pub fn to_network_config(&self) -> NetworkConfig {
        self.network.clone()
    }

    /// Convert to STOQ transport configuration
    pub fn to_stoq_transport_config(&self) -> stoq::transport::TransportConfig {
        stoq::transport::TransportConfig {
            bind_address: self.network.bind_address,
            port: self.network.port,
            max_connections: self.network.max_connections,
            connection_timeout: self.network.connection_timeout,
            enable_migration: self.network.enable_migration,
            enable_0rtt: self.network.enable_0rtt,
            max_idle_timeout: self.network.max_idle_timeout,
            max_concurrent_streams: self.network.max_concurrent_streams,
            send_buffer_size: self.performance.send_buffer_size,
            receive_buffer_size: self.performance.receive_buffer_size,
            cert_rotation_interval: std::time::Duration::from_secs(24 * 60 * 60), // 24 hours
        }
    }

    // REMOVED: STOQ routing, chunking, and edge configurations
    // These are application-layer concerns not supported by STOQ transport protocol
    // STOQ focuses on pure transport: packet delivery, connection management,
    // flow control, and congestion control only
    
    /// Load configuration from file
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&content)?;
        Ok(config)
    }
    
    /// Save configuration to file
    pub fn to_file(&self, path: &str) -> anyhow::Result<()> {
        let content = serde_yaml::to_string(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    
    /// Validate configuration
    pub fn validate(&self) -> anyhow::Result<()> {
        // Validate network settings
        if self.network.port == 0 {
            return Err(anyhow::anyhow!("Port cannot be 0"));
        }
        
        if self.network.max_concurrent_streams == 0 {
            return Err(anyhow::anyhow!("Max concurrent streams cannot be 0"));
        }
        
        // Validate connection pool settings
        if self.pool.max_pool_size == 0 {
            return Err(anyhow::anyhow!("Connection pool size cannot be 0"));
        }
        
        // Validate performance settings
        if self.performance.send_buffer_size == 0 || self.performance.receive_buffer_size == 0 {
            return Err(anyhow::anyhow!("Buffer sizes cannot be 0"));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config_validation() {
        let config = HyperMeshTransportConfig::default();
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_network_config_conversion() {
        let config = HyperMeshTransportConfig::default();
        let network_config = config.to_network_config();
        
        assert_eq!(network_config.bind_address, config.network.bind_address);
        assert_eq!(network_config.port, config.network.port);
        assert_eq!(network_config.enable_migration, config.network.enable_migration);
    }
    
    #[test]
    fn test_config_serialization() {
        let config = HyperMeshTransportConfig::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let deserialized: HyperMeshTransportConfig = serde_yaml::from_str(&yaml).unwrap();
        
        assert_eq!(config.network.port, deserialized.network.port);
    }
}