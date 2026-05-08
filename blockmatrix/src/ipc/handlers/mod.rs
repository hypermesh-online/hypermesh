// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! IPC handler modules for daemon RPC methods.
//!
//! Each submodule registers a group of related methods. The [`register_all`]
//! function wires every handler into a [`RequestHandler`].

pub mod asset;
pub mod auth;
pub mod blockchain;
pub mod caesar;
pub mod capability_registry;
pub mod config;
pub mod dashboard;
pub mod dns;
pub mod engauge;
pub mod gateway;
pub mod intelligence;
pub mod domain;
pub mod message;
pub mod network;
pub mod share;
pub mod shard;
pub mod stoq;
pub mod store;
pub mod system;
pub mod topology;
pub mod trustchain;

use std::sync::Arc;

use crate::ipc::handler::RequestHandler;
use crate::ipc::state::DaemonState;

/// Register all built-in IPC handlers (ping, shutdown, status, and domain
/// modules) against the provided [`RequestHandler`].
pub fn register_all(handler: &mut RequestHandler, state: Arc<DaemonState>) {
    // --- core: ping / shutdown / status ---
    handler.register(
        "ping",
        Arc::new(|_params| Box::pin(async { Ok(serde_json::json!("pong")) })),
    );

    {
        let s = state.clone();
        handler.register(
            "shutdown",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move {
                    let _ = s.shutdown_tx.send(true);
                    Ok(serde_json::json!({"status": "shutting_down"}))
                })
            }),
        );
    }

    {
        let s = state.clone();
        handler.register(
            "status",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move {
                    let height = s.blockchain.get_height().await;
                    let peer_count = match &s.network {
                        Some(net) => net.get_connected_nodes().await.len(),
                        None => 0,
                    };
                    let uptime = s.started_at.elapsed().as_secs();
                    Ok(serde_json::json!({
                        "node_id": s.node_id,
                        "coordinate": {
                            "x": s.coordinate.x,
                            "y": s.coordinate.y,
                            "z": s.coordinate.z,
                        },
                        "chain_height": height,
                        "privacy_mode": s.privacy_mode,
                        "peers": peer_count,
                        "uptime_secs": uptime,
                    }))
                })
            }),
        );
    }

    // --- domain modules ---
    dns::register(handler, &state);
    domain::register(handler, &state);
    blockchain::register(handler, &state);
    network::register(handler, &state);
    topology::register(handler, &state);
    asset::register(handler, &state);
    dashboard::register(handler, &state);
    shard::register(handler, &state);
    store::register(handler, &state);
    gateway::register(handler, &state);
    share::register(handler, &state);
    message::register(handler, &state);
    intelligence::register(handler, &state);
    caesar::register(handler, &state);
    engauge::register(handler, &state);
    trustchain::register(handler, &state);
    stoq::register(handler, &state);
    system::register(handler, &state);
    auth::register(handler, &state);
    config::register(handler);

    // Phase K.2 — install capability enforcement when the daemon was
    // started with an issuer. Alpha-default inert: when no issuer is
    // configured, the handler dispatches with no token check (preserves
    // pre-K.2 localhost-only IPC behavior).
    if let Some(issuer) = state.capability_token_issuer.as_ref() {
        let ctx = crate::ipc::handler::CapabilityContext::new(
            issuer.as_ref(),
            state.revocation_registry.clone(),
        );
        handler.set_capability_context(ctx);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::blockchain::node_chain::NodeBlockchain;
    use crate::ipc::protocol::RpcRequest;
    use crate::matrix::coordinate::MatrixCoordinate;
    use crate::network::shard_store::ShardStore;
    use crate::persistence::{PersistenceConfig, PersistenceManager};
    use std::path::PathBuf;
    use std::time::Instant;

    /// Build a minimal DaemonState for testing.
    pub(crate) async fn test_state() -> Arc<DaemonState> {
        let coord =
            MatrixCoordinate::new(0, 0, 0).expect("test: valid coord");
        let blockchain = Arc::new(NodeBlockchain::new(coord));
        let config = PersistenceConfig {
            storage_dir: PathBuf::from("/tmp"),
            ..PersistenceConfig::default()
        };
        let persistence = Arc::new(
            PersistenceManager::new(config, "test-node".to_string())
                .await
                .expect("test: create persistence"),
        );
        let (shutdown_tx, _rx) = tokio::sync::watch::channel(false);
        let dns = crate::bootstrap::DnsResolver::default();

        Arc::new(DaemonState {
            blockchain,
            persistence,
            network: None,
            shard_store: Arc::new(ShardStore::new()),
            shard_transport: None,
            coordinate: coord,
            node_id: "test-node".to_string(),
            data_dir: PathBuf::from("/tmp"),
            privacy_mode: "Private".to_string(),
            started_at: Instant::now(),
            shutdown_tx,
            dns_resolver: dns,
            dns_popularity_tracker: None,
            shard_location_index: None,
            consumer_provider_manager: None,
            #[cfg(feature = "caesar")]
            caesar: None,
            #[cfg(feature = "intelligence")]
            engauge_bridge: None,
            #[cfg(feature = "intelligence")]
            federation_manager: None,
            #[cfg(feature = "intelligence")]
            threshold_coordinator: None,
            transfer_coordinator: None,
            foundation_signing_key: None,
            dns_registrar: None,
            release_feed_subscriber: None,
            receipt_validator: Arc::new(
                crate::assets::cross_chain::CrossChainReceiptValidator::new(),
            ),
            capability_token_issuer: None,
            revocation_registry: Arc::new(crate::auth::RevocationRegistry::new()),
            light_sync_manager: None,
        })
    }

    #[tokio::test]
    async fn test_register_all_ping() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register_all(&mut handler, state);

        let req = RpcRequest::new("ping", serde_json::json!(null));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        assert_eq!(
            resp.result.expect("test: result present"),
            serde_json::json!("pong"),
        );
    }

    #[tokio::test]
    async fn test_register_all_status() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register_all(&mut handler, state);

        let req = RpcRequest::new("status", serde_json::json!(null));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result present");
        assert_eq!(result["node_id"], "test-node");
    }

    #[tokio::test]
    async fn test_register_all_has_domain_methods() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register_all(&mut handler, state);

        // Verify domain handlers are registered by calling them
        for method in &[
            "dns.resolve",
            "dns.list",
            "domain.register",
            "domain.list",
            "domain.join",
            "blockchain.height",
            "blockchain.block",
            "blockchain.validate",
            "network.peers",
            "topology.info",
            "topology.neighbors",
            "topology.routing_cost",
            "topology.path",
            "asset.list",
            "dashboard.deploy",
            "dashboard.list",
            "dashboard.info",
            "gateway.transfer",
            "gateway.status",
            "gateway.list",
            "gateway.initiate_transfer",
            "share.send",
            "share.inbox",
            "share.accept",
            "share.reject",
            "message.send",
            "message.inbox",
            "message.history",
            "message.read",
            "identity.pubkey",
            "identity.rotate",
            "peer.pubkey",
            "intelligence.stats",
            "caesar.overview",
            "caesar.balance",
            "caesar.transactions",
            "caesar.rewards",
            "caesar.staking",
            "engauge.capacity",
            "engauge.traffic",
            "engauge.throttle",
            "engauge.routing",
            "trustchain.status",
            "trustchain.certs",
            "trustchain.identity",
            "trustchain.federation",
            "trustchain.request_cert",
            "stoq.stats",
            "stoq.connections",
            "stoq.performance",
            "config.show",
            "config.get",
            "config.set",
            "auth.create_session",
            "auth.list_sessions",
            "auth.revoke_session",
        ] {
            let req = RpcRequest::new(method, serde_json::json!({}));
            let resp = handler.dispatch(req).await;
            // Should NOT be "method not found" -- handler exists
            if let Some(ref err) = resp.error {
                assert_ne!(
                    err.code,
                    crate::ipc::protocol::METHOD_NOT_FOUND,
                    "method {method} should be registered",
                );
            }
        }
    }
}
