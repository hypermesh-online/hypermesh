// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Consumer-becomes-provider flow (R12).
//!
//! After a node fetches shards during retrieval, it stores them locally
//! and announces availability to the mesh. This turns every consumer into
//! a provider, creating self-scaling distribution where popularity drives
//! replication. See `papers/HYPERMESH.md` R12 for the protocol requirement.

use std::sync::Arc;
use std::time::Duration;

use hypermesh_lib::ContentHash;
use tracing::{debug, info, warn};

use crate::network::shard_dedup::DedupPolicy;
use crate::network::shard_store::ShardStore;
use crate::network::swarm_provider::{
    build_shard_announce_payload, build_shard_locate_request, parse_shard_locate_response,
    ShardLocationIndex,
};

/// Result of processing fetched shards through the consumer-becomes-provider pipeline.
#[derive(Debug, Clone)]
pub struct ConsumerProviderResult {
    /// Number of shards that were newly stored.
    pub shards_stored: usize,
    /// Number of shards that were deduplicated (already existed).
    pub shards_deduped: usize,
    /// TAG_SHARD_ANNOUNCE payload ready for broadcast, or `None` if no shards to announce.
    pub announcement_payload: Option<Vec<u8>>,
}

/// Handles the consumer-becomes-provider flow (R12).
///
/// After fetching shards, the node stores them and announces availability.
/// This makes every consumer a provider, enabling O(log N) per-node load
/// as popularity drives replication across the mesh.
pub struct ConsumerProviderManager {
    shard_store: Arc<ShardStore>,
    shard_location_index: Arc<ShardLocationIndex>,
    local_node_id: String,
}

impl ConsumerProviderManager {
    /// Create a new consumer-provider manager.
    pub fn new(
        shard_store: Arc<ShardStore>,
        shard_location_index: Arc<ShardLocationIndex>,
        local_node_id: String,
    ) -> Self {
        Self {
            shard_store,
            shard_location_index,
            local_node_id,
        }
    }

    /// Process fetched shards: store locally, register as provider, build announcement.
    ///
    /// Uses the given `DedupPolicy` for storage. With `Full`, refcounts are
    /// tracked and the node is announced as a provider. With `HashOnly`
    /// (Anonymous R4), storage dedup is performed but provider registration
    /// and shard announcements are skipped to preserve privacy.
    pub async fn process_fetched_shards(
        &self,
        shards: Vec<(ContentHash, Vec<u8>)>,
    ) -> ConsumerProviderResult {
        self.process_fetched_shards_with_policy(shards, DedupPolicy::Full)
            .await
    }

    /// Process fetched shards with an explicit dedup policy.
    ///
    /// When `policy` is `HashOnly` (Anonymous scope), shards are stored
    /// but the node is NOT registered as a provider and no announcement
    /// payload is generated — preserving Anonymous privacy guarantees.
    pub async fn process_fetched_shards_with_policy(
        &self,
        shards: Vec<(ContentHash, Vec<u8>)>,
        policy: DedupPolicy,
    ) -> ConsumerProviderResult {
        if shards.is_empty() {
            return ConsumerProviderResult {
                shards_stored: 0,
                shards_deduped: 0,
                announcement_payload: None,
            };
        }

        let mut stored = 0usize;
        let mut deduped = 0usize;
        let mut announced_hashes: Vec<ContentHash> = Vec::with_capacity(shards.len());

        for (hash, data) in &shards {
            let result = self
                .shard_store
                .store_with_dedup(*hash, data.clone(), policy)
                .await;

            match result {
                crate::network::shard_dedup::ShardStoreResult::Stored => {
                    stored += 1;
                    debug!("Stored fetched shard {}", hex::encode(hash.0));
                }
                crate::network::shard_dedup::ShardStoreResult::Deduplicated { ref_count } => {
                    deduped += 1;
                    debug!(
                        "Deduped fetched shard {} (refcount={})",
                        hex::encode(hash.0),
                        ref_count
                    );
                }
            }
            announced_hashes.push(*hash);
        }

        // For HashOnly (Anonymous R4): skip provider registration and
        // announcement to avoid leaking identity/location information.
        let announcement_payload = if policy == DedupPolicy::HashOnly {
            debug!(
                "HashOnly policy: skipping provider registration and announcement for {} shards",
                announced_hashes.len()
            );
            None
        } else {
            // Register ourselves as a provider for all shards
            self.shard_location_index
                .register_provider(&self.local_node_id, &announced_hashes)
                .await;

            // Build announcement payload
            if announced_hashes.is_empty() {
                None
            } else {
                info!(
                    "Consumer-becomes-provider: announcing {} shards (stored={}, deduped={})",
                    announced_hashes.len(),
                    stored,
                    deduped
                );
                Some(build_shard_announce_payload(&announced_hashes))
            }
        };

        ConsumerProviderResult {
            shards_stored: stored,
            shards_deduped: deduped,
            announcement_payload,
        }
    }

