// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Catalog IPC handlers — typedef dependency resolution.
//!
//! Phase M.4.5b: surfaces real dependency graphs from the catalog
//! registry when it has been wired into the daemon. The catalog crate
//! itself depends on blockmatrix (so blockmatrix cannot depend on
//! catalog without forming a cycle); the daemon therefore holds the
//! registry as `Arc<dyn crate::catalog::CatalogProvider>` and an
//! adapter in the catalog crate (or a downstream binary) provides the
//! impl. Until then, the field stays `None` and this handler returns
//! an honest "not wired" response.

use std::sync::Arc;

use crate::catalog::{
    CatalogDependencyNode, CatalogPeerClient, CatalogProvider, CatalogTypeInfo,
};
use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::{RpcError, INTERNAL_ERROR, INVALID_PARAMS};
use crate::ipc::state::DaemonState;
use hypermesh_lib::ContentHash;

fn invalid_params(msg: impl Into<String>) -> RpcError {
    RpcError {
        code: INVALID_PARAMS,
        message: msg.into(),
        data: None,
    }
}

fn internal_error(msg: impl Into<String>) -> RpcError {
    RpcError {
        code: INTERNAL_ERROR,
        message: msg.into(),
        data: None,
    }
}

/// Register catalog-related IPC methods.
pub fn register(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    {
        let s = state.clone();
        handler.register(
            "catalog.dependencies",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move { handle_dependencies(&s, params).await })
            }),
        );
    }
    {
        let s = state.clone();
        handler.register(
            "catalog.search",
            Arc::new(move |params| {
                let s = s.clone();
                Box::pin(async move { handle_search(&s, params).await })
            }),
        );
    }
}

/// Empty alpha-inert response shape. Kept as a helper so both the
/// "registry not wired" branch and the "type_name not found" branch
/// return a consistent envelope.
fn empty_response(status: &str, note: Option<String>) -> serde_json::Value {
    let mut body = serde_json::json!({
        "status": status,
        "direct_deps": [],
        "transitive_deps": [],
        "missing": [],
        "total": 0,
    });
    if let Some(n) = note {
        body["note"] = serde_json::Value::String(n);
    }
    body
}

/// `catalog.dependencies` IPC handler.
///
/// Accepts either:
/// - `{ "type_hash": "<hex BLAKE3>" }` — canonical, resolves directly,
/// - `{ "type_name": "<name>" }`       — convenience, resolves the
///   name to a hash first via [`CatalogProvider::lookup_by_name`].
///
/// Response shape (success):
/// ```json
/// {
///   "status": "ok",
///   "type_hash": "<hex>",
///   "direct_deps":     [ { "type_hash", "name", "version", "depth" }, ... ],
///   "transitive_deps": [ ... ],
///   "missing":         [ "<hex>", ... ],
///   "total": <usize>
/// }
/// ```
///
/// When the registry is not wired (alpha-default), returns:
/// ```json
/// {
///   "status": "alpha",
///   "note": "catalog registry not wired",
///   "direct_deps": [], "transitive_deps": [], "missing": [], "total": 0
/// }
/// ```
async fn handle_dependencies(
    state: &DaemonState,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let registry = match state.catalog_registry.as_ref() {
        Some(r) => r.clone(),
        None => {
            return Ok(empty_response(
                "alpha",
                Some("catalog registry not wired".to_string()),
            ));
        }
    };

    // Resolve type_hash from params (either directly, or via type_name lookup).
    let type_hash = match resolve_type_hash(&params, registry.as_ref()).await? {
        ResolvedHash::Hash(h) => h,
        ResolvedHash::NameNotFound(name) => {
            return Ok(empty_response(
                "ok",
                Some(format!("type not found: {name}")),
            ));
        }
    };

    let graph = registry
        .resolve_dependencies(&type_hash)
        .await
        .map_err(|e| internal_error(format!("resolve failed: {e}")))?;

    let direct: Vec<serde_json::Value> = graph.direct.iter().map(node_to_json).collect();
    let transitive: Vec<serde_json::Value> =
        graph.transitive.iter().map(node_to_json).collect();
    let missing: Vec<String> = graph
        .missing
        .iter()
        .map(|h| hex::encode(h.as_bytes()))
        .collect();

    Ok(serde_json::json!({
        "status": "ok",
        "type_hash": hex::encode(type_hash.as_bytes()),
        "direct_deps": direct,
        "transitive_deps": transitive,
        "missing": missing,
        "total": graph.direct.len() + graph.transitive.len(),
    }))
}

