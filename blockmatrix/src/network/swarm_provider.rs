// Copyright 2026 Hypermesh Foundation. All rights reserved.
// Licensed under the Business Source License 1.1.
// See the LICENSE file in the repository root for full license text.

//! Swarm provider system for consumer-becomes-provider (R12).
//!
//! When a node fetches shards to reconstruct an asset, it stores them
//! locally and announces availability. This makes the consumer a provider,
//! creating self-scaling distribution where popularity drives replication.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

use hypermesh_lib::{ContentHash, NetworkId};

/// Default provider-record lifetime (P6). Mirrors the DNS TTL cache
/// (`dns/cache.rs`): a provider learned via `TAG_SHARD_ANNOUNCE` is trusted
/// as a location hint only for this window unless refreshed by a fresh
/// announcement. This bounds staleness — a peer that goes offline stops being
/// handed out as a source once its record expires, and a re-announcing peer
/// keeps its record alive. 5 min matches the DNS default and the swarm
/// analytics window.
pub const DEFAULT_PROVIDER_TTL: Duration = Duration::from_secs(300);

/// A single provider record with TTL bookkeeping.
///
/// Mirrors `dns::cache::CacheEntry` (`registered_at`/`expires_at` + an
/// `is_expired` check) so the shard-location cache ages exactly like the DNS
/// cache instead of accumulating stale providers forever.
#[derive(Clone, Debug)]
struct ProviderEntry {
    /// When this provider was first learned or last refreshed. Used for
    /// oldest-first tie-breaking during introspection (`age`), mirroring the
    /// DNS cache's `cached_at`.
    registered_at: SystemTime,
    /// When this location hint should be considered stale.
    expires_at: SystemTime,
}

impl ProviderEntry {
    fn new(ttl: Duration) -> Self {
        let now = SystemTime::now();
        Self {
            registered_at: now,
            expires_at: now + ttl,
        }
    }

    /// Refresh an existing entry in place (keeps `registered_at`, extends TTL).
    fn refresh(&mut self, ttl: Duration) {
        self.expires_at = SystemTime::now() + ttl;
    }

    /// Whether this location hint has aged out.
    fn is_expired(&self) -> bool {
        SystemTime::now() > self.expires_at
    }

    /// How long ago this record was first registered (mirror of the DNS
    /// cache's `cached_at`-derived age). Used for oldest-first ordering.
    fn age(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.registered_at)
            .unwrap_or_default()
    }
}

/// Index mapping shard hashes to the node IDs that provide them, each with a
/// TTL (P6). Populated from `TAG_SHARD_ANNOUNCE` announcements received from
/// peers and from local consumer-becomes-provider fetches.
///
/// Unlike a plain set, entries EXPIRE: `get_providers` filters out stale hints
/// and `cleanup_expired` reclaims them, so a peer that disappears without a
/// clean `remove_provider` still ages out of the index (bounded staleness,
/// mirrors `dns/cache.rs`).
pub struct ShardLocationIndex {
    /// `(network, shard)` -> provider node id -> TTL record. With a single
    /// default network, every key's network is [`DEFAULT_NETWORK`] and lookup /
    /// iteration is identical to the flat `ContentHash`-keyed map.
    locations: Arc<RwLock<HashMap<(NetworkId, ContentHash), HashMap<String, ProviderEntry>>>>,
    /// TTL applied to newly-registered / refreshed provider records.
    ttl: Duration,
}

impl ShardLocationIndex {
    /// Create a new empty shard location index with the default TTL.
    pub fn new() -> Self {
        Self::with_ttl(DEFAULT_PROVIDER_TTL)
    }

