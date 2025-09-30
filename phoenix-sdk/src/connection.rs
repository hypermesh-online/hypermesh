//! Phoenix Connection Management
//!
//! Provides high-performance connection handling with automatic optimization,
//! compression, and monitoring.

use std::sync::Arc;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use bytes::Bytes;
use tokio::sync::mpsc;
use tracing::{debug, warn, error};
use serde::{Serialize, Deserialize};
use stoq::{StoqTransport, Connection as StoqConnection, Stream as StoqStream};

use crate::{
    config::PhoenixConfig,
    metrics::{MetricsCollector, ConnectionMetrics},
    security::SecurityManager,
    compression::CompressionEngine,
    errors::{PhoenixError, Result},
};

/// Phoenix connection with automatic optimization
#[derive(Clone)]
pub struct PhoenixConnection {
    /// Connection ID for tracking
    id: String,
    /// Remote address
    remote_addr: SocketAddr,
    /// Underlying STOQ connection
    inner: Arc<StoqConnection>,
    /// Connection state
    state: Arc<RwLock<ConnectionState>>,
    /// Metrics collector
    metrics: Arc<MetricsCollector>,
    /// Security context
    security: Arc<SecurityManager>,
    /// Compression engine
    compression: Option<Arc<CompressionEngine>>,
    /// Connection configuration
    config: ConnectionConfig,
}

/// Connection state tracking
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    /// Connection is being established
    Connecting,
    /// Connection is active and ready
    Connected,
    /// Connection is idle
    Idle,
    /// Connection is closing
    Closing,
    /// Connection is closed
    Closed,
}

/// Connection-specific configuration
#[derive(Debug, Clone)]
struct ConnectionConfig {
    /// Enable compression
    enable_compression: bool,
    /// Compression threshold in bytes
    compression_threshold: usize,
    /// Send buffer size
    send_buffer_size: usize,
    /// Receive buffer size
    recv_buffer_size: usize,
    /// Keep-alive interval
    keep_alive_interval: Option<Duration>,
}

impl PhoenixConnection {
    /// Create new Phoenix connection
    pub(crate) fn new(
        inner: StoqConnection,
        remote_addr: SocketAddr,
        config: &PhoenixConfig,
        metrics: Arc<MetricsCollector>,
        security: Arc<SecurityManager>,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();

        let compression = if config.enable_compression {
            Some(Arc::new(CompressionEngine::new(
                config.performance_tier.clone(),
            )))
        } else {
            None
        };

        let conn_config = ConnectionConfig {
            enable_compression: config.enable_compression,
            compression_threshold: 1024, // Compress data larger than 1KB
            send_buffer_size: config.performance_tier.buffer_size(),
            recv_buffer_size: config.performance_tier.buffer_size(),
            keep_alive_interval: Some(Duration::from_secs(30)),
        };

        Self {
            id: id.clone(),
            remote_addr,
            inner: Arc::new(inner),
            state: Arc::new(RwLock::new(ConnectionState::Connected)),
            metrics,
            security,
            compression,
            config: conn_config,
        }
    }

    /// Send data with automatic optimization
    ///
    /// # Example
    /// ```rust
    /// conn.send(&"Hello, Phoenix!").await?;
    /// conn.send(&MyStruct { field: "value" }).await?;
    /// ```
    pub async fn send<T: Serialize>(&self, data: &T) -> Result<()> {
        // Check connection state
        if !self.is_connected() {
            return Err(PhoenixError::ConnectionClosed);
        }

        let start = Instant::now();

        // Serialize data
        let serialized = bincode::serialize(data)
            .map_err(|e| PhoenixError::SerializationError(e.to_string()))?;

        // Compress if enabled and above threshold
        let payload = if self.config.enable_compression && serialized.len() > self.config.compression_threshold {
            if let Some(compression) = &self.compression {
                compression.compress(&serialized)?
            } else {
                serialized
            }
        } else {
            serialized
        };

        // Send through STOQ
        self.send_raw(Bytes::from(payload)).await?;

        // Update metrics
        let elapsed = start.elapsed();
        self.metrics.record_send(serialized.len(), elapsed);

        Ok(())
    }

