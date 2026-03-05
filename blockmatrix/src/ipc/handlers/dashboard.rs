// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Dashboard IPC handlers: deploy, list, info.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::assets::core::{AssetCategory, AssetData, AssetRegistration, BaseSystemType, NetworkScope};
use crate::dashboard::deploy;
use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::{RpcError, INTERNAL_ERROR, INVALID_PARAMS};
use crate::ipc::state::DaemonState;
use crate::StateProof;

/// Register dashboard-related IPC methods.
pub fn register(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    register_deploy(handler, state);
    register_list(handler, state);
    register_info(handler, state);
}

/// `dashboard.deploy` -- deploy a dashboard from provided manifest and files.
///
/// Params: `{ name: str, manifest_toml: str, files: { path: base64_content } }`
fn register_deploy(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    let s = state.clone();
    handler.register(
        "dashboard.deploy",
        Arc::new(move |params| {
            let s = s.clone();
            Box::pin(async move {
                let name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError {
                        code: INVALID_PARAMS,
                        message: "missing 'name' parameter".into(),
                        data: None,
                    })?
                    .to_string();

                let manifest_toml = params
                    .get("manifest_toml")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError {
                        code: INVALID_PARAMS,
                        message: "missing 'manifest_toml' parameter".into(),
                        data: None,
                    })?
                    .to_string();

                let files_obj = params
                    .get("files")
                    .and_then(|v| v.as_object())
                    .ok_or_else(|| RpcError {
                        code: INVALID_PARAMS,
                        message: "missing 'files' parameter (object)".into(),
                        data: None,
                    })?;

                // Validate manifest
                let manifest = crate::dashboard::parse_manifest(&manifest_toml)
                    .map_err(|e| RpcError {
                        code: INVALID_PARAMS,
                        message: format!("invalid manifest: {e}"),
                        data: None,
                    })?;

                // Decode base64 files
                use base64::Engine as _;
                let mut files = BTreeMap::new();
                for (path, val) in files_obj {
                    let b64 = val.as_str().ok_or_else(|| RpcError {
                        code: INVALID_PARAMS,
                        message: format!("file value for '{path}' must be a base64 string"),
                        data: None,
                    })?;
                    let content = base64::engine::general_purpose::STANDARD
                        .decode(b64)
                        .map_err(|e| RpcError {
                            code: INVALID_PARAMS,
                            message: format!("invalid base64 for '{path}': {e}"),
                            data: None,
                        })?;
                    files.insert(path.clone(), content);
                }

                // Bundle and hash
                let bundle = deploy::bundle_files(&files);
                let content_hash = deploy::blake3_hash(&bundle);

                // Register as blockchain asset
                let asset_data = AssetData {
                    config: format!("DASHBOARD:DEPLOY:{name}").into_bytes(),
                    definition: bundle,
                    metadata: manifest_toml.as_bytes().to_vec(),
                };
                let registration = AssetRegistration::from_asset_data(
                    &asset_data,
                    NetworkScope::Global,
                    AssetCategory::BaseSystem(BaseSystemType::Dashboard),
                );
                let state_proof = StateProof::new_for_testing();
                let block = s
                    .blockchain
                    .register_asset_record(registration, &state_proof)
                    .await
                    .map_err(|e| RpcError {
                        code: INTERNAL_ERROR,
                        message: format!("blockchain write failed: {e}"),
                        data: None,
                    })?;

                // Persist to disk for local serving
                deploy::persist_dashboard(&s.data_dir, &name, &manifest_toml, &files)
                    .map_err(|e| RpcError {
                        code: INTERNAL_ERROR,
                        message: format!("failed to persist dashboard: {e}"),
                        data: None,
                    })?;

                Ok(serde_json::json!({
                    "status": "deployed",
                    "name": name,
                    "version": manifest.dashboard.version,
                    "domain": manifest.dashboard.domain,
                    "hash": hex::encode(content_hash),
                    "block": block.index,
                    "files": files.len(),
                }))
            })
        }),
    );
}

/// `dashboard.list` -- scan `<data_dir>/dashboards/` for deployed dashboards.
fn register_list(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    let s = state.clone();
    handler.register(
        "dashboard.list",
        Arc::new(move |_params| {
            let s = s.clone();
            Box::pin(async move {
                let names = deploy::list_dashboards(&s.data_dir);
                let dashboards: Vec<serde_json::Value> = names
                    .iter()
                    .map(|name| {
                        if let Some(m) = deploy::load_dashboard_manifest(&s.data_dir, name) {
                            serde_json::json!({
                                "name": m.dashboard.name,
                                "version": m.dashboard.version,
                                "domain": m.dashboard.domain,
                                "description": m.dashboard.description,
                            })
                        } else {
                            serde_json::json!({
                                "name": name,
                                "error": "failed to parse manifest",
                            })
                        }
                    })
                    .collect();

                Ok(serde_json::json!({
                    "count": dashboards.len(),
                    "dashboards": dashboards,
                }))
            })
        }),
    );
}

