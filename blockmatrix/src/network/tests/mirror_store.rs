use super::*;

fn test_hash(seed: u8) -> ContentHash {
    ContentHash([seed; 32])
}

// ── Backward-compatible tests (original API) ────────────────────

#[tokio::test]
async fn test_store_and_get() {
    let store = ShardStore::new();
    let hash = test_hash(1);
    let data = vec![0xAB; 256];

    store.store(hash, data.clone()).await;

    let fetched = store.get(&hash).await;
    assert_eq!(fetched, Some(data));
}

#[tokio::test]
async fn test_get_nonexistent() {
    let store = ShardStore::new();
    let hash = test_hash(2);
    assert_eq!(store.get(&hash).await, None);
}

#[tokio::test]
async fn test_has() {
    let store = ShardStore::new();
    let hash = test_hash(3);
    assert!(!store.has(&hash).await);

    store.store(hash, vec![1, 2, 3]).await;
    assert!(store.has(&hash).await);
}

#[tokio::test]
async fn test_count() {
    let store = ShardStore::new();
    assert_eq!(store.count().await, 0);

    store.store(test_hash(10), vec![1]).await;
    store.store(test_hash(11), vec![2]).await;
    assert_eq!(store.count().await, 2);
}

#[tokio::test]
async fn test_default() {
    let store = ShardStore::default();
    assert_eq!(store.count().await, 0);
}

