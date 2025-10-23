//! Runtime configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Complete runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Image management configuration
    pub image: super::image::ImageConfig,
    
    /// Isolation configuration
    pub isolation: IsolationConfig,
    
    /// Resource management configuration
    pub resources: ResourceConfig,
    
    /// Networking configuration
    pub networking: NetworkingConfig,
    
    /// Storage configuration
    pub storage: StorageConfig,
    
    /// Security configuration
    pub security: SecurityConfig,
    
    /// Logging configuration
    pub logging: LoggingConfig,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            image: super::image::ImageConfig::default(),
            isolation: IsolationConfig::default(),
            resources: ResourceConfig::default(),
            networking: NetworkingConfig::default(),
            storage: StorageConfig::default(),
            security: SecurityConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

/// Isolation configuration for containers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolationConfig {
    /// Enable namespace isolation
    pub enable_namespaces: bool,
    
    /// Namespace types to enable
    pub namespace_types: Vec<NamespaceType>,
    
    /// Enable cgroup resource limits
    pub enable_cgroups: bool,
    
    /// Cgroup version (v1 or v2)
    pub cgroup_version: CgroupVersion,
    
    /// Enable seccomp filtering
    pub enable_seccomp: bool,
    
    /// Default seccomp profile
    pub default_seccomp_profile: String,
    
    /// Enable AppArmor/SELinux
    pub enable_mandatory_access_control: bool,
}

impl Default for IsolationConfig {
    fn default() -> Self {
        Self {
            enable_namespaces: true,
            namespace_types: vec![
                NamespaceType::Pid,
                NamespaceType::Net,
                NamespaceType::Ipc,
                NamespaceType::Uts,
                NamespaceType::Mount,
                NamespaceType::User,
            ],
            enable_cgroups: true,
            cgroup_version: CgroupVersion::V2,
            enable_seccomp: true,
            default_seccomp_profile: "default".to_string(),
            enable_mandatory_access_control: true,
        }
    }
}

/// Namespace types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NamespaceType {
    Pid,
    Net,
    Ipc,
    Uts,
    Mount,
    User,
    Cgroup,
    Time,
}

/// Cgroup version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CgroupVersion {
    V1,
    V2,
}

/// Resource management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    /// Default CPU limit (cores)
    pub default_cpu_limit: f64,
    
    /// Default memory limit (MB)
    pub default_memory_limit: u64,
    
    /// Default storage limit (GB)
    pub default_storage_limit: f64,
    
    /// Enable resource monitoring
    pub enable_monitoring: bool,
    
    /// Monitoring interval
    pub monitoring_interval_seconds: u64,
    
    /// Enable OOM killer
    pub enable_oom_killer: bool,
    
    /// Resource accounting precision
    pub accounting_precision: AccountingPrecision,
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            default_cpu_limit: 1.0,
            default_memory_limit: 512,
            default_storage_limit: 10.0,
            enable_monitoring: true,
            monitoring_interval_seconds: 10,
            enable_oom_killer: true,
            accounting_precision: AccountingPrecision::High,
        }
    }
}

/// Resource accounting precision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccountingPrecision {
    Low,
    Medium,
    High,
    Realtime,
}

/// Networking configuration for containers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkingConfig {
    /// Default network mode
    pub default_network_mode: NetworkMode,
    
    /// Enable port mapping
    pub enable_port_mapping: bool,
    
    /// Default bridge name
    pub default_bridge: String,
    
    /// Enable network isolation
    pub enable_isolation: bool,
    
    /// DNS servers
    pub dns_servers: Vec<String>,
    
    /// Search domains
    pub search_domains: Vec<String>,
    
    /// Enable IPv6
    pub enable_ipv6: bool,
    
    /// MTU size
    pub mtu: u16,
}

impl Default for NetworkingConfig {
    fn default() -> Self {
        Self {
            default_network_mode: NetworkMode::Bridge,
            enable_port_mapping: true,
            default_bridge: "nexus0".to_string(),
            enable_isolation: true,
            dns_servers: vec!["1.1.1.1".to_string(), "8.8.8.8".to_string()],
            search_domains: vec!["local".to_string()],
            enable_ipv6: true,
            mtu: 1500,
        }
    }
}

/// Network modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMode {
    Bridge,
    Host,
    None,
    Container(String),
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Data directory for container storage
    pub data_dir: String,
    
    /// Root filesystem path
    pub rootfs_path: String,
    
    /// Default storage driver
    pub default_driver: StorageDriver,
    
    /// Enable storage encryption
    pub enable_encryption: bool,
    
    /// Enable deduplication
    pub enable_deduplication: bool,
    
    /// Enable compression
    pub enable_compression: bool,
    
    /// Compression algorithm
    pub compression_algorithm: CompressionAlgorithm,
    
    /// Maximum container size (GB)
    pub max_container_size_gb: f64,
    
    /// Enable quotas
    pub enable_quotas: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: "./data/containers".to_string(),
            rootfs_path: "./data/rootfs".to_string(),
            default_driver: StorageDriver::Overlay2,
            enable_encryption: true,
            enable_deduplication: true,
            enable_compression: true,
            compression_algorithm: CompressionAlgorithm::Zstd,
            max_container_size_gb: 100.0,
            enable_quotas: true,
        }
    }
}

/// Storage drivers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageDriver {
    Overlay2,
    Btrfs,
    Zfs,
    DeviceMapper,
}

