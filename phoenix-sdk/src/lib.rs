//! Phoenix SDK - High-performance distributed computing made simple
//!
//! Phoenix provides a zero-configuration, developer-friendly API for building
//! high-performance distributed applications. It combines the power of STOQ's
//! adaptive transport with TrustChain's security infrastructure to deliver
//! enterprise-grade networking with minimal complexity.
//!
//! # Quick Start
//!
//! ```rust
//! use phoenix_sdk::Phoenix;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create Phoenix instance with zero configuration
//!     let phoenix = Phoenix::new("my-app").await?;
//!
//!     // Connect to another Phoenix application
//!     let conn = phoenix.connect("remote-app.example.com:8080").await?;
//!
//!     // Send and receive data with automatic optimization
//!     conn.send(&"Hello, Phoenix!").await?;
//!     let response: String = conn.receive().await?;
//!
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod connection;
pub mod listener;
pub mod metrics;
pub mod errors;
pub mod compression;
pub mod security;

#[cfg(feature = "dashboard")]
pub mod dashboard;

// Re-export commonly used types
pub use config::{PhoenixConfig, PerformanceTier, SecurityLevel};
pub use connection::{PhoenixConnection, ConnectionState};
pub use listener::{PhoenixListener, ListenerState};
pub use metrics::{PhoenixMetrics, LiveMetrics, ConnectionMetrics};
pub use errors::{PhoenixError, Result};
pub use security::{SecurityContext, CertificateInfo};

use std::sync::Arc;
use std::net::SocketAddr;
use std::time::Duration;
use parking_lot::RwLock;
use dashmap::DashMap;
use tracing::{info, warn, error, debug};
use stoq::{StoqTransport, StoqConfig, TransportConfig};
use trustchain::{TrustChain, TrustChainConfig, TrustChainStoqClient};
use connection::ConnectionManager;
use listener::ListenerManager;
use metrics::MetricsCollector;
use security::SecurityManager;

/// Phoenix SDK version
pub const PHOENIX_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default Phoenix port
pub const DEFAULT_PORT: u16 = 8080;

/// Phoenix SDK - High-performance distributed computing made simple
pub struct Phoenix {
    /// Application name for identification
    app_name: String,
    /// Phoenix configuration
    config: PhoenixConfig,
    /// STOQ transport layer
    transport: Arc<StoqTransport>,
    /// TrustChain security layer
    trustchain: Arc<TrustChain>,
    /// Connection manager
    connections: Arc<ConnectionManager>,
    /// Listener manager
    listeners: Arc<ListenerManager>,
    /// Metrics collector
    metrics: Arc<MetricsCollector>,
    /// Security manager
    security: Arc<SecurityManager>,
    /// Active connections
    active_connections: Arc<DashMap<String, Arc<PhoenixConnection>>>,
    /// Phoenix state
    state: Arc<RwLock<PhoenixState>>,
}

/// Phoenix internal state
#[derive(Debug, Clone)]
struct PhoenixState {
    /// Whether Phoenix is initialized
    initialized: bool,
    /// Start time for uptime tracking
    start_time: std::time::Instant,
    /// Total bytes sent
    bytes_sent: u64,
    /// Total bytes received
    bytes_received: u64,
    /// Total connections established
    total_connections: u64,
}

impl Phoenix {
    /// Create new Phoenix instance with zero configuration for development
    ///
    /// # Example
    /// ```rust
    /// let phoenix = Phoenix::new("my-app").await?;
    /// ```
    pub async fn new(app_name: &str) -> Result<Self> {
        Self::with_config(PhoenixConfig {
            app_name: app_name.to_string(),
            performance_tier: PerformanceTier::Development,
            security_level: SecurityLevel::Development,
            region: None,
            auto_optimize: true,
            enable_compression: true,
            enable_metrics: true,
            connection_timeout: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(300),
            max_connections: 1000,
        }).await
    }

