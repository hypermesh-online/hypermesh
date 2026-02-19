// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ Transport Manager - Main transport layer implementation

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use dashmap::DashMap;
use parking_lot::RwLock;
use quinn::{self, TransportConfig as QuinnTransportConfig, VarInt};
use std::collections::VecDeque;
use std::net::{SocketAddr, Ipv6Addr};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tracing::{info, debug, warn};

use super::certificates::CertificateManager;
use super::certificate_strategy::NetworkType;
use super::config::{TransportConfig, NetworkTier, CongestionControl};
use super::connection::{Connection, Endpoint, MemoryPool};
use super::metrics::TransportMetrics;
use super::stats::{ConnectionPoolStats, PerformanceStats};
use super::falcon::FalconTransport;
use super::adaptive::{AdaptiveConnection, AdaptationManager};

use crate::protocol::{StoqProtocolHandler, handshake::StoqHandshakeExtension, StoqPosIntegration};
use crate::extensions::DefaultStoqExtensions;

use super::ebpf::StoqEbpfTransport;

// Global initialization for crypto provider
static CRYPTO_INIT: std::sync::Once = std::sync::Once::new();

/// STOQ transport implementation using QUIC over IPv6
pub struct StoqTransport {
    pub(crate) config: TransportConfig,
    pub(crate) endpoint: Arc<quinn::Endpoint>,
    pub(crate) connections: Arc<DashMap<String, Arc<Connection>>>,
    pub(crate) connection_pool: Arc<DashMap<String, Vec<Arc<Connection>>>>,
    pub cert_manager: Arc<CertificateManager>,
    pub(crate) metrics: Arc<TransportMetrics>,
    pub(crate) cached_client_config: Arc<RwLock<Option<quinn::ClientConfig>>>,
    pub(crate) memory_pool: Arc<MemoryPool>,
    pub(crate) connection_multiplexer: Arc<DashMap<String, VecDeque<Arc<Connection>>>>,
    pub(crate) performance_stats: Arc<RwLock<PerformanceStats>>,
    /// FALCON quantum-resistant cryptography (optional)
    pub(crate) falcon_transport: Option<Arc<RwLock<FalconTransport>>>,
    /// STOQ protocol handler for extensions
    pub(crate) protocol_handler: Arc<StoqProtocolHandler>,
    /// STOQ handshake extension
    pub(crate) handshake_extension: Arc<StoqHandshakeExtension>,
    /// Adaptive connection optimization manager
    pub(crate) adaptation_manager: Arc<AdaptationManager>,
    /// Adaptive connections mapping
    pub(crate) adaptive_connections: Arc<DashMap<String, Arc<AdaptiveConnection>>>,
    /// eBPF transport acceleration (delegates to hypermesh-ebpf)
    pub(crate) ebpf_transport: Option<Arc<RwLock<StoqEbpfTransport>>>,
    /// Pre-created AF_XDP socket for zero-copy send/receive (created once during init)
    pub(crate) af_xdp_socket: Option<Arc<super::ebpf::AfXdpSocket>>,
    /// STOQ + PoS protocol integration
    pub(crate) pos_integration: Arc<StoqPosIntegration>,
}

impl StoqTransport {
    /// Create a new STOQ transport using QUIC over IPv6
    pub async fn new(config: TransportConfig) -> Result<Self> {
        // Initialize crypto provider once (globally)
        CRYPTO_INIT.call_once(|| {
            if let Err(e) = rustls::crypto::ring::default_provider().install_default() {
                // Provider might already be installed, log but don't fail
                debug!("Crypto provider initialization: {:?}", e);
            }
        });

        info!("Initializing STOQ transport on [{}]:{}", config.bind_address, config.port);
        info!("Transport config: zero_copy={}, pool_size={}, max_streams={}",
              config.enable_zero_copy, config.connection_pool_size, config.max_concurrent_streams);

        // Initialize certificate manager with IPv6-only production configuration
        let cert_config = if config.bind_address == Ipv6Addr::LOCALHOST {
            super::certificates::CertificateConfig::default() // Localhost testing
        } else {
            super::certificates::CertificateConfig::production(
                format!("{}-{}", "stoq-node", config.port),
                "stoq.hypermesh.online".to_string(),
                vec![config.bind_address],
            )
        };

        let cert_manager = Arc::new(CertificateManager::new(cert_config).await?);

        Self::new_with_cert_manager(config, cert_manager).await
    }