    /// Create a new empty shard location index with an explicit provider TTL.
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            locations: Arc::new(RwLock::new(HashMap::new())),
            ttl,
        }
    }

    /// Register a node as a provider within a specific network.
    pub async fn register_provider_in_network(
        &self,
        network_id: NetworkId,
        node_id: &str,
        shard_ids: &[ContentHash],
    ) {
        self.register_provider_in_network_with_ttl(network_id, node_id, shard_ids, self.ttl)
            .await;
    }

    /// Register with an explicit TTL within a specific network.
    pub async fn register_provider_in_network_with_ttl(
        &self,
        network_id: NetworkId,
        node_id: &str,
        shard_ids: &[ContentHash],
        ttl: Duration,
    ) {
        let mut locs = self.locations.write().await;
        for shard_id in shard_ids {
            let providers = locs.entry((network_id, *shard_id)).or_default();
            providers
                .entry(node_id.to_string())
                .and_modify(|e| e.refresh(ttl))
                .or_insert_with(|| ProviderEntry::new(ttl));
        }
    }

    /// Remove a node from all shard provider sets across all networks (e.g., on
    /// disconnect).
    pub async fn remove_provider(&self, node_id: &str) {
        let mut locs = self.locations.write().await;
        for providers in locs.values_mut() {
            providers.remove(node_id);
        }
        locs.retain(|_, providers| !providers.is_empty());
    }

    /// Get all known, non-expired providers for a shard within a specific network,
    /// freshest-first.
    ///
    /// Expired location hints are filtered out (but not reclaimed — call
    /// [`cleanup_expired`] on a cadence to free memory). Ordering is by record
    /// age (most-recently registered/refreshed first) so callers that take the
    /// head of the list prefer the freshest announcement.
    pub async fn get_providers_in_network(
        &self,
        network_id: NetworkId,
        shard_id: &ContentHash,
    ) -> Vec<String> {
        let locs = self.locations.read().await;
        locs.get(&(network_id, *shard_id))
            .map(|providers| {
                let mut live: Vec<(&String, Duration)> = providers
                    .iter()
                    .filter(|(_, entry)| !entry.is_expired())
                    .map(|(id, entry)| (id, entry.age()))
                    .collect();
                // Freshest (smallest age) first; stable tie-break by node id.
                live.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(b.0)));
                live.into_iter().map(|(id, _)| id.clone()).collect()
            })
            .unwrap_or_default()
    }

    /// Reclaim expired provider records across all shards. Returns the number
    /// of provider records removed. Mirrors `DnsCache::cleanup_expired`.
    pub async fn cleanup_expired(&self) -> usize {
        let mut locs = self.locations.write().await;
        let mut removed = 0usize;
        for providers in locs.values_mut() {
            let before = providers.len();
            providers.retain(|_, entry| !entry.is_expired());
            removed += before - providers.len();
        }
        locs.retain(|_, providers| !providers.is_empty());
        removed
    }

    /// Get the number of tracked shards (including any with only expired
    /// records that have not yet been reclaimed).
    pub async fn shard_count(&self) -> usize {
        self.locations.read().await.len()
    }

    /// Get the number of unique, non-expired providers across all shards.
    pub async fn provider_count(&self) -> usize {
        let locs = self.locations.read().await;
        let mut all_providers = HashSet::new();
        for providers in locs.values() {
            for (id, entry) in providers {
                if !entry.is_expired() {
                    all_providers.insert(id.clone());
                }
            }
        }
        all_providers.len()
    }
}

impl Default for ShardLocationIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Test-only ergonomic wrappers keyed on [`hypermesh_lib::DEFAULT_NETWORK`].
///
/// Production code MUST name a shard's network explicitly via the `*_in_network`
/// methods — a shard belongs to its network and can never silently flatten to
/// the default. These `#[cfg(test)]` helpers keep the large single-network test
/// suite terse without reintroducing a production flattening path.
#[cfg(test)]
impl ShardLocationIndex {
    pub(crate) async fn register_provider(&self, node_id: &str, shard_ids: &[ContentHash]) {
        self.register_provider_in_network(hypermesh_lib::DEFAULT_NETWORK, node_id, shard_ids)
            .await;
    }

    pub(crate) async fn register_provider_with_ttl(
        &self,
        node_id: &str,
        shard_ids: &[ContentHash],
        ttl: Duration,
    ) {
        self.register_provider_in_network_with_ttl(
            hypermesh_lib::DEFAULT_NETWORK,
            node_id,
            shard_ids,
            ttl,
        )
        .await;
    }

    pub(crate) async fn get_providers(&self, shard_id: &ContentHash) -> Vec<String> {
        self.get_providers_in_network(hypermesh_lib::DEFAULT_NETWORK, shard_id)
            .await
    }
}

/// Build the wire payload for a TAG_SHARD_ANNOUNCE message.
///
/// Format: tag(1) + count(4 bytes u32 LE) + [shard_hash(32)]...
pub fn build_shard_announce_payload(shard_ids: &[ContentHash]) -> Vec<u8> {
    let count = shard_ids.len() as u32;
    let mut buf = Vec::with_capacity(5 + shard_ids.len() * 32);
    buf.push(0x04); // TAG_SHARD_ANNOUNCE
    buf.extend_from_slice(&count.to_le_bytes());
    for id in shard_ids {
        buf.extend_from_slice(&id.0);
    }
    buf
}

