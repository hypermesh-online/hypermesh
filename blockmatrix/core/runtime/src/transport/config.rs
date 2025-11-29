//! Transport configuration types for HyperMesh container communication
//!
//! This module contains all configuration types for the QUIC transport integration,
//! including container communication, P2P networking, security, and performance settings.

use nexus_transport::TransportConfig;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;

/// Configuration for container transport
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerTransportConfig {
    /// QUIC transport configuration
    pub quic_config: TransportConfig,
    
    /// Container communication settings
    pub container_comm: ContainerCommConfig,
    
    /// P2P mesh communication settings
    pub p2p_comm: P2PCommConfig,
    
    /// Security settings
    pub security: TransportSecurityConfig,
    
    /// Performance tuning
    pub performance: TransportPerformanceConfig,
    
    /// Message routing configuration
    pub routing: RoutingConfig,
}

/// Container communication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerCommConfig {
    /// Enable container-to-container communication
    pub enabled: bool,
    
    /// Maximum concurrent connections per container
    pub max_connections_per_container: usize,
    
    /// Connection timeout
    pub connection_timeout: Duration,
    
    /// Message timeout
    pub message_timeout: Duration,
    
    /// Keep-alive interval
    pub keep_alive_interval: Duration,
    
    /// Buffer sizes
    pub buffer_config: BufferConfig,
}

/// P2P communication configuration  
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PCommConfig {
    /// Enable P2P mesh communication
    pub enabled: bool,
    
    /// Bootstrap peers for initial connectivity
    pub bootstrap_peers: Vec<SocketAddr>,
    
    /// Maximum peer connections
    pub max_peer_connections: usize,
    
    /// Peer discovery settings
    pub discovery: PeerDiscoveryConfig,
    
    /// Connection migration settings
    pub migration: ConnectionMigrationConfig,
}

/// Buffer configuration for transport
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferConfig {
    /// Send buffer size
    pub send_buffer_size: usize,
    
    /// Receive buffer size
    pub recv_buffer_size: usize,
    
    /// Stream buffer size
    pub stream_buffer_size: usize,
    
    /// Connection buffer count
    pub connection_buffer_count: usize,
}

/// Peer discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerDiscoveryConfig {
    /// Enable automatic peer discovery
    pub enabled: bool,
    
    /// Discovery interval
    pub interval: Duration,
    
    /// Discovery timeout
    pub timeout: Duration,
    
    /// Multicast address for discovery
    pub multicast_addr: Option<SocketAddr>,
    
    /// DHT configuration for distributed discovery
    pub dht_config: Option<DHTConfig>,
}

/// Distributed Hash Table configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DHTConfig {
    /// DHT bootstrap nodes
    pub bootstrap_nodes: Vec<SocketAddr>,
    
    /// Node ID in DHT
    pub node_id: Vec<u8>,
    
    /// Replication factor
    pub replication_factor: usize,
    
    /// Bucket refresh interval
    pub refresh_interval: Duration,
}

/// Connection migration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMigrationConfig {
    /// Enable connection migration
    pub enabled: bool,
    
    /// Migration timeout
    pub timeout: Duration,
    
    /// Maximum migration attempts
    pub max_attempts: u32,
    
    /// Probing interval during migration
    pub probing_interval: Duration,
}

/// Transport security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportSecurityConfig {
    /// Enable transport layer encryption
    pub enable_encryption: bool,
    
    /// Certificate validation mode
    pub cert_validation: CertValidationMode,
    
    /// Message authentication
    pub message_auth: MessageAuthConfig,
    
    /// Byzantine protection settings
    pub byzantine_protection: ByzantineProtectionConfig,
    
    /// Rate limiting configuration
    pub rate_limiting: RateLimitConfig,
}

/// Certificate validation modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CertValidationMode {
    /// Strict certificate validation
    Strict,
    
    /// Relaxed validation (for development)
    Relaxed,
    
    /// Custom validation logic
    Custom(String),
}

/// Message authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageAuthConfig {
    /// Enable message authentication
    pub enabled: bool,
    
    /// Authentication method
    pub method: MessageAuthMethod,
    
    /// Key rotation interval
    pub key_rotation_interval: Duration,
    
    /// Authentication timeout
    pub auth_timeout: Duration,
}

/// Message authentication methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageAuthMethod {
    /// HMAC with SHA-256
    HmacSha256,
    
    /// Digital signatures
    DigitalSignature,
    
    /// Symmetric encryption
    SymmetricEncryption,
    
    /// Zero-knowledge proofs
    ZKProofs,
}