    /// Create a new STOQ transport for specific network type
    pub async fn new_for_network(config: TransportConfig, network_type: NetworkType) -> Result<Self> {
        // Initialize crypto provider once (globally)
        CRYPTO_INIT.call_once(|| {
            if let Err(e) = rustls::crypto::ring::default_provider().install_default() {
                // Provider might already be installed, log but don't fail
                debug!("Crypto provider initialization: {:?}", e);
            }
        });

        info!("Initializing STOQ transport for network type: {:?}", network_type);
        info!("Transport config: zero_copy={}, pool_size={}, max_streams={}",
              config.enable_zero_copy, config.connection_pool_size, config.max_concurrent_streams);

        // Create network-aware certificate configuration
        let cert_config = super::certificates::CertificateConfig::with_network_type(
            format!("{}-{}", "stoq-node", config.port),
            "stoq.hypermesh.online".to_string(),
            vec![config.bind_address],
            network_type,
        );

        let cert_manager = Arc::new(CertificateManager::new(cert_config).await?);

        Self::new_with_cert_manager(config, cert_manager).await
    }

    /// Internal: Create transport with provided certificate manager
    async fn new_with_cert_manager(config: TransportConfig, cert_manager: Arc<CertificateManager>) -> Result<Self> {

        // Configure QUIC transport for adaptive network tiers performance
        let mut server_transport_config = QuinnTransportConfig::default();
        server_transport_config.max_concurrent_bidi_streams(config.max_concurrent_streams.into());
        server_transport_config.max_concurrent_uni_streams(config.max_concurrent_streams.into());
        server_transport_config.max_idle_timeout(Some(config.max_idle_timeout.try_into()?));

        // QUIC performance optimizations
        server_transport_config.send_window(config.send_buffer_size as u64);
        server_transport_config.receive_window(VarInt::try_from(config.receive_buffer_size as u64).unwrap_or(VarInt::MAX));
        server_transport_config.datagram_receive_buffer_size(Some(config.max_datagram_size));
        server_transport_config.datagram_send_buffer_size(config.max_datagram_size);

        // Create client transport config
        let mut client_transport_config = QuinnTransportConfig::default();
        client_transport_config.max_concurrent_bidi_streams(config.max_concurrent_streams.into());
        client_transport_config.max_concurrent_uni_streams(config.max_concurrent_streams.into());
        client_transport_config.max_idle_timeout(Some(config.max_idle_timeout.try_into()?));
        client_transport_config.send_window(config.send_buffer_size as u64);
        client_transport_config.receive_window(VarInt::try_from(config.receive_buffer_size as u64).unwrap_or(VarInt::MAX));
        client_transport_config.datagram_receive_buffer_size(Some(config.max_datagram_size));
        client_transport_config.datagram_send_buffer_size(config.max_datagram_size);

        // Advanced congestion control for high performance
        match config.congestion_control {
            CongestionControl::Bbr2 => {
                // BBR v2 would be configured here when available in Quinn
                debug!("Using BBR-optimized settings for high performance");
            }
            CongestionControl::Cubic => {
                debug!("Using CUBIC congestion control");
            }
            CongestionControl::NewReno => {
                debug!("Using NewReno congestion control");
            }
        }

        // Create server configuration with TLS
        let rustls_server_config = cert_manager.server_crypto_config().await?;
        let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(
            quinn::crypto::rustls::QuicServerConfig::try_from(rustls_server_config)?
        ));
        server_config.transport_config(Arc::new(server_transport_config));

