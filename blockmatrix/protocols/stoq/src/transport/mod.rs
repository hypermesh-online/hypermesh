//! STOQ Transport Layer - QUIC over IPv6 implementation

use async_trait::async_trait;
use quinn::{self, ClientConfig, ServerConfig, TransportConfig as QuinnTransportConfig};
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use std::net::{SocketAddr, Ipv6Addr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use anyhow::{Result, anyhow};
use bytes::Bytes;
use parking_lot::RwLock;
use dashmap::DashMap;
use tracing::{info, debug, warn};
use serde::{Serialize, Deserialize};

pub mod certificates;
pub mod streams;
pub mod metrics;
pub mod adaptive;

use certificates::CertificateManager;
use metrics::TransportMetrics;
use adaptive::AdaptiveBandwidthDetector;

/// STOQ Transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    /// Bind address (IPv6 only)
    pub bind_address: Ipv6Addr,
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
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            bind_address: Ipv6Addr::UNSPECIFIED,
            port: crate::DEFAULT_PORT,
            max_connections: None, // Unlimited by default
            connection_timeout: Duration::from_secs(10),
            enable_migration: true,
            enable_0rtt: true,
            max_idle_timeout: Duration::from_secs(30),
            cert_rotation_interval: Duration::from_secs(24 * 60 * 60),
            max_concurrent_streams: 100,
            send_buffer_size: 1024 * 1024, // 1MB
            receive_buffer_size: 1024 * 1024, // 1MB
        }
    }
}

/// Connection endpoint information
#[derive(Debug, Clone)]
pub struct Endpoint {
    /// IPv6 address
    pub address: Ipv6Addr,
    /// Port number
    pub port: u16,
    /// Optional server name for SNI
    pub server_name: Option<String>,
}

impl Endpoint {
    /// Create a new endpoint
    pub fn new(address: Ipv6Addr, port: u16) -> Self {
        Self {
            address,
            port,
            server_name: None,
        }
    }
    
    /// Set server name for SNI
    pub fn with_server_name(mut self, name: String) -> Self {
        self.server_name = Some(name);
        self
    }
    
    /// Convert to socket address
    pub fn to_socket_addr(&self) -> SocketAddr {
        SocketAddr::from((self.address, self.port))
    }
}

/// Active QUIC connection
pub struct Connection {
    inner: quinn::Connection,
    endpoint: Endpoint,
    metrics: Arc<TransportMetrics>,
}

impl Connection {
    /// Get the connection ID
    pub fn id(&self) -> String {
        format!("{:?}", self.inner.stable_id())
    }
    
    /// Get the remote endpoint
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }
    
    /// Open a new bidirectional stream
    pub async fn open_stream(&self) -> Result<Stream> {
        let (send, recv) = self.inner.open_bi().await?;
        Ok(Stream::new(send, recv, self.metrics.clone()))
    }
    
    /// Accept an incoming bidirectional stream
    pub async fn accept_stream(&self) -> Result<Stream> {
        let (send, recv) = self.inner.accept_bi().await?;
        Ok(Stream::new(send, recv, self.metrics.clone()))
    }
    
    /// Check if connection is still active
    pub fn is_active(&self) -> bool {
        self.inner.close_reason().is_none()
    }
    
    /// Close the connection gracefully
    pub fn close(&self) {
        self.inner.close(0u32.into(), b"closing");
    }
}

/// Bidirectional stream over a connection
pub struct Stream {
    send: quinn::SendStream,
    recv: quinn::RecvStream,
    metrics: Arc<TransportMetrics>,
}

impl Stream {
    fn new(send: quinn::SendStream, recv: quinn::RecvStream, metrics: Arc<TransportMetrics>) -> Self {
        Self { send, recv, metrics }
    }
    
    /// Send data over the stream
    pub async fn send(&mut self, data: &[u8]) -> Result<()> {
        self.send.write_all(data).await?;
        self.send.finish().await?;
        self.metrics.record_bytes_sent(data.len());
        Ok(())
    }
    