/// Byzantine protection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByzantineProtectionConfig {
    /// Enable Byzantine fault protection
    pub enabled: bool,
    
    /// Message validation timeout
    pub validation_timeout: Duration,
    
    /// Reputation threshold for message acceptance
    pub reputation_threshold: f64,
    
    /// Suspicious message detection threshold
    pub anomaly_threshold: f64,
    
    /// Quarantine duration for suspicious peers
    pub quarantine_duration: Duration,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Enable rate limiting
    pub enabled: bool,
    
    /// Messages per second limit
    pub messages_per_second: u64,
    
    /// Bytes per second limit
    pub bytes_per_second: u64,
    
    /// Burst allowance
    pub burst_allowance: u64,
    
    /// Rate limit window
    pub window_duration: Duration,
}

/// Transport performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportPerformanceConfig {
    /// Connection pool size
    pub connection_pool_size: usize,
    
    /// Worker thread count
    pub worker_threads: usize,
    
    /// I/O batch size
    pub io_batch_size: usize,
    
    /// Congestion control algorithm
    pub congestion_control: CongestionControlAlgorithm,
    
    /// Flow control settings
    pub flow_control: FlowControlConfig,
    
    /// Performance monitoring
    pub monitoring: PerformanceMonitoringConfig,
}

/// Congestion control algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CongestionControlAlgorithm {
    /// NewReno algorithm
    NewReno,
    
    /// Cubic algorithm  
    Cubic,
    
    /// BBR algorithm
    BBR,
    
    /// Custom algorithm
    Custom(String),
}

/// Flow control configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowControlConfig {
    /// Initial window size
    pub initial_window_size: u64,
    
    /// Maximum window size
    pub max_window_size: u64,
    
    /// Window scaling factor
    pub scaling_factor: f64,
    
    /// Flow control mode
    pub mode: FlowControlMode,
}

/// Flow control modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FlowControlMode {
    /// Automatic flow control
    Automatic,
    
    /// Manual flow control
    Manual,
    
    /// Adaptive flow control
    Adaptive,
}

/// Performance monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMonitoringConfig {
    /// Enable performance monitoring
    pub enabled: bool,
    
    /// Metrics collection interval
    pub collection_interval: Duration,
    
    /// Latency histogram buckets
    pub latency_buckets: Vec<f64>,
    
    /// Throughput measurement window
    pub throughput_window: Duration,
}

/// Message routing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    /// Default routing strategy
    pub default_strategy: RoutingStrategy,
    
    /// Load balancing configuration
    pub load_balancing: LoadBalancingConfig,
    
    /// Failover settings
    pub failover: FailoverConfig,
    
    /// Routing table refresh interval
    pub refresh_interval: Duration,
}

/// Routing strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutingStrategy {
    /// Direct routing (shortest path)
    Direct,
    
    /// Load-balanced routing
    LoadBalanced,
    
    /// Redundant routing (multiple paths)
    Redundant,
    
    /// Adaptive routing based on performance
    Adaptive,
}

/// Load balancing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancingConfig {
    /// Load balancing algorithm
    pub algorithm: LoadBalancingAlgorithm,
    
    /// Health check settings
    pub health_checks: HealthCheckConfig,
    
    /// Weight calculation method
    pub weight_calculation: WeightCalculationMethod,
}

/// Load balancing algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancingAlgorithm {
    /// Round-robin
    RoundRobin,
    
    /// Weighted round-robin
    WeightedRoundRobin,
    
    /// Least connections
    LeastConnections,
    
    /// Lowest latency
    LowestLatency,
    
    /// Hash-based
    Hash,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Health check interval
    pub interval: Duration,
    
    /// Health check timeout
    pub timeout: Duration,
    
    /// Failure threshold
    pub failure_threshold: u32,
    
    /// Recovery threshold
    pub recovery_threshold: u32,
}

/// Weight calculation methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WeightCalculationMethod {
    /// Static weights
    Static,
    
    /// Dynamic based on latency
    DynamicLatency,
    
    /// Dynamic based on throughput
    DynamicThroughput,
    
    /// Dynamic based on error rate
    DynamicErrorRate,
    
    /// Composite scoring
    Composite,
}

/// Failover configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverConfig {
    /// Enable automatic failover
    pub enabled: bool,
    
    /// Failover timeout
    pub timeout: Duration,
    
    /// Maximum failover attempts
    pub max_attempts: u32,
    
    /// Failover strategy
    pub strategy: FailoverStrategy,
}