    /// Receive data with automatic deserialization
    ///
    /// # Example
    /// ```rust
    /// let message: String = conn.receive().await?;
    /// let data: MyStruct = conn.receive().await?;
    /// ```
    pub async fn receive<T: for<'de> Deserialize<'de>>(&self) -> Result<T> {
        // Check connection state
        if !self.is_connected() {
            return Err(PhoenixError::ConnectionClosed);
        }

        let start = Instant::now();

        // Receive raw data
        let payload = self.receive_raw().await?;

        // Decompress if needed
        let data = if self.config.enable_compression {
            if let Some(compression) = &self.compression {
                if compression.is_compressed(&payload) {
                    compression.decompress(&payload)?
                } else {
                    payload.to_vec()
                }
            } else {
                payload.to_vec()
            }
        } else {
            payload.to_vec()
        };

        // Deserialize
        let result = bincode::deserialize(&data)
            .map_err(|e| PhoenixError::DeserializationError(e.to_string()))?;

        // Update metrics
        let elapsed = start.elapsed();
        self.metrics.record_receive(data.len(), elapsed);

        Ok(result)
    }

    /// Send raw bytes without serialization
    pub async fn send_raw(&self, data: Bytes) -> Result<()> {
        let mut stream = self.inner.open_uni().await
            .map_err(|e| PhoenixError::TransportError(e.to_string()))?;

        stream.write_all(&data).await
            .map_err(|e| PhoenixError::TransportError(e.to_string()))?;

        stream.finish().await
            .map_err(|e| PhoenixError::TransportError(e.to_string()))?;

        Ok(())
    }

    /// Receive raw bytes without deserialization
    pub async fn receive_raw(&self) -> Result<Bytes> {
        let mut stream = self.inner.accept_uni().await
            .map_err(|e| PhoenixError::TransportError(e.to_string()))?;

        let data = stream.read_to_end(65536).await
            .map_err(|e| PhoenixError::TransportError(e.to_string()))?;

        Ok(Bytes::from(data))
    }

    /// Open bidirectional stream for streaming data
    pub async fn open_stream(&self) -> Result<PhoenixStream> {
        let stream = self.inner.open_bi().await
            .map_err(|e| PhoenixError::TransportError(e.to_string()))?;

        Ok(PhoenixStream::new(
            stream,
            self.compression.clone(),
            self.metrics.clone(),
        ))
    }

    /// Accept bidirectional stream from remote
    pub async fn accept_stream(&self) -> Result<PhoenixStream> {
        let stream = self.inner.accept_bi().await
            .map_err(|e| PhoenixError::TransportError(e.to_string()))?;

        Ok(PhoenixStream::new(
            stream,
            self.compression.clone(),
            self.metrics.clone(),
        ))
    }

    /// Get connection metrics
    pub fn metrics(&self) -> ConnectionMetrics {
        ConnectionMetrics {
            connection_id: self.id.clone(),
            remote_addr: self.remote_addr,
            state: self.state.read().clone(),
            bytes_sent: self.metrics.get_connection_bytes_sent(&self.id),
            bytes_received: self.metrics.get_connection_bytes_received(&self.id),
            round_trip_time: self.get_rtt(),
            congestion_window: self.get_congestion_window(),
        }
    }

    /// Check if connection is active
    pub fn is_connected(&self) -> bool {
        matches!(*self.state.read(), ConnectionState::Connected)
    }

    /// Get connection ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get remote address
    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }

    /// Get round-trip time
    pub fn get_rtt(&self) -> Duration {
        self.inner.rtt()
    }

    /// Get congestion window
    pub fn get_congestion_window(&self) -> u64 {
        // This would need to be implemented in the STOQ layer
        0
    }

    /// Close connection gracefully
    pub async fn close(&self) -> Result<()> {
        *self.state.write() = ConnectionState::Closing;

        self.inner.close(0u32.into(), b"closing");

        *self.state.write() = ConnectionState::Closed;

        Ok(())
    }
}

/// Phoenix stream for bidirectional streaming
pub struct PhoenixStream {
    inner: (StoqStream, StoqStream),
    compression: Option<Arc<CompressionEngine>>,
    metrics: Arc<MetricsCollector>,
}

