// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ Transport Layer Integration for P2P Distribution
//!
//! Uses the real STOQ crate (stoq::StoqTransport) for QUIC-over-IPv6 P2P
//! communication. StoqTransport manages connection pooling, FALCON crypto,
//! adaptive optimization, and bidirectional streams internally.

use anyhow::{Result, Context};
use std::sync::Arc;
use std::net::{SocketAddr, Ipv6Addr};
use tokio::sync::{RwLock, mpsc};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::assets::AssetPackageId;
use super::{DistributionConfig, PackageManager};
use super::dht::NodeId;

/// STOQ transport layer for P2P communication.
///
/// Wraps `stoq::StoqTransport` which provides QUIC-over-IPv6 with connection
/// pooling, FALCON quantum-resistant crypto, adaptive optimization, and eBPF
/// acceleration. Each connection yields `quinn::SendStream`/`quinn::RecvStream`
/// bidirectional streams that implement `AsyncWrite`/`AsyncRead`.
#[allow(dead_code)] // Transport fields used during P2P operations
pub struct StoqTransportLayer {
    /// Real STOQ transport (manages quinn endpoint, connection pools, crypto)
    transport: Arc<stoq::StoqTransport>,
    /// Active connections keyed by peer NodeId
    connections: Arc<RwLock<HashMap<NodeId, Arc<stoq::Connection>>>>,
    /// Incoming connection handler
    incoming_handler: Arc<RwLock<Option<mpsc::Sender<IncomingRequest>>>>,
    /// Transport configuration
    config: TransportLayerConfig,
    /// Connection pool for multiplexing
    connection_pool: Arc<ConnectionPool>,
    /// Bandwidth manager
    bandwidth_manager: Arc<BandwidthManager>,
}

/// Transport layer configuration
#[derive(Debug, Clone)]
pub struct TransportLayerConfig {
    /// Local bind address (IPv6)
    pub bind_addr: Ipv6Addr,
    /// Local port
    pub port: u16,
    /// Maximum concurrent connections
    pub max_connections: usize,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Maximum stream buffer size
    pub max_buffer_size: usize,
    /// Enable encryption
    pub enable_encryption: bool,
    /// Enable compression
    pub enable_compression: bool,
}

/// Incoming request from a peer
#[derive(Debug)]
pub struct IncomingRequest {
    /// Peer node ID
    pub peer_id: NodeId,
    /// Request type
    pub request_type: RequestType,
    /// Response channel
    pub response: mpsc::Sender<ResponseData>,
}

/// Request types for P2P communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequestType {
    /// Request package metadata
    GetPackageInfo(AssetPackageId),
    /// Request package chunk
    GetChunk {
        package_id: AssetPackageId,
        chunk_index: usize,
    },
    /// Request multiple chunks
    GetChunks {
        package_id: AssetPackageId,
        chunk_indices: Vec<usize>,
    },
    /// Announce package availability
    AnnouncePackage(PackageAnnouncement),
    /// Search for packages
    SearchPackages(String),
    /// Ping for liveness check
    Ping,
    /// Request peer list
    GetPeers,
}

/// Response data for P2P communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseData {
    /// Package information
    PackageInfo(PackageInfo),
    /// Package chunk data
    Chunk(ChunkData),
    /// Multiple chunks
    Chunks(Vec<ChunkData>),
    /// Search results
    SearchResults(Vec<AssetPackageId>),
    /// Peer list
    Peers(Vec<NodeId>),
    /// Acknowledgment
    Ack,
    /// Error response
    Error(String),
}

/// Package announcement for DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageAnnouncement {
    /// Package ID
    pub package_id: AssetPackageId,
    /// Package metadata
    pub metadata: PackageMetadata,
    /// Content addresses
    pub content_addresses: Vec<String>,
}