enum ResolvedHash {
    Hash(ContentHash),
    NameNotFound(String),
}

/// Parse `{type_hash}` (preferred) or `{type_name}` (convenience) from
/// the request and return the canonical hash. Errors with
/// `INVALID_PARAMS` only when neither is provided or the hex is
/// malformed; an unknown name returns [`ResolvedHash::NameNotFound`]
/// so the caller can render an honest empty response with a note.
async fn resolve_type_hash(
    params: &serde_json::Value,
    registry: &dyn CatalogProvider,
) -> Result<ResolvedHash, RpcError> {
    if let Some(hex) = params.get("type_hash").and_then(|v| v.as_str()) {
        return parse_content_hash_hex(hex).map(ResolvedHash::Hash);
    }
    if let Some(name) = params.get("type_name").and_then(|v| v.as_str()) {
        let info = registry
            .lookup_by_name(name)
            .await
            .map_err(|e| internal_error(format!("lookup failed: {e}")))?;
        return match info {
            Some(i) => Ok(ResolvedHash::Hash(i.type_hash)),
            None => Ok(ResolvedHash::NameNotFound(name.to_string())),
        };
    }
    Err(invalid_params(
        "either 'type_hash' (hex) or 'type_name' (string) required",
    ))
}

/// Parse a 64-char hex string into a [`ContentHash`]. Rejects any
/// other length with `INVALID_PARAMS` to keep the wire contract strict.
fn parse_content_hash_hex(hex_str: &str) -> Result<ContentHash, RpcError> {
    if hex_str.len() != 64 {
        return Err(invalid_params(format!(
            "type_hash must be 64 hex chars (BLAKE3-256), got {}",
            hex_str.len()
        )));
    }
    let bytes = hex::decode(hex_str)
        .map_err(|e| invalid_params(format!("invalid hex in type_hash: {e}")))?;
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(ContentHash::from_bytes(arr))
}

fn node_to_json(n: &CatalogDependencyNode) -> serde_json::Value {
    serde_json::json!({
        "type_hash": hex::encode(n.type_hash.as_bytes()),
        "name": n.name,
        "version": n.version,
        "depth": n.depth,
    })
}

/// Render a [`CatalogTypeInfo`] as the wire match shape used by
/// `catalog.search`. The `source` field is supplied by the caller —
/// `"local"` for local-registry hits, `"neighbor:<node_id>"` for hits
/// pulled from a peer's `catalog/search` STOQ endpoint.
fn type_info_to_match(info: &CatalogTypeInfo, source: &str) -> serde_json::Value {
    serde_json::json!({
        "type_hash": hex::encode(info.type_hash.as_bytes()),
        "name": info.type_name,
        "version": info.version,
        "source": source,
    })
}

