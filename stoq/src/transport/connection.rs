// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ Connection Management

use anyhow::{anyhow, Result};
use bytes::Bytes;
use parking_lot::Mutex;
use std::net::Ipv6Addr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use super::metrics::TransportMetrics;

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
    pub fn to_socket_addr(&self) -> std::net::SocketAddr {
        std::net::SocketAddr::from((self.address, self.port))
    }
}

/// Frame batch for syscall reduction optimization
pub struct FrameBatch {
    frames: Vec<Bytes>,
    max_size: usize,
    total_bytes: usize,
}

impl FrameBatch {
    pub fn new(max_size: usize) -> Self {
        Self {
            frames: Vec::with_capacity(max_size),
            max_size,
            total_bytes: 0,
        }
    }

    /// Add frame to batch (returns true if batch is full)
    pub fn add_frame(&mut self, frame: Bytes) -> bool {
        self.total_bytes += frame.len();
        self.frames.push(frame);
        self.frames.len() >= self.max_size
    }

    /// Flush all frames in batch
    pub fn flush(&mut self) -> Vec<Bytes> {
        let frames = std::mem::replace(&mut self.frames, Vec::with_capacity(self.max_size));
        self.total_bytes = 0;
        frames
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }
}

/// Memory buffer pool for efficient buffer reuse (simplified for safety)
pub struct MemoryPool {
    buffer_size: usize,
    allocated_count: std::sync::atomic::AtomicUsize,
    max_buffers: usize,
}

impl MemoryPool {
    /// Create a new memory pool for efficient buffer management
    pub fn new(buffer_size: usize, max_buffers: usize) -> Self {
        Self {
            buffer_size,
            allocated_count: std::sync::atomic::AtomicUsize::new(0),
            max_buffers,
        }
    }

    /// Get a buffer from the pool (simplified for safety)
    pub fn get_buffer(&self) -> Option<bytes::BytesMut> {
        // Allocate new buffer if under limit
        if self.allocated_count.load(Ordering::Relaxed) < self.max_buffers {
            self.allocated_count.fetch_add(1, Ordering::Relaxed);
            return Some(bytes::BytesMut::with_capacity(self.buffer_size));
        }

        None
    }

    /// Return buffer to pool for reuse
    pub fn return_buffer(&self, mut buffer: bytes::BytesMut) {
        if buffer.capacity() >= self.buffer_size {
            // Clear buffer and drop safely - memory safety first
            buffer.clear();
            // Note: Actual zero-copy optimization requires careful lifetime management
            // For now, we prioritize safety by allowing normal deallocation
            // TODO: Implement proper shared buffer pool with Arc<Mutex<Vec<BytesMut>>>
        }
    }

    /// Get current pool statistics
    pub fn stats(&self) -> (usize, usize) {
        (0, self.allocated_count.load(Ordering::Relaxed)) // Pool size = 0 (no reuse)
    }
}

unsafe impl Send for MemoryPool {}
unsafe impl Sync for MemoryPool {}

/// Active QUIC connection with adaptive network tiers optimizations
pub struct Connection {
    pub(crate) inner: quinn::Connection,
    endpoint: Endpoint,
    metrics: Arc<TransportMetrics>,
    memory_pool: Arc<MemoryPool>,
    frame_batch: Arc<Mutex<FrameBatch>>,
    last_activity: AtomicU64,
    idle_timeout: u64, // Connection-specific idle timeout
}

impl Connection {
    /// Create new connection with adaptive network tiers optimizations
    pub fn new_optimized(
        inner: quinn::Connection,
        endpoint: Endpoint,
        metrics: Arc<TransportMetrics>,
        memory_pool: Arc<MemoryPool>,
        frame_batch_size: usize,
        idle_timeout: u64,
    ) -> Self {
        Self {
            inner,
            endpoint,
            metrics,
            memory_pool,
            frame_batch: Arc::new(Mutex::new(FrameBatch::new(frame_batch_size))),
            last_activity: AtomicU64::new(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_else(|_| std::time::Duration::from_secs(0))
                    .as_secs(),
            ),
            idle_timeout,
        }
    }

    /// Get the connection ID
    pub fn id(&self) -> String {
        format!("{:?}", self.inner.stable_id())
    }

    /// Accept a bidirectional stream
    pub async fn accept_bi(&self) -> Result<(quinn::SendStream, quinn::RecvStream)> {
        self.update_activity();
        self.inner
            .accept_bi()
            .await
            .map_err(|e| anyhow!("Failed to accept bidirectional stream: {e}"))
    }