/// Package metadata for P2P sharing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Package size in bytes
    pub size: u64,
    /// Number of chunks
    pub chunk_count: usize,
    /// Chunk size in bytes
    pub chunk_size: usize,
    /// Package hash
    pub hash: String,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Package information response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    /// Package metadata
    pub metadata: PackageMetadata,
    /// Available chunk indices
    pub available_chunks: Vec<usize>,
    /// Merkle root hash
    pub merkle_root: String,
}

/// Chunk data for transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkData {
    /// Chunk index
    pub index: usize,
    /// Chunk data
    pub data: Vec<u8>,
    /// Chunk hash
    pub hash: String,
}

/// Connection pool for multiplexing (uses real stoq::Connection)
pub struct ConnectionPool {
    /// Pool of connections per peer
    pools: Arc<RwLock<HashMap<NodeId, Vec<Arc<stoq::Connection>>>>>,
    /// Maximum connections per peer
    max_per_peer: usize,
}

/// Bandwidth manager for rate limiting
pub struct BandwidthManager {
    /// Upload rate limiter
    upload_limiter: Arc<RwLock<RateLimiter>>,
    /// Download rate limiter
    download_limiter: Arc<RwLock<RateLimiter>>,
    /// Current upload rate (bytes/sec)
    current_upload_rate: Arc<std::sync::atomic::AtomicU64>,
    /// Current download rate (bytes/sec)
    current_download_rate: Arc<std::sync::atomic::AtomicU64>,
}

/// Simple token bucket rate limiter
struct RateLimiter {
    /// Maximum rate (bytes/sec)
    max_rate: u64,
    /// Available tokens
    tokens: f64,
    /// Last update time
    last_update: std::time::Instant,
}

impl StoqTransportLayer {
    /// Create a new STOQ transport layer backed by real stoq::StoqTransport.
    ///
    /// This initializes the QUIC-over-IPv6 transport with connection pooling,
    /// FALCON quantum-resistant crypto, and adaptive optimization.
    pub async fn new(dist_config: DistributionConfig) -> Result<Self> {
        // Build real STOQ TransportConfig from distribution settings
        let mut stoq_config = stoq::TransportConfig::default();
        stoq_config.bind_address = Ipv6Addr::UNSPECIFIED;
        stoq_config.port = stoq::DEFAULT_PORT;
        stoq_config.max_connections = Some(dist_config.max_concurrent_transfers as u32);
        // 0-RTT disabled by default in STOQ for security (replay attack risk)
        stoq_config.enable_0rtt = false;
        stoq_config.enable_migration = true;

        // Create real STOQ transport (manages quinn endpoint internally)
        let transport = Arc::new(
            stoq::StoqTransport::new(stoq_config)
                .await
                .context("Failed to create STOQ transport")?
        );

        // Create transport layer configuration
        let config = TransportLayerConfig {
            bind_addr: Ipv6Addr::UNSPECIFIED,
            port: stoq::DEFAULT_PORT,
            max_connections: dist_config.max_concurrent_transfers,
            connection_timeout: Duration::from_secs(30),
            max_buffer_size: 16 * 1024 * 1024, // 16MB
            enable_encryption: true,
            enable_compression: true,
        };

        // Create connection pool
        let connection_pool = Arc::new(ConnectionPool {
            pools: Arc::new(RwLock::new(HashMap::new())),
            max_per_peer: 5,
        });

        // Create bandwidth manager
        let bandwidth_manager = Arc::new(BandwidthManager::new(
            dist_config.max_upload_bandwidth,
            dist_config.max_download_bandwidth,
        ));

        Ok(Self {
            transport,
            connections: Arc::new(RwLock::new(HashMap::new())),
            incoming_handler: Arc::new(RwLock::new(None)),
            config,
            connection_pool,
            bandwidth_manager,
        })
    }

