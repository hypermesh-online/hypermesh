//! Configuration for Internet 2.0 Protocol Stack
//! 
//! Unified configuration system that coordinates all three layers:
//! - STOQ Transport configuration (QUIC, performance, networking)
//! - HyperMesh Assets configuration (consensus, asset management, VM execution) 
//! - TrustChain Authority configuration (CA, DNS, certificate transparency)

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::net::Ipv6Addr;
use std::path::Path;
use std::time::Duration;

/// Master configuration for Internet 2.0 protocol stack
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Internet2Config {
    /// Global server configuration
    pub global: GlobalConfig,
    
    /// STOQ Transport layer configuration
    pub stoq: StoqConfig,
    
    /// HyperMesh Assets layer configuration  
    pub hypermesh: HyperMeshConfig,
    
    /// TrustChain Authority layer configuration
    pub trustchain: TrustChainConfig,
    
    /// Integration and performance configuration
    pub integration: IntegrationConfig,
    
    /// Deployment-specific settings
    pub deployment: DeploymentConfig,
}

/// Global server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Server bind address (IPv6 only)
    pub bind_address: Ipv6Addr,
    
    /// Server port
    pub port: u16,
    
    /// Server identifier
    pub server_id: String,
    
    /// Maximum concurrent connections
    pub max_connections: u32,
    
    /// Enable IPv6-only networking (IPv4 explicitly disabled)
    pub ipv6_only: bool,
    
    /// Logging configuration
    pub log_level: String,
    
    /// Metrics collection interval
    pub metrics_interval: Duration,
}

/// STOQ Transport layer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoqConfig {
    /// Performance optimization settings
    pub performance: StoqPerformanceConfig,
    
    /// QUIC transport settings
    pub quic: QuicConfig,
    
    /// Certificate integration settings
    pub certificates: StoqCertificateConfig,
    
    /// DNS integration settings
    pub dns: StoqDnsConfig,
}

/// STOQ performance configuration for 40 Gbps target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoqPerformanceConfig {
    /// Target throughput in Gbps
    pub target_throughput_gbps: f64,
    
    /// Enable zero-copy operations
    pub enable_zero_copy: bool,
    
    /// Enable hardware acceleration
    pub enable_hardware_acceleration: bool,
    
    /// Connection pool size for multiplexing
    pub connection_pool_size: usize,
    
    /// Memory pool size for zero-copy operations
    pub memory_pool_size: usize,
    
    /// Frame batching size for syscall reduction
    pub frame_batch_size: usize,
    
    /// Enable CPU affinity for network threads
    pub enable_cpu_affinity: bool,
    
    /// Enable large send offload
    pub enable_large_send_offload: bool,
}

/// QUIC transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuicConfig {
    /// Maximum concurrent streams per connection
    pub max_concurrent_streams: u32,
    
    /// Send buffer size
    pub send_buffer_size: usize,
    
    /// Receive buffer size
    pub receive_buffer_size: usize,
    
    /// Connection timeout
    pub connection_timeout: Duration,
    
    /// Idle timeout
    pub idle_timeout: Duration,
    
    /// Enable 0-RTT resumption
    pub enable_0rtt: bool,
    
    /// Enable connection migration
    pub enable_migration: bool,
    
    /// Congestion control algorithm
    pub congestion_control: CongestionControl,
    
    /// Maximum datagram size
    pub max_datagram_size: usize,
}

/// Congestion control algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CongestionControl {
    Bbr2,
    Cubic,
    NewReno,
}

/// STOQ certificate integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoqCertificateConfig {
    /// Validate certificates at connection establishment
    pub validate_at_connection: bool,
    
    /// Certificate validation timeout
    pub validation_timeout: Duration,
    
    /// Enable certificate caching
    pub enable_caching: bool,
    
    /// Certificate cache size
    pub cache_size: usize,
    
    /// Certificate cache TTL
    pub cache_ttl: Duration,
}

/// STOQ DNS integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoqDnsConfig {
    /// Use embedded DNS resolver
    pub use_embedded_resolver: bool,
    
    /// DNS query timeout
    pub query_timeout: Duration,
    
    /// Enable DNS caching
    pub enable_caching: bool,
    
    /// DNS cache size
    pub cache_size: usize,
    
    /// DNS cache TTL
    pub cache_ttl: Duration,
}

/// HyperMesh Assets layer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperMeshConfig {
    /// Consensus configuration
    pub consensus: ConsensusConfig,
    
    /// Asset management configuration
    pub assets: AssetConfig,
    
    /// VM execution configuration
    pub vm: VmConfig,
    
    /// Proxy and NAT configuration
    pub proxy: ProxyConfig,
}

