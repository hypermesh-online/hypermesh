// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! IPC handlers for configuration management: show, get, set.

use std::sync::Arc;

use crate::ipc::config::{get_dotpath, set_dotpath, HypermeshConfig};
use crate::ipc::handler::RequestHandler;
use crate::ipc::protocol::{INTERNAL_ERROR, INVALID_PARAMS};

/// Register config-related IPC methods.
pub fn register(handler: &mut RequestHandler) {
    // config.show — return entire config as JSON
    handler.register(
        "config.show",
        Arc::new(|_params| {
            Box::pin(async {
                let config = HypermeshConfig::load();
                serde_json::to_value(&config).map_err(|e| crate::ipc::protocol::RpcError {
                    code: INTERNAL_ERROR,
                    message: format!("failed to serialize config: {e}"),
                    data: None,
                })
            })
        }),
    );

    // config.get — get a single dotpath key
    handler.register(
        "config.get",
        Arc::new(|params| {
            Box::pin(async move {
                let key = params
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| crate::ipc::protocol::RpcError {
                        code: INVALID_PARAMS,
                        message: "missing 'key' parameter".into(),
                        data: None,
                    })?;

                let config = HypermeshConfig::load();
                let value = serde_json::to_value(&config).map_err(|e| {
                    crate::ipc::protocol::RpcError {
                        code: INTERNAL_ERROR,
                        message: format!("failed to serialize config: {e}"),
                        data: None,
                    }
                })?;

                match get_dotpath(&value, key) {
                    Some(v) => Ok(v.clone()),
                    None => Err(crate::ipc::protocol::RpcError {
                        code: INVALID_PARAMS,
                        message: format!("key not found: {key}"),
                        data: None,
                    }),
                }
            })
        }),
    );

    // config.set — set a single dotpath key and save to disk
    handler.register(
        "config.set",
        Arc::new(|params| {
            Box::pin(async move {
                let key = params
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| crate::ipc::protocol::RpcError {
                        code: INVALID_PARAMS,
                        message: "missing 'key' parameter".into(),
                        data: None,
                    })?
                    .to_string();

                let new_value = params.get("value").ok_or_else(|| {
                    crate::ipc::protocol::RpcError {
                        code: INVALID_PARAMS,
                        message: "missing 'value' parameter".into(),
                        data: None,
                    }
                })?;

                // Load, update, and save
                let config = HypermeshConfig::load();
                let mut json_value = serde_json::to_value(&config).map_err(|e| {
                    crate::ipc::protocol::RpcError {
                        code: INTERNAL_ERROR,
                        message: format!("failed to serialize config: {e}"),
                        data: None,
                    }
                })?;

                set_dotpath(&mut json_value, &key, new_value.clone()).map_err(|e| {
                    crate::ipc::protocol::RpcError {
                        code: INVALID_PARAMS,
                        message: e,
                        data: None,
                    }
                })?;

                // Deserialize back and save
                let updated: HypermeshConfig =
                    serde_json::from_value(json_value).map_err(|e| {
                        crate::ipc::protocol::RpcError {
                            code: INTERNAL_ERROR,
                            message: format!("invalid config after update: {e}"),
                            data: None,
                        }
                    })?;
                updated.save().map_err(|e| crate::ipc::protocol::RpcError {
                    code: INTERNAL_ERROR,
                    message: e,
                    data: None,
                })?;

                Ok(serde_json::json!({"key": key, "value": new_value, "saved": true}))
            })
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::protocol::RpcRequest;

    #[tokio::test]
    async fn test_config_show() {
        let mut handler = RequestHandler::new();
        register(&mut handler);

        let req = RpcRequest::new("config.show", serde_json::json!({}));
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "config.show should succeed");
        let result = resp.result.expect("test: result present");
        // Should have top-level sections
        assert!(result.get("node").is_some());
        assert!(result.get("network").is_some());
        assert!(result.get("logging").is_some());
    }

    #[tokio::test]
    async fn test_config_get_valid_key() {
        let mut handler = RequestHandler::new();
        register(&mut handler);

        let req = RpcRequest::new(
            "config.get",
            serde_json::json!({"key": "network.stoq_port"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_none(), "config.get should succeed");
        let result = resp.result.expect("test: result present");
        assert_eq!(result, serde_json::json!(9292));
    }

    #[tokio::test]
    async fn test_config_get_missing_key() {
        let mut handler = RequestHandler::new();
        register(&mut handler);

        let req = RpcRequest::new(
            "config.get",
            serde_json::json!({"key": "nonexistent.path"}),
        );
        let resp = handler.dispatch(req).await;
        assert!(resp.error.is_some(), "should fail for missing key");
    }
}