        // Create client configuration with TLS and cache it for performance
        let rustls_client_config = cert_manager.client_crypto_config().await?;
        let mut client_config = quinn::ClientConfig::new(Arc::new(
            quinn::crypto::rustls::QuicClientConfig::try_from(rustls_client_config)?
        ));
        client_config.transport_config(Arc::new(client_transport_config));

        // Bind to IPv6 address ONLY - enforce IPv6-only networking
        // Use port 0 (OS-assigned random port) for testing to avoid port binding conflicts
        #[cfg(test)]
        let bind_port = 0;
        #[cfg(not(test))]
        let bind_port = config.port;

        let socket_addr = SocketAddr::from((config.bind_address, bind_port));

        // Verify we're binding to IPv6
        if !socket_addr.is_ipv6() {
            return Err(anyhow!("STOQ only supports IPv6 addresses, got: {}", socket_addr));
        }

        let socket = std::net::UdpSocket::bind(socket_addr)?;

        // Set socket options for adaptive network tiers performance
        let socket = if let SocketAddr::V6(_) = socket_addr {
            let socket2_sock = socket2::Socket::from(socket);

            // Enable SO_REUSEADDR to allow quick rebinding in tests
            if let Err(e) = socket2_sock.set_reuse_address(true) {
                warn!("Could not set SO_REUSEADDR (continuing anyway): {}", e);
            }

            // IPv6-only flag
            if let Err(e) = socket2_sock.set_only_v6(true) {
                warn!("Could not set IPv6-only socket option (continuing anyway): {}", e);
            }

            // Socket optimizations
            if let Err(e) = socket2_sock.set_send_buffer_size(config.send_buffer_size) {
                warn!("Could not set send buffer size: {}", e);
            }
            if let Err(e) = socket2_sock.set_recv_buffer_size(config.receive_buffer_size) {
                warn!("Could not set receive buffer size: {}", e);
            }

            socket2_sock.into()
        } else {
            socket
        };

        let mut endpoint = quinn::Endpoint::new(
            quinn::EndpointConfig::default(),
            Some(server_config),
            socket,
            Arc::new(quinn::TokioRuntime),
        )?;

        endpoint.set_default_client_config(client_config.clone());

        // Initialize metrics and transport optimizations
        let metrics = Arc::new(TransportMetrics::new());

        // Initialize memory pool for zero-copy operations
        let memory_pool = Arc::new(MemoryPool::new(
            config.max_datagram_size,
            config.memory_pool_size,
        ));

        // Initialize FALCON quantum-resistant cryptography if enabled
        let falcon_transport = if config.enable_falcon_crypto {
            let mut falcon = FalconTransport::new(config.falcon_variant);
            if let Err(e) = falcon.generate_local_keypair() {
                warn!("Failed to generate FALCON keypair: {}", e);
                None
            } else {
                info!("FALCON quantum-resistant cryptography enabled with {:?}", config.falcon_variant);
                Some(Arc::new(RwLock::new(falcon)))
            }
        } else {
            info!("FALCON cryptography disabled");
            None
        };

        // Initialize protocol extensions
        let extensions = Arc::new(DefaultStoqExtensions::with_metrics(metrics.clone()));

        // Create protocol handler
        let protocol_handler = Arc::new(StoqProtocolHandler::new(
            extensions.clone(),
            falcon_transport.clone(),
            config.max_datagram_size,
        ));

        // Create handshake extension
        let handshake_extension = Arc::new(StoqHandshakeExtension::new(
            falcon_transport.clone(),
            false, // Don't require FALCON (backwards compatibility)
            config.enable_falcon_crypto, // Use hybrid mode if FALCON enabled
        ));

        // Create adaptation manager with 1 second interval
        let adaptation_manager = Arc::new(AdaptationManager::new(Duration::from_secs(1)));

        // Create STOQ + PoS integration with 5-minute cache TTL
        let pos_integration = Arc::new(StoqPosIntegration::new(Duration::from_secs(300)));

