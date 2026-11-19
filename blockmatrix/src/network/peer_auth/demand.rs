use std::collections::HashMap;

/// Tracks shard fetch demand from network peers.
///
/// Records which shards are being requested and by whom, providing
/// the data needed for ngauge's `SwarmAnalytics` to make replication
/// and dispersion decisions.
pub struct SwarmDemandTracker {
    /// Per-shard request counts and last-access timestamps, keyed by
    /// `(network, shard)`. With a single default network, every key's network
    /// is [`hypermesh_lib::DEFAULT_NETWORK`] and behavior is identical to the
    /// flat map.
    entries: tokio::sync::Mutex<
        HashMap<(hypermesh_lib::NetworkId, hypermesh_lib::ContentHash), DemandEntry>,
    >,
}

/// A single shard's demand record.
#[derive(Debug, Clone)]
pub struct DemandEntry {
    /// Total number of fetch requests for this shard.
    pub request_count: u64,
    /// Unix microsecond timestamp of last fetch request.
    pub last_request_us: u64,
    /// Node IDs of requesters (deduplicated for unique consumer count).
    pub requester_ids: std::collections::HashSet<String>,
}

impl SwarmDemandTracker {
    /// Create a new empty demand tracker.
    pub fn new() -> Self {
        Self {
            entries: tokio::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Record a fetch request for a shard within a specific network.
    pub async fn record_fetch_in_network(
        &self,
        network_id: hypermesh_lib::NetworkId,
        shard_id: hypermesh_lib::ContentHash,
        requester_node_id: &str,
    ) {
        let now_us = chrono::Utc::now().timestamp_micros() as u64;
        let mut entries = self.entries.lock().await;
        record_fetch_locked(
            &mut entries,
            network_id,
            shard_id,
            requester_node_id,
            now_us,
        );
    }

    /// The sole fetch commit order: tracker mutex, then one current auth row.
    /// The row's own canonical network is used without a second lookup/fallback.
    pub(crate) async fn commit_authorized_fetch(
        &self,
        peers: &super::AuthenticatedPeers,
        node_id: &str,
        connection_id: &str,
        our_network_id: &str,
        shard_id: hypermesh_lib::ContentHash,
    ) -> bool {
        let mut entries = self.entries.lock().await;
        let peers = peers.read().await;
        let Some(peer) = peers.get(node_id) else {
            return false;
        };
        let Some(network_id) = super::shard_demand_network(peer, connection_id, our_network_id)
        else {
            return false;
        };
        let now_us = chrono::Utc::now().timestamp_micros() as u64;
        record_fetch_locked(&mut entries, network_id, shard_id, node_id, now_us);
        true
    }

    /// Get a snapshot of all demand entries across every network, flattened to
    /// `shard -> entry`. Retained for network-agnostic consumers
    /// (`NGaugeBridge::metrics_to_transmit`); the network-attributed feed path
    /// uses [`snapshot_in_network`](Self::snapshot_in_network) instead so demand
    /// is fed into `SwarmAnalytics` under the shard's own network.
    pub async fn snapshot(&self) -> HashMap<hypermesh_lib::ContentHash, DemandEntry> {
        self.entries
            .lock()
            .await
            .iter()
            .map(|((_network, shard), entry)| (*shard, entry.clone()))
            .collect()
    }

    /// Get a `shard -> entry` snapshot of the demand recorded for a single
    /// network. This is the network-attributed feed input: the replication feed
    /// iterates the node's joined networks and records each network's demand into
    /// `SwarmAnalytics` under that same network, so demand never flattens across
    /// networks.
    pub async fn snapshot_in_network(
        &self,
        network_id: hypermesh_lib::NetworkId,
    ) -> HashMap<hypermesh_lib::ContentHash, DemandEntry> {
        self.entries
            .lock()
            .await
            .iter()
            .filter(|((network, _shard), _)| *network == network_id)
            .map(|((_network, shard), entry)| (*shard, entry.clone()))
            .collect()
    }

    /// Get demand entry for a specific shard within a network.
    pub async fn get_in_network(
        &self,
        network_id: hypermesh_lib::NetworkId,
        shard_id: &hypermesh_lib::ContentHash,
    ) -> Option<DemandEntry> {
        self.entries
            .lock()
            .await
            .get(&(network_id, *shard_id))
            .cloned()
    }
}

impl Default for SwarmDemandTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Test-only ergonomic wrappers keyed on [`hypermesh_lib::DEFAULT_NETWORK`].
///
/// Production code MUST name the network explicitly via the `*_in_network`
/// methods so demand cannot silently flatten to the default network. These
/// `#[cfg(test)]` helpers keep the single-network test suite terse.
#[cfg(test)]
impl SwarmDemandTracker {
    pub(crate) async fn record_fetch(
        &self,
        shard_id: hypermesh_lib::ContentHash,
        requester_node_id: &str,
    ) {
        self.record_fetch_in_network(hypermesh_lib::DEFAULT_NETWORK, shard_id, requester_node_id)
            .await;
    }

    pub(crate) async fn get(&self, shard_id: &hypermesh_lib::ContentHash) -> Option<DemandEntry> {
        self.get_in_network(hypermesh_lib::DEFAULT_NETWORK, shard_id)
            .await
    }
}

fn record_fetch_locked(
    entries: &mut HashMap<(hypermesh_lib::NetworkId, hypermesh_lib::ContentHash), DemandEntry>,
    network_id: hypermesh_lib::NetworkId,
    shard_id: hypermesh_lib::ContentHash,
    requester_node_id: &str,
    now_us: u64,
) {
    let entry = entries
        .entry((network_id, shard_id))
        .or_insert_with(|| DemandEntry {
            request_count: 0,
            last_request_us: 0,
            requester_ids: std::collections::HashSet::new(),
        });
    entry.request_count += 1;
    entry.last_request_us = now_us;
    entry.requester_ids.insert(requester_node_id.to_string());
}
