//! STOQ Transport Configuration

use serde::{Serialize, Deserialize};
use std::time::Duration;
use tracing::{debug, warn};

/// Network tier classification for adaptive configuration
#[derive(Debug, Clone)]
pub enum NetworkTier {
    /// Slow networks (<100 Mbps)
    Slow { mbps: f64 },
    /// Home broadband (100 Mbps - 1 Gbps)
    Home { mbps: f64 },
    /// Standard gigabit (1-2.5 Gbps)
    Standard { gbps: f64 },
    /// Performance networks (2.5-10 Gbps)
    Performance { gbps: f64 },
    /// Enterprise networks (10-25 Gbps)
    Enterprise { gbps: f64 },
    /// Data center networks (25+ Gbps)
    DataCenter { gbps: f64 },
}

impl NetworkTier {
    /// Create network tier from Gbps measurement
    pub fn from_gbps(gbps: f64) -> Self {
        let mbps = gbps * 1000.0;
        match gbps {
            g if g >= 25.0 => NetworkTier::DataCenter { gbps: g },
            g if g >= 10.0 => NetworkTier::Enterprise { gbps: g },
            g if g >= 2.5 => NetworkTier::Performance { gbps: g },
            g if g >= 1.0 => NetworkTier::Standard { gbps: g },
            _g if mbps >= 100.0 => NetworkTier::Home { mbps },
            _ => NetworkTier::Slow { mbps },
        }
    }

    // Backward compatibility for tests using old variant names

    #[deprecated(since = "0.1.0", note = "use from_gbps(1.0) instead")]
    pub fn auto() -> Self {
        NetworkTier::Standard { gbps: 1.0 }
    }

    #[deprecated(since = "0.1.0", note = "use from_gbps(10.0) instead")]
    pub fn lan() -> Self {
        NetworkTier::Enterprise { gbps: 10.0 }
    }

    #[deprecated(since = "0.1.0", note = "use from_gbps(0.1) instead")]
    pub fn wan() -> Self {
        NetworkTier::Home { mbps: 100.0 }
    }

    #[deprecated(since = "0.1.0", note = "use from_gbps(0.5) instead")]
    pub fn metro() -> Self {
        NetworkTier::Home { mbps: 500.0 }
    }

    #[deprecated(since = "0.1.0", note = "use from_gbps(0.001) instead")]
    pub fn satellite() -> Self {
        NetworkTier::Slow { mbps: 1.0 }
    }

    #[deprecated(since = "0.1.0", note = "network isolation is now in NetworkIsolationManager")]
    pub fn anonymous() -> Self {
        NetworkTier::Slow { mbps: 10.0 }
    }
}

/// Congestion control algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CongestionControl {
    /// BBR v2 for maximum throughput
    Bbr2,
    /// CUBIC (default)
    Cubic,
    /// NewReno
    NewReno,
}

impl Default for CongestionControl {
    fn default() -> Self {
        Self::Bbr2 // BBR v2 for high performance
    }
}

use super::falcon::FalconVariant;

/// STOQ Transport configuration for QUIC over IPv6
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    /// Bind address (IPv6 only)
    pub bind_address: std::net::Ipv6Addr,
    /// Port to bind to
    pub port: u16,
    /// Maximum concurrent connections (None = unlimited)
    pub max_connections: Option<u32>,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Enable connection migration
    pub enable_migration: bool,
    /// Enable 0-RTT resumption
    pub enable_0rtt: bool,
    /// Maximum idle timeout
    pub max_idle_timeout: Duration,
    /// Certificate rotation interval
    pub cert_rotation_interval: Duration,
    /// Maximum concurrent streams per connection
    pub max_concurrent_streams: u32,
    /// Send buffer size
    pub send_buffer_size: usize,
    /// Receive buffer size
    pub receive_buffer_size: usize,
    /// Connection pool size for multiplexing
    pub connection_pool_size: usize,
    /// Enable zero-copy operations
    pub enable_zero_copy: bool,
    /// Maximum datagram size
    pub max_datagram_size: usize,
    /// Congestion control algorithm
    pub congestion_control: CongestionControl,
    /// Enable memory pool optimization for zero-copy
    pub enable_memory_pool: bool,
    /// Memory pool size for zero-copy operations
    pub memory_pool_size: usize,
    /// Frame batching size for syscall reduction
    pub frame_batch_size: usize,
    /// Health check interval in seconds (0 disables health checks)
    pub health_check_interval: u64,
    /// Connection idle timeout in seconds before marking unhealthy
    pub connection_idle_timeout: u64,
    /// Enable CPU affinity for network threads
    pub enable_cpu_affinity: bool,
    /// Enable large send offload optimization
    pub enable_large_send_offload: bool,
    /// Enable FALCON quantum-resistant cryptography
    pub enable_falcon_crypto: bool,
    /// FALCON variant to use
    pub falcon_variant: FalconVariant,
}