impl PhoenixStream {
    fn new(
        inner: (StoqStream, StoqStream),
        compression: Option<Arc<CompressionEngine>>,
        metrics: Arc<MetricsCollector>,
    ) -> Self {
        Self {
            inner,
            compression,
            metrics,
        }
    }

    /// Send data through stream
    pub async fn send<T: Serialize>(&mut self, data: &T) -> Result<()> {
        let serialized = bincode::serialize(data)
            .map_err(|e| PhoenixError::SerializationError(e.to_string()))?;

        let payload = if let Some(compression) = &self.compression {
            compression.compress(&serialized)?
        } else {
            serialized
        };

        self.inner.0.write_all(&payload).await
            .map_err(|e| PhoenixError::TransportError(e.to_string()))?;

        Ok(())
    }

    /// Receive data from stream
    pub async fn receive<T: for<'de> Deserialize<'de>>(&mut self) -> Result<T> {
        let mut buffer = vec![0u8; 65536];
        let n = self.inner.1.read(&mut buffer).await
            .map_err(|e| PhoenixError::TransportError(e.to_string()))?
            .ok_or(PhoenixError::ConnectionClosed)?;

        buffer.truncate(n);

        let data = if let Some(compression) = &self.compression {
            if compression.is_compressed(&buffer) {
                compression.decompress(&buffer)?
            } else {
                buffer
            }
        } else {
            buffer
        };

        bincode::deserialize(&data)
            .map_err(|e| PhoenixError::DeserializationError(e.to_string()))
    }

    /// Close stream
    pub async fn close(mut self) -> Result<()> {
        self.inner.0.finish().await
            .map_err(|e| PhoenixError::TransportError(e.to_string()))?;

        Ok(())
    }
}

/// Connection manager for handling multiple connections
pub(crate) struct ConnectionManager {
    transport: Arc<StoqTransport>,
    max_connections: usize,
    connections: Arc<dashmap::DashMap<String, Arc<PhoenixConnection>>>,
}

impl ConnectionManager {
    pub fn new(transport: Arc<StoqTransport>, max_connections: usize) -> Self {
        Self {
            transport,
            max_connections,
            connections: Arc::new(dashmap::DashMap::new()),
        }
    }

    pub async fn connect(
        &self,
        addr: SocketAddr,
        config: &PhoenixConfig,
        metrics: Arc<MetricsCollector>,
        security: Arc<SecurityManager>,
    ) -> Result<Arc<PhoenixConnection>> {
        // Check connection limit
        if self.connections.len() >= self.max_connections {
            return Err(PhoenixError::ConnectionLimitExceeded);
        }

        // Create STOQ connection
        let endpoint = self.transport.endpoint(addr).await
            .map_err(|e| PhoenixError::TransportError(e.to_string()))?;

        let stoq_conn = self.transport.connect(&endpoint).await
            .map_err(|e| PhoenixError::TransportError(e.to_string()))?;

        // Create Phoenix connection
        let conn = Arc::new(PhoenixConnection::new(
            stoq_conn,
            addr,
            config,
            metrics,
            security,
        ));

        // Track connection
        self.connections.insert(conn.id.clone(), conn.clone());

        Ok(conn)
    }

    pub async fn shutdown(&self) {
        for entry in self.connections.iter() {
            if let Err(e) = entry.value().close().await {
                warn!("Error closing connection {}: {}", entry.key(), e);
            }
        }
        self.connections.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_state() {
        let state = ConnectionState::Connecting;
        assert_eq!(state, ConnectionState::Connecting);

        let state = ConnectionState::Connected;
        assert_eq!(state, ConnectionState::Connected);
    }

    #[test]
    fn test_connection_config() {
        let config = ConnectionConfig {
            enable_compression: true,
            compression_threshold: 1024,
            send_buffer_size: 65536,
            recv_buffer_size: 65536,
            keep_alive_interval: Some(Duration::from_secs(30)),
        };

        assert!(config.enable_compression);
        assert_eq!(config.compression_threshold, 1024);
        assert_eq!(config.send_buffer_size, 65536);
    }
}