        // Initialize eBPF transport acceleration (delegates to hypermesh-ebpf)
        let (ebpf_transport, af_xdp_socket) = match StoqEbpfTransport::new() {
            Ok(mut ebpf) => {
                if ebpf.is_available() {
                    info!("eBPF transport acceleration available");

                    // Try to attach XDP to loopback for testing
                    if config.bind_address == Ipv6Addr::LOCALHOST {
                        if let Err(e) = ebpf.attach_xdp("lo") {
                            warn!("Failed to attach XDP to loopback: {}", e);
                        }
                    }

                    // Create a single AF_XDP socket during init and reuse it.
                    // This avoids the "duplicate socket key" error that occurs
                    // when create_af_xdp_socket is called on every send().
                    let socket = match ebpf.create_af_xdp_socket("lo", 0) {
                        Ok(s) => {
                            info!("AF_XDP zero-copy socket created for lo:0");
                            Some(Arc::new(s))
                        }
                        Err(e) => {
                            debug!("AF_XDP socket not available (will use standard I/O): {}", e);
                            None
                        }
                    };

                    (Some(Arc::new(RwLock::new(ebpf))), socket)
                } else {
                    info!("eBPF not available, using standard transport");
                    (None, None)
                }
            }
            Err(e) => {
                warn!("Failed to initialize eBPF: {}", e);
                (None, None)
            }
        };

