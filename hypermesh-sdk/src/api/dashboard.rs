// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Dashboard deployment and management API.

use crate::client::HyperMeshClient;
use crate::error::SdkError;

/// Zero-cost wrapper providing dashboard operations.
#[derive(Debug)]
pub struct DashboardApi<'a> {
    pub(crate) client: &'a HyperMeshClient,
}

impl<'a> DashboardApi<'a> {
    /// Deploy a dashboard (name, manifest TOML, and file map).
    pub async fn deploy(
        &self,
        name: &str,
        manifest_toml: &str,
        files: &serde_json::Value,
    ) -> Result<serde_json::Value, SdkError> {
        self.client
            .raw_call(
                "dashboard.deploy",
                serde_json::json!({
                    "name": name,
                    "manifest_toml": manifest_toml,
                    "files": files
                }),
            )
            .await
    }

    /// List all deployed dashboards on this node.
    pub async fn list(&self) -> Result<serde_json::Value, SdkError> {
        self.client
            .raw_call("dashboard.list", serde_json::json!({}))
            .await
    }

    /// Get info about a specific dashboard by name.
    pub async fn info(&self, name: &str) -> Result<serde_json::Value, SdkError> {
        self.client
            .raw_call("dashboard.info", serde_json::json!({"name": name}))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_api_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<DashboardApi<'_>>();
    }
}