/// Consensus configuration for four-proof validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Require four-proof consensus (PoSpace+PoStake+PoWork+PoTime)
    pub mandatory_four_proof: bool,
    
    /// Consensus validation timeout
    pub validation_timeout: Duration,
    
    /// Minimum stake requirement for PoStake
    pub min_stake_requirement: u64,
    
    /// Proof of Work difficulty
    pub pow_difficulty: u32,
    
    /// Enable Byzantine fault detection
    pub enable_byzantine_detection: bool,
    
    /// Maximum consensus participants
    pub max_consensus_participants: u32,
}

/// Asset management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetConfig {
    /// Maximum assets per node
    pub max_assets_per_node: u32,
    
    /// Asset allocation timeout
    pub allocation_timeout: Duration,
    
    /// Enable asset pooling
    pub enable_pooling: bool,
    
    /// Asset pool size
    pub pool_size: usize,
    
    /// Asset cleanup interval
    pub cleanup_interval: Duration,
    
    /// Default resource capacity for pools
    pub default_resource_capacity: f64,
    
    /// Require consensus for allocation operations
    pub require_consensus_for_allocation: bool,
}

/// VM execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    /// Enable VM execution
    pub enable_vm_execution: bool,
    
    /// Maximum VMs per node
    pub max_vms_per_node: u32,
    
    /// VM execution timeout
    pub execution_timeout: Duration,
    
    /// VM memory limit
    pub memory_limit_mb: u32,
    
    /// VM CPU limit
    pub cpu_limit_cores: u32,
    
    /// Enable VM snapshots
    pub enable_snapshots: bool,
}

/// Proxy and NAT configuration for remote addressing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Enable NAT-like addressing
    pub enable_nat_addressing: bool,
    
    /// Proxy connection timeout
    pub connection_timeout: Duration,
    
    /// Maximum proxy connections
    pub max_proxy_connections: u32,
    
    /// Proxy trust validation
    pub enable_trust_validation: bool,
    
    /// Proxy performance monitoring
    pub enable_performance_monitoring: bool,
}

/// TrustChain Authority layer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustChainConfig {
    /// Certificate Authority configuration
    pub ca: CaConfig,
    
    /// DNS resolver configuration
    pub dns: DnsConfig,
    
    /// Certificate Transparency configuration
    pub ct: CtConfig,
    
    /// Post-quantum cryptography configuration
    pub pqc: PqcConfig,
}

/// Certificate Authority configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaConfig {
    /// CA mode (embedded vs external)
    pub ca_mode: CaMode,
    
    /// Certificate validity period
    pub certificate_validity_days: u32,
    
    /// Certificate rotation interval
    pub rotation_interval: Duration,
    
    /// Enable automatic rotation
    pub enable_auto_rotation: bool,
    
    /// CA certificate chain depth
    pub max_chain_depth: u32,
}

/// Certificate Authority modes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaMode {
    Embedded,   // Embedded CA (Internet 2.0)
    External,   // External CA (Internet 1.0 compatibility)
    Hybrid,     // Both embedded and external
}

/// DNS resolver configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    /// DNS mode (embedded vs external)
    pub dns_mode: DnsMode,
    
    /// DNS server port
    pub dns_port: u16,
    
    /// DNS query timeout
    pub query_timeout: Duration,
    
    /// Enable DNS caching
    pub enable_caching: bool,
    
    /// DNS cache TTL
    pub cache_ttl: Duration,
    
    /// Maximum DNS cache size
    pub max_cache_size: usize,
}

/// DNS resolver modes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DnsMode {
    Embedded,   // Embedded DNS (Internet 2.0)
    External,   // External DNS (Internet 1.0 compatibility)
    Hybrid,     // Both embedded and external
}

/// Certificate Transparency configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CtConfig {
    /// Enable Certificate Transparency logging
    pub enable_ct_logging: bool,
    
    /// CT log submission timeout
    pub submission_timeout: Duration,
    
    /// CT log verification
    pub enable_verification: bool,
    
    /// Maximum CT log entries
    pub max_log_entries: u32,
}

/// Post-quantum cryptography configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PqcConfig {
    /// Enable post-quantum cryptography
    pub enable_pqc: bool,
    
    /// Use FALCON-1024 signatures
    pub enable_falcon: bool,
    
    /// Use Kyber encryption
    pub enable_kyber: bool,
    
    /// Enable hybrid classical+quantum crypto
    pub enable_hybrid: bool,
    
    /// Quantum security level
    pub security_level: u32,
}

/// Integration and cross-layer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    /// Enable cross-layer optimization
    pub enable_cross_layer_optimization: bool,
    
    /// Integration validation timeout
    pub validation_timeout: Duration,
    
    /// Performance coordination interval
    pub coordination_interval: Duration,
    
    /// Enable layer monitoring
    pub enable_layer_monitoring: bool,
}

