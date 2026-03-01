// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ Transport Manager - Main transport layer implementation

use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::Arc;

use super::adaptive::{AdaptationManager, AdaptiveConnection};
use super::certificates::CertificateManager;
use super::config::TransportConfig;
use super::connection::{Connection, MemoryPool};
use super::falcon::FalconTransport;
use super::metrics::TransportMetrics;
use super::stats::PerformanceStats;

use crate::protocol::pos_fast_validator::PosFastValidator;
use crate::protocol::{handshake::StoqHandshakeExtension, StoqPosIntegration, StoqProtocolHandler};

use super::ebpf::StoqEbpfTransport;

mod connections;
mod constructors;
mod monitoring;
mod pos;
mod trait_impl;

// Global initialization for crypto provider
pub(crate) static CRYPTO_INIT: std::sync::Once = std::sync::Once::new();

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
    /// Fast PoS pre-validator for line-rate filtering
    pub(crate) pos_fast_validator: Arc<PosFastValidator>,
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
            pos_fast_validator: self.pos_fast_validator.clone(),
        }
    }
}
