// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! TrustChain Configuration Management
//!
//! Central configuration for TrustChain services with IPv6-only networking
//! and state proof validation parameters.

use crate::ca::CAConfig;
use crate::proof_of_state::StateRequirements;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::net::Ipv6Addr;
use std::time::Duration;

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
    /// State proof requirements for CT operations
    pub state_requirements: StateRequirements,
}

impl Default for CTConfig {
    fn default() -> Self {
        Self {
            log_id: "trustchain-ct-localhost".to_string(),
            bind_address: Ipv6Addr::LOCALHOST,
            port: 6962, // Standard CT log port (use testing() method for port 0)
            max_entries_per_shard: 1_000_000,
            merkle_update_interval: Duration::from_secs(60),
            storage_path: "/tmp/trustchain_ct".to_string(),
            enable_realtime_fingerprinting: true,
            state_requirements: StateRequirements::localhost_testing(),
        }
    }
}

impl CTConfig {
    /// Testing CT configuration with OS-assigned random port
    pub fn testing() -> Self {
        Self {
            log_id: "trustchain-ct-test".to_string(),
            bind_address: Ipv6Addr::LOCALHOST,
            port: 0, // OS-assigned random port to avoid conflicts
            max_entries_per_shard: 1_000_000,
            merkle_update_interval: Duration::from_secs(60),
            storage_path: "/tmp/trustchain_ct".to_string(),
            enable_realtime_fingerprinting: false, // Disabled for testing
            state_requirements: StateRequirements::localhost_testing(),
        }
    }

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
            state_requirements: StateRequirements::production(),
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
    /// State proof requirements for DNS operations
    pub state_requirements: StateRequirements,
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            server_id: "trustchain-dns-localhost".to_string(),
            bind_address: Ipv6Addr::LOCALHOST,
            quic_port: 8853, // DNS-over-QUIC port (use testing() method for port 0)
            port: 53, // Traditional DNS port for compatibility (use testing() method for port 0)
            dns_port: None, // IPv6-only, no traditional DNS
            upstream_resolvers: vec![
                Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888), // Google IPv6
                Ipv6Addr::new(0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1111), // Cloudflare IPv6
            ],
            cache_ttl: Duration::from_secs(300),
            enable_cert_validation: true,
            trustchain_domains: vec![
                "hypermesh".to_string(),
                "caesar".to_string(),
                "trust".to_string(),
                "assets".to_string(),
                "catalog".to_string(),
                "ngauge".to_string(),
            ],
            state_requirements: StateRequirements::localhost_testing(),
        }
    }
}

impl DnsConfig {
    /// Testing DNS configuration with OS-assigned random ports
    pub fn testing() -> Self {
        Self {
            server_id: "trustchain-dns-test".to_string(),
            bind_address: Ipv6Addr::LOCALHOST,
            quic_port: 0, // OS-assigned random port to avoid conflicts
            port: 0,      // OS-assigned random port to avoid conflicts
            dns_port: None,
            upstream_resolvers: vec![
                Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888), // Google IPv6
                Ipv6Addr::new(0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1111), // Cloudflare IPv6
            ],
            cache_ttl: Duration::from_secs(300),
            enable_cert_validation: false, // Disabled for testing
            trustchain_domains: vec![
                "hypermesh".to_string(),
                "caesar".to_string(),
                "trust".to_string(),
                "assets".to_string(),
                "catalog".to_string(),
                "ngauge".to_string(),
            ],
            state_requirements: StateRequirements::localhost_testing(),
        }
    }

    /// Production DNS configuration
    pub fn production() -> Self {
        Self {
            server_id: "trustchain-dns-production".to_string(),
            bind_address: Ipv6Addr::UNSPECIFIED,
            quic_port: 8853,
            port: 53,
            dns_port: None,
            upstream_resolvers: vec![
                Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888), // Google IPv6
                Ipv6Addr::new(0x2606, 0x4700, 0x4700, 0, 0, 0, 0, 0x1111), // Cloudflare IPv6
            ],
            cache_ttl: Duration::from_secs(600),
            enable_cert_validation: true,
            trustchain_domains: vec![
                "hypermesh".to_string(),
                "caesar".to_string(),
                "trust".to_string(),
                "assets".to_string(),
                "catalog".to_string(),
                "ngauge".to_string(),
            ],
            state_requirements: StateRequirements::production(),
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
    /// State proof requirements for API operations
    pub state_requirements: StateRequirements,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            server_id: "trustchain-api-localhost".to_string(),
            bind_address: Ipv6Addr::LOCALHOST,
            port: 8080,        // Standard API port (use testing() method for port 0)
            enable_tls: false, // Disabled for localhost testing
            rate_limit_per_minute: 60,
            max_body_size: 1024 * 1024,          // 1MB
            cors_origins: vec!["*".to_string()], // Permissive for testing
            state_requirements: StateRequirements::localhost_testing(),
        }
    }
}