/// Deployment-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentConfig {
    /// Deployment mode
    pub mode: DeploymentMode,
    
    /// Enable production security
    pub production_security: bool,
    
    /// Enable consensus mandatory mode
    pub consensus_mandatory: bool,
    
    /// Enable legacy compatibility
    pub legacy_compatibility: bool,
    
    /// Enable federated bootstrap
    pub federated_bootstrap: bool,
}

/// Deployment modes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeploymentMode {
    Development,
    Testing,
    Staging,
    Production,
    Bootstrap,
    Gateway,
}

impl Internet2Config {
    /// Load configuration from file with CLI overrides
    pub async fn load(config_path: &str, bind_address: &str, port: u16) -> Result<Self> {
        let mut config = if Path::new(config_path).exists() {
            let content = tokio::fs::read_to_string(config_path).await?;
            toml::from_str(&content)
                .map_err(|e| anyhow!("Failed to parse config file: {}", e))?
        } else {
            Self::default_production()
        };
        
        // Apply CLI overrides
        config.global.bind_address = bind_address.parse()
            .map_err(|e| anyhow!("Invalid bind address: {}", e))?;
        config.global.port = port;
        
        Ok(config)
    }
    
    /// Default production configuration
    pub fn default_production() -> Self {
        Self {
            global: GlobalConfig {
                bind_address: Ipv6Addr::UNSPECIFIED,
                port: 443,
                server_id: "internet2-server-001".to_string(),
                max_connections: 100000,
                ipv6_only: true,
                log_level: "info".to_string(),
                metrics_interval: Duration::from_secs(60),
            },
            stoq: StoqConfig {
                performance: StoqPerformanceConfig {
                    target_throughput_gbps: 40.0,
                    enable_zero_copy: true,
                    enable_hardware_acceleration: true,
                    connection_pool_size: 1000,
                    memory_pool_size: 2048,
                    frame_batch_size: 64,
                    enable_cpu_affinity: true,
                    enable_large_send_offload: true,
                },
                quic: QuicConfig {
                    max_concurrent_streams: 1000,
                    send_buffer_size: 16 * 1024 * 1024,
                    receive_buffer_size: 16 * 1024 * 1024,
                    connection_timeout: Duration::from_secs(5),
                    idle_timeout: Duration::from_secs(120),
                    enable_0rtt: true,
                    enable_migration: true,
                    congestion_control: CongestionControl::Bbr2,
                    max_datagram_size: 65507,
                },
                certificates: StoqCertificateConfig {
                    validate_at_connection: true,
                    validation_timeout: Duration::from_secs(10),
                    enable_caching: true,
                    cache_size: 10000,
                    cache_ttl: Duration::from_secs(3600),
                },
                dns: StoqDnsConfig {
                    use_embedded_resolver: true,
                    query_timeout: Duration::from_secs(5),
                    enable_caching: true,
                    cache_size: 10000,
                    cache_ttl: Duration::from_secs(300),
                },
            },
            hypermesh: HyperMeshConfig {
                consensus: ConsensusConfig {
                    mandatory_four_proof: true,
                    validation_timeout: Duration::from_millis(100),
                    min_stake_requirement: 1000,
                    pow_difficulty: 4,
                    enable_byzantine_detection: true,
                    max_consensus_participants: 100,
                },
                assets: AssetConfig {
                    max_assets_per_node: 10000,
                    allocation_timeout: Duration::from_secs(30),
                    enable_pooling: true,
                    pool_size: 1000,
                    cleanup_interval: Duration::from_secs(300),
                    default_resource_capacity: 1000.0,
                    require_consensus_for_allocation: true,
                },
                vm: VmConfig {
                    enable_vm_execution: true,
                    max_vms_per_node: 100,
                    execution_timeout: Duration::from_secs(3600),
                    memory_limit_mb: 8192,
                    cpu_limit_cores: 16,
                    enable_snapshots: true,
                },
                proxy: ProxyConfig {
                    enable_nat_addressing: true,
                    connection_timeout: Duration::from_secs(30),
                    max_proxy_connections: 10000,
                    enable_trust_validation: true,
                    enable_performance_monitoring: true,
                },
            },
            trustchain: TrustChainConfig {
                ca: CaConfig {
                    ca_mode: CaMode::Embedded,
                    certificate_validity_days: 90,
                    rotation_interval: Duration::from_secs(24 * 3600),
                    enable_auto_rotation: true,
                    max_chain_depth: 5,
                },
                dns: DnsConfig {
                    dns_mode: DnsMode::Embedded,
                    dns_port: 53,
                    query_timeout: Duration::from_secs(5),
                    enable_caching: true,
                    cache_ttl: Duration::from_secs(300),
                    max_cache_size: 10000,
                },
                ct: CtConfig {
                    enable_ct_logging: true,
                    submission_timeout: Duration::from_secs(30),
                    enable_verification: true,
                    max_log_entries: 1000000,
                },
                pqc: PqcConfig {
                    enable_pqc: true,
                    enable_falcon: true,
                    enable_kyber: true,
                    enable_hybrid: true,
                    security_level: 128,
                },
            },
            integration: IntegrationConfig {
                enable_cross_layer_optimization: true,
                validation_timeout: Duration::from_secs(30),
                coordination_interval: Duration::from_secs(60),
                enable_layer_monitoring: true,
            },
            deployment: DeploymentConfig {
                mode: DeploymentMode::Production,
                production_security: true,
                consensus_mandatory: true,
                legacy_compatibility: false,
                federated_bootstrap: true,
            },
        }
    }
    
