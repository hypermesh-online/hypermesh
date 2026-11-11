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
use super::shard_store::ShardStore;

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
struct ProviderEntry {
    /// When this provider was first learned or last refreshed. Used for
    /// oldest-first tie-breaking during introspection (`age`), mirroring the
    /// DNS cache's `cached_at`.
    registered_at: SystemTime,
    /// When this location hint should be considered stale.
    expires_at: SystemTime,
    mirror_hold: Option<(Arc<ShardStore>, ContentHash)>,
}

impl ProviderEntry {
    fn new(ttl: Duration) -> Self {
        let now = SystemTime::now();
        Self {
            registered_at: now,
            expires_at: now + ttl,
            mirror_hold: None,
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

impl Drop for ProviderEntry {
    fn drop(&mut self) {
        if let Some((store, shard_id)) = &self.mirror_hold {
            store.release_mirror(shard_id);
        }
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

    /// Atomically claim serve bytes and register one local mirror provider hint.
    /// Lock order: index write → synchronous store lock; no await while locked.
    pub(crate) async fn register_mirror_provider_in_network(
        &self, network_id: NetworkId, node_id: &str, shard_id: ContentHash,
        store: Arc<ShardStore>, data: &[u8],
    ) -> bool {
        let mut locs = self.locations.write().await;
        self.insert_mirror_entry(&mut locs, network_id, node_id, shard_id, store, data)
    }

    fn insert_mirror_entry(
        &self,
        locs: &mut HashMap<(NetworkId, ContentHash), HashMap<String, ProviderEntry>>,
        network_id: NetworkId, node_id: &str, shard_id: ContentHash,
        store: Arc<ShardStore>, data: &[u8],
    ) -> bool {
        let key = (network_id, shard_id);
        if let Some(entry) = locs.get_mut(&key).and_then(|p| p.get_mut(node_id)) {
            return self.refresh_mirror_entry(entry, shard_id, store, data);
        }
        if !store.claim_mirror(shard_id, data) { return false; }
        let mut entry = ProviderEntry::new(self.ttl);
        entry.mirror_hold = Some((store, shard_id));
        locs.entry(key).or_default().insert(node_id.to_string(), entry);
        true
    }

    fn refresh_mirror_entry(
        &self, entry: &mut ProviderEntry, shard_id: ContentHash,
        store: Arc<ShardStore>, data: &[u8],
    ) -> bool {
        if let Some((held, _)) = &entry.mirror_hold {
            if !Arc::ptr_eq(held, &store) || !store.mirror_bytes_match(&shard_id, data) {
                return false;
            }
            entry.refresh(self.ttl);
            return true;
        }
        if !store.claim_mirror(shard_id, data) { return false; }
        entry.mirror_hold = Some((store, shard_id));
        entry.refresh(self.ttl);
        true
    }

    #[cfg(test)]
    pub(crate) async fn register_mirror_with_hook<F: FnOnce() + Send>(
        &self, network_id: NetworkId, node_id: &str, shard_id: ContentHash,
        store: Arc<ShardStore>, data: &[u8], hook: F,
    ) -> bool {
        let mut locs = self.locations.write().await;
        hook();
        self.insert_mirror_entry(&mut locs, network_id, node_id, shard_id, store, data)
    }

    #[cfg(test)]
    pub(crate) async fn expire_provider_for_test(
        &self, network_id: NetworkId, node_id: &str, shard_id: ContentHash,
    ) {
        let mut locs = self.locations.write().await;
        let Some(entry) = locs.get_mut(&(network_id, shard_id))
            .and_then(|providers| providers.get_mut(node_id)) else { return; };
        entry.expires_at = SystemTime::UNIX_EPOCH;
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
