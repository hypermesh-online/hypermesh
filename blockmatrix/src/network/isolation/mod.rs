// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Network Isolation Manager - Ensures zero data leakage between networks
//!
//! This module provides packet-level isolation to prevent any cross-network
//! communication. Each network operates in complete isolation with:
//! - Separate connection pools
//! - Packet origin tracking
//! - Network boundary validation
//! - Violation detection and logging

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;
use uuid::Uuid;

// Import network types from parent module
use super::trust::NetworkType;
pub use hypermesh_lib::NetworkId;

pub mod default;
#[cfg(test)]
mod tests;

pub use default::DefaultIsolationManager;

/// Packet identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PacketId(Uuid);

impl PacketId {
    /// Create new unique packet ID
    pub fn new_v4() -> Self {
        PacketId(Uuid::new_v4())
    }

    /// Get UUID representation
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl std::fmt::Display for PacketId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Hash type for packet payloads
pub type Hash = [u8; 32];

/// Timestamp for violations and events
pub type Timestamp = DateTime<Utc>;

/// Isolation Manager trait for network separation
#[async_trait]
pub trait IsolationManager: Send + Sync {
    /// Configure isolation for new network
    async fn configure_network(
        &self,
        network_id: NetworkId,
        network_type: NetworkType,
    ) -> Result<()>;

    /// Remove network isolation configuration
    async fn remove_network(&self, network_id: NetworkId) -> Result<()>;

    /// Validate packet doesn't cross network boundary
    async fn validate_packet(&self, packet: &Packet) -> Result<()>;

    /// Get isolated connection pool for network
    async fn get_connection_pool(&self, network_id: NetworkId) -> Result<Arc<ConnectionPool>>;

    /// Check for isolation violations
    async fn check_violations(&self) -> Vec<IsolationViolation>;

    /// Clear violation history
    async fn clear_violations(&self) -> Result<()>;

    /// Get isolation statistics
    async fn get_stats(&self) -> IsolationStats;
}

/// Packet metadata for isolation validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Packet {
    /// Unique packet identifier
    pub id: PacketId,
    /// Source network
    pub source_network: NetworkId,
    /// Destination network
    pub destination_network: NetworkId,
    /// Hash of packet payload for integrity
    pub payload_hash: Hash,
    /// Timestamp when packet was created
    pub timestamp: Timestamp,
}

impl Packet {
    /// Create new packet
    pub fn new(source: NetworkId, destination: NetworkId, payload_hash: Hash) -> Self {
        Packet {
            id: PacketId::new_v4(),
            source_network: source,
            destination_network: destination,
            payload_hash,
            timestamp: Utc::now(),
        }
    }

    /// Check if packet crosses network boundary
    pub fn crosses_boundary(&self) -> bool {
        self.source_network != self.destination_network
    }
}

/// Isolation violation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolationViolation {
    /// Type of violation detected
    pub violation_type: ViolationType,
    /// Source network involved
    pub source_network: NetworkId,
    /// Destination network involved
    pub destination_network: NetworkId,
    /// When violation occurred
    pub timestamp: Timestamp,
    /// Optional packet ID if related to specific packet
    pub packet_id: Option<PacketId>,
    /// Additional context about the violation
    pub details: String,
}

/// Types of isolation violations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationType {
    /// Packet attempted to cross network boundary
    CrossNetworkPacket,
    /// Unauthorized access to network assets
    UnauthorizedAssetAccess,
    /// Identity information leaked between networks
    IdentityLeakage,
    /// Connection pool sharing detected
    ConnectionPoolSharing,
    /// Configuration violation
    ConfigurationViolation,
}

impl std::fmt::Display for ViolationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ViolationType::CrossNetworkPacket => write!(f, "Cross-network packet"),
            ViolationType::UnauthorizedAssetAccess => write!(f, "Unauthorized asset access"),
            ViolationType::IdentityLeakage => write!(f, "Identity leakage"),
            ViolationType::ConnectionPoolSharing => write!(f, "Connection pool sharing"),
            ViolationType::ConfigurationViolation => write!(f, "Configuration violation"),
        }
    }
}