/// `dashboard.info` -- return manifest data for a specific dashboard.
fn register_info(handler: &mut RequestHandler, state: &Arc<DaemonState>) {
    let s = state.clone();
    handler.register(
        "dashboard.info",
        Arc::new(move |params| {
            let s = s.clone();
            Box::pin(async move {
                let name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| RpcError {
                        code: INVALID_PARAMS,
                        message: "missing 'name' parameter".into(),
                        data: None,
                    })?;

                match deploy::load_dashboard_manifest(&s.data_dir, name) {
                    Some(m) => {
                        // Count files on disk
                        let dash_dir = s.data_dir.join("dashboards").join(name);
                        let file_count = walkdir::WalkDir::new(&dash_dir)
                            .into_iter()
                            .filter_map(|e| e.ok())
                            .filter(|e| {
                                e.file_type().is_file()
                                    && e.file_name() != "dashboard.toml"
                            })
                            .count();

                        Ok(serde_json::json!({
                            "name": m.dashboard.name,
                            "version": m.dashboard.version,
                            "domain": m.dashboard.domain,
                            "description": m.dashboard.description,
                            "found": true,
                            "files": file_count,
                            "access": {
                                "public": m.access.public,
                                "private": m.access.private,
                                "admin": m.access.admin,
                            },
                        }))
                    }
                    None => Ok(serde_json::json!({
                        "name": name,
                        "found": false,
                    })),
                }
            })
        }),
    );
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

    async fn test_state(data_dir: PathBuf) -> Arc<DaemonState> {
        let coord = MatrixCoordinate::new(0, 0, 0).expect("test: coord");
        let bc = Arc::new(NodeBlockchain::new(coord));
        let config = PersistenceConfig {
            storage_dir: data_dir.clone(),
            ..PersistenceConfig::default()
        };
        let persistence = Arc::new(
            PersistenceManager::new(config, "dash-test".into())
                .await
                .expect("test: persistence"),
        );
        let (shutdown_tx, _rx) = tokio::sync::watch::channel(false);

        Arc::new(DaemonState {
            blockchain: bc,
            persistence,
            network: None,
            coordinate: coord,
            node_id: "dash-test".into(),
            data_dir,
            privacy_mode: "Private".into(),
            started_at: Instant::now(),
            shutdown_tx,
            dns_resolver: DnsResolver::default(),
        })
    }

    #[tokio::test]
    async fn test_dashboard_list_empty() {
        let tmp = tempfile::TempDir::new().expect("test: tmpdir");
        let state = test_state(tmp.path().to_path_buf()).await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("dashboard.list", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["count"], 0);
    }

    #[tokio::test]
    async fn test_dashboard_info_not_found() {
        let tmp = tempfile::TempDir::new().expect("test: tmpdir");
        let state = test_state(tmp.path().to_path_buf()).await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "dashboard.info",
            serde_json::json!({"name": "nonexistent"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["found"], false);
    }

    #[tokio::test]
    async fn test_dashboard_info_missing_param() {
        let tmp = tempfile::TempDir::new().expect("test: tmpdir");
        let state = test_state(tmp.path().to_path_buf()).await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new("dashboard.info", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some());
        let err = resp.error.expect("test: error");
        assert_eq!(err.code, INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_dashboard_deploy_and_list() {
        let tmp = tempfile::TempDir::new().expect("test: tmpdir");
        let state = test_state(tmp.path().to_path_buf()).await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        use base64::Engine as _;
        let b64_html = base64::engine::general_purpose::STANDARD
            .encode(b"<h1>hello</h1>");

        let req = RpcRequest::new(
            "dashboard.deploy",
            serde_json::json!({
                "name": "my-dash",
                "manifest_toml": r#"[dashboard]
name = "my-dash"
version = "1.0.0"
description = "Test dashboard"
domain = "my-dash.hypermesh"

[access]
public = "dist/public/"
"#,
                "files": {
                    "public/index.html": b64_html,
                },
            }),
        );
        let resp = handler.dispatch(req).await;
        assert!(
            resp.error.is_none(),
            "deploy failed: {:?}",
            resp.error
        );
        let result = resp.result.expect("test: result");
        assert_eq!(result["status"], "deployed");
        assert_eq!(result["name"], "my-dash");
        assert_eq!(result["files"], 1);
        assert!(result["block"].as_u64().is_some());

        // Now list should show it
        let req = RpcRequest::new("dashboard.list", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["count"], 1);
        assert_eq!(result["dashboards"][0]["name"], "my-dash");

        // Info should return details
        let req = RpcRequest::new(
            "dashboard.info",
            serde_json::json!({"name": "my-dash"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("test: result");
        assert_eq!(result["found"], true);
        assert_eq!(result["name"], "my-dash");
        assert_eq!(result["version"], "1.0.0");
        assert_eq!(result["files"], 1);
    }

    #[tokio::test]
    async fn test_dashboard_deploy_missing_name() {
        let tmp = tempfile::TempDir::new().expect("test: tmpdir");
        let state = test_state(tmp.path().to_path_buf()).await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "dashboard.deploy",
            serde_json::json!({"manifest_toml": "x", "files": {}}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some());
        assert_eq!(resp.error.expect("test: err").code, INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_dashboard_deploy_invalid_manifest() {
        let tmp = tempfile::TempDir::new().expect("test: tmpdir");
        let state = test_state(tmp.path().to_path_buf()).await;
        let mut handler = RequestHandler::new();
        register(&mut handler, &state);

        let req = RpcRequest::new(
            "dashboard.deploy",
            serde_json::json!({
                "name": "bad",
                "manifest_toml": "not valid toml [[[",
                "files": {},
            }),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some());
    }
}
