// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Synchronous construction helpers for the transport manager.

use crate::extensions::DefaultStoqExtensions;
use crate::protocol::{
    handshake::StoqHandshakeExtension,
    pos_fast_validator::{FastValidationConfig, PosFastValidator},
    StoqProtocolHandler,
};
use crate::transport::config::{CongestionControl, TransportConfig};
use crate::transport::connection::MemoryPool;
use crate::transport::falcon::FalconTransport;
use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use quinn::{TransportConfig as QuinnTransportConfig, VarInt};
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

pub(super) fn quinn_transport(config: &TransportConfig) -> Result<QuinnTransportConfig> {
    let mut transport = QuinnTransportConfig::default();
    transport.max_concurrent_bidi_streams(config.max_concurrent_streams.into());
    transport.max_concurrent_uni_streams(config.max_concurrent_streams.into());
    transport.max_idle_timeout(Some(config.max_idle_timeout.try_into()?));
    transport.send_window(config.send_buffer_size as u64);
    transport
        .receive_window(VarInt::try_from(config.receive_buffer_size as u64).unwrap_or(VarInt::MAX));
    transport.datagram_receive_buffer_size(Some(config.max_datagram_size));
    transport.datagram_send_buffer_size(config.max_datagram_size);
    Ok(transport)
}

pub(super) fn log_congestion(config: &TransportConfig) {
    match config.congestion_control {
        CongestionControl::Bbr2 => debug!("Using BBR-optimized settings for high performance"),
        CongestionControl::Cubic => debug!("Using CUBIC congestion control"),
        CongestionControl::NewReno => debug!("Using NewReno congestion control"),
    }
}

pub(super) fn server_config(config: rustls::ServerConfig) -> Result<quinn::ServerConfig> {
    Ok(quinn::ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(config)?,
    )))
}

pub(super) fn client_config(config: rustls::ClientConfig) -> Result<quinn::ClientConfig> {
    Ok(quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(config)?,
    )))
}

pub(super) fn bind_socket(config: &TransportConfig) -> Result<UdpSocket> {
    #[cfg(test)]
    let bind_port = 0;
    #[cfg(not(test))]
    let bind_port = config.port;
    let socket_addr = SocketAddr::from((config.bind_address, bind_port));
    if !socket_addr.is_ipv6() {
        return Err(anyhow!(
            "STOQ only supports IPv6 addresses, got: {socket_addr}"
        ));
    }
    let socket = UdpSocket::bind(socket_addr)?;
    if let SocketAddr::V6(_) = socket_addr {
        let socket = socket2::Socket::from(socket);
        set_socket_options(&socket, config);
        return Ok(socket.into());
    }
    Ok(socket)
}

fn set_socket_options(socket: &socket2::Socket, config: &TransportConfig) {
    if let Err(e) = socket.set_reuse_address(true) {
        warn!("Could not set SO_REUSEADDR (continuing anyway): {}", e);
    }
    if let Err(e) = socket.set_only_v6(true) {
        warn!(
            "Could not set IPv6-only socket option (continuing anyway): {}",
            e
        );
    }
    if let Err(e) = socket.set_send_buffer_size(config.send_buffer_size) {
        warn!("Could not set send buffer size: {}", e);
    }
    if let Err(e) = socket.set_recv_buffer_size(config.receive_buffer_size) {
        warn!("Could not set receive buffer size: {}", e);
    }
}

pub(super) fn falcon(config: &TransportConfig) -> Option<Arc<RwLock<FalconTransport>>> {
    if !config.enable_falcon_crypto {
        info!("FALCON cryptography disabled");
        return None;
    }
    let mut falcon = FalconTransport::new(config.falcon_variant);
    if let Err(e) = falcon.generate_local_keypair() {
        warn!("Failed to generate FALCON keypair: {}", e);
        return None;
    }
    info!(
        "FALCON quantum-resistant cryptography enabled with {:?}",
        config.falcon_variant
    );
    Some(Arc::new(RwLock::new(falcon)))
}

pub(super) fn protocol_handler(
    extensions: &Arc<DefaultStoqExtensions>,
    falcon: &Option<Arc<RwLock<FalconTransport>>>,
    config: &TransportConfig,
) -> Arc<StoqProtocolHandler> {
    Arc::new(StoqProtocolHandler::new(
        extensions.clone(),
        falcon.clone(),
        config.max_datagram_size,
    ))
}

pub(super) fn handshake_extension(
    falcon: &Option<Arc<RwLock<FalconTransport>>>,
    config: &TransportConfig,
) -> Arc<StoqHandshakeExtension> {
    Arc::new(StoqHandshakeExtension::new(
        falcon.clone(),
        false,
        config.enable_falcon_crypto,
    ))
}

pub(super) fn endpoint(
    server: quinn::ServerConfig,
    socket: UdpSocket,
    client: &quinn::ClientConfig,
) -> Result<quinn::Endpoint> {
    let mut endpoint = quinn::Endpoint::new(
        quinn::EndpointConfig::default(),
        Some(server),
        socket,
        Arc::new(quinn::TokioRuntime),
    )?;
    endpoint.set_default_client_config(client.clone());
    Ok(endpoint)
}

pub(super) fn memory_pool(config: &TransportConfig) -> Arc<MemoryPool> {
    Arc::new(MemoryPool::new(
        config.max_datagram_size,
        config.memory_pool_size,
    ))
}

pub(super) fn pos_fast_validator() -> Arc<PosFastValidator> {
    let validator = Arc::new(crate::protocol::pos_validator::PosTokenValidator::new(
        Duration::from_secs(300),
    ));
    Arc::new(PosFastValidator::new(
        FastValidationConfig::default(),
        validator,
    ))
}
