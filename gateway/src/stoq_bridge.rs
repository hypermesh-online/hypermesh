// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ bridge for gateway operations.
//!
//! Wraps `StoqTransport` to provide gateway-specific connection management,
//! stats tracking, and lifecycle operations for STOQ protocol bridging.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::Result;
use dashmap::DashMap;
use tracing::{debug, error, info};

use hypermesh_lib::{BlockchainScope, PrivacyMode};
use stoq::transport::config::TransportConfig;
use stoq::transport::connection::{Connection, Endpoint};
use stoq::transport::manager::StoqTransport;

use crate::error::GatewayError;

/// Configuration for the STOQ bridge.
#[derive(Debug, Clone)]
pub struct StoqBridgeConfig {
    /// Socket address to bind the STOQ listener on (must be IPv6).
    pub bind_addr: SocketAddr,
    /// Maximum concurrent STOQ connections.
    pub max_connections: u32,
    /// Default privacy mode for bridge connections.
    pub default_privacy_mode: PrivacyMode,
    /// Default blockchain scope for bridge connections.
    pub default_blockchain_scope: BlockchainScope,
}

impl Default for StoqBridgeConfig {
    fn default() -> Self {
        Self {
            bind_addr: "[::]:8444"
                .parse()
                .expect("default STOQ bind addr should be valid"),
            max_connections: 100,
            default_privacy_mode: PrivacyMode::PUBLIC,
            default_blockchain_scope: BlockchainScope::Device,
        }
    }
}

/// Wraps `StoqTransport` for gateway bridge operations.
///
/// Manages the lifecycle of STOQ connections accepted or initiated by
/// the gateway, tracks bridge-level statistics, and provides cleanup
/// operations for stale connections.
pub struct StoqBridge {
    transport: StoqTransport,
    connections: Arc<DashMap<String, Arc<Connection>>>,
    privacy_mode: PrivacyMode,
    blockchain_scope: BlockchainScope,
    stats: Arc<BridgeStats>,
}

/// Atomic counters for bridge-level statistics.
struct BridgeStats {
    connections_accepted: AtomicU64,
    connections_initiated: AtomicU64,
    bytes_bridged: AtomicU64,
    errors: AtomicU64,
}

impl BridgeStats {
    fn new() -> Self {
        Self {
            connections_accepted: AtomicU64::new(0),
            connections_initiated: AtomicU64::new(0),
            bytes_bridged: AtomicU64::new(0),
            errors: AtomicU64::new(0),
        }
    }
}

/// Snapshot of bridge statistics at a point in time.
#[derive(Debug, Clone)]
pub struct BridgeStatsSnapshot {
    pub connections_accepted: u64,
    pub connections_initiated: u64,
    pub bytes_bridged: u64,
    pub errors: u64,
    pub active_connections: usize,
}

impl StoqBridge {
    /// Create a new STOQ bridge with the given configuration.
    ///
    /// Initializes the underlying `StoqTransport`, binding to the address
    /// specified in `config`. Returns an error if the address is not IPv6
    /// or if transport creation fails.
    pub async fn new(config: StoqBridgeConfig) -> Result<Self> {
        let bind_ip = match config.bind_addr {
            SocketAddr::V6(v6) => *v6.ip(),
            SocketAddr::V4(_) => {
                return Err(GatewayError::Config("STOQ requires IPv6 bind address".into()).into());
            }
        };

        let transport_config = TransportConfig {
            bind_address: bind_ip,
            port: config.bind_addr.port(),
            max_connections: Some(config.max_connections),
            ..TransportConfig::default()
        };

        info!(
            "Initializing STOQ bridge on [{}]:{}",
            bind_ip,
            config.bind_addr.port()
        );

        let transport = StoqTransport::new(transport_config).await?;

        Ok(Self {
            transport,
            connections: Arc::new(DashMap::new()),
            privacy_mode: config.default_privacy_mode,
            blockchain_scope: config.default_blockchain_scope,
            stats: Arc::new(BridgeStats::new()),
        })
    }

    /// Connect to a remote STOQ backend endpoint.
    ///
    /// Maps IPv4 addresses to IPv6-mapped addresses automatically.
    /// The connection is tracked in the bridge's connection map and
    /// returned as an `Arc<Connection>`.
    pub async fn connect_backend(
        &self,
        addr: SocketAddr,
        server_name: Option<&str>,
    ) -> Result<Arc<Connection>> {
        let ip = match addr {
            SocketAddr::V6(v6) => *v6.ip(),
            SocketAddr::V4(v4) => v4.ip().to_ipv6_mapped(),
        };

        let mut endpoint = Endpoint::new(ip, addr.port());
        if let Some(name) = server_name {
            endpoint = endpoint.with_server_name(name.to_string());
        }

        debug!("Connecting to STOQ backend [{}]:{}", ip, addr.port());

        let conn = match self.transport.connect(&endpoint).await {
            Ok(c) => c,
            Err(e) => {
                self.stats.errors.fetch_add(1, Ordering::Relaxed);
                error!("Failed to connect to STOQ backend: {}", e);
                return Err(e);
            }
        };

        self.connections.insert(conn.id(), conn.clone());
        self.stats
            .connections_initiated
            .fetch_add(1, Ordering::Relaxed);

        info!("Connected to STOQ backend [{}]:{}", ip, addr.port());
        Ok(conn)
    }