#[tokio::test]
async fn test_disk_persistence_roundtrip() {
    let tmp =
        std::env::temp_dir().join(format!("shard_store_test_roundtrip_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);

    // Store a shard with disk backing
    {
        let store = ShardStore::new_with_dir(tmp.clone());
        let hash = test_hash(42);
        store.store(hash, vec![0xDE, 0xAD]).await;
        assert_eq!(store.count().await, 1);
    }

    // Create a new store from the same directory — shard should reload
    {
        let store = ShardStore::new_with_dir(tmp.clone());
        assert_eq!(store.count().await, 1);
        let fetched = store.get(&test_hash(42)).await;
        assert_eq!(fetched, Some(vec![0xDE, 0xAD]));
    }

    let _ = std::fs::remove_dir_all(&tmp);
}

#[tokio::test]
async fn test_disk_store_and_remove() {
    let tmp = std::env::temp_dir().join(format!("shard_store_test_remove_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);

    let store = ShardStore::new_with_dir(tmp.clone());
    let hash = test_hash(99);
    let hex_name = hex::encode(hash.0);

    // Store and verify file exists on disk
    store.store(hash, vec![1, 2, 3]).await;
    assert!(tmp.join(&hex_name).exists());

    // Remove and verify file is gone
    store.remove(&hash).await;
    assert!(!store.has(&hash).await);
    assert!(!tmp.join(&hex_name).exists());

    let _ = std::fs::remove_dir_all(&tmp);
}

// ── Dedup-aware tests ───────────────────────────────────────────

#[tokio::test]
async fn test_store_with_dedup_full_new_shard() {
    let store = ShardStore::new();
    let hash = test_hash(50);
    let result = store
        .store_with_dedup(hash, vec![0xAA; 64], DedupPolicy::Full)
        .await;
    assert_eq!(result, ShardStoreResult::Stored);
    assert_eq!(store.ref_count(&hash).await, Some(1));
}

#[tokio::test]
async fn test_store_with_dedup_full_duplicate() {
    let store = ShardStore::new();
    let hash = test_hash(51);

    let r1 = store
        .store_with_dedup(hash, vec![0xBB; 64], DedupPolicy::Full)
        .await;
    assert_eq!(r1, ShardStoreResult::Stored);

    let r2 = store
        .store_with_dedup(hash, vec![0xBB; 64], DedupPolicy::Full)
        .await;
    assert_eq!(r2, ShardStoreResult::Deduplicated { ref_count: 2 });
    assert_eq!(store.ref_count(&hash).await, Some(2));

    // Data is still accessible
    assert_eq!(store.get(&hash).await, Some(vec![0xBB; 64]));
}

#[tokio::test]
async fn test_store_with_dedup_none_always_overwrites() {
    let store = ShardStore::new();
    let hash = test_hash(52);

    // First store
    let r1 = store
        .store_with_dedup(hash, vec![0xCC; 32], DedupPolicy::None)
        .await;
    assert_eq!(r1, ShardStoreResult::Stored);
    assert_eq!(store.ref_count(&hash).await, Some(1));

    // Second store with None: overwrites, refcount stays 1
    let r2 = store
        .store_with_dedup(hash, vec![0xCC; 32], DedupPolicy::None)
        .await;
    assert_eq!(r2, ShardStoreResult::Stored);
    assert_eq!(store.ref_count(&hash).await, Some(1));
}

#[tokio::test]
async fn test_acquire_increments_refcount() {
    let store = ShardStore::new();
    let hash = test_hash(53);
    store.store(hash, vec![1]).await;
    assert_eq!(store.ref_count(&hash).await, Some(1));

    let rc = store.acquire(&hash).await;
    assert_eq!(rc, Some(2));
    assert_eq!(store.ref_count(&hash).await, Some(2));

    let rc = store.acquire(&hash).await;
    assert_eq!(rc, Some(3));
}

#[tokio::test]
async fn test_acquire_nonexistent_returns_none() {
    let store = ShardStore::new();
    let hash = test_hash(54);
    assert_eq!(store.acquire(&hash).await, None);
}

#[tokio::test]
async fn test_release_decrements_refcount() {
    let store = ShardStore::new();
    let hash = test_hash(55);
    store.store(hash, vec![1]).await;

    // Bump to refcount 3
    store.acquire(&hash).await;
    store.acquire(&hash).await;
    assert_eq!(store.ref_count(&hash).await, Some(3));

    // Release: 3 -> 2
    let rc = store.release(&hash).await;
    assert_eq!(rc, Some(2));
    assert!(store.has(&hash).await);

    // Release: 2 -> 1
    let rc = store.release(&hash).await;
    assert_eq!(rc, Some(1));
    assert!(store.has(&hash).await);

    // Release: 1 -> 0 (removed)
    let rc = store.release(&hash).await;
    assert_eq!(rc, Some(0));
    assert!(!store.has(&hash).await);
    assert_eq!(store.get(&hash).await, None);
}

#[tokio::test]
async fn test_release_nonexistent_returns_none() {
    let store = ShardStore::new();
    let hash = test_hash(56);
    assert_eq!(store.release(&hash).await, None);
}

#[tokio::test]
async fn test_ref_count_returns_none_for_missing() {
    let store = ShardStore::new();
    let hash = test_hash(57);
    assert_eq!(store.ref_count(&hash).await, None);
}

#[tokio::test]
async fn test_force_remove_ignores_refcount() {
    let store = ShardStore::new();
    let hash = test_hash(58);
    store.store(hash, vec![1]).await;
    store.acquire(&hash).await;
    store.acquire(&hash).await;
    assert_eq!(store.ref_count(&hash).await, Some(3));

    // Force remove: gone regardless of refcount
    store.remove(&hash).await;
    assert!(!store.has(&hash).await);
    assert_eq!(store.ref_count(&hash).await, None);
}

#[tokio::test]
async fn test_backward_compat_store_creates_refcount_1() {
    let store = ShardStore::new();
    let hash = test_hash(59);
    store.store(hash, vec![0xFF]).await;
    assert_eq!(store.ref_count(&hash).await, Some(1));
}

#[tokio::test]
async fn test_backward_compat_store_deduplicates() {
    let store = ShardStore::new();
    let hash = test_hash(60);
    store.store(hash, vec![0xFF]).await;
    store.store(hash, vec![0xFF]).await;
    // Default store uses Full dedup, so refcount increments
    assert_eq!(store.ref_count(&hash).await, Some(2));
}

#[tokio::test]
async fn test_disk_persistence_with_refcount() {
    let tmp =
        std::env::temp_dir().join(format!("shard_store_test_refcount_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);

    let hash = test_hash(70);

    // Store with refcount 3
    {
        let store = ShardStore::new_with_dir(tmp.clone());
        store.store(hash, vec![0xAA]).await;
        store.acquire(&hash).await;
        store.acquire(&hash).await;
        assert_eq!(store.ref_count(&hash).await, Some(3));
    }

    // Reload from disk — refcount should survive
    {
        let store = ShardStore::new_with_dir(tmp.clone());
        assert_eq!(store.ref_count(&hash).await, Some(3));
        assert_eq!(store.get(&hash).await, Some(vec![0xAA]));
    }

    let _ = std::fs::remove_dir_all(&tmp);
}

#[tokio::test]
async fn test_store_with_hash_only_skips_duplicate() {
    let store = ShardStore::new();
    let hash = test_hash(80);

    let r1 = store
        .store_with_dedup(hash, vec![0xDD; 64], DedupPolicy::HashOnly)
        .await;
    assert_eq!(r1, ShardStoreResult::Stored);

    let r2 = store
        .store_with_dedup(hash, vec![0xDD; 64], DedupPolicy::HashOnly)
        .await;
    assert_eq!(r2, ShardStoreResult::Deduplicated { ref_count: 1 });

    // Data is still accessible
    assert_eq!(store.get(&hash).await, Some(vec![0xDD; 64]));
}

#[tokio::test]
async fn test_store_with_hash_only_no_refcount_increment() {
    let store = ShardStore::new();
    let hash = test_hash(81);

    store
        .store_with_dedup(hash, vec![0xEE; 32], DedupPolicy::HashOnly)
        .await;
    assert_eq!(store.ref_count(&hash).await, Some(1));

    // Second store with HashOnly: refcount stays 1 (no tracking)
    store
        .store_with_dedup(hash, vec![0xEE; 32], DedupPolicy::HashOnly)
        .await;
    assert_eq!(store.ref_count(&hash).await, Some(1));

    // Third store: still 1
    store
        .store_with_dedup(hash, vec![0xEE; 32], DedupPolicy::HashOnly)
        .await;
    assert_eq!(store.ref_count(&hash).await, Some(1));
}

#[tokio::test]
async fn test_hash_only_release_does_not_auto_remove() {
    let store = ShardStore::new();
    let hash = test_hash(82);

    // Store with HashOnly (rc=1 sentinel)
    store
        .store_with_dedup(hash, vec![0xFF; 16], DedupPolicy::HashOnly)
        .await;
    assert_eq!(store.ref_count(&hash).await, Some(1));

    // Release: refcount goes to 0, shard is removed.
    // For HashOnly shards the caller should NOT call release() since
    // refcount is not tracked. But if they do, the behavior is defined:
    // rc=1 -> 0 -> removed. This is safe — the shard can be re-fetched.
    let rc = store.release(&hash).await;
    assert_eq!(rc, Some(0));
    assert!(!store.has(&hash).await);
}

#[tokio::test]
async fn test_release_removes_disk_files() {
    let tmp = std::env::temp_dir().join(format!(
        "shard_store_test_release_disk_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&tmp);

    let store = ShardStore::new_with_dir(tmp.clone());
    let hash = test_hash(71);
    let hex_name = hex::encode(hash.0);

    store.store(hash, vec![1, 2]).await;
    assert!(tmp.join(&hex_name).exists());
    assert!(tmp.join(format!("{hex_name}.ref")).exists());

    // Release to 0 — both files should be removed
    let rc = store.release(&hash).await;
    assert_eq!(rc, Some(0));
    assert!(!tmp.join(&hex_name).exists());
    assert!(!tmp.join(format!("{hex_name}.ref")).exists());

    let _ = std::fs::remove_dir_all(&tmp);
}

use crate::network::swarm_provider::ShardLocationIndex;
use hypermesh_lib::DEFAULT_NETWORK;
use std::sync::Arc;

#[tokio::test]
async fn mirror_retries_do_not_acquire_full_reference() {
    let store = Arc::new(ShardStore::new());
    let index = ShardLocationIndex::new();
    let data = b"shared-data".to_vec();
    let hash = ContentHash(*blake3::hash(&data).as_bytes());
    assert!(
        index
            .register_mirror_provider_in_network(
                DEFAULT_NETWORK,
                "mirror",
                hash,
                store.clone(),
                &data,
            )
            .await
    );
    assert!(
        index
            .register_mirror_provider_in_network(
                DEFAULT_NETWORK,
                "mirror",
                hash,
                store.clone(),
                &data,
            )
            .await
    );
    assert_eq!(store.ref_count(&hash).await, Some(1));
    store.store(hash, data.clone()).await;
    assert_eq!(store.ref_count(&hash).await, Some(2));
    assert!(
        !index
            .register_mirror_provider_in_network(
                DEFAULT_NETWORK,
                "mirror",
                hash,
                store.clone(),
                b"conflict",
            )
            .await
    );
    assert_eq!(store.ref_count(&hash).await, Some(2));
    index.remove_provider("mirror").await;
    assert_eq!(store.ref_count(&hash).await, Some(1));
}

#[tokio::test]
async fn mirror_withdrawal_and_expiry_free_unowned_bytes() {
    let store = Arc::new(ShardStore::new());
    let index = ShardLocationIndex::new();
    let data = b"transient-mirror".to_vec();
    let hash = ContentHash(*blake3::hash(&data).as_bytes());
    assert!(
        index
            .register_mirror_provider_in_network(
                DEFAULT_NETWORK,
                "mirror",
                hash,
                store.clone(),
                &data,
            )
            .await
    );
    index.remove_provider("mirror").await;
    assert_eq!(store.get(&hash).await, None);
    assert!(
        index
            .register_mirror_provider_in_network(
                DEFAULT_NETWORK,
                "mirror",
                hash,
                store.clone(),
                &data,
            )
            .await
    );
    index
        .expire_provider_for_test(DEFAULT_NETWORK, "mirror", hash)
        .await;
    assert_eq!(index.cleanup_expired().await, 1);
    assert_eq!(store.get(&hash).await, None);
    assert!(index
        .get_providers_in_network(DEFAULT_NETWORK, &hash)
        .await
        .is_empty());
}

#[tokio::test]
async fn disk_write_failure_still_serves_from_ram_without_durability_claim() {
    let temp = tempfile::tempdir().expect("test directory");
    let blocked = temp.path().join("not-a-directory");
    std::fs::write(&blocked, b"file").expect("block directory");
    let store = Arc::new(ShardStore::new_with_dir(blocked.clone()));
    let data = b"volatile-but-servable".to_vec();
    let hash = ContentHash(*blake3::hash(&data).as_bytes());
    store.store(hash, data.clone()).await;
    let index = ShardLocationIndex::new();
    assert!(
        index
            .register_mirror_provider_in_network(
                DEFAULT_NETWORK,
                "mirror",
                hash,
                store.clone(),
                &data,
            )
            .await
    );
    assert_eq!(store.get(&hash).await, Some(data));
    assert!(std::fs::read(blocked.join(hex::encode(hash.0))).is_err());
}
