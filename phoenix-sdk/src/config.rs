//! Phoenix SDK Configuration
//!
//! Provides configuration options for Phoenix SDK with sensible defaults
//! and builder patterns for customization.

use std::time::Duration;
use serde::{Serialize, Deserialize};

/// Phoenix Configuration with zero-config defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoenixConfig {
    /// Application name for identification
    pub app_name: String,

    /// Performance tier for automatic optimization
    pub performance_tier: PerformanceTier,

    /// Security level for certificate and encryption settings
    pub security_level: SecurityLevel,

    /// Optional region for geographic optimization
    pub region: Option<String>,

    /// Enable automatic performance optimization
    pub auto_optimize: bool,

    /// Enable compression for data transfer
    pub enable_compression: bool,

    /// Enable metrics collection
    pub enable_metrics: bool,

    /// Connection timeout
    pub connection_timeout: Duration,

    /// Idle connection timeout
    pub idle_timeout: Duration,

    /// Maximum concurrent connections
    pub max_connections: usize,
}

impl PhoenixConfig {
    /// Create development configuration with minimal security
    pub fn development(app_name: &str) -> Self {
        Self {
            app_name: app_name.to_string(),
            performance_tier: PerformanceTier::Development,
            security_level: SecurityLevel::Development,
            region: None,
            auto_optimize: true,
            enable_compression: true,
            enable_metrics: true,
            connection_timeout: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(300),
            max_connections: 100,
        }
    }

    /// Create production configuration with standard settings
    pub fn production(app_name: &str) -> Self {
        Self {
            app_name: app_name.to_string(),
            performance_tier: PerformanceTier::Production,
            security_level: SecurityLevel::Standard,
            region: None,
            auto_optimize: true,
            enable_compression: true,
            enable_metrics: true,
            connection_timeout: Duration::from_secs(5),
            idle_timeout: Duration::from_secs(600),
            max_connections: 1000,
        }
    }

    /// Create high-performance configuration
    pub fn high_performance(app_name: &str) -> Self {
        Self {
            app_name: app_name.to_string(),
            performance_tier: PerformanceTier::HighThroughput,
            security_level: SecurityLevel::Enhanced,
            region: None,
            auto_optimize: true,
            enable_compression: false, // Disable for maximum throughput
            enable_metrics: true,
            connection_timeout: Duration::from_secs(3),
            idle_timeout: Duration::from_secs(1800),
            max_connections: 10000,
        }
    }

    /// Set performance tier
    pub fn with_performance_tier(mut self, tier: PerformanceTier) -> Self {
        self.performance_tier = tier;
        self
    }

    /// Set security level
    pub fn with_security_level(mut self, level: SecurityLevel) -> Self {
        self.security_level = level;
        self
    }

    /// Set region for geographic optimization
    pub fn with_region(mut self, region: &str) -> Self {
        self.region = Some(region.to_string());
        self
    }

    /// Enable or disable auto-optimization
    pub fn with_auto_optimize(mut self, enable: bool) -> Self {
        self.auto_optimize = enable;
        self
    }

    /// Enable or disable compression
    pub fn with_compression(mut self, enable: bool) -> Self {
        self.enable_compression = enable;
        self
    }

    /// Enable or disable metrics
    pub fn with_metrics(mut self, enable: bool) -> Self {
        self.enable_metrics = enable;
        self
    }

    /// Set connection timeout
    pub fn with_connection_timeout(mut self, timeout: Duration) -> Self {
        self.connection_timeout = timeout;
        self
    }

    /// Set idle timeout
    pub fn with_idle_timeout(mut self, timeout: Duration) -> Self {
        self.idle_timeout = timeout;
        self
    }

    /// Set maximum connections
    pub fn with_max_connections(mut self, max: usize) -> Self {
        self.max_connections = max;
        self
    }
}

impl Default for PhoenixConfig {
    fn default() -> Self {
        Self::development("phoenix-app")
    }
}

/// Performance tiers with automatic optimization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PerformanceTier {
    /// Development tier (<1 Gbps, local testing)
    Development,

    /// Production tier (1-10 Gbps, standard applications)
    Production,

    /// High-throughput tier (10+ Gbps, data-intensive)
    HighThroughput,

    /// Custom tier with specific Gbps target
    Custom(u64),
}

impl PerformanceTier {
    /// Get target throughput in Gbps
    pub fn target_gbps(&self) -> f64 {
        match self {
            Self::Development => 1.0,
            Self::Production => 10.0,
            Self::HighThroughput => 40.0,
            Self::Custom(gbps) => *gbps as f64,
        }
    }

    /// Get recommended buffer size
    pub fn buffer_size(&self) -> usize {
        match self {
            Self::Development => 65536,      // 64 KB
            Self::Production => 262144,      // 256 KB
            Self::HighThroughput => 1048576, // 1 MB
            Self::Custom(gbps) => {
                if *gbps <= 1 {
                    65536
                } else if *gbps <= 10 {
                    262144
                } else {
                    1048576
                }
            }
        }
    }

    /// Get recommended connection pool size
    pub fn connection_pool_size(&self) -> usize {
        match self {
            Self::Development => 10,
            Self::Production => 100,
            Self::HighThroughput => 1000,
            Self::Custom(gbps) => {
                if *gbps <= 1 {
                    10
                } else if *gbps <= 10 {
                    100
                } else {
                    1000
                }
            }
        }
    }
}

