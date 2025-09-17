//! TrustChain Configuration Management
//! 
//! Central configuration for TrustChain services with IPv6-only networking
//! and consensus validation parameters.

use std::net::Ipv6Addr;
use std::time::Duration;
use serde::{Serialize, Deserialize};
use anyhow::{Result, anyhow};
use crate::consensus::ConsensusRequirements;
use crate::ca::CAConfig;

/// Main TrustChain configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrustChainConfig {
    /// Certificate Authority configuration
    pub ca: CAConfig,
    /// Certificate Transparency configuration
    pub ct: CTConfig,
    /// DNS resolver configuration
    pub dns: DnsConfig,
    /// API server configuration
    pub api: ApiConfig,
    /// Network configuration
    pub network: NetworkConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
}

/// Certificate Transparency configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CTConfig {
    /// CT log identifier
    pub log_id: String,
    /// IPv6 bind address
    pub bind_address: Ipv6Addr,
    /// Port for CT services
    pub port: u16,
    /// Maximum log entries per shard
    pub max_entries_per_shard: u64,
    /// Merkle tree update interval
    pub merkle_update_interval: Duration,
    /// Log storage path
    pub storage_path: String,
    /// Enable real-time fingerprinting
    pub enable_realtime_fingerprinting: bool,
    /// Consensus requirements for CT operations
    pub consensus_requirements: ConsensusRequirements,
}

impl Default for CTConfig {
    fn default() -> Self {
        Self {
            log_id: "trustchain-ct-localhost".to_string(),
            bind_address: Ipv6Addr::LOCALHOST,
            port: 6962, // Standard CT log port
            max_entries_per_shard: 1_000_000,
            merkle_update_interval: Duration::from_secs(60),
            storage_path: "/tmp/trustchain_ct".to_string(),
            enable_realtime_fingerprinting: true,
            consensus_requirements: ConsensusRequirements::localhost_testing(),
        }
    }
}

impl CTConfig {
    /// Production CT configuration
    pub fn production() -> Self {
        Self {
            log_id: "trustchain-ct-production".to_string(),
            bind_address: Ipv6Addr::UNSPECIFIED,
            port: 6962,
            max_entries_per_shard: 10_000_000,
            merkle_update_interval: Duration::from_secs(30),
            storage_path: "/var/lib/trustchain/ct".to_string(),
            enable_realtime_fingerprinting: true,
            consensus_requirements: ConsensusRequirements::production(),
        }
    }
}

/// DNS resolver configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsConfig {
    /// DNS server identifier
    pub server_id: String,
    /// IPv6 bind address
    pub bind_address: Ipv6Addr,
    /// Port for DNS-over-QUIC
    pub quic_port: u16,
    /// Traditional DNS port (for backward compatibility)
    pub port: u16,
    /// Traditional DNS port (disabled for IPv6-only)
    pub dns_port: Option<u16>,
    /// Upstream DNS resolvers
    pub upstream_resolvers: Vec<Ipv6Addr>,
    /// DNS cache TTL
    pub cache_ttl: Duration,
    /// Enable certificate DNS validation
    pub enable_cert_validation: bool,
    /// TrustChain domains to resolve
    pub trustchain_domains: Vec<String>,
    /// Consensus requirements for DNS operations
    pub consensus_requirements: ConsensusRequirements,
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            server_id: "trustchain-dns-localhost".to_string(),
            bind_address: Ipv6Addr::LOCALHOST,
            quic_port: 8853, // DNS-over-QUIC port
            port: 53, // Traditional DNS port for compatibility
            dns_port: None, // IPv6-only, no traditional DNS
            upstream_resolvers: vec![
                "2001:4860:4860::8888".parse().unwrap(), // Google IPv6
                "2606:4700:4700::1111".parse().unwrap(), // Cloudflare IPv6
            ],
            cache_ttl: Duration::from_secs(300),
            enable_cert_validation: true,
            trustchain_domains: vec![
                "hypermesh".to_string(),
                "caesar".to_string(),
                "trust".to_string(),
                "assets".to_string(),
            ],
            consensus_requirements: ConsensusRequirements::localhost_testing(),
        }
    }
}

