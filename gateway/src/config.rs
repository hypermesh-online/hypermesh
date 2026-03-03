// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Source of TLS certificates for the gateway's QUIC endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CertificateSource {
    /// Load certificates from PEM/DER files on disk.
    File {
        cert_path: PathBuf,
        key_path: PathBuf,
    },
    /// Obtain certificates from a TrustChain CA endpoint.
    ///
    /// Falls back to self-signed because FALCON-1024 certificates
    /// are not trusted by clearnet browsers.
    TrustChain {
        ca_addr: SocketAddr,
        common_name: String,
    },
    /// Generate an ephemeral self-signed certificate at startup.
    SelfSigned { common_name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Gateway listening address
    pub listen_addr: SocketAddr,

    /// TrustChain backend address
    pub trustchain_addr: SocketAddr,

    /// BlockMatrix backend address
    pub blockmatrix_addr: SocketAddr,

    /// Caesar backend address
    pub caesar_addr: SocketAddr,

    /// Catalog backend address
    pub catalog_addr: SocketAddr,

    /// engauge analytics backend address
    pub engauge_addr: SocketAddr,

    /// TLS server name for TrustChain backend (SNI)
    pub trustchain_server_name: String,

    /// TLS server name for BlockMatrix backend (SNI)
    pub blockmatrix_server_name: String,

    /// TLS server name for Caesar backend (SNI)
    pub caesar_server_name: String,

    /// TLS server name for Catalog backend (SNI)
    pub catalog_server_name: String,

    /// TLS server name for engauge backend (SNI)
    pub engauge_server_name: String,

    /// Certificate configuration
    pub cert_path: PathBuf,
    pub key_path: PathBuf,

    /// Connection pool configuration
    pub pool: ConnectionPoolConfig,

    /// Retry configuration
    pub retry: RetryConfig,

    /// CORS configuration
    pub cors: CorsConfig,

    /// Logging level
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolConfig {
    /// Maximum connections per backend
    pub max_connections: usize,

    /// Idle timeout for connections
    pub idle_timeout: Duration,

    /// Connection timeout
    pub connect_timeout: Duration,

    /// Keep-alive interval
    pub keep_alive_interval: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,

    /// Base delay between retries
    pub base_delay: Duration,

    /// Maximum delay between retries
    pub max_delay: Duration,

    /// Exponential backoff multiplier
    pub multiplier: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// Allowed origins
    pub allowed_origins: Vec<String>,

    /// Allowed methods
    pub allowed_methods: Vec<String>,

    /// Allowed headers
    pub allowed_headers: Vec<String>,

    /// Allow credentials
    pub allow_credentials: bool,

    /// Max age in seconds
    pub max_age: u64,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            listen_addr: "[::]:8443"
                .parse()
                .expect("hardcoded default listen addr is valid"),
            trustchain_addr: "[::1]:8444"
                .parse()
                .expect("hardcoded default trustchain addr is valid"),
            blockmatrix_addr: "[::1]:9292"
                .parse()
                .expect("hardcoded default blockmatrix addr is valid"),
            caesar_addr: "[::1]:9294"
                .parse()
                .expect("hardcoded default caesar addr is valid"),
            catalog_addr: "[::1]:9295"
                .parse()
                .expect("hardcoded default catalog addr is valid"),
            engauge_addr: "[::1]:9296"
                .parse()
                .expect("hardcoded default engauge addr is valid"),
            trustchain_server_name: "trustchain".to_string(),
            blockmatrix_server_name: "blockmatrix".to_string(),
            caesar_server_name: "caesar".to_string(),
            catalog_server_name: "catalog".to_string(),
            engauge_server_name: "engauge".to_string(),
            cert_path: PathBuf::from("certs/server.crt"),
            key_path: PathBuf::from("certs/server.key"),
            pool: ConnectionPoolConfig::default(),
            retry: RetryConfig::default(),
            cors: CorsConfig::default(),
            log_level: "info".to_string(),
        }
    }
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            idle_timeout: Duration::from_secs(300),
            connect_timeout: Duration::from_secs(10),
            keep_alive_interval: Duration::from_secs(30),
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            multiplier: 2.0,
        }
    }
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["http://localhost:5173".to_string()],
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "OPTIONS".to_string(),
            ],
            allowed_headers: vec![
                "Content-Type".to_string(),
                "Authorization".to_string(),
                "X-Request-ID".to_string(),
            ],
            allow_credentials: true,
            max_age: 3600,
        }
    }
}

impl GatewayConfig {
    pub fn from_env() -> Result<Self> {
        let mut config = Self::default();

        // Override from environment variables if set
        if let Ok(addr) = std::env::var("GATEWAY_LISTEN_ADDR") {
            config.listen_addr = addr.parse()?;
        }

        if let Ok(addr) = std::env::var("TRUSTCHAIN_ADDR") {
            config.trustchain_addr = addr.parse()?;
        }

        if let Ok(addr) = std::env::var("BLOCKMATRIX_ADDR") {
            config.blockmatrix_addr = addr.parse()?;
        }

        if let Ok(addr) = std::env::var("CAESAR_ADDR") {
            config.caesar_addr = addr.parse()?;
        }

        if let Ok(name) = std::env::var("TRUSTCHAIN_SERVER_NAME") {
            config.trustchain_server_name = name;
        }

        if let Ok(name) = std::env::var("BLOCKMATRIX_SERVER_NAME") {
            config.blockmatrix_server_name = name;
        }

        if let Ok(name) = std::env::var("CAESAR_SERVER_NAME") {
            config.caesar_server_name = name;
        }

        if let Ok(addr) = std::env::var("CATALOG_ADDR") {
            config.catalog_addr = addr.parse()?;
        }

        if let Ok(name) = std::env::var("CATALOG_SERVER_NAME") {
            config.catalog_server_name = name;
        }

        if let Ok(addr) = std::env::var("ENGAUGE_ADDR") {
            config.engauge_addr = addr.parse()?;
        }

        if let Ok(name) = std::env::var("ENGAUGE_SERVER_NAME") {
            config.engauge_server_name = name;
        }

        if let Ok(path) = std::env::var("CERT_PATH") {
            config.cert_path = PathBuf::from(path);
        }

        if let Ok(path) = std::env::var("KEY_PATH") {
            config.key_path = PathBuf::from(path);
        }

        if let Ok(level) = std::env::var("LOG_LEVEL") {
            config.log_level = level;
        }

        Ok(config)
    }

    /// Load configuration from a TOML file.
    pub fn from_file(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        let config: Self = toml::from_str(&contents)
            .with_context(|| format!("failed to parse config file: {}", path.display()))?;
        Ok(config)
    }
}
