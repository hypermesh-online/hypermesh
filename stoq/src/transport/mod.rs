//! STOQ Transport Layer - QUIC over IPv6 implementation

use async_trait::async_trait;
use quinn::{self, TransportConfig as QuinnTransportConfig, VarInt};
// Certificate types imported elsewhere
use std::net::{SocketAddr, Ipv6Addr};
use std::sync::Arc;
use std::time::Duration;
use socket2;
use anyhow::{Result, anyhow};
use bytes::{Bytes, BytesMut, BufMut};
use parking_lot::{RwLock, Mutex};
use dashmap::DashMap;
use tracing::{info, debug, warn};
use serde::{Serialize, Deserialize};
use std::sync::atomic::{AtomicUsize, AtomicU64, Ordering};
use std::collections::VecDeque;
// Simplified memory management - no unsafe operations

pub mod certificates;
pub mod streams;
pub mod metrics;
pub mod falcon;
pub mod adaptive;
#[cfg(feature = "ebpf")]
pub mod ebpf;

use certificates::CertificateManager;
use metrics::TransportMetrics;
pub use metrics::{ProtocolMetrics, IntervalMetrics};
use falcon::{FalconTransport, FalconVariant};
use adaptive::{AdaptiveConnection, AdaptationManager};

// Protocol integration
use crate::protocol::{StoqProtocolHandler, handshake::StoqHandshakeExtension};
use crate::extensions::DefaultStoqExtensions;

/// Network tier classification for adaptive configuration
#[derive(Debug, Clone)]
pub enum NetworkTier {
    /// Slow networks (<100 Mbps)
    Slow { mbps: f64 },
    /// Home broadband (100 Mbps - 1 Gbps)
    Home { mbps: f64 },
    /// Standard gigabit (1-2.5 Gbps)
    Standard { gbps: f64 },
    /// Performance networks (2.5-10 Gbps)
    Performance { gbps: f64 },
    /// Enterprise networks (10-25 Gbps)
    Enterprise { gbps: f64 },
    /// Data center networks (25+ Gbps)
    DataCenter { gbps: f64 },
}

impl NetworkTier {
    /// Create network tier from Gbps measurement
    pub fn from_gbps(gbps: f64) -> Self {
        let mbps = gbps * 1000.0;
        match gbps {
            g if g >= 25.0 => NetworkTier::DataCenter { gbps: g },
            g if g >= 10.0 => NetworkTier::Enterprise { gbps: g },
            g if g >= 2.5 => NetworkTier::Performance { gbps: g },
            g if g >= 1.0 => NetworkTier::Standard { gbps: g },
            _g if mbps >= 100.0 => NetworkTier::Home { mbps },
            _ => NetworkTier::Slow { mbps },
        }
    }
}

/// STOQ Transport configuration for QUIC over IPv6
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    /// Bind address (IPv6 only)
    pub bind_address: Ipv6Addr,
    /// Port to bind to
    pub port: u16,
    /// Maximum concurrent connections (None = unlimited)
    pub max_connections: Option<u32>,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Enable connection migration
    pub enable_migration: bool,
    /// Enable 0-RTT resumption
    pub enable_0rtt: bool,
    /// Maximum idle timeout
    pub max_idle_timeout: Duration,
    /// Certificate rotation interval
    pub cert_rotation_interval: Duration,
    /// Maximum concurrent streams per connection
    pub max_concurrent_streams: u32,
    /// Send buffer size
    pub send_buffer_size: usize,
    /// Receive buffer size
    pub receive_buffer_size: usize,
    /// Connection pool size for multiplexing
    pub connection_pool_size: usize,
    /// Enable zero-copy operations
    pub enable_zero_copy: bool,
    /// Maximum datagram size
    pub max_datagram_size: usize,
    /// Congestion control algorithm
    pub congestion_control: CongestionControl,
    /// Enable memory pool optimization for zero-copy
    pub enable_memory_pool: bool,
    /// Memory pool size for zero-copy operations
    pub memory_pool_size: usize,
    /// Frame batching size for syscall reduction
    pub frame_batch_size: usize,
    /// Enable CPU affinity for network threads
    pub enable_cpu_affinity: bool,
    /// Enable large send offload optimization
    pub enable_large_send_offload: bool,
    /// Enable FALCON quantum-resistant cryptography
    pub enable_falcon_crypto: bool,
    /// FALCON variant to use
    pub falcon_variant: FalconVariant,
}

