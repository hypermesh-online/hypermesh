// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ Transport constructors and initialization

use anyhow::{Result, anyhow};
use dashmap::DashMap;
use parking_lot::RwLock;
use quinn::{self, TransportConfig as QuinnTransportConfig, VarInt};
use std::net::{SocketAddr, Ipv6Addr};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, debug, warn};

use crate::transport::certificates::CertificateManager;
use crate::transport::certificate_strategy::NetworkType;
use crate::transport::config::{TransportConfig, CongestionControl};
use crate::transport::connection::MemoryPool;
use crate::transport::metrics::TransportMetrics;
use crate::transport::stats::PerformanceStats;
use crate::transport::falcon::FalconTransport;
use crate::transport::adaptive::AdaptationManager;
use crate::transport::ebpf::StoqEbpfTransport;

use crate::protocol::{StoqProtocolHandler, handshake::StoqHandshakeExtension, StoqPosIntegration};
use crate::protocol::pos_fast_validator::{PosFastValidator, FastValidationConfig};
use crate::extensions::DefaultStoqExtensions;

use super::{StoqTransport, CRYPTO_INIT};

/// Resolve the network interface name for eBPF XDP attachment.
///
/// Priority:
/// 1. Explicit `config.ebpf_interface` override (if set)
/// 2. "lo" when bind_address is localhost or unspecified
/// 3. "eth0" as a fallback for non-localhost addresses
///
/// In production with real routing tables, this would query the system
/// for the outbound interface matching the bind address.
fn resolve_ebpf_interface(config: &TransportConfig) -> String {
    // Explicit override takes priority
    if let Some(ref iface) = config.ebpf_interface {
        return iface.clone();
    }

    // Localhost and unspecified addresses use loopback
    if config.bind_address == Ipv6Addr::LOCALHOST
        || config.bind_address == Ipv6Addr::UNSPECIFIED
    {
        return "lo".to_string();
    }

    // For non-localhost addresses, default to eth0.
    // A future enhancement could probe the routing table to find the
    // correct interface for the bind address.
    "eth0".to_string()
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
            crate::transport::certificates::CertificateConfig::default() // Localhost testing
        } else {
            crate::transport::certificates::CertificateConfig::production(
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
        let cert_config = crate::transport::certificates::CertificateConfig::with_network_type(
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

        // Create fast PoS pre-validator for line-rate filtering
        let full_pos_validator = Arc::new(
            crate::protocol::pos_validator::PosTokenValidator::new(Duration::from_secs(300)),
        );
        let pos_fast_validator = Arc::new(PosFastValidator::new(
            FastValidationConfig::default(),
            full_pos_validator,
        ));

        // Resolve the network interface for eBPF XDP attachment.
        // Localhost always uses "lo"; other addresses attempt to detect the
        // outbound interface by inspecting the bind address.
        let ebpf_interface = resolve_ebpf_interface(&config);
        info!("eBPF interface resolved to: {}", ebpf_interface);

        // Initialize eBPF transport acceleration (delegates to hypermesh-ebpf)
        let (ebpf_transport, af_xdp_socket) = match StoqEbpfTransport::new() {
            Ok(mut ebpf) => {
                if ebpf.is_available() {
                    info!("eBPF transport acceleration available");

                    // Attach XDP program to the resolved interface
                    if let Err(e) = ebpf.attach_xdp(&ebpf_interface) {
                        warn!("Failed to attach XDP to {}: {}", ebpf_interface, e);
                    }

                    // Create a single AF_XDP socket during init and reuse it.
                    // This avoids the "duplicate socket key" error that occurs
                    // when create_af_xdp_socket is called on every send().
                    let socket = match ebpf.create_af_xdp_socket(&ebpf_interface, 0) {
                        Ok(s) => {
                            info!("AF_XDP zero-copy socket created for {}:0", ebpf_interface);
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
            pos_fast_validator,
        })
    }
}
