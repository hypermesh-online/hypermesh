use super::*;
use crate::network::shard_transport::{handle_shard_message, MockShardTransport};
use hypermesh_lib::{ContentHash, NodeId, DEFAULT_NETWORK};
use ngauge::FallbackStrategy;

struct Harness {
    remote: NodeId,
    transport: Arc<MockShardTransport>,
    store: Arc<ShardStore>,
    index: Arc<ShardLocationIndex>,
    analytics: Arc<Mutex<ngauge::SwarmAnalytics>>,
    executor: StoqDmsExecutor,
}

impl Harness {
    fn new() -> Self {
        let remote = NodeId::from_public_key(b"remote");
        let transport = Arc::new(MockShardTransport::new());
        let store = Arc::new(ShardStore::new());
        let index = Arc::new(ShardLocationIndex::new());
        let analytics = Arc::new(Mutex::new(ngauge::SwarmAnalytics::new()));
        let executor = StoqDmsExecutor::new(
            transport.clone(), store.clone(), index.clone(), "local".into(), analytics.clone(),
        );
        Self { remote, transport, store, index, analytics, executor }
    }

    fn action(&self, hash: ContentHash) -> MirrorAction {
        MirrorAction {
            network: DEFAULT_NETWORK,
            shard_id: hash,
            source: self.remote,
            urgency: 1.0,
            fallback: FallbackStrategy::Sequential,
        }
    }

    async fn assert_no_provider(&self, hash: ContentHash) {
        assert!(self.index.get_providers_in_network(DEFAULT_NETWORK, &hash).await.is_empty());
        assert_eq!(self.analytics.lock().expect("analytics").get_replica_count(&hash), 0);
        assert!(self.store.get(&hash).await.is_none());
    }
}

async fn serve(store: &ShardStore, hash: ContentHash) -> Vec<u8> {
    let mut request = vec![0x02];
    request.extend_from_slice(&hash.0);
    handle_shard_message(&request, store, None)
        .await.expect("serve handler").expect("serve response")
}

#[tokio::test]
async fn rejects_wrong_bytes_without_provider() {
    let h = Harness::new();
    let hash = ContentHash(*blake3::hash(b"correct").as_bytes());
    h.transport.insert_shard(&h.remote, &hash, b"wrong".to_vec()).await;
    assert!(h.executor.fetch_and_register(&h.action(hash)).await.is_err());
    h.assert_no_provider(hash).await;
}

#[tokio::test]
async fn rejects_empty_malformed_and_transport_failure() {
    let h = Harness::new();
    let hash = ContentHash(*blake3::hash(b"correct").as_bytes());
    assert!(h.executor.fetch_and_register(&h.action(hash)).await.is_err());
    h.assert_no_provider(hash).await;
    h.transport.insert_shard(&h.remote, &hash, Vec::new()).await;
    assert!(h.executor.fetch_and_register(&h.action(hash)).await.is_err());
    h.assert_no_provider(hash).await;
    h.transport.insert_shard(&h.remote, &hash, vec![0, 0, 0, 99, 1]).await;
    assert!(h.executor.fetch_and_register(&h.action(hash)).await.is_err());
    h.assert_no_provider(hash).await;
}

#[tokio::test]
async fn retains_valid_raw_shard_before_advertisement() {
    let h = Harness::new();
    let data = b"raw-mirror-shard".to_vec();
    let hash = ContentHash(*blake3::hash(&data).as_bytes());
    h.transport.insert_shard(&h.remote, &hash, data.clone()).await;
    assert_eq!(h.executor.fetch_and_register(&h.action(hash)).await.expect("mirror"), 1);
    let response = serve(&h.store, hash).await;
    let (served, _) = parse_shard_response(&response, &hash);
    assert_eq!(served, data);
    assert_eq!(h.index.get_providers_in_network(DEFAULT_NETWORK, &hash).await, vec!["local"]);
    assert_eq!(h.analytics.lock().expect("analytics").get_replica_count(&hash), 1);
}

#[tokio::test]
async fn retains_valid_enveloped_shard_before_advertisement() {
    let h = Harness::new();
    let data = b"enveloped-mirror-shard".to_vec();
    let hash = ContentHash(*blake3::hash(&data).as_bytes());
    let source_store = ShardStore::new();
    source_store.store(hash, data.clone()).await;
    h.transport.insert_shard(&h.remote, &hash, serve(&source_store, hash).await).await;
    assert_eq!(h.executor.fetch_and_register(&h.action(hash)).await.expect("mirror"), 1);
    let (served, _) = parse_shard_response(&serve(&h.store, hash).await, &hash);
    assert_eq!(served, data);
    assert_eq!(h.store.ref_count(&hash).await, Some(1));
}

#[tokio::test]
async fn duplicate_and_concurrent_retry_do_not_inflate_reference_or_count() {
    let h = Harness::new();
    let data = b"same-shard".to_vec();
    let hash = ContentHash(*blake3::hash(&data).as_bytes());
    h.transport.insert_shard(&h.remote, &hash, data).await;
    let action = h.action(hash);
    let (first, second) = tokio::join!(
        h.executor.fetch_and_register(&action), h.executor.fetch_and_register(&action)
    );
    assert_eq!(first.expect("first mirror"), 1);
    assert_eq!(second.expect("second mirror"), 1);
    assert_eq!(h.executor.fetch_and_register(&action).await.expect("retry"), 1);
    assert_eq!(h.store.ref_count(&hash).await, Some(1));
    assert_eq!(h.analytics.lock().expect("analytics").get_replica_count(&hash), 1);
}