    /// Receive data from the stream
    pub async fn receive(&mut self) -> Result<Bytes> {
        let data = self.recv.read_to_end(crate::STOQ_MTU).await?;
        self.metrics.record_bytes_received(data.len());
        Ok(data.into())
    }
}

/// Main STOQ transport implementation
pub struct StoqTransport {
    config: TransportConfig,
    endpoint: Arc<quinn::Endpoint>,
    connections: Arc<DashMap<String, Arc<Connection>>>,
    cert_manager: Arc<CertificateManager>,
    metrics: Arc<TransportMetrics>,
    adaptive_detector: Arc<AdaptiveBandwidthDetector>,
}

impl StoqTransport {
    /// Create a new STOQ transport
    pub async fn new(config: TransportConfig) -> Result<Self> {
        info!("Initializing STOQ transport on [{}]:{}", config.bind_address, config.port);
        
        // Initialize certificate manager
        let cert_manager = Arc::new(CertificateManager::new(config.cert_rotation_interval)?);
        
        // Configure QUIC transport
        let mut transport_config = QuinnTransportConfig::default();
        transport_config.max_concurrent_bidi_streams(config.max_concurrent_streams.into());
        transport_config.max_idle_timeout(Some(config.max_idle_timeout.try_into()?));
        // Note: enable_migration is not available in this Quinn version
        // transport_config.enable_migration(config.enable_migration);
        
        // Create server configuration
        let server_config = ServerConfig::with_crypto(Arc::new(
            cert_manager.server_crypto_config()?
        ));
        
        // Create client configuration
        let client_config = ClientConfig::with_native_roots();
        
        // Bind to IPv6 address
        let socket_addr = SocketAddr::from((config.bind_address, config.port));
        let endpoint = quinn::Endpoint::server(server_config, socket_addr)?;
        
        // Initialize metrics
        let metrics = Arc::new(TransportMetrics::new());

        // Initialize adaptive bandwidth detector
        let adaptive_detector = Arc::new(AdaptiveBandwidthDetector::new());

        Ok(Self {
            config,
            endpoint: Arc::new(endpoint),
            connections: Arc::new(DashMap::new()),
            cert_manager,
            metrics,
            adaptive_detector,
        })
    }
    
    /// Connect to a remote endpoint
    pub async fn connect(&self, endpoint: &Endpoint) -> Result<Arc<Connection>> {
        debug!("Connecting to [{}]:{}", endpoint.address, endpoint.port);
        
        let socket_addr = endpoint.to_socket_addr();
        let connecting = self.endpoint.connect(socket_addr, endpoint.server_name.as_deref().unwrap_or("localhost"))?;
        
        let quinn_conn = connecting.await?;
        
        let connection = Arc::new(Connection {
            inner: quinn_conn,
            endpoint: endpoint.clone(),
            metrics: self.metrics.clone(),
        });
        
        self.connections.insert(connection.id(), connection.clone());
        self.metrics.record_connection_established();
        
        info!("Connected to {}", socket_addr);
        Ok(connection)
    }
    
    /// Accept incoming connections
    pub async fn accept(&self) -> Result<Arc<Connection>> {
        let incoming = self.endpoint.accept().await.ok_or_else(|| anyhow!("No incoming connection"))?;
        let quinn_conn = incoming.await?;
        
        let remote_addr = quinn_conn.remote_address();
        let endpoint = Endpoint::new(
            match remote_addr {
                SocketAddr::V6(addr) => *addr.ip(),
                _ => return Err(anyhow!("IPv4 not supported")),
            },
            remote_addr.port(),
        );
        
        let connection = Arc::new(Connection {
            inner: quinn_conn,
            endpoint,
            metrics: self.metrics.clone(),
        });
        
        self.connections.insert(connection.id(), connection.clone());
        self.metrics.record_connection_established();
        
        info!("Accepted connection from {}", remote_addr);
        Ok(connection)
    }
    
    /// Send data to a connection
    pub async fn send(&self, conn: &Connection, data: &[u8]) -> Result<()> {
        let start_time = Instant::now();
        let mut stream = conn.open_stream().await?;
        stream.send(data).await?;
        let transfer_time = start_time.elapsed();

        // Record bandwidth measurement for adaptive detection
        self.adaptive_detector.record_transfer(data.len() as u64, transfer_time);

        Ok(())
    }
    