/// Compression algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    None,
    Gzip,
    Lz4,
    Zstd,
    Brotli,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Default user ID for containers
    pub default_user_id: u32,
    
    /// Default group ID for containers
    pub default_group_id: u32,
    
    /// Enable rootless mode
    pub rootless_mode: bool,
    
    /// Allow privileged containers
    pub allow_privileged: bool,
    
    /// Default capabilities to drop
    pub default_cap_drop: Vec<String>,
    
    /// Default capabilities to add
    pub default_cap_add: Vec<String>,
    
    /// Enable no-new-privileges
    pub no_new_privileges: bool,
    
    /// Default AppArmor profile
    pub default_apparmor_profile: Option<String>,
    
    /// Default SELinux context
    pub default_selinux_context: Option<String>,
    
    /// Enable user namespaces
    pub enable_user_namespaces: bool,
    
    /// Trust policy for images
    pub image_trust_policy: ImageTrustPolicy,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            default_user_id: 1000,
            default_group_id: 1000,
            rootless_mode: false,
            allow_privileged: false,
            default_cap_drop: vec![
                "ALL".to_string(),
            ],
            default_cap_add: vec![
                "CHOWN".to_string(),
                "DAC_OVERRIDE".to_string(),
                "FSETID".to_string(),
                "FOWNER".to_string(),
                "SETGID".to_string(),
                "SETUID".to_string(),
                "SETPCAP".to_string(),
                "NET_BIND_SERVICE".to_string(),
                "KILL".to_string(),
            ],
            no_new_privileges: true,
            default_apparmor_profile: Some("docker-default".to_string()),
            default_selinux_context: None,
            enable_user_namespaces: true,
            image_trust_policy: ImageTrustPolicy::RequireSigned,
        }
    }
}

/// Image trust policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImageTrustPolicy {
    /// Accept any image
    AcceptAll,
    /// Require signed images
    RequireSigned,
    /// Require images from trusted registries only
    TrustedRegistriesOnly,
    /// Require both signing and trusted registry
    RequireSignedFromTrustedRegistry,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Default log driver
    pub default_driver: LogDriver,
    
    /// Log level
    pub level: String,
    
    /// Enable structured logging
    pub structured_logging: bool,
    
    /// Log rotation settings
    pub rotation: LogRotationConfig,
    
    /// Enable log aggregation
    pub enable_aggregation: bool,
    
    /// Aggregation endpoint
    pub aggregation_endpoint: Option<String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            default_driver: LogDriver::JsonFile,
            level: "info".to_string(),
            structured_logging: true,
            rotation: LogRotationConfig::default(),
            enable_aggregation: false,
            aggregation_endpoint: None,
        }
    }
}

/// Log drivers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogDriver {
    JsonFile,
    Journald,
    Syslog,
    Fluentd,
    None,
}

/// Log rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRotationConfig {
    /// Maximum log file size in MB
    pub max_file_size_mb: u64,
    
    /// Maximum number of log files
    pub max_files: u32,
    
    /// Enable compression of rotated logs
    pub compress: bool,
}

impl Default for LogRotationConfig {
    fn default() -> Self {
        Self {
            max_file_size_mb: 100,
            max_files: 5,
            compress: true,
        }
    }
}

impl RuntimeConfig {
    /// Load configuration from file
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: RuntimeConfig = toml::from_str(&content)?;
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
        // Validate resource limits
        if self.resources.default_cpu_limit <= 0.0 {
            return Err("Default CPU limit must be greater than 0".to_string());
        }
        
        if self.resources.default_memory_limit == 0 {
            return Err("Default memory limit must be greater than 0".to_string());
        }
        
        // Validate storage configuration
        if self.storage.max_container_size_gb <= 0.0 {
            return Err("Maximum container size must be greater than 0".to_string());
        }
        
        // Validate security configuration
        if self.security.allow_privileged && self.security.rootless_mode {
            return Err("Cannot enable privileged mode in rootless mode".to_string());
        }
        
        Ok(())
    }
    
    /// Get configuration for development environment
    pub fn development() -> Self {
        let mut config = Self::default();
        
        // Relax security for development
        config.security.allow_privileged = true;
        config.security.rootless_mode = false;
        config.security.image_trust_policy = ImageTrustPolicy::AcceptAll;
        
        // Enable more logging
        config.logging.level = "debug".to_string();
        
        // Smaller resource limits
        config.resources.default_cpu_limit = 0.5;
        config.resources.default_memory_limit = 256;
        
        config
    }
    
    /// Get configuration for production environment
    pub fn production() -> Self {
        let mut config = Self::default();
        
        // Enhanced security for production
        config.security.allow_privileged = false;
        config.security.rootless_mode = true;
        config.security.image_trust_policy = ImageTrustPolicy::RequireSignedFromTrustedRegistry;
        
        // Production logging
        config.logging.level = "warn".to_string();
        config.logging.enable_aggregation = true;
        
        // Higher resource limits
        config.resources.default_cpu_limit = 2.0;
        config.resources.default_memory_limit = 2048;
        
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = RuntimeConfig::default();
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_development_config() {
        let config = RuntimeConfig::development();
        assert!(config.validate().is_ok());
        assert!(config.security.allow_privileged);
    }
    
    #[test]
    fn test_production_config() {
        let config = RuntimeConfig::production();
        assert!(config.validate().is_ok());
        assert!(!config.security.allow_privileged);
        assert!(config.security.rootless_mode);
    }
    
    #[test]
    fn test_config_serialization() {
        let config = RuntimeConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: RuntimeConfig = toml::from_str(&toml_str).unwrap();
        
        assert_eq!(config.resources.default_cpu_limit, parsed.resources.default_cpu_limit);
    }
    
    #[test]
    fn test_invalid_config() {
        let mut config = RuntimeConfig::default();
        config.resources.default_cpu_limit = -1.0;
        assert!(config.validate().is_err());
    }
}