    /// Create Phoenix instance with custom configuration
    ///
    /// # Example
    /// ```rust
    /// let config = PhoenixConfig::production("my-app")
    ///     .with_performance_tier(PerformanceTier::HighThroughput)
    ///     .with_security_level(SecurityLevel::Enhanced);
    ///
    /// let phoenix = Phoenix::with_config(config).await?;
    /// ```
    pub async fn with_config(config: PhoenixConfig) -> Result<Self> {
        info!("Initializing Phoenix SDK v{} for '{}'", PHOENIX_VERSION, config.app_name);

        // Create STOQ transport configuration based on performance tier
        let stoq_config = Self::create_stoq_config(&config)?;
        let transport = Arc::new(StoqTransport::new(stoq_config.transport).await?);

        // Create TrustChain configuration based on security level
        let trustchain_config = Self::create_trustchain_config(&config)?;
        let trustchain = Arc::new(TrustChain::new(trustchain_config).await?);

        // Initialize managers
        let connections = Arc::new(ConnectionManager::new(
            transport.clone(),
            config.max_connections,
        ));

        let listeners = Arc::new(ListenerManager::new(
            transport.clone(),
            trustchain.clone(),
        ));

        let metrics = Arc::new(MetricsCollector::new(&config.app_name));

        let security = Arc::new(SecurityManager::new(
            trustchain.clone(),
            config.security_level.clone(),
        ));

        let state = Arc::new(RwLock::new(PhoenixState {
            initialized: true,
            start_time: std::time::Instant::now(),
            bytes_sent: 0,
            bytes_received: 0,
            total_connections: 0,
        }));

        let phoenix = Self {
            app_name: config.app_name.clone(),
            config,
            transport,
            trustchain,
            connections,
            listeners,
            metrics,
            security,
            active_connections: Arc::new(DashMap::new()),
            state,
        };

        // Start background monitoring if metrics are enabled
        if phoenix.config.enable_metrics {
            phoenix.start_metrics_collection().await?;
        }

        info!("Phoenix SDK initialized successfully for '{}'", phoenix.app_name);
        Ok(phoenix)
    }

    /// Connect to another Phoenix application
    ///
    /// # Example
    /// ```rust
    /// let conn = phoenix.connect("remote-app.example.com:8080").await?;
    /// ```
    pub async fn connect(&self, target: &str) -> Result<PhoenixConnection> {
        info!("Connecting to {}", target);

        // Parse target address
        let addr = Self::parse_target(target)?;

        // Create connection through connection manager
        let conn = self.connections.connect(
            addr,
            &self.config,
            self.metrics.clone(),
            self.security.clone(),
        ).await?;

        // Track connection
        self.active_connections.insert(target.to_string(), conn.clone());
        self.state.write().total_connections += 1;

        info!("Connected to {} successfully", target);
        Ok(*conn)
    }