#[tokio::test]
async fn owner_release_does_not_strand_active_mirror_hint() {
    let h = Harness::new();
    let data = b"borrowed-owner-shard".to_vec();
    let hash = ContentHash(*blake3::hash(&data).as_bytes());
    h.store.store(hash, data.clone()).await;
    h.transport.insert_shard(&h.remote, &hash, data.clone()).await;
    assert_eq!(h.executor.fetch_and_register(&h.action(hash)).await.expect("mirror"), 1);
    assert_eq!(h.store.release(&hash).await, Some(1));
    let (served, _) = parse_shard_response(&serve(&h.store, hash).await, &hash);
    assert_eq!(served, data);
    assert_eq!(h.index.get_providers_in_network(DEFAULT_NETWORK, &hash).await, vec!["local"]);
}

#[tokio::test]
async fn explicit_owner_remove_does_not_strand_active_mirror_hint() {
    let h = Harness::new();
    let data = b"removed-owner-shard".to_vec();
    let hash = ContentHash(*blake3::hash(&data).as_bytes());
    h.store.store(hash, data.clone()).await;
    h.transport.insert_shard(&h.remote, &hash, data.clone()).await;
    assert_eq!(h.executor.fetch_and_register(&h.action(hash)).await.expect("mirror"), 1);
    h.store.remove(&hash).await;
    let (served, _) = parse_shard_response(&serve(&h.store, hash).await, &hash);
    assert_eq!(served, data);
    assert_eq!(h.index.get_providers_in_network(DEFAULT_NETWORK, &hash).await, vec!["local"]);
}

mod provider_regression_tests {
    use super::*;
    use crate::network::swarm_provider::build_shard_announce_payload;

    fn hash(seed: u8) -> ContentHash { ContentHash([seed; 32]) }

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

}

#[tokio::test]
async fn owner_remove_before_registration_still_requires_new_verified_bytes() {
    let h = Harness::new();
    let data = b"before-register".to_vec();
    let hash = ContentHash(*blake3::hash(&data).as_bytes());
    h.store.store(hash, data.clone()).await;
    h.store.remove(&hash).await;
    h.transport.insert_shard(&h.remote, &hash, data.clone()).await;
    assert_eq!(h.executor.fetch_and_register(&h.action(hash)).await.expect("mirror"), 1);
    let (served, _) = parse_shard_response(&serve(&h.store, hash).await, &hash);
    assert_eq!(served, data);
    h.index.remove_provider("local").await;
    assert_eq!(h.store.get(&hash).await, None);
}

#[tokio::test]
async fn owner_release_during_registration_keeps_bytes_until_withdrawal() {
    let h = Harness::new();
    let data = b"during-release".to_vec();
    let hash = ContentHash(*blake3::hash(&data).as_bytes());
    h.store.store(hash, data.clone()).await;
    let store_for_hook = h.store.clone();
    assert!(h.index.register_mirror_with_hook(
        DEFAULT_NETWORK, "local", hash, h.store.clone(), &data,
        move || { assert_eq!(store_for_hook.release_owner_now(&hash), Some(0)); },
    ).await);
    assert_eq!(h.store.get(&hash).await, Some(data));
    h.index.remove_provider("local").await;
    assert_eq!(h.store.get(&hash).await, None);
}

#[tokio::test]
async fn owner_remove_during_registration_keeps_bytes_until_expiry() {
    let h = Harness::new();
    let data = b"during-remove".to_vec();
    let hash = ContentHash(*blake3::hash(&data).as_bytes());
    h.store.store(hash, data.clone()).await;
    let store_for_hook = h.store.clone();
    assert!(h.index.register_mirror_with_hook(
        DEFAULT_NETWORK, "local", hash, h.store.clone(), &data,
        move || store_for_hook.remove_owner_now(&hash),
    ).await);
    assert_eq!(h.store.get(&hash).await, Some(data));
    h.index.expire_provider_for_test(DEFAULT_NETWORK, "local", hash).await;
    assert_eq!(h.index.cleanup_expired().await, 1);
    assert_eq!(h.store.get(&hash).await, None);
}

#[tokio::test]
async fn failed_claims_do_not_leave_empty_buckets() {
    let store = Arc::new(ShardStore::new());
    let index = ShardLocationIndex::new();
    let mut known = Vec::new();
    for i in 0..64u8 {
        let data = vec![i];
        let hash = ContentHash(*blake3::hash(&data).as_bytes());
        store.store(hash, data.clone()).await;
        known.push((hash, data));
    }
    let before_store = store.count().await;
    for (hash, _) in &known {
        assert!(!index.register_mirror_provider_in_network(
            DEFAULT_NETWORK, "local", *hash, store.clone(), b"wrong-bytes",
        ).await);
    }
    assert_eq!(index.shard_count().await, 0);
    assert_eq!(index.provider_count().await, 0);
    assert_eq!(store.count().await, before_store);
    for (hash, _) in &known { assert_eq!(store.ref_count(hash).await, Some(1)); }
    let (hash, data) = &known[0];
    assert!(index.register_mirror_provider_in_network(
        DEFAULT_NETWORK, "local", *hash, store.clone(), data,
    ).await);
    assert_eq!(index.shard_count().await, 1);
    assert_eq!(index.provider_count().await, 1);
    assert_eq!(store.ref_count(hash).await, Some(2));
}
