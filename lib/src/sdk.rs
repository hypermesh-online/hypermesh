// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Public SDK types for third-party integration.
//!
//! These types provide a stable, simplified API surface for external consumers
//! that need to query or interact with HyperMesh nodes without depending on
//! internal implementation details.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::asset::SystemAssetKind;
use crate::runtime::NodeState;
use crate::types::{AssetId, MatrixPosition, NodeId, PrivacyMode};

/// Current SDK version string.
pub const SDK_VERSION: &str = env!("CARGO_PKG_VERSION");

// ---------------------------------------------------------------------------
// SdkCapabilities
// ---------------------------------------------------------------------------

/// Describes features supported by this SDK build.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdkCapabilities {
    /// SDK version (matches crate version).
    pub version: String,
    /// Supported asset query operations.
    pub supports_asset_query: bool,
    /// Supported node discovery operations.
    pub supports_node_discovery: bool,
    /// Whether Proof of State validation is available.
    pub supports_pos_validation: bool,
    /// Whether streaming metrics are available.
    pub supports_streaming_metrics: bool,
}

impl Default for SdkCapabilities {
    fn default() -> Self {
        Self {
            version: SDK_VERSION.to_string(),
            supports_asset_query: true,
            supports_node_discovery: true,
            supports_pos_validation: true,
            supports_streaming_metrics: false,
        }
    }
}

impl fmt::Display for SdkCapabilities {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SDK v{} (query={}, discovery={}, pos={})",
            self.version,
            self.supports_asset_query,
            self.supports_node_discovery,
            self.supports_pos_validation,
        )
    }
}

// ---------------------------------------------------------------------------
// AssetDescriptor — simplified asset info for external consumers
// ---------------------------------------------------------------------------

/// Simplified asset metadata for third-party integrators.
///
/// Contains only the fields needed for discovery and display, without
/// internal blockchain details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetDescriptor {
    /// Unique asset identifier.
    pub id: AssetId,
    /// Human-readable name.
    pub name: String,
    /// System asset kind (if system asset).
    pub kind: Option<SystemAssetKind>,
    /// Asset payload size in bytes.
    pub size_bytes: u64,
}

impl fmt::Display for AssetDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            Some(k) => write!(f, "Asset({}, {}, {}B)", self.id, k, self.size_bytes),
            None => write!(f, "Asset({}, user-defined, {}B)", self.id, self.size_bytes),
        }
    }
}

// ---------------------------------------------------------------------------
// NodeDescriptor — simplified node info for external consumers
// ---------------------------------------------------------------------------

/// Simplified node information for third-party integrators.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDescriptor {
    /// Cryptographic node identity.
    pub id: NodeId,
    /// Position in the Block-MATRIX topology.
    pub position: MatrixPosition,
    /// Current node lifecycle state.
    pub state: NodeState,
    /// Active privacy mode for this node's transport.
    pub privacy_mode: PrivacyMode,
}

impl fmt::Display for NodeDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Node({}, pos=({:.0},{:.0},{:.0}), {}, {})",
            self.id, self.position.x, self.position.y, self.position.z,
            self.state, self.privacy_mode,
        )
    }
}

// ---------------------------------------------------------------------------
// QueryResult<T> — paginated result wrapper
// ---------------------------------------------------------------------------

/// Standard paginated result type for SDK queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult<T> {
    /// Result items for this page.
    pub items: Vec<T>,
    /// Total number of matching items across all pages.
    pub total: u64,
    /// Offset of the first item in this page (0-based).
    pub offset: u64,
    /// Maximum items per page.
    pub limit: u64,
}

impl<T> QueryResult<T> {
    /// Create an empty result with the given pagination parameters.
    pub fn empty(offset: u64, limit: u64) -> Self {
        Self {
            items: Vec::new(),
            total: 0,
            offset,
            limit,
        }
    }

    /// Whether there are more pages after this one.
    pub fn has_next_page(&self) -> bool {
        self.offset + self.limit < self.total
    }

    /// Number of items in this page.
    pub fn page_size(&self) -> usize {
        self.items.len()
    }
}

impl<T: fmt::Debug> fmt::Display for QueryResult<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "QueryResult({} items, {}/{} total, offset={})",
            self.items.len(),
            self.items.len(),
            self.total,
            self.offset,
        )
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdk_capabilities_default() {
        let caps = SdkCapabilities::default();
        assert!(!caps.version.is_empty());
        assert!(caps.supports_asset_query);
        assert!(caps.supports_node_discovery);
        assert!(caps.supports_pos_validation);
    }

    #[test]
    fn sdk_capabilities_serde_roundtrip() {
        let caps = SdkCapabilities::default();
        let json = serde_json::to_string(&caps).expect("test: serialize");
        let back: SdkCapabilities = serde_json::from_str(&json).expect("test: deserialize");
        assert_eq!(caps.version, back.version);
    }

    #[test]
    fn asset_descriptor_display() {
        let desc = AssetDescriptor {
            id: AssetId::from("asset-001"),
            name: "Test Asset".to_string(),
            kind: Some(SystemAssetKind::Storage),
            size_bytes: 1024,
        };
        let s = desc.to_string();
        assert!(s.contains("Storage"), "got: {s}");
        assert!(s.contains("1024"), "got: {s}");
    }

    #[test]
    fn node_descriptor_display() {
        let desc = NodeDescriptor {
            id: NodeId::from_public_key(b"test-key"),
            position: MatrixPosition { x: 1.0, y: 2.0, z: 3.0 },
            state: NodeState::Ready,
            privacy_mode: PrivacyMode::PUBLIC,
        };
        let s = desc.to_string();
        assert!(s.contains("Ready"), "got: {s}");
        assert!(s.contains("Public"), "got: {s}");
    }

    #[test]
    fn query_result_empty() {
        let result: QueryResult<String> = QueryResult::empty(0, 10);
        assert_eq!(result.total, 0);
        assert_eq!(result.page_size(), 0);
        assert!(!result.has_next_page());
    }

    #[test]
    fn query_result_pagination() {
        let result: QueryResult<u32> = QueryResult {
            items: vec![1, 2, 3],
            total: 10,
            offset: 0,
            limit: 3,
        };
        assert_eq!(result.page_size(), 3);
        assert!(result.has_next_page());

        let last_page: QueryResult<u32> = QueryResult {
            items: vec![10],
            total: 10,
            offset: 9,
            limit: 3,
        };
        assert!(!last_page.has_next_page());
    }

    #[test]
    fn query_result_serde_roundtrip() {
        let result: QueryResult<String> = QueryResult {
            items: vec!["a".to_string(), "b".to_string()],
            total: 5,
            offset: 0,
            limit: 2,
        };
        let json = serde_json::to_string(&result).expect("test: serialize");
        let back: QueryResult<String> = serde_json::from_str(&json).expect("test: deserialize");
        assert_eq!(back.items.len(), 2);
        assert_eq!(back.total, 5);
    }
}