impl DnsConfig {
    /// Production DNS configuration
    pub fn production() -> Self {
        Self {
            server_id: "trustchain-dns-production".to_string(),
            bind_address: Ipv6Addr::UNSPECIFIED,
            quic_port: 8853,
            port: 53,
            dns_port: None,
            upstream_resolvers: vec![
                "2001:4860:4860::8888".parse().unwrap(),
                "2606:4700:4700::1111".parse().unwrap(),
            ],
            cache_ttl: Duration::from_secs(600),
            enable_cert_validation: true,
            trustchain_domains: vec![
                "hypermesh".to_string(),
                "caesar".to_string(),
                "trust".to_string(),
                "assets".to_string(),
            ],
            consensus_requirements: ConsensusRequirements::production(),
        }
    }
}

/// API server configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiConfig {
    /// API server identifier
    pub server_id: String,
    /// IPv6 bind address
    pub bind_address: Ipv6Addr,
    /// Port for REST API
    pub port: u16,
    /// Enable TLS for API endpoints
    pub enable_tls: bool,
    /// API rate limiting
    pub rate_limit_per_minute: u32,
    /// Maximum request body size
    pub max_body_size: usize,
    /// CORS allowed origins
    pub cors_origins: Vec<String>,
    /// Consensus requirements for API operations
    pub consensus_requirements: ConsensusRequirements,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            server_id: "trustchain-api-localhost".to_string(),
            bind_address: Ipv6Addr::LOCALHOST,
            port: 8080,
            enable_tls: false, // Disabled for localhost testing
            rate_limit_per_minute: 60,
            max_body_size: 1024 * 1024, // 1MB
            cors_origins: vec!["*".to_string()], // Permissive for testing
            consensus_requirements: ConsensusRequirements::localhost_testing(),
        }
    }
}

impl ApiConfig {
    /// Production API configuration
    pub fn production() -> Self {
        Self {
            server_id: "trustchain-api-production".to_string(),
            bind_address: Ipv6Addr::UNSPECIFIED,
            port: 8080,
            enable_tls: true,
            rate_limit_per_minute: 300,
            max_body_size: 10 * 1024 * 1024, // 10MB
            cors_origins: vec![
                "https://hypermesh.online".to_string(),
                "https://trust.hypermesh.online".to_string(),
            ],
            consensus_requirements: ConsensusRequirements::production(),
        }
    }
}

/// Network configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// IPv6-only networking
    pub ipv6_only: bool,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Keep-alive interval
    pub keep_alive_interval: Duration,
    /// Maximum concurrent connections
    pub max_concurrent_connections: u32,
    /// TLS configuration
    pub tls: TlsConfig,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            ipv6_only: true,
            connection_timeout: Duration::from_secs(30),
            keep_alive_interval: Duration::from_secs(60),
            max_concurrent_connections: 1000,
            tls: TlsConfig::default(),
        }
    }
}

/// TLS configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Minimum TLS version
    pub min_version: TlsVersion,
    /// Cipher suites
    pub cipher_suites: Vec<String>,
    /// Certificate validation mode
    pub cert_validation: CertValidationMode,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TlsVersion {
    #[serde(rename = "1.2")]
    V12,
    #[serde(rename = "1.3")]
    V13,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CertValidationMode {
    /// Strict certificate validation
    Strict,
    /// Allow self-signed certificates
    AllowSelfSigned,
    /// Development mode (bypass validation)
    Development,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            min_version: TlsVersion::V13,
            cipher_suites: vec![
                "TLS_AES_256_GCM_SHA384".to_string(),
                "TLS_CHACHA20_POLY1305_SHA256".to_string(),
                "TLS_AES_128_GCM_SHA256".to_string(),
            ],
            cert_validation: CertValidationMode::AllowSelfSigned,
        }
    }
}

/// Logging configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: LogLevel,
    /// Log format
    pub format: LogFormat,
    /// Log output destination
    pub output: LogOutput,
    /// Enable structured logging
    pub structured: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LogLevel {
    #[serde(rename = "trace")]
    Trace,
    #[serde(rename = "debug")]
    Debug,
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "warn")]
    Warn,
    #[serde(rename = "error")]
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LogFormat {
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "pretty")]
    Pretty,
    #[serde(rename = "compact")]
    Compact,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LogOutput {
    #[serde(rename = "stdout")]
    Stdout,
    #[serde(rename = "stderr")]
    Stderr,
    #[serde(rename = "file")]
    File { path: String },
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: LogFormat::Pretty,
            output: LogOutput::Stdout,
            structured: false,
        }
    }
}

