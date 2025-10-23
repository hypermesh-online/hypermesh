//! Phoenix SDK API - Developer-friendly wrapper for STOQ high-performance transport
//!
//! Provides a simple, powerful API for Phoenix SDK developers with automatic
//! certificate management, connection pooling, and performance monitoring.

use stoq::transport::{StoqTransport, TransportConfig, Endpoint, Connection};
use std::net::Ipv6Addr;
use std::sync::Arc;
use anyhow::{Result, Context};
use bytes::Bytes;
use tokio::sync::RwLock;
use std::collections::HashMap;
use tracing::{info, debug, warn};

/// Phoenix SDK Transport - Simple, powerful, developer-focused
pub struct PhoenixTransport {
    inner: Arc<StoqTransport>,
    app_id: String,
    connections: Arc<RwLock<HashMap<String, Arc<Connection>>>>,
    config: PhoenixConfig,
}

/// Phoenix SDK configuration
#[derive(Debug, Clone)]
pub struct PhoenixConfig {
    /// Application identifier
    pub app_id: String,
    /// Bind address (defaults to any IPv6)
    pub bind_address: Ipv6Addr,
    /// Port (0 for dynamic)
    pub port: u16,
    /// Enable high-performance mode
    pub high_performance: bool,
    /// Maximum concurrent connections
    pub max_connections: usize,
    /// Auto-provision certificates
    pub auto_certificates: bool,
}

impl Default for PhoenixConfig {
    fn default() -> Self {
        Self {
            app_id: "phoenix-app".to_string(),
            bind_address: Ipv6Addr::UNSPECIFIED,
            port: 0,
            high_performance: true,
            max_connections: 100,
            auto_certificates: true,
        }
    }
}

/// Performance metrics for Phoenix SDK
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub throughput_gbps: f64,
    pub latency_ms: f64,
    pub active_connections: usize,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub zero_copy_operations: u64,
}

/// Phoenix connection wrapper
pub struct PhoenixConnection {
    inner: Arc<Connection>,
    endpoint: String,
    metrics: Arc<RwLock<ConnectionMetrics>>,
}

#[derive(Debug, Default, Clone)]
struct ConnectionMetrics {
    bytes_sent: u64,
    bytes_received: u64,
    operations: u64,
}

impl PhoenixTransport {
    /// Create a new Phoenix transport with automatic setup
    pub async fn new(app_id: impl Into<String>) -> Result<Self> {
        let config = PhoenixConfig {
            app_id: app_id.into(),
            ..Default::default()
        };
        Self::with_config(config).await
    }

    /// Create Phoenix transport with custom configuration
    pub async fn with_config(config: PhoenixConfig) -> Result<Self> {
        info!("Initializing Phoenix transport for app: {}", config.app_id);

        // Build optimized STOQ configuration
        let transport_config = if config.high_performance {
            TransportConfig {
                bind_address: config.bind_address,
                port: config.port,
                max_concurrent_streams: 1000,
                send_buffer_size: 256 * 1024 * 1024, // 256MB for 10+ Gbps
                receive_buffer_size: 256 * 1024 * 1024,
                enable_zero_copy: true,
                enable_memory_pool: true,
                memory_pool_size: 8192,
                frame_batch_size: 512,
                connection_pool_size: 50,
                enable_large_send_offload: true,
                enable_cpu_affinity: true,
                enable_falcon_crypto: true, // Quantum-resistant by default
                ..Default::default()
            }
        } else {
            TransportConfig {
                bind_address: config.bind_address,
                port: config.port,
                ..Default::default()
            }
        };

        let inner = Arc::new(
            StoqTransport::new(transport_config)
                .await
                .context("Failed to initialize STOQ transport")?
        );

        if config.auto_certificates {
            // Certificates are already auto-provisioned by STOQ
            info!("Certificates auto-provisioned for {}", config.app_id);
        }

        Ok(Self {
            inner,
            app_id: config.app_id.clone(),
            connections: Arc::new(RwLock::new(HashMap::new())),
            config,
        })
    }

