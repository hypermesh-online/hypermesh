// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Authentication mode for the gateway's STOQ listener (security finding F8).
///
/// The gateway can operate the STOQ port in two distinct modes:
///
/// * [`StoqAuthMode::HttpProxy`] — the listener acts as a plain reverse
///   proxy for backwards compatibility. Incoming STOQ connections are
///   accepted and forwarded WITHOUT requiring a bilateral Proof-of-State
///   handshake. This is correct when the gateway is fronting a
///   clearnet/HTTP endpoint (there is no HyperMesh asset to authenticate
///   against). **Default**, to preserve existing deployments.
///
/// * [`StoqAuthMode::FullStoqPos`] — the listener is a decentralized port
///   onto a HyperMesh asset. Every incoming connection MUST complete a
///   bilateral PoS handshake (FALCON-1024 identity binding + four-proof
///   state proof validation, inheriting the F2 signer↔identity binding)
///   before any bytes are handled. Connections that fail to handshake are
///   dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StoqAuthMode {
    /// Plain reverse-proxy passthrough — no PoS handshake required.
    HttpProxy,
    /// Decentralized HyperMesh port — bilateral PoS handshake required.
    FullStoqPos,
}

impl Default for StoqAuthMode {
    fn default() -> Self {
        // Backwards-compatible default: today's listener accepts without a
        // handshake, which is passthrough/HTTP-proxy behavior. Opt in to
        // FullStoqPos explicitly.
        Self::HttpProxy
    }
}