/// Connection pool for isolated network connections
#[derive(Debug)]
pub struct ConnectionPool {
    /// Network this pool belongs to
    pub network_id: NetworkId,
    /// Active connections in this pool
    connections: Arc<RwLock<Vec<Connection>>>,
    /// Maximum connections allowed
    max_connections: usize,
    /// Connection timeout in seconds
    _timeout_seconds: u64,
}

impl ConnectionPool {
    /// Create new connection pool for network
    pub fn new(network_id: NetworkId) -> Self {
        ConnectionPool {
            network_id,
            connections: Arc::new(RwLock::new(Vec::new())),
            max_connections: 100,
            _timeout_seconds: 300,
        }
    }

    /// Create connection pool with custom limits
    pub fn with_limits(
        network_id: NetworkId,
        max_connections: usize,
        timeout_seconds: u64,
    ) -> Self {
        ConnectionPool {
            network_id,
            connections: Arc::new(RwLock::new(Vec::new())),
            max_connections,
            _timeout_seconds: timeout_seconds,
        }
    }

    /// Add connection to pool
    pub async fn add_connection(&self, conn: Connection) -> Result<()> {
        let mut connections = self.connections.write().await;
        if connections.len() >= self.max_connections {
            return Err(anyhow!(
                "Connection pool full: max {} connections",
                self.max_connections
            ));
        }
        connections.push(conn);
        Ok(())
    }

    /// Remove connection from pool
    pub async fn remove_connection(&self, conn_id: &str) -> Result<()> {
        let mut connections = self.connections.write().await;
        connections.retain(|c| c.id != conn_id);
        Ok(())
    }

    /// Get active connection count
    pub async fn connection_count(&self) -> usize {
        self.connections.read().await.len()
    }

    /// Close all connections in pool
    pub async fn close_all(&self) -> Result<()> {
        let mut connections = self.connections.write().await;
        for conn in connections.iter() {
            conn.close().await?;
        }
        connections.clear();
        Ok(())
    }

    /// Check if pool has capacity
    pub async fn has_capacity(&self) -> bool {
        self.connections.read().await.len() < self.max_connections
    }
}

/// Individual connection in a pool
#[derive(Debug, Clone)]
pub struct Connection {
    /// Connection identifier
    pub id: String,
    /// Remote peer address
    pub remote_addr: String,
    /// Connection state
    pub state: ConnectionState,
    /// Creation timestamp
    pub created_at: Timestamp,
}

impl Connection {
    /// Create new connection
    pub fn new(remote_addr: String) -> Self {
        Connection {
            id: Uuid::new_v4().to_string(),
            remote_addr,
            state: ConnectionState::Connecting,
            created_at: Utc::now(),
        }
    }

    /// Close the connection
    pub async fn close(&self) -> Result<()> {
        // In real implementation, would close actual network connection
        debug!("Closing connection {} to {}", self.id, self.remote_addr);
        Ok(())
    }

    /// Check if connection is active
    pub fn is_active(&self) -> bool {
        matches!(self.state, ConnectionState::Active)
    }
}

/// Connection states
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    /// Connection being established
    Connecting,
    /// Connection active and ready
    Active,
    /// Connection closing
    Closing,
    /// Connection closed
    Closed,
    /// Connection failed
    Failed,
}

/// Isolation statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IsolationStats {
    /// Total packets validated
    pub packets_validated: u64,
    /// Packets rejected due to violations
    pub packets_rejected: u64,
    /// Total violations detected
    pub violations_detected: u64,
    /// Violations by type
    pub violations_by_type: HashMap<String, u64>,
    /// Active networks
    pub active_networks: usize,
    /// Total connections across all pools
    pub total_connections: usize,
}

/// Helper to create zero hash
pub fn zero_hash() -> Hash {
    [0u8; 32]
}