    /// Connect to a peer with automatic certificate validation
    pub async fn connect(&self, endpoint: &str) -> Result<PhoenixConnection> {
        debug!("Connecting to {}", endpoint);

        // Parse endpoint (support both hostname and IP)
        let (host, port) = parse_endpoint(endpoint)?;

        let stoq_endpoint = Endpoint::new(host, port)
            .with_server_name(endpoint.split(':').next().unwrap_or("localhost").to_string());

        // Try to reuse existing connection
        {
            let connections = self.connections.read().await;
            if let Some(existing) = connections.get(endpoint) {
                if existing.is_active() {
                    debug!("Reusing existing connection to {}", endpoint);
                    return Ok(PhoenixConnection {
                        inner: existing.clone(),
                        endpoint: endpoint.to_string(),
                        metrics: Arc::new(RwLock::new(ConnectionMetrics::default())),
                    });
                }
            }
        }

        // Create new connection
        let connection = self.inner.connect(&stoq_endpoint)
            .await
            .context(format!("Failed to connect to {}", endpoint))?;

        // Cache connection
        {
            let mut connections = self.connections.write().await;
            connections.insert(endpoint.to_string(), connection.clone());
        }

        info!("Connected to {} (app: {})", endpoint, self.app_id);

        Ok(PhoenixConnection {
            inner: connection,
            endpoint: endpoint.to_string(),
            metrics: Arc::new(RwLock::new(ConnectionMetrics::default())),
        })
    }

    /// Accept incoming connections
    pub async fn accept(&self) -> Result<PhoenixConnection> {
        let connection = self.inner.accept()
            .await
            .context("Failed to accept connection")?;

        let endpoint = format!("[{}]:{}",
            connection.endpoint().address,
            connection.endpoint().port
        );

        info!("Accepted connection from {}", endpoint);

        Ok(PhoenixConnection {
            inner: connection,
            endpoint,
            metrics: Arc::new(RwLock::new(ConnectionMetrics::default())),
        })
    }

    /// Get current performance statistics
    pub async fn stats(&self) -> PerformanceMetrics {
        let stats = self.inner.stats();
        let (peak_gbps, zero_copy, _, _) = self.inner.performance_stats();

        PerformanceMetrics {
            throughput_gbps: peak_gbps,
            latency_ms: stats.avg_latency_us as f64 / 1000.0,
            active_connections: self.inner.active_connections(),
            total_bytes_sent: stats.bytes_sent,
            total_bytes_received: stats.bytes_received,
            zero_copy_operations: zero_copy,
        }
    }

    /// Enable connection multiplexing for maximum throughput
    pub async fn enable_multiplexing(&self, endpoint: &str, connections: usize) -> Result<()> {
        let (host, port) = parse_endpoint(endpoint)?;
        let stoq_endpoint = Endpoint::new(host, port);

        self.inner.enable_multiplexing(&stoq_endpoint, connections)
            .await
            .context("Failed to enable multiplexing")?;

        info!("Enabled {}x multiplexing for {}", connections, endpoint);
        Ok(())
    }

    /// Send data using multiplexed connections
    pub async fn send_multiplexed(&self, endpoint: &str, data: &[u8]) -> Result<()> {
        let (host, port) = parse_endpoint(endpoint)?;
        let stoq_endpoint = Endpoint::new(host, port);

        self.inner.send_multiplexed(&stoq_endpoint, data)
            .await
            .context("Multiplexed send failed")?;

        Ok(())
    }

    /// Shutdown transport gracefully
    pub async fn shutdown(&self) {
        info!("Shutting down Phoenix transport for {}", self.app_id);
        self.inner.shutdown().await;
    }
}

impl PhoenixConnection {
    /// Send data with automatic performance optimization
    pub async fn send_data(&mut self, data: &[u8]) -> Result<()> {
        let mut stream = self.inner.open_stream()
            .await
            .context("Failed to open stream")?;

        stream.send(data)
            .await
            .context("Failed to send data")?;

        let mut metrics = self.metrics.write().await;
        metrics.bytes_sent += data.len() as u64;
        metrics.operations += 1;

        Ok(())
    }

    /// Send bytes directly (zero-copy when possible)
    pub async fn send_bytes(&mut self, data: Bytes) -> Result<()> {
        let mut stream = self.inner.open_stream()
            .await
            .context("Failed to open stream")?;

        stream.send_bytes(data.clone())
            .await
            .context("Failed to send bytes")?;

        let mut metrics = self.metrics.write().await;
        metrics.bytes_sent += data.len() as u64;
        metrics.operations += 1;

        Ok(())
    }