    /// Build a TAG_SHARD_ANNOUNCE payload from raw shard hashes.
    ///
    /// Wraps `build_shard_announce_payload` for callers that already
    /// have hash arrays rather than `ContentHash` values.
    pub fn build_announcement(&self, shard_hashes: &[[u8; 32]]) -> Vec<u8> {
        let content_hashes: Vec<ContentHash> =
            shard_hashes.iter().map(|h| ContentHash(*h)).collect();
        build_shard_announce_payload(&content_hashes)
    }
}

/// Broadcast a TAG_SHARD_ANNOUNCE payload to a set of connected peers.
///
/// Each peer is sent the payload over a fresh unidirectional STOQ stream.
/// Failures are logged at warn level but do not abort the broadcast — best-
/// effort delivery is the goal (other consumers will pick up the slack via
/// ngauge popularity tracking).
///
/// Returns the number of peers successfully reached.
///
/// Wire format: payload begins with `TAG_SHARD_ANNOUNCE` (0x04) followed by
/// the count and shard hashes — produced by `build_shard_announce_payload`
/// or `ConsumerProviderManager::process_fetched_shards`.
pub async fn broadcast_announcement(
    payload: &[u8],
    peers: &[Arc<stoq::Connection>],
) -> usize {
    if payload.is_empty() || peers.is_empty() {
        return 0;
    }
    let mut sent = 0usize;
    for conn in peers {
        if !conn.is_active() {
            continue;
        }
        match conn.open_stream().await {
            Ok(mut stream) => {
                if let Err(e) = stream.send(payload).await {
                    warn!("Failed to send TAG_SHARD_ANNOUNCE to peer: {e}");
                    continue;
                }
                sent += 1;
            }
            Err(e) => {
                warn!("Failed to open stream for TAG_SHARD_ANNOUNCE: {e}");
            }
        }
    }
    debug!(
        "Broadcast TAG_SHARD_ANNOUNCE to {}/{} peers ({} bytes)",
        sent,
        peers.len(),
        payload.len(),
    );
    sent
}

/// Default per-hop timeout for an upstream shard-locate query. Bounds a slow or
/// silent upstream so the fetch path fails over promptly instead of hanging.
pub const SHARD_LOCATE_TIMEOUT: Duration = Duration::from_secs(3);

