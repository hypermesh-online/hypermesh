// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// The four gateway roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GatewayMode {
    /// HTTP/3 at trust.hypermesh.online — serves initial STOQ connection info.
    Bootstrap,
    /// HTTP/3 access to HyperMesh dashboards.
    InboundProxy,
    /// Bridge private HyperMesh resources to clearnet endpoints.
    OutboundProxy,
    /// Bridge between federated/private/public HyperMesh networks (STOQ-to-STOQ).
    InterNetwork,
}

/// Protocol for backend communication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendProtocol {
    Http3,
    Stoq,
}

/// Configuration for a single backend service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    pub name: String,
    pub addr: SocketAddr,
    pub server_name: String,
    pub protocol: BackendProtocol,
    pub path_prefixes: Vec<String>,
    pub domains: Vec<String>,
}

/// Gateway mode configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeConfig {
    pub enabled_modes: Vec<GatewayMode>,
    pub http3_listen_addr: SocketAddr,
    pub stoq_listen_addr: Option<SocketAddr>,
}

impl Default for ModeConfig {
    fn default() -> Self {
        Self {
            enabled_modes: vec![GatewayMode::Bootstrap, GatewayMode::InboundProxy],
            http3_listen_addr: "[::]:8443".parse().expect("valid default listen addr"),
            stoq_listen_addr: None,
        }
    }
}

impl ModeConfig {
    /// Check whether a given mode is enabled.
    pub fn is_mode_enabled(&self, mode: GatewayMode) -> bool {
        self.enabled_modes.contains(&mode)
    }

    /// Returns `true` when any enabled mode requires a STOQ listener.
    pub fn needs_stoq_listener(&self) -> bool {
        self.enabled_modes.iter().any(|m| {
            matches!(
                m,
                GatewayMode::OutboundProxy | GatewayMode::InterNetwork
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mode_config() {
        let cfg = ModeConfig::default();
        assert_eq!(cfg.enabled_modes.len(), 2);
        assert!(cfg.is_mode_enabled(GatewayMode::Bootstrap));
        assert!(cfg.is_mode_enabled(GatewayMode::InboundProxy));
        assert!(!cfg.is_mode_enabled(GatewayMode::OutboundProxy));
        assert!(!cfg.is_mode_enabled(GatewayMode::InterNetwork));
        assert!(cfg.stoq_listen_addr.is_none());
    }

    #[test]
    fn needs_stoq_listener_false_for_defaults() {
        let cfg = ModeConfig::default();
        assert!(!cfg.needs_stoq_listener());
    }

    #[test]
    fn needs_stoq_listener_true_for_outbound() {
        let cfg = ModeConfig {
            enabled_modes: vec![GatewayMode::OutboundProxy],
            ..ModeConfig::default()
        };
        assert!(cfg.needs_stoq_listener());
    }

    #[test]
    fn needs_stoq_listener_true_for_internetwork() {
        let cfg = ModeConfig {
            enabled_modes: vec![GatewayMode::InterNetwork],
            ..ModeConfig::default()
        };
        assert!(cfg.needs_stoq_listener());
    }

    #[test]
    fn is_mode_enabled_returns_false_for_absent() {
        let cfg = ModeConfig {
            enabled_modes: vec![GatewayMode::Bootstrap],
            ..ModeConfig::default()
        };
        assert!(!cfg.is_mode_enabled(GatewayMode::InboundProxy));
    }

    #[test]
    fn backend_config_roundtrip_serde() {
        let bc = BackendConfig {
            name: "trustchain".into(),
            addr: "[::1]:50053".parse().expect("test: valid addr"),
            server_name: "trustchain".into(),
            protocol: BackendProtocol::Http3,
            path_prefixes: vec!["/api/v1/trustchain".into()],
            domains: vec!["trust.hypermesh.online".into()],
        };
        let json = serde_json::to_string(&bc).expect("test: serialize");
        let parsed: BackendConfig = serde_json::from_str(&json).expect("test: deserialize");
        assert_eq!(parsed.name, "trustchain");
        assert_eq!(parsed.protocol, BackendProtocol::Http3);
        assert_eq!(parsed.path_prefixes.len(), 1);
    }

    #[test]
    fn gateway_mode_serde_roundtrip() {
        let mode = GatewayMode::InterNetwork;
        let json = serde_json::to_string(&mode).expect("test: serialize");
        let parsed: GatewayMode = serde_json::from_str(&json).expect("test: deserialize");
        assert_eq!(parsed, GatewayMode::InterNetwork);
    }
}
