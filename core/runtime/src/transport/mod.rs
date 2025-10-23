//! QUIC Transport Integration for Container Communication
//!
//! This module integrates the HyperMesh QUIC transport layer with container networking
//! to provide secure, high-performance inter-container communication with Byzantine
//! fault tolerance and P2P mesh networking capabilities.

pub mod config;
pub mod connections;

// Re-export commonly used types
pub use self::config::{
    ContainerTransportConfig, ContainerCommConfig, P2PCommConfig, BufferConfig,
    PeerDiscoveryConfig, DHTConfig, ConnectionMigrationConfig, TransportSecurityConfig,
    CertValidationMode, MessageAuthConfig, MessageAuthMethod, ByzantineProtectionConfig,
    RateLimitConfig, TransportPerformanceConfig, CongestionControlAlgorithm,
    FlowControlConfig, FlowControlMode, PerformanceMonitoringConfig, RoutingConfig,
    RoutingStrategy, LoadBalancingConfig, LoadBalancingAlgorithm, HealthCheckConfig,
    WeightCalculationMethod, FailoverConfig, FailoverStrategy,
};

pub use self::connections::{
    ContainerConnection, PeerConnection, ConnectionState, ConnectionMetrics,
    ConnectionQuality, QualityTrend, ConnectionMetadata, QoSClass, SecurityLevel,
    ConnectionPool, ConnectionPoolConfig, ConnectionPoolStatistics,
};

use crate::{RuntimeError, Result};
use crate::networking::{NetworkManager, NetworkEvent};
use crate::health::{HealthMonitor, HealthEvent};
use nexus_transport::{QuicClient, QuicServer, TransportMessage, MessageType, StreamType};
use nexus_consensus::byzantine::ByzantineGuard;
use nexus_shared::{NodeId, ResourceId, Timestamp};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::{mpsc, RwLock as AsyncRwLock, Mutex, oneshot};
use tracing::{debug, info, warn, error, instrument};

/// Container communication manager with QUIC transport integration
#[derive(Debug)]
pub struct ContainerTransportManager {
    /// Node identifier
    node_id: NodeId,
    
    /// Configuration
    config: ContainerTransportConfig,
    
    /// Network manager reference
    network_manager: Arc<NetworkManager>,
    
    /// Health monitor reference
    health_monitor: Arc<HealthMonitor>,
    
    /// Byzantine guard for message validation
    byzantine_guard: Arc<AsyncRwLock<ByzantineGuard>>,
    
    /// QUIC server for incoming connections
    quic_server: Arc<Mutex<Option<QuicServer>>>,
    
    /// QUIC client for outgoing connections
    quic_client: Arc<QuicClient>,
    
    /// Connection pool for efficient reuse
    connection_pool: Arc<ConnectionPool>,
    
    /// Event subscribers
    event_subscribers: Arc<RwLock<Vec<mpsc::UnboundedSender<TransportEvent>>>>,
}

/// Transport events for monitoring and coordination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransportEvent {
    ConnectionEstablished {
        peer: ConnectionPeer,
        connection_id: String,
        timestamp: Timestamp,
    },
    ConnectionClosed {
        peer: ConnectionPeer,
        connection_id: String,
        reason: ConnectionCloseReason,
        timestamp: Timestamp,
    },
    MessageReceived {
        source: MessageSource,
        message_type: String,
        size_bytes: usize,
        timestamp: Timestamp,
    },
    MessageSent {
        destination: String,
        message_type: String,
        size_bytes: usize,
        timestamp: Timestamp,
    },
    ConnectionError {
        peer: ConnectionPeer,
        error_type: TransportErrorType,
        error_message: String,
        timestamp: Timestamp,
    },
    QualityChanged {
        peer: ConnectionPeer,
        old_quality: f64,
        new_quality: f64,
        timestamp: Timestamp,
    },
}

/// Connection peer identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionPeer {
    Container(ResourceId),
    Node(NodeId),
}

