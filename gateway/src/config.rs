// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Gateway listening address
    pub listen_addr: SocketAddr,

    /// TrustChain backend address
    pub trustchain_addr: SocketAddr,

    /// BlockMatrix backend address
    pub blockmatrix_addr: SocketAddr,

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
            listen_addr: "[::]:8443".parse().unwrap(),
            trustchain_addr: "[::1]:50053".parse().unwrap(),
            blockmatrix_addr: "[::1]:8446".parse().unwrap(),
            cert_path: PathBuf::from("/home/persist/repos/projects/web3/certs/server.crt"),
            key_path: PathBuf::from("/home/persist/repos/projects/web3/certs/server.key"),
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
}