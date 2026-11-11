// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Optional kernel acceleration; disabled configurations never probe the kernel.

use crate::transport::config::TransportConfig;
use crate::transport::ebpf::{AfXdpSocket, StoqEbpfTransport};
use parking_lot::RwLock;
use std::{net::Ipv6Addr, sync::Arc};
use tracing::{debug, info, warn};

type Acceleration = (
    Option<Arc<RwLock<StoqEbpfTransport>>>,
    Option<Arc<AfXdpSocket>>,
);

/// Fallback interface detection when no interface was injected.
///
/// The Substrate (`base::Substrate::active_interface()`) is the real, carrier-aware
/// producer; the node binary injects its result into `config.interface`. This
/// probe list is only the degraded fallback for callers that construct a transport
/// without a Substrate (tests, embedded uses). Probes common names and returns the
/// first that exists, falling back to "lo".
fn detect_outbound_interface() -> String {
    // Check common interface names in priority order
    for iface in &["eth0", "ens3", "ens4", "enp0s3", "wlan0", "wlp2s0"] {
        let path = format!("/sys/class/net/{}", iface);
        if std::path::Path::new(&path).exists() {
            debug!("Fallback-detected outbound interface: {}", iface);
            return iface.to_string();
        }
    }
    debug!("No outbound interface detected, falling back to lo");
    "lo".to_string()
}

/// Resolve the network interface name for eBPF XDP attachment.
///
/// Priority:
/// 1. Explicit `config.ebpf_interface` override (if set)
/// 2. Substrate-injected `config.interface` (R16 carrier-aware selection)
/// 3. "lo" when bind_address is localhost (and NOT wan_enabled)
/// 4. Fallback auto-detection when UNSPECIFIED or wan_enabled or other
pub(super) fn resolve_ebpf_interface(config: &TransportConfig) -> String {
    // Explicit override takes highest priority.
    if let Some(ref iface) = config.ebpf_interface {
        return iface.clone();
    }

    // Substrate-selected interface: the real, carrier-aware choice injected by
    // the node binary. Preferred over the hardcoded probe list.
    if let Some(ref iface) = config.interface {
        debug!("Using Substrate-injected interface: {}", iface);
        return iface.clone();
    }

    // Localhost without WAN uses loopback.
    if config.bind_address == Ipv6Addr::LOCALHOST && !config.wan_enabled {
        return "lo".to_string();
    }

    // UNSPECIFIED, wan_enabled, or any other address: fall back to detection.
    detect_outbound_interface()
}

pub(super) fn initialize(config: &TransportConfig) -> Acceleration {
    if !config.enable_kernel_acceleration {
        return (None, None);
    }
    let interface = resolve_ebpf_interface(config);
    info!("eBPF interface resolved to: {}", interface);
    match StoqEbpfTransport::new() {
        Ok(mut ebpf) if ebpf.is_available() => {
            info!("eBPF transport acceleration available");
            let socket = attach_and_create_socket(&mut ebpf, &interface);
            (Some(Arc::new(RwLock::new(ebpf))), socket)
        }
        Ok(_) => {
            info!("eBPF not available, using standard transport");
            (None, None)
        }
        Err(e) => {
            warn!("Failed to initialize eBPF: {}", e);
            (None, None)
        }
    }
}

fn attach_and_create_socket(
    ebpf: &mut StoqEbpfTransport,
    interface: &str,
) -> Option<Arc<AfXdpSocket>> {
    if let Err(e) = ebpf.attach_xdp(interface) {
        warn!("Failed to attach XDP to {}: {}", interface, e);
    }
    match ebpf.create_af_xdp_socket(interface, 0) {
        Ok(s) => {
            info!("AF_XDP zero-copy socket created for {}:0", interface);
            Some(Arc::new(s))
        }
        Err(e) => {
            debug!("AF_XDP socket not available (will use standard I/O): {}", e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_and_disabled_kernel_acceleration() {
        let mut config = TransportConfig::default();
        assert!(config.enable_kernel_acceleration);
        config.enable_kernel_acceleration = false;
        config.bind_address = Ipv6Addr::LOCALHOST;
        config.port = 0;
        assert_eq!(config.bind_address, Ipv6Addr::LOCALHOST);
        assert_eq!(config.port, 0);
        assert!(config.max_datagram_size > 0);
        let (ebpf, socket) = initialize(&config);
        assert!(ebpf.is_none());
        assert!(socket.is_none());
    }

    #[test]
    fn missing_serialized_setting_defaults_to_enabled() {
        let mut value = serde_json::to_value(TransportConfig::default()).expect("serialize config");
        value
            .as_object_mut()
            .expect("config object")
            .remove("enable_kernel_acceleration");
        let config: TransportConfig = serde_json::from_value(value).expect("deserialize config");
        assert!(config.enable_kernel_acceleration);
    }
}