impl std::fmt::Display for PerformanceTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Development => write!(f, "Development (<1 Gbps)"),
            Self::Production => write!(f, "Production (1-10 Gbps)"),
            Self::HighThroughput => write!(f, "High Throughput (10+ Gbps)"),
            Self::Custom(gbps) => write!(f, "Custom ({} Gbps)", gbps),
        }
    }
}

/// Security levels from development to post-quantum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecurityLevel {
    /// Development level - self-signed certificates, fast setup
    Development,

    /// Standard level - TLS with certificate validation
    Standard,

    /// Enhanced level - Mutual TLS with certificate pinning
    Enhanced,

    /// Post-quantum level - FALCON quantum-resistant cryptography
    PostQuantum,
}

impl SecurityLevel {
    /// Check if certificate validation is required
    pub fn requires_cert_validation(&self) -> bool {
        !matches!(self, Self::Development)
    }

    /// Check if mutual TLS is required
    pub fn requires_mutual_tls(&self) -> bool {
        matches!(self, Self::Enhanced | Self::PostQuantum)
    }

    /// Check if post-quantum crypto is required
    pub fn requires_post_quantum(&self) -> bool {
        matches!(self, Self::PostQuantum)
    }

    /// Get TLS version requirement
    pub fn min_tls_version(&self) -> &str {
        match self {
            Self::Development => "TLS 1.2",
            Self::Standard => "TLS 1.3",
            Self::Enhanced => "TLS 1.3",
            Self::PostQuantum => "TLS 1.3",
        }
    }

    /// Get cipher suite preferences
    pub fn cipher_suites(&self) -> Vec<&str> {
        match self {
            Self::Development => vec![
                "TLS_AES_128_GCM_SHA256",
                "TLS_AES_256_GCM_SHA384",
            ],
            Self::Standard => vec![
                "TLS_AES_256_GCM_SHA384",
                "TLS_CHACHA20_POLY1305_SHA256",
                "TLS_AES_128_GCM_SHA256",
            ],
            Self::Enhanced => vec![
                "TLS_AES_256_GCM_SHA384",
                "TLS_CHACHA20_POLY1305_SHA256",
            ],
            Self::PostQuantum => vec![
                // Post-quantum cipher suites when available
                "TLS_AES_256_GCM_SHA384",
                "TLS_CHACHA20_POLY1305_SHA256",
            ],
        }
    }
}

impl std::fmt::Display for SecurityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Development => write!(f, "Development (self-signed)"),
            Self::Standard => write!(f, "Standard (TLS 1.3)"),
            Self::Enhanced => write!(f, "Enhanced (mTLS + pinning)"),
            Self::PostQuantum => write!(f, "Post-Quantum (FALCON-1024)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_development_config() {
        let config = PhoenixConfig::development("test-app");
        assert_eq!(config.app_name, "test-app");
        assert_eq!(config.performance_tier, PerformanceTier::Development);
        assert_eq!(config.security_level, SecurityLevel::Development);
        assert_eq!(config.max_connections, 100);
    }

    #[test]
    fn test_production_config() {
        let config = PhoenixConfig::production("prod-app");
        assert_eq!(config.app_name, "prod-app");
        assert_eq!(config.performance_tier, PerformanceTier::Production);
        assert_eq!(config.security_level, SecurityLevel::Standard);
        assert_eq!(config.max_connections, 1000);
    }

    #[test]
    fn test_config_builder() {
        let config = PhoenixConfig::development("test")
            .with_performance_tier(PerformanceTier::Custom(25))
            .with_security_level(SecurityLevel::Enhanced)
            .with_region("us-west")
            .with_max_connections(5000);

        assert_eq!(config.performance_tier, PerformanceTier::Custom(25));
        assert_eq!(config.security_level, SecurityLevel::Enhanced);
        assert_eq!(config.region, Some("us-west".to_string()));
        assert_eq!(config.max_connections, 5000);
    }

    #[test]
    fn test_performance_tier_settings() {
        assert_eq!(PerformanceTier::Development.target_gbps(), 1.0);
        assert_eq!(PerformanceTier::Production.target_gbps(), 10.0);
        assert_eq!(PerformanceTier::HighThroughput.target_gbps(), 40.0);
        assert_eq!(PerformanceTier::Custom(100).target_gbps(), 100.0);

        assert_eq!(PerformanceTier::Development.buffer_size(), 65536);
        assert_eq!(PerformanceTier::Production.buffer_size(), 262144);
        assert_eq!(PerformanceTier::HighThroughput.buffer_size(), 1048576);
    }

    #[test]
    fn test_security_level_settings() {
        assert!(!SecurityLevel::Development.requires_cert_validation());
        assert!(SecurityLevel::Standard.requires_cert_validation());
        assert!(SecurityLevel::Enhanced.requires_mutual_tls());
        assert!(SecurityLevel::PostQuantum.requires_post_quantum());

        assert_eq!(SecurityLevel::Development.min_tls_version(), "TLS 1.2");
        assert_eq!(SecurityLevel::Standard.min_tls_version(), "TLS 1.3");
    }
}