// ── Phase A2: shard-locate wire codec (upstream tracker fallback) ──────
//
// A LOCATE query returns provider node_ids for a content hash WITHOUT
// transferring the shard bytes (that is `TAG_SHARD_FETCH`). It is the shard
// analog of the DNS upstream query, letting a node ask a peer "who has X?"
// when its own local store, live-mirror index, and canonical placement all
// miss among directly-connected peers.

/// Tag byte for a shard-locate request. Must match
/// `message_handlers::protocol::TAG_SHARD_LOCATE`.
pub const TAG_SHARD_LOCATE: u8 = 0x52;
/// Tag byte for a shard-locate response. Must match
/// `message_handlers::protocol::TAG_SHARD_LOCATE_RESPONSE`.
pub const TAG_SHARD_LOCATE_RESPONSE: u8 = 0x53;

/// Build a shard-locate request payload: `tag(1) + content_hash(32)`.
pub fn build_shard_locate_request(content_hash: &ContentHash) -> Vec<u8> {
    let mut buf = Vec::with_capacity(33);
    buf.push(TAG_SHARD_LOCATE);
    buf.extend_from_slice(&content_hash.0);
    buf
}

/// Parse a shard-locate request, returning the queried content hash.
///
/// Returns `None` if the payload is malformed (wrong tag or too short).
pub fn parse_shard_locate_request(data: &[u8]) -> Option<ContentHash> {
    if data.len() < 33 || data[0] != TAG_SHARD_LOCATE {
        return None;
    }
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&data[1..33]);
    Some(ContentHash(hash))
}

/// Build a shard-locate response payload:
/// `tag(1) + count(4 LE) + [node_id_len(2 LE) + node_id_utf8]...`.
///
/// Node ids are length-prefixed UTF-8 (they are variable-length hex
/// strings). Ids longer than `u16::MAX` bytes are skipped defensively —
/// real node ids are 64 hex chars, so this never trips in practice.
pub fn build_shard_locate_response(node_ids: &[String]) -> Vec<u8> {
    let usable: Vec<&String> = node_ids
        .iter()
        .filter(|id| id.len() <= u16::MAX as usize)
        .collect();
    let count = usable.len() as u32;
    let mut buf = Vec::with_capacity(5 + usable.iter().map(|id| 2 + id.len()).sum::<usize>());
    buf.push(TAG_SHARD_LOCATE_RESPONSE);
    buf.extend_from_slice(&count.to_le_bytes());
    for id in usable {
        buf.extend_from_slice(&(id.len() as u16).to_le_bytes());
        buf.extend_from_slice(id.as_bytes());
    }
    buf
}