impl Default for TrustChainConfig {
    fn default() -> Self {
        Self::localhost_testing()
    }
}

impl TrustChainConfig {
    /// Configuration for localhost testing
    pub fn localhost_testing() -> Self {
        Self {
            ca: CAConfig::default(),
            ct: CTConfig::default(),
            dns: DnsConfig::default(),
            api: ApiConfig::default(),
            network: NetworkConfig::default(),
            logging: LoggingConfig::default(),
        }
    }

    /// Configuration for production deployment
    pub fn production() -> Self {
        Self {
            ca: CAConfig::production(),
            ct: CTConfig::production(),
            dns: DnsConfig::production(),
            api: ApiConfig::production(),
            network: NetworkConfig::default(),
            logging: LoggingConfig {
                level: LogLevel::Info,
                format: LogFormat::Json,
                output: LogOutput::File {
                    path: "/var/log/trustchain/trustchain.log".to_string(),
                },
                structured: true,
            },
        }
    }

    /// Load configuration from file
    pub fn from_file(path: &str) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read config file {}: {}", path, e))?;

        if path.ends_with(".toml") {
            toml::from_str(&contents)
                .map_err(|e| anyhow!("Failed to parse TOML config: {}", e))
        } else if path.ends_with(".yaml") || path.ends_with(".yml") {
            serde_yaml::from_str(&contents)
                .map_err(|e| anyhow!("Failed to parse YAML config: {}", e))
        } else if path.ends_with(".json") {
            serde_json::from_str(&contents)
                .map_err(|e| anyhow!("Failed to parse JSON config: {}", e))
        } else {
            Err(anyhow!("Unsupported config file format: {}", path))
        }
    }

    /// Save configuration to file
    pub fn to_file(&self, path: &str) -> Result<()> {
        let contents = if path.ends_with(".toml") {
            toml::to_string_pretty(self)
                .map_err(|e| anyhow!("Failed to serialize config to TOML: {}", e))?
        } else if path.ends_with(".yaml") || path.ends_with(".yml") {
            serde_yaml::to_string(self)
                .map_err(|e| anyhow!("Failed to serialize config to YAML: {}", e))?
        } else if path.ends_with(".json") {
            serde_json::to_string_pretty(self)
                .map_err(|e| anyhow!("Failed to serialize config to JSON: {}", e))?
        } else {
            return Err(anyhow!("Unsupported config file format: {}", path));
        };

        std::fs::write(path, contents)
            .map_err(|e| anyhow!("Failed to write config file {}: {}", path, e))?;

        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate port conflicts
        let mut ports = vec![self.ca.port, self.ct.port, self.dns.quic_port, self.api.port];
        ports.sort();
        for window in ports.windows(2) {
            if window[0] == window[1] {
                return Err(anyhow!("Port conflict detected: {}", window[0]));
            }
        }

        // Validate IPv6 addresses
        if !self.network.ipv6_only {
            return Err(anyhow!("TrustChain requires IPv6-only networking"));
        }

        // Validate consensus requirements consistency
        if self.ca.consensus_requirements.minimum_stake != self.ct.consensus_requirements.minimum_stake {
            return Err(anyhow!("Consensus requirements must be consistent across services"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = TrustChainConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_production_config() {
        let config = TrustChainConfig::production();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_serialization() {
        let config = TrustChainConfig::localhost_testing();
        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: TrustChainConfig = toml::from_str(&toml_str).unwrap();
        
        assert_eq!(config.ca.ca_id, deserialized.ca.ca_id);
    }

    #[test]
    fn test_config_file_operations() {
        let config = TrustChainConfig::localhost_testing();
        
        // Test TOML
        let toml_file = NamedTempFile::new().unwrap();
        config.to_file(toml_file.path().to_str().unwrap()).unwrap();
        let loaded_config = TrustChainConfig::from_file(toml_file.path().to_str().unwrap()).unwrap();
        assert_eq!(config.ca.ca_id, loaded_config.ca.ca_id);
    }

    #[test]
    fn test_port_conflict_detection() {
        let mut config = TrustChainConfig::localhost_testing();
        config.api.port = config.ca.port; // Create port conflict
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_ipv6_only_validation() {
        let mut config = TrustChainConfig::localhost_testing();
        config.network.ipv6_only = false;
        assert!(config.validate().is_err());
    }
}