impl ApiConfig {
    /// Testing API configuration with OS-assigned random port
    pub fn testing() -> Self {
        Self {
            server_id: "trustchain-api-test".to_string(),
            bind_address: Ipv6Addr::LOCALHOST,
            port: 0, // OS-assigned random port to avoid conflicts
            enable_tls: false,
            rate_limit_per_minute: 1000, // Higher limit for tests
            max_body_size: 1024 * 1024,
            cors_origins: vec!["*".to_string()],
            state_requirements: StateRequirements::localhost_testing(),
        }
    }

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
            state_requirements: StateRequirements::production(),
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
    /// Configuration for localhost testing with OS-assigned random ports
    pub fn localhost_testing() -> Self {
        Self {
            ca: CAConfig::testing(),
            ct: CTConfig::testing(),
            dns: DnsConfig::testing(),
            api: ApiConfig::testing(),
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

    /// Load configuration with the following precedence:
    /// 1. `TRUSTCHAIN_CONFIG` env var (path to config file)
    /// 2. `~/.hypermesh/trustchain.toml` (user config)
    /// 3. `/etc/hypermesh/trustchain.toml` (system config)
    /// 4. Default (localhost testing)
    ///
    /// Returns the loaded config and the source path (if any).
    pub fn load() -> Result<(Self, Option<String>)> {
        // 1. Check env var
        if let Ok(path) = std::env::var("TRUSTCHAIN_CONFIG") {
            let config = Self::from_file(&path)?;
            tracing::info!("Loaded config from TRUSTCHAIN_CONFIG={}", path);
            return Ok((config, Some(path)));
        }

        // 2. Check ~/.hypermesh/trustchain.toml
        if let Some(home) = dirs_path() {
            let user_path = format!("{home}/.hypermesh/trustchain.toml");
            if std::path::Path::new(&user_path).exists() {
                let config = Self::from_file(&user_path)?;
                tracing::info!("Loaded config from {}", user_path);
                return Ok((config, Some(user_path)));
            }
        }

        // 3. Check /etc/hypermesh/trustchain.toml
        let system_path = "/etc/hypermesh/trustchain.toml";
        if std::path::Path::new(system_path).exists() {
            let config = Self::from_file(system_path)?;
            tracing::info!("Loaded config from {}", system_path);
            return Ok((config, Some(system_path.to_string())));
        }

        // 4. Fall back to defaults
        tracing::info!("No config file found, using defaults");
        Ok((Self::default(), None))
    }

    /// Load configuration from file
    pub fn from_file(path: &str) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read config file {path}: {e}"))?;

        if path.ends_with(".toml") {
            toml::from_str(&contents).map_err(|e| anyhow!("Failed to parse TOML config: {e}"))
        } else if path.ends_with(".yaml") || path.ends_with(".yml") {
            serde_yaml::from_str(&contents).map_err(|e| anyhow!("Failed to parse YAML config: {e}"))
        } else if path.ends_with(".json") {
            serde_json::from_str(&contents).map_err(|e| anyhow!("Failed to parse JSON config: {e}"))
        } else {
            Err(anyhow!("Unsupported config file format: {path}"))
        }
    }

    /// Save configuration to file
    pub fn to_file(&self, path: &str) -> Result<()> {
        let contents = if path.ends_with(".toml") {
            toml::to_string_pretty(self)
                .map_err(|e| anyhow!("Failed to serialize config to TOML: {e}"))?
        } else if path.ends_with(".yaml") || path.ends_with(".yml") {
            serde_yaml::to_string(self)
                .map_err(|e| anyhow!("Failed to serialize config to YAML: {e}"))?
        } else if path.ends_with(".json") {
            serde_json::to_string_pretty(self)
                .map_err(|e| anyhow!("Failed to serialize config to JSON: {e}"))?
        } else {
            return Err(anyhow!("Unsupported config file format: {path}"));
        };

        std::fs::write(path, contents)
            .map_err(|e| anyhow!("Failed to write config file {path}: {e}"))?;

        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate port conflicts (skip port 0 which means OS-assigned)
        let mut ports: Vec<u16> = vec![
            self.ca.port,
            self.ct.port,
            self.dns.quic_port,
            self.api.port,
        ]
        .into_iter()
        .filter(|&p| p != 0)
        .collect();
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

        // Validate state proof requirements consistency. Proofs carry no
        // magnitude, so consistency is checked on the WHEN-freshness bound
        // (`max_time_offset`) shared across services.
        if self.ca.state_requirements.max_time_offset
            != self.ct.state_requirements.max_time_offset
        {
            return Err(anyhow!(
                "State proof requirements must be consistent across services"
            ));
        }

        Ok(())
    }
}

/// Get the user's home directory path as a String.
fn dirs_path() -> Option<String> {
    std::env::var("HOME").ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TrustChainConfig::default();
        match config.validate() {
            Ok(_) => {}
            Err(e) => unreachable!("Config validation failed: {e}"),
        }
    }

    #[test]
    fn test_production_config() {
        let config = TrustChainConfig::production();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_serialization() {
        let config = TrustChainConfig::localhost_testing();
        let toml_str = toml::to_string(&config).expect("test: expected success");
        let deserialized: TrustChainConfig = toml::from_str(&toml_str).expect("test: expected success");

        assert_eq!(config.ca.ca_id, deserialized.ca.ca_id);
    }

    #[test]
    fn test_config_file_operations() {
        let config = TrustChainConfig::localhost_testing();

        // Test TOML (use Builder to add .toml suffix)
        let toml_file = tempfile::Builder::new().suffix(".toml").tempfile().expect("test: creation");
        config.to_file(toml_file.path().to_str().expect("test: expected success")).expect("test: expected success");
        let loaded_config =
            TrustChainConfig::from_file(toml_file.path().to_str().expect("test: expected success")).expect("test: expected success");
        assert_eq!(config.ca.ca_id, loaded_config.ca.ca_id);
    }

    #[test]
    fn test_port_conflict_detection() {
        let mut config = TrustChainConfig::localhost_testing();
        // Set both ports to same non-zero value to create conflict
        config.api.port = 8443;
        config.ca.port = 8443;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_ipv6_only_validation() {
        let mut config = TrustChainConfig::localhost_testing();
        config.network.ipv6_only = false;
        assert!(config.validate().is_err());
    }
}