impl Default for TransportConfig {
    fn default() -> Self {
        // Use port 0 (OS-assigned) for tests to avoid binding conflicts
        #[cfg(test)]
        let port = 0;
        #[cfg(not(test))]
        let port = crate::DEFAULT_PORT;

        Self {
            bind_address: std::net::Ipv6Addr::LOCALHOST, // Default to localhost for testing
            port,
            max_connections: Some(100), // Limited for DoS protection
            connection_timeout: Duration::from_secs(5), // Reduced for performance
            enable_migration: true,
            enable_0rtt: false, // Disabled due to replay attack vulnerability
            max_idle_timeout: Duration::from_secs(120), // Increased for connection reuse
            cert_rotation_interval: Duration::from_secs(24 * 60 * 60), // 24 hours
            max_concurrent_streams: 1000, // High concurrency support
            send_buffer_size: 16 * 1024 * 1024, // 16MB send buffer
            receive_buffer_size: 16 * 1024 * 1024, // 16MB receive buffer
            connection_pool_size: 100, // Connection multiplexing
            enable_zero_copy: true, // Zero-copy optimization
            max_datagram_size: 65507, // Maximum UDP datagram
            congestion_control: CongestionControl::default(),
            enable_memory_pool: true, // Memory pool optimization
            memory_pool_size: 1024, // 1024 buffers per pool
            frame_batch_size: 64, // Batch 64 frames per syscall
            health_check_interval: 10, // Health check every 10 seconds
            connection_idle_timeout: 30, // Mark connections unhealthy after 30s idle
            enable_cpu_affinity: true, // CPU affinity optimization
            enable_large_send_offload: true, // LSO for large transfers
            enable_falcon_crypto: true, // Quantum-resistant FALCON cryptography
            falcon_variant: FalconVariant::Falcon1024, // Maximum security level
        }
    }
}

impl TransportConfig {
    /// Adapt configuration based on detected network tier for true adaptive behavior
    pub fn adapt_for_network_tier(&mut self, network_tier: &NetworkTier) {
        match network_tier {
            NetworkTier::Slow { .. } => {
                // Optimize for low bandwidth (<100 Mbps)
                self.send_buffer_size = 256 * 1024; // 256KB
                self.receive_buffer_size = 256 * 1024;
                self.max_concurrent_streams = 10;
                self.frame_batch_size = 4;
                self.enable_zero_copy = false;
                self.max_datagram_size = 1200; // Conservative MTU
                debug!("Adapted config for slow network tier");
            },
            NetworkTier::Home { .. } => {
                // Standard home broadband (100 Mbps - 1 Gbps)
                self.send_buffer_size = 2 * 1024 * 1024; // 2MB
                self.receive_buffer_size = 2 * 1024 * 1024;
                self.max_concurrent_streams = 100;
                self.frame_batch_size = 16;
                self.enable_zero_copy = true;
                self.max_datagram_size = 1500;
                debug!("Adapted config for home network tier");
            },
            NetworkTier::Standard { .. } => {
                // Gigabit networks (1-2.5 Gbps)
                self.send_buffer_size = 8 * 1024 * 1024; // 8MB
                self.receive_buffer_size = 8 * 1024 * 1024;
                self.max_concurrent_streams = 500;
                self.frame_batch_size = 32;
                self.enable_zero_copy = true;
                self.enable_large_send_offload = true;
                self.max_datagram_size = 9000; // Jumbo frames
                debug!("Adapted config for standard gigabit network tier");
            },
            NetworkTier::Performance { .. } | NetworkTier::Enterprise { .. } | NetworkTier::DataCenter { .. } => {
                // High-performance networks (2.5+ Gbps)
                self.send_buffer_size = 16 * 1024 * 1024; // 16MB
                self.receive_buffer_size = 16 * 1024 * 1024;
                self.max_concurrent_streams = 1000;
                self.frame_batch_size = 64;
                self.enable_zero_copy = true;
                self.enable_memory_pool = true;
                self.enable_large_send_offload = true;
                self.enable_cpu_affinity = true;
                self.max_datagram_size = 9000; // Jumbo frames
                debug!("Adapted config for high-performance network tier");
            }
        }
    }

    // Backward compatibility methods for test suite

    /// Legacy: Deprecated, use bind_address field directly
    #[deprecated(since = "0.1.0", note = "use bind_address field directly")]
    pub fn with_bind_addr(mut self, addr: std::net::Ipv6Addr) -> Self {
        self.bind_address = addr;
        self
    }

    /// Legacy: Deprecated, use max_datagram_size field directly
    #[deprecated(since = "0.1.0", note = "use max_datagram_size field directly")]
    pub fn with_max_packet_size(mut self, size: usize) -> Self {
        self.max_datagram_size = size;
        self
    }

    /// Legacy: Deprecated, use adapt_for_network_tier instead
    #[deprecated(since = "0.1.0", note = "use adapt_for_network_tier instead")]
    pub fn with_network_tier(self, _tier: NetworkTier) -> Self {
        warn!("with_network_tier is deprecated, tier is auto-detected");
        self
    }

    /// Legacy: Deprecated, network isolation is always available
    #[deprecated(since = "0.1.0", note = "network isolation is always available")]
    pub fn enable_network_isolation(self, _enable: bool) -> Self {
        warn!("enable_network_isolation is deprecated, always available");
        self
    }
}