/// Parse a shard-locate response into its provider node_ids.
///
/// Returns an empty vec on malformed input (wrong tag, truncated length
/// prefix, or invalid UTF-8) rather than erroring — a missing/garbled
/// upstream answer is treated the same as "upstream knows nobody".
pub fn parse_shard_locate_response(data: &[u8]) -> Vec<String> {
    if data.len() < 5 || data[0] != TAG_SHARD_LOCATE_RESPONSE {
        return Vec::new();
    }
    // Cap `count` to what the buffer could possibly contain (each provider is a
    // 2-byte length prefix + >=1 byte), so an untrusted upstream peer cannot make
    // us pre-allocate ~103 GB from a crafted 5-byte response. Mirrors the guard
    // `handle_shard_announce` already applies before its `with_capacity`.
    let claimed = u32::from_le_bytes([data[1], data[2], data[3], data[4]]) as usize;
    let count = claimed.min(data.len().saturating_sub(5) / 3);
    let mut out = Vec::with_capacity(count);
    let mut off = 5usize;
    for _ in 0..count {
        if off + 2 > data.len() {
            break;
        }
        let len = u16::from_le_bytes([data[off], data[off + 1]]) as usize;
        off += 2;
        if off + len > data.len() {
            break;
        }
        if let Ok(s) = std::str::from_utf8(&data[off..off + len]) {
            out.push(s.to_string());
        }
        off += len;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(seed: u8) -> ContentHash {
        ContentHash([seed; 32])
    }

    #[tokio::test]
    async fn test_swarm_provider_register_and_get() {
        let index = ShardLocationIndex::new();
        let h1 = hash(0xAA);
        let h2 = hash(0xBB);

        index.register_provider("node-a", &[h1, h2]).await;

        let providers = index.get_providers(&h1).await;
        assert_eq!(providers.len(), 1);
        assert!(providers.contains(&"node-a".to_string()));

        let providers = index.get_providers(&h2).await;
        assert_eq!(providers.len(), 1);
    }

    #[tokio::test]
    async fn test_swarm_provider_multiple_providers() {
        let index = ShardLocationIndex::new();
        let h1 = hash(0xCC);

        index.register_provider("node-a", &[h1]).await;
        index.register_provider("node-b", &[h1]).await;

        let providers = index.get_providers(&h1).await;
        assert_eq!(providers.len(), 2);
        assert!(providers.contains(&"node-a".to_string()));
        assert!(providers.contains(&"node-b".to_string()));
    }

    #[tokio::test]
    async fn test_swarm_provider_remove() {
        let index = ShardLocationIndex::new();
        let h1 = hash(0xDD);
        let h2 = hash(0xEE);

        index.register_provider("node-a", &[h1, h2]).await;
        index.register_provider("node-b", &[h1]).await;

        index.remove_provider("node-a").await;

        // h1 still has node-b
        let providers = index.get_providers(&h1).await;
        assert_eq!(providers.len(), 1);
        assert!(providers.contains(&"node-b".to_string()));

        // h2 had only node-a, so it should be removed entirely
        let providers = index.get_providers(&h2).await;
        assert!(providers.is_empty());
        assert_eq!(index.shard_count().await, 1);
    }

    #[tokio::test]
    async fn test_swarm_provider_shard_count() {
        let index = ShardLocationIndex::new();
        assert_eq!(index.shard_count().await, 0);

        index.register_provider("n1", &[hash(1), hash(2), hash(3)]).await;
        assert_eq!(index.shard_count().await, 3);
    }

    #[tokio::test]
    async fn test_swarm_provider_provider_count() {
        let index = ShardLocationIndex::new();
        assert_eq!(index.provider_count().await, 0);

        index.register_provider("n1", &[hash(1)]).await;
        index.register_provider("n2", &[hash(1), hash(2)]).await;
        index.register_provider("n3", &[hash(3)]).await;

        assert_eq!(index.provider_count().await, 3);
    }

    #[tokio::test]
    async fn test_swarm_provider_get_unknown_shard() {
        let index = ShardLocationIndex::new();
        let providers = index.get_providers(&hash(0xFF)).await;
        assert!(providers.is_empty());
    }

    #[tokio::test]
    async fn test_swarm_provider_duplicate_register() {
        let index = ShardLocationIndex::new();
        let h1 = hash(0x11);

        // Registering same node twice should not duplicate
        index.register_provider("node-a", &[h1]).await;
        index.register_provider("node-a", &[h1]).await;

        let providers = index.get_providers(&h1).await;
        assert_eq!(providers.len(), 1);
    }

    // -- P6 TTL-expiry tests -----------------------------------------------

    #[tokio::test]
    async fn test_provider_ttl_expires_from_get() {
        use std::time::Duration;
        let index = ShardLocationIndex::new();
        let h1 = hash(0x55);

        // Register with an already-elapsed TTL so the record is born stale.
        index
            .register_provider_with_ttl("node-a", &[h1], Duration::from_millis(0))
            .await;
        // Ensure SystemTime has advanced past expires_at.
        std::thread::sleep(Duration::from_millis(5));

        let providers = index.get_providers(&h1).await;
        assert!(
            providers.is_empty(),
            "expired provider must not be handed out as a source"
        );
        // provider_count also filters expired.
        assert_eq!(index.provider_count().await, 0);
    }

    #[tokio::test]
    async fn test_provider_ttl_refresh_keeps_alive() {
        use std::time::Duration;
        let index = ShardLocationIndex::new();
        let h1 = hash(0x56);

        // Short TTL, then refresh with a long TTL before it expires.
        index
            .register_provider_with_ttl("node-a", &[h1], Duration::from_millis(30))
            .await;
        index
            .register_provider_with_ttl("node-a", &[h1], Duration::from_secs(300))
            .await;
        std::thread::sleep(Duration::from_millis(50));

        let providers = index.get_providers(&h1).await;
        assert_eq!(
            providers.len(),
            1,
            "a re-announced provider must stay alive past the original TTL"
        );
    }

    #[tokio::test]
    async fn test_cleanup_expired_reclaims_records() {
        use std::time::Duration;
        let index = ShardLocationIndex::new();
        let h1 = hash(0x57);
        let h2 = hash(0x58);

        // node-a stale on both shards; node-b fresh on h1.
        index
            .register_provider_with_ttl("node-a", &[h1, h2], Duration::from_millis(0))
            .await;
        index
            .register_provider_with_ttl("node-b", &[h1], Duration::from_secs(300))
            .await;
        std::thread::sleep(Duration::from_millis(5));

        let removed = index.cleanup_expired().await;
        assert_eq!(removed, 2, "both stale node-a records reclaimed");

        // h1 keeps node-b; h2 is emptied and dropped entirely.
        assert_eq!(index.get_providers(&h1).await, vec!["node-b".to_string()]);
        assert!(index.get_providers(&h2).await.is_empty());
        assert_eq!(index.shard_count().await, 1);
    }

    #[test]
    fn test_build_shard_announce_payload_empty() {
        let payload = build_shard_announce_payload(&[]);
        assert_eq!(payload.len(), 5);
        assert_eq!(payload[0], 0x04); // TAG_SHARD_ANNOUNCE
        assert_eq!(u32::from_le_bytes([payload[1], payload[2], payload[3], payload[4]]), 0);
    }

    #[test]
    fn test_build_shard_announce_payload_wire_format() {
        let h1 = hash(0xAA);
        let h2 = hash(0xBB);
        let payload = build_shard_announce_payload(&[h1, h2]);

        // tag(1) + count(4) + 2*hash(32) = 69
        assert_eq!(payload.len(), 69);
        assert_eq!(payload[0], 0x04);
        assert_eq!(u32::from_le_bytes([payload[1], payload[2], payload[3], payload[4]]), 2);

        // First hash
        assert_eq!(&payload[5..37], &[0xAA; 32]);
        // Second hash
        assert_eq!(&payload[37..69], &[0xBB; 32]);
    }

    #[test]
    fn test_build_shard_announce_payload_roundtrip() {
        let shards = vec![hash(0x01), hash(0x02), hash(0x03)];
        let payload = build_shard_announce_payload(&shards);

        // Parse it back
        assert_eq!(payload[0], 0x04);
        let count = u32::from_le_bytes([payload[1], payload[2], payload[3], payload[4]]) as usize;
        assert_eq!(count, 3);

        for i in 0..count {
            let offset = 5 + i * 32;
            let mut h = [0u8; 32];
            h.copy_from_slice(&payload[offset..offset + 32]);
            assert_eq!(ContentHash(h), shards[i]);
        }
    }

    // -- A2 shard-locate codec tests ---------------------------------------

    #[test]
    fn test_shard_locate_request_roundtrip() {
        let ch = hash(0x9A);
        let req = build_shard_locate_request(&ch);
        assert_eq!(req.len(), 33);
        assert_eq!(req[0], TAG_SHARD_LOCATE);
        assert_eq!(parse_shard_locate_request(&req), Some(ch));
    }

    #[test]
    fn test_shard_locate_request_rejects_malformed() {
        // Too short.
        assert_eq!(parse_shard_locate_request(&[TAG_SHARD_LOCATE, 0x01]), None);
        // Wrong tag.
        let mut wrong = build_shard_locate_request(&hash(0x01));
        wrong[0] = 0xFF;
        assert_eq!(parse_shard_locate_request(&wrong), None);
    }

    #[test]
    fn test_shard_locate_response_roundtrip() {
        let ids = vec![
            "9f4fc6ed4ba7".to_string(),
            "deadbeefcafe0011".to_string(),
        ];
        let resp = build_shard_locate_response(&ids);
        assert_eq!(resp[0], TAG_SHARD_LOCATE_RESPONSE);
        let parsed = parse_shard_locate_response(&resp);
        assert_eq!(parsed, ids);
    }

    #[test]
    fn test_shard_locate_response_empty() {
        let resp = build_shard_locate_response(&[]);
        assert_eq!(resp.len(), 5);
        assert!(parse_shard_locate_response(&resp).is_empty());
    }

    #[test]
    fn test_shard_locate_response_truncated_is_lenient() {
        // A count claiming 2 ids but with the second truncated yields only
        // the intact first id (garbled upstream answer == fewer providers).
        let mut resp = build_shard_locate_response(&["abc".to_string()]);
        // Bump count to 2 without adding the second id.
        resp[1] = 2;
        let parsed = parse_shard_locate_response(&resp);
        assert_eq!(parsed, vec!["abc".to_string()]);
    }

    #[test]
    fn test_shard_locate_response_rejects_oversized_count_no_alloc() {
        // A malicious upstream answers with a 5-byte response claiming u32::MAX
        // providers. `count` must be capped to what the buffer can hold so we do
        // NOT pre-allocate ~103 GB (a remote SIGABRT). Empty buffer → 0 count.
        let resp = [TAG_SHARD_LOCATE_RESPONSE, 0xFF, 0xFF, 0xFF, 0xFF];
        let parsed = parse_shard_locate_response(&resp);
        assert!(parsed.is_empty());
    }
}