    /// Accept an incoming STOQ connection.
    ///
    /// Blocks until a new connection arrives on the bound address.
    /// The connection is tracked in the bridge's connection map.
    pub async fn accept_connection(&self) -> Result<Arc<Connection>> {
        let conn = match self.transport.accept().await {
            Ok(c) => c,
            Err(e) => {
                self.stats.errors.fetch_add(1, Ordering::Relaxed);
                return Err(e);
            }
        };

        self.connections.insert(conn.id(), conn.clone());
        self.stats
            .connections_accepted
            .fetch_add(1, Ordering::Relaxed);

        debug!("Accepted STOQ connection: {}", conn.id());
        Ok(conn)
    }

    /// Record bytes bridged through this transport (for stats tracking).
    pub fn record_bytes_bridged(&self, count: u64) {
        self.stats.bytes_bridged.fetch_add(count, Ordering::Relaxed);
    }

    /// Record an error event (for stats tracking).
    pub fn record_error(&self) {
        self.stats.errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Get underlying STOQ transport statistics.
    pub fn transport_stats(&self) -> stoq::TransportStats {
        self.transport.stats()
    }

    /// Get a snapshot of bridge-specific statistics.
    pub fn bridge_stats(&self) -> BridgeStatsSnapshot {
        BridgeStatsSnapshot {
            connections_accepted: self.stats.connections_accepted.load(Ordering::Relaxed),
            connections_initiated: self.stats.connections_initiated.load(Ordering::Relaxed),
            bytes_bridged: self.stats.bytes_bridged.load(Ordering::Relaxed),
            errors: self.stats.errors.load(Ordering::Relaxed),
            active_connections: self.connections.len(),
        }
    }

    /// Number of active (tracked) connections.
    pub fn active_connection_count(&self) -> usize {
        self.connections.len()
    }

    /// Get the bridge's configured privacy mode.
    pub fn privacy_mode(&self) -> PrivacyMode {
        self.privacy_mode
    }

    /// Get the bridge's configured blockchain scope.
    pub fn blockchain_scope(&self) -> BlockchainScope {
        self.blockchain_scope
    }

    /// Shutdown the bridge gracefully.
    ///
    /// Closes all tracked connections, clears the connection map, and
    /// shuts down the underlying STOQ transport.
    pub async fn shutdown(&self) {
        info!("Shutting down STOQ bridge");

        for entry in self.connections.iter() {
            entry.value().close();
        }
        self.connections.clear();
        self.transport.shutdown().await;

        info!("STOQ bridge shutdown complete");
    }

    /// Remove disconnected or unhealthy connections from the tracking map.
    ///
    /// Returns the number of connections that were removed.
    pub fn cleanup_stale(&self) -> usize {
        let before = self.connections.len();
        self.connections.retain(|_key, conn| conn.is_healthy());
        let removed = before - self.connections.len();

        if removed > 0 {
            debug!("Cleaned up {} stale STOQ connections", removed);
        }

        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bridge_config_has_ipv6_bind() {
        let config = StoqBridgeConfig::default();
        assert!(config.bind_addr.is_ipv6());
        assert_eq!(config.bind_addr.port(), 8444);
        assert_eq!(config.max_connections, 100);
        assert_eq!(config.default_privacy_mode, PrivacyMode::PUBLIC);
        assert_eq!(config.default_blockchain_scope, BlockchainScope::Device);
    }

    #[test]
    fn bridge_stats_snapshot_initial_values() {
        let stats = BridgeStats::new();
        assert_eq!(stats.connections_accepted.load(Ordering::Relaxed), 0);
        assert_eq!(stats.connections_initiated.load(Ordering::Relaxed), 0);
        assert_eq!(stats.bytes_bridged.load(Ordering::Relaxed), 0);
        assert_eq!(stats.errors.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn bridge_stats_snapshot_from_counters() {
        let stats = Arc::new(BridgeStats::new());
        stats.connections_accepted.store(10, Ordering::Relaxed);
        stats.connections_initiated.store(5, Ordering::Relaxed);
        stats.bytes_bridged.store(1024, Ordering::Relaxed);
        stats.errors.store(2, Ordering::Relaxed);

        let snapshot = BridgeStatsSnapshot {
            connections_accepted: stats.connections_accepted.load(Ordering::Relaxed),
            connections_initiated: stats.connections_initiated.load(Ordering::Relaxed),
            bytes_bridged: stats.bytes_bridged.load(Ordering::Relaxed),
            errors: stats.errors.load(Ordering::Relaxed),
            active_connections: 7,
        };

        assert_eq!(snapshot.connections_accepted, 10);
        assert_eq!(snapshot.connections_initiated, 5);
        assert_eq!(snapshot.bytes_bridged, 1024);
        assert_eq!(snapshot.errors, 2);
        assert_eq!(snapshot.active_connections, 7);
    }

    #[test]
    fn bridge_config_custom_values() {
        let config = StoqBridgeConfig {
            bind_addr: "[::1]:9000".parse().expect("test: valid addr"),
            max_connections: 50,
            default_privacy_mode: PrivacyMode::PRIVATE,
            default_blockchain_scope: BlockchainScope::Network,
        };

        assert_eq!(config.bind_addr.port(), 9000);
        assert_eq!(config.max_connections, 50);
        assert_eq!(config.default_privacy_mode, PrivacyMode::PRIVATE);
        assert_eq!(config.default_blockchain_scope, BlockchainScope::Network);
    }
}
