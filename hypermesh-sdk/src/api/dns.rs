// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.

//! DNS registration and resolution API.

use crate::client::HyperMeshClient;
use crate::error::SdkError;

/// Zero-cost wrapper providing DNS operations.
#[derive(Debug)]
pub struct DnsApi<'a> {
    pub(crate) client: &'a HyperMeshClient,
}

/// A registered DNS entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DnsEntry {
    /// The registered name (e.g. "myapp.hypermesh").
    pub name: String,
    /// The IPv6 address this name resolves to.
    pub address: String,
}

impl<'a> DnsApi<'a> {
    /// Register a DNS name pointing to the given address.
    pub async fn register(&self, name: &str, address: &str) -> Result<DnsEntry, SdkError> {
        let val = self
            .client
            .raw_call(
                "dns.register",
                serde_json::json!({"name": name, "address": address}),
            )
            .await?;
        serde_json::from_value(val).map_err(|e| SdkError::Serialization(e.to_string()))
    }

    /// Resolve a DNS name to its address.
    pub async fn resolve(&self, name: &str) -> Result<String, SdkError> {
        let val = self
            .client
            .raw_call("dns.resolve", serde_json::json!({"name": name}))
            .await?;
        val.as_str()
            .map(String::from)
            .ok_or_else(|| SdkError::Serialization("expected string address".into()))
    }

    /// List all registered DNS entries on this node.
    pub async fn list(&self) -> Result<Vec<DnsEntry>, SdkError> {
        let val = self
            .client
            .raw_call("dns.list", serde_json::json!({}))
            .await?;
        serde_json::from_value(val).map_err(|e| SdkError::Serialization(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_dns_entry() {
        let json = serde_json::json!({
            "name": "myapp.hypermesh",
            "address": "fd00::1"
        });
        let entry: DnsEntry = serde_json::from_value(json).expect("test: deserialize DnsEntry");
        assert_eq!(entry.name, "myapp.hypermesh");
        assert_eq!(entry.address, "fd00::1");
    }

    #[test]
    fn deserialize_dns_list() {
        let json = serde_json::json!([
            {"name": "a.hypermesh", "address": "fd00::1"},
            {"name": "b.hypermesh", "address": "fd00::2"}
        ]);
        let entries: Vec<DnsEntry> =
            serde_json::from_value(json).expect("test: deserialize DnsEntry list");
        assert_eq!(entries.len(), 2);
    }
}