        Ok(Self {
            config,
            endpoint: Arc::new(endpoint),
            connections: Arc::new(DashMap::new()),
            connection_pool: Arc::new(DashMap::new()),
            cert_manager,
            metrics,
            cached_client_config: Arc::new(RwLock::new(Some(client_config))),
            memory_pool,
            connection_multiplexer: Arc::new(DashMap::new()),
            performance_stats: Arc::new(RwLock::new(PerformanceStats::default())),
            falcon_transport,
            protocol_handler,
            handshake_extension,
            adaptation_manager,
            adaptive_connections: Arc::new(DashMap::new()),
            ebpf_transport,
            af_xdp_socket,
            pos_integration,
        })
    }

    /// Connect to a remote endpoint with connection pooling for performance
    pub async fn connect(&self, endpoint: &Endpoint) -> Result<Arc<Connection>> {
        let pool_key = format!("{}:{}", endpoint.address, endpoint.port);

        // Try to reuse existing connection from pool for maximum performance
        if let Some(mut pool) = self.connection_pool.get_mut(&pool_key) {
            // Clean up unhealthy connections first
            pool.retain(|conn| conn.is_healthy());

            // Try to get a healthy connection
            if let Some(pooled_conn) = pool.pop() {
                debug!("Reusing pooled connection to [{}]:{}", endpoint.address, endpoint.port);
                pooled_conn.update_activity(); // Mark as recently used
                self.performance_stats.read().record_connection_reuse();
                return Ok(pooled_conn);
            }
        }

        debug!("Creating new connection to [{}]:{}", endpoint.address, endpoint.port);

        let socket_addr = endpoint.to_socket_addr();
        let connecting = self.endpoint.connect(socket_addr, endpoint.server_name.as_deref().unwrap_or("localhost"))?;

        let quinn_conn = connecting.await?;

        let quinn_conn_arc = Arc::new(quinn_conn);

        let connection = Arc::new(Connection::new_optimized(
            quinn_conn_arc.as_ref().clone(),
            endpoint.clone(),
            self.metrics.clone(),
            self.memory_pool.clone(),
            self.config.frame_batch_size,
            self.config.connection_idle_timeout,
        ));

        let conn_id = connection.id();
        self.connections.insert(conn_id.clone(), connection.clone());

        // Register connection with adaptation manager
        self.adaptation_manager.register_connection(conn_id.clone(), quinn_conn_arc.clone());

        // Create and store adaptive connection wrapper
        let adaptive_conn = Arc::new(AdaptiveConnection::new(quinn_conn_arc));
        self.adaptive_connections.insert(conn_id, adaptive_conn);

        self.metrics.record_connection_established();

        info!("Connected to {} with adaptive optimization (pool_size={})", socket_addr, self.config.connection_pool_size);
        Ok(connection)
    }

    /// Return connection to pool for reuse with LRU eviction
    pub fn return_to_pool(&self, connection: Arc<Connection>) {
        if !connection.is_active() {
            return; // Don't pool inactive connections
        }

        let pool_key = format!("{}:{}", connection.endpoint().address, connection.endpoint().port);
        let mut pool = self.connection_pool.entry(pool_key).or_insert_with(Vec::new);

        // Update activity before returning to pool
        connection.update_activity();

        if pool.len() >= self.config.connection_pool_size {
            // Pool is full, need to evict LRU connection
            // Find the least recently used connection
            let mut lru_idx = 0;
            let mut oldest_time = u64::MAX;

            for (idx, conn) in pool.iter().enumerate() {
                let activity = conn.last_activity();
                if activity < oldest_time {
                    oldest_time = activity;
                    lru_idx = idx;
                }
            }

            // Remove the LRU connection
            pool.remove(lru_idx);
            self.performance_stats.read().record_pool_eviction();
        }

        // Add the new connection
        pool.push(connection);
    }

    /// Clean up unhealthy connections from all pools
    pub fn cleanup_unhealthy_connections(&self) {
        let mut total_removed = 0;
        let mut total_remaining = 0;

        // Track that we're doing a health check
        self.performance_stats.read().record_health_check();

        for mut entry in self.connection_pool.iter_mut() {
            let pool_key = entry.key().clone();
            let pool = entry.value_mut();

            // Remove unhealthy connections
            let initial_size = pool.len();
            pool.retain(|conn| conn.is_healthy());
            let removed = initial_size - pool.len();

            if removed > 0 {
                debug!("Removed {} unhealthy connections from pool {}", removed, pool_key);
                total_removed += removed;
            }
            total_remaining += pool.len();
        }

        if total_removed > 0 {
            info!("Health check removed {} unhealthy connections, {} remaining in pools",
                  total_removed, total_remaining);
            self.performance_stats.read().record_unhealthy_removed(total_removed);
        }
    }

    /// Get FALCON transport for quantum-resistant operations
    pub fn falcon_transport(&self) -> Option<Arc<RwLock<FalconTransport>>> {
        self.falcon_transport.clone()
    }

    /// Sign data using FALCON quantum-resistant cryptography
    pub fn falcon_sign(&self, data: &[u8]) -> Result<Option<super::falcon::FalconSignature>> {
        if let Some(falcon) = &self.falcon_transport {
            let falcon_guard = falcon.read();
            Ok(Some(falcon_guard.sign_handshake_data(data)?))
        } else {
            Ok(None)
        }
    }

    /// Verify FALCON signature
    pub fn falcon_verify(&self, key_id: &str, signature: &super::falcon::FalconSignature, data: &[u8]) -> Result<bool> {
        if let Some(falcon) = &self.falcon_transport {
            let falcon_guard = falcon.read();
            falcon_guard.verify_handshake_signature(key_id, signature, data)
        } else {
            Err(anyhow!("FALCON transport not enabled"))
        }
    }

    /// Accept incoming connections
    pub async fn accept(&self) -> Result<Arc<Connection>> {
        let incoming = self.endpoint.accept().await.ok_or_else(|| anyhow!("No incoming connection"))?;
        let quinn_conn = incoming.await?;

        let remote_addr = quinn_conn.remote_address();
        let endpoint = Endpoint::new(
            match remote_addr {
                SocketAddr::V6(addr) => *addr.ip(),
                SocketAddr::V4(_) => return Err(anyhow!("IPv4 connections are not supported - STOQ is IPv6-only")),
            },
            remote_addr.port(),
        );

        let connection = Arc::new(Connection::new_optimized(
            quinn_conn,
            endpoint,
            self.metrics.clone(),
            self.memory_pool.clone(),
            self.config.frame_batch_size,
            self.config.connection_idle_timeout,
        ));

        self.connections.insert(connection.id(), connection.clone());
        self.metrics.record_connection_established();

        info!("Accepted connection from {}", remote_addr);
        Ok(connection)
    }

    /// Get transport statistics with performance metrics
    pub fn stats(&self) -> crate::TransportStats {
        let base_stats = self.metrics.get_stats(self.connections.len());

        // Add performance metrics
        let perf_stats = self.performance_stats.read();
        let (pool_available, pool_allocated) = self.memory_pool.stats();

        info!("Performance: {:.1} Gbps peak, Zero-copy ops: {}, Pool hits/misses: {}/{}, Frame batches: {}",
              perf_stats.peak_throughput(),
              perf_stats.zero_copy_operations.load(Ordering::Relaxed),
              perf_stats.memory_pool_hits.load(Ordering::Relaxed),
              perf_stats.memory_pool_misses.load(Ordering::Relaxed),
              perf_stats.frame_batches_sent.load(Ordering::Relaxed));

        info!("Memory Pool Stats: Available buffers: {}, Allocated: {}", pool_available, pool_allocated);

        base_stats
    }

    /// Get active connections count
    pub fn active_connections(&self) -> usize {
        self.connections.len()
    }

    /// Close all connections and connection pools
    pub async fn shutdown(&self) {
        info!("Shutting down STOQ transport");

        // Close all active connections
        for conn in self.connections.iter() {
            conn.close();
        }
        self.connections.clear();

        // Clear connection pools
        self.connection_pool.clear();

        // Close endpoint
        self.endpoint.close(0u32.into(), b"shutdown");

        info!("STOQ transport shutdown complete");
    }

    /// Get connection pool statistics for monitoring
    pub fn pool_stats(&self) -> ConnectionPoolStats {
        let mut total_connections = 0;
        let mut total_healthy = 0;
        let mut pool_details = Vec::new();

        for entry in self.connection_pool.iter() {
            let pool_key = entry.key().clone();
            let pool = entry.value();
            let pool_size = pool.len();
            let healthy_count = pool.iter().filter(|conn| conn.is_healthy()).count();

            total_connections += pool_size;
            total_healthy += healthy_count;
            pool_details.push((pool_key, pool_size, healthy_count));
        }

        let perf_stats = self.performance_stats.read();

        ConnectionPoolStats {
            total_connections,
            total_healthy,
            pool_details,
            reuse_count: perf_stats.connection_reuse_count.load(Ordering::Relaxed),
            eviction_count: perf_stats.connection_pool_evictions.load(Ordering::Relaxed),
            health_check_count: perf_stats.connection_health_checks.load(Ordering::Relaxed),
            unhealthy_removed: perf_stats.unhealthy_connections_removed.load(Ordering::Relaxed),
        }
    }

    /// Get transport performance statistics
    pub fn performance_stats(&self) -> (f64, u64, u64, u64) {
        let stats = self.performance_stats.read();
        let peak_gbps = stats.peak_throughput();
        let zero_copy_ops = stats.zero_copy_operations.load(Ordering::Relaxed);
        let pool_hits = stats.memory_pool_hits.load(Ordering::Relaxed);
        let frame_batches = stats.frame_batches_sent.load(Ordering::Relaxed);

        (peak_gbps, zero_copy_ops, pool_hits, frame_batches)
    }

    /// Get detailed protocol metrics for monitoring
    pub fn get_protocol_metrics(&self) -> super::metrics::ProtocolMetrics {
        self.metrics.get_protocol_metrics()
    }

    /// Get interval-based metrics for rate calculations
    pub fn get_interval_metrics(&self) -> super::metrics::IntervalMetrics {
        self.metrics.get_interval_metrics()
    }

    /// Reset interval metrics for periodic reporting
    pub fn reset_interval_metrics(&self) {
        self.metrics.reset_interval_metrics();
    }

    /// Adapt transport configuration for detected network tier
    pub fn adapt_config_for_tier(&mut self, gbps: f64) {
        let tier = NetworkTier::from_gbps(gbps);
        self.config.adapt_for_network_tier(&tier);
        info!("Adapted STOQ configuration for network tier: {:?}", tier);
    }

    /// Get local address of the endpoint
    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.endpoint.local_addr()
    }

    /// Get the protocol handler
    pub fn protocol_handler(&self) -> &StoqProtocolHandler {
        &self.protocol_handler
    }

    /// Get access to the underlying Quinn endpoint for HTTP/3 integration
    /// This allows using STOQ's transport layer with h3 protocol implementations
    pub fn quinn_endpoint(&self) -> Arc<quinn::Endpoint> {
        self.endpoint.clone()
    }

    /// Get STOQ + PoS integration instance
    pub fn pos_integration(&self) -> &Arc<StoqPosIntegration> {
        &self.pos_integration
    }

    /// Validate connection with PoS token (for public networks)
    pub async fn validate_connection_with_pos(
        &self,
        connection_id: String,
        network_type: &NetworkType,
        pos_token: Option<&crate::protocol::PosToken>,
    ) -> Result<bool> {
        self.pos_integration.validate_connection(connection_id, network_type, pos_token).await
    }

    /// Register shard address for matrix-aware distribution
    pub fn register_shard_address(
        &self,
        shard_id: u32,
        position: crate::protocol::MatrixPosition,
        network_id: String,
        node_id: Option<String>,
    ) {
        self.pos_integration.register_shard_address(shard_id, position, network_id, node_id);
    }

    /// Get shard addresses for retrieval
    pub fn get_shard_addresses(&self, shard_ids: &[u32]) -> Vec<crate::protocol::ShardAddress> {
        self.pos_integration.get_shard_addresses(shard_ids)
    }

    /// Calculate optimal shard positions using matrix topology
    pub fn calculate_shard_positions(
        &self,
        num_shards: usize,
        origin: crate::protocol::MatrixPosition,
        min_distance: f64,
        max_distance: f64,
    ) -> Vec<crate::protocol::MatrixPosition> {
        self.pos_integration.calculate_shard_positions(num_shards, origin, min_distance, max_distance)
    }

    /// Validate asset hash at protocol level
    pub fn validate_asset_hash(
        &self,
        connection_id: &str,
        asset_id: &[u8],
        content_hash: &[u8; 32],
        data: &[u8],
    ) -> Result<bool> {
        self.pos_integration.validate_asset_hash(connection_id, asset_id, content_hash, data)
    }

    /// Get STOQ + PoS integration statistics
    pub fn get_pos_integration_stats(&self) -> crate::protocol::IntegrationStats {
        self.pos_integration.get_stats()
    }

    /// Cleanup expired connections and assets (call periodically)
    pub fn cleanup_expired(&self) {
        self.pos_integration.cleanup_expired_connections();
        self.pos_integration.cleanup_expired_assets(Duration::from_secs(3600)); // 1 hour TTL
    }
}

