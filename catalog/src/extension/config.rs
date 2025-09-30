//! Extension Configuration for Catalog
//!
//! This module defines the configuration structures for the Catalog extension,
//! including settings for library management, consensus validation, and resource limits.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Main configuration for the Catalog extension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogExtensionConfig {
    /// Path to the asset library storage
    pub library_path: PathBuf,

    /// Maximum cache size in bytes
    pub cache_size: u64,

    /// Maximum package size in bytes
    pub max_package_size: u64,

    /// Enable P2P distribution via STOQ
    pub enable_p2p: bool,

    /// Enable consensus validation for operations
    pub consensus_validation: bool,

    /// HyperMesh network address
    pub hypermesh_address: String,

    /// TrustChain certificate path
    pub trustchain_cert_path: Option<String>,

    /// TrustChain certificate fingerprint for verification
    pub certificate_fingerprint: Option<String>,

    /// Maximum memory usage in bytes
    pub max_memory_usage: u64,

    /// Maximum concurrent operations
    pub max_concurrent_ops: usize,

    /// Enable debug mode
    pub debug_mode: bool,

    /// Asset indexing configuration
    pub indexing: IndexingConfig,

    /// Security configuration
    pub security: SecurityConfig,

    /// Performance tuning
    pub performance: PerformanceConfig,
}

impl Default for CatalogExtensionConfig {
    fn default() -> Self {
        Self {
            library_path: PathBuf::from("./catalog-library"),
            cache_size: 1024 * 1024 * 1024, // 1GB
            max_package_size: 100 * 1024 * 1024, // 100MB
            enable_p2p: true,
            consensus_validation: true,
            hypermesh_address: "catalog.hypermesh.online".to_string(),
            trustchain_cert_path: None,
            certificate_fingerprint: None,
            max_memory_usage: 2 * 1024 * 1024 * 1024, // 2GB
            max_concurrent_ops: 100,
            debug_mode: false,
            indexing: IndexingConfig::default(),
            security: SecurityConfig::default(),
            performance: PerformanceConfig::default(),
        }
    }
}

impl CatalogExtensionConfig {
    /// Create a new configuration with custom settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder pattern for library path
    pub fn with_library_path(mut self, path: PathBuf) -> Self {
        self.library_path = path;
        self
    }

    /// Builder pattern for cache size
    pub fn with_cache_size(mut self, size: u64) -> Self {
        self.cache_size = size;
        self
    }

    /// Builder pattern for P2P distribution
    pub fn with_p2p(mut self, enable: bool) -> Self {
        self.enable_p2p = enable;
        self
    }

    /// Builder pattern for consensus validation
    pub fn with_consensus_validation(mut self, enable: bool) -> Self {
        self.consensus_validation = enable;
        self
    }

    /// Builder pattern for HyperMesh address
    pub fn with_hypermesh_address(mut self, address: String) -> Self {
        self.hypermesh_address = address;
        self
    }

    /// Builder pattern for TrustChain certificate
    pub fn with_trustchain_cert(mut self, cert_path: String) -> Self {
        self.trustchain_cert_path = Some(cert_path);
        self
    }

    /// Apply settings from ExtensionSettings
    pub fn apply_settings(&mut self, settings: ExtensionSettings) {
        if let Some(path) = settings.library_path {
            self.library_path = path;
        }
        if let Some(size) = settings.cache_size {
            self.cache_size = size;
        }
        if let Some(size) = settings.max_package_size {
            self.max_package_size = size;
        }
        if let Some(enable) = settings.enable_p2p {
            self.enable_p2p = enable;
        }
        if let Some(enable) = settings.consensus_validation {
            self.consensus_validation = enable;
        }
        if let Some(debug) = settings.debug_mode {
            self.debug_mode = debug;
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Check library path
        if !self.library_path.exists() {
            return Err(ConfigError::InvalidPath(
                format!("Library path does not exist: {:?}", self.library_path)
            ));
        }

        // Check cache size
        if self.cache_size == 0 {
            return Err(ConfigError::InvalidValue(
                "Cache size must be greater than 0".to_string()
            ));
        }

        // Check max package size
        if self.max_package_size == 0 {
            return Err(ConfigError::InvalidValue(
                "Max package size must be greater than 0".to_string()
            ));
        }

        // Check memory limit
        if self.max_memory_usage < self.cache_size {
            return Err(ConfigError::InvalidValue(
                "Max memory usage must be greater than cache size".to_string()
            ));
        }

        Ok(())
    }
}

/// Extension-specific settings passed during initialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionSettings {
    /// Library path override
    pub library_path: Option<PathBuf>,

    /// Cache size override
    pub cache_size: Option<u64>,

    /// Max package size override
    pub max_package_size: Option<u64>,

    /// P2P distribution toggle
    pub enable_p2p: Option<bool>,

    /// Consensus validation toggle
    pub consensus_validation: Option<bool>,

    /// Debug mode toggle
    pub debug_mode: Option<bool>,

    /// Additional custom settings
    pub custom: Option<serde_json::Value>,
}

/// Asset indexing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
    /// Enable automatic indexing
    pub auto_index: bool,

    /// Indexing interval in seconds
    pub index_interval: u64,

    /// Maximum index size in bytes
    pub max_index_size: u64,

    /// Enable full-text search
    pub enable_full_text: bool,

    /// Index compression
    pub compress_index: bool,
}