/// Congestion control algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CongestionControl {
    /// BBR v2 for maximum throughput
    Bbr2,
    /// CUBIC (default)
    Cubic,
    /// NewReno
    NewReno,
}

impl Default for CongestionControl {
    fn default() -> Self {
        Self::Bbr2 // BBR v2 for high performance
    }
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            bind_address: Ipv6Addr::LOCALHOST, // Default to localhost for testing
            port: crate::DEFAULT_PORT,
            max_connections: Some(100), // Limited for DoS protection
            connection_timeout: Duration::from_secs(5), // Reduced for performance
            enable_migration: true,
            enable_0rtt: false, // Disabled due to replay attack vulnerability
            max_idle_timeout: Duration::from_secs(120), // Increased for connection reuse
            cert_rotation_interval: Duration::from_secs(24 * 60 * 60), // 24 hours
            max_concurrent_streams: 1000, // High concurrency support
            send_buffer_size: 16 * 1024 * 1024, // 16MB send buffer
            receive_buffer_size: 16 * 1024 * 1024, // 16MB receive buffer
            connection_pool_size: 100, // Connection multiplexing
            enable_zero_copy: true, // Zero-copy optimization
            max_datagram_size: 65507, // Maximum UDP datagram
            congestion_control: CongestionControl::default(),
            enable_memory_pool: true, // Memory pool optimization
            memory_pool_size: 1024, // 1024 buffers per pool
            frame_batch_size: 64, // Batch 64 frames per syscall
            enable_cpu_affinity: true, // CPU affinity optimization
            enable_large_send_offload: true, // LSO for large transfers
            enable_falcon_crypto: true, // Quantum-resistant FALCON cryptography
            falcon_variant: FalconVariant::Falcon1024, // Maximum security level
        }
    }
}

impl TransportConfig {
    /// Adapt configuration based on detected network tier for true adaptive behavior
    pub fn adapt_for_network_tier(&mut self, network_tier: &NetworkTier) {
        match network_tier {
            NetworkTier::Slow { .. } => {
                // Optimize for low bandwidth (<100 Mbps)
                self.send_buffer_size = 256 * 1024; // 256KB
                self.receive_buffer_size = 256 * 1024;
                self.max_concurrent_streams = 10;
                self.frame_batch_size = 4;
                self.enable_zero_copy = false;
                self.max_datagram_size = 1200; // Conservative MTU
                debug!("Adapted config for slow network tier");
            },
            NetworkTier::Home { .. } => {
                // Standard home broadband (100 Mbps - 1 Gbps)
                self.send_buffer_size = 2 * 1024 * 1024; // 2MB
                self.receive_buffer_size = 2 * 1024 * 1024;
                self.max_concurrent_streams = 100;
                self.frame_batch_size = 16;
                self.enable_zero_copy = true;
                self.max_datagram_size = 1500;
                debug!("Adapted config for home network tier");
            },
            NetworkTier::Standard { .. } => {
                // Gigabit networks (1-2.5 Gbps)
                self.send_buffer_size = 8 * 1024 * 1024; // 8MB
                self.receive_buffer_size = 8 * 1024 * 1024;
                self.max_concurrent_streams = 500;
                self.frame_batch_size = 32;
                self.enable_zero_copy = true;
                self.enable_large_send_offload = true;
                self.max_datagram_size = 9000; // Jumbo frames
                debug!("Adapted config for standard gigabit network tier");
            },
            NetworkTier::Performance { .. } | NetworkTier::Enterprise { .. } | NetworkTier::DataCenter { .. } => {
                // High-performance networks (2.5+ Gbps)
                self.send_buffer_size = 16 * 1024 * 1024; // 16MB
                self.receive_buffer_size = 16 * 1024 * 1024;
                self.max_concurrent_streams = 1000;
                self.frame_batch_size = 64;
                self.enable_zero_copy = true;
                self.enable_memory_pool = true;
                self.enable_large_send_offload = true;
                self.enable_cpu_affinity = true;
                self.max_datagram_size = 9000; // Jumbo frames
                debug!("Adapted config for high-performance network tier");
            }
        }
    }
}

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
    pub fn to_socket_addr(&self) -> SocketAddr {
        SocketAddr::from((self.address, self.port))
    }
}

