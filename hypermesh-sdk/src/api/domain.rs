// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! Domain registration and network management API.

use crate::client::HyperMeshClient;
use crate::error::SdkError;

/// Zero-cost wrapper providing domain operations.
#[derive(Debug)]
pub struct DomainApi<'a> {
    pub(crate) client: &'a HyperMeshClient,
}

impl<'a> DomainApi<'a> {
    /// Register a new domain, creating a Network-scope blockchain.
    pub async fn register(
        &self,
        name: &str,
        privacy: Option<&str>,
    ) -> Result<serde_json::Value, SdkError> {
        let mut params = serde_json::json!({"name": name});
        if let Some(p) = privacy {
            params["privacy"] = serde_json::Value::String(p.to_string());
        }
        self.client.raw_call("domain.register", params).await
    }

    /// List all domains registered on this node.
    pub async fn list(&self) -> Result<serde_json::Value, SdkError> {
        self.client
            .raw_call("domain.list", serde_json::json!({}))
            .await
    }

    /// Join an existing domain network.
    pub async fn join(&self, domain: &str) -> Result<serde_json::Value, SdkError> {
        self.client
            .raw_call("domain.join", serde_json::json!({"domain": domain}))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_api_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<DomainApi<'_>>();
    }
}