    /// Connect to a peer using real STOQ transport.
    ///
    /// Creates a stoq::Endpoint from the socket address and uses
    /// StoqTransport::connect() which handles TLS, FALCON crypto,
    /// connection pooling, and adaptive optimization internally.
    pub async fn connect(&self, peer_addr: SocketAddr) -> Result<NodeId> {
        // Build STOQ endpoint from peer address (IPv6 required)
        let ipv6_addr = match peer_addr {
            SocketAddr::V6(v6) => *v6.ip(),
            SocketAddr::V4(_) => {
                return Err(anyhow::anyhow!(
                    "STOQ requires IPv6 addresses, got IPv4: {}",
                    peer_addr
                ));
            }
        };
        let stoq_endpoint = stoq::Endpoint::new(ipv6_addr, peer_addr.port());

        // Connect using real STOQ transport (handles TLS, pooling, crypto)
        let connection = self.transport
            .connect(&stoq_endpoint)
            .await
            .context("Failed to connect to peer via STOQ")?;

        // Generate node ID from peer address
        let node_id = NodeId::from_address(&peer_addr);

        // Store connection
        {
            let mut connections = self.connections.write().await;
            connections.insert(node_id.clone(), connection.clone());
        }

        // Add to connection pool
        self.connection_pool.add_connection(node_id.clone(), connection).await?;

        Ok(node_id)
    }

    /// Send a request to a peer over a STOQ bidirectional stream.
    ///
    /// Opens a quinn bidirectional stream via stoq::Connection::open_bi(),
    /// serializes the request with bincode, sends it, then reads the response.
    /// quinn::SendStream implements AsyncWrite, quinn::RecvStream implements AsyncRead.
    pub async fn send_request(
        &self,
        peer_id: &NodeId,
        request: RequestType,
    ) -> Result<ResponseData> {
        let connection = self.get_connection(peer_id).await?;

        // Open a bidirectional stream (returns quinn::SendStream, quinn::RecvStream)
        let (mut send, mut recv) = connection
            .open_bi()
            .await
            .context("Failed to open bidirectional stream")?;

        // Apply bandwidth limiting for upload
        let request_data = bincode::serialize(&request)?;
        self.bandwidth_manager.limit_upload(request_data.len()).await?;

        // Send request (quinn::SendStream implements AsyncWrite via tokio)
        send.write_all(&request_data).await
            .context("Failed to write request data")?;
        // Signal end of request by finishing the send stream
        send.finish()
            .context("Failed to finish send stream")?;

        // Receive response (quinn::RecvStream implements AsyncRead)
        let response_data = recv
            .read_to_end(self.config.max_buffer_size)
            .await
            .context("Failed to read response data")?;

        // Apply bandwidth limiting for download
        self.bandwidth_manager.limit_download(response_data.len()).await?;

        // Deserialize response
        let response: ResponseData = bincode::deserialize(&response_data)?;

        Ok(response)
    }