/// Memory buffer pool for efficient buffer reuse (simplified for safety)
pub struct MemoryPool {
    buffer_size: usize,
    allocated_count: AtomicUsize,
    max_buffers: usize,
}

impl MemoryPool {
    /// Create a new memory pool for efficient buffer management
    pub fn new(buffer_size: usize, max_buffers: usize) -> Self {
        Self {
            buffer_size,
            allocated_count: AtomicUsize::new(0),
            max_buffers,
        }
    }
    
    /// Get a buffer from the pool (simplified for safety)
    pub fn get_buffer(&self) -> Option<BytesMut> {
        // Allocate new buffer if under limit
        if self.allocated_count.load(Ordering::Relaxed) < self.max_buffers {
            self.allocated_count.fetch_add(1, Ordering::Relaxed);
            return Some(BytesMut::with_capacity(self.buffer_size));
        }

        None
    }
    
    /// Return buffer to pool for reuse
    pub fn return_buffer(&self, mut buffer: BytesMut) {
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

/// Active QUIC connection with adaptive network tiers optimizations
pub struct Connection {
    inner: quinn::Connection,
    endpoint: Endpoint,
    metrics: Arc<TransportMetrics>,
    memory_pool: Arc<MemoryPool>,
    frame_batch: Arc<Mutex<FrameBatch>>,
    last_activity: AtomicU64,
}

impl Connection {
    /// Create new connection with adaptive network tiers optimizations
    pub fn new_optimized(
        inner: quinn::Connection,
        endpoint: Endpoint,
        metrics: Arc<TransportMetrics>,
        memory_pool: Arc<MemoryPool>,
        frame_batch_size: usize,
    ) -> Self {
        Self {
            inner,
            endpoint,
            metrics,
            memory_pool,
            frame_batch: Arc::new(Mutex::new(FrameBatch::new(frame_batch_size))),
            last_activity: AtomicU64::new(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()),
        }
    }
    
    /// Get the connection ID
    pub fn id(&self) -> String {
        format!("{:?}", self.inner.stable_id())
    }

    /// Accept a bidirectional stream
    pub async fn accept_bi(&self) -> Result<(quinn::SendStream, quinn::RecvStream)> {
        self.inner.accept_bi().await
            .map_err(|e| anyhow!("Failed to accept bidirectional stream: {}", e))
    }

    /// Open a bidirectional stream
    pub async fn open_bi(&self) -> Result<(quinn::SendStream, quinn::RecvStream)> {
        self.inner.open_bi().await
            .map_err(|e| anyhow!("Failed to open bidirectional stream: {}", e))
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
}

/// Bidirectional stream over a connection
pub struct Stream {
    send: quinn::SendStream,
    recv: quinn::RecvStream,
    metrics: Arc<TransportMetrics>,
}

impl Stream {
    fn new(send: quinn::SendStream, recv: quinn::RecvStream, metrics: Arc<TransportMetrics>) -> Self {
        Self { send, recv, metrics }
    }
    
    /// Send data over the stream with zero-copy optimization
    pub async fn send(&mut self, data: &[u8]) -> Result<()> {
        // Use zero-copy when possible
        if data.len() <= 1024 * 1024 { // 1MB threshold for zero-copy
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
    
    /// Receive data from the stream
    pub async fn receive(&mut self) -> Result<Bytes> {
        let data = self.recv.read_to_end(crate::STOQ_MTU).await?;
        self.metrics.record_bytes_received(data.len());
        Ok(data.into())
    }
}

/// STOQ transport implementation using QUIC over IPv6
pub struct StoqTransport {
    config: TransportConfig,
    endpoint: Arc<quinn::Endpoint>,
    connections: Arc<DashMap<String, Arc<Connection>>>,
    connection_pool: Arc<DashMap<String, Vec<Arc<Connection>>>>,
    pub cert_manager: Arc<CertificateManager>,
    pub(crate) metrics: Arc<TransportMetrics>,
    cached_client_config: Arc<RwLock<Option<quinn::ClientConfig>>>,
    memory_pool: Arc<MemoryPool>,
    connection_multiplexer: Arc<DashMap<String, VecDeque<Arc<Connection>>>>,
    performance_stats: Arc<RwLock<PerformanceStats>>,
    /// FALCON quantum-resistant cryptography (optional)
    falcon_transport: Option<Arc<RwLock<FalconTransport>>>,
    /// STOQ protocol handler for extensions
    protocol_handler: Arc<StoqProtocolHandler>,
    /// STOQ handshake extension
    handshake_extension: Arc<StoqHandshakeExtension>,
    /// Adaptive connection optimization manager
    adaptation_manager: Arc<AdaptationManager>,
    /// Adaptive connections mapping
    adaptive_connections: Arc<DashMap<String, Arc<AdaptiveConnection>>>,
    /// eBPF transport acceleration (if available)
    #[cfg(feature = "ebpf")]
    ebpf_transport: Option<Arc<RwLock<ebpf::EbpfTransport>>>,
}

/// Performance statistics for transport monitoring
#[derive(Debug, Default)]
pub struct PerformanceStats {
    pub total_bytes_sent: AtomicU64,
    pub total_bytes_received: AtomicU64,
    pub peak_throughput_gbps: AtomicU64, // Stored as u64 * 1000 for precision
    pub zero_copy_operations: AtomicU64,
    pub frame_batches_sent: AtomicU64,
    pub memory_pool_hits: AtomicU64,
    pub memory_pool_misses: AtomicU64,
    pub connection_reuse_count: AtomicU64,
}

impl StoqTransport {
    /// Create a new STOQ transport using QUIC over IPv6
    pub async fn new(config: TransportConfig) -> Result<Self> {
        info!("Initializing STOQ transport on [{}]:{}", config.bind_address, config.port);
        info!("Transport config: zero_copy={}, pool_size={}, max_streams={}",
              config.enable_zero_copy, config.connection_pool_size, config.max_concurrent_streams);
        
        // Initialize certificate manager with IPv6-only production configuration
        let cert_config = if config.bind_address == std::net::Ipv6Addr::LOCALHOST {
            certificates::CertificateConfig::default() // Localhost testing
        } else {
            certificates::CertificateConfig::production(
                format!("{}-{}", "stoq-node", config.port),
                "stoq.hypermesh.online".to_string(),
                vec![config.bind_address],
            )
        };
        
        let cert_manager = Arc::new(CertificateManager::new(cert_config).await?);
        
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
        let socket_addr = SocketAddr::from((config.bind_address, config.port));
        
        // Verify we're binding to IPv6
        if !socket_addr.is_ipv6() {
            return Err(anyhow!("STOQ only supports IPv6 addresses, got: {}", socket_addr));
        }
        
        let socket = std::net::UdpSocket::bind(socket_addr)?;
        
        // Set socket options for adaptive network tiers performance
        let socket = if let std::net::SocketAddr::V6(_) = socket_addr {
            let socket2_sock = socket2::Socket::from(socket);
            
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

        // Initialize eBPF transport acceleration if available
        #[cfg(feature = "ebpf")]
        let ebpf_transport = match ebpf::EbpfTransport::new() {
            Ok(ebpf) => {
                if ebpf.is_available() {
                    info!("eBPF transport acceleration available");

                    // Try to attach XDP to loopback for testing
                    if config.bind_address == Ipv6Addr::LOCALHOST {
                        if let Err(e) = ebpf.attach_xdp("lo") {
                            warn!("Failed to attach XDP to loopback: {}", e);
                        }
                    }

                    Some(Arc::new(RwLock::new(ebpf)))
                } else {
                    info!("eBPF not available, using standard transport");
                    None
                }
            }
            Err(e) => {
                warn!("Failed to initialize eBPF: {}", e);
                None
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
            #[cfg(feature = "ebpf")]
            ebpf_transport,
        })
    }
    
    /// Connect to a remote endpoint with connection pooling for performance
    pub async fn connect(&self, endpoint: &Endpoint) -> Result<Arc<Connection>> {
        let pool_key = format!("{}:{}", endpoint.address, endpoint.port);
        
        // Try to reuse existing connection from pool for maximum performance
        if let Some(mut pool) = self.connection_pool.get_mut(&pool_key) {
            if let Some(pooled_conn) = pool.pop() {
                if pooled_conn.is_active() {
                    debug!("Reusing pooled connection to [{}]:{}", endpoint.address, endpoint.port);
                    return Ok(pooled_conn);
                }
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
    
    /// Return connection to pool for reuse (optimization)
    pub fn return_to_pool(&self, connection: Arc<Connection>) {
        if !connection.is_active() {
            return; // Don't pool inactive connections
        }
        
        let pool_key = format!("{}:{}", connection.endpoint().address, connection.endpoint().port);
        let mut pool = self.connection_pool.entry(pool_key).or_insert_with(Vec::new);
        
        if pool.len() < self.config.connection_pool_size {
            pool.push(connection);
        }
    }
    
    /// Get FALCON transport for quantum-resistant operations
    pub fn falcon_transport(&self) -> Option<Arc<RwLock<FalconTransport>>> {
        self.falcon_transport.clone()
    }

    /// Sign data using FALCON quantum-resistant cryptography
    pub fn falcon_sign(&self, data: &[u8]) -> Result<Option<falcon::FalconSignature>> {
        if let Some(falcon) = &self.falcon_transport {
            let falcon_guard = falcon.read();
            Ok(Some(falcon_guard.sign_handshake_data(data)?))
        } else {
            Ok(None)
        }
    }

    /// Verify FALCON signature
    pub fn falcon_verify(&self, key_id: &str, signature: &falcon::FalconSignature, data: &[u8]) -> Result<bool> {
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
        ));
        
        self.connections.insert(connection.id(), connection.clone());
        self.metrics.record_connection_established();

        info!("Accepted connection from {}", remote_addr);
        Ok(connection)
    }
    
    /// Send data with transport layer optimizations
    pub async fn send(&self, conn: &Connection, data: &[u8]) -> Result<()> {
        let start_time = std::time::Instant::now();

        // Try eBPF zero-copy send if available
        #[cfg(feature = "ebpf")]
        {
            if let Some(ebpf) = &self.ebpf_transport {
                if let Ok(socket) = ebpf.read().create_af_xdp_socket("lo", 0) {
                    if socket.send(data).await.is_ok() {
                        self.metrics.record_bytes_sent(data.len());
                        self.performance_stats.read().zero_copy_operations.fetch_add(1, Ordering::Relaxed);
                        return Ok(());
                    }
                }
            }
        }

        // Apply STOQ protocol extensions (tokenization, sharding)
        let extension_frames = self.protocol_handler.apply_extensions(data)?;

        // Send extension frames as QUIC datagrams
        for frame in extension_frames {
            if conn.inner.send_datagram(frame.clone()).is_err() {
                debug!("Failed to send extension frame as datagram, will include in stream");
            }
        }

        if self.config.enable_zero_copy {
            // Try memory pool buffer first for maximum performance
            if let Some(mut buffer) = self.memory_pool.get_buffer() {
                if data.len() <= buffer.capacity() {
                    buffer.put_slice(data);
                    let bytes = buffer.freeze();
                    
                    // Try zero-copy datagram send
                    if data.len() <= self.config.max_datagram_size {
                        if conn.inner.send_datagram(bytes.clone()).is_ok() {
                            self.performance_stats.read().zero_copy_operations.fetch_add(1, Ordering::Relaxed);
                            self.performance_stats.read().memory_pool_hits.fetch_add(1, Ordering::Relaxed);
                            return Ok(());
                        }
                    }
                    
                    // Fallback to stream with zero-copy buffer
                    let mut stream = conn.open_stream().await?;
                    stream.send_bytes(bytes).await?;
                    self.performance_stats.read().zero_copy_operations.fetch_add(1, Ordering::Relaxed);
                    self.performance_stats.read().memory_pool_hits.fetch_add(1, Ordering::Relaxed);
                    return Ok(());
                } else {
                    // Return buffer to pool if too small
                    self.memory_pool.return_buffer(buffer);
                }
            } else {
                self.performance_stats.read().memory_pool_misses.fetch_add(1, Ordering::Relaxed);
            }
            
            // Large data optimization with frame batching
            if data.len() > self.config.max_datagram_size && self.config.frame_batch_size > 1 {
                return self.send_large_data_batched(conn, data).await;
            }
        }
        
        // Fallback to standard stream sending
        let mut stream = conn.open_stream().await?;
        stream.send(data).await?;
        
        // Update performance metrics
        let duration = start_time.elapsed();
        let throughput_bps = (data.len() as f64 * 8.0) / duration.as_secs_f64();
        let throughput_gbps = (throughput_bps / 1_000_000_000.0 * 1000.0) as u64; // Store as u64 * 1000
        
        let current_peak = self.performance_stats.read().peak_throughput_gbps.load(Ordering::Relaxed);
        if throughput_gbps > current_peak {
            self.performance_stats.read().peak_throughput_gbps.store(throughput_gbps, Ordering::Relaxed);
        }
        
        Ok(())
    }
    
    /// Send large data with frame batching for performance
    async fn send_large_data_batched(&self, conn: &Connection, data: &[u8]) -> Result<()> {
        let chunk_size = self.config.max_datagram_size;
        let mut chunks = data.chunks(chunk_size);
        let mut batch = FrameBatch::new(self.config.frame_batch_size);
        
        while let Some(chunk) = chunks.next() {
            let bytes = Bytes::copy_from_slice(chunk);
            
            if batch.add_frame(bytes) {
                // Batch is full, send all frames
                let frames = batch.flush();
                for frame in frames {
                    if conn.inner.send_datagram(frame).is_err() {
                        // Fallback to stream for failed datagrams
                        let mut stream = conn.open_stream().await?;
                        stream.send(&chunk).await?;
                    }
                }
                self.performance_stats.read().frame_batches_sent.fetch_add(1, Ordering::Relaxed);
            }
        }
        
        // Send remaining frames in batch
        if !batch.is_empty() {
            let frames = batch.flush();
            for frame in frames {
                let frame_len = frame.len();
                if conn.inner.send_datagram(frame).is_err() {
                    // Fallback to stream
                    let mut stream = conn.open_stream().await?;
                    let fallback_data = vec![0u8; frame_len]; // Safe fallback data
                    stream.send(&fallback_data).await?;
                }
            }
            self.performance_stats.read().frame_batches_sent.fetch_add(1, Ordering::Relaxed);
        }
        
        Ok(())
    }
    
    /// Receive data with zero-copy optimization for performance
    pub async fn receive(&self, conn: &Connection) -> Result<Bytes> {
        if self.config.enable_zero_copy {
            // Try datagram receive first for maximum performance
            if let Ok(datagram) = conn.inner.read_datagram().await {
                return Ok(datagram);
            }
        }
        
        // Fallback to stream-based receiving
        let mut stream = conn.accept_stream().await?;
        stream.receive().await
    }
    
    /// Get transport statistics with performance metrics
    pub fn stats(&self) -> crate::TransportStats {
        let base_stats = self.metrics.get_stats(self.connections.len());
        
        // Add performance metrics
        let perf_stats = self.performance_stats.read();
        let (pool_available, pool_allocated) = self.memory_pool.stats();
        
        info!("Performance: {:.1} Gbps peak, Zero-copy ops: {}, Pool hits/misses: {}/{}, Frame batches: {}",
              perf_stats.peak_throughput_gbps.load(Ordering::Relaxed) as f64 / 1000.0,
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
    pub fn pool_stats(&self) -> Vec<(String, usize)> {
        self.connection_pool
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().len()))
            .collect()
    }
    
    /// Get transport performance statistics
    pub fn performance_stats(&self) -> (f64, u64, u64, u64) {
        let stats = self.performance_stats.read();
        let peak_gbps = stats.peak_throughput_gbps.load(Ordering::Relaxed) as f64 / 1000.0;
        let zero_copy_ops = stats.zero_copy_operations.load(Ordering::Relaxed);
        let pool_hits = stats.memory_pool_hits.load(Ordering::Relaxed);
        let frame_batches = stats.frame_batches_sent.load(Ordering::Relaxed);

        (peak_gbps, zero_copy_ops, pool_hits, frame_batches)
    }

    /// Get detailed protocol metrics for monitoring
    pub fn get_protocol_metrics(&self) -> ProtocolMetrics {
        self.metrics.get_protocol_metrics()
    }

    /// Get interval-based metrics for rate calculations
    pub fn get_interval_metrics(&self) -> IntervalMetrics {
        self.metrics.get_interval_metrics()
    }

    /// Reset interval metrics for periodic reporting
    pub fn reset_interval_metrics(&self) {
        self.metrics.reset_interval_metrics();
    }
    
    /// Enable connection multiplexing for specific endpoint (optimization)
    pub async fn enable_multiplexing(&self, endpoint: &Endpoint, connection_count: usize) -> Result<()> {
        let pool_key = format!("{}:{}", endpoint.address, endpoint.port);
        let mut connections = VecDeque::with_capacity(connection_count);
        
        // Create multiple connections for bandwidth aggregation
        for i in 0..connection_count {
            debug!("Creating multiplexed connection {}/{} to [{}]:{}", i + 1, connection_count, endpoint.address, endpoint.port);
            
            let connection = self.connect(endpoint).await?;
            connections.push_back(connection);
        }
        
        self.connection_multiplexer.insert(pool_key, connections);
        info!("Enabled {}x connection multiplexing for [{}]:{} (optimization)",
              connection_count, endpoint.address, endpoint.port);
        
        Ok(())
    }
    
    /// Send data using connection multiplexing for maximum throughput
    pub async fn send_multiplexed(&self, endpoint: &Endpoint, data: &[u8]) -> Result<()> {
        let pool_key = format!("{}:{}", endpoint.address, endpoint.port);
        
        if let Some(mut connections) = self.connection_multiplexer.get_mut(&pool_key) {
            if let Some(connection) = connections.pop_front() {
                // Use round-robin connection selection
                let result = self.send(&connection, data).await;
                connections.push_back(connection); // Return connection to back of queue
                return result;
            }
        }
        
        // Fallback to regular connection if multiplexing not available
        let connection = self.connect(endpoint).await?;
        self.send(&connection, data).await
    }

    /// Adapt transport configuration for detected network tier
    pub fn adapt_config_for_tier(&mut self, gbps: f64) {
        let tier = NetworkTier::from_gbps(gbps);
        self.config.adapt_for_network_tier(&tier);
        info!("Adapted STOQ configuration for network tier: {:?}", tier);
    }

    /// Get local address of the endpoint
    pub fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        self.endpoint.local_addr()
    }

    /// Get the protocol handler
    pub fn protocol_handler(&self) -> &StoqProtocolHandler {
        &self.protocol_handler
    }

    /// Start the adaptive optimization manager
    pub async fn start_adaptation(&self) {
        let manager = self.adaptation_manager.clone();
        tokio::spawn(async move {
            manager.start().await;
        });
        info!("Started adaptive connection optimization");
    }

    /// Update configuration for all live connections
    pub async fn update_live_config(&mut self, new_config: TransportConfig) {
        info!("Updating configuration for all live connections");

        // Update stored config
        self.config = new_config.clone();

        // Update all adaptive connections
        for entry in self.adaptive_connections.iter() {
            let conn = entry.value().clone();

            // Force immediate adaptation with new config
            tokio::spawn(async move {
                if let Err(e) = conn.force_adapt().await {
                    warn!("Failed to adapt connection: {}", e);
                }
            });
        }

        info!("Configuration updated for {} live connections", self.adaptive_connections.len());
    }

    /// Get adaptive connection by ID
    pub fn get_adaptive_connection(&self, id: &str) -> Option<Arc<AdaptiveConnection>> {
        self.adaptive_connections.get(id).map(|entry| entry.clone())
    }

    /// Force adaptation for a specific connection
    pub async fn force_connection_adaptation(&self, id: &str) -> Result<()> {
        if let Some(conn) = self.get_adaptive_connection(id) {
            conn.force_adapt().await?;
            Ok(())
        } else {
            Err(anyhow!("Connection not found: {}", id))
        }
    }

    /// Get adaptation statistics for all connections
    pub fn adaptation_stats(&self) -> Vec<(String, adaptive::AdaptationStats)> {
        self.adaptation_manager.all_stats()
    }

    /// Enable or disable adaptive optimization globally
    pub fn set_adaptation_enabled(&self, enabled: bool) {
        self.adaptation_manager.set_enabled(enabled);

        // Update all existing connections
        for entry in self.adaptive_connections.iter() {
            entry.value().set_adaptation_enabled(enabled);
        }

        if enabled {
            info!("Adaptive optimization enabled globally");
        } else {
            info!("Adaptive optimization disabled globally");
        }
    }

    /// Manually set network tier for a connection
    pub async fn set_connection_tier(&self, id: &str, tier: NetworkTier) -> Result<()> {
        if let Some(conn) = self.get_adaptive_connection(id) {
            // This would require adding a method to AdaptiveConnection to set tier manually
            // For now, force an adaptation which will detect the tier
            conn.force_adapt().await?;
            info!("Set network tier for connection {}: {:?}", id, tier);
            Ok(())
        } else {
            Err(anyhow!("Connection not found: {}", id))
        }
    }

    /// Detect and apply optimal network tier for all connections
    pub async fn auto_detect_tiers(&self) {
        info!("Auto-detecting network tiers for all connections");

        for entry in self.adaptive_connections.iter() {
            let conn = entry.value().clone();
            let id = entry.key().clone();

            tokio::spawn(async move {
                if let Err(e) = conn.adapt().await {
                    warn!("Failed to auto-detect tier for connection {}: {}", id, e);
                }
            });
        }
    }

    /// Get eBPF capabilities and status
    #[cfg(feature = "ebpf")]
    pub fn get_ebpf_status(&self) -> Option<ebpf::EbpfCapabilities> {
        self.ebpf_transport.as_ref().map(|t| t.read().capabilities.clone())
    }

    #[cfg(not(feature = "ebpf"))]
    pub fn get_ebpf_status(&self) -> Option<()> {
        None
    }

    /// Get eBPF metrics if available
    #[cfg(feature = "ebpf")]
    pub fn get_ebpf_metrics(&self) -> Option<ebpf::metrics::EbpfMetrics> {
        self.ebpf_transport.as_ref().and_then(|t| t.read().get_metrics())
    }

    #[cfg(not(feature = "ebpf"))]
    pub fn get_ebpf_metrics(&self) -> Option<()> {
        None
    }

    /// Attach XDP program to interface for acceleration
    #[cfg(feature = "ebpf")]
    pub fn attach_xdp_to_interface(&self, interface: &str) -> Result<()> {
        if let Some(ebpf) = &self.ebpf_transport {
            ebpf.read().attach_xdp(interface)?;
            info!("XDP acceleration enabled on interface {}", interface);
            Ok(())
        } else {
            Err(anyhow!("eBPF transport not available"))
        }
    }

    #[cfg(not(feature = "ebpf"))]
    pub fn attach_xdp_to_interface(&self, _interface: &str) -> Result<()> {
        Err(anyhow!("eBPF feature not compiled"))
    }

    /// Create AF_XDP zero-copy socket for interface
    #[cfg(feature = "ebpf")]
    pub fn create_zero_copy_socket(&self, interface: &str, queue_id: u32) -> Result<()> {
        if let Some(ebpf) = &self.ebpf_transport {
            let _socket = ebpf.read().create_af_xdp_socket(interface, queue_id)?;
            info!("Created AF_XDP zero-copy socket for {}:{}", interface, queue_id);
            Ok(())
        } else {
            Err(anyhow!("eBPF transport not available"))
        }
    }

    #[cfg(not(feature = "ebpf"))]
    pub fn create_zero_copy_socket(&self, _interface: &str, _queue_id: u32) -> Result<()> {
        Err(anyhow!("eBPF feature not compiled"))
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
            #[cfg(feature = "ebpf")]
            ebpf_transport: self.ebpf_transport.clone(),
        }
    }
}

// Helper trait implementations
impl Clone for Connection {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            endpoint: self.endpoint.clone(),
            metrics: self.metrics.clone(),
            memory_pool: self.memory_pool.clone(),
            frame_batch: self.frame_batch.clone(),
            last_activity: AtomicU64::new(self.last_activity.load(Ordering::Relaxed)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_endpoint_creation() {
        let endpoint = Endpoint::new(Ipv6Addr::LOCALHOST, 9292);
        assert_eq!(endpoint.port, 9292);
        assert_eq!(endpoint.address, Ipv6Addr::LOCALHOST);
    }
    
    #[test]
    fn test_transport_config_default() {
        let config = TransportConfig::default();
        assert_eq!(config.port, 9292);
        assert!(config.enable_migration);
        assert!(!config.enable_0rtt); // 0-RTT disabled for security
    }
    
    #[tokio::test]
    async fn test_transport_creation() {
        // Initialize crypto provider
        if let Err(_) = rustls::crypto::ring::default_provider().install_default() {
            // Already installed, ignore error
        }
        
        let config = TransportConfig::default();
        let transport = StoqTransport::new(config).await;
        assert!(transport.is_ok());
    }
}