    /// Receive data from a connection
    pub async fn receive(&self, conn: &Connection) -> Result<Bytes> {
        let start_time = Instant::now();
        let mut stream = conn.accept_stream().await?;
        let data = stream.receive().await?;
        let transfer_time = start_time.elapsed();

        // Record bandwidth measurement for adaptive detection
        self.adaptive_detector.record_transfer(data.len() as u64, transfer_time);

        Ok(data)
    }
    
    /// Get transport statistics
    pub fn stats(&self) -> crate::TransportStats {
        self.metrics.get_stats(self.connections.len())
    }
    
    /// Get active connections count
    pub fn active_connections(&self) -> usize {
        self.connections.len()
    }
    
    /// Get the current detected network tier
    pub fn network_tier(&self) -> adaptive::NetworkTier {
        self.adaptive_detector.current_tier()
    }

    /// Get adaptive bandwidth detector statistics
    pub fn adaptive_stats(&self) -> adaptive::DetectorStats {
        self.adaptive_detector.get_stats()
    }

    /// Force a bandwidth tier analysis and update
    pub fn force_tier_update(&self) -> bool {
        self.adaptive_detector.force_update()
    }

    /// Apply current adaptive configuration to the transport
    pub async fn apply_adaptive_config(&mut self) -> Result<()> {
        let mut new_config = self.config.clone();
        self.adaptive_detector.apply_to_transport_config(&mut new_config);

        // Only update if configuration actually changed
        if new_config.max_concurrent_streams != self.config.max_concurrent_streams ||
           new_config.send_buffer_size != self.config.send_buffer_size ||
           new_config.receive_buffer_size != self.config.receive_buffer_size {

            info!(
                "Applying adaptive configuration for {} tier",
                self.adaptive_detector.current_tier().description()
            );

            self.config = new_config;

            // Note: In a real implementation, we would need to update the Quinn
            // transport configuration here, but Quinn doesn't support runtime
            // configuration changes. This would require reconnection.

            debug!("Adaptive configuration applied (requires reconnection for full effect)");
        }

        Ok(())
    }

    /// Close all connections
    pub async fn shutdown(&self) {
        info!("Shutting down STOQ transport");
        for conn in self.connections.iter() {
            conn.close();
        }
        self.connections.clear();
        self.endpoint.close(0u32.into(), b"shutdown");
    }
}

#[async_trait]
impl crate::Transport for StoqTransport {
    async fn connect(&self, endpoint: &Endpoint) -> Result<Connection> {
        Ok((*self.connect(endpoint).await?).clone())
    }
    
    async fn listen(&self, addr: SocketAddr) -> Result<Box<dyn crate::Listener>> {
        // For standalone STOQ, listening is handled by accept()
        unimplemented!("Use accept() method for listening")
    }
    
    async fn send(&self, conn: &Connection, data: &[u8]) -> Result<()> {
        self.send(conn, data).await
    }
    
    async fn receive(&self, conn: &Connection) -> Result<Bytes> {
        self.receive(conn).await
    }
    
    fn stats(&self) -> crate::TransportStats {
        self.stats()
    }
}

// Helper trait implementations
impl Clone for Connection {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            endpoint: self.endpoint.clone(),
            metrics: self.metrics.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_endpoint_creation() {
        let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 9292);
        assert_eq!(endpoint.port, 9292);
        assert_eq!(endpoint.address, Ipv6Addr::LOCALHOST);
    }
    
    #[test]
    fn test_transport_config_default() {
        let config = TransportConfig::default();
        assert_eq!(config.port, 9292);
        assert!(config.enable_migration);
        assert!(config.enable_0rtt);
    }
    
    #[tokio::test]
    async fn test_transport_creation() {
        let mut config = TransportConfig::default();
        // Use port 0 for dynamic port assignment to avoid conflicts in concurrent tests
        config.port = 0;
        let transport = StoqTransport::new(config).await;
        assert!(transport.is_ok());
    }
}