    /// Listen for incoming package requests via STOQ transport.
    ///
    /// Spawns a background task that accepts incoming STOQ connections
    /// and handles package requests (metadata, chunks, etc.) on each.
    pub async fn listen_for_package_requests(
        &self,
        package_id: AssetPackageId,
        package_manager: Arc<PackageManager>,
    ) -> Result<()> {
        let transport = self.transport.clone();

        // Spawn listener task that accepts connections via StoqTransport
        tokio::spawn(async move {
            loop {
                let connection = match transport.accept().await {
                    Ok(conn) => conn,
                    Err(e) => {
                        tracing::warn!("Failed to accept STOQ connection: {}", e);
                        // Brief backoff before retrying
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                };

                // Handle each connection in its own task
                let pm = package_manager.clone();
                tokio::spawn(Self::handle_connection(
                    connection,
                    package_id,
                    pm,
                ));
            }
        });

        Ok(())
    }

    /// Handle incoming connection streams.
    ///
    /// Accepts bidirectional streams from a stoq::Connection, reads bincode
    /// requests, dispatches to handle_request, and writes bincode responses.
    /// quinn::SendStream/RecvStream provide the AsyncWrite/AsyncRead impls.
    async fn handle_connection(
        connection: Arc<stoq::Connection>,
        package_id: AssetPackageId,
        package_manager: Arc<PackageManager>,
    ) {
        loop {
            // accept_bi returns (quinn::SendStream, quinn::RecvStream)
            match connection.accept_bi().await {
                Ok((mut send, mut recv)) => {
                    // Receive request via quinn::RecvStream::read_to_end
                    let request_data = match recv.read_to_end(16 * 1024 * 1024).await {
                        Ok(data) => data,
                        Err(e) => {
                            tracing::warn!("Failed to receive request: {}", e);
                            break;
                        }
                    };

                    // Deserialize request
                    let request: RequestType = match bincode::deserialize(&request_data) {
                        Ok(req) => req,
                        Err(e) => {
                            tracing::warn!("Failed to deserialize request: {}", e);
                            continue;
                        }
                    };

                    // Handle request
                    let response = Self::handle_request(
                        request,
                        package_id,
                        package_manager.clone(),
                    ).await;

                    // Send response via quinn::SendStream (implements AsyncWrite)
                    let response_data = match bincode::serialize(&response) {
                        Ok(data) => data,
                        Err(e) => {
                            tracing::warn!("Failed to serialize response: {}", e);
                            continue;
                        }
                    };

                    if let Err(e) = send.write_all(&response_data).await {
                        tracing::warn!("Failed to send response: {}", e);
                    }
                    // Signal end of response
                    let _ = send.finish();
                }
                Err(e) => {
                    tracing::debug!("Connection closed: {}", e);
                    break;
                }
            }
        }
    }

    /// Handle a request
    async fn handle_request(
        request: RequestType,
        package_id: AssetPackageId,
        package_manager: Arc<PackageManager>,
    ) -> ResponseData {
        match request {
            RequestType::GetPackageInfo(req_package_id) => {
                if req_package_id != package_id {
                    return ResponseData::Error("Package not found".to_string());
                }

                match package_manager.get_package_info(&package_id).await {
                    Ok(info) => ResponseData::PackageInfo(info),
                    Err(e) => ResponseData::Error(e.to_string()),
                }
            }
            RequestType::GetChunk { package_id: req_id, chunk_index } => {
                if req_id != package_id {
                    return ResponseData::Error("Package not found".to_string());
                }

                match package_manager.get_chunk(&package_id, chunk_index).await {
                    Ok(chunk) => ResponseData::Chunk(chunk),
                    Err(e) => ResponseData::Error(e.to_string()),
                }
            }
            RequestType::GetChunks { package_id: req_id, chunk_indices } => {
                if req_id != package_id {
                    return ResponseData::Error("Package not found".to_string());
                }

                let mut chunks = Vec::new();
                for index in chunk_indices {
                    match package_manager.get_chunk(&package_id, index).await {
                        Ok(chunk) => chunks.push(chunk),
                        Err(e) => {
                            return ResponseData::Error(format!("Failed to get chunk {}: {}", index, e));
                        }
                    }
                }

                ResponseData::Chunks(chunks)
            }
            RequestType::Ping => ResponseData::Ack,
            _ => ResponseData::Error("Unsupported request".to_string()),
        }
    }

    /// Get connection to a peer
    async fn get_connection(&self, peer_id: &NodeId) -> Result<Arc<stoq::Connection>> {
        let connections = self.connections.read().await;
        connections
            .get(peer_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Not connected to peer {}", peer_id))
    }

    /// Disconnect from a peer by closing the STOQ connection.
    pub async fn disconnect(&self, peer_id: &NodeId) -> Result<()> {
        let mut connections = self.connections.write().await;
        if let Some(connection) = connections.remove(peer_id) {
            connection.close();
        }
        Ok(())
    }

    /// Get connected peers
    pub async fn get_connected_peers(&self) -> Vec<NodeId> {
        let connections = self.connections.read().await;
        connections.keys().cloned().collect()
    }
}

impl ConnectionPool {
    /// Add a connection to the pool
    async fn add_connection(&self, node_id: NodeId, connection: Arc<stoq::Connection>) -> Result<()> {
        let mut pools = self.pools.write().await;
        let pool = pools.entry(node_id).or_insert_with(Vec::new);

        if pool.len() < self.max_per_peer {
            pool.push(connection);
        }

        Ok(())
    }

    /// Get a connection from the pool
    #[allow(dead_code)] // Pool access method for P2P operations
    async fn get_connection(&self, node_id: &NodeId) -> Option<Arc<stoq::Connection>> {
        let pools = self.pools.read().await;
        pools.get(node_id)?.first().cloned()
    }

    /// Remove all connections for a peer
    #[allow(dead_code)] // Cleanup method for connection management
    async fn remove_peer(&self, node_id: &NodeId) {
        let mut pools = self.pools.write().await;
        pools.remove(node_id);
    }
}

impl BandwidthManager {
    /// Create a new bandwidth manager
    fn new(max_upload: Option<u64>, max_download: Option<u64>) -> Self {
        Self {
            upload_limiter: Arc::new(RwLock::new(RateLimiter::new(max_upload))),
            download_limiter: Arc::new(RwLock::new(RateLimiter::new(max_download))),
            current_upload_rate: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            current_download_rate: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Limit upload bandwidth
    async fn limit_upload(&self, bytes: usize) -> Result<()> {
        let mut limiter = self.upload_limiter.write().await;
        limiter.consume(bytes as u64).await?;

        // Update current rate
        self.current_upload_rate.fetch_add(
            bytes as u64,
            std::sync::atomic::Ordering::Relaxed,
        );

        Ok(())
    }

    /// Limit download bandwidth
    async fn limit_download(&self, bytes: usize) -> Result<()> {
        let mut limiter = self.download_limiter.write().await;
        limiter.consume(bytes as u64).await?;

        // Update current rate
        self.current_download_rate.fetch_add(
            bytes as u64,
            std::sync::atomic::Ordering::Relaxed,
        );

        Ok(())
    }

    /// Get current upload rate
    pub fn get_upload_rate(&self) -> u64 {
        self.current_upload_rate.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get current download rate
    pub fn get_download_rate(&self) -> u64 {
        self.current_download_rate.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl RateLimiter {
    /// Create a new rate limiter
    fn new(max_rate: Option<u64>) -> Self {
        Self {
            max_rate: max_rate.unwrap_or(u64::MAX),
            tokens: max_rate.unwrap_or(u64::MAX) as f64,
            last_update: std::time::Instant::now(),
        }
    }

    /// Consume tokens (with waiting if necessary)
    async fn consume(&mut self, bytes: u64) -> Result<()> {
        if self.max_rate == u64::MAX {
            return Ok(()); // No limit
        }

        // Update tokens based on elapsed time
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();
        self.tokens += elapsed * self.max_rate as f64;
        self.tokens = self.tokens.min(self.max_rate as f64);
        self.last_update = now;

        // Wait if not enough tokens
        while self.tokens < bytes as f64 {
            let needed = bytes as f64 - self.tokens;
            let wait_time = Duration::from_secs_f64(needed / self.max_rate as f64);
            tokio::time::sleep(wait_time).await;

            // Update tokens again
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(self.last_update).as_secs_f64();
            self.tokens += elapsed * self.max_rate as f64;
            self.tokens = self.tokens.min(self.max_rate as f64);
            self.last_update = now;
        }

        // Consume tokens
        self.tokens -= bytes as f64;

        Ok(())
    }
}

// Real STOQ integration: The `stoq` crate is imported at crate level via
// Cargo.toml dependency. All types (StoqTransport, Connection, Endpoint,
// TransportConfig) come from the real stoq crate.
//
// stoq::Connection::open_bi() returns (quinn::SendStream, quinn::RecvStream)
// which implement tokio::io::AsyncWrite and tokio::io::AsyncRead respectively.
// stoq::Connection::accept_bi() does the same for incoming streams.
// stoq::Connection::close() gracefully closes with code 0.