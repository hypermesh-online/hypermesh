// Copyright (c) 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! DNS-over-HTTPS (DoH) endpoint for browser-compatible DNS resolution.
//!
//! Provides a simplified JSON-based DNS query/response interface. This is
//! **not** the full RFC 8484 binary wire format -- it accepts and returns JSON,
//! which is sufficient for the alpha phase and easy to integrate from
//! JavaScript clients.
//!
//! Queries are resolved against a `HashMap<String, SocketAddr>` that the
//! gateway populates from the node's blockchain DNS registry on startup.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;

/// A simplified DNS query (JSON-based, not wire format).
#[derive(Debug, Clone, Deserialize)]
pub struct DnsQuery {
    /// The name to resolve (e.g. `"persist"`).
    pub name: String,

    /// Query type hint (e.g. `"A"`, `"AAAA"`). Defaults to `"A"`.
    #[serde(default = "default_query_type")]
    pub query_type: String,
}

fn default_query_type() -> String {
    "A".to_string()
}

/// A simplified DNS response.
#[derive(Debug, Clone, Serialize)]
pub struct DnsResponse {
    /// The queried name.
    pub name: String,

    /// Echoed query type.
    pub query_type: String,

    /// Resolved address, or `None` for NXDOMAIN.
    pub address: Option<String>,

    /// Time-to-live in seconds (informational).
    pub ttl: u32,

    /// DNS status: `"NOERROR"` on success, `"NXDOMAIN"` when not found.
    pub status: String,
}

/// Resolve a DNS query against a set of known name-to-address records.
///
/// Returns a `DnsResponse` with status `"NOERROR"` and the resolved IP
/// address on success, or `"NXDOMAIN"` when no record exists for the
/// queried name.
pub fn resolve_query(query: &DnsQuery, records: &HashMap<String, SocketAddr>) -> DnsResponse {
    match records.get(&query.name) {
        Some(addr) => DnsResponse {
            name: query.name.clone(),
            query_type: query.query_type.clone(),
            address: Some(addr.ip().to_string()),
            ttl: 300,
            status: "NOERROR".to_string(),
        },
        None => DnsResponse {
            name: query.name.clone(),
            query_type: query.query_type.clone(),
            address: None,
            ttl: 0,
            status: "NXDOMAIN".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_records() -> HashMap<String, SocketAddr> {
        let mut records = HashMap::new();
        records.insert(
            "persist".to_string(),
            "[::1]:8080".parse().expect("test: valid addr"),
        );
        records.insert(
            "trust".to_string(),
            "[::1]:8443".parse().expect("test: valid addr"),
        );
        records
    }

    #[test]
    fn resolve_known_name() {
        let records = test_records();
        let query = DnsQuery {
            name: "persist".to_string(),
            query_type: "AAAA".to_string(),
        };
        let resp = resolve_query(&query, &records);
        assert_eq!(resp.status, "NOERROR");
        assert_eq!(resp.name, "persist");
        assert_eq!(resp.query_type, "AAAA");
        assert!(resp.address.is_some());
        assert_eq!(resp.address.as_deref(), Some("::1"));
        assert_eq!(resp.ttl, 300);
    }

    #[test]
    fn resolve_unknown_name() {
        let records = HashMap::new();
        let query = DnsQuery {
            name: "nonexistent".to_string(),
            query_type: "A".to_string(),
        };
        let resp = resolve_query(&query, &records);
        assert_eq!(resp.status, "NXDOMAIN");
        assert!(resp.address.is_none());
        assert_eq!(resp.ttl, 0);
    }

    #[test]
    fn resolve_second_record() {
        let records = test_records();
        let query = DnsQuery {
            name: "trust".to_string(),
            query_type: "A".to_string(),
        };
        let resp = resolve_query(&query, &records);
        assert_eq!(resp.status, "NOERROR");
        assert!(resp.address.is_some());
    }

    #[test]
    fn default_query_type_is_a() {
        let json = r#"{"name": "persist"}"#;
        let query: DnsQuery = serde_json::from_str(json).expect("test: parse query");
        assert_eq!(query.query_type, "A");
    }

    #[test]
    fn dns_response_serializes_to_json() {
        let resp = DnsResponse {
            name: "test".to_string(),
            query_type: "AAAA".to_string(),
            address: Some("::1".to_string()),
            ttl: 300,
            status: "NOERROR".to_string(),
        };
        let json = serde_json::to_string(&resp).expect("test: serialize");
        assert!(json.contains("NOERROR"));
        assert!(json.contains("::1"));
    }

    #[test]
    fn nxdomain_response_serializes_with_null_address() {
        let resp = DnsResponse {
            name: "missing".to_string(),
            query_type: "A".to_string(),
            address: None,
            ttl: 0,
            status: "NXDOMAIN".to_string(),
        };
        let json = serde_json::to_string(&resp).expect("test: serialize");
        assert!(json.contains("NXDOMAIN"));
        assert!(json.contains("null"));
    }
}
