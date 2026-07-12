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

use hypermesh_lib::{BlockchainScope, NodeSigner, PrivacyMode, StateProofProvider};
use stoq::transport::config::TransportConfig;
use stoq::transport::connection::{Connection, Endpoint};
use stoq::transport::manager::StoqTransport;

use crate::config::StoqAuthMode;
use crate::error::GatewayError;

/// Timeout for completing a bilateral PoS handshake on an incoming
/// connection in full-STOQ-PoS mode. A peer that cannot complete the
/// 3-message exchange in this window is rejected (dropped).
const POS_HANDSHAKE_TIMEOUT_SECS: u64 = 10;

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
    /// STOQ listener authentication mode (F8). Determines whether accepted
    /// connections must complete a bilateral PoS handshake before handling.
    pub auth_mode: StoqAuthMode,
    /// This gateway node's matrix coordinate, sent during the PoS handshake
    /// in full-STOQ-PoS mode. Ignored in HTTP-proxy mode.
    pub local_coordinate: (i64, i64, i64),
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
            auth_mode: StoqAuthMode::default(),
            local_coordinate: (0, 0, 0),
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
    /// STOQ listener authentication mode (F8).
    auth_mode: StoqAuthMode,
    /// This gateway node's matrix coordinate (used in the PoS handshake).
    local_coordinate: (i64, i64, i64),
    /// FALCON-1024 node identity used to answer bilateral PoS challenges.
    /// `Some` only in full-STOQ-PoS mode; `None` in HTTP-proxy mode.
    signer: Option<Arc<dyn NodeSigner>>,
    /// State-proof provider used to generate/validate four-proof state
    /// proofs during the handshake. `Some` only in full-STOQ-PoS mode.
    proof_provider: Option<Arc<dyn StateProofProvider>>,
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
    ///
    /// This constructor does NOT install a PoS identity. If
    /// `config.auth_mode` is [`StoqAuthMode::FullStoqPos`], this returns a
    /// configuration error — use [`StoqBridge::new_with_pos`] instead, which
    /// requires a signer and proof provider.
    pub async fn new(config: StoqBridgeConfig) -> Result<Self> {
        if config.auth_mode.requires_pos_handshake() {
            return Err(GatewayError::Config(
                "full-stoq-pos mode requires a NodeSigner and StateProofProvider; \
                 use StoqBridge::new_with_pos"
                    .into(),
            )
            .into());
        }
        Self::build(config, None, None).await
    }

    /// Create a STOQ bridge that enforces bilateral PoS on incoming
    /// connections (full-STOQ-PoS mode, F8).
    ///
    /// The `signer` (FALCON-1024 node identity) and `proof_provider`
    /// (four-proof state proof source) are used to answer the bilateral
    /// handshake as the responder. In this mode, [`StoqBridge::perform_pos_handshake`]
    /// must complete before any connection is handled.
    ///
    /// Returns a configuration error if `config.auth_mode` is not
    /// [`StoqAuthMode::FullStoqPos`] (a signer was supplied for a
    /// non-authenticating mode, which is almost certainly a mistake).
    pub async fn new_with_pos(
        config: StoqBridgeConfig,
        signer: Arc<dyn NodeSigner>,
        proof_provider: Arc<dyn StateProofProvider>,
    ) -> Result<Self> {
        if !config.auth_mode.requires_pos_handshake() {
            return Err(GatewayError::Config(
                "new_with_pos requires auth_mode = FullStoqPos".into(),
            )
            .into());
        }
        Self::build(config, Some(signer), Some(proof_provider)).await
    }

    /// Shared construction path: builds the transport and assembles the
    /// bridge with the given (optional) PoS identity.
    async fn build(
        config: StoqBridgeConfig,
        signer: Option<Arc<dyn NodeSigner>>,
        proof_provider: Option<Arc<dyn StateProofProvider>>,
    ) -> Result<Self> {
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
            "Initializing STOQ bridge on [{}]:{} (auth_mode={})",
            bind_ip,
            config.bind_addr.port(),
            config.auth_mode.as_str(),
        );

        let transport = StoqTransport::new(transport_config).await?;

        Ok(Self {
            transport,
            connections: Arc::new(DashMap::new()),
            privacy_mode: config.default_privacy_mode,
            blockchain_scope: config.default_blockchain_scope,
            stats: Arc::new(BridgeStats::new()),
            auth_mode: config.auth_mode,
            local_coordinate: config.local_coordinate,
            signer,
            proof_provider,
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

    /// Local socket address the bridge's STOQ transport is bound to.
    ///
    /// Useful when the bridge was created with an OS-assigned port
    /// (`[::]:0`) — for tests and for advertising the real listen address.
    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.transport.local_addr()
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

    /// Get the bridge's configured STOQ authentication mode (F8).
    pub fn auth_mode(&self) -> StoqAuthMode {
        self.auth_mode
    }

    /// Run the bilateral PoS handshake as the RESPONDER on an accepted
    /// connection (full-STOQ-PoS mode, F8).
    ///
    /// Mirrors the node-side path
    /// (`blockmatrix::network::message_handlers::peer_connection::handle_incoming_connection`):
    /// accept the initiator's stream, then run [`stoq::accept_handshake`]
    /// with this gateway's signer + proof provider. The proof provider
    /// inherits the F2 signer↔identity binding — a peer whose state-proof
    /// signer does not match its authenticated FALCON identity is rejected.
    ///
    /// Returns `Ok(())` only when the peer completes the 3-message
    /// exchange, its identity binds to its FALCON key, and its four-proof
    /// state proof validates. Any failure (missing identity, timeout, bad
    /// proof) returns `Err`, and the caller MUST drop the connection.
    ///
    /// Returns a configuration error if the bridge was not built with a
    /// signer + proof provider (i.e. not in full-STOQ-PoS mode).
    pub async fn perform_pos_handshake(&self, conn: &Arc<Connection>) -> Result<()> {
        let signer = self.signer.as_ref().ok_or_else(|| {
            GatewayError::Config("perform_pos_handshake called without a NodeSigner".into())
        })?;
        let proof_provider = self.proof_provider.as_ref().ok_or_else(|| {
            GatewayError::Config(
                "perform_pos_handshake called without a StateProofProvider".into(),
            )
        })?;

        let handshake = async {
            // Accept the bidirectional stream the initiator opened, then run
            // the responder side of the bilateral handshake.
            let mut stream = conn.accept_stream().await?;
            stoq::accept_handshake(
                &mut stream,
                signer.as_ref(),
                proof_provider.as_ref(),
                self.local_coordinate,
            )
            .await
        };

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(POS_HANDSHAKE_TIMEOUT_SECS),
            handshake,
        )
        .await;

        match result {
            Ok(Ok(hs)) => {
                let short = &hs.peer_node_id[..8.min(hs.peer_node_id.len())];
                debug!(
                    peer = short,
                    conn = %conn.id(),
                    "bilateral PoS handshake complete — peer authenticated"
                );
                Ok(())
            }
            Ok(Err(e)) => {
                self.stats.errors.fetch_add(1, Ordering::Relaxed);
                error!(conn = %conn.id(), error = %e, "PoS handshake failed — rejecting connection");
                Err(e)
            }
            Err(_) => {
                self.stats.errors.fetch_add(1, Ordering::Relaxed);
                error!(
                    conn = %conn.id(),
                    timeout_secs = POS_HANDSHAKE_TIMEOUT_SECS,
                    "PoS handshake timed out — rejecting connection"
                );
                Err(GatewayError::Config("PoS handshake timed out".into()).into())
            }
        }
    }

    /// Remove a connection from the tracking map (used when a connection is
    /// rejected after a failed PoS handshake so it is not counted as active).
    pub fn drop_connection(&self, conn_id: &str) {
        if let Some((_, conn)) = self.connections.remove(conn_id) {
            conn.close();
        }
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
            auth_mode: StoqAuthMode::FullStoqPos,
            local_coordinate: (3, 4, 5),
        };

        assert_eq!(config.bind_addr.port(), 9000);
        assert_eq!(config.max_connections, 50);
        assert_eq!(config.default_privacy_mode, PrivacyMode::PRIVATE);
        assert_eq!(config.default_blockchain_scope, BlockchainScope::Network);
        assert_eq!(config.auth_mode, StoqAuthMode::FullStoqPos);
        assert_eq!(config.local_coordinate, (3, 4, 5));
    }

    #[test]
    fn default_bridge_config_auth_mode_is_http_proxy() {
        let config = StoqBridgeConfig::default();
        assert_eq!(config.auth_mode, StoqAuthMode::HttpProxy);
        assert!(!config.auth_mode.requires_pos_handshake());
    }

    #[tokio::test]
    async fn new_rejects_full_stoq_pos_without_signer() {
        let config = StoqBridgeConfig {
            bind_addr: "[::1]:0".parse().expect("test: valid addr"),
            auth_mode: StoqAuthMode::FullStoqPos,
            ..StoqBridgeConfig::default()
        };
        // new() (no signer) must refuse to build a FullStoqPos bridge —
        // silently degrading a secure listener to passthrough is the bug F8
        // is fixing.
        assert!(StoqBridge::new(config).await.is_err());
    }
}