    /// Default development configuration (reduced security)
    pub fn default_development() -> Self {
        let mut config = Self::default_production();
        
        // Reduce security for development
        config.deployment.mode = DeploymentMode::Development;
        config.deployment.production_security = false;
        config.deployment.consensus_mandatory = false;
        config.deployment.legacy_compatibility = true;
        config.deployment.federated_bootstrap = false;
        
        // Reduce performance requirements
        config.stoq.performance.target_throughput_gbps = 1.0;
        config.hypermesh.consensus.mandatory_four_proof = false;
        config.trustchain.pqc.enable_pqc = false;
        
        config
    }
    
    /// Apply production settings
    pub fn with_production_settings(mut self, federated: bool) -> Self {
        self.deployment.mode = DeploymentMode::Production;
        self.deployment.production_security = true;
        self.deployment.consensus_mandatory = true;
        self.deployment.legacy_compatibility = false;
        self.deployment.federated_bootstrap = federated;
        
        // Maximum security settings
        self.hypermesh.consensus.mandatory_four_proof = true;
        self.trustchain.pqc.enable_pqc = true;
        self.trustchain.ca.ca_mode = CaMode::Embedded;
        self.trustchain.dns.dns_mode = DnsMode::Embedded;
        
        self
    }
    
    /// Apply development settings
    pub fn with_development_settings(mut self, legacy_gateway: bool) -> Self {
        self.deployment.mode = DeploymentMode::Development;
        self.deployment.production_security = false;
        self.deployment.consensus_mandatory = false;
        self.deployment.legacy_compatibility = legacy_gateway;
        
        // Reduced security for testing
        self.hypermesh.consensus.mandatory_four_proof = false;
        self.trustchain.pqc.enable_pqc = false;
        
        self
    }
    
    /// Apply bootstrap settings
    pub fn with_bootstrap_settings(mut self, root_authority: bool) -> Self {
        self.deployment.mode = DeploymentMode::Bootstrap;
        self.deployment.federated_bootstrap = true;
        
        if root_authority {
            // Bootstrap as root certificate authority
            self.trustchain.ca.ca_mode = CaMode::Embedded;
            self.trustchain.dns.dns_mode = DnsMode::Embedded;
        }
        
        self
    }
    
    /// Apply gateway settings
    pub fn with_gateway_settings(mut self, translate_protocols: bool) -> Self {
        self.deployment.mode = DeploymentMode::Gateway;
        self.deployment.legacy_compatibility = true;
        
        if translate_protocols {
            // Enable protocol translation
            self.trustchain.ca.ca_mode = CaMode::Hybrid;
            self.trustchain.dns.dns_mode = DnsMode::Hybrid;
        }
        
        self
    }
    
    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate IPv6-only requirement
        if !self.global.ipv6_only {
            return Err(anyhow!("IPv4 is not supported - Internet 2.0 is IPv6-only"));
        }
        
        // Validate performance targets
        if self.stoq.performance.target_throughput_gbps <= 0.0 {
            return Err(anyhow!("Invalid throughput target: must be > 0"));
        }
        
        // Validate security requirements for production
        if self.deployment.mode == DeploymentMode::Production {
            if self.trustchain.ca.ca_mode != CaMode::Embedded {
                return Err(anyhow!("Production mode requires embedded CA"));
            }
            
            if self.trustchain.dns.dns_mode != DnsMode::Embedded {
                return Err(anyhow!("Production mode requires embedded DNS"));
            }
            
            if !self.deployment.consensus_mandatory {
                return Err(anyhow!("Production mode requires mandatory consensus"));
            }
        }
        
        // Validate consensus requirements
        if self.deployment.consensus_mandatory && !self.hypermesh.consensus.mandatory_four_proof {
            return Err(anyhow!("Mandatory consensus requires four-proof validation"));
        }
        
        Ok(())
    }
}

impl Default for Internet2Config {
    fn default() -> Self {
        Self::default_production()
    }
}