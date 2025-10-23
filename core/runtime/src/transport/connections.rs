//! Connection management for HyperMesh container transport
//!
//! This module handles QUIC connections for both container-to-container and peer-to-peer
//! communication, including connection pooling, state management, and quality monitoring.

use crate::{Result, RuntimeError};
use nexus_transport::{Connection, QuicStream};
use nexus_shared::{NodeId, ResourceId, Timestamp};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock as AsyncRwLock;
use tracing::{debug, info, warn, error, instrument};

/// Container connection information
#[derive(Debug, Clone)]
pub struct ContainerConnection {
    /// Container ID
    pub container_id: ResourceId,
    
    /// Connection handle
    pub connection: Arc<Connection>,
    
    /// Connection state
    pub state: ConnectionState,
    
    /// Active streams
    pub streams: HashMap<u64, Arc<QuicStream>>,
    
    /// Connection metrics
    pub metrics: ConnectionMetrics,
    
    /// Last activity timestamp
    pub last_activity: Timestamp,
    
    /// Connection metadata
    pub metadata: ConnectionMetadata,
}

/// P2P peer connection information
#[derive(Debug, Clone)]
pub struct PeerConnection {
    /// Peer node ID
    pub peer_id: NodeId,
    
    /// Connection handle
    pub connection: Arc<Connection>,
    
    /// Peer address
    pub peer_address: SocketAddr,
    
    /// Connection state
    pub state: ConnectionState,
    
    /// Connection quality metrics
    pub quality: ConnectionQuality,
    
    /// Last activity timestamp
    pub last_activity: Timestamp,
    
    /// Reputation score
    pub reputation: f64,
}

/// Connection states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionState {
    /// Connection is being established
    Connecting,
    
    /// Connection is active and healthy
    Connected,
    
    /// Connection is degraded but functional
    Degraded,
    
    /// Connection is being migrated
    Migrating,
    
    /// Connection is being closed
    Closing,
    
    /// Connection is closed
    Closed,
    
    /// Connection failed
    Failed,
}

/// Connection metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectionMetrics {
    /// Total bytes sent
    pub bytes_sent: u64,
    
    /// Total bytes received
    pub bytes_received: u64,
    
    /// Messages sent count
    pub messages_sent: u64,
    
    /// Messages received count
    pub messages_received: u64,
    
    /// Average round-trip time
    pub avg_rtt_ms: f64,
    
    /// Current bandwidth (bytes/sec)
    pub bandwidth: u64,
    
    /// Error count
    pub error_count: u64,
    
    /// Connection uptime
    pub uptime: Duration,
    
    /// Last metrics update
    pub last_updated: Timestamp,
}

/// Connection quality assessment
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectionQuality {
    /// Latency score (0-1, higher is better)
    pub latency_score: f64,
    
    /// Throughput score (0-1, higher is better)
    pub throughput_score: f64,
    
    /// Reliability score (0-1, higher is better)
    pub reliability_score: f64,
    
    /// Overall quality score (0-1, higher is better)
    pub overall_score: f64,
    
    /// Quality trend
    pub trend: QualityTrend,
}

/// Quality trend analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum QualityTrend {
    Improving,
    #[default]
    Stable,
    Degrading,
    Unknown,
}

/// Connection metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectionMetadata {
    /// Connection tags
    pub tags: HashMap<String, String>,
    
    /// Connection priority
    pub priority: u8,
    
    /// Quality of service class
    pub qos_class: QoSClass,
    
    /// Security level
    pub security_level: SecurityLevel,
}

/// Quality of Service classes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum QoSClass {
    /// Low latency, real-time communication
    RealTime,
    
    /// Standard communication
    #[default]
    Standard,
    
    /// Bulk data transfer
    Bulk,
    
    /// Background tasks
    Background,
}

/// Security levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SecurityLevel {
    /// Minimal security (for testing)
    Minimal,
    
    /// Standard security
    #[default]
    Standard,
    
    /// High security
    High,
    
    /// Maximum security
    Maximum,
}

/// Connection pool for efficient connection reuse
#[derive(Debug)]
pub struct ConnectionPool {
    /// Pool configuration
    config: ConnectionPoolConfig,
    
    /// Available container connections
    container_connections: Arc<RwLock<HashMap<ResourceId, Vec<Arc<ContainerConnection>>>>>,
    
    /// Available peer connections
    peer_connections: Arc<RwLock<HashMap<NodeId, Vec<Arc<PeerConnection>>>>>,
    