/// Query a single upstream peer "who has `content_hash`?" (A2 upstream tracker
/// fallback).
///
/// This is the shard analog of the DNS upstream hop (`dns/resolver.rs`
/// `trustchain_client` fallback): when a node's local store, live-mirror index,
/// AND directly-connected peers all miss, it asks an UPSTREAM peer for provider
/// node_ids to widen the resolve. A single bounded, timeout-guarded hop over
/// STOQ mirroring the shard-announce wire pattern (`TAG_SHARD_LOCATE`).
///
/// Returns the provider node_ids the upstream knows (possibly empty). On any
/// transport error or timeout, returns an empty list — a failed upstream is
/// treated as "upstream knows nobody", never a hard error.
pub async fn locate_shard_upstream(
    upstream: &Arc<stoq::Connection>,
    content_hash: &ContentHash,
    timeout: Duration,
) -> Vec<String> {
    if !upstream.is_active() {
        return Vec::new();
    }

    let query = build_shard_locate_request(content_hash);

    let result = tokio::time::timeout(timeout, async {
        let mut stream = upstream
            .open_stream()
            .await
            .map_err(|e| format!("open stream: {e}"))?;
        stream
            .send(&query)
            .await
            .map_err(|e| format!("send locate: {e}"))?;
        let resp = stream
            .receive()
            .await
            .map_err(|e| format!("receive locate: {e}"))?;
        Ok::<Vec<u8>, String>(resp.to_vec())
    })
    .await;

    match result {
        Ok(Ok(bytes)) => parse_shard_locate_response(&bytes),
        Ok(Err(e)) => {
            debug!("Upstream shard-locate failed: {e}");
            Vec::new()
        }
        Err(_) => {
            debug!("Upstream shard-locate timed out after {:?}", timeout);
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_hash(seed: u8) -> ContentHash {
        ContentHash([seed; 32])
    }

    fn make_manager() -> ConsumerProviderManager {
        ConsumerProviderManager::new(
            Arc::new(ShardStore::new()),
            Arc::new(ShardLocationIndex::new()),
            "local-node-001".to_string(),
        )
    }

    #[tokio::test]
    async fn test_process_empty_shard_list() {
        let mgr = make_manager();
        let result = mgr.process_fetched_shards(vec![]).await;

        assert_eq!(result.shards_stored, 0);
        assert_eq!(result.shards_deduped, 0);
        assert!(result.announcement_payload.is_none());
    }

    #[tokio::test]
    async fn test_process_single_shard_stored_and_announced() {
        let mgr = make_manager();
        let hash = test_hash(0xAA);
        let data = vec![0xDE, 0xAD, 0xBE, 0xEF];

        let result = mgr
            .process_fetched_shards(vec![(hash, data.clone())])
            .await;

        assert_eq!(result.shards_stored, 1);
        assert_eq!(result.shards_deduped, 0);
        assert!(result.announcement_payload.is_some());

        // Verify shard is actually in the store
        let fetched = mgr.shard_store.get(&hash).await;
        assert_eq!(fetched, Some(data));

        // Verify local node is registered as provider
        let providers = mgr.shard_location_index.get_providers(&hash).await;
        assert!(providers.contains(&"local-node-001".to_string()));
    }

    #[tokio::test]
    async fn test_process_multiple_shards_with_dedup() {
        let mgr = make_manager();
        let hash_a = test_hash(0xBB);
        let hash_b = test_hash(0xCC);

        // Pre-store hash_a so it will be deduplicated
        mgr.shard_store
            .store(hash_a, vec![1, 2, 3])
            .await;

        let shards = vec![
            (hash_a, vec![1, 2, 3]), // already exists -> dedup
            (hash_b, vec![4, 5, 6]), // new -> stored
        ];

        let result = mgr.process_fetched_shards(shards).await;

        assert_eq!(result.shards_stored, 1);
        assert_eq!(result.shards_deduped, 1);
        assert!(result.announcement_payload.is_some());

        // Both shards should be in the store
        assert!(mgr.shard_store.has(&hash_a).await);
        assert!(mgr.shard_store.has(&hash_b).await);

        // Deduped shard should have refcount 2
        assert_eq!(mgr.shard_store.ref_count(&hash_a).await, Some(2));
    }

    #[tokio::test]
    async fn test_build_announcement_payload() {
        let mgr = make_manager();
        let hashes: Vec<[u8; 32]> = vec![[0x11; 32], [0x22; 32]];

        let payload = mgr.build_announcement(&hashes);

        // tag(1) + count(4) + 2*hash(32) = 69
        assert_eq!(payload.len(), 69);
        assert_eq!(payload[0], 0x04); // TAG_SHARD_ANNOUNCE
        let count = u32::from_le_bytes([payload[1], payload[2], payload[3], payload[4]]);
        assert_eq!(count, 2);
        assert_eq!(&payload[5..37], &[0x11; 32]);
        assert_eq!(&payload[37..69], &[0x22; 32]);
    }

    #[tokio::test]
    async fn test_local_node_registered_as_provider() {
        let mgr = make_manager();
        let hash_x = test_hash(0xDD);
        let hash_y = test_hash(0xEE);

        let shards = vec![
            (hash_x, vec![10, 20]),
            (hash_y, vec![30, 40]),
        ];

        mgr.process_fetched_shards(shards).await;

        // Local node should be a provider for both shards
        let providers_x = mgr.shard_location_index.get_providers(&hash_x).await;
        let providers_y = mgr.shard_location_index.get_providers(&hash_y).await;

        assert_eq!(providers_x.len(), 1);
        assert_eq!(providers_y.len(), 1);
        assert!(providers_x.contains(&"local-node-001".to_string()));
        assert!(providers_y.contains(&"local-node-001".to_string()));

        // Provider count should be 1 (just the local node)
        assert_eq!(mgr.shard_location_index.provider_count().await, 1);
        // Shard count should be 2
        assert_eq!(mgr.shard_location_index.shard_count().await, 2);
    }

    #[tokio::test]
    async fn test_anonymous_skips_provider_registration() {
        let mgr = make_manager();
        let hash = test_hash(0xF1);
        let data = vec![0xAA, 0xBB, 0xCC];

        let result = mgr
            .process_fetched_shards_with_policy(
                vec![(hash, data.clone())],
                DedupPolicy::HashOnly,
            )
            .await;

        // Shard should be stored
        assert_eq!(result.shards_stored, 1);
        assert_eq!(result.shards_deduped, 0);
        let fetched = mgr.shard_store.get(&hash).await;
        assert_eq!(fetched, Some(data));

        // But NOT registered as provider
        let providers = mgr.shard_location_index.get_providers(&hash).await;
        assert!(
            providers.is_empty(),
            "expected no providers for HashOnly, got {:?}",
            providers
        );
    }

    #[tokio::test]
    async fn test_anonymous_skips_announcement() {
        let mgr = make_manager();
        let hash = test_hash(0xF2);

        let result = mgr
            .process_fetched_shards_with_policy(
                vec![(hash, vec![1, 2, 3])],
                DedupPolicy::HashOnly,
            )
            .await;

        assert!(
            result.announcement_payload.is_none(),
            "expected no announcement for HashOnly policy"
        );
    }

    #[tokio::test]
    async fn test_full_policy_still_registers_and_announces() {
        let mgr = make_manager();
        let hash = test_hash(0xF3);

        let result = mgr
            .process_fetched_shards_with_policy(
                vec![(hash, vec![4, 5, 6])],
                DedupPolicy::Full,
            )
            .await;

        assert_eq!(result.shards_stored, 1);
        assert!(result.announcement_payload.is_some());
        let providers = mgr.shard_location_index.get_providers(&hash).await;
        assert!(providers.contains(&"local-node-001".to_string()));
    }

    #[tokio::test]
    async fn test_build_announcement_empty() {
        let mgr = make_manager();
        let payload = mgr.build_announcement(&[]);

        assert_eq!(payload.len(), 5);
        assert_eq!(payload[0], 0x04);
        let count = u32::from_le_bytes([payload[1], payload[2], payload[3], payload[4]]);
        assert_eq!(count, 0);
    }
}
