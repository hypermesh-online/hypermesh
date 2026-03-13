// Copyright © 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Value storage for DHT (package announcements, peer lists, search indices)

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use super::{DhtNodeId, Distance, NodeInfo};
use crate::assets::AssetPackageId;
use crate::distribution::stoq_transport::PackageAnnouncement;

/// Value store for DHT
pub(crate) struct ValueStore {
    /// Stored values by key
    pub(crate) values: HashMap<ValueKey, Vec<StoredValue>>,
    /// Package index
    pub(crate) package_index: HashMap<AssetPackageId, HashSet<ValueKey>>,
}

/// Key for stored values
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ValueKey(pub(crate) [u8; 32]);

impl ValueKey {
    /// Create key from package ID
    pub(crate) fn from_package_id(id: &AssetPackageId) -> Self {
        let hash = blake3::hash(id.as_bytes());
        Self(*hash.as_bytes())
    }

    /// Create key from search query
    pub(crate) fn from_query(query: &str) -> Self {
        let hash = blake3::hash(query.as_bytes());
        Self(*hash.as_bytes())
    }
}

/// Stored value in DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StoredValue {
    /// The actual value data
    pub(crate) data: ValueData,
    /// Publisher node ID
    pub(crate) publisher: DhtNodeId,
    /// Publication timestamp
    pub(crate) published_at: SystemTime,
    /// Expiration time
    pub(crate) expires_at: SystemTime,
}

/// Value data types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum ValueData {
    /// Package announcement
    PackageAnnouncement(PackageAnnouncement),
    /// Peer list for a package
    PackagePeers {
        package_id: AssetPackageId,
        peers: Vec<DhtNodeId>,
    },
    /// Search index entry
    SearchIndex {
        keyword: String,
        packages: Vec<AssetPackageId>,
    },
}

/// Query ID for tracking pending queries
pub(crate) type QueryId = [u8; 16];

/// Pending query information
pub(crate) struct PendingQuery {
    /// Query type
    pub(crate) _query_type: QueryType,
    /// Target key
    pub(crate) _target: ValueKey,
    /// Nodes to query
    pub(crate) _to_query: Vec<DhtNodeId>,
    /// Nodes already queried
    pub(crate) _queried: HashSet<DhtNodeId>,
    /// Best nodes found so far
    pub(crate) _best_nodes: BTreeMap<Distance, NodeInfo>,
    /// Values found
    pub(crate) _values: Vec<StoredValue>,
    /// Query start time
    pub(crate) _started_at: std::time::Instant,
}

/// Query types
pub(crate) enum QueryType {
    _FindNode,
    _FindValue,
    _Store,
}

impl ValueStore {
    pub(crate) fn new() -> Self {
        Self {
            values: HashMap::new(),
            package_index: HashMap::new(),
        }
    }

    pub(crate) fn store(&mut self, key: ValueKey, value: StoredValue) {
        // Extract package ID if present
        if let ValueData::PackageAnnouncement(ref announcement) = value.data {
            self.package_index
                .entry(announcement.package_id)
                .or_default()
                .insert(key.clone());
        }

        self.values.entry(key).or_default().push(value);
    }

    pub(crate) fn get(&self, key: &ValueKey) -> Option<Vec<StoredValue>> {
        self.values.get(key).cloned()
    }

    pub(crate) fn clean_expired(&mut self) {
        let now = SystemTime::now();

        // Remove expired values
        for values in self.values.values_mut() {
            values.retain(|v| v.expires_at > now);
        }

        // Remove empty entries
        self.values.retain(|_, v| !v.is_empty());

        // Update package index
        self.package_index.retain(|_, keys| {
            keys.retain(|k| self.values.contains_key(k));
            !keys.is_empty()
        });
    }
}