/// Connection close reasons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionCloseReason {
    Normal,
    Timeout,
    Error,
    Shutdown,
}

/// Message source identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageSource {
    Container(ResourceId),
    Peer(NodeId),
}

/// Transport error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransportErrorType {
    Connection,
    Authentication,
    Validation,
    Network,
    Protocol,
}

impl ContainerTransportManager {
    /// Create a new container transport manager
    pub async fn new(
        node_id: NodeId,
        config: ContainerTransportConfig,
        network_manager: Arc<NetworkManager>,
        health_monitor: Arc<HealthMonitor>,
        byzantine_guard: Arc<AsyncRwLock<ByzantineGuard>>,
    ) -> Result<Self> {
        // Create QUIC client
        // Create certificate manager for QUIC client 
        // TODO: Use proper certificate paths in production
        let cert_manager = Arc::new(nexus_transport::CertificateManager::new_self_signed(
            "HyperMesh".to_string(),  // Certificate common name
            365,                      // Days valid
            Duration::from_secs(30),  // Renewal interval
        ).await.map_err(|e| RuntimeError::Transport { message: format!("Failed to create certificate manager: {}", e) })?);
        let quic_client = Arc::new(QuicClient::new(config.quic_config.clone(), cert_manager).await.map_err(|e| RuntimeError::Transport { message: format!("Failed to create QUIC client: {}", e) })?);
        
        // Create connection pool
        let pool_config = ConnectionPoolConfig {
            max_connections_per_container: config.container_comm.max_connections_per_container,
            max_connections_per_peer: config.p2p_comm.max_peer_connections,
            idle_timeout: config.container_comm.connection_timeout,
            ..Default::default()
        };
        let connection_pool = Arc::new(ConnectionPool::new(pool_config));
        
        Ok(Self {
            node_id,
            config,
            network_manager,
            health_monitor,
            byzantine_guard,
            quic_server: Arc::new(Mutex::new(None)),
            quic_client,
            connection_pool,
            event_subscribers: Arc::new(RwLock::new(Vec::new())),
        })
    }
    
    /// Start the transport manager
    #[instrument(skip(self))]
    pub async fn start(&self) -> Result<()> {
        info!(node_id = %self.node_id, "Starting container transport manager");
        
        // TODO: Start QUIC server
        // TODO: Initialize P2P discovery
        // TODO: Start background tasks
        
        info!(node_id = %self.node_id, "Container transport manager started");
        Ok(())
    }
    
    /// Stop the transport manager
    pub async fn stop(&self) -> Result<()> {
        info!(node_id = %self.node_id, "Stopping container transport manager");
        
        // TODO: Stop QUIC server
        // TODO: Close all connections
        // TODO: Cleanup resources
        
        info!(node_id = %self.node_id, "Container transport manager stopped");
        Ok(())
    }
    
    /// Get connection pool statistics
    pub async fn get_connection_stats(&self) -> Result<ConnectionPoolStatistics> {
        self.connection_pool.get_statistics().await
    }
    
    /// Subscribe to transport events
    pub async fn subscribe_events(&self) -> Result<mpsc::UnboundedReceiver<TransportEvent>> {
        let (tx, rx) = mpsc::unbounded_channel();
        
        let mut subscribers = self.event_subscribers.write()
            .map_err(|e| RuntimeError::LockPoisoned(format!("Event subscribers: {}", e)))?;
        subscribers.push(tx);
        
        Ok(rx)
    }
}

impl Clone for ContainerTransportManager {
    fn clone(&self) -> Self {
        Self {
            node_id: self.node_id.clone(),
            config: self.config.clone(),
            network_manager: Arc::clone(&self.network_manager),
            health_monitor: Arc::clone(&self.health_monitor),
            byzantine_guard: Arc::clone(&self.byzantine_guard),
            quic_server: Arc::clone(&self.quic_server),
            quic_client: Arc::clone(&self.quic_client),
            connection_pool: Arc::clone(&self.connection_pool),
            event_subscribers: Arc::clone(&self.event_subscribers),
        }
    }
}