/// `catalog.search` IPC handler.
///
/// Parameters:
/// - `query` (required, string): substring/exact match against typedef
///   names.
/// - `recursive` (optional, bool, default `false`): when true, fan out
///   the same query to up to `max_neighbors` connected peers via
///   [`CatalogPeerClient`].
/// - `max_neighbors` (optional, u64, default `8`): cap on peer fan-out.
/// - `timeout_ms` (optional, u64, default `2000`): per-peer timeout
///   for recursive fan-out.
/// - `limit` (optional, u64, default `50`): max local matches before
///   considering peers.
///
/// Response shape (success):
/// ```json
/// {
///   "status": "ok",
///   "query": "<echoed>",
///   "matches": [
///     { "type_hash": "<hex>", "name": "...", "version": "...",
///       "source": "local" | "neighbor:<node_id>" },
///     ...
///   ],
///   "total": <usize>,
///   "neighbors_queried": <usize>,
///   "neighbor_errors": [{ "node_id": "...", "error": "..." }]
/// }
/// ```
///
/// When the catalog registry is not wired, returns
/// `{ "status": "alpha", "note": "catalog registry not wired", ... }`
/// — never a fabricated result list.
async fn handle_search(
    state: &DaemonState,
    params: serde_json::Value,
) -> Result<serde_json::Value, RpcError> {
    let query = params
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| invalid_params("'query' (string) required"))?
        .to_string();

    let recursive = params
        .get("recursive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let max_neighbors = params
        .get("max_neighbors")
        .and_then(|v| v.as_u64())
        .unwrap_or(8) as usize;
    let timeout_ms = params
        .get("timeout_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(2000);
    let limit = params
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(50) as usize;

    // --- Branch 1: registry not wired -> alpha-honest empty response.
    let registry = match state.catalog_registry.as_ref() {
        Some(r) => r.clone(),
        None => {
            return Ok(serde_json::json!({
                "status": "alpha",
                "note": "catalog registry not wired",
                "query": query,
                "matches": [],
                "total": 0,
                "neighbors_queried": 0,
                "neighbor_errors": [],
            }));
        }
    };

    // --- Step 1: local search.
    let local_results = registry
        .search_local(&query, limit)
        .await
        .map_err(|e| internal_error(format!("local search failed: {e}")))?;

    let mut matches: Vec<serde_json::Value> = local_results
        .iter()
        .map(|info| type_info_to_match(info, "local"))
        .collect();
    let mut seen_hashes: std::collections::HashSet<String> = matches
        .iter()
        .filter_map(|m| m["type_hash"].as_str().map(str::to_owned))
        .collect();

    // --- Step 2: optional recursive fan-out.
    let mut neighbors_queried: usize = 0;
    let mut neighbor_errors: Vec<serde_json::Value> = Vec::new();
    if recursive {
        match state.network.as_ref() {
            Some(network) => {
                let client = CatalogPeerClient::new(network.clone());
                let peer_results = client
                    .search_neighbors(&query, max_neighbors, timeout_ms)
                    .await;
                neighbors_queried = peer_results.len();
                for (node_id, result) in peer_results {
                    match result {
                        Ok(rows) => {
                            for row in rows {
                                if row.type_hash.is_empty() {
                                    // Skip rows that lack a canonical
                                    // hash — we never fabricate one.
                                    continue;
                                }
                                if !seen_hashes.insert(row.type_hash.clone()) {
                                    continue;
                                }
                                matches.push(serde_json::json!({
                                    "type_hash": row.type_hash,
                                    "name": row.name,
                                    "version": row.version,
                                    "source": format!("neighbor:{node_id}"),
                                }));
                            }
                        }
                        Err(e) => {
                            neighbor_errors.push(serde_json::json!({
                                "node_id": node_id,
                                "error": e.to_string(),
                            }));
                        }
                    }
                }
            }
            None => {
                // recursive=true but no network manager wired. Surface
                // honestly so the caller can distinguish "asked for
                // recursive but got local-only" from "asked for local
                // only".
                return Ok(serde_json::json!({
                    "status": "ok",
                    "note": "recursive requested but no network manager wired; returning local-only",
                    "query": query,
                    "matches": matches,
                    "total": matches.len(),
                    "neighbors_queried": 0,
                    "neighbor_errors": [],
                }));
            }
        }
    }

    let total = matches.len();
    Ok(serde_json::json!({
        "status": "ok",
        "query": query,
        "matches": matches,
        "total": total,
        "neighbors_queried": neighbors_queried,
        "neighbor_errors": neighbor_errors,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::provider::test_support::{FakeCatalogProvider, FakeTypedef};
    use crate::catalog::{CatalogProvider, CatalogTypeInfo};
    use crate::ipc::protocol::RpcRequest;

    fn hash_from(label: &[u8]) -> ContentHash {
        ContentHash::from_bytes(*blake3::hash(label).as_bytes())
    }

    fn typedef(name: &str, hash: ContentHash, deps: Vec<ContentHash>) -> FakeTypedef {
        FakeTypedef {
            info: CatalogTypeInfo {
                type_name: name.to_string(),
                type_hash: hash,
                version: "1.0.0".to_string(),
            },
            dependencies: deps,
        }
    }

    /// Build a [`DaemonState`] with a fake catalog provider already
    /// wired in. We mutate the `catalog_registry` field after building
    /// via the shared `tests::test_state()` helper so we exercise the
    /// real "Some(provider)" branch.
    async fn state_with_provider(
        provider: Arc<dyn CatalogProvider>,
    ) -> Arc<DaemonState> {
        let base = super::super::tests::test_state().await;
        // Re-build with the provider field populated. We cannot mutate
        // through `Arc<DaemonState>`, so reconstruct.
        let coord = base.coordinate;
        let blockchain = base.blockchain.clone();
        let persistence = base.persistence.clone();
        let (shutdown_tx, _rx) = tokio::sync::watch::channel(false);
        Arc::new(DaemonState {
            blockchain,
            persistence,
            network: None,
            shard_store: base.shard_store.clone(),
            shard_transport: None,
            coordinate: coord,
            node_id: base.node_id.clone(),
            data_dir: base.data_dir.clone(),
            privacy_mode: base.privacy_mode.clone(),
            started_at: base.started_at,
            shutdown_tx,
            dns_resolver: base.dns_resolver.clone(),
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
            catalog_registry: Some(provider),
        })
    }

    #[tokio::test]
    async fn test_catalog_dependencies_no_registry_returns_alpha() {
        // test_state() leaves catalog_registry = None.
        let state = super::super::tests::test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "catalog.dependencies",
            serde_json::json!({"type_name": "Message"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");
        assert_eq!(result["status"], "alpha");
        assert!(
            result["note"]
                .as_str()
                .expect("test: note str")
                .contains("not wired"),
            "note should explain that registry is not wired: {:?}",
            result["note"]
        );
        assert_eq!(result["total"], 0);
        assert!(result["direct_deps"].as_array().expect("array").is_empty());
        assert!(
            result["transitive_deps"].as_array().expect("array").is_empty()
        );
        assert!(result["missing"].as_array().expect("array").is_empty());
    }

    #[tokio::test]
    async fn test_catalog_dependencies_unknown_type_name() {
        let provider = Arc::new(FakeCatalogProvider::new());
        let state = state_with_provider(provider).await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "catalog.dependencies",
            serde_json::json!({"type_name": "Nonexistent"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");
        assert_eq!(result["status"], "ok");
        assert!(result["note"]
            .as_str()
            .expect("test: note str")
            .contains("type not found"));
        assert_eq!(result["total"], 0);
    }

    #[tokio::test]
    async fn test_catalog_dependencies_real_graph_by_hash() {
        // Build A -> B -> C and query A by content hash. Expect direct=[B],
        // transitive=[C], missing=[], total=2.
        let provider = Arc::new(FakeCatalogProvider::new());
        let c_hash = hash_from(b"C-leaf");
        let b_hash = hash_from(b"B-mid");
        let a_hash = hash_from(b"A-root");
        provider.insert(typedef("C", c_hash, vec![])).await;
        provider.insert(typedef("B", b_hash, vec![c_hash])).await;
        provider.insert(typedef("A", a_hash, vec![b_hash])).await;

        let state = state_with_provider(provider).await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "catalog.dependencies",
            serde_json::json!({"type_hash": hex::encode(a_hash.as_bytes())}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");

        assert_eq!(result["status"], "ok");
        assert_eq!(
            result["type_hash"].as_str().expect("hex"),
            hex::encode(a_hash.as_bytes())
        );
        assert_eq!(result["total"], 2);

        let direct = result["direct_deps"].as_array().expect("direct array");
        assert_eq!(direct.len(), 1, "exactly one direct dep");
        assert_eq!(direct[0]["name"], "B");
        assert_eq!(direct[0]["depth"], 1);
        assert_eq!(
            direct[0]["type_hash"].as_str().expect("hash"),
            hex::encode(b_hash.as_bytes())
        );

        let trans = result["transitive_deps"]
            .as_array()
            .expect("transitive array");
        assert_eq!(trans.len(), 1, "exactly one transitive dep");
        assert_eq!(trans[0]["name"], "C");
        assert_eq!(trans[0]["depth"], 2);

        let missing = result["missing"].as_array().expect("missing array");
        assert!(missing.is_empty());
    }

    #[tokio::test]
    async fn test_catalog_dependencies_real_graph_by_name() {
        let provider = Arc::new(FakeCatalogProvider::new());
        let b_hash = hash_from(b"name-b");
        let a_hash = hash_from(b"name-a");
        provider.insert(typedef("B", b_hash, vec![])).await;
        provider.insert(typedef("A", a_hash, vec![b_hash])).await;

        let state = state_with_provider(provider).await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "catalog.dependencies",
            serde_json::json!({"type_name": "A"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");

        assert_eq!(result["status"], "ok");
        assert_eq!(result["total"], 1);
        assert_eq!(
            result["type_hash"].as_str().expect("hash"),
            hex::encode(a_hash.as_bytes())
        );
        let direct = result["direct_deps"].as_array().expect("direct array");
        assert_eq!(direct.len(), 1);
        assert_eq!(direct[0]["name"], "B");
    }

    #[tokio::test]
    async fn test_catalog_dependencies_missing_surfaces_honestly() {
        // A declares a dep on a phantom hash that has no typedef.
        let provider = Arc::new(FakeCatalogProvider::new());
        let phantom = hash_from(b"phantom-dep");
        let a_hash = hash_from(b"A-missing-dep");
        provider.insert(typedef("A", a_hash, vec![phantom])).await;

        let state = state_with_provider(provider).await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "catalog.dependencies",
            serde_json::json!({"type_hash": hex::encode(a_hash.as_bytes())}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result present");

        assert_eq!(result["status"], "ok");
        assert_eq!(result["total"], 0, "phantom counted only as missing");
        let missing = result["missing"].as_array().expect("missing array");
        assert_eq!(missing.len(), 1);
        assert_eq!(
            missing[0].as_str().expect("hex"),
            hex::encode(phantom.as_bytes())
        );
    }

    #[tokio::test]
    async fn test_catalog_dependencies_invalid_params() {
        // Wire a real provider so the handler reaches param parsing.
        let provider = Arc::new(FakeCatalogProvider::new());
        let state = state_with_provider(provider).await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("catalog.dependencies", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some(), "expected INVALID_PARAMS error");
        let err = resp.error.expect("test: error present");
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(
            err.message.contains("type_hash") && err.message.contains("type_name"),
            "error must mention both alternatives: {}",
            err.message,
        );
    }

    #[tokio::test]
    async fn test_catalog_dependencies_invalid_hex_length() {
        let provider = Arc::new(FakeCatalogProvider::new());
        let state = state_with_provider(provider).await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "catalog.dependencies",
            serde_json::json!({"type_hash": "deadbeef"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some());
        let err = resp.error.expect("test: error present");
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(err.message.contains("64 hex chars"));
    }

    #[tokio::test]
    async fn test_catalog_dependencies_invalid_hex_chars() {
        let provider = Arc::new(FakeCatalogProvider::new());
        let state = state_with_provider(provider).await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        // 64 chars but contains non-hex.
        let bad = "z".repeat(64);
        let req = RpcRequest::new(
            "catalog.dependencies",
            serde_json::json!({ "type_hash": bad }),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some());
        let err = resp.error.expect("test: error present");
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(err.message.contains("invalid hex"));
    }

    // =====================================================================
    // catalog.search — M.4.5c.2
    // =====================================================================

    #[tokio::test]
    async fn test_catalog_search_no_registry_returns_alpha() {
        let state = super::super::tests::test_state().await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "catalog.search",
            serde_json::json!({"query": "Message"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result");
        assert_eq!(result["status"], "alpha");
        assert!(result["note"]
            .as_str()
            .expect("test: note str")
            .contains("not wired"));
        assert_eq!(result["query"], "Message");
        assert_eq!(result["total"], 0);
        assert!(result["matches"]
            .as_array()
            .expect("matches array")
            .is_empty());
        assert_eq!(result["neighbors_queried"], 0);
    }

    #[tokio::test]
    async fn test_catalog_search_local_only_no_recursive() {
        // FakeCatalogProvider with three typedefs; query "Inv" matches
        // exactly one and the response carries source="local".
        let provider = Arc::new(FakeCatalogProvider::new());
        let msg_h = hash_from(b"msg-typedef");
        let inv_h = hash_from(b"inv-typedef");
        let doc_h = hash_from(b"doc-typedef");
        provider.insert(typedef("Message", msg_h, vec![])).await;
        provider.insert(typedef("Invitation", inv_h, vec![])).await;
        provider.insert(typedef("Document", doc_h, vec![])).await;

        let state = state_with_provider(provider).await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "catalog.search",
            serde_json::json!({"query": "Inv", "recursive": false}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result");
        assert_eq!(result["status"], "ok");
        assert_eq!(result["query"], "Inv");
        assert_eq!(result["neighbors_queried"], 0);

        let matches = result["matches"].as_array().expect("matches array");
        assert_eq!(matches.len(), 1, "exactly one local match");
        assert_eq!(matches[0]["name"], "Invitation");
        assert_eq!(matches[0]["source"], "local");
        assert_eq!(
            matches[0]["type_hash"].as_str().expect("hash"),
            hex::encode(inv_h.as_bytes()),
        );
    }

    #[tokio::test]
    async fn test_catalog_search_local_browse_all_empty_query() {
        let provider = Arc::new(FakeCatalogProvider::new());
        let a_h = hash_from(b"AAA");
        let b_h = hash_from(b"BBB");
        provider.insert(typedef("Alpha", a_h, vec![])).await;
        provider.insert(typedef("Beta", b_h, vec![])).await;

        let state = state_with_provider(provider).await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "catalog.search",
            serde_json::json!({"query": ""}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result");
        assert_eq!(result["status"], "ok");
        let matches = result["matches"].as_array().expect("matches array");
        assert_eq!(matches.len(), 2, "empty query browses all");
        for m in matches {
            assert_eq!(m["source"], "local");
        }
    }

    #[tokio::test]
    async fn test_catalog_search_recursive_no_network() {
        // recursive=true but state.network is None: handler returns
        // local-only matches with an honest note explaining the situation.
        let provider = Arc::new(FakeCatalogProvider::new());
        let m_h = hash_from(b"msg-recursive");
        provider.insert(typedef("Message", m_h, vec![])).await;
        let state = state_with_provider(provider).await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "catalog.search",
            serde_json::json!({"query": "Message", "recursive": true, "max_neighbors": 4}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let result = resp.result.expect("test: result");
        assert_eq!(result["status"], "ok");
        assert!(
            result["note"]
                .as_str()
                .expect("test: note str")
                .contains("no network manager"),
            "must explain that peer fan-out was skipped: {:?}",
            result["note"]
        );
        assert_eq!(result["neighbors_queried"], 0);
        let matches = result["matches"].as_array().expect("matches array");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["name"], "Message");
        assert_eq!(matches[0]["source"], "local");
    }

    #[tokio::test]
    async fn test_catalog_search_invalid_params_missing_query() {
        let provider = Arc::new(FakeCatalogProvider::new());
        let state = state_with_provider(provider).await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("catalog.search", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some(), "missing query must error");
        let err = resp.error.expect("test: error present");
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(
            err.message.contains("'query'"),
            "error must mention 'query': {}",
            err.message
        );
    }

    #[tokio::test]
    async fn test_catalog_search_no_match_returns_empty_array() {
        let provider = Arc::new(FakeCatalogProvider::new());
        let h = hash_from(b"only-one");
        provider.insert(typedef("OnlyOne", h, vec![])).await;
        let state = state_with_provider(provider).await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "catalog.search",
            serde_json::json!({"query": "Nonexistent"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["status"], "ok");
        assert_eq!(result["total"], 0);
        assert!(result["matches"]
            .as_array()
            .expect("matches array")
            .is_empty());
    }
}