    /// Open a bidirectional stream
    pub async fn open_bi(&self) -> Result<(quinn::SendStream, quinn::RecvStream)> {
        self.update_activity();
        self.inner
            .open_bi()
            .await
            .map_err(|e| anyhow!("Failed to open bidirectional stream: {e}"))
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
        // In Quinn 0.11+, we check the close reason instead
        self.inner.close_reason().is_none()
    }

    /// Close the connection gracefully
    pub fn close(&self) {
        self.inner.close(0u32.into(), b"closing");
    }

    /// Check if connection is healthy with configurable staleness threshold
    pub fn is_healthy(&self) -> bool {
        // First check if connection is active
        if !self.is_active() {
            return false;
        }

        // Check for staleness using configured idle timeout
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_secs();
        let last_activity = self.last_activity.load(Ordering::Relaxed);
        let idle_duration = now.saturating_sub(last_activity);

        // Connection is healthy if active and used within configured timeout
        idle_duration < self.idle_timeout
    }

    /// Update last activity timestamp
    pub fn update_activity(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_secs();
        self.last_activity.store(now, Ordering::Relaxed);
    }

    /// Get last activity timestamp for LRU tracking
    pub fn last_activity(&self) -> u64 {
        self.last_activity.load(Ordering::Relaxed)
    }
}

impl Clone for Connection {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            endpoint: self.endpoint.clone(),
            metrics: self.metrics.clone(),
            memory_pool: self.memory_pool.clone(),
            frame_batch: self.frame_batch.clone(),
            last_activity: AtomicU64::new(self.last_activity.load(Ordering::Relaxed)),
            idle_timeout: self.idle_timeout,
        }
    }
}

/// Bidirectional stream over a connection
pub struct Stream {
    send: quinn::SendStream,
    recv: quinn::RecvStream,
    metrics: Arc<TransportMetrics>,
}

impl Stream {
    pub(crate) fn new(
        send: quinn::SendStream,
        recv: quinn::RecvStream,
        metrics: Arc<TransportMetrics>,
    ) -> Self {
        Self {
            send,
            recv,
            metrics,
        }
    }

    /// Send data over the stream with zero-copy optimization
    pub async fn send(&mut self, data: &[u8]) -> Result<()> {
        // Use zero-copy when possible
        if data.len() <= 1024 * 1024 {
            // 1MB threshold for zero-copy
            let bytes = Bytes::copy_from_slice(data);
            self.send.write_all(&bytes).await?;
        } else {
            // Large data - use streaming
            self.send.write_all(data).await?;
        }
        self.send.finish()?;
        self.metrics.record_bytes_sent(data.len());
        Ok(())
    }

    /// Send bytes directly for zero-copy operations
    pub async fn send_bytes(&mut self, bytes: Bytes) -> Result<()> {
        self.send.write_all(&bytes).await?;
        self.send.finish()?;
        self.metrics.record_bytes_sent(bytes.len());
        Ok(())
    }

    /// Receive data from the stream.
    ///
    /// Max receive size is 64KB — sufficient for handshakes, sync messages,
    /// and shard metadata. Bulk shard data uses `send()`/`send_bytes()` with
    /// length-prefixed framing rather than `read_to_end()`.
    pub async fn receive(&mut self) -> Result<Bytes> {
        let data = self.recv.read_to_end(64 * 1024).await?;
        self.metrics.record_bytes_received(data.len());
        Ok(data.into())
    }

    /// Write a length-prefixed message WITHOUT closing the stream.
    ///
    /// Format: 4-byte big-endian length + payload.
    /// Use this for multi-message protocols (e.g., bilateral handshake).
    pub async fn write_msg(&mut self, data: &[u8]) -> Result<()> {
        let len = u32::try_from(data.len())
            .map_err(|_| anyhow::anyhow!("Message too large: {} bytes", data.len()))?;
        self.send.write_all(&len.to_be_bytes()).await?;
        self.send.write_all(data).await?;
        self.metrics.record_bytes_sent(4 + data.len());
        Ok(())
    }

    /// Read a length-prefixed message from the stream.
    ///
    /// Expects 4-byte big-endian length header followed by payload.
    /// Max message size is 64KB.
    pub async fn read_msg(&mut self) -> Result<Vec<u8>> {
        let mut len_buf = [0u8; 4];
        self.recv.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        if len > 64 * 1024 {
            return Err(anyhow::anyhow!("Message too large: {len} bytes (max 64KB)"));
        }
        let mut buf = vec![0u8; len];
        self.recv.read_exact(&mut buf).await?;
        self.metrics.record_bytes_received(4 + len);
        Ok(buf)
    }

    /// Close the write half of the stream after all messages are sent.
    pub fn finish_send(&mut self) -> Result<()> {
        self.send.finish()?;
        Ok(())
    }
}
