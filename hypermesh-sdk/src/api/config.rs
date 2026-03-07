// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Node configuration API.

use crate::client::HyperMeshClient;
use crate::error::SdkError;

/// Zero-cost wrapper providing config operations.
#[derive(Debug)]
pub struct ConfigApi<'a> {
    pub(crate) client: &'a HyperMeshClient,
}

impl<'a> ConfigApi<'a> {
    /// Show the full node configuration.
    pub async fn show(&self) -> Result<serde_json::Value, SdkError> {
        self.client
            .raw_call("config.show", serde_json::json!({}))
            .await
    }

    /// Get a config value by dotpath key (e.g. "network.stoq_port").
    pub async fn get(&self, key: &str) -> Result<serde_json::Value, SdkError> {
        self.client
            .raw_call("config.get", serde_json::json!({"key": key}))
            .await
    }

    /// Set a config value by dotpath key.
    pub async fn set(
        &self,
        key: &str,
        value: serde_json::Value,
    ) -> Result<serde_json::Value, SdkError> {
        self.client
            .raw_call("config.set", serde_json::json!({"key": key, "value": value}))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_api_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<ConfigApi<'_>>();
    }
}
