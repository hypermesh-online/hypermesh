//! Container runtime configuration

use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::time::Duration;

/// Container runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    /// Runtime configuration
    pub runtime: RuntimeConfig,
    /// Storage configuration
    pub storage_usage: StorageConfig,
    /// Network configuration
    pub network_usage: NetworkConfig,
    /// Security configuration
    pub security: SecurityConfig,
    /// Resource limits
    pub limits: ResourceLimits,
    /// Monitoring configuration
    pub monitoring: MonitoringConfig,
}

/// Runtime-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Runtime name
    pub name: String,
    /// Runtime version
    pub version: String,
    /// Data directory
    pub data_dir: PathBuf,
    /// Enable hardware virtualization
    pub hardware_virtualization: bool,
    /// Memory protection enabled
    pub memory_protection: bool,
    /// CPU isolation enabled
    pub cpu_isolation: bool,
    /// Maximum containers
    pub max_containers: u32,
    /// Container startup timeout
    pub startup_timeout: Duration,
    /// Container shutdown timeout
    pub shutdown_timeout: Duration,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage driver (overlay2, zfs, etc.)
    pub driver: String,
    /// Root storage path
    pub root: PathBuf,
    /// Image storage path
    pub images: PathBuf,
    /// Container storage path
    pub containers: PathBuf,
    /// Temporary directory
    pub tmp_dir: PathBuf,
    /// Enable compression
    pub compression: bool,
    /// Compression algorithm
    pub compression_algo: CompressionAlgorithm,
    /// Enable deduplication
    pub deduplication: bool,
    /// Maximum layer size
    pub max_layer_size: u64,
    /// Garbage collection policy
    pub gc_policy: GarbageCollectionPolicy,
}

/// Network configuration for runtime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Enable networking
    pub enabled: bool,
    /// Default network mode
    pub default_mode: String,
    /// Bridge name for bridge networking
    pub bridge_name: String,
    /// IP address range for containers
    pub ip_range: String,
    /// DNS servers
    pub dns_servers: Vec<String>,
    /// Enable NAT
    pub enable_nat: bool,
    /// Enable IPv6
    pub enable_ipv6: bool,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable capability-based security
    pub capabilities_enabled: bool,
    /// Default capability set
    pub default_capabilities: Vec<String>,
    /// Prohibited capabilities
    pub prohibited_capabilities: Vec<String>,
    /// Enable AppArmor
    pub apparmor_enabled: bool,
    /// Default AppArmor profile
    pub default_apparmor_profile: String,
    /// Enable SELinux
    pub selinux_enabled: bool,
    /// Default SELinux context
    pub default_selinux_context: String,
    /// Enable seccomp
    pub seccomp_enabled: bool,
    /// Default seccomp profile
    pub default_seccomp_profile: String,
}

/// Resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory per container
    pub max_memory_per_container: u64,
    /// Maximum CPU per container
    pub max_cpu_per_container: f64,
    /// Maximum network bandwidth
    pub max_network_bandwidth: u64,
    /// Maximum filesystem size
    pub max_filesystem_size: u64,
    /// Maximum file descriptors
    pub max_file_descriptors: u32,
    /// Maximum processes per container
    pub max_processes: u32,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable monitoring
    pub enabled: bool,
    /// Metrics collection interval
    pub collection_interval: Duration,
    /// Prometheus metrics port
    pub metrics_port: u16,
    /// Enable performance profiling
    pub profiling_enabled: bool,
    /// Log level
    pub log_level: String,
    /// Health check interval
    pub health_check_interval: Duration,
}

/// Compression algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    /// No compression
    None,
    /// GZIP compression
    Gzip,
    /// ZSTD compression
    Zstd,
    /// LZ4 compression
    Lz4,
}

/// Garbage collection policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GarbageCollectionPolicy {
    /// Maximum age for unused images
    pub max_age: Duration,
    /// Maximum total size
    pub max_size: u64,
    /// Minimum free space to maintain
    pub min_free_space: u64,
    /// Enable automatic cleanup
    pub auto_cleanup: bool,
    /// Cleanup interval
    pub cleanup_interval: Duration,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            runtime: RuntimeConfig::default(),
            storage_usage: StorageConfig::default(),
            network_usage: NetworkConfig::default(),
            security: SecurityConfig::default(),
            limits: ResourceLimits::default(),
            monitoring: MonitoringConfig::default(),
        }
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            name: "hypermesh-runtime".to_string(),
            version: "1.0".to_string(),
            data_dir: PathBuf::from("/var/lib/hypermesh"),
            hardware_virtualization: true,
            memory_protection: true,
            cpu_isolation: true,
            max_containers: 1000,
            startup_timeout: Duration::from_millis(100),
            shutdown_timeout: Duration::from_secs(5),
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            driver: "overlay2".to_string(),
            root: PathBuf::from("/var/lib/hypermesh/storage"),
            images: PathBuf::from("/var/lib/hypermesh/images"),
            containers: PathBuf::from("/var/lib/hypermesh/containers"),
            tmp_dir: PathBuf::from("/tmp/hypermesh"),
            compression: true,
            compression_algo: CompressionAlgorithm::Zstd,
            deduplication: true,
            max_layer_size: 1024 * 1024 * 1024, // 1GB
            gc_policy: GarbageCollectionPolicy::default(),
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_mode: "bridge".to_string(),
            bridge_name: "hypermesh0".to_string(),
            ip_range: "172.17.0.0/16".to_string(),
            dns_servers: vec!["8.8.8.8".to_string(), "8.8.4.4".to_string()],
            enable_nat: true,
            enable_ipv6: true,
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            capabilities_enabled: true,
            default_capabilities: vec![
                "CAP_NET_BIND_SERVICE".to_string(),
                "CAP_SETUID".to_string(),
                "CAP_SETGID".to_string(),
            ],
            prohibited_capabilities: vec![
                "CAP_SYS_MODULE".to_string(),
                "CAP_SYS_RAWIO".to_string(),
                "CAP_SYS_ADMIN".to_string(),
            ],
            apparmor_enabled: true,
            default_apparmor_profile: "hypermesh-default".to_string(),
            selinux_enabled: true,
            default_selinux_context: "container_t".to_string(),
            seccomp_enabled: true,
            default_seccomp_profile: "hypermesh-default".to_string(),
        }
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_per_container: 8 * 1024 * 1024 * 1024, // 8GB
            max_cpu_per_container: 4.0,
            max_network_bandwidth: 1024 * 1024 * 1024, // 1Gbps
            max_filesystem_size: 100 * 1024 * 1024 * 1024, // 100GB
            max_file_descriptors: 65536,
            max_processes: 1024,
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            collection_interval: Duration::from_secs(10),
            metrics_port: 9090,
            profiling_enabled: false,
            log_level: "info".to_string(),
            health_check_interval: Duration::from_secs(30),
        }
    }
}

impl Default for GarbageCollectionPolicy {
    fn default() -> Self {
        Self {
            max_age: Duration::from_secs(30 * 24 * 3600), // 30 days
            max_size: 100 * 1024 * 1024 * 1024, // 100GB
            min_free_space: 10 * 1024 * 1024 * 1024, // 10GB
            auto_cleanup: true,
            cleanup_interval: Duration::from_secs(24 * 3600), // 24 hours
        }
    }
}