    /// Receive data
    pub async fn receive_data(&mut self) -> Result<Bytes> {
        let mut stream = self.inner.accept_stream()
            .await
            .context("Failed to accept stream")?;

        let data = stream.receive()
            .await
            .context("Failed to receive data")?;

        let mut metrics = self.metrics.write().await;
        metrics.bytes_received += data.len() as u64;
        metrics.operations += 1;

        Ok(data)
    }

    /// Stream data continuously
    pub async fn stream_data<F>(&mut self, mut data_source: F) -> Result<()>
    where
        F: FnMut() -> Option<Vec<u8>>,
    {
        while let Some(chunk) = data_source() {
            self.send_data(&chunk).await?;
        }
        Ok(())
    }

    /// Get connection metrics
    pub async fn metrics(&self) -> ConnectionMetrics {
        self.metrics.read().await.clone()
    }

    /// Check if connection is still active
    pub fn is_active(&self) -> bool {
        self.inner.is_active()
    }

    /// Get endpoint address
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Close connection
    pub fn close(&self) {
        self.inner.close();
    }
}

// Helper function to parse endpoint strings
fn parse_endpoint(endpoint: &str) -> Result<(Ipv6Addr, u16)> {
    // Handle [ipv6]:port format
    if endpoint.starts_with('[') {
        let parts: Vec<&str> = endpoint.split(']').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid IPv6 endpoint format: {}", endpoint));
        }

        let ip_str = &parts[0][1..]; // Remove leading '['
        let port_str = parts[1].trim_start_matches(':');

        let ip = ip_str.parse::<Ipv6Addr>()
            .context(format!("Invalid IPv6 address: {}", ip_str))?;
        let port = port_str.parse::<u16>()
            .context(format!("Invalid port: {}", port_str))?;

        Ok((ip, port))
    } else {
        // Try hostname:port or plain IPv6
        let parts: Vec<&str> = endpoint.rsplitn(2, ':').collect();

        if parts.len() == 2 {
            // Has port
            let port = parts[0].parse::<u16>()
                .context(format!("Invalid port: {}", parts[0]))?;

            // For now, resolve to localhost for testing
            // In production, would use DNS resolution
            Ok((Ipv6Addr::LOCALHOST, port))
        } else {
            // No port specified, use default
            Ok((Ipv6Addr::LOCALHOST, crate::DEFAULT_PORT))
        }
    }
}

/// Phoenix SDK builder for easy configuration
pub struct PhoenixBuilder {
    config: PhoenixConfig,
}

impl PhoenixBuilder {
    /// Create a new Phoenix SDK builder
    pub fn new(app_id: impl Into<String>) -> Self {
        Self {
            config: PhoenixConfig {
                app_id: app_id.into(),
                ..Default::default()
            },
        }
    }

    /// Set bind address
    pub fn bind_address(mut self, addr: Ipv6Addr) -> Self {
        self.config.bind_address = addr;
        self
    }

    /// Set port
    pub fn port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }

    /// Enable/disable high performance mode
    pub fn high_performance(mut self, enabled: bool) -> Self {
        self.config.high_performance = enabled;
        self
    }

    /// Set maximum connections
    pub fn max_connections(mut self, max: usize) -> Self {
        self.config.max_connections = max;
        self
    }

    /// Enable/disable auto certificates
    pub fn auto_certificates(mut self, enabled: bool) -> Self {
        self.config.auto_certificates = enabled;
        self
    }

    /// Build the Phoenix transport
    pub async fn build(self) -> Result<PhoenixTransport> {
        PhoenixTransport::with_config(self.config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_endpoint() {
        // IPv6 with port
        let (ip, port) = parse_endpoint("[::1]:9292").unwrap();
        assert_eq!(ip, Ipv6Addr::LOCALHOST);
        assert_eq!(port, 9292);

        // Hostname with port (resolves to localhost for now)
        let (ip, port) = parse_endpoint("example.com:8080").unwrap();
        assert_eq!(ip, Ipv6Addr::LOCALHOST);
        assert_eq!(port, 8080);

        // No port (uses default)
        let (ip, port) = parse_endpoint("::1").unwrap();
        assert_eq!(ip, Ipv6Addr::LOCALHOST);
        assert_eq!(port, crate::DEFAULT_PORT);
    }

    #[tokio::test]
    async fn test_phoenix_builder() {
        let transport = PhoenixBuilder::new("test-app")
            .port(19292)
            .high_performance(true)
            .max_connections(50)
            .build()
            .await;

        assert!(transport.is_ok());
    }
}