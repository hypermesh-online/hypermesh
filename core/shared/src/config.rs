//! Configuration management for Nexus components

use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv6Addr};
use std::time::Duration;

/// Global configuration for Nexus core
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NexusConfig {
    pub node: NodeConfig,
    pub transport: TransportConfig,
    pub security: SecurityConfig,
    pub storage: StorageConfig,
    pub logging: LoggingConfig,
}

impl Default for NexusConfig {
    fn default() -> Self {
        Self {
            node: NodeConfig::default(),
            transport: TransportConfig::default(),
            security: SecurityConfig::default(),
            storage: StorageConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

/// Node-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Node identifier (auto-generated if not specified)
    pub id: Option<String>,
    
    /// Node name for display purposes
    pub name: String,
    
    /// Data directory for persistent storage
    pub data_dir: String,
    
    /// Maximum CPU cores to use (0 = all available)
    pub max_cpu_cores: u32,
    
    /// Maximum memory to use in MB (0 = 80% of available)
    pub max_memory_mb: u64,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            id: None,
            name: "nexus-node".to_string(),
            data_dir: "./data".to_string(),
            max_cpu_cores: 0,
            max_memory_mb: 0,
        }
    }
}

/// Transport layer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    /// IPv6 address to bind to
    pub bind_address: IpAddr,
    
    /// Port to bind to for QUIC connections
    pub port: u16,
    
    /// Maximum number of concurrent connections
    pub max_connections: u32,
    
    /// Connection timeout in milliseconds
    pub connection_timeout_ms: u64,
    
    /// Keep-alive interval in milliseconds
    pub keep_alive_ms: u64,
    
    /// Maximum packet size
    pub max_packet_size: u32,
    
    /// Enable connection migration
    pub enable_migration: bool,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            bind_address: IpAddr::V6(Ipv6Addr::UNSPECIFIED),
            port: 7777,
            max_connections: 10000,
            connection_timeout_ms: 30000,
            keep_alive_ms: 5000,
            max_packet_size: 1400,
            enable_migration: true,
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Path to TLS certificate file
    pub cert_path: String,
    
    /// Path to TLS private key file
    pub key_path: String,
    
    /// Certificate authority bundle path
    pub ca_bundle_path: Option<String>,
    
    /// Enable client certificate verification
    pub require_client_cert: bool,
    
    /// Certificate rotation interval in hours
    pub cert_rotation_hours: u64,
    
    /// Enable encryption at rest
    pub encrypt_at_rest: bool,
    
    /// Encryption key derivation rounds
    pub key_derivation_rounds: u32,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            cert_path: "./certs/server.pem".to_string(),
            key_path: "./certs/server.key".to_string(),
            ca_bundle_path: None,
            require_client_cert: true,
            cert_rotation_hours: 24,
            encrypt_at_rest: true,
            key_derivation_rounds: 100_000,
        }
    }
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage backend type
    pub backend: StorageBackend,
    
    /// Maximum storage size in MB
    pub max_size_mb: u64,
    
    /// Compaction threshold (0.0-1.0)
    pub compaction_threshold: f64,
    
    /// Backup interval in minutes
    pub backup_interval_minutes: u64,
    
    /// Number of backup copies to retain
    pub backup_retention_count: u32,
    
    /// Enable write-ahead logging
    pub enable_wal: bool,
    
    /// WAL sync mode
    pub wal_sync_mode: WalSyncMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageBackend {
    RocksDB,
    Sled,
    Memory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalSyncMode {
    None,
    Normal,
    Full,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: StorageBackend::RocksDB,
            max_size_mb: 10 * 1024, // 10GB
            compaction_threshold: 0.8,
            backup_interval_minutes: 60,
            backup_retention_count: 7,
            enable_wal: true,
            wal_sync_mode: WalSyncMode::Normal,
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    
    /// Log format (json, pretty)
    pub format: String,
    
    /// Enable logging to file
    pub file_enabled: bool,
    
    /// Log file path
    pub file_path: String,
    
    /// Maximum log file size in MB
    pub max_file_size_mb: u64,
    
    /// Number of log files to retain
    pub max_files: u32,
    
    /// Enable structured logging
    pub structured: bool,
    
    /// Enable metrics collection
    pub metrics_enabled: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "pretty".to_string(),
            file_enabled: true,
            file_path: "./logs/nexus.log".to_string(),
            max_file_size_mb: 100,
            max_files: 10,
            structured: true,
            metrics_enabled: true,
        }
    }
}

impl NexusConfig {
    /// Load configuration from file
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: NexusConfig = toml::from_str(&content)?;
        Ok(config)
    }
    
    /// Save configuration to file
    pub fn to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    
    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.transport.port == 0 {
            return Err("Transport port cannot be zero".to_string());
        }
        
        if self.transport.max_connections == 0 {
            return Err("Maximum connections must be greater than zero".to_string());
        }
        
        if self.storage.max_size_mb == 0 {
            return Err("Storage max size must be greater than zero".to_string());
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NexusConfig::default();
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_config_serialization() {
        let config = NexusConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: NexusConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(config.transport.port, parsed.transport.port);
    }
}