    /// Pool statistics
    statistics: Arc<RwLock<ConnectionPoolStatistics>>,
}

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolConfig {
    /// Maximum connections per container
    pub max_connections_per_container: usize,
    
    /// Maximum connections per peer
    pub max_connections_per_peer: usize,
    
    /// Connection idle timeout
    pub idle_timeout: Duration,
    
    /// Connection reuse timeout
    pub reuse_timeout: Duration,
    
    /// Pool cleanup interval
    pub cleanup_interval: Duration,
    
    /// Enable connection validation
    pub enable_validation: bool,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_connections_per_container: 10,
            max_connections_per_peer: 5,
            idle_timeout: Duration::from_secs(300),
            reuse_timeout: Duration::from_secs(60),
            cleanup_interval: Duration::from_secs(30),
            enable_validation: true,
        }
    }
}

/// Connection pool statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectionPoolStatistics {
    /// Total connections created
    pub connections_created: u64,
    
    /// Total connections reused
    pub connections_reused: u64,
    
    /// Total connections closed
    pub connections_closed: u64,
    
    /// Current active connections
    pub active_connections: usize,
    
    /// Current idle connections
    pub idle_connections: usize,
    
    /// Connection pool hit rate
    pub hit_rate: f64,
    
    /// Average connection lifetime
    pub avg_lifetime: Duration,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(config: ConnectionPoolConfig) -> Self {
        Self {
            config,
            container_connections: Arc::new(RwLock::new(HashMap::new())),
            peer_connections: Arc::new(RwLock::new(HashMap::new())),
            statistics: Arc::new(RwLock::new(ConnectionPoolStatistics::default())),
        }
    }

    /// Get a connection to a container, creating one if necessary
    #[instrument(skip(self))]
    pub async fn get_container_connection(&self, container_id: &ResourceId) -> Result<Arc<ContainerConnection>> {
        // Try to get an existing connection from the pool
        if let Some(connection) = self.get_pooled_container_connection(container_id).await? {
            // Update statistics
            {
                let mut stats = self.statistics.write()
                    .map_err(|e| RuntimeError::LockPoisoned(format!("Pool statistics: {}", e)))?;
                stats.connections_reused += 1;
                stats.hit_rate = stats.connections_reused as f64 / (stats.connections_created + stats.connections_reused) as f64;
            }
            
            debug!(container_id = %container_id, "Reused existing connection from pool");
            return Ok(connection);
        }

        // Create a new connection
        self.create_container_connection(container_id).await
    }

    /// Get a connection to a peer, creating one if necessary
    #[instrument(skip(self))]
    pub async fn get_peer_connection(&self, peer_id: &NodeId) -> Result<Arc<PeerConnection>> {
        // Try to get an existing connection from the pool
        if let Some(connection) = self.get_pooled_peer_connection(peer_id).await? {
            // Update statistics
            {
                let mut stats = self.statistics.write()
                    .map_err(|e| RuntimeError::LockPoisoned(format!("Pool statistics: {}", e)))?;
                stats.connections_reused += 1;
                stats.hit_rate = stats.connections_reused as f64 / (stats.connections_created + stats.connections_reused) as f64;
            }
            
            debug!(peer_id = %peer_id, "Reused existing connection from pool");
            return Ok(connection);
        }

        // Create a new connection
        self.create_peer_connection(peer_id).await
    }

    /// Get an existing container connection from the pool
    async fn get_pooled_container_connection(&self, container_id: &ResourceId) -> Result<Option<Arc<ContainerConnection>>> {
        let mut connections = self.container_connections.write()
            .map_err(|e| RuntimeError::LockPoisoned(format!("Container connections: {}", e)))?;
        
        if let Some(pool) = connections.get_mut(container_id) {
            while let Some(connection) = pool.pop() {
                if self.is_connection_valid(&connection).await {
                    return Ok(Some(connection));
                } else {
                    // Connection is invalid, close it
                    self.close_container_connection(&connection).await?;
                }
            }
        }
        
        Ok(None)
    }

    /// Get an existing peer connection from the pool
    async fn get_pooled_peer_connection(&self, peer_id: &NodeId) -> Result<Option<Arc<PeerConnection>>> {
        let mut connections = self.peer_connections.write()
            .map_err(|e| RuntimeError::LockPoisoned(format!("Peer connections: {}", e)))?;
        
        if let Some(pool) = connections.get_mut(peer_id) {
            while let Some(connection) = pool.pop() {
                if self.is_peer_connection_valid(&connection).await {
                    return Ok(Some(connection));
                } else {
                    // Connection is invalid, close it
                    self.close_peer_connection(&connection).await?;
                }
            }
        }
        
        Ok(None)
    }

    /// Create a new container connection
    async fn create_container_connection(&self, container_id: &ResourceId) -> Result<Arc<ContainerConnection>> {
        // TODO: Implement actual connection creation logic
        // This is a placeholder that would integrate with the QUIC client
        
        // TODO: This is a placeholder implementation. In production, this would:
        // 1. Establish actual QUIC connection to the container 
        // 2. Use proper node IDs from the container deployment info
        // For now, return an error to indicate unimplemented
        Err(RuntimeError::Internal("Container connection creation not yet implemented".to_string()))
    }

    /// Create a new peer connection
    async fn create_peer_connection(&self, peer_id: &NodeId) -> Result<Arc<PeerConnection>> {
        // TODO: Implement actual connection creation logic
        // This is a placeholder that would integrate with the QUIC client
        Err(RuntimeError::Internal("Peer connection creation not yet implemented".to_string()))
    }

    /// Check if a container connection is valid and can be reused
    async fn is_connection_valid(&self, connection: &ContainerConnection) -> bool {
        if !self.config.enable_validation {
            return true;
        }

        // Check connection state
        if connection.state != ConnectionState::Connected {
            return false;
        }

        // Check if connection has been idle too long
        let now = SystemTime::now();
        if let Ok(elapsed) = now.duration_since(SystemTime::from(connection.last_activity)) {
            if elapsed > self.config.idle_timeout {
                return false;
            }
        }

        // TODO: Implement additional validation checks
        // - Connection health check
        // - Network connectivity test
        // - Authentication status

        true
    }

    /// Check if a peer connection is valid and can be reused
    async fn is_peer_connection_valid(&self, connection: &PeerConnection) -> bool {
        if !self.config.enable_validation {
            return true;
        }

        // Check connection state
        if connection.state != ConnectionState::Connected {
            return false;
        }

        // Check if connection has been idle too long
        let now = SystemTime::now();
        if let Ok(elapsed) = now.duration_since(SystemTime::from(connection.last_activity)) {
            if elapsed > self.config.idle_timeout {
                return false;
            }
        }

        // Check reputation score
        if connection.reputation < 0.5 {
            return false;
        }

        true
    }

    /// Return a container connection to the pool
    pub async fn return_container_connection(&self, connection: Arc<ContainerConnection>) -> Result<()> {
        let container_id = connection.container_id.clone();
        
        // Only return healthy connections to the pool
        if connection.state != ConnectionState::Connected {
            return self.close_container_connection(&connection).await;
        }

        let mut connections = self.container_connections.write()
            .map_err(|e| RuntimeError::LockPoisoned(format!("Container connections: {}", e)))?;
        
        let pool = connections.entry(container_id.clone()).or_insert_with(Vec::new);
        
        // Respect pool size limits
        if pool.len() < self.config.max_connections_per_container {
            pool.push(connection);
            
            // Update statistics
            {
                let mut stats = self.statistics.write()
                    .map_err(|e| RuntimeError::LockPoisoned(format!("Pool statistics: {}", e)))?;
                stats.idle_connections += 1;
                stats.active_connections = stats.active_connections.saturating_sub(1);
            }
            
            debug!(container_id = %container_id, "Returned connection to pool");
        } else {
            // Pool is full, close the connection
            self.close_container_connection(&connection).await?;
        }

        Ok(())
    }

    /// Return a peer connection to the pool
    pub async fn return_peer_connection(&self, connection: Arc<PeerConnection>) -> Result<()> {
        let peer_id = connection.peer_id.clone();
        
        // Only return healthy connections to the pool
        if connection.state != ConnectionState::Connected || connection.reputation < 0.7 {
            return self.close_peer_connection(&connection).await;
        }

        let mut connections = self.peer_connections.write()
            .map_err(|e| RuntimeError::LockPoisoned(format!("Peer connections: {}", e)))?;
        
        let pool = connections.entry(peer_id.clone()).or_insert_with(Vec::new);
        
        // Respect pool size limits
        if pool.len() < self.config.max_connections_per_peer {
            pool.push(connection);
            
            // Update statistics
            {
                let mut stats = self.statistics.write()
                    .map_err(|e| RuntimeError::LockPoisoned(format!("Pool statistics: {}", e)))?;
                stats.idle_connections += 1;
                stats.active_connections = stats.active_connections.saturating_sub(1);
            }
            
            debug!(peer_id = %peer_id, "Returned connection to pool");
        } else {
            // Pool is full, close the connection
            self.close_peer_connection(&connection).await?;
        }

        Ok(())
    }

    /// Close a container connection
    async fn close_container_connection(&self, connection: &ContainerConnection) -> Result<()> {
        // TODO: Implement actual connection closing logic
        // This would close the QUIC connection and clean up resources
        
        // Update statistics
        {
            let mut stats = self.statistics.write()
                .map_err(|e| RuntimeError::LockPoisoned(format!("Pool statistics: {}", e)))?;
            stats.connections_closed += 1;
            stats.active_connections = stats.active_connections.saturating_sub(1);
        }

        debug!(container_id = %connection.container_id, "Closed container connection");
        Ok(())
    }

    /// Close a peer connection
    async fn close_peer_connection(&self, connection: &PeerConnection) -> Result<()> {
        // TODO: Implement actual connection closing logic
        // This would close the QUIC connection and clean up resources
        
        // Update statistics
        {
            let mut stats = self.statistics.write()
                .map_err(|e| RuntimeError::LockPoisoned(format!("Pool statistics: {}", e)))?;
            stats.connections_closed += 1;
            stats.active_connections = stats.active_connections.saturating_sub(1);
        }

        debug!(peer_id = %connection.peer_id, "Closed peer connection");
        Ok(())
    }

    /// Get connection pool statistics
    pub async fn get_statistics(&self) -> Result<ConnectionPoolStatistics> {
        let stats = self.statistics.read()
            .map_err(|e| RuntimeError::LockPoisoned(format!("Pool statistics: {}", e)))?;
        Ok(stats.clone())
    }

    /// Perform connection pool cleanup
    #[instrument(skip(self))]
    pub async fn cleanup(&self) -> Result<()> {
        let mut cleaned_up = 0;

        // Cleanup container connections
        {
            let mut connections = self.container_connections.write()
                .map_err(|e| RuntimeError::LockPoisoned(format!("Container connections: {}", e)))?;
            
            for (container_id, pool) in connections.iter_mut() {
                pool.retain(|connection| {
                    // Remove connections that have been idle too long
                    let now = SystemTime::now();
                    if let Ok(elapsed) = now.duration_since(SystemTime::from(connection.last_activity)) {
                        if elapsed <= self.config.idle_timeout {
                            return true;
                        }
                    }
                    
                    cleaned_up += 1;
                    false
                });
            }
            
            // Remove empty pools
            connections.retain(|_, pool| !pool.is_empty());
        }

        // Cleanup peer connections
        {
            let mut connections = self.peer_connections.write()
                .map_err(|e| RuntimeError::LockPoisoned(format!("Peer connections: {}", e)))?;
            
            for (peer_id, pool) in connections.iter_mut() {
                pool.retain(|connection| {
                    // Remove connections that have been idle too long or have low reputation
                    let now = SystemTime::now();
                    if let Ok(elapsed) = now.duration_since(SystemTime::from(connection.last_activity)) {
                        if elapsed <= self.config.idle_timeout && connection.reputation >= 0.5 {
                            return true;
                        }
                    }
                    
                    cleaned_up += 1;
                    false
                });
            }
            
            // Remove empty pools
            connections.retain(|_, pool| !pool.is_empty());
        }

        // Update statistics
        {
            let mut stats = self.statistics.write()
                .map_err(|e| RuntimeError::LockPoisoned(format!("Pool statistics: {}", e)))?;
            stats.connections_closed += cleaned_up;
            stats.idle_connections = stats.idle_connections.saturating_sub(cleaned_up as usize);
        }

        if cleaned_up > 0 {
            debug!(cleaned_up = cleaned_up, "Cleaned up idle connections");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_pool_creation() {
        let config = ConnectionPoolConfig::default();
        let pool = ConnectionPool::new(config);
        
        let stats = pool.get_statistics().await.unwrap();
        assert_eq!(stats.connections_created, 0);
        assert_eq!(stats.active_connections, 0);
    }

    #[test]
    fn test_connection_state_transitions() {
        let state = ConnectionState::Connecting;
        assert_ne!(state, ConnectionState::Connected);
        
        let state = ConnectionState::Connected;
        assert_eq!(state, ConnectionState::Connected);
    }

    #[test]
    fn test_quality_trend_default() {
        let trend = QualityTrend::default();
        assert_eq!(trend, QualityTrend::Stable);
    }

    #[test]
    fn test_connection_metadata_default() {
        let metadata = ConnectionMetadata::default();
        assert_eq!(metadata.qos_class, QoSClass::Standard);
        assert_eq!(metadata.security_level, SecurityLevel::Standard);
        assert_eq!(metadata.priority, 0);
    }
}