impl Default for IndexingConfig {
    fn default() -> Self {
        Self {
            auto_index: true,
            index_interval: 300, // 5 minutes
            max_index_size: 100 * 1024 * 1024, // 100MB
            enable_full_text: true,
            compress_index: true,
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Verify package signatures
    pub verify_signatures: bool,

    /// Require consensus for all operations
    pub require_consensus: bool,

    /// Minimum consensus proofs required
    pub min_consensus_proofs: u8,

    /// Enable sandboxing for execution
    pub enable_sandbox: bool,

    /// Allowed execution languages
    pub allowed_languages: Vec<String>,

    /// Blocked package patterns
    pub blocked_patterns: Vec<String>,

    /// Maximum execution time in seconds
    pub max_execution_time: u64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            verify_signatures: true,
            require_consensus: false,
            min_consensus_proofs: 2, // At least 2 of 4 proofs
            enable_sandbox: true,
            allowed_languages: vec![
                "julia".to_string(),
                "python".to_string(),
                "wasm".to_string(),
                "javascript".to_string(),
            ],
            blocked_patterns: vec![],
            max_execution_time: 300, // 5 minutes
        }
    }
}

/// Performance tuning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Thread pool size for async operations
    pub thread_pool_size: usize,

    /// Maximum concurrent downloads
    pub max_concurrent_downloads: usize,

    /// Maximum concurrent uploads
    pub max_concurrent_uploads: usize,

    /// Connection pool size
    pub connection_pool_size: usize,

    /// Request timeout in seconds
    pub request_timeout: u64,

    /// Enable caching
    pub enable_cache: bool,

    /// Cache TTL in seconds
    pub cache_ttl: u64,

    /// Enable compression for network transfers
    pub enable_compression: bool,

    /// Compression level (1-9)
    pub compression_level: u8,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            thread_pool_size: 4,
            max_concurrent_downloads: 10,
            max_concurrent_uploads: 5,
            connection_pool_size: 20,
            request_timeout: 30,
            enable_cache: true,
            cache_ttl: 3600, // 1 hour
            enable_compression: true,
            compression_level: 6,
        }
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Invalid path
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// Invalid value
    #[error("Invalid value: {0}")]
    InvalidValue(String),

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Configuration conflict
    #[error("Configuration conflict: {0}")]
    Conflict(String),
}

/// Environment-based configuration loader
pub struct ConfigLoader;

impl ConfigLoader {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<CatalogExtensionConfig, ConfigError> {
        let mut config = CatalogExtensionConfig::default();

        // Load from environment variables with CATALOG_ prefix
        if let Ok(path) = std::env::var("CATALOG_LIBRARY_PATH") {
            config.library_path = PathBuf::from(path);
        }

        if let Ok(size) = std::env::var("CATALOG_CACHE_SIZE") {
            config.cache_size = size.parse()
                .map_err(|_| ConfigError::InvalidValue("Invalid cache size".to_string()))?;
        }

        if let Ok(enable) = std::env::var("CATALOG_ENABLE_P2P") {
            config.enable_p2p = enable.parse()
                .map_err(|_| ConfigError::InvalidValue("Invalid P2P setting".to_string()))?;
        }

        if let Ok(enable) = std::env::var("CATALOG_CONSENSUS_VALIDATION") {
            config.consensus_validation = enable.parse()
                .map_err(|_| ConfigError::InvalidValue("Invalid consensus setting".to_string()))?;
        }

        if let Ok(address) = std::env::var("CATALOG_HYPERMESH_ADDRESS") {
            config.hypermesh_address = address;
        }

        if let Ok(cert) = std::env::var("CATALOG_TRUSTCHAIN_CERT") {
            config.trustchain_cert_path = Some(cert);
        }

        config.validate()?;
        Ok(config)
    }

    /// Load configuration from file
    pub fn from_file(path: &PathBuf) -> Result<CatalogExtensionConfig, ConfigError> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::InvalidPath(format!("Cannot read config file: {}", e)))?;

        let config: CatalogExtensionConfig = toml::from_str(&contents)
            .map_err(|e| ConfigError::InvalidValue(format!("Invalid TOML: {}", e)))?;

        config.validate()?;
        Ok(config)
    }

    /// Load with fallback to defaults
    pub fn load() -> CatalogExtensionConfig {
        // Try environment first
        if let Ok(config) = Self::from_env() {
            return config;
        }

        // Try default config file
        let default_path = PathBuf::from("catalog-extension.toml");
        if default_path.exists() {
            if let Ok(config) = Self::from_file(&default_path) {
                return config;
            }
        }

        // Fall back to defaults
        CatalogExtensionConfig::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CatalogExtensionConfig::default();
        assert_eq!(config.cache_size, 1024 * 1024 * 1024);
        assert!(config.enable_p2p);
        assert!(config.consensus_validation);
    }

    #[test]
    fn test_config_builder() {
        let config = CatalogExtensionConfig::new()
            .with_library_path(PathBuf::from("/tmp/catalog"))
            .with_cache_size(512 * 1024 * 1024)
            .with_p2p(false)
            .with_consensus_validation(false);

        assert_eq!(config.library_path, PathBuf::from("/tmp/catalog"));
        assert_eq!(config.cache_size, 512 * 1024 * 1024);
        assert!(!config.enable_p2p);
        assert!(!config.consensus_validation);
    }

    #[test]
    fn test_indexing_config_default() {
        let config = IndexingConfig::default();
        assert!(config.auto_index);
        assert_eq!(config.index_interval, 300);
        assert!(config.enable_full_text);
    }

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert!(config.verify_signatures);
        assert_eq!(config.min_consensus_proofs, 2);
        assert!(config.enable_sandbox);
        assert_eq!(config.allowed_languages.len(), 4);
    }

    #[test]
    fn test_performance_config_default() {
        let config = PerformanceConfig::default();
        assert_eq!(config.thread_pool_size, 4);
        assert_eq!(config.max_concurrent_downloads, 10);
        assert!(config.enable_cache);
        assert!(config.enable_compression);
    }
}