#[async_trait]
impl crate::Transport for StoqTransport {
    async fn connect(&self, endpoint: &Endpoint) -> Result<Connection> {
        Ok((*self.connect(endpoint).await?).clone())
    }

    async fn accept(&self) -> Result<Connection> {
        Ok((*self.accept().await?).clone())
    }

    fn stats(&self) -> crate::TransportStats {
        self.stats()
    }

    async fn shutdown(&self) {
        self.shutdown().await
    }
}

impl Clone for StoqTransport {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            endpoint: self.endpoint.clone(),
            connections: self.connections.clone(),
            connection_pool: self.connection_pool.clone(),
            cert_manager: self.cert_manager.clone(),
            metrics: self.metrics.clone(),
            cached_client_config: self.cached_client_config.clone(),
            memory_pool: self.memory_pool.clone(),
            connection_multiplexer: self.connection_multiplexer.clone(),
            performance_stats: self.performance_stats.clone(),
            falcon_transport: self.falcon_transport.clone(),
            protocol_handler: self.protocol_handler.clone(),
            handshake_extension: self.handshake_extension.clone(),
            adaptation_manager: self.adaptation_manager.clone(),
            adaptive_connections: self.adaptive_connections.clone(),
            ebpf_transport: self.ebpf_transport.clone(),
            af_xdp_socket: self.af_xdp_socket.clone(),
            pos_integration: self.pos_integration.clone(),
        }
    }
}
