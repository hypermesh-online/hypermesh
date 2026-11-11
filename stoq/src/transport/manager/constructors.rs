// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! STOQ Transport constructors and initialization

use anyhow::Result;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::net::Ipv6Addr;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info};

use crate::transport::adaptive::AdaptationManager;
use crate::transport::certificate_strategy::NetworkType;
use crate::transport::certificates::CertificateManager;
use crate::transport::config::TransportConfig;
use crate::transport::metrics::TransportMetrics;
use crate::transport::stats::PerformanceStats;

use crate::extensions::DefaultStoqExtensions;
use crate::protocol::StoqPosIntegration;

use super::{construction, kernel_acceleration, StoqTransport, CRYPTO_INIT};

#[cfg(test)]
use super::kernel_acceleration::resolve_ebpf_interface;

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

        info!(
            "Initializing STOQ transport on [{}]:{}",
            config.bind_address, config.port
        );
        info!(
            "Transport config: zero_copy={}, pool_size={}, max_streams={}",
            config.enable_zero_copy, config.connection_pool_size, config.max_concurrent_streams
        );

        // Initialize certificate manager with IPv6-only production configuration.
        // Use production certs when wan_enabled or binding to a non-localhost address.
        let cert_config = if config.wan_enabled
            || (config.bind_address != Ipv6Addr::LOCALHOST
                && config.bind_address != Ipv6Addr::UNSPECIFIED)
        {
            let advertised_addr = config.public_ipv6.unwrap_or(config.bind_address);
            crate::transport::certificates::CertificateConfig::production(
                format!("{}-{}", "stoq-node", config.port),
                "stoq.hypermesh.online".to_string(),
                vec![advertised_addr],
            )
        } else {
            crate::transport::certificates::CertificateConfig::default() // Localhost/testing
        };

        let cert_manager = Arc::new(CertificateManager::new(cert_config).await?);

        Self::new_with_cert_manager(config, cert_manager).await
    }

    /// Create a new STOQ transport for specific network type
    pub async fn new_for_network(
        config: TransportConfig,
        network_type: NetworkType,
    ) -> Result<Self> {
        // Initialize crypto provider once (globally)
        CRYPTO_INIT.call_once(|| {
            if let Err(e) = rustls::crypto::ring::default_provider().install_default() {
                // Provider might already be installed, log but don't fail
                debug!("Crypto provider initialization: {:?}", e);
            }
        });

        info!(
            "Initializing STOQ transport for network type: {:?}",
            network_type
        );
        info!(
            "Transport config: zero_copy={}, pool_size={}, max_streams={}",
            config.enable_zero_copy, config.connection_pool_size, config.max_concurrent_streams
        );

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
    async fn new_with_cert_manager(
        config: TransportConfig,
        cert_manager: Arc<CertificateManager>,
    ) -> Result<Self> {
        let server_transport_config = construction::quinn_transport(&config)?;
        let client_transport_config = construction::quinn_transport(&config)?;
        construction::log_congestion(&config);
        let rustls_server_config = cert_manager.server_crypto_config().await?;
        let mut server_config = construction::server_config(rustls_server_config)?;
        server_config.transport_config(Arc::new(server_transport_config));
        let rustls_client_config = cert_manager.client_crypto_config().await?;
        let mut client_config = construction::client_config(rustls_client_config)?;
        client_config.transport_config(Arc::new(client_transport_config));
        let socket = construction::bind_socket(&config)?;
        let endpoint = construction::endpoint(server_config, socket, &client_config)?;
        let metrics = Arc::new(TransportMetrics::new());
        let memory_pool = construction::memory_pool(&config);
        let falcon_transport = construction::falcon(&config);
        let extensions = Arc::new(DefaultStoqExtensions::with_metrics(metrics.clone()));
        let protocol_handler =
            construction::protocol_handler(&extensions, &falcon_transport, &config);
        let handshake_extension = construction::handshake_extension(&falcon_transport, &config);
        let adaptation_manager = Arc::new(AdaptationManager::new(Duration::from_secs(1)));
        let pos_integration = Arc::new(StoqPosIntegration::new(Duration::from_secs(300)));
        let pos_fast_validator = construction::pos_fast_validator();
        let (ebpf_transport, af_xdp_socket) = kernel_acceleration::initialize(&config);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substrate_injected_interface_is_used() {
        let mut config = TransportConfig {
            bind_address: Ipv6Addr::UNSPECIFIED,
            wan_enabled: true,
            ..TransportConfig::default()
        };
        config.interface = Some("eno1".to_string());
        assert_eq!(resolve_ebpf_interface(&config), "eno1");
    }

    #[test]
    fn explicit_ebpf_interface_overrides_substrate_injection() {
        let mut config = TransportConfig::default();
        config.ebpf_interface = Some("wlan5".to_string());
        config.interface = Some("eno1".to_string());
        // Explicit eBPF override wins over the Substrate-injected interface.
        assert_eq!(resolve_ebpf_interface(&config), "wlan5");
    }

    #[test]
    fn localhost_without_injection_uses_loopback() {
        let config = TransportConfig {
            bind_address: Ipv6Addr::LOCALHOST,
            wan_enabled: false,
            ..TransportConfig::default()
        };
        assert!(config.interface.is_none());
        assert_eq!(resolve_ebpf_interface(&config), "lo");
    }
}
