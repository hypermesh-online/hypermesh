// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! DNS IPC handlers: resolve, list, register, foundation_grant (H.1).

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::dns::FoundationGrant;
use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::{INTERNAL_ERROR, INVALID_PARAMS, RpcError};
use crate::ipc::state::DaemonState;
use hypermesh_lib::NodeSigner;

/// Register DNS-related IPC methods.
pub fn register(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    // dns.resolve — resolve a single name
    {
        let s = state.clone();
        handler.register(
            "dns.resolve",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move {
                    let name = params
                        .get("name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| crate::ipc::protocol::RpcError {
                            code: INVALID_PARAMS,
                            message: "missing 'name' parameter".into(),
                            data: None,
                        })?;

                    match s.dns_resolver.resolve(name).await {
                        Some(addr) => Ok(serde_json::json!({
                            "name": name,
                            "address": addr.to_string(),
                        })),
                        None => Ok(serde_json::json!({
                            "name": name,
                            "address": null,
                            "error": "not found",
                        })),
                    }
                })
            }),
        );
    }

    // dns.list — list all registered DNS records
    {
        let s = state.clone();
        handler.register(
            "dns.list",
            Arc::new(move |_params| {
                let s = s.clone();
                Box::pin(async move {
                    let records = s.dns_resolver.all_records().await;
                    let entries: Vec<serde_json::Value> = records
                        .into_iter()
                        .map(|(name, addr)| {
                            serde_json::json!({"name": name, "address": addr.to_string()})
                        })
                        .collect();
                    Ok(serde_json::json!({"records": entries, "count": entries.len()}))
                })
            }),
        );
    }

    // dns.reserved — diagnostic: list foundation-reserved domains
    {
        handler.register(
            "dns.reserved",
            Arc::new(move |_params| {
                Box::pin(async move {
                    let list = crate::dns::reserved_list();
                    let count = list.len();
                    Ok(serde_json::json!({
                        "domains": list,
                        "count": count,
                    }))
                })
            }),
        );
    }

    // dns.foundation_grant — admin-only: sign a reserved-domain grant
    //
    // Args:
    //   domain: string                  — reserved domain to authorize
    //   recipient_pubkey: string (hex)  — recipient FALCON-1024 pubkey
    //   valid_until_secs: u64           — Unix timestamp grant expires
    //   dues_paid_until_secs: u64       — Unix timestamp dues lapse
    //
    // Alpha-default inert: when `state.foundation_signing_key` is None
    // returns "foundation root key not configured". Operators opt-in by
    // populating that field at daemon startup with the foundation root
    // identity.
    {
        let s = state.clone();
        handler.register(
            "dns.foundation_grant",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move {
                    let signing_key = match s.foundation_signing_key.clone() {
                        Some(k) => k,
                        None => {
                            return Err(RpcError {
                                code: INTERNAL_ERROR,
                                message:
                                    "foundation root key not configured (alpha-default inert)"
                                        .into(),
                                data: None,
                            });
                        }
                    };

                    let domain = params
                        .get("domain")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| RpcError {
                            code: INVALID_PARAMS,
                            message: "missing 'domain' parameter".into(),
                            data: None,
                        })?
                        .to_string();

                    if !crate::dns::reserved::is_reserved(&domain) {
                        return Err(RpcError {
                            code: INVALID_PARAMS,
                            message: format!(
                                "domain '{}' is not on the foundation reserved list",
                                domain
                            ),
                            data: None,
                        });
                    }

                    let recipient_hex = params
                        .get("recipient_pubkey")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| RpcError {
                            code: INVALID_PARAMS,
                            message: "missing 'recipient_pubkey' parameter".into(),
                            data: None,
                        })?;
                    let recipient_pubkey = hex::decode(recipient_hex).map_err(|e| RpcError {
                        code: INVALID_PARAMS,
                        message: format!("recipient_pubkey hex decode failed: {e}"),
                        data: None,
                    })?;

                    let valid_until_secs = params
                        .get("valid_until_secs")
                        .and_then(|v| v.as_u64())
                        .ok_or_else(|| RpcError {
                            code: INVALID_PARAMS,
                            message: "missing 'valid_until_secs' parameter".into(),
                            data: None,
                        })?;
                    let valid_until = UNIX_EPOCH + Duration::from_secs(valid_until_secs);

                    let dues_paid_until_secs = params
                        .get("dues_paid_until_secs")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(valid_until_secs);
                    let dues_paid_until =
                        UNIX_EPOCH + Duration::from_secs(dues_paid_until_secs);

                    // Build unsigned grant, sign canonical payload.
                    let mut grant = FoundationGrant::new_unsigned(
                        domain.clone(),
                        recipient_pubkey,
                        valid_until,
                        dues_paid_until,
                    );
                    let payload = grant.signing_payload();
                    let sig = signing_key.sign(&payload).map_err(|e| RpcError {
                        code: INTERNAL_ERROR,
                        message: format!("FALCON sign failed: {e}"),
                        data: None,
                    })?;
                    grant.foundation_signature = sig;

                    // Note: H.1 records the grant as a Catalog asset
                    // (`foundation.dns_grant/v1`). The full Catalog wire
                    // path is gated behind the `caesar` build of the
                    // catalog crate; for alpha we return the serialized
                    // grant + verifier metadata so the operator can
                    // distribute the grant out-of-band and audit later.
                    let grant_json =
                        serde_json::to_value(&grant).map_err(|e| RpcError {
                            code: INTERNAL_ERROR,
                            message: format!("serialize grant: {e}"),
                            data: None,
                        })?;
                    let issued_at_secs = grant
                        .issued_at
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);

                    Ok(serde_json::json!({
                        "grant": grant_json,
                        "domain": domain,
                        "issued_at_secs": issued_at_secs,
                        "foundation_pubkey": hex::encode(&signing_key.public_key),
                        "typedef": "foundation.dns_grant/v1",
                    }))
                })
            }),
        );
    }
    // Suppress unused-import warning when the only use of `SystemTime`
    // is in tests below.
    let _ = SystemTime::now;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::node_chain::NodeBlockchain;
    use crate::bootstrap::DnsResolver;
    use crate::ipc::protocol::RpcRequest;
    use crate::matrix::coordinate::MatrixCoordinate;
    use crate::persistence::{PersistenceConfig, PersistenceManager};
    use std::path::PathBuf;
    use std::time::Instant;

    async fn test_state() -> Arc<DaemonState> {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let bc = Arc::new(NodeBlockchain::new(coord));
        let config = PersistenceConfig {
            storage_dir: PathBuf::from("/tmp"),
            ..PersistenceConfig::default()
        };
        let persistence = Arc::new(
            PersistenceManager::new(config, "dns-test".into())
                .await
                .expect("test: persistence"),
        );
        let (shutdown_tx, _rx) = tokio::sync::watch::channel(false);
        let dns = DnsResolver::default();

        Arc::new(DaemonState {
            blockchain: bc,
            persistence,
            network: None,
            shard_store: Arc::new(crate::network::shard_store::ShardStore::new()),
            shard_transport: None,
            coordinate: coord,
            node_id: "dns-test".into(),
            network_id: "trustnet-test".into(),
            data_dir: PathBuf::from("/tmp"),
            privacy_mode: "Private".into(),
            started_at: Instant::now(),
            shutdown_tx,
            dns_resolver: dns,
            shard_location_index: None,
            consumer_provider_manager: None,
            #[cfg(feature = "caesar")]
            caesar: None,
            #[cfg(feature = "intelligence")]
            ngauge_bridge: None,
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
            catalog_registry: None,
            inbox_store: None,
        })
    }

    #[tokio::test]
    async fn test_dns_resolve_not_found() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "dns.resolve",
            serde_json::json!({"name": "nonexistent"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert!(result["address"].is_null());
    }

    #[tokio::test]
    async fn test_dns_resolve_found() {
        let state = test_state().await;
        // Pre-register a record
        state
            .dns_resolver
            .register(
                "mynode".into(),
                std::net::IpAddr::from(std::net::Ipv6Addr::LOCALHOST),
            )
            .await;

        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "dns.resolve",
            serde_json::json!({"name": "mynode"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["address"], "::1");
    }

    #[tokio::test]
    async fn test_dns_list_empty() {
        let state = test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        // The default DnsResolver has built-in records (localhost, trustchain, etc.)
        let req = RpcRequest::new("dns.list", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert!(result["count"].is_number());
    }
}
