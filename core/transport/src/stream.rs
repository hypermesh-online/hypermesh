//! Stream management for QUIC connections

use crate::{Result, TransportError};
use quinn::{SendStream, RecvStream};
use tokio::io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Stream type for different use cases
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StreamType {
    /// Unidirectional stream for one-way communication
    Unidirectional,
    /// Bidirectional stream for request-response
    Bidirectional,
}

/// QUIC stream wrapper with additional functionality
pub struct QuicStream {
    stream_type: StreamType,
    send_stream: Option<Arc<Mutex<SendStream>>>,
    recv_stream: Option<Arc<Mutex<RecvStream>>>,
    stream_id: u64,
    bytes_sent: Arc<std::sync::atomic::AtomicU64>,
    bytes_received: Arc<std::sync::atomic::AtomicU64>,
}

impl QuicStream {
    /// Create a new unidirectional send stream
    pub fn new_send(send_stream: SendStream) -> Self {
        let stream_id = send_stream.id().index();
        
        Self {
            stream_type: StreamType::Unidirectional,
            send_stream: Some(Arc::new(Mutex::new(send_stream))),
            recv_stream: None,
            stream_id,
            bytes_sent: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            bytes_received: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }
    
    /// Create a new unidirectional receive stream
    pub fn new_recv(recv_stream: RecvStream) -> Self {
        let stream_id = recv_stream.id().index();
        
        Self {
            stream_type: StreamType::Unidirectional,
            send_stream: None,
            recv_stream: Some(Arc::new(Mutex::new(recv_stream))),
            stream_id,
            bytes_sent: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            bytes_received: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }
    
    /// Create a new bidirectional stream
    pub fn new_bidirectional(send_stream: SendStream, recv_stream: RecvStream) -> Self {
        let stream_id = send_stream.id().index();
        
        Self {
            stream_type: StreamType::Bidirectional,
            send_stream: Some(Arc::new(Mutex::new(send_stream))),
            recv_stream: Some(Arc::new(Mutex::new(recv_stream))),
            stream_id,
            bytes_sent: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            bytes_received: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }
    
    /// Get stream type
    pub fn stream_type(&self) -> StreamType {
        self.stream_type
    }
    
    /// Get stream ID
    pub fn id(&self) -> u64 {
        self.stream_id
    }
    
    /// Write data to the stream
    pub async fn write(&self, data: &[u8]) -> Result<()> {
        let send_stream = self.send_stream.as_ref()
            .ok_or_else(|| TransportError::Stream { 
                message: "No send stream available".to_string() 
            })?;
        
        let mut stream = send_stream.lock().await;
        stream.write_all(data).await
            .map_err(|e| TransportError::Stream { 
                message: format!("Write failed: {}", e) 
            })?;
        
        self.bytes_sent.fetch_add(
            data.len() as u64, 
            std::sync::atomic::Ordering::Relaxed
        );
        
        Ok(())
    }
    
    /// Read data from the stream
    pub async fn read(&self, buffer: &mut [u8]) -> Result<usize> {
        let recv_stream = self.recv_stream.as_ref()
            .ok_or_else(|| TransportError::Stream { 
                message: "No receive stream available".to_string() 
            })?;
        
        let mut stream = recv_stream.lock().await;
        let bytes_read = stream.read(buffer).await
            .map_err(|e| TransportError::Stream { 
                message: format!("Read failed: {}", e) 
            })?;
        
        let bytes_count = bytes_read.unwrap_or(0);
        self.bytes_received.fetch_add(
            bytes_count as u64, 
            std::sync::atomic::Ordering::Relaxed
        );
        
        Ok(bytes_count)
    }
    
    /// Read exact amount of data from the stream
    pub async fn read_exact(&self, buffer: &mut [u8]) -> Result<()> {
        let recv_stream = self.recv_stream.as_ref()
            .ok_or_else(|| TransportError::Stream { 
                message: "No receive stream available".to_string() 
            })?;
        
        let mut stream = recv_stream.lock().await;
        stream.read_exact(buffer).await
            .map_err(|e| TransportError::Stream { 
                message: format!("Read exact failed: {}", e) 
            })?;
        
        self.bytes_received.fetch_add(
            buffer.len() as u64, 
            std::sync::atomic::Ordering::Relaxed
        );
        
        Ok(())
    }
    
    /// Read all remaining data from the stream
    pub async fn read_to_end(&self, buffer: &mut Vec<u8>) -> Result<usize> {
        // TODO: Implement proper read_to_end using Quinn's API
        // For now, return 0 as a placeholder
        Ok(0)
    }
    
    /// Finish the send side of the stream
    pub async fn finish(&self) -> Result<()> {
        let send_stream = self.send_stream.as_ref()
            .ok_or_else(|| TransportError::Stream { 
                message: "No send stream available".to_string() 
            })?;
        
        let mut stream = send_stream.lock().await;
        stream.finish().await
            .map_err(|e| TransportError::Stream { 
                message: format!("Finish failed: {}", e) 
            })?;
        
        Ok(())
    }
    
    /// Reset the stream with an error code
    pub async fn reset(&self, error_code: u32) -> Result<()> {
        if let Some(send_stream) = &self.send_stream {
            let mut stream = send_stream.lock().await;
            stream.reset(error_code.into())
                .map_err(|e| TransportError::Stream { 
                    message: format!("Reset failed: {}", e) 
                })?;
        }
        
        Ok(())
    }
    
    /// Stop the receive side of the stream
    pub async fn stop(&self, error_code: u32) -> Result<()> {
        if let Some(recv_stream) = &self.recv_stream {
            let mut stream = recv_stream.lock().await;
            stream.stop(error_code.into())
                .map_err(|e| TransportError::Stream { 
                    message: format!("Stop failed: {}", e) 
                })?;
        }
        
        Ok(())
    }
    
    /// Get bytes sent on this stream
    pub fn bytes_sent(&self) -> u64 {
        self.bytes_sent.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    /// Get bytes received on this stream
    pub fn bytes_received(&self) -> u64 {
        self.bytes_received.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    /// Check if stream can send
    pub fn can_send(&self) -> bool {
        self.send_stream.is_some()
    }
    
    /// Check if stream can receive
    pub fn can_receive(&self) -> bool {
        self.recv_stream.is_some()
    }
}

impl std::fmt::Debug for QuicStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QuicStream")
            .field("stream_type", &self.stream_type)
            .field("stream_id", &self.stream_id)
            .field("bytes_sent", &self.bytes_sent.load(std::sync::atomic::Ordering::Relaxed))
            .field("bytes_received", &self.bytes_received.load(std::sync::atomic::Ordering::Relaxed))
            .field("can_send", &self.can_send())
            .field("can_receive", &self.can_receive())
            .finish()
    }
}

/// Stream manager for managing multiple streams
pub struct StreamManager {
    streams: Arc<tokio::sync::RwLock<std::collections::HashMap<u64, Arc<QuicStream>>>>,
    next_stream_id: Arc<std::sync::atomic::AtomicU64>,
}

impl StreamManager {
    /// Create a new stream manager
    pub fn new() -> Self {
        Self {
            streams: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            next_stream_id: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }
    
    /// Add a stream to management
    pub async fn add_stream(&self, stream: QuicStream) -> u64 {
        let stream_id = stream.id();
        let stream_arc = Arc::new(stream);
        
        self.streams.write().await.insert(stream_id, stream_arc);
        stream_id
    }
    
    /// Get a stream by ID
    pub async fn get_stream(&self, stream_id: u64) -> Option<Arc<QuicStream>> {
        self.streams.read().await.get(&stream_id).cloned()
    }
    
    /// Remove a stream
    pub async fn remove_stream(&self, stream_id: u64) -> Option<Arc<QuicStream>> {
        self.streams.write().await.remove(&stream_id)
    }
    
    /// Get all stream IDs
    pub async fn stream_ids(&self) -> Vec<u64> {
        self.streams.read().await.keys().cloned().collect()
    }
    
    /// Get stream count
    pub async fn stream_count(&self) -> usize {
        self.streams.read().await.len()
    }
    
    /// Get total bytes sent across all streams
    pub async fn total_bytes_sent(&self) -> u64 {
        let streams = self.streams.read().await;
        streams.values().map(|s| s.bytes_sent()).sum()
    }
    
    /// Get total bytes received across all streams
    pub async fn total_bytes_received(&self) -> u64 {
        let streams = self.streams.read().await;
        streams.values().map(|s| s.bytes_received()).sum()
    }
    
    /// Close all streams
    pub async fn close_all(&self) {
        let mut streams = self.streams.write().await;
        
        for (stream_id, stream) in streams.drain() {
            tokio::spawn(async move {
                if let Err(e) = stream.reset(0).await {
                    tracing::warn!("Failed to reset stream {}: {}", stream_id, e);
                }
            });
        }
    }
}

impl Default for StreamManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Async reader wrapper for QuicStream
pub struct StreamReader {
    stream: Arc<QuicStream>,
}

impl StreamReader {
    pub fn new(stream: Arc<QuicStream>) -> Self {
        Self { stream }
    }
}

impl AsyncRead for StreamReader {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        // This is a simplified implementation
        // A full implementation would need to properly handle the async nature
        Poll::Pending
    }
}

/// Async writer wrapper for QuicStream
pub struct StreamWriter {
    stream: Arc<QuicStream>,
}

impl StreamWriter {
    pub fn new(stream: Arc<QuicStream>) -> Self {
        Self { stream }
    }
}

impl AsyncWrite for StreamWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        // This is a simplified implementation
        Poll::Pending
    }
    
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_stream_manager_creation() {
        let manager = StreamManager::new();
        assert_eq!(manager.next_stream_id.load(std::sync::atomic::Ordering::Relaxed), 0);
    }
    
    #[tokio::test]
    async fn test_stream_manager_operations() {
        let manager = StreamManager::new();
        
        assert_eq!(manager.stream_count().await, 0);
        assert!(manager.stream_ids().await.is_empty());
        
        // Would need actual QUIC streams to test further
        // This demonstrates the API structure
    }
    
    #[test]
    fn test_stream_type() {
        assert_eq!(StreamType::Unidirectional, StreamType::Unidirectional);
        assert_ne!(StreamType::Unidirectional, StreamType::Bidirectional);
    }
}