/// Failover strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FailoverStrategy {
    /// Immediate failover
    Immediate,
    
    /// Gradual failover with retry
    Gradual,
    
    /// Circuit breaker pattern
    CircuitBreaker,
}

impl Default for ContainerTransportConfig {
    fn default() -> Self {
        Self {
            quic_config: TransportConfig::default(),
            container_comm: ContainerCommConfig {
                enabled: true,
                max_connections_per_container: 10,
                connection_timeout: Duration::from_secs(30),
                message_timeout: Duration::from_secs(10),
                keep_alive_interval: Duration::from_secs(15),
                buffer_config: BufferConfig {
                    send_buffer_size: 64 * 1024,
                    recv_buffer_size: 64 * 1024,
                    stream_buffer_size: 16 * 1024,
                    connection_buffer_count: 8,
                },
            },
            p2p_comm: P2PCommConfig {
                enabled: true,
                bootstrap_peers: Vec::new(),
                max_peer_connections: 50,
                discovery: PeerDiscoveryConfig {
                    enabled: true,
                    interval: Duration::from_secs(30),
                    timeout: Duration::from_secs(5),
                    multicast_addr: None,
                    dht_config: None,
                },
                migration: ConnectionMigrationConfig {
                    enabled: true,
                    timeout: Duration::from_secs(30),
                    max_attempts: 3,
                    probing_interval: Duration::from_secs(1),
                },
            },
            security: TransportSecurityConfig {
                enable_encryption: true,
                cert_validation: CertValidationMode::Strict,
                message_auth: MessageAuthConfig {
                    enabled: true,
                    method: MessageAuthMethod::HmacSha256,
                    key_rotation_interval: Duration::from_secs(3600),
                    auth_timeout: Duration::from_secs(5),
                },
                byzantine_protection: ByzantineProtectionConfig {
                    enabled: true,
                    validation_timeout: Duration::from_secs(5),
                    reputation_threshold: 0.7,
                    anomaly_threshold: 0.8,
                    quarantine_duration: Duration::from_secs(300),
                },
                rate_limiting: RateLimitConfig {
                    enabled: true,
                    messages_per_second: 1000,
                    bytes_per_second: 10 * 1024 * 1024, // 10 MB/s
                    burst_allowance: 100,
                    window_duration: Duration::from_secs(1),
                },
            },
            performance: TransportPerformanceConfig {
                connection_pool_size: 100,
                worker_threads: 4,
                io_batch_size: 32,
                congestion_control: CongestionControlAlgorithm::BBR,
                flow_control: FlowControlConfig {
                    initial_window_size: 65536,
                    max_window_size: 1048576,
                    scaling_factor: 1.5,
                    mode: FlowControlMode::Adaptive,
                },
                monitoring: PerformanceMonitoringConfig {
                    enabled: true,
                    collection_interval: Duration::from_secs(10),
                    latency_buckets: vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0],
                    throughput_window: Duration::from_secs(30),
                },
            },
            routing: RoutingConfig {
                default_strategy: RoutingStrategy::Adaptive,
                load_balancing: LoadBalancingConfig {
                    algorithm: LoadBalancingAlgorithm::LowestLatency,
                    health_checks: HealthCheckConfig {
                        interval: Duration::from_secs(10),
                        timeout: Duration::from_secs(5),
                        failure_threshold: 3,
                        recovery_threshold: 2,
                    },
                    weight_calculation: WeightCalculationMethod::Composite,
                },
                failover: FailoverConfig {
                    enabled: true,
                    timeout: Duration::from_secs(10),
                    max_attempts: 3,
                    strategy: FailoverStrategy::Gradual,
                },
                refresh_interval: Duration::from_secs(60),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_container_transport_config() {
        let config = ContainerTransportConfig::default();
        
        assert!(config.container_comm.enabled);
        assert_eq!(config.container_comm.max_connections_per_container, 10);
        assert!(config.security.enable_encryption);
        assert!(config.performance.monitoring.enabled);
    }

    #[test]
    fn test_buffer_config_defaults() {
        let config = ContainerTransportConfig::default();
        let buffer = &config.container_comm.buffer_config;
        
        assert_eq!(buffer.send_buffer_size, 64 * 1024);
        assert_eq!(buffer.recv_buffer_size, 64 * 1024);
        assert_eq!(buffer.connection_buffer_count, 8);
    }

    #[test]
    fn test_security_config_defaults() {
        let config = ContainerTransportConfig::default();
        let security = &config.security;
        
        assert!(security.enable_encryption);
        assert!(security.message_auth.enabled);
        assert!(security.byzantine_protection.enabled);
        assert!(security.rate_limiting.enabled);
    }
}