impl StoqAuthMode {
    /// Parse a mode from a case-insensitive string. Accepts several
    /// spellings for ergonomics in config files and env vars.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "http-proxy" | "http" | "proxy" | "passthrough" => Some(Self::HttpProxy),
            "full-stoq-pos" | "stoq-pos" | "stoq" | "pos" | "full" => Some(Self::FullStoqPos),
            _ => None,
        }
    }

    /// Stable string form (matches the kebab-case serde representation).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HttpProxy => "http-proxy",
            Self::FullStoqPos => "full-stoq-pos",
        }
    }

    /// True when this mode requires a completed bilateral PoS handshake.
    pub fn requires_pos_handshake(self) -> bool {
        matches!(self, Self::FullStoqPos)
    }
}

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

    /// STOQ listener address (default [::]:8444).
    /// Set to `None` to disable the STOQ listener entirely.
    pub stoq_listen_addr: Option<SocketAddr>,

    /// Maximum concurrent STOQ connections through the bridge.
    pub stoq_max_connections: u32,

    /// STOQ listener authentication mode (security finding F8).
    ///
    /// [`StoqAuthMode::HttpProxy`] (default) accepts connections without a
    /// PoS handshake for reverse-proxy backwards compatibility.
    /// [`StoqAuthMode::FullStoqPos`] requires a completed bilateral PoS
    /// handshake on every connection. `#[serde(default)]` keeps older TOML
    /// files (which omit this field) parseable.
    #[serde(default)]
    pub stoq_auth_mode: StoqAuthMode,
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
            stoq_listen_addr: Some(
                "[::]:8444"
                    .parse()
                    .expect("hardcoded default STOQ listen addr is valid"),
            ),
            stoq_max_connections: 100,
            stoq_auth_mode: StoqAuthMode::default(),
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
    /// Default config file path: `~/.hypermesh/gateway.toml`.
    pub fn default_path() -> PathBuf {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        home.join(".hypermesh").join("gateway.toml")
    }

    /// Load configuration with cascading priority:
    ///
    /// 1. `GATEWAY_CONFIG` env var (explicit file path)
    /// 2. `~/.hypermesh/gateway.toml` (user config)
    /// 3. Environment variable overrides (applied on top)
    /// 4. Built-in defaults (for any fields not set)
    ///
    /// If no config file exists, returns defaults with env overrides.
    pub fn load() -> Result<Self> {
        // Check for explicit config path via env var
        if let Ok(path) = std::env::var("GATEWAY_CONFIG") {
            let file_path = Path::new(&path);
            if file_path.exists() {
                let mut config = Self::from_file(file_path)?;
                config.apply_env_overrides()?;
                return Ok(config);
            }
            anyhow::bail!(
                "GATEWAY_CONFIG points to non-existent file: {}",
                file_path.display()
            );
        }

        // Try default path
        let default = Self::default_path();
        let mut config = if default.exists() {
            Self::from_file(&default)?
        } else {
            Self::default()
        };

        config.apply_env_overrides()?;
        Ok(config)
    }

    /// Apply environment variable overrides to an existing config.
    fn apply_env_overrides(&mut self) -> Result<()> {
        if let Ok(addr) = std::env::var("GATEWAY_LISTEN_ADDR") {
            self.listen_addr = addr.parse()?;
        }
        if let Ok(addr) = std::env::var("TRUSTCHAIN_ADDR") {
            self.trustchain_addr = addr.parse()?;
        }
        if let Ok(addr) = std::env::var("BLOCKMATRIX_ADDR") {
            self.blockmatrix_addr = addr.parse()?;
        }
        if let Ok(addr) = std::env::var("CAESAR_ADDR") {
            self.caesar_addr = addr.parse()?;
        }
        if let Ok(name) = std::env::var("TRUSTCHAIN_SERVER_NAME") {
            self.trustchain_server_name = name;
        }
        if let Ok(name) = std::env::var("BLOCKMATRIX_SERVER_NAME") {
            self.blockmatrix_server_name = name;
        }
        if let Ok(name) = std::env::var("CAESAR_SERVER_NAME") {
            self.caesar_server_name = name;
        }
        if let Ok(addr) = std::env::var("CATALOG_ADDR") {
            self.catalog_addr = addr.parse()?;
        }
        if let Ok(name) = std::env::var("CATALOG_SERVER_NAME") {
            self.catalog_server_name = name;
        }
        if let Ok(addr) = std::env::var("ENGAUGE_ADDR") {
            self.engauge_addr = addr.parse()?;
        }
        if let Ok(name) = std::env::var("ENGAUGE_SERVER_NAME") {
            self.engauge_server_name = name;
        }
        if let Ok(path) = std::env::var("CERT_PATH") {
            self.cert_path = PathBuf::from(path);
        }
        if let Ok(path) = std::env::var("KEY_PATH") {
            self.key_path = PathBuf::from(path);
        }
        if let Ok(level) = std::env::var("LOG_LEVEL") {
            self.log_level = level;
        }
        if let Ok(addr) = std::env::var("STOQ_LISTEN_ADDR") {
            if addr.eq_ignore_ascii_case("none") || addr.eq_ignore_ascii_case("disabled") {
                self.stoq_listen_addr = None;
            } else {
                self.stoq_listen_addr = Some(addr.parse()?);
            }
        }
        if let Ok(max) = std::env::var("STOQ_MAX_CONNECTIONS") {
            self.stoq_max_connections = max.parse()?;
        }
        if let Ok(mode) = std::env::var("STOQ_AUTH_MODE") {
            self.stoq_auth_mode = StoqAuthMode::parse(&mode)
                .with_context(|| format!("invalid STOQ_AUTH_MODE: {mode:?}"))?;
        }
        Ok(())
    }

    /// Load configuration from environment variables only (legacy API).
    ///
    /// Prefer [`GatewayConfig::load`] which supports TOML files + env overrides.
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

        if let Ok(addr) = std::env::var("STOQ_LISTEN_ADDR") {
            if addr.eq_ignore_ascii_case("none") || addr.eq_ignore_ascii_case("disabled") {
                config.stoq_listen_addr = None;
            } else {
                config.stoq_listen_addr = Some(addr.parse()?);
            }
        }

        if let Ok(max) = std::env::var("STOQ_MAX_CONNECTIONS") {
            config.stoq_max_connections = max.parse()?;
        }

        if let Ok(mode) = std::env::var("STOQ_AUTH_MODE") {
            config.stoq_auth_mode = StoqAuthMode::parse(&mode)
                .with_context(|| format!("invalid STOQ_AUTH_MODE: {mode:?}"))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn default_config_is_valid() {
        let config = GatewayConfig::default();
        assert_eq!(config.listen_addr.port(), 8443);
        assert_eq!(config.stoq_max_connections, 100);
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn default_stoq_auth_mode_is_http_proxy() {
        // Backwards-compatible default preserves existing passthrough behavior.
        let config = GatewayConfig::default();
        assert_eq!(config.stoq_auth_mode, StoqAuthMode::HttpProxy);
        assert!(!config.stoq_auth_mode.requires_pos_handshake());
    }

    #[test]
    fn stoq_auth_mode_parse_variants() {
        assert_eq!(StoqAuthMode::parse("http-proxy"), Some(StoqAuthMode::HttpProxy));
        assert_eq!(StoqAuthMode::parse("HTTP_PROXY"), Some(StoqAuthMode::HttpProxy));
        assert_eq!(StoqAuthMode::parse("passthrough"), Some(StoqAuthMode::HttpProxy));
        assert_eq!(
            StoqAuthMode::parse("full-stoq-pos"),
            Some(StoqAuthMode::FullStoqPos),
        );
        assert_eq!(StoqAuthMode::parse("stoq"), Some(StoqAuthMode::FullStoqPos));
        assert_eq!(StoqAuthMode::parse("pos"), Some(StoqAuthMode::FullStoqPos));
        assert_eq!(StoqAuthMode::parse("garbage"), None);
    }

    #[test]
    fn stoq_auth_mode_as_str_round_trip() {
        for m in [StoqAuthMode::HttpProxy, StoqAuthMode::FullStoqPos] {
            assert_eq!(StoqAuthMode::parse(m.as_str()), Some(m));
        }
        assert!(StoqAuthMode::FullStoqPos.requires_pos_handshake());
        assert!(!StoqAuthMode::HttpProxy.requires_pos_handshake());
    }

    #[test]
    fn stoq_auth_mode_serde_round_trip() {
        let mode = StoqAuthMode::FullStoqPos;
        let json = serde_json::to_string(&mode).expect("test: serialize");
        assert_eq!(json, "\"full-stoq-pos\"");
        let back: StoqAuthMode = serde_json::from_str(&json).expect("test: deserialize");
        assert_eq!(back, mode);
    }

    #[test]
    fn from_file_without_stoq_auth_mode_defaults_to_http_proxy() {
        // Older config files omit stoq_auth_mode; #[serde(default)] must
        // keep them parseable and default to HttpProxy.
        let dir = tempfile::tempdir().expect("test: create temp dir");
        let file_path = dir.path().join("legacy-gateway.toml");
        let toml_content = r#"
listen_addr = "[::]:8443"
trustchain_addr = "[::1]:8444"
blockmatrix_addr = "[::1]:9292"
caesar_addr = "[::1]:9294"
catalog_addr = "[::1]:9295"
engauge_addr = "[::1]:9296"
trustchain_server_name = "tc"
blockmatrix_server_name = "bm"
caesar_server_name = "cs"
catalog_server_name = "cat"
engauge_server_name = "eng"
cert_path = "certs/server.crt"
key_path = "certs/server.key"
log_level = "info"
stoq_max_connections = 100

[pool]
max_connections = 10
idle_timeout = { secs = 300, nanos = 0 }
connect_timeout = { secs = 10, nanos = 0 }
keep_alive_interval = { secs = 30, nanos = 0 }

[retry]
max_attempts = 3
base_delay = { secs = 0, nanos = 100000000 }
max_delay = { secs = 5, nanos = 0 }
multiplier = 2.0

[cors]
allowed_origins = ["http://localhost:5173"]
allowed_methods = ["GET"]
allowed_headers = ["Content-Type"]
allow_credentials = true
max_age = 3600
"#;
        std::fs::write(&file_path, toml_content).expect("test: write config");
        let config = GatewayConfig::from_file(&file_path).expect("test: parse legacy config");
        assert_eq!(config.stoq_auth_mode, StoqAuthMode::HttpProxy);
    }

    #[test]
    fn from_file_parses_full_stoq_pos_mode() {
        let dir = tempfile::tempdir().expect("test: create temp dir");
        let file_path = dir.path().join("secure-gateway.toml");
        let toml_content = r#"
listen_addr = "[::]:8443"
trustchain_addr = "[::1]:8444"
blockmatrix_addr = "[::1]:9292"
caesar_addr = "[::1]:9294"
catalog_addr = "[::1]:9295"
engauge_addr = "[::1]:9296"
trustchain_server_name = "tc"
blockmatrix_server_name = "bm"
caesar_server_name = "cs"
catalog_server_name = "cat"
engauge_server_name = "eng"
cert_path = "certs/server.crt"
key_path = "certs/server.key"
log_level = "info"
stoq_max_connections = 100
stoq_auth_mode = "full-stoq-pos"

[pool]
max_connections = 10
idle_timeout = { secs = 300, nanos = 0 }
connect_timeout = { secs = 10, nanos = 0 }
keep_alive_interval = { secs = 30, nanos = 0 }

[retry]
max_attempts = 3
base_delay = { secs = 0, nanos = 100000000 }
max_delay = { secs = 5, nanos = 0 }
multiplier = 2.0

[cors]
allowed_origins = ["http://localhost:5173"]
allowed_methods = ["GET"]
allowed_headers = ["Content-Type"]
allow_credentials = true
max_age = 3600
"#;
        std::fs::write(&file_path, toml_content).expect("test: write config");
        let config = GatewayConfig::from_file(&file_path).expect("test: parse secure config");
        assert_eq!(config.stoq_auth_mode, StoqAuthMode::FullStoqPos);
        assert!(config.stoq_auth_mode.requires_pos_handshake());
    }

    #[test]
    fn default_path_contains_gateway_toml() {
        let path = GatewayConfig::default_path();
        assert!(path.ends_with("gateway.toml"));
        assert!(path.to_string_lossy().contains(".hypermesh"));
    }

    #[test]
    fn from_file_parses_toml() {
        let dir = tempfile::tempdir().expect("test: create temp dir");
        let file_path = dir.path().join("gateway.toml");

        let toml_content = r#"
listen_addr = "[::]:9999"
trustchain_addr = "[::1]:8444"
blockmatrix_addr = "[::1]:9292"
caesar_addr = "[::1]:9294"
catalog_addr = "[::1]:9295"
engauge_addr = "[::1]:9296"
trustchain_server_name = "tc"
blockmatrix_server_name = "bm"
caesar_server_name = "cs"
catalog_server_name = "cat"
engauge_server_name = "eng"
cert_path = "/tmp/cert.pem"
key_path = "/tmp/key.pem"
log_level = "debug"
stoq_max_connections = 50

[pool]
max_connections = 5
idle_timeout = { secs = 60, nanos = 0 }
connect_timeout = { secs = 5, nanos = 0 }
keep_alive_interval = { secs = 15, nanos = 0 }

[retry]
max_attempts = 2
base_delay = { secs = 0, nanos = 200000000 }
max_delay = { secs = 3, nanos = 0 }
multiplier = 1.5

[cors]
allowed_origins = ["http://localhost:3000"]
allowed_methods = ["GET", "POST"]
allowed_headers = ["Content-Type"]
allow_credentials = false
max_age = 1800
"#;
        let mut f = std::fs::File::create(&file_path).expect("test: create file");
        f.write_all(toml_content.as_bytes())
            .expect("test: write toml");

        let config = GatewayConfig::from_file(&file_path).expect("test: parse config");
        assert_eq!(config.listen_addr.port(), 9999);
        assert_eq!(config.log_level, "debug");
        assert_eq!(config.stoq_max_connections, 50);
        assert_eq!(config.pool.max_connections, 5);
        assert_eq!(config.retry.max_attempts, 2);
        assert!(!config.cors.allow_credentials);
    }

    #[test]
    fn from_file_missing_returns_error() {
        let result = GatewayConfig::from_file(Path::new("/tmp/nonexistent-gateway.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn load_returns_defaults_when_no_config_exists() {
        // Clear env vars that might interfere
        std::env::remove_var("GATEWAY_CONFIG");
        // load() will try ~/.hypermesh/gateway.toml which likely doesn't exist
        // in test env — should fall through to defaults
        let config = GatewayConfig::load().expect("test: load should succeed with defaults");
        assert_eq!(config.listen_addr.port(), 8443);
    }

    #[test]
    fn load_uses_gateway_config_env() {
        let dir = tempfile::tempdir().expect("test: create temp dir");
        let file_path = dir.path().join("custom-gateway.toml");

        // Write a minimal valid config
        let toml_content = format!(
            r#"
listen_addr = "[::]:7777"
trustchain_addr = "[::1]:8444"
blockmatrix_addr = "[::1]:9292"
caesar_addr = "[::1]:9294"
catalog_addr = "[::1]:9295"
engauge_addr = "[::1]:9296"
trustchain_server_name = "tc"
blockmatrix_server_name = "bm"
caesar_server_name = "cs"
catalog_server_name = "cat"
engauge_server_name = "eng"
cert_path = "certs/server.crt"
key_path = "certs/server.key"
log_level = "warn"
stoq_max_connections = 25

[pool]
max_connections = 10
idle_timeout = {{ secs = 300, nanos = 0 }}
connect_timeout = {{ secs = 10, nanos = 0 }}
keep_alive_interval = {{ secs = 30, nanos = 0 }}

[retry]
max_attempts = 3
base_delay = {{ secs = 0, nanos = 100000000 }}
max_delay = {{ secs = 5, nanos = 0 }}
multiplier = 2.0

[cors]
allowed_origins = ["http://localhost:5173"]
allowed_methods = ["GET", "POST", "PUT", "DELETE", "OPTIONS"]
allowed_headers = ["Content-Type", "Authorization", "X-Request-ID"]
allow_credentials = true
max_age = 3600
"#
        );
        std::fs::write(&file_path, toml_content).expect("test: write config");

        std::env::set_var("GATEWAY_CONFIG", file_path.to_str().expect("test: path"));
        let config = GatewayConfig::load().expect("test: load from GATEWAY_CONFIG");
        std::env::remove_var("GATEWAY_CONFIG");

        assert_eq!(config.listen_addr.port(), 7777);
        assert_eq!(config.log_level, "warn");
        assert_eq!(config.stoq_max_connections, 25);
    }
}