    /// Start listening for incoming connections
    ///
    /// # Example
    /// ```rust
    /// let listener = phoenix.listen(8080).await?;
    ///
    /// // Accept connections in a loop
    /// while let Ok(conn) = listener.accept().await {
    ///     tokio::spawn(async move {
    ///         // Handle connection
    ///     });
    /// }
    /// ```
    pub async fn listen(&self, port: u16) -> Result<PhoenixListener> {
        info!("Starting listener on port {}", port);

        let addr = SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 0], port));

        let listener = self.listeners.create_listener(
            addr,
            &self.config,
            self.metrics.clone(),
            self.security.clone(),
        ).await?;

        info!("Listener started on port {}", port);
        Ok(listener)
    }

    /// Get real-time performance metrics
    ///
    /// # Example
    /// ```rust
    /// let metrics = phoenix.metrics().await;
    /// println!("Throughput: {:.2} Gbps", metrics.throughput_gbps);
    /// println!("Active connections: {}", metrics.active_connections);
    /// ```
    pub async fn metrics(&self) -> PhoenixMetrics {
        let state = self.state.read();
        let transport_stats = self.transport.stats();

        PhoenixMetrics {
            app_name: self.app_name.clone(),
            uptime: state.start_time.elapsed(),
            bytes_sent: state.bytes_sent + transport_stats.bytes_sent,
            bytes_received: state.bytes_received + transport_stats.bytes_received,
            active_connections: self.active_connections.len(),
            total_connections: state.total_connections,
            throughput_gbps: transport_stats.throughput_gbps,
            avg_latency_us: transport_stats.avg_latency_us,
            performance_tier: self.config.performance_tier.clone(),
            security_level: self.config.security_level.clone(),
        }
    }

    /// Shutdown Phoenix and close all connections
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down Phoenix SDK");

        // Close all active connections
        for entry in self.active_connections.iter() {
            if let Err(e) = entry.value().close().await {
                warn!("Error closing connection to {}: {}", entry.key(), e);
            }
        }

        // Shutdown managers
        self.connections.shutdown().await;
        self.listeners.shutdown().await;

        // Shutdown transport and trustchain
        self.transport.shutdown().await;
        self.trustchain.shutdown().await?;

        info!("Phoenix SDK shutdown complete");
        Ok(())
    }

    // Internal helper methods

    fn create_stoq_config(config: &PhoenixConfig) -> Result<StoqConfig> {
        let transport_config = match &config.performance_tier {
            PerformanceTier::Development => TransportConfig {
                bind_address: "[::]:0".parse()?,
                target_throughput_gbps: 1.0,
                max_connections: 100,
                ..Default::default()
            },
            PerformanceTier::Production => TransportConfig {
                bind_address: "[::]:0".parse()?,
                target_throughput_gbps: 10.0,
                max_connections: 1000,
                ..Default::default()
            },
            PerformanceTier::HighThroughput => TransportConfig {
                bind_address: "[::]:0".parse()?,
                target_throughput_gbps: 40.0,
                max_connections: 10000,
                ..Default::default()
            },
            PerformanceTier::Custom(gbps) => TransportConfig {
                bind_address: "[::]:0".parse()?,
                target_throughput_gbps: *gbps as f64,
                max_connections: config.max_connections,
                ..Default::default()
            },
        };

        Ok(StoqConfig {
            transport: transport_config,
            ..Default::default()
        })
    }

    fn create_trustchain_config(config: &PhoenixConfig) -> Result<TrustChainConfig> {
        let trustchain_config = match &config.security_level {
            SecurityLevel::Development => TrustChainConfig::localhost_testing(),
            SecurityLevel::Standard | SecurityLevel::Enhanced | SecurityLevel::PostQuantum => {
                TrustChainConfig::production()
            }
        };

        Ok(trustchain_config)
    }

    fn parse_target(target: &str) -> Result<SocketAddr> {
        // Support various target formats
        if let Ok(addr) = target.parse::<SocketAddr>() {
            return Ok(addr);
        }

        // Try adding default port
        let with_port = if target.contains(':') {
            target.to_string()
        } else {
            format!("{}:{}", target, DEFAULT_PORT)
        };

        with_port.parse()
            .map_err(|e| PhoenixError::InvalidTarget {
                target: target.to_string(),
                reason: e.to_string(),
            })
    }

    async fn start_metrics_collection(&self) -> Result<()> {
        let metrics = self.metrics.clone();
        let transport = self.transport.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                let stats = transport.stats();
                metrics.update_transport_stats(stats);
            }
        });

        Ok(())
    }
}

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        Phoenix, PhoenixConfig, PhoenixConnection, PhoenixListener,
        PhoenixMetrics, PhoenixError, Result,
        PerformanceTier, SecurityLevel,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_phoenix_creation() {
        let phoenix = Phoenix::new("test-app").await.unwrap();
        assert_eq!(phoenix.app_name, "test-app");
    }

    #[tokio::test]
    async fn test_phoenix_with_config() {
        let config = PhoenixConfig::production("test-app");
        let phoenix = Phoenix::with_config(config).await.unwrap();
        assert_eq!(phoenix.app_name, "test-app");
    }

    #[tokio::test]
    async fn test_metrics() {
        let phoenix = Phoenix::new("test-app").await.unwrap();
        let metrics = phoenix.metrics().await;
        assert_eq!(metrics.app_name, "test-app");
        assert_eq!(metrics.